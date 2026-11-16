//! Tests `cli::run` as called from a tool that embeds superfamiconv.

use std::path::Path;

use superfamiconv::cli;

#[test]
fn run_reports_resolve_errors() {
    let err = cli::run(Path::new(""), ["convert", "-p", "out.pal"]).unwrap_err();
    assert!(err.contains("Input image required"), "{err}");
}

#[test]
fn run_reports_parse_errors_without_styling() {
    let err = cli::run(Path::new(""), ["tiles", "--no-such-flag"]).unwrap_err();
    assert!(err.contains("--no-such-flag"), "{err}");
    assert!(!err.contains('\x1b'), "{err}");
}

/// Relative paths are read from the given directory.
#[test]
fn run_converts_an_image_from_dir() {
    let dir = std::env::temp_dir().join(format!("sfc-embedded-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy("test_data/basic/rgba_rgbw.png", dir.join("in.png")).unwrap();

    cli::run(&dir, ["convert", "-i", "in.png", "-t", "out.4bpp"]).unwrap();
    assert!(std::fs::metadata(dir.join("out.4bpp")).unwrap().len() > 0);
    std::fs::remove_dir_all(&dir).ok();
}
