//! Color representations.

use std::ops::Deref;

use quantette::color_space::srgb8_to_oklab;
use quantette::deps::palette::{Oklab, Srgb};

/// A raw 8bpc RGBA color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Rgba8888 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba8888 {
    pub const TRANSPARENT: Rgba8888 = Rgba8888 { r: 0, g: 0, b: 0, a: 0 };

    pub fn new(
        r: u8,
        g: u8,
        b: u8,
        a: u8,
    ) -> Self {
        Rgba8888 { r, g, b, a }
    }

    pub fn from_bytes(bytes: [u8; 4]) -> Self {
        Rgba8888::new(bytes[0], bytes[1], bytes[2], bytes[3])
    }

    pub fn to_bytes(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }

    pub fn is_transparent(self) -> bool {
        self.a == Self::TRANSPARENT.a
    }
}

/// A full precision color.
///
/// Used prior to mode-specific reduction or when scaled up for preview images.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct NormalizedColor(pub Rgba8888);

impl NormalizedColor {
    pub fn to_hexstring(
        self,
        alpha: bool,
    ) -> String {
        if alpha {
            format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
        } else {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        }
    }

    /// Rec.709/sRGB luma in range 0..=1.
    pub fn luma_f32(self) -> f32 {
        let r = f32::from(self.r) / f32::from(u8::MAX);
        let g = f32::from(self.g) / f32::from(u8::MAX);
        let b = f32::from(self.b) / f32::from(u8::MAX);
        r * 0.2126 + g * 0.7152 + b * 0.0722
    }

    /// Rec.709/sRGB luma in u8 range.
    pub fn luma_u8(self) -> u8 {
        let min = f32::from(u8::MIN);
        let max = f32::from(u8::MAX);
        (self.luma_f32() * max).clamp(min, max) as u8
    }

    pub fn to_oklab(self) -> Oklab {
        srgb8_to_oklab(&[Srgb::new(self.r, self.g, self.b)])[0]
    }
}

/// A color reduced to a target `Mode`'s native precision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ReducedColor(pub Rgba8888);

/// A nearest-color match candidate, pairing a mode-reduced color with its full precision
/// normalized form, and a precomputed `Oklab` representation for distance comparison.
#[derive(Debug, Clone, Copy)]
pub struct CandidateColor {
    pub reduced: ReducedColor,
    pub normalized: NormalizedColor,
    pub oklab: Oklab,
}

impl CandidateColor {
    pub fn new(
        reduced: ReducedColor,
        normalized: NormalizedColor,
    ) -> Self {
        CandidateColor {
            reduced,
            normalized,
            oklab: normalized.to_oklab(),
        }
    }
}

macro_rules! rgba8888_newtype {
    ($name:ident) => {
        impl $name {
            pub const TRANSPARENT: $name = $name(Rgba8888::TRANSPARENT);

            pub fn new(
                r: u8,
                g: u8,
                b: u8,
                a: u8,
            ) -> Self {
                $name(Rgba8888::new(r, g, b, a))
            }
        }

        impl Deref for $name {
            type Target = Rgba8888;

            fn deref(&self) -> &Rgba8888 {
                &self.0
            }
        }
    };
}

rgba8888_newtype!(NormalizedColor);
rgba8888_newtype!(ReducedColor);

/// Squared distance between two `Oklab` colors.
pub fn oklab_sqdist(
    a: Oklab,
    b: Oklab,
) -> f32 {
    let dl = a.l - b.l;
    let da = a.a - b.a;
    let db = a.b - b.b;
    dl * dl + da * da + db * db
}

/// Squared distance between two `Oklab` colors, with `chroma_weight` factor.
pub fn oklab_sqdist_hue_weighted(
    a: Oklab,
    b: Oklab,
    chroma_weight: f32,
) -> f32 {
    let dl = a.l - b.l;
    let da = a.a - b.a;
    let db = a.b - b.b;
    dl * dl + chroma_weight * (da * da + db * db)
}

/// Summed distance from `colors` to their nearest entry in `candidates`.
pub fn summed_distance(
    colors: impl IntoIterator<Item = NormalizedColor>,
    candidates: &[CandidateColor],
) -> f32 {
    colors
        .into_iter()
        .map(|color| {
            let color = color.to_oklab();
            candidates
                .iter()
                .map(|c| oklab_sqdist(color, c.oklab))
                .reduce(f32::min)
                .unwrap_or(0.0)
        })
        .sum()
}

/// Parses a CSS-style hex color (`#rrggbb` or `#rrggbbaa`) into a full precision color.
/// - A 6-character string is given full alpha (`0xff`).
/// - Quotation marks (`'`, `"`) and number signs (`#`) are stripped.
pub fn from_hexstring(s: &str) -> Result<NormalizedColor, String> {
    let stripped: String = s.chars().filter(|c| !matches!(c, '#' | '"' | '\'')).collect();
    let full = match stripped.len() {
        6 => format!("{stripped}ff"),
        8 => stripped,
        _ => {
            return Err(format!("Color '{s}' is not a 6 or 8 character hex-string"));
        }
    };
    let value = u32::from_str_radix(&full, 16).map_err(|_| format!("Color '{s}' could not be parsed"))?;
    let [r, g, b, a] = value.to_be_bytes();
    Ok(NormalizedColor::new(r, g, b, a))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hexstring_alpha() {
        assert_eq!(
            from_hexstring("#ff8000").unwrap(),
            NormalizedColor::new(0xff, 0x80, 0x00, 0xff)
        );
        assert_eq!(
            from_hexstring("ff800080").unwrap(),
            NormalizedColor::new(0xff, 0x80, 0x00, 0x80)
        );
    }

    #[test]
    fn hexstring_strip() {
        let a = from_hexstring("#505050").unwrap();
        let b = from_hexstring("\"505050\"").unwrap();
        let c = from_hexstring("'505050'").unwrap();
        assert_eq!(a, b);
        assert_eq!(b, c);
    }

    #[test]
    fn hexstring_reject() {
        assert!(from_hexstring("fff").is_err());
        assert!(from_hexstring("ff80008012").is_err());
        assert!(from_hexstring("zzzzzz").is_err());
    }

    #[test]
    fn hexstring_roundtrip() {
        let c = from_hexstring("#a1b2c3d4").unwrap();
        assert_eq!(c.to_hexstring(true), "#a1b2c3d4");
        assert_eq!(c.to_hexstring(false), "#a1b2c3");
    }

    #[test]
    fn is_transparent() {
        assert!(Rgba8888::TRANSPARENT.is_transparent());
        assert!(!Rgba8888::new(0, 0, 0, 1).is_transparent());
        assert!(NormalizedColor::TRANSPARENT.is_transparent());
        assert!(ReducedColor::TRANSPARENT.is_transparent());
    }
}
