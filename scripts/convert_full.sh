#!/bin/zsh

mkdir -p out

tile_size=8
dims=$(file "$2" | grep -oE '[0-9]+ x [0-9]+')
img_width=${dims%% x *}
img_height=${dims##* x }
map_width=$(( (img_width + tile_size - 1) / tile_size ))
map_height=$(( (img_height + tile_size - 1) / tile_size ))

cargo run --release palette -v --mode "$1" --in-image "$2" --out-data out/palette.bin --out-image out/palette.png --out-json out/palette.json && \
cargo run --release tiles -v --mode "$1" --in-image "$2" --in-palette out/palette.bin --out-data out/tiles.bin --out-image out/tiles.png && \
cargo run --release map -v --mode "$1" --in-image "$2" --in-palette out/palette.bin --in-tiles out/tiles.bin --out-data out/map.bin --out-json out/map.json && \
cargo run --release map -v --mode "$1" --in-data out/map.bin --map-width "$map_width" --map-height "$map_height" --in-palette out/palette.bin --in-tiles out/tiles.bin --out-image out/map.png
