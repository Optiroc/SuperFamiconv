//! Mode-specific tile conversion.

use super::{Mode, Mode::*};

pub trait ModeTile {
    /// Packs one tile's pixel indices into mode-native representation.
    fn pack_tile(
        &self,
        data: &[u8],
        bpp: u32,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>, String>;

    /// Unpacks one tile's mode-native format data to pixel indices.
    fn unpack_tile(
        &self,
        data: &[u8],
        bpp: u32,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>, String>;
}

impl ModeTile for Mode {
    fn pack_tile(
        &self,
        data: &[u8],
        bpp: u32,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>, String> {
        // Regular bit planes
        fn make_1bit_planes(
            data: &[u8],
            plane: u32,
            reverse: bool,
        ) -> Vec<u8> {
            assert!(
                data.len().is_multiple_of(8),
                "tile data not a multiple of 8 in make_1bit_planes"
            );
            let mask: u8 = 1 << plane;
            data.chunks_exact(8)
                .map(|row| {
                    let mut byte = 0u8;
                    for (b, &v) in row.iter().enumerate() {
                        if v & mask != 0 {
                            byte |= 1 << (if reverse { 7 - b } else { b });
                        }
                    }
                    byte
                })
                .collect()
        }

        // snes/gb style bit planes
        fn make_2bit_planes(
            data: &[u8],
            plane_index: u32,
        ) -> Vec<u8> {
            assert!(
                data.len().is_multiple_of(8),
                "tile data not a multiple of 8 in make_2bit_planes"
            );
            let mut p = vec![0u8; 16];
            if data.is_empty() {
                return p;
            }
            let mask0: u8 = 1 << plane_index;
            let mask1: u8 = mask0 << 1;
            for y in 0..8 {
                for x in 0..8 {
                    let v = data[y * 8 + x];
                    p[y * 2] |= ((v & mask0) >> plane_index) << (7 - x);
                    p[y * 2 + 1] |= ((v & mask1) >> (plane_index + 1)) << (7 - x);
                }
            }
            p
        }

        // wsc/sms/gg planar style bit planes
        fn make_4bit_planes(
            data: &[u8],
            plane_index: u32,
        ) -> Vec<u8> {
            assert!(
                data.len().is_multiple_of(4),
                "tile data not a multiple of 4 in make_2bpp_bitpack"
            );
            let mut p = vec![0u8; 32];
            if data.is_empty() {
                return p;
            }
            let masks: [u8; 4] = [1 << plane_index, 2 << plane_index, 4 << plane_index, 8 << plane_index];
            for y in 0..8 {
                for x in 0..8 {
                    let v = data[y * 8 + x];
                    for (plane, &mask) in masks.iter().enumerate() {
                        p[y * 4 + plane] |= ((v & mask) >> (plane_index + plane as u32)) << (7 - x);
                    }
                }
            }
            p
        }

        // ngp/vb style 4 pixels per byte data
        fn make_2bpp_bitpack(
            data: &[u8],
            reverse: bool,
        ) -> Vec<u8> {
            let mut p = vec![0u8; 16];
            if data.is_empty() {
                return p;
            }
            for y in 0..8 {
                for x in 0..8 {
                    let px = if reverse { 7 - x } else { x };
                    p[(y << 1) | (px >> 2)] |= (data[y * 8 + x] & 0x03) << ((px << 1) & 6);
                }
            }
            p
        }

        // gba/md style 2 pixels per byte data
        fn make_4bpp_bitpack(
            data: &[u8],
            endian_swap: bool,
        ) -> Vec<u8> {
            data.chunks_exact(2)
                .map(|px| {
                    if endian_swap {
                        (0x0f & px[1]) | (0xf0 & (px[0] << 4))
                    } else {
                        (0x0f & px[0]) | (0xf0 & (px[1] << 4))
                    }
                })
                .collect()
        }

        let require_8x8 = |mode: Mode| -> Result<(), String> {
            if width != 8 || height != 8 {
                Err(format!("Tile size must be 8x8 for mode '{mode}'"))
            } else {
                Ok(())
            }
        };

        match self {
            Snes | Gb | Gbc | Pce => {
                require_8x8(*self)?;
                let mut nd = Vec::new();
                if bpp == 1 {
                    nd.extend(make_1bit_planes(data, 0, true));
                } else {
                    for i in 0..bpp / 2 {
                        nd.extend(make_2bit_planes(data, i * 2));
                    }
                }
                Ok(nd)
            }
            Ws | Wsc | Gg | Sms => {
                require_8x8(*self)?;
                match bpp {
                    4 => Ok(make_4bit_planes(data, 0)),
                    2 => Ok(make_2bit_planes(data, 0)),
                    _ => unreachable!(),
                }
            }
            Ngp | Ngpc => {
                require_8x8(*self)?;
                match bpp {
                    2 => Ok(make_2bpp_bitpack(data, true)),
                    _ => unreachable!(),
                }
            }
            SnesMode7 => Ok(data.to_vec()),
            Gba | GbaAffine | Md | WscPacked => match bpp {
                8 => Ok(data.to_vec()),
                4 => Ok(make_4bpp_bitpack(data, *self == WscPacked)),
                _ => unreachable!(),
            },
            PceSprite => {
                let mut nd = Vec::new();
                for p in 0..4 {
                    nd.extend(make_1bit_planes(data, p, false));
                }
                Ok(nd)
            }
        }
    }

    /// Unpack one tile's native format data to pixel indices.
    fn unpack_tile(
        &self,
        data: &[u8],
        bpp: u32,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>, String> {
        fn add_1bit_plane(
            out: &mut [u8],
            data: &[u8],
            plane: u32,
        ) {
            let plane_offset = (((plane >> 1) * 16) + (plane & 1)) as usize;
            for y in 0..8 {
                for x in 0..8 {
                    out[y * 8 + x] += ((data[plane_offset + y * 2] >> (7 - x)) & 1) << plane;
                }
            }
        }

        fn add_1bit_plane_4bpp(
            out: &mut [u8],
            data: &[u8],
            plane: u32,
        ) {
            let plane_offset = (((plane >> 2) * 32) + (plane & 3)) as usize;
            for y in 0..8 {
                for x in 0..8 {
                    out[y * 8 + x] += ((data[plane_offset + y * 4] >> (7 - x)) & 1) << plane;
                }
            }
        }

        fn add_2bpp_bitpack(
            out: &mut [u8],
            data: &[u8],
            reverse: bool,
        ) {
            for y in 0..8 {
                for x in 0..8 {
                    let px = if reverse { 7 - x } else { x };
                    out[y * 8 + x] = (data[(y << 1) | (px >> 2)] >> ((px << 1) & 6)) & 0x03;
                }
            }
        }

        let mut ud = vec![0u8; (width * height) as usize];
        match self {
            Snes | Gb | Gbc | Pce => {
                for i in 0..bpp {
                    add_1bit_plane(&mut ud, data, i);
                }
            }
            Ws | Wsc | Gg | Sms => match bpp {
                4 => {
                    for i in 0..bpp {
                        add_1bit_plane_4bpp(&mut ud, data, i);
                    }
                }
                2 => {
                    for i in 0..bpp {
                        add_1bit_plane(&mut ud, data, i);
                    }
                }
                _ => unreachable!(),
            },
            SnesMode7 => ud = data.to_vec(),
            Gba | GbaAffine | Md => {
                if bpp == 4 {
                    for (i, &byte) in data.iter().enumerate() {
                        ud[i << 1] = byte & 0x0f;
                        ud[(i << 1) + 1] = (byte & 0xf0) >> 4;
                    }
                } else {
                    ud = data.to_vec();
                }
            }
            WscPacked => {
                for (i, &byte) in data.iter().enumerate() {
                    ud[i << 1] = (byte & 0xf0) >> 4;
                    ud[(i << 1) + 1] = byte & 0x0f;
                }
            }
            Ngp | Ngpc => {
                if bpp == 2 {
                    add_2bpp_bitpack(&mut ud, data, true);
                } else {
                    return Err(format!("Unsupported bpp for mode '{self}'"));
                }
            }
            // TODO: Implement unpacking of pce_sprite data
            PceSprite => return Err("Using 'pce_sprite' native data as input not implemented".into()),
        }
        Ok(ud)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create 8x8 checkerboard tile as raw u8 array, "white" value based on given bit depth.
    fn checkerboard_tile(bpp: u32) -> Vec<u8> {
        let max = (1u32 << bpp) - 1;
        (0..64).map(|i| (i % (max + 1)) as u8).collect()
    }

    #[test]
    fn raw_native_roundtrip() {
        use Mode::*;
        for (mode, bpp) in [
            (Snes, 2),
            (Snes, 4),
            (Snes, 8),
            (Gb, 2),
            (Gbc, 4),
            (Pce, 4),
            (Ws, 2),
            (Wsc, 4),
            (Gg, 4),
            (Sms, 4),
            (Ngp, 2),
            (Ngpc, 2),
            (SnesMode7, 8),
            (Gba, 4),
            (Gba, 8),
            (Md, 4),
            (WscPacked, 4),
        ] {
            let data = checkerboard_tile(bpp);
            let packed = mode.pack_tile(&data, bpp, 8, 8).unwrap();
            let unpacked = mode.unpack_tile(&packed, bpp, 8, 8).unwrap();
            assert_eq!(unpacked, data, "raw_native_roundtrip failed for {mode} {bpp}bpp");
        }
    }
}
