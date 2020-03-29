# superfamiconv
A tile graphics converter with flexible and composable command line options.

Developed by David Lindecrantz and distributed under the terms of the [MIT license](./LICENSE).


## dependencies
A C++14 capable compiler.

## building
In a Unix-like environment simply `make` the binary. On Windows use CMake to generate a build environment.

## operation

	superfamiconv <command> [<options>]

Where `<command>` is either `palette`, `tiles`, `map` or left blank for a simpler "short hand" operation.

In short hand mode, the following options are available:

	-i --in-image         Input: image
	-p --out-palette      Output: palette data
	-t --out-tiles        Output: tile data
	-m --out-map          Output: map data
	--out-palette-image   Output: palette image
	--out-palette-act     Output: photoshop palette
	--out-tiles-image     Output: tiles image
	--out-scaled-image    Output: image scaled to destination colorspace

	-M --mode             Mode <default: snes>
	-B --bpp              Bits per pixel
	-W --tile-width       Tile width
	-H --tile-height      Tile height
	-R --no-remap         Don't remap colors <switch>
	-D --no-discard       Don't discard redundant tiles <switch>
	-F --no-flip          Don't discard using tile flipping <switch>
	-T --tile-base-offset Tile base offset for map data
	-S --sprite-mode      Apply sprite output settings <switch>
	--color-zero          Set color #0

	-v --verbose          Verbose logging <switch>
	-l --license          Show licenses <switch>
	-h --help             Show this help <switch>

This command mode accepts one image (either indexed, RGB or RGBA mode PNG – which are the formats supported for all image inputs) and outputs palette, tile and/or map data.

The `mode` option, which is common for all commands, affects the color space handling and binary output format. It takes one of the following arguments: 

* `snes` 
* `snes_mode7` 
* `gb` 
* `gbc` 
* `gba` 
* `gba_affine` 
* `md` 
* `pce` 
* `pce_sprite` 

Sensible default options are applied, and differ depending on selected mode.

Example:

	superfamiconv -v --in-image snes.png --out-palette snes.palette --out-tiles snes.tiles --out-map snes.map --out-tiles-image tiles.png
	Loaded image from "snes.png" (256x224px, indexed color)
	Mapping optimized palette (16x16 entries)
	Setting color zero to #505050
	Created palette with 24 colors [16,8]
	Created optimized tileset with 156 entries (discarded 740 redudant tiles)
	Mapping 896 8x8px tiles from image
	Saved native palette data to "snes.palette"
	Saved native tile data to "snes.tiles"
	Saved native map data to "snes.map"
	Saved tileset image to "tiles.png"


For more flexibility use the sub commands, which have the following options respectively:

**superfamiconv palette**

	Usage: superfamiconv palette [<options>]
	  -i --in-image         Input: image
	  -d --out-data         Output: native data
	  -a --out-act          Output: photoshop palette
	  -j --out-json         Output: json
	  -o --out-image        Output: image
	
	Settings:
	  -M --mode             Mode <default: snes>
	  -P --palettes         Number of subpalettes
	  -C --colors           Colors per subpalette
	  -W --tile-width       Tile width
	  -H --tile-height      Tile height
	  -R --no-remap         Don't remap colors <switch>
	  -S --sprite-mode      Apply sprite output settings <switch>
	  -0 --color-zero       Set color #0
	
	  -v --verbose          Verbose logging <switch>
	  -h --help             Show this help <switch>


**superfamiconv tiles**

	Usage: superfamiconv tiles [<options>]
	  -i --in-image         Input: image
	  -n --in-data          Input: native data
	  -p --in-palette       Input: palette (native/json)
	  -d --out-data         Output: native data
	  -o --out-image        Output: image

	Settings:
	  -M --mode             Mode <default: snes>
	  -B --bpp              Bits per pixel
	  -W --tile-width       Tile width
	  -H --tile-height      Tile height
	  -R --no-remap         Don't remap colors <switch>
	  -D --no-discard       Don't discard redundant tiles <switch>
	  -F --no-flip          Don't discard using tile flipping <switch>
	  -S --sprite-mode      Apply sprite output settings <switch>
	  -T --max-tiles        Maximum number of tiles

	  -v --verbose          Verbose logging <switch>
	  -h --help             Show this help <switch>


**superfamiconv map**

	Usage: superfamiconv map [<options>]
	  -i --in-image         Input: image
	  -p --in-palette       Input: palette (json/native)
	  -t --in-tiles         Input: tiles (native)
	  -d --out-data         Output: native data
	  -j --out-json         Output: json
	  -7 --out-m7-data      Output: interleaved map/tile data (snes_mode7)
	  --out-gbc-bank        Output: banked map data (gbc)

	Settings:
	  -M --mode             Mode <default: snes>
	  -B --bpp              Bits per pixel
	  -W --tile-width       Tile width
	  -H --tile-height      Tile height
	  -F --no-flip          Don't use flipped tiles <switch>
	  -T --tile-base-offset Tile base offset for map data
	  --map-width           Map width (in tiles)
	  --map-height          Map height (in tiles)
	  --split-width         Split output into columns of <tiles> width
	  --split-height        Split output into rows of <tiles> height
	  --column-order        Output data in column-major order <switch>

	  -v --verbose          Verbose logging <switch>
	  -h --help             Show this help <switch>


## future work
* Better error diagnostics
* Better documentation and example usage

## acknowledgments
superfamiconv uses the following libraries:

* [{fmt}](http://fmtlib.net) by Victor Zverovich
* [JSON for Modern C++](https://github.com/nlohmann/json) by Niels Lohmann
* [LodePNG](http://lodev.org/lodepng/) by Lode Vandevenne
