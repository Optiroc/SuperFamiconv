//! Dithering implementation.

use clap::ValueEnum;

use crate::color::{CandidateColor, NormalizedColor, ReducedColor, eq_rgb, oklab_sqdist_hue_weighted};
use crate::image::{self, Image};
use crate::mode::Mode;
use crate::mode::color::{ColorRounding, ModeColor};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ValueEnum)]
#[value(rename_all = "snake_case")]
pub enum Dither {
    /// No dithering.
    Off,
    /// Bayer 2x2 dithering.
    #[clap(name = "bayer2")]
    Bayer2x2,
    /// Bayer 4x4 dithering.
    #[clap(name = "bayer4")]
    Bayer4x4,
    /// Bayer 8x8 dithering.
    #[clap(name = "bayer8")]
    Bayer8x8,
    /// Checkerboard dithering.
    Checker,
    /// Horizontal stippled dithering.
    StippleH,
    /// Vertical stippled dithering.
    StippleV,
    /// Atkinson error-diffusion dithering.
    Atkinson,
    /// Floyd-Steinberg error-diffusion dithering.
    #[clap(name = "fs")]
    FloydSteinberg,
}

/// Chroma mismatch penalty when chosing dither candidates.
const CHROMA_WEIGHT: f32 = 3.0;

#[rustfmt::skip]
const BAYER_2X2_MATRIX: [[u8; 2]; 2] = [
    [0, 2],
    [3, 1],
];

#[rustfmt::skip]
const BAYER_4X4_MATRIX: [[u8; 4]; 4] = [
    [ 0,  8,  2, 10],
    [12,  4, 14,  6],
    [ 3, 11,  1,  9],
    [15,  7, 13,  5],
];

#[rustfmt::skip]
const BAYER_8X8_MATRIX: [[u8; 8]; 8] = [
    [ 0, 48, 12, 60,  3, 51, 15, 63],
    [32, 16, 44, 28, 35, 19, 47, 31],
    [ 8, 56,  4, 52, 11, 59,  7, 55],
    [40, 24, 36, 20, 43, 27, 39, 23],
    [ 2, 50, 14, 62,  1, 49, 13, 61],
    [34, 18, 46, 30, 33, 17, 45, 29],
    [10, 58,  6, 54,  9, 57,  5, 53],
    [42, 26, 38, 22, 41, 25, 37, 21],
];

#[rustfmt::skip]
const CHECKER_MATRIX: [[u8; 2]; 2] = [
    [0, 1],
    [1, 0],
];

#[rustfmt::skip]
const STIPPLE_H_MATRIX: [[u8; 2]; 4] = [
    [0, 6],
    [4, 2],
    [1, 7],
    [5, 3],
];

#[rustfmt::skip]
const STIPPLE_V_MATRIX: [[u8; 4]; 2] = [
    [0, 4, 1, 5],
    [6, 2, 7, 3],
];

/// An error-diffusion kernel: (dx, dy, diffusion factor).
type ErrorKernel = &'static [(i32, i32, f32)];

#[rustfmt::skip]
const ATKINSON_KERNEL: ErrorKernel = &[
                        (1, 0, 1.0 / 8.0), (2, 0, 1.0 / 8.0),
    (-1, 1, 1.0 / 8.0), (0, 1, 1.0 / 8.0), (1, 1, 1.0 / 8.0),
                        (0, 2, 1.0 / 8.0),
];

#[rustfmt::skip]
const FLOYD_STEINBERG_KERNEL: ErrorKernel = &[
                                             (1, 0, 7.0 / 16.0),
    (-1, 1, 3.0 / 16.0), (0, 1, 5.0 / 16.0), (1, 1, 1.0 / 16.0),
];

/// Quantizes `width * height` pixels against `palette`, applying `dither`.
pub fn quantize_pixels(
    mode: Mode,
    palette: &[ReducedColor],
    width: u32,
    height: u32,
    dither: Dither,
    rounding: ColorRounding,
    color_at: impl Fn(usize) -> NormalizedColor,
) -> (Vec<u8>, Vec<NormalizedColor>) {
    let candidates: Vec<CandidateColor> = palette
        .iter()
        .map(|&r| CandidateColor::new(r, mode.normalize_color(r)))
        .collect();

    let size = (width * height) as usize;
    let mut indexed_data = vec![0u8; size];
    let mut data = vec![NormalizedColor::TRANSPARENT; size];
    let mut ditherer = Ditherer::new(dither, 0, 0, width, height);

    for i in 0..size {
        let nc = color_at(i);
        if mode.reduce_color(nc, rounding).is_transparent() {
            continue;
        }
        let x = (i as u32) % width;
        let y = (i as u32) / width;
        let chosen = ditherer.color_at(x, y, nc, &candidates);
        let index = palette.iter().position(|&c| c == chosen).unwrap();
        indexed_data[i] = index as u8;
        data[i] = mode.normalize_color(chosen);
    }

    (indexed_data, data)
}

