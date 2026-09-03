//! Mode-specific color transformations and palette conversion.

use super::{Mode, Mode::*};
use crate::color::{NormalizedColor, ReducedColor};

pub trait ModeColor {
    /// Scales a normalized color down to `mode`'s native range.
    /// - Colors with alpha below 0x80 become fully transparent for modes that support it.
    /// - TODO: Add option (or default to?) nearest color rather than truncation?
    fn reduce_color(
        &self,
        color: NormalizedColor,
    ) -> ReducedColor;

    /// Scales a `mode`-native color to normalized range.
    fn normalize_color(
        &self,
        color: ReducedColor,
    ) -> NormalizedColor;

    /// Reduces `color` to `mode`'s native range and back, snapping it to the nearest
    /// value `mode` can represent.
    fn quantize_color(
        &self,
        color: NormalizedColor,
    ) -> NormalizedColor;

    /// Packs `color` into mode-native representation.
    fn pack_color(
        &self,
        color: ReducedColor,
    ) -> Vec<u8>;

    /// Packs `colors` into mode-native representation.
    fn pack_colors(
        &self,
        colors: &[ReducedColor],
    ) -> Result<Vec<u8>, String>;

    /// Unpacks native format palette to reduced-space colors.
    fn unpack_colors(
        &self,
        data: &[u8],
    ) -> Result<Vec<ReducedColor>, String>;
}

impl ModeColor for Mode {
    fn reduce_color(
        &self,
        color: NormalizedColor,
    ) -> ReducedColor {
        match self {
            Snes | SnesMode7 | Gbc | Gba | GbaAffine => match color.a {
                0x00..0x80 => ReducedColor::TRANSPARENT,
                _ => ReducedColor::new(color.r >> 3, color.g >> 3, color.b >> 3, 0xff),
            },
            Gb => {
                let c = opaque_threshold(color);
                let l = c.luma_u8();
                let gray = match l {
                    0x00..0x40 => 0,
                    0x40..0x80 => 1,
                    0x80..0xc0 => 2,
                    _ => 3,
                };
                ReducedColor::new(gray, gray, gray, 0xff)
            }
            Ngp | Ws => {
                // WonderSwan technically supports 8 out of 16 gray shades with
                // it's palette indirection, but we just treat it as NGP.
                let c = opaque_threshold(color);
                let gray = c.luma_u8() >> 5;
                ReducedColor::new(gray, gray, gray, 0xff)
            }
            Md | Pce | PceSprite => {
                if color.a < 0x80 {
                    ReducedColor::TRANSPARENT
                } else {
                    ReducedColor::new(color.r >> 5, color.g >> 5, color.b >> 5, 0xff)
                }
            }
            Sms => {
                let c = opaque_threshold(color);
                ReducedColor::new(c.r >> 6, c.g >> 6, c.b >> 6, 0xff)
            }
            Ngpc | Gg | Wsc | WscPacked => {
                if color.a < 0x80 {
                    ReducedColor::TRANSPARENT
                } else {
                    ReducedColor::new(color.r >> 4, color.g >> 4, color.b >> 4, 0xff)
                }
            }
        }
    }

    fn normalize_color(
        &self,
        color: ReducedColor,
    ) -> NormalizedColor {
        let shift = match self {
            Snes | SnesMode7 | Gbc | Gba | GbaAffine => 3,
            Gb | Sms => 6,
            Wsc | WscPacked | Ngpc | Gg => 4,
            Md | Pce | PceSprite | Ws | Ngp => 5,
        };
        NormalizedColor::new(
            scale_up(color.r, shift),
            scale_up(color.g, shift),
            scale_up(color.b, shift),
            scale_up(color.a, shift),
        )
    }

    fn quantize_color(
        &self,
        color: NormalizedColor,
    ) -> NormalizedColor {
        self.normalize_color(self.reduce_color(color))
    }

