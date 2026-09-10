#!/bin/zsh

mkdir -p out
cargo run --release palette -v --mode "$1" --in-image "$2" --out-data out/palette.bin --out-image out/palette.png --out-json out/palette.json && \
cargo run --release tiles -v --mode "$1" --in-image "$2" --in-palette out/palette.bin --out-data out/tiles.bin --out-image out/tiles.png && \
cargo run --release map -v --mode "$1" --in-image "$2" --in-palette out/palette.bin --in-tiles out/tiles.bin --out-data out/map.bin --out-json out/map.json && \
cargo run --release map -v --mode "$1" --in-data out/map.bin --map-width 2 --map-height 2 --in-palette out/palette.bin --in-tiles out/tiles.bin --out-image out/map.png
