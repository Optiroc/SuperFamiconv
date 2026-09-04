//! Dithering implementation.

use clap::ValueEnum;

use crate::color::{CandidateColor, NormalizedColor, ReducedColor, oklab_sqdist_hue_weighted};
use crate::mode::Mode;
use crate::mode::color::ModeColor;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ValueEnum)]
#[value(rename_all = "snake_case")]
pub enum Dither {
    /// No dithering.
    Off,
    /// Ordered dithering between the two closest palette colors, using a 2x2 Bayer matrix.
    Bayer2x2,
    /// Ordered dithering between the two closest palette colors, using a 4x4 Bayer matrix.
    Bayer4x4,
    /// Atkinson error-diffusion dithering.
    Atkinson,
}

/// Chroma mismatch penalty when chosing dither candidates.
const CHROMA_WEIGHT: f32 = 3.0;

const BAYER_2X2_MATRIX: [[u8; 2]; 2] = [[0, 2], [3, 1]];
const BAYER_4X4_MATRIX: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
const ATKINSON_KERNEL: [(i32, i32); 6] = [(1, 0), (2, 0), (-1, 1), (0, 1), (1, 1), (0, 2)];

const fn bayer_threshold(
    dither: Dither,
    x: u32,
    y: u32,
) -> f32 {
    match dither {
        Dither::Bayer2x2 => (BAYER_2X2_MATRIX[(y & 1) as usize][(x & 1) as usize] as f32 + 0.5) / 4.0,
        Dither::Bayer4x4 => (BAYER_4X4_MATRIX[(y & 3) as usize][(x & 3) as usize] as f32 + 0.5) / 16.0,
        Dither::Off | Dither::Atkinson => panic!("bayer_threshold called for non-Bayer dither"),
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
        let error = if dither == Dither::Atkinson {
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
            Dither::Bayer2x2 | Dither::Bayer4x4 => ordered_color_at(x, y, color, candidates, self.dither),
            Dither::Atkinson => self.atkinson_color_at(x - self.origin_x, y - self.origin_y, color, candidates),
        }
    }

    fn atkinson_color_at(
        &mut self,
        local_x: u32,
        local_y: u32,
        color: NormalizedColor,
        candidates: &[CandidateColor],
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
        self.diffuse(local_x, local_y, residual);
        color.reduced
    }

    fn diffuse(
        &mut self,
        x: u32,
        y: u32,
        residual: [f32; 3],
    ) {
        for (dx, dy) in ATKINSON_KERNEL {
            let (nx, ny) = (x as i32 + dx, y as i32 + dy);
            if nx < 0 || ny < 0 || nx as u32 >= self.width || ny as u32 >= self.height {
                continue;
            }
            let index = (ny as u32 * self.width + nx as u32) as usize;
            for (c, &share) in residual.iter().enumerate() {
                self.error[index][c] += share / 8.0;
            }
        }
    }
}

/// Quantizes `width * height` pixels against `palette`, applying `dither`.
///
/// Returns `None` if the image is empty.
pub fn quantize_image(
    mode: Mode,
    palette: &[ReducedColor],
    width: u32,
    height: u32,
    dither: Dither,
    color_at: impl Fn(usize) -> NormalizedColor,
) -> (Vec<u8>, Vec<u8>) {
    let candidates: Vec<CandidateColor> = palette
        .iter()
        .map(|&r| CandidateColor::new(r, mode.normalize_color(r)))
        .collect();

    let size = (width * height) as usize;
    let mut indexed_data = vec![0u8; size];
    let mut data = vec![0u8; size * 4];
    let mut ditherer = Ditherer::new(dither, 0, 0, width, height);

    for i in 0..size {
        let nc = color_at(i);
        if mode.reduce_color(nc).is_transparent() {
            continue;
        }
        let x = (i as u32) % width;
        let y = (i as u32) / width;
        let chosen = ditherer.color_at(x, y, nc, &candidates);
        let index = palette.iter().position(|&c| c == chosen).unwrap();
        indexed_data[i] = index as u8;
        data[i * 4..i * 4 + 4].copy_from_slice(&mode.normalize_color(chosen).to_bytes());
    }

    (indexed_data, data)
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
    let t = lerp_t(a.normalized, b.normalized, color);
    if t > bayer_threshold(dither, x, y) {
        b.reduced
    } else {
        a.reduced
    }
}

fn lerp_t(
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
