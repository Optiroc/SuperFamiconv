# superfamiconv

A tile graphics converter with a flexible and composable command line interface.

`superfamiconv` converts images into data suitable to use on a range of video game consoles: Super Nintendo, Game Boy (Color), Game Boy Advance, Mega Drive, Master System, Game Gear, PC Engine, Neo Geo Pocket (Color) and WonderSwan (Color). 

The initial target system was Super Nintendo, which is known as Super Famicom in Japan. That informed not only the name of the tool, but also some fundamental design decisions. Given the similarities between most tile-based platforms of the era, adding support for many other systems was relatively painless.

## overview
The targeted consoles generally don't draw arbitrary bitmaps. Instead, graphics are composed from three data structures: palettes, tiles and maps. The number of colors, tiles and specific features a map supports differ between systems, but the general idea is:

- Colors come from a "palette" consisting of one or more "subpalettes".
- Pixel information comes from "tile" definitions, typically an array of 8x8 values that each represent an index into a "subpalette".
- The image is pieced together from a "map", typically an array of 32x32 values that each represent tile and subpalette indices. 

`superfamiconv` turns an image into these representations in discrete stages:

- `palette`: colors are reduced to the console's native color depth and packed into one or more subpalettes.
- `tiles`: the image is sliced into tiles, remapped against the palette, and deduplicated into a tileset.
- `map`: each tile position in the image is mapped against both the tileset and the palette to form to a map entry.

`superfamiconv` has its own subcommand for running a specific stage of this process (`palette`, `tiles`, `map`). This is where "flexible and composable" comes in: for example it allows for generating a single set of subpalettes and tiles reused across many images (say, for different levels in a game).

