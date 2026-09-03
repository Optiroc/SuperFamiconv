//! Output binary sanity checks.

use assert_cmd::Command;
use predicates::prelude::*;

fn sfc() -> Command {
    Command::cargo_bin("superfamiconv").unwrap()
}

#[test]
fn no_subcommand_no_args() {
    sfc()
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: superfamiconv"));
}

#[test]
fn bare_subcommand_no_args() {
    sfc()
        .arg("tiles")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: superfamiconv tiles"));
}

#[test]
fn convert_missing_input_image() {
    sfc()
        .args(["convert", "-p", "out.pal"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Input image required"));
}

#[test]
fn tiles_incompatible_bpp() {
    sfc()
        .args(["tiles", "-i", "in.png", "-M", "gb", "-B", "4"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("bpp=4 is not allowed for mode \'gb\'"));
}