/// Quantizes `image` constrained to `mode`'s color depth, applying `dither`.
pub fn dither_to_mode(
    image: &Image,
    mode: Mode,
    dither: Dither,
    rounding: ColorRounding,
    color_zero: Option<ReducedColor>,
) -> Image {
    let mut reduced: Vec<ReducedColor> = image
        .data
        .iter()
        .map(|&c| mode.reduce_color(c, rounding))
        .filter(|c| !c.is_transparent())
        .collect();
    reduced.sort();
    reduced.dedup();

    let candidates: Vec<CandidateColor> = reduced
        .iter()
        .map(|&r| CandidateColor::new(r, mode.normalize_color(r)))
        .collect();

    let size = (image.width * image.height) as usize;
    let mut data = vec![NormalizedColor::TRANSPARENT; size];
    let mut ditherer = Ditherer::new(dither, image.src_x, image.src_y, image.width, image.height);

    for (i, out) in data.iter_mut().enumerate() {
        let nc = image.color_at(i);
        let rc = mode.reduce_color(nc, rounding);
        if rc.is_transparent() {
            continue;
        }

        let chosen = if color_zero.is_some_and(|cz| eq_rgb(rc, cz)) {
            // Leave color-zero pixels untouched, same as the final palette-based dither pass
            rc
        } else {
            let x = image.src_x + (i as u32) % image.width;
            let y = image.src_y + (i as u32) / image.width;
            ditherer.color_at(x, y, nc, &candidates)
        };
        *out = mode.normalize_color(chosen);
    }

    let colors = image::colors_in(&data);

    Image {
        width: image.width,
        height: image.height,
        src_x: image.src_x,
        src_y: image.src_y,
        data,
        indexed_data: Vec::new(),
        palette: Vec::new(),
        colors,
    }
}

const fn ordered_threshold(
    dither: Dither,
    x: u32,
    y: u32,
) -> f32 {
    match dither {
        Dither::Bayer2x2 => (BAYER_2X2_MATRIX[(y & 1) as usize][(x & 1) as usize] as f32 + 0.5) / 4.0,
        Dither::Bayer4x4 => (BAYER_4X4_MATRIX[(y & 3) as usize][(x & 3) as usize] as f32 + 0.5) / 16.0,
        Dither::Bayer8x8 => (BAYER_8X8_MATRIX[(y & 7) as usize][(x & 7) as usize] as f32 + 0.5) / 64.0,
        Dither::Checker => (CHECKER_MATRIX[(y & 1) as usize][(x & 1) as usize] as f32 + 0.5) / 2.0,
        Dither::StippleH => (STIPPLE_H_MATRIX[(y & 1) as usize][(x & 3) as usize] as f32 + 0.5) / 8.0,
        Dither::StippleV => (STIPPLE_V_MATRIX[(y & 3) as usize][(x & 1) as usize] as f32 + 0.5) / 8.0,
        Dither::Off | Dither::Atkinson | Dither::FloydSteinberg => {
            panic!("ordered_threshold called for non-ordered dither")
        }
    }
}

pub struct Ditherer {
    dither: Dither,
    origin_x: u32,
    origin_y: u32,
    width: u32,
    height: u32,
    error: Vec<[f32; 3]>,
}

impl Ditherer {
    pub fn new(
        dither: Dither,
        origin_x: u32,
        origin_y: u32,
        width: u32,
        height: u32,
    ) -> Self {
        let error = if matches!(dither, Dither::Atkinson | Dither::FloydSteinberg) {
            vec![[0.0f32; 3]; (width * height) as usize]
        } else {
            Vec::new()
        };
        Ditherer {
            dither,
            origin_x,
            origin_y,
            width,
            height,
            error,
        }
    }

    pub fn color_at(
        &mut self,
        x: u32,
        y: u32,
        color: NormalizedColor,
        candidates: &[CandidateColor],
    ) -> ReducedColor {
        match self.dither {
            Dither::Off => nearest(candidates, color).reduced,
            Dither::Bayer2x2
            | Dither::Bayer4x4
            | Dither::Bayer8x8
            | Dither::Checker
            | Dither::StippleH
            | Dither::StippleV => ordered_color_at(x, y, color, candidates, self.dither),
            Dither::Atkinson | Dither::FloydSteinberg => {
                self.diffusion_color_at(x - self.origin_x, y - self.origin_y, color, candidates, self.dither)
            }
        }
    }