You can also run all three stages in one fell swoop using the [`convert`](#convert) subcommand.

## key concepts

### mode
The target system is specified with the `-M/--mode` setting, available in all subcommands. If omitted, `snes` is the default.

Palette size, tile size, bit depth and other default settings are applied depending on the selected mode. These can be overridden using various settings available on each subcommand.

Supported modes and default settings:

| mode | target | bpp | tile size | max tile count | max subpalette count | flip |
|--|--|-:|-:|-:|-:|-:|
| `snes` | Super Nintendo (modes 0-6) | 4 | 8x8 | 1024 | 8 | yes |
| `snes_mode7` | Super Nintendo (mode 7) | 8 | 8x8 | 256 | 1 | no |
| `gb` | Game Boy | 2 | 8x8 | 256 | 1 | no |
| `gbc` | Game Boy Color | 2 | 8x8 | 512 | 8 | yes |
| `gba` | Game Boy Advance | 4 | 8x8 | 1024 | 16 | yes |
| `gba_affine` | Game Boy Advance (affine background) | 8 | 8x8 | 256 | 1 | no |
| `md` | Mega Drive | 4 | 8x8 | 2048 | 4 | yes |
| `sms` | Master System | 4 | 8x8 | 512 | 2 | no |
| `gg` | Game Gear | 4 | 8x8 | 512 | 2 | no |
| `pce` | PC Engine | 4 | 8x8 | 2048 | 16 | no |
| `pce_sprite` | PC Engine (sprite data) | 4 | 16x16 | 2048 | 16 | no |
| `ngp` | Neo Geo Pocket | 2 | 8x8 | 512 | 2 | yes |
| `ngpc` | Neo Geo Pocket Color | 2 | 8x8 | 512 | 16 | yes |
| `ws` | WonderSwan | 2 | 8x8 | 512 | 16 | yes |
| `wsc` | WonderSwan Color (planar) | 4 | 8x8 | 1024 | 16 | yes |
| `wsc_packed` | WonderSwan Color (packed) | 4 | 8x8 | 1024 | 16 | yes |

### palette generation and color zero
Colors are reduced to the target's native depth and packed into as few subpalettes as possible. On targets where color index 0 is shared or transparent across all subpalettes (most consoles except `gb`, `gbc`, `sms` or `gg`), special care is sometimes needed to correctly convert the input. By default the color forming the longest continuous run of pixels in the source image is selected, but it can be overridden with the `--color-zero` setting.

Note that no color space transformation is performed on input images. The raw RGB values are used directly when mapping to target specific precision, which I find most predictable. When performing color and luma comparisons the raw RGB values are treated as sRGB regardless of PNG metadata.

### tile deduplication and flipping
Identical tiles are merged into a single tileset entry. On formats that support flipped tiles (see table above), tiles that are duplicates only after a horizontal and/or vertical flip can also be merged. The flip information is stored in tilemap attribbutes instead of the pixel data.

Pass `--no-discard` to keep every tile distinct, or `--no-flip` to disable flip-deduplication while still discarding exact duplicates.

### working from indexed images
Normally, colors are quantized from 24-bit color information and packed into subpalettes. With `--no-remap`, `superfamiconv` will instead use the palette and indexed-color pixels from an image as-is:
- The `palette` subcommand creates a palette without reording colors. Only quantization to the target bit depth is applied.
- The `tiles` subcommand uses pixel indices straight from the image, without remapping against a supplied palette.

The `--no-remap` option requires the input PNG to be saved in indexed color mode.


## detailed operation

### command overview
TODO: Subcommands, help, bla bla
```
Usage: superfamiconv <COMMAND>

Commands:
  convert  Convert an image to palette, tile and/or map data
  palette  Convert an image to palette data
  tiles    Convert an image and palette (or native tile data) to tile data
  map      Convert an image, palette and tileset to map data
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

### convert
`superfamiconv convert` takes one image as input and outputs palette, tile and/or map data. Sensible mode-dependent defaults are applied, but can of course be overridden.

Example:
```
superfamiconv convert -v --mode snes --in-image snes.png --out-palette snes_palette.bin --out-tiles snes_tiles.bin --out-map snes_map.bin --out-palette-image snes_palette.png --out-tiles-image snes_tiles.png
Performing convert operation (mode: snes)
Loaded image from 'snes.png' (256x256px RGB)
Mapping palette with at most 8x16 entries
Locking color zero to #000000ff
Created palette with 112 colors [16, 16, 16, 16, 16, 16, 16]
Saved native palette data to 'snes_palette.bin'
Saved palette image to 'snes_palette.png'
Created tileset with 583 entries (441 tiles deduplicated)
Saved native tile data to 'snes_tiles.bin'
Saved tileset image to 'snes_tiles.png'
Mapping 1024 8x8px tiles from image
Map laid out in single group, 32x32 entries
Saved native map data to 'snes_map.bin'
```

Full usage:
```
Usage: superfamiconv convert [OPTIONS]

Options:
  -i, --in-image <IN_IMAGE>                    Input: image
  -p, --out-palette <OUT_PALETTE>              Output: palette data
  -t, --out-tiles <OUT_TILES>                  Output: tile data
  -m, --out-map <OUT_MAP>                      Output: map data
      --out-palette-image <OUT_PALETTE_IMAGE>  Output: palette image
      --out-palette-act <OUT_PALETTE_ACT>      Output: photoshop palette
      --out-tiles-image <OUT_TILES_IMAGE>      Output: tiles image
      --out-preview-image <OUT_PREVIEW_IMAGE>  Output: preview image
  -v, --verbose...                             Verbose logging (-vv for extra verbosity)
  -h, --help                                   Print help (see more with '--help')

Settings:
  -M, --mode <MODE>
          Mode [default: snes] [possible values: snes, snes_mode7, gb, gbc, gba, gba_affine, md, sms, gg, pce, pce_sprite, ngp, ngpc, ws, wsc, wsc_packed]
  -B, --bpp <BPP>
          Bits per pixel [default: mode-dependent]
  -N, --palettes <PALETTES>
          Number of subpalettes [default: mode-dependent]
  -C, --colors <COLORS>
          Colors per subpalette [default: mode-dependent]
  -E, --effort <EFFORT>
          Palette optimization effort [default: medium] [possible values: low, medium, high]
  -W, --tile-width <TILE_WIDTH>
          Tile width [default: mode-dependent]
  -H, --tile-height <TILE_HEIGHT>
          Tile height [default: mode-dependent]
  -R, --no-remap
          Don't remap colors
  -D, --no-discard
          Don't deduplicate redundant tiles
  -F, --no-flip
          Don't deduplicate using tile flipping
  -T, --max-tiles <MAX_TILES>
          Maximum number of tiles [default: mode-dependent]
  -S, --sprite-mode
          Apply sprite output settings
  -Z, --color-zero <COLOR_ZERO>
          Set color #0 (6 or 8 character hex string)
  -Q, --quantize
          Quantize colors and tiles to fit target palette settings
      --dither <DITHER>
          Dithering to apply if quantizing [default: bayer4x4] [possible values: off, bayer2x2, bayer4x4, atkinson]
      --tile-base-offset <TILE_BASE_OFFSET>
          Tile base offset for map data [default: 0]
      --palette-base-offset <PALETTE_BASE_OFFSET>
          Palette base offset for map data [default: 0]
```

### palette
TODO: Convert an image to palette data.

Full usage:
```
Usage: superfamiconv palette [OPTIONS]

Options:
  -i, --in-image <IN_IMAGE>    Input: image
  -d, --out-data <OUT_DATA>    Output: native data
  -a, --out-act <OUT_ACT>      Output: adobe color table
  -j, --out-json <OUT_JSON>    Output: json
  -o, --out-image <OUT_IMAGE>  Output: image
  -v, --verbose...             Verbose logging (-vv for extra verbosity)
  -h, --help                   Print help (see more with '--help')

Settings:
  -M, --mode <MODE>                Mode [default: snes] [possible values: snes, snes_mode7, gb, gbc, gba, gba_affine, md, sms, gg, pce, pce_sprite, ngp, ngpc, ws, wsc, wsc_packed]
  -N, --palettes <PALETTES>        Number of subpalettes [default: mode-dependent]
  -C, --colors <COLORS>            Colors per subpalette [default: mode-dependent]
  -E, --effort <EFFORT>            Palette optimization effort [default: medium] [possible values: low, medium, high]
  -W, --tile-width <TILE_WIDTH>    Tile width [default: mode-dependent]
  -H, --tile-height <TILE_HEIGHT>  Tile height [default: mode-dependent]
  -R, --no-remap                   Don't remap colors
  -S, --sprite-mode                Apply sprite output settings
  -Z, --color-zero <COLOR_ZERO>    Set color #0 (6 or 8 character hex string)
  -Q, --quantize                   Quantize colors to fit target palette settings
```


## tiles
TODO: Convert an image and palette (or native tile data) to tile data.

Full usage:
```
Usage: superfamiconv tiles [OPTIONS]

Options:
  -i, --in-image <IN_IMAGE>      Input: image
  -n, --in-data <IN_DATA>        Input: native data
  -p, --in-palette <IN_PALETTE>  Input: palette (native/json)
  -d, --out-data <OUT_DATA>      Output: native data
  -o, --out-image <OUT_IMAGE>    Output: image
  -v, --verbose...               Verbose logging (-vv for extra verbosity)
  -h, --help                     Print help (see more with '--help')

Settings:
  -M, --mode <MODE>
          Mode [default: snes] [possible values: snes, snes_mode7, gb, gbc, gba, gba_affine, md, sms, gg, pce, pce_sprite, ngp, ngpc, ws, wsc, wsc_packed]
  -B, --bpp <BPP>
          Bits per pixel [default: mode-dependent]
  -W, --tile-width <TILE_WIDTH>
          Tile width [default: mode-dependent]
  -H, --tile-height <TILE_HEIGHT>
          Tile height [default: mode-dependent]
  -R, --no-remap
          Don't remap colors
  -D, --no-discard
          Don't deduplicate redundant tiles
  -F, --no-flip
          Don't deduplicate using tile flipping
  -T, --max-tiles <MAX_TILES>
          Maximum number of tiles [default: mode-dependent]
  -S, --sprite-mode
          Apply sprite output settings
  -Q, --quantize
          Quantize (match tiles to the closest subpalette and color)
      --dither <DITHER>
          Dithering to apply if quantizing [default: bayer4x4] [possible values: off, bayer2x2, bayer4x4, atkinson]
      --out-image-width <OUT_IMAGE_WIDTH>
          Width of output tileset image
```

### map
TODO: Convert an image, palette and tileset to map data.

Full usage:
```
Usage: superfamiconv map [OPTIONS]

Options:
  -i, --in-image <IN_IMAGE>          Input: image
  -n, --in-data <IN_DATA>            Input: native data
  -p, --in-palette <IN_PALETTE>      Input: palette (json/native)
  -t, --in-tiles <IN_TILES>          Input: tiles (native)
  -d, --out-data <OUT_DATA>          Output: native data
  -j, --out-json <OUT_JSON>          Output: json
  -7, --out-m7-data <OUT_M7_DATA>    Output: interleaved map/tile data (snes_mode7)
      --out-gbc-bank <OUT_GBC_BANK>  Output: banked map data (gbc)
      --out-pal-map <OUT_PAL_MAP>    Output: palette map (native 16-bit LE)
  -o, --out-image <OUT_IMAGE>        Output: image
  -v, --verbose...                   Verbose logging (-vv for extra verbosity)
  -h, --help                         Print help (see more with '--help')

Settings:
  -M, --mode <MODE>
          Mode [default: snes] [possible values: snes, snes_mode7, gb, gbc, gba, gba_affine, md, sms, gg, pce, pce_sprite, ngp, ngpc, ws, wsc, wsc_packed]
  -B, --bpp <BPP>
          Bits per pixel [default: mode-dependent]
  -W, --tile-width <TILE_WIDTH>
          Tile width [default: mode-dependent]
  -H, --tile-height <TILE_HEIGHT>
          Tile height [default: mode-dependent]
  -F, --no-flip
          Don't use flipped tiles
  -Q, --quantize
          Quantize (match tiles to the closest subpalette and color)
      --dither <DITHER>
          Dithering to apply if quantizing [default: bayer4x4] [possible values: off, bayer2x2, bayer4x4, atkinson]
      --map-width <MAP_WIDTH>
          Map width (in tiles) [default: image width]
      --map-height <MAP_HEIGHT>
          Map height (in tiles) [default: image height]
      --split-width <SPLIT_WIDTH>
          Split output into columns of <tiles> width [default: mode-dependent]
      --split-height <SPLIT_HEIGHT>
          Split output into rows of <tiles> height [default: mode-dependent]
      --column-order
          Output data in column-major order [default: row-major]
      --tile-base-offset <TILE_BASE_OFFSET>
          Tile base offset for map data [default: 0]
      --palette-base-offset <PALETTE_BASE_OFFSET>
          Palette base offset for map data [default: 0]
```


## history
- v0.0-v0.2 (2005.02.05-): Initial version. Not publicly circulated.
- v0.3-v0.11 (2017.04.17-): "Modern" C++ rewrite.
- v0.12- (2017.04.17-): Rust rewrite.


## about
superfamiconv is developed by david lindecrantz and [contributors](https://github.com/Optiroc/SuperFamiconv/graphs/contributors?all=1). distributed under the terms of the [MIT license](./LICENSE).
