//! Mode specific defaults, constraints and functions.

use clap::ValueEnum;

pub mod color;
pub mod map;
pub mod tile;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, ValueEnum)]
#[value(rename_all = "snake_case")]
pub enum Mode {
    /// Super Nintendo (Mode 0-6)
    Snes,
    /// Super Nintendo (Mode 7)
    SnesMode7,
    /// Game Boy
    Gb,
    /// Game Boy Color
    Gbc,
    /// Game Boy Advance
    Gba,
    /// Game Boy Advance (Affine BG)
    GbaAffine,
    /// Mega Drive
    Md,
    /// Master System
    Sms,
    /// Game Gear
    Gg,
    /// PC Engine
    Pce,
    /// PC Engine (Sprite)
    PceSprite,
    /// Neo Geo Pocket
    Ngp,
    /// Neo Geo Pocket Color
    Ngpc,
    /// WonderSwan
    Ws,
    /// WonderSwan Color
    Wsc,
    /// WonderSwan Color (Packed)
    WscPacked,
}

impl std::fmt::Display for Mode {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        let s = match self {
            Mode::Snes => "snes",
            Mode::SnesMode7 => "snes_mode7",
            Mode::Gb => "gb",
            Mode::Gbc => "gbc",
            Mode::Gba => "gba",
            Mode::GbaAffine => "gba_affine",
            Mode::Md => "md",
            Mode::Sms => "sms",
            Mode::Gg => "gg",
            Mode::Pce => "pce",
            Mode::PceSprite => "pce_sprite",
            Mode::Ngp => "ngp",
            Mode::Ngpc => "ngpc",
            Mode::Ws => "ws",
            Mode::Wsc => "wsc",
            Mode::WscPacked => "wsc_packed",
        };
        f.write_str(s)
    }
}

impl Mode {
    pub const fn default_bpp(self) -> u32 {
        use Mode::*;
        match self {
            Gb | Gbc | Ngp | Ngpc | Ws => 2,
            Snes | Gba | Md | Sms | Gg | Pce | PceSprite | Wsc | WscPacked => 4,
            SnesMode7 | GbaAffine => 8,
        }
    }

    pub fn available_bpp(self) -> Vec<u32> {
        use Mode::*;
        match self {
            Snes => vec![2, 4, 8],
            SnesMode7 => vec![8],
            Gb | Gbc => vec![1, 2],
            Gba => vec![4, 8],
            GbaAffine => vec![8],
            Md | Sms | Gg => vec![4],
            Pce | PceSprite => vec![4],
            Ngp => vec![1, 2],
            Ngpc => vec![2],
            Ws => vec![1, 2],
            Wsc => vec![2, 4],
            WscPacked => vec![4],
        }
    }

    pub fn bpp_is_allowed(
        self,
        bpp: u32,
    ) -> bool {
        self.available_bpp().contains(&bpp)
    }

    /*
    pub fn min_bpp_for_palette_size(
        self,
        size: u32,
    ) -> Option<u32> {
        for bpp in self.available_bpp() {
            if size <= palette_size_at_bpp(bpp) {
                return Some(bpp)
            }
        }
        None
    }
    */

    pub const fn default_tile_size(self) -> u32 {
        use Mode::*;
        match self {
            PceSprite => 16,
            _ => 8,
        }
    }

    pub const fn max_tile_count(self) -> u32 {
        use Mode::*;
        match self {
            SnesMode7 | Gb | GbaAffine => 256,
            Gbc | Ngpc | Sms | Gg | Ngp | Ws => 512,
            Snes | Gba | Wsc | WscPacked => 1024,
            Md | Pce | PceSprite => 2048,
        }
    }

    pub const fn tile_width_is_allowed(
        self,
        width: u32,
    ) -> bool {
        use Mode::*;
        match self {
            Snes => width == 8 || width == 16,
            PceSprite => width == 16,
            SnesMode7 | Gb | Gbc | Gba | GbaAffine | Md | Sms | Gg | Pce | Ngp | Ngpc | Ws | Wsc | WscPacked => {
                width == 8
            }
        }
    }

    pub const fn tile_height_is_allowed(
        self,
        height: u32,
    ) -> bool {
        use Mode::*;
        match self {
            Snes | Gb | Gbc => height == 8 || height == 16,
            PceSprite => height == 16,
            SnesMode7 | Gba | GbaAffine | Md | Sms | Gg | Pce | Ngp | Ngpc | Ws | Wsc | WscPacked => height == 8,
        }
    }

    pub const fn tile_flipping_is_allowed(self) -> bool {
        use Mode::*;
        match self {
            Snes | Gbc | Gba | Md | Ngp | Ngpc | Ws | Wsc | WscPacked => true,
            SnesMode7 | Gb | GbaAffine | Sms | Gg | Pce | PceSprite => false,
        }
    }

    pub const fn map_generation_is_supported(self) -> bool {
        use Mode::*;
        !matches!(self, PceSprite)
    }

    pub const fn default_map_size(self) -> Option<u32> {
        use Mode::*;
        match self {
            Snes | Gb | Gbc | Gba | GbaAffine | Md | Sms | Gg | Pce | Ngp | Ngpc | Ws | Wsc | WscPacked => Some(32),
            SnesMode7 => Some(128),
            PceSprite => None,
        }
    }

    pub const fn default_palette_count(self) -> u32 {
        use Mode::*;
        match self {
            SnesMode7 | Gb | GbaAffine => 1,
            Sms | Gg | Ngp => 2,
            Md => 4,
            Snes | Gbc => 8,
            Gba | Pce | PceSprite | Ngpc | Ws | Wsc | WscPacked => 16,
        }
    }

    pub const fn color_zero_is_shared(self) -> bool {
        use Mode::*;
        !matches!(self, Gb | Gbc | Sms | Gg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pce_sprite_requires_16x16_tiles() {
        assert_eq!(Mode::PceSprite.default_tile_size(), 16);
        assert!(Mode::PceSprite.tile_width_is_allowed(16));
        assert!(!Mode::PceSprite.tile_width_is_allowed(8));
        assert!(!Mode::PceSprite.tile_flipping_is_allowed());
    }

    #[test]
    fn bpp_constraints() {
        assert!(Mode::Snes.bpp_is_allowed(2));
        assert!(Mode::Snes.bpp_is_allowed(4));
        assert!(Mode::Snes.bpp_is_allowed(8));
        assert!(!Mode::Snes.bpp_is_allowed(1));

        assert!(Mode::SnesMode7.bpp_is_allowed(8));
        assert!(!Mode::SnesMode7.bpp_is_allowed(4));

        assert!(Mode::Ngpc.bpp_is_allowed(2));
        assert!(!Mode::Ngpc.bpp_is_allowed(4));
    }
}
