# superfamiconv
A tile graphics converter with flexible and composable command line options.

Developed by David Lindecrantz and distributed under the terms of the [MIT license](./LICENSE).


## dependencies
A C++11 capable compiler.

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
	--out-tiles-image     Output: tiles image

	-M --mode             Mode <default: snes>
	-B --bpp              Bits per pixel <default: 4>
	--tile-width          Tile width <default: 8>
	--tile-height         Tile height <default: 8>
	--map-width           Map width (in tiles) <default: inferred>
	--map-height          Map height (in tiles) <default: inferred>
	--no-discard          Don't discard redundant tiles <switch>
	--no-flip             Don't discard using tile flipping <switch>
	--color-zero          Set color #0 <default: color at 0,0>

	-v --verbose          Verbose logging <switch>
	-L --license          Show license <switch>
	-h --help             Show this help <switch>

This command mode accepts one image (either indexed, RGB or RGBA mode PNG – which are the formats supported for all image inputs) and outputs palette, tile and/or map data.

The `mode` option, which is common for all commands, affects the color space handling and binary output format. It takes one of the following arguments: 

* `snes` 
* `snes_mode7` 

More will be added as I (or you!) need them.

Example:

	superfamiconv -v --in-image snes.png --out-palette snes.palette --out-tiles snes.tiles --out-map snes.map --out-tiles-image tiles.png
	Loaded image from "snes.png" (256x224, indexed color)
	Mapping optimized palette (16x16 color palettes, 8x8 tiles)
	Setting color zero to #505050
	Generated palette with [16,8] colors, 24 total
	Created optimized tileset with 156 tiles (discarded 740 redudant tiles)
	Mapping 896 8x8 image slices
	Saved native palette data to "snes.palette"
	Saved native tile data to "snes.tiles"
	Saved native map data to "snes.map"
	Saved tileset image to "tiles.png"


For more flexibility use the sub commands, which have the following options respectively:

**superfamiconv palette**

	superfamiconv palette [<options>]
	-i --in-image         Input: image
	-d --out-data         Output: native data
	-j --out-json         Output: json
	-o --out-image        Output: image
	
	-M --mode             Mode <default: snes>
	-P --palettes         Number of subpalettes <default: 8>
	-C --colors           Colors per subpalette <default: 16>
	-W --tile-width       Tile width <default: 8>
	-H --tile-height      Tile height <default: 8>
	-R --no-remap         Don't remap colors <switch>
	-0 --color-zero       Set color #0 <default: color at 0,0>
	
	-v --verbose          Verbose logging <switch>
	-h --help             Show this help <switch>

**superfamiconv tiles**

	superfamiconv tiles [<options>]
	-i --in-image         Input: image
	-p --in-palette       Input: palette (native/json)
	-d --out-data         Output: native data
	-o --out-image        Output: image
	
	-M --mode             Mode <default: snes>
	-B --bpp              Bits per pixel <default: 4>
	-D --no-discard       Don't discard redundant tiles <switch>
	-F --no-flip          Don't discard using tile flipping <switch>
	-W --tile-width       Tile width <default: 8>
	-H --tile-height      Tile height <default: 8>
	-R --no-remap         Don't remap colors <switch>
	-T --max-tiles        Maximum number of tiles
	
	-v --verbose          Verbose logging <switch>
	-h --help             Show this help <switch>


**superfamiconv map**

	superfamiconv map [<options>]
	-i --in-image         Input: image
	-p --in-palette       Input: palette (json/native)
	-t --in-tiles         Input: tiles (native)
	-d --out-data         Output: native data
	-j --out-json         Output: json
	-7 --out-m7-data      Output: interleaved map/tile data

	-M --mode             Mode <default: snes>
	-B --bpp              Bits per pixel <default: 4>
	-W --tile-width       Tile width <default: 8>
	-H --tile-height      Tile height <default: 8>
	-F --no-flip          Don't use flipped tiles <switch>
	--map-width           Map width (in tiles) <default: inferred>
	--map-height          Map height (in tiles) <default: inferred>
	--split-width         Split output into columns of <tiles> width
	--split-height        Split output into rows of <tiles> height
	--column-order        Output data in column-major order <switch>
	
	-v --verbose          Verbose logging <switch>
	-h --help             Show this help <switch>


## future work
* More output formats
* Error diagnostics
* Better documentation and example usage

## acknowledgments
superfamiconv uses the following libraries:

* [LodePNG](http://lodev.org/lodepng/) by Lode Vandevenne
* [JSON for Modern C++](https://github.com/nlohmann/json) by Niels Lohmann
