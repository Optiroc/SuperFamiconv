//! Mode-specific map conversion.

use super::{Mode, Mode::*};
use crate::map::Mapentry;

pub trait ModeMap {
    /// Size of one packed map entry.
    fn mapentry_size(&self) -> usize;

    /// Packs one map `entry` into mode-native bytes.
    fn pack_mapentry(
        &self,
        entry: Mapentry,
    ) -> Vec<u8>;

    /// Unpacks one map entry from mode-native `bytes`.
    fn unpack_mapentry(
        &self,
        bytes: &[u8],
    ) -> Mapentry;
}

impl ModeMap for Mode {
    fn mapentry_size(&self) -> usize {
        match self {
            SnesMode7 | Gb | GbaAffine => 1,
            PceSprite => 0,
            Snes | Sms | Gg | Gbc | Gba | Md | Pce | Ws | Wsc | WscPacked | Ngp | Ngpc => 2,
        }
    }

    fn pack_mapentry(
        &self,
        entry: Mapentry,
    ) -> Vec<u8> {
        let t = entry.tile_index;
        let p = entry.palette_index;
        let h = u32::from(entry.flip_h);
        let v = u32::from(entry.flip_v);

        match self {
            Snes => vec![
                (t & 0xff) as u8,
                (((t >> 8) & 0x03) | ((p << 2) & 0x1c) | (h << 6) | (v << 7)) as u8,
            ],
            SnesMode7 | Gb | GbaAffine => vec![(t & 0xff) as u8],
            Sms | Gg => vec![
                (t & 0xff) as u8,
                (((t >> 8) & 0x01) | (h << 1) | (v << 2) | ((p << 3) & 0x8)) as u8,
            ],
            Gbc => vec![
                (t & 0xff) as u8,
                ((p & 0x07) | ((t >> 5) & 0x08) | (h << 5) | (v << 6)) as u8,
            ],
            Gba => vec![
                (t & 0xff) as u8,
                (((t >> 8) & 0x03) | (h << 2) | (v << 3) | ((p << 4) & 0xf0)) as u8,
            ],
            Md => vec![
                (t & 0xff) as u8,
                (((t >> 8) & 0x07) | (h << 3) | (v << 4) | ((p << 5) & 0x60)) as u8,
            ],
            Pce => vec![(t & 0xff) as u8, (((t >> 8) & 0x0f) | ((p << 4) & 0xf0)) as u8],
            Ws | Wsc | WscPacked => vec![
                (t & 0xff) as u8,
                (((t >> 8) & 0x01) | ((p << 1) & 0x1e) | ((t >> 4) & 0x20) | (h << 6) | (v << 7)) as u8,
            ],
            Ngp => vec![
                (t & 0xff) as u8,
                (((t >> 8) & 0x01) | ((p << 5) & 0x20) | (v << 6) | (h << 7)) as u8,
            ],
            Ngpc => vec![
                (t & 0xff) as u8,
                (((t >> 8) & 0x01) | ((p << 1) & 0x1e) | (v << 6) | (h << 7)) as u8,
            ],
            PceSprite => Vec::new(),
        }
    }

