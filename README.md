# superfamiconv

A tile graphics converter with a flexible and composable command line interface.

`superfamiconv` converts images into data suitable for use on a range of video game consoles: Super Nintendo, Game Boy, Game Boy Color, Game Boy Advance, Mega Drive, Master System, Game Gear, PC Engine, Neo Geo Pocket, Neo Geo Pocket Color, WonderSwan and WonderSwan Color. 

The initial target system was Super Nintendo, which is known as Super Famicom in Japan. That informed not only the name of the tool, but also some fundamental design decisions. Given the similarities between most tile-based platforms of the era, adding support for many other systems was relatively painless.

The current version, v0.12, sheds the old C++ codebase completely in favor of Rust. The basic design is still mostly similar, but many longstanding issues have been fixed (and without a doubt new ones have been introduced). Refer to [this section](#notable-changes-in-v012) for a list of the changes you should be aware of if you're moving workflows from an older version.

## overview
The targeted consoles generally don't draw arbitrary bitmaps. Instead, graphics are composed from three basic data structures: palettes, tiles and maps. The number of colors, tiles and specific features a map supports differ between systems, but the general idea is:

- Colors come from a "palette" consisting of one or more "subpalettes".
- Pixel information come from "tile" definitions, typically an array of 8x8 values that each represent an index into a "subpalette".
- The image is pieced together from a "map", typically an array of 32x32 entries that each represent tile and subpalette indices, and sometimes additional attributes.

`superfamiconv` turns an image into these representations in discrete stages:

- `palette`: colors are reduced to the console's native color depth and packed into one or more subpalettes.
- `tiles`: the image is sliced into tiles, remapped against the palette, and deduplicated into a tileset.
- `map`: each tile position in the image is mapped against both the tileset and the palette to form to a map entry.

`superfamiconv` has its own subcommand for running a specific stage of this process (`palette`, `tiles`, `map`). This is where "flexible and composable" comes in: for example it allows for generating a single set of subpalettes and tiles reused across many images (say, for different levels in a game).

You can also run all three stages in one fell swoop using the [`convert`](#convert) subcommand.

### example
Before we get into the weeds let's run a simple example. Converting this logo to palette, tile and map data ready to be displayed by the Super Nintendo, using the `convert` command. With verbose logging (`-v`) turned on, each step of the process is detailed.

![megaboys m logo](img/mb_logo.png)
```
$ superfamiconv convert -v -M snes -i mb_logo.png -p palette.bin -t tiles.bin -m map.bin --pi palette.png --ti tiles.png
Performing convert operation (mode: snes)
Loaded image from 'mb_logo.png' (256x224px RGB)
Mapping palette with at most 8x16 entries
Locking color zero to #d61818ff
Created palette with 12 colors
Saved native palette data to 'palette.bin'
Saved palette image to 'palette.png'
Created tileset with 81 entries (815 tiles deduplicated)
Saved native tile data to 'tiles.bin'
Saved tileset image to 'tiles.png'
Mapping 896 8x8px tiles from image
Map laid out in single group, 32x28 entries
Saved native map data to 'map.bin'
```
In addition to the native data, preview images of the palette and tiles were saved. Only one subpalette was needed for this simple image:

![palette](img/palette_16x.png)

And the tiles, looking like an extended board of "15 Puzzle" (except impossible to clear, since there's only 81 tiles, and the full image consists of many more; identical tiles are discarded):

![tiles](img/tiles_2x.png)

## essential concepts

### mode
The target system is specified with the `-M/--mode` option which available for all subcommands. If omitted, `snes` is the default.

Palette size, tile size, bit depth and other default settings are applied depending on the selected mode. These can be overridden using various settings available on each subcommand.

Supported modes and default settings:

| mode | target | tile size | tile count | subpalette count | bpp |  flip |
|---|---|--:|--:|--:|--:|:-:|
| `snes` | Super Nintendo (modes 0-6) | 8x8 | 1024 | 8 | 4 | ◯ |
| `snes_mode7` | Super Nintendo (mode 7) | 8x8 | 256 | 1 | 8 | ✕ |
| `gb` | Game Boy | 8x8 | 256 | 1 | 2 | ✕ |
| `gbc` | Game Boy Color | 8x8 | 512 | 8 | 2 | ◯ |
| `gba` | Game Boy Advance | 8x8 | 1024 | 16 | 4 | ◯ |
| `gba_affine` | Game Boy Advance (affine) | 8x8 | 256 | 1 | 8 | ✕ |
| `md` | Mega Drive | 8x8 | 2048 | 4 | 4 | ◯ |
| `sms` | Master System | 8x8 | 512 | 2 | 4 | ✕ |
| `gg` | Game Gear | 8x8 | 512 | 2 | 4 | ✕ |
| `pce` | PC Engine | 8x8 | 2048 | 16 | 4 | ✕ |
| `pce_sprite` | PC Engine (sprite) | 16x16 | ✕ | 16 | 4 | ✕ |
| `ngp` | Neo Geo Pocket | 8x8 | 512 | 2 | 2 | ◯ |
| `ngpc` | Neo Geo Pocket Color | 8x8 | 512 | 16 | 2 | ◯ |
| `ws` | WonderSwan | 8x8 | 512 | 16 | 2 | ◯ |
| `wsc` | WonderSwan Color (planar) | 8x8 | 1024 | 16 | 4 | ◯ |
| `wsc_packed` | WonderSwan Color (packed) | 8x8 | 1024 | 16 | 4 | ◯ |

### palette generation and color zero
Colors are reduced to the target's native depth and packed into as few subpalettes as possible. On targets where color index 0 is shared or transparent across all subpalettes (most consoles except `gb`, `gbc`, `sms` or `gg`), special care is sometimes needed to correctly convert the input. By default the color forming the longest continuous run of pixels in the source image is selected, but it can be overridden with the `--color-zero` setting.

Note that no color space transformation is performed on input images. The raw RGB values are used directly when mapping to target specific precision. When performing color and luma comparisons the raw RGB values are treated as sRGB regardless of PNG metadata.

### tile deduplication and flipping
Identical tiles are merged into a single tileset entry. On formats that support flipped tiles (see table above), tiles that are duplicates only after a horizontal and/or vertical flip can also be merged. The flip information is stored in tilemap attribbutes instead of the pixel data.

Pass `--no-discard` to keep every tile distinct, or `--no-flip` to disable flip-deduplication while still discarding exact duplicates.

### working from indexed color images
Normally, colors are read from each pixel and packed into subpalettes. With `--no-remap`, `superfamiconv` will instead use the palette and indexed-color pixels from an image as-is:
- The `palette` subcommand creates a palette without reording colors. Only quantization to the target bit depth is applied.
- The `tiles` subcommand uses pixel indices straight from the image, without remapping against a supplied palette.

The `--no-remap` option requires the input PNG to be saved in indexed color mode.

### quantization
The default operation of `superfamiconv` is to perform lossless conversion, except for the inevitable loss of color precision when going from 8-bit per channel source data to the native precision of the target `mode`.

When converting pixel art this is essential; you don't want carefully chosen shades of color in hand drawn art to get discarded.

Sometimes this might be okay or even expected, though. Common examples might be high color illustrations or 3D renderings intended for a title screen or cut scene. In those cases you can let `superfamiconv` perform lossy conversion by enabling the `-Q`/`--quantize` option. Palettes will be created that loses the least of the original color information. Tiles, in turn, are rendered using the palette that most closely matches its color contents. By default dithering is applied but it can be disabled, or a different algorithm can be chosen.


## option reference
TODO

## detailed operation

### command overview
TODO
```
Usage: superfamiconv <COMMAND>

Commands:
  convert  Convert an image to palette, tile and/or map data
  palette  Convert an image to palette data
  tiles    Convert an image and palette (or native tile data) to tile data
  map      Convert an image, palette and tileset to map data
  help     Print this message or the help of the given subcommand(s)

Info:
  -h, --help     Print help
  -V, --version  Print version
```


### convert
`superfamiconv convert` takes one image as input and outputs palette, tile and/or map data. Sensible mode-dependent defaults are applied, but can of course be overridden.

Full usage:
```
Usage: superfamiconv convert [OPTIONS]

Input files:
  -i, --in-image <FILE>           Source image(s)
  -a, --in-attribute-map <FILE>   Priority attribute map image

Output files:
  -p, --out-palette <FILE>        Native palette data
  -t, --out-tiles <FILE>          Native tile data
  -m, --out-map <FILE>            Native map data
      --out-palette-map <FILE>    Palette map [alias: --pm]
      --out-tile-map <FILE>       Tile map [alias: --tm]
      --out-attribute-map <FILE>  Attribute map [alias: --am]
      --out-mode7-data <FILE>     Interleaved map/tile data [snes_mode7] [alias: --m7]
      --out-palette-image <FILE>  Palette image [alias: --pi]
      --out-tile-image <FILE>     Tile image [alias: --ti]
      --out-preview-image <FILE>  Preview image [alias: --img]
      --out-palette-act <FILE>    Adobe color table [alias: --act]

Options:
  -M, --mode <MODE>               Mode [default: snes]
  -B, --bpp <BPP>                 Bits per pixel
  -N, --palettes <N>              Number of subpalettes
  -C, --colors <N>                Colors per subpalette
  -W, --tile-width <W>            Tile width
  -H, --tile-height <H>           Tile height
  -R, --no-remap                  Do not remap colors
  -D, --no-discard                Do not deduplicate identical tiles
  -F, --no-flip                   Do not deduplicate via tile flipping
  -T, --max-tiles <N>             Maximum number of tiles
  -S, --sprite-mode               Apply sprite output settings
  -Z, --color-zero <COLOR>        Set color zero
  -Q, --quantize                  Quantize colors and tiles to fit target palette
      --dither <DITHER>           Dithering to apply if quantizing [default: bayer4]
      --round                     Round colors instead of truncating
      --tile-base-offset <N>      Tile base offset for map data [default: 0]
      --palette-base-offset <N>   Palette base offset for map data [default: 0]

Info:
  -v, --verbose...  Verbose logging (-vv for extra verbosity)
  -h, --help        Print help
```

### palette
TODO: Convert an image to palette data.

Full usage:
```
Usage: superfamiconv palette [OPTIONS]

Input files:
  -i, --in-image <FILE>  Source image(s)

Output files:
  -d, --out-data <FILE>     Native palette data
  -o, --out-image <FILE>    Palette image
  -j, --out-json <FILE>     Palette json
      --out-act <FILE>      Adobe color table [alias: --act]

Options:
  -M, --mode <MODE>         Mode [default: snes]
  -N, --palettes <N>        Number of subpalettes
  -C, --colors <N>          Colors per subpalette
  -W, --tile-width <W>      Tile width
  -H, --tile-height <H>     Tile height
  -R, --no-remap            Do not remap colors
  -Z, --color-zero <COLOR>  Set color zero
  -Q, --quantize            Quantize colors to fit target palette
      --round               Round colors instead of truncating

Info:
  -v, --verbose...  Verbose logging (-vv for extra verbosity)
  -h, --help        Print help
```


## tiles
TODO: Convert an image and palette (or native tile data) to tile data.

Full usage:
```
Usage: superfamiconv tiles [OPTIONS]

Input files:
  -i, --in-image <FILE>      Source image (multiple allowed)
  -n, --in-data <FILE>       Native tile data
  -p, --in-palette <FILE>    Palette (native or json)

Output files:
  -d, --out-data <FILE>      Native tile data
  -o, --out-image <FILE>     Tile image

Options:
  -M, --mode <MODE>          Mode [default: snes]
  -B, --bpp <BPP>            Bits per pixel
  -W, --tile-width <W>       Tile width
  -H, --tile-height <H>      Tile height
  -R, --no-remap             Do not remap colors
  -D, --no-discard           Do not deduplicate identical tiles
  -F, --no-flip              Do not deduplicate via tile flipping
  -T, --max-tiles <N>        Maximum number of tiles
  -S, --sprite-mode          Apply sprite output settings
  -Q, --quantize             Quantize (match tiles to the closest subpalette)
      --dither <DITHER>      Dithering to apply if quantizing [default: bayer4]
      --round                Round colors instead of truncating
      --out-image-width <W>  Width of output tile image

Info:
  -v, --verbose...  Verbose logging (-vv for extra verbosity)
  -h, --help        Print help
```

### map
TODO: Convert an image, palette and tileset to map data. Note that you need to pass the same `quantize`, `dither` and `round` settings as when creating palette and tileset, for the tool to be able to match tiles to the map image. 

Full usage:
```
Usage: superfamiconv map [OPTIONS]

Input files:
  -i, --in-image <FILE>           Source image(s)
  -n, --in-data <FILE>            Native map data
  -p, --in-palette <FILE>         Palette (native or json)
  -t, --in-tiles <FILE>           Native tile data
  -a, --in-attribute-map <FILE>   Priority attribute map image

Output files:
  -d, --out-data <FILE>           Native map data
  -j, --out-json <FILE>           JSON map data
  -o, --out-image <FILE>          Map image
      --out-palette-map <FILE>    Palette map [alias: --pm]
      --out-tile-map <FILE>       Tile map [alias: --tm]
      --out-attribute-map <FILE>  Attribute map [alias: --am]
      --out-mode7-data <FILE>     Interleaved native map/tile data [snes_mode7] [alias: --m7]

Options:
  -M, --mode <MODE>               Mode [default: snes]
  -B, --bpp <BPP>                 Bits per pixel
  -W, --tile-width <W>            Tile width
  -H, --tile-height <H>           Tile height
  -F, --no-flip                   Do not allow tile flipping
  -Q, --quantize                  Quantize (match tiles to the closest subpalette)
      --dither <DITHER>           Dithering to apply if quantizing [default: bayer4]
      --round                     Round colors instead of truncating
      --map-width <W>             Map width (in tiles) [alias: --mw]
      --map-height <H>            Map height (in tiles) [alias: --mh]
      --split-width <W>           Split output into columns of <tiles> width [alias: --sw]
      --split-height <H>          Split output into rows of <tiles> height [alias: --sh]
      --tile-base-offset <N>      Tile base offset for map data [default: 0]
      --palette-base-offset <N>   Palette base offset for map data [default: 0]
      --column-order              Output data in column-major order

Info:
  -v, --verbose...  Verbose logging (-vv for extra verbosity)
  -h, --help        Print help
```


## notable changes in v0.12

### added
TODO

### changed
TODO

### removed
TODO


## history
- v0.0-v0.2 (2005.02.05): C-style C++98. Not publicly circulated.
- v0.3-v0.11 (2017.04.17): C++14 rewrite.
- v0.12- (2026.xx.yy): Rust rewrite.


## about
superfamiconv is developed by david lindecrantz and [contributors](https://github.com/Optiroc/SuperFamiconv/graphs/contributors?all=1). distributed under the terms of the [MIT license](./LICENSE).