    fn pack_color(
        &self,
        color: ReducedColor,
    ) -> Vec<u8> {
        let color = u32::from_le_bytes(color.to_bytes());
        match self {
            Snes | SnesMode7 | Gbc | Gba | GbaAffine => {
                vec![
                    ((color & 0x1f) | ((color >> 3) & 0xe0)) as u8,
                    (((color >> 11) & 0x03) | ((color >> 14) & 0x7c)) as u8,
                ]
            }
            Gb => vec![((0xffu32.wrapping_sub(color & 0x3)) & 0x3) as u8],
            Md => vec![
                (((color << 1) & 0x0e) | ((color >> 3) & 0xe0)) as u8,
                ((color >> 15) & 0x0e) as u8,
            ],
            Pce | PceSprite => {
                vec![
                    (((color >> 16) & 0x07) | ((color << 3) & 0x38) | ((color >> 2) & 0xc0)) as u8,
                    ((color >> 10) & 0x01) as u8,
                ]
            }
            Ws | Ngp => vec![(color ^ 0x07) as u8],
            Wsc | Gg | WscPacked => vec![
                (((color >> 16) & 0x0f) | ((color >> 4) & 0xf0)) as u8,
                (color & 0x0f) as u8,
            ],
            Ngpc => vec![
                ((color & 0x0f) | ((color >> 4) & 0xf0)) as u8,
                ((color >> 16) & 0x0f) as u8,
            ],
            Sms => vec![(((color >> 12) & 0x30) | ((color >> 6) & 0x0c) | (color & 3)) as u8],
        }
    }

    fn pack_colors(
        &self,
        colors: &[ReducedColor],
    ) -> Result<Vec<u8>, String> {
        match self {
            Mode::Gb => {
                let [c0, c1, c2, c3] = *colors else {
                    return Err("gb palette size not equal to 4".into());
                };
                let packed = self.pack_color(c0)[0]
                    | (self.pack_color(c1)[0] << 2)
                    | (self.pack_color(c2)[0] << 4)
                    | (self.pack_color(c3)[0] << 6);
                Ok(vec![packed])
            }
            Mode::Ws => {
                let [c0, c1, c2, c3] = *colors else {
                    return Err("ws palette size not equal to 4".into());
                };
                let packed: u16 = u16::from(self.pack_color(c0)[0])
                    | (u16::from(self.pack_color(c1)[0]) << 4)
                    | (u16::from(self.pack_color(c2)[0]) << 8)
                    | (u16::from(self.pack_color(c3)[0]) << 12);
                Ok(vec![(packed & 0xff) as u8, (packed >> 8) as u8])
            }
            _ => Ok(colors.iter().flat_map(|&c| self.pack_color(c)).collect()),
        }
    }