    fn diffusion_color_at(
        &mut self,
        local_x: u32,
        local_y: u32,
        color: NormalizedColor,
        candidates: &[CandidateColor],
        dither: Dither,
    ) -> ReducedColor {
        let index = (local_y * self.width + local_x) as usize;
        let e = self.error[index];
        let biased = NormalizedColor::new(
            clamp_u8(f32::from(color.r) + e[0]),
            clamp_u8(f32::from(color.g) + e[1]),
            clamp_u8(f32::from(color.b) + e[2]),
            color.a,
        );
        let color = nearest(candidates, biased);
        let residual = [
            f32::from(biased.r) - f32::from(color.normalized.r),
            f32::from(biased.g) - f32::from(color.normalized.g),
            f32::from(biased.b) - f32::from(color.normalized.b),
        ];
        let kernel = match dither {
            Dither::Atkinson => ATKINSON_KERNEL,
            Dither::FloydSteinberg => FLOYD_STEINBERG_KERNEL,
            _ => unreachable!(),
        };
        self.diffuse(local_x, local_y, residual, kernel);
        color.reduced
    }

    fn diffuse(
        &mut self,
        x: u32,
        y: u32,
        residual: [f32; 3],
        kernel: ErrorKernel,
    ) {
        for &(dx, dy, ratio) in kernel {
            let (nx, ny) = (x as i32 + dx, y as i32 + dy);
            if nx < 0 || ny < 0 || nx as u32 >= self.width || ny as u32 >= self.height {
                continue;
            }
            let index = (ny as u32 * self.width + nx as u32) as usize;
            for (c, &r) in residual.iter().enumerate() {
                self.error[index][c] += r * ratio;
            }
        }
    }
}

fn ordered_color_at(
    x: u32,
    y: u32,
    color: NormalizedColor,
    candidates: &[CandidateColor],
    dither: Dither,
) -> ReducedColor {
    let (a, b) = nearest_two(color, candidates);
    let Some(b) = b else {
        return a.reduced;
    };
    let t = lerp(a.normalized, b.normalized, color);
    if t > ordered_threshold(dither, x, y) {
        b.reduced
    } else {
        a.reduced
    }
}

fn nearest(
    candidates: &[CandidateColor],
    color: NormalizedColor,
) -> &CandidateColor {
    let color = color.to_oklab();
    let mut best: Option<(&CandidateColor, f32)> = None;
    for candidate in candidates {
        let d = oklab_sqdist_hue_weighted(color, candidate.oklab, CHROMA_WEIGHT);
        if best.is_none_or(|(_, bd)| d < bd) {
            best = Some((candidate, d));
        }
    }
    best.unwrap().0
}

fn nearest_two(
    color: NormalizedColor,
    candidates: &[CandidateColor],
) -> (&CandidateColor, Option<&CandidateColor>) {
    let color = color.to_oklab();
    let mut best: Option<(&CandidateColor, f32)> = None;
    let mut second: Option<(&CandidateColor, f32)> = None;
    for candidate in candidates {
        let d1 = oklab_sqdist_hue_weighted(color, candidate.oklab, CHROMA_WEIGHT);
        if best.is_none_or(|(_, bd)| d1 < bd) {
            second = best;
            best = Some((candidate, d1));
        } else if second.is_none_or(|(_, d2)| d1 < d2) {
            second = Some((candidate, d1));
        }
    }
    (best.unwrap().0, second.map(|(c, _)| c))
}

fn lerp(
    c1: NormalizedColor,
    c2: NormalizedColor,
    color: NormalizedColor,
) -> f32 {
    let (dx, dy, dz) = (
        f32::from(c2.r) - f32::from(c1.r),
        f32::from(c2.g) - f32::from(c1.g),
        f32::from(c2.b) - f32::from(c1.b),
    );
    let len2 = dx * dx + dy * dy + dz * dz;
    if len2 == 0.0 {
        return 0.0;
    }
    let (px, py, pz) = (
        f32::from(color.r) - f32::from(c1.r),
        f32::from(color.g) - f32::from(c1.g),
        f32::from(color.b) - f32::from(c1.b),
    );
    ((px * dx + py * dy + pz * dz) / len2).clamp(0.0, 1.0)
}

fn clamp_u8(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}
