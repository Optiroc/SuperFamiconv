#!/bin/zsh

mkdir -p out
cargo run --release convert -v --mode "$1" --in-image "$2" --out-palette out/out_palette.bin --out-tiles out/out_tiles.bin --out-map out/out_map.bin --out-palette-image out/out_palette.png --out-tiles-image out/out_tiles.png --out-preview-image out/out_image.png -Q --dither bayer4x4