    fn unpack_mapentry(
        &self,
        bytes: &[u8],
    ) -> Mapentry {
        let b0 = bytes.first().copied().map_or(0, u32::from);
        let b1 = bytes.get(1).copied().map_or(0, u32::from);

        match self {
            Snes => Mapentry::new(
                b0 | ((b1 & 0x03) << 8),
                (b1 >> 2) & 0x07,
                (b1 >> 6) & 1 == 1,
                (b1 >> 7) & 1 == 1,
            ),
            SnesMode7 | Gb | GbaAffine => Mapentry::new(b0, 0, false, false),
            Sms | Gg => Mapentry::new(
                b0 | ((b1 & 0x01) << 8),
                (b1 >> 3) & 0x01,
                (b1 >> 1) & 1 == 1,
                (b1 >> 2) & 1 == 1,
            ),
            Gbc => Mapentry::new(
                b0 | ((b1 & 0x08) << 5),
                b1 & 0x07,
                (b1 >> 5) & 1 == 1,
                (b1 >> 6) & 1 == 1,
            ),
            Gba => Mapentry::new(
                b0 | ((b1 & 0x03) << 8),
                (b1 >> 4) & 0x0f,
                (b1 >> 2) & 1 == 1,
                (b1 >> 3) & 1 == 1,
            ),
            Md => Mapentry::new(
                b0 | ((b1 & 0x07) << 8),
                (b1 >> 5) & 0x03,
                (b1 >> 3) & 1 == 1,
                (b1 >> 4) & 1 == 1,
            ),
            Pce => Mapentry::new(b0 | ((b1 & 0x0f) << 8), (b1 >> 4) & 0x0f, false, false),
            Ws | Wsc | WscPacked => Mapentry::new(
                b0 | ((b1 & 0x01) << 8) | ((b1 & 0x20) << 4),
                (b1 >> 1) & 0x0f,
                (b1 >> 6) & 1 == 1,
                (b1 >> 7) & 1 == 1,
            ),
            Ngp => Mapentry::new(
                b0 | ((b1 & 0x01) << 8),
                (b1 >> 5) & 0x01,
                (b1 >> 7) & 1 == 1,
                (b1 >> 6) & 1 == 1,
            ),
            Ngpc => Mapentry::new(
                b0 | ((b1 & 0x01) << 8),
                (b1 >> 1) & 0x0f,
                (b1 >> 7) & 1 == 1,
                (b1 >> 6) & 1 == 1,
            ),
            PceSprite => Mapentry::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_mapentry_snes() {
        let e = Mapentry::new(0x01ab, 3, true, false);
        assert_eq!(Mode::Snes.pack_mapentry(e), vec![0xab, 0x01 | (3 << 2) | (1 << 6)]);
    }

    #[test]
    fn pack_mapentry_gbc() {
        let e = Mapentry::new(0x0120, 5, false, true);
        let t: u32 = 0x120;
        assert_eq!(
            Mode::Gbc.pack_mapentry(e),
            vec![0x20, (5 | ((t >> 5) & 0x08) | (1 << 6)) as u8]
        );
    }

    #[test]
    fn pack_mapentry_gba() {
        let e = Mapentry::new(0x0321, 0xa, true, true);
        assert_eq!(
            Mode::Gba.pack_mapentry(e),
            vec![0x21, (0x03) | (1 << 2) | (1 << 3) | ((0xa << 4) & 0xf0)]
        );
    }

    #[test]
    fn pack_mapentry_md() {
        let e = Mapentry::new(0x01f0, 2, true, false);
        assert_eq!(
            Mode::Md.pack_mapentry(e),
            vec![0xf0, ((0x01f0 >> 8) & 0x07) as u8 | (1 << 3) | ((2 << 5) & 0x60)]
        );
    }

    #[test]
    fn pack_mapentry_pce() {
        let e = Mapentry::new(0x02ab, 4, false, false);
        assert_eq!(
            Mode::Pce.pack_mapentry(e),
            vec![0xab, ((0x02ab >> 8) & 0x0f) as u8 | ((4 << 4) & 0xf0)]
        );
    }

    #[test]
    fn pack_mapentry_wsc() {
        let e = Mapentry::new(0x31, 6, true, true);
        assert_eq!(
            Mode::Wsc.pack_mapentry(e),
            vec![0x31, ((6 << 1) & 0x1e) | (1 << 6) | (1 << 7)]
        );
    }

    #[test]
    fn pack_unpack_roundtrip() {
        // (mode, tile_index bits, palette_index bits, supports flip)
        let tests: &[(Mode, u32, u32, bool)] = &[
            (Snes, 10, 3, true),
            (SnesMode7, 8, 0, false),
            (Gb, 8, 0, false),
            (GbaAffine, 8, 0, false),
            (Sms, 9, 1, true),
            (Gg, 9, 1, true),
            (Gbc, 9, 3, true),
            (Gba, 10, 4, true),
            (Md, 11, 2, true),
            (Pce, 12, 4, false),
            (Ws, 10, 4, true),
            (Wsc, 10, 4, true),
            (WscPacked, 10, 4, true),
            (Ngp, 9, 1, true),
            (Ngpc, 9, 4, true),
        ];

        for &(mode, tile_bits, palette_bits, supports_flip) in tests {
            let tile_max = (1u32 << tile_bits) - 1;
            let palette_max = (1u32 << palette_bits) - 1;
            let flips: &[(bool, bool)] = if supports_flip {
                &[(false, false), (true, false), (false, true), (true, true)]
            } else {
                &[(false, false)]
            };

            for tile_index in [0, tile_max] {
                for palette_index in [0, palette_max] {
                    for &(flip_h, flip_v) in flips {
                        let entry = Mapentry::new(tile_index, palette_index, flip_h, flip_v);
                        let packed = mode.pack_mapentry(entry);
                        assert_eq!(
                            mode.unpack_mapentry(&packed),
                            entry,
                            "{mode} tile={tile_index:#x} palette={palette_index} h={flip_h} v={flip_v}"
                        );
                    }
                }
            }
        }
    }
}
