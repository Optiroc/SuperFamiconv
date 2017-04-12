//  sfc_map
//  part of superfamiconv
//
//  david lindecrantz <optiroc@gmail.com>

#include <Options.h>
#include "Common.h"
#include "Image.h"
#include "Map.h"
#include "Palette.h"
#include "Tiles.h"

namespace SfcMap {
struct Settings {
  std::string in_image;
  std::string in_palette;
  std::string in_tileset;

  std::string out_data;
  std::string out_json;
  std::string out_m7_data;

  sfc::Mode mode;
  unsigned bpp;
  unsigned tile_w;
  unsigned tile_h;
  bool no_flip;
  unsigned map_w;
  unsigned map_h;
  unsigned map_split_w;
  unsigned map_split_h;
  bool column_order;
};
};

#ifdef SFC_MONOLITH
int sfc_map(int argc, char* argv[]) {
#else
int main(int argc, char* argv[]) {
#endif

  SfcMap::Settings settings = {};
  bool verbose = false;
  bool dummy = false;

  try {
    bool help = false;
    bool license = false;
    std::string mode_str;

    Options options;
    options.IndentDescription = sfc::Constants::options_indent;

#ifdef SFC_MONOLITH
    int min_args = 2;
    options.Header =
      "Usage: superfamiconv map [<options>]\n";
#else
    int min_args = 1;
    options.Header =
      "SuperFamiconv/sfc_map - Create map data from palette and tiles\n"
      "Usage: sfc_map [<options>]\n";
#endif

    // clang-format off
    options.Add(settings.in_image,            'i', "in-image",       "Input: image");
    options.Add(settings.in_palette,          'p', "in-palette",     "Input: palette (json/native)");
    options.Add(settings.in_tileset,          't', "in-tiles",       "Input: tiles (native)");
    options.Add(settings.out_data,            'd', "out-data",       "Output: native data");
    options.Add(settings.out_json,            'j', "out-json",       "Output: json");
    options.Add(settings.out_m7_data,         '7', "out-m7-data",    "Output: interleaved map/tile data");

    options.Add(mode_str,                     'M', "mode",           "Mode",                              std::string("snes"),  "Settings");
    options.Add(settings.bpp,                 'B', "bpp",            "Bits per pixel",                    unsigned(4),          "Settings");
    options.Add(settings.tile_w,              'W', "tile-width",     "Tile width",                        unsigned(8),          "Settings");
    options.Add(settings.tile_h,              'H', "tile-height",    "Tile height",                       unsigned(8),          "Settings");
    options.AddSwitch(settings.no_flip,       'F', "no-flip",        "Don't use flipped tiles",           false,                "Settings");
    options.Add(settings.map_w,              '\0', "map-width",      "Map width (in tiles) <default: inferred>",   unsigned(0), "Settings");
    options.Add(settings.map_h,              '\0', "map-height",     "Map height (in tiles) <default: inferred>",  unsigned(0), "Settings");
    options.Add(settings.map_split_w,        '\0', "split-width",    "Split output into columns of <tiles> width", unsigned(0), "Settings");
    options.Add(settings.map_split_h,        '\0', "split-height",   "Split output into rows of <tiles> height",   unsigned(0), "Settings");
    options.AddSwitch(settings.column_order, '\0', "column-order",   "Output data in column-major order",          false,       "Settings");

    options.AddSwitch(verbose, 'v', "verbose", "Verbose logging", false, "_");
#ifndef SFC_MONOLITH
    options.AddSwitch(license, 'L', "license", "Show license",    false, "_");
#endif
    options.AddSwitch(help,    'h', "help",    "Show this help",  false, "_");

    options.AddSwitch(help,    '?', std::string(), std::string(), false);
    options.AddSwitch(dummy,   '9', std::string(), std::string(), false);
    // clang-format on

    if (argc <= min_args || !options.Parse(argc, argv) || help) {
      std::cout << options.Usage();
      return 0;
    }

    if (license) {
      std::cout << "\nSuperFamiconv/sfc_map\nCopyright (C) 2017 David Lindecrantz\n\n"
                << sfc::LICENSE << '\n';
      return 0;
    }

    settings.mode = sfc::mode(mode_str);

    if (settings.mode == sfc::Mode::snes_mode7 && settings.bpp != 8) {
      settings.bpp = 8;
      if (verbose) std::cout << "Less than 8 bpp not available for snes_mode7: defaulting to 8\n";
    }

    if (!sfc::bpp_allowed_for_mode(settings.bpp, settings.mode)) throw std::runtime_error("bpp setting not compatible with specified mode");

  } catch (const std::exception& e) {
    std::cerr << "Error: " << e.what() << '\n';
    return 1;
  }

  try {
    if (settings.in_image.empty()) throw std::runtime_error("input image required");
    if (settings.in_palette.empty()) throw std::runtime_error("input palette required");
    if (settings.in_tileset.empty()) throw std::runtime_error("input tileset required");

    if (settings.map_split_w == 0) settings.map_split_w = sfc::default_map_size_for_mode(settings.mode);
    if (settings.map_split_h == 0) settings.map_split_h = sfc::default_map_size_for_mode(settings.mode);

    sfc::Image image(settings.in_image);
    if (verbose) std::cout << "Loaded image from \"" << settings.in_image << "\" (" << image << ")\n";

    if (settings.map_w == 0) settings.map_w = sfc::div_ceil(image.width(), settings.tile_w);
    if (settings.map_h == 0) settings.map_h = sfc::div_ceil(image.height(), settings.tile_h);

    if (settings.map_w * settings.tile_w != image.width() || settings.map_h * settings.tile_h != image.height()) {
      image = image.crop(0, 0, settings.map_w * settings.tile_w, settings.map_h * settings.tile_h);
    }

    sfc::Palette palette(settings.in_palette, settings.mode, sfc::palette_size_at_bpp(settings.bpp));
    if (verbose) std::cout << "Loaded palette from \"" << settings.in_palette << "\" (" << palette << ")\n";

    sfc::Tileset tileset(sfc::read_binary(settings.in_tileset), settings.mode, settings.bpp, settings.tile_w, settings.tile_h, settings.no_flip);
    if (verbose) std::cout << "Loaded tiles from \"" << settings.in_tileset << "\" (" << tileset.size() << " tiles)\n";

    std::vector<sfc::Image> crops = image.crops(settings.tile_w, settings.tile_h);
    if (verbose) std::cout << "Mapping " << crops.size() << " " << settings.tile_w << "x" << settings.tile_h << " image slices\n";

    sfc::Map map(settings.mode, settings.map_w, settings.map_h);
    for (unsigned i = 0; i < crops.size(); ++i) {
      map.add(crops[i], tileset, palette, settings.bpp, i % settings.map_w, i / settings.map_w);
    }

    if (verbose && settings.column_order) std::cout << "Using column-major order for output\n";

    if (!settings.out_data.empty()) {
      map.save(settings.out_data, settings.column_order, settings.map_split_w, settings.map_split_h);
      if (verbose) std::cout << "Saved native map data to \"" << settings.out_data << "\"\n";
    }

    if (!settings.out_json.empty()) {
      sfc::write_file(settings.out_json, map.to_json(settings.column_order, settings.map_split_w, settings.map_split_h));
      if (verbose) std::cout << "Saved json map data to \"" << settings.out_json << "\"\n";
    }

    if (!settings.out_m7_data.empty()) {
      std::vector<uint8_t> md = map.native_data();
      std::vector<uint8_t> td = tileset.native_data();

      size_t sz = (td.size() > md.size()) ? td.size() : md.size();
      std::vector<uint8_t> id = std::vector<uint8_t>(sz * 2);
      for (unsigned i = 0; i < md.size(); ++i) id[(i << 1)] = md[i];
      for (unsigned i = 0; i < td.size(); ++i) id[(i << 1) + 1] = td[i];

      sfc::write_file(settings.out_m7_data, id);
      if (verbose) std::cout << "Saved interleaved data to \"" << settings.out_m7_data << "\"\n";
    }

  } catch (const std::exception& e) {
    std::cerr << "Error: " << e.what() << '\n';
    return 1;
  }

  return 0;
}
