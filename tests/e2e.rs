//! Output binary end-to-end tests.

use assert_cmd::Command;
use predicates::prelude::*;

fn sfc() -> Command {
    Command::cargo_bin("superfamiconv").unwrap()
}
#[test]
fn issue_63_no_remap() {
    sfc()
        .args(["convert", "-v", "-i", "test_data/issues/63/image.png", "-R"])
        .assert()
        .success()
        .stdout(predicate::str::contains("671 entries (225 tiles deduplicated)"));
}

#[test]
fn convert_flip() {
    sfc()
        .args(["convert", "-v", "-i", "test_data/basic/rgba_flip.png"])
        .assert()
        .success()
        .stdout(predicate::str::contains("2 entries"));
    sfc()
        .args(["convert", "-v", "-i", "test_data/basic/rgba_flip.png", "-F"])
        .assert()
        .success()
        .stdout(predicate::str::contains("4 entries"));
}

#[test]
fn convert_quantize() {
    sfc()
        .args(["convert", "-v", "-i", "test_data/quantization/bryggen.png", "-N", "2"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("too many (23 > 15) unique colors"));
    sfc()
        .args([
            "convert",
            "-v",
            "-i",
            "test_data/quantization/bryggen.png",
            "-N",
            "2",
            "-Q",
            "--dither",
            "off",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Quantizing palette with at most 2x16 entries"))
        .stdout(predicate::str::contains("Created palette with 32 colors [16, 16]"));
}
