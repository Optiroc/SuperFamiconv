//! Output binary end-to-end tests.

mod common;

use assert_cmd::Command;
use common::{TestDir, file_bytes, file_le_words, file_len};

fn sfc() -> Command {
    Command::cargo_bin("superfamiconv").unwrap()
}

#[test]
fn convert_snes_flip() {
    const EXPECTED_MAP_FLIP: [u16; 4] = [0x0000, 0x0001, 0xc001, 0xc000];
    const EXPECTED_MAP_NOPE: [u16; 4] = [0x0000, 0x0001, 0x0002, 0x0003];
    const TILE_LEN: u64 = 32;

    let dir = TestDir::new("convert_snes_flip");
    let flip_map = dir.file("flip_map.bin");
    let flip_tiles = dir.file("flip_tiles.bin");
    let noflip_map = dir.file("noflip_map.bin");
    let noflip_tiles = dir.file("noflip_tiles.bin");

    sfc()
        .args(["convert", "-i", "test_data/basic/rgba_flip.png"])
        .arg("-m")
        .arg(&flip_map)
        .arg("-t")
        .arg(&flip_tiles)
        .assert()
        .success();
    assert_eq!(file_len(&flip_tiles), TILE_LEN * 2);
    assert_eq!(file_le_words(&flip_map), EXPECTED_MAP_FLIP);

    sfc()
        .args(["convert", "-i", "test_data/basic/rgba_flip.png", "-F"])
        .arg("-m")
        .arg(&noflip_map)
        .arg("-t")
        .arg(&noflip_tiles)
        .assert()
        .success();
    assert_eq!(file_len(&noflip_tiles), TILE_LEN * 4);
    assert_eq!(file_le_words(&noflip_map), EXPECTED_MAP_NOPE);
}

#[test]
fn convert_snes_attribute_map_image() {
    const EXPECTED_MAP: [u16; 4] = [0x2000, 0x0001, 0xc001, 0xe000];

    let dir = TestDir::new("convert_snes_attribute_map_image");
    let map = dir.file("map.bin");

    sfc()
        .args([
            "convert",
            "-i",
            "test_data/basic/rgba_flip.png",
            "-a",
            "test_data/basic/rgba_flip_attr.png",
        ])
        .arg("-m")
        .arg(&map)
        .assert()
        .success();
    assert_eq!(file_le_words(&map), EXPECTED_MAP);
}

#[test]
fn convert_gbc_attrmap_tilemap() {
    #[rustfmt::skip]
    const EXPECTED_TILEMAP: [u8; 24] = [
        0x00, 0x00, 0x01, 0x01, 0x02, 0x02, 0x03, 0x03,
        0x04, 0x04, 0x05, 0x05, 0x00, 0x00, 0x01, 0x01,
        0x02, 0x02, 0x03, 0x03, 0x04, 0x04, 0x05, 0x05,
    ];

    #[rustfmt::skip]
    const EXPECTED_ATTRMAP: [u8; 24] = [
        0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01,
        0x00, 0x01, 0x00, 0x01, 0x02, 0x03, 0x02, 0x03,
        0x02, 0x03, 0x02, 0x03, 0x02, 0x03, 0x02, 0x03,
    ];

    let dir = TestDir::new("convert_gbc_attrmap_tilemap");
    let tilemap = dir.file("tilemap.bin");
    let attrmap = dir.file("attrmap.bin");

    sfc()
        .args([
            "convert",
            "-M",
            "gbc",
            "-i",
            "test_data/tricky_palette_packing/gbc3_max_4x4.png",
        ])
        .arg("--tm")
        .arg(&tilemap)
        .arg("--am")
        .arg(&attrmap)
        .assert()
        .success();
    assert_eq!(file_bytes(&tilemap), EXPECTED_TILEMAP);
    assert_eq!(file_bytes(&attrmap), EXPECTED_ATTRMAP);
}

#[test]
fn issue_63_snes_no_remap() {
    #[rustfmt::skip]
    const EXPECTED_PALETTE: [u16; 128] = [
        0x7c1f, 0x5f7d, 0x4ab5, 0x1496, 0x108e, 0x31ae, 0x1d09, 0x39f0,
        0x14a6, 0x0c85, 0x2d8d, 0x254b, 0x18e8, 0x6fdf, 0x2530, 0x18c6,
        0x7c1f, 0x0463, 0x5b3c, 0x1909, 0x2db0, 0x2d07, 0x4ab7, 0x52f9,
        0x256d, 0x0ca5, 0x3612, 0x6bbe, 0x10c7, 0x3a55, 0x637d, 0x212b,
        0x7c1f, 0x0863, 0x082a, 0x3e0f, 0x0421, 0x0c84, 0x4250, 0x18e7,
        0x2d6b, 0x1d09, 0x318c, 0x254a, 0x358d, 0x39ad, 0x10a6, 0x1d0a,
        0x7c1f, 0x6fbe, 0x294c, 0x0863, 0x358d, 0x4653, 0x3df0, 0x5af9,
        0x2109, 0x5251, 0x30c1, 0x5673, 0x6b9e, 0x39ae, 0x10c6, 0x5a6f,
        0x7c1f, 0x522f, 0x20a3, 0x4e0d, 0x3927, 0x5a4e, 0x5a92, 0x292a,
        0x1ce8, 0x5671, 0x562d, 0x39ad, 0x5ef7, 0x1441, 0x316c, 0x5ab4,
        0x7c1f, 0x5ed6, 0x520d, 0x5651, 0x62f8, 0x5ab4, 0x5250, 0x5a70,
        0x5a2c, 0x6b7b, 0x522f, 0x5a93, 0x5a2d, 0x673a, 0x5672, 0x5a93,
        0x7c1f, 0x4ad8, 0x08a5, 0x10e8, 0x1909, 0x3631, 0x1d4c, 0x0042,
        0x2daf, 0x0cc7, 0x2d6c, 0x08a6, 0x254a, 0x3e75, 0x0484, 0x0463,
        0x7c1f, 0x0442, 0x52fa, 0x0ca6, 0x25d4, 0x190a, 0x3a33, 0x3a77,
        0x1d91, 0x0484, 0x318d, 0x6b9e, 0x10c8, 0x2a36, 0x5f7d, 0x192d,
    ];
    const TILE_LEN: u64 = 32;

    let dir = TestDir::new("issue_63_snes_no_remap");
    let palette = dir.file("palette.bin");
    let tiles = dir.file("tiles.bin");

    sfc()
        .args(["convert", "-i", "test_data/issues/63/image.png", "-R"])
        .arg("-p")
        .arg(&palette)
        .arg("-t")
        .arg(&tiles)
        .assert()
        .success();
    assert_eq!(file_le_words(&palette), EXPECTED_PALETTE);
    assert_eq!(file_len(&tiles), TILE_LEN * 671);
}