    fn unpack_colors(
        &self,
        data: &[u8],
    ) -> Result<Vec<ReducedColor>, String> {
        let mut v = Vec::new();
        match self {
            Snes | SnesMode7 | Gbc | Gba | GbaAffine => {
                if !data.len().is_multiple_of(2) {
                    return Err("Native palette size not a multiple of 2".into());
                }
                for chunk in data.chunks_exact(2) {
                    let cw = u16::from_le_bytes([chunk[0], chunk[1]]);
                    let (r, g, b) = ((cw & 0x1f) as u8, ((cw >> 5) & 0x1f) as u8, ((cw >> 10) & 0x1f) as u8);
                    v.push(ReducedColor::new(r, g, b, 0xff));
                }
            }
            Sms => {
                for &byte in data {
                    let (r, g, b) = (byte & 0x3, (byte >> 2) & 0x3, (byte >> 4) & 0x3);
                    v.push(ReducedColor::new(r, g, b, 0xff));
                }
            }
            Gb => {
                if data.len() != 1 {
                    return Err("Native palette size not 1 byte".into());
                }
                for i in 0..4u32 {
                    let gray = 3 - ((data[0] >> (i * 2)) & 0x3);
                    v.push(ReducedColor::new(gray, gray, gray, 0xff));
                }
            }
            Gg | Wsc | WscPacked => {
                if !data.len().is_multiple_of(2) {
                    return Err("Native palette size not a multiple of 2".into());
                }
                for chunk in data.chunks_exact(2) {
                    let cw = u16::from_le_bytes([chunk[0], chunk[1]]);
                    let (r, g, b) = (((cw >> 8) & 0xf) as u8, ((cw >> 4) & 0xf) as u8, (cw & 0xf) as u8);
                    v.push(ReducedColor::new(r, g, b, 0xff));
                }
            }
            Md => {
                if !data.len().is_multiple_of(2) {
                    return Err("Native palette size not a multiple of 2".into());
                }
                for chunk in data.chunks_exact(2) {
                    let cw = u16::from_le_bytes([chunk[0], chunk[1]]);
                    let (r, g, b) = (
                        ((cw >> 1) & 0x7) as u8,
                        ((cw >> 5) & 0x7) as u8,
                        ((cw >> 9) & 0x7) as u8,
                    );
                    v.push(ReducedColor::new(r, g, b, 0xff));
                }
            }
            Pce | PceSprite => {
                if !data.len().is_multiple_of(2) {
                    return Err("Native palette size not a multiple of 2".into());
                }
                for chunk in data.chunks_exact(2) {
                    let cw = u16::from_le_bytes([chunk[0], chunk[1]]);
                    let (r, g, b) = (((cw >> 3) & 0x7) as u8, ((cw >> 6) & 0x7) as u8, (cw & 0x7) as u8);
                    v.push(ReducedColor::new(r, g, b, 0xff));
                }
            }
            Ws => {
                if data.len() != 2 {
                    return Err("Native palette size not 2 bytes".into());
                }
                for i in 0..4usize {
                    let gray = ((data[i >> 1] >> ((i & 1) * 4)) & 0x7) ^ 0x7;
                    v.push(ReducedColor::new(gray, gray, gray, 0xff));
                }
            }
            Ngp => {
                if data.len() != 4 {
                    return Err("Native palette size not 4 bytes".into());
                }
                for &byte in data {
                    let gray = (byte & 0x7) ^ 0x7;
                    v.push(ReducedColor::new(gray, gray, gray, 0xff));
                }
            }
            Ngpc => {
                if !data.len().is_multiple_of(2) {
                    return Err("Native palette size not a multiple of 2".into());
                }
                for chunk in data.chunks_exact(2) {
                    let cw = u16::from_le_bytes([chunk[0], chunk[1]]);
                    let (r, g, b) = ((cw & 0xf) as u8, ((cw >> 4) & 0xf) as u8, ((cw >> 8) & 0xf) as u8);
                    v.push(ReducedColor::new(r, g, b, 0xff));
                }
            }
        }
        Ok(v)
    }
}

fn opaque_threshold(color: NormalizedColor) -> NormalizedColor {
    if color.a < 0x80 {
        NormalizedColor::TRANSPARENT
    } else {
        color
    }
}

/// Scales up a value using left-bit replication.
const fn scale_up(
    value: u8,
    shift: u32,
) -> u8 {
    let bits = 8 - shift;
    let mut v = value as u32;
    let mut n = bits;
    while n < 8 {
        v |= v << n;
        n *= 2;
    }
    (v >> (n - 8)) as u8
}

#[cfg(test)]
mod tests {
    use clap::ValueEnum;

    use super::*;

    fn n(
        r: u8,
        g: u8,
        b: u8,
        a: u8,
    ) -> NormalizedColor {
        NormalizedColor::new(r, g, b, a)
    }

    fn r(
        r: u8,
        g: u8,
        b: u8,
        a: u8,
    ) -> ReducedColor {
        ReducedColor::new(r, g, b, a)
    }

    #[test]
    fn left_bit_replication() {
        assert_eq!(scale_up(0b0_0000001, 1), 0b0000001_0);
        assert_eq!(scale_up(0b0_1000001, 1), 0b1000001_1);
        assert_eq!(scale_up(0b00_100001, 2), 0b100001_10);
        assert_eq!(scale_up(0b000_00001, 3), 0b00001_000);
        assert_eq!(scale_up(0b000_11011, 3), 0b11011_110);
        assert_eq!(scale_up(0b000_10111, 3), 0b10111_101);
        assert_eq!(scale_up(0b0000_0001, 4), 0b0001_0001);
        assert_eq!(scale_up(0b00000_001, 5), 0b001_00100);
        assert_eq!(scale_up(0b000000_01, 6), 0b01_010101);
        assert_eq!(scale_up(0b0000000_1, 7), 0b1_1111111);
        assert_eq!(scale_up(0b0000000_0, 7), 0b0_0000000);
    }

    #[test]
    fn reduce_snes_white_roundtrip() {
        let white = n(255, 255, 255, 255);
        let reduced = Mode::Snes.reduce_color(white);
        assert_eq!(reduced, r(31, 31, 31, 0xff));
        assert_eq!(Mode::Snes.normalize_color(reduced), white);
    }

    #[test]
    fn reduce_white_black_roundtrip() {
        for mode in Mode::value_variants() {
            let white = n(255, 255, 255, 255);
            let black = n(0, 0, 0, 255);
            assert_eq!(
                mode.normalize_color(mode.reduce_color(white)),
                white,
                "reduce_white_black_roundtrip white failed for mode '{mode}'"
            );
            assert_eq!(
                mode.normalize_color(mode.reduce_color(black)),
                black,
                "reduce_white_black_roundtrip black failed for mode '{mode}'"
            );
        }
    }

    #[test]
    fn reduce_transparent() {
        assert!(Mode::Snes.reduce_color(n(255, 255, 255, 0x7f)).is_transparent());
        assert!(Mode::Md.reduce_color(n(255, 255, 255, 0x7f)).is_transparent());
        assert!(Mode::Wsc.reduce_color(n(255, 255, 255, 0x7f)).is_transparent());
        // gb has no shared transparent index; low-alpha pixels become opaque black
        assert_eq!(Mode::Gb.reduce_color(n(255, 255, 255, 0x7f)), r(0, 0, 0, 0xff));
    }

    #[test]
    fn reduce_gb_levels() {
        assert_eq!(Mode::Gb.reduce_color(n(0, 0, 0, 255)), r(0, 0, 0, 0xff));
        assert_eq!(Mode::Gb.reduce_color(n(255, 255, 255, 255)), r(3, 3, 3, 0xff));
    }

    #[test]
    fn pack_color_snes_white() {
        let packed = Mode::Snes.pack_color(Mode::Snes.reduce_color(n(255, 255, 255, 255)));
        assert_eq!(packed.len(), 2);
        assert_eq!(u16::from_le_bytes([packed[0], packed[1]]), 0x7fff);
    }

    #[test]
    fn pack_colors_gb() {
        let colors = [r(0, 0, 0, 0xff), r(1, 1, 1, 0xff), r(2, 2, 2, 0xff), r(3, 3, 3, 0xff)];
        let packed = Mode::Gb.pack_colors(&colors).unwrap();
        assert_eq!(packed.len(), 1);
        assert_eq!(packed[0], 0b00011011);
    }

    #[test]
    fn pack_colors_ws() {
        let colors = [r(0, 0, 0, 0xff), r(1, 1, 1, 0xff), r(2, 2, 2, 0xff), r(3, 3, 3, 0xff)];
        let packed = Mode::Ws.pack_colors(&colors).unwrap();
        assert_eq!(packed.len(), 2);
        assert_eq!(u16::from_le_bytes([packed[0], packed[1]]), 0x4567);
    }

    #[test]
    fn pack_colors_roundtrip() {
        use Mode::*;
        for mode in Mode::value_variants() {
            let colors: Vec<ReducedColor> = match mode {
                Gb => vec![r(0, 0, 0, 0xff), r(1, 1, 1, 0xff), r(2, 2, 2, 0xff), r(3, 3, 3, 0xff)],
                Ws => vec![r(0, 0, 0, 0xff), r(2, 2, 2, 0xff), r(4, 4, 4, 0xff), r(7, 7, 7, 0xff)],
                Ngp => vec![r(0, 0, 0, 0xff), r(2, 2, 2, 0xff), r(4, 4, 4, 0xff), r(7, 7, 7, 0xff)],
                Snes | SnesMode7 | Gbc | Gba | GbaAffine => {
                    vec![r(3, 17, 29, 0xff), r(31, 0, 12, 0xff)]
                }
                Sms => vec![r(1, 2, 3, 0xff), r(3, 0, 2, 0xff)],
                Md | Pce | PceSprite => vec![r(1, 5, 6, 0xff), r(7, 2, 0, 0xff)],
                Gg | Wsc | WscPacked | Ngpc => vec![r(1, 9, 14, 0xff), r(15, 0, 5, 0xff)],
            };
            let packed = mode.pack_colors(&colors).unwrap();
            let unpacked = mode.unpack_colors(&packed).unwrap();
            assert_eq!(unpacked, colors, "pack_colors_roundtrip failed for {mode}");
        }
    }
}
