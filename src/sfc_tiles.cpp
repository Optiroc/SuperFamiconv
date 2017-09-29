// sfc_tiles
// part of superfamiconv
//
// david lindecrantz <optiroc@gmail.com>

#include <Options.h>
#include "Common.h"
#include "Image.h"
#include "Palette.h"
#include "Tiles.h"

namespace SfcTiles {
struct Settings {
  std::string in_image;
  std::string in_data;
  std::string in_palette;
  std::string out_data;
  std::string out_image;

  sfc::Mode mode;
  unsigned bpp;
  bool no_discard;
  bool no_flip;
  unsigned tile_w;
  unsigned tile_h;
  bool no_remap;
  unsigned max_tiles;
};
};

#ifdef SFC_MONOLITH
int sfc_tiles(int argc, char* argv[]) {
#else
int main(int argc, char* argv[]) {
#endif

  SfcTiles::Settings settings = {};
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
      "Usage: superfamiconv tiles [<options>]\n";
#else
    int min_args = 1;
    options.Header =
      "SuperFamiconv/sfc_tiles - Create tile data from image\n"
      "Usage: sfc_tiles [<options>]\n";
#endif

    // clang-format off
    options.Add(settings.in_image,           'i', "in-image",       "Input: image");
    options.Add(settings.in_data,            'n', "in-data",        "Input: native data");
    options.Add(settings.in_palette,         'p', "in-palette",     "Input: palette (native/json)");
    options.Add(settings.out_data,           'd', "out-data",       "Output: native data");
    options.Add(settings.out_image,          'o', "out-image",      "Output: image");

    options.Add(mode_str,                    'M', "mode",           "Mode",                              std::string("snes"), "Settings");
    options.Add(settings.bpp,                'B', "bpp",            "Bits per pixel",                    unsigned(4),         "Settings");
    options.AddSwitch(settings.no_discard,   'D', "no-discard",     "Don't discard redundant tiles",     false,               "Settings");
    options.AddSwitch(settings.no_flip,      'F', "no-flip",        "Don't discard using tile flipping", false,               "Settings");
    options.Add(settings.tile_w,             'W', "tile-width",     "Tile width",                        unsigned(8),         "Settings");
    options.Add(settings.tile_h,             'H', "tile-height",    "Tile height",                       unsigned(8),         "Settings");
    options.AddSwitch(settings.no_remap,     'R', "no-remap",       "Don't remap colors",                false,               "Settings");
    options.Add(settings.max_tiles,          'T', "max-tiles",      "Maximum number of tiles",           unsigned(),          "Settings");

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
      std::cout << "\nSuperFamiconv/sfc_tiles\nCopyright (C) 2017 David Lindecrantz\n\n"
                << sfc::LICENSE << '\n';
      return 0;
    }

    settings.mode = sfc::mode(mode_str);

    if (settings.mode == sfc::Mode::snes_mode7 && settings.bpp != 8) {
      settings.bpp = 8;
      if (verbose) std::cout << "Less than 8 bpp not available for snes_mode7: defaulting to 8\n";
    }

    if (!sfc::bpp_allowed_for_mode(settings.bpp, settings.mode)) throw std::runtime_error("bpp setting not allowed for specified mode");

    if (!sfc::tile_width_allowed_for_mode(settings.tile_w, settings.mode)) {
      settings.tile_w = sfc::default_tile_size_for_mode(settings.mode);
      if (verbose) std::cout << "Tile width not allowed for specified mode, using default (" << settings.tile_w << ")\n";
    }

    if (!sfc::tile_height_allowed_for_mode(settings.tile_h, settings.mode)) {
      settings.tile_h = sfc::default_tile_size_for_mode(settings.mode);
      if (verbose) std::cout << "Tile height not allowed for specified mode, using default (" << settings.tile_h << ")\n";
    }

    if (settings.mode == sfc::Mode::snes_mode7 && !settings.no_flip) {
      settings.no_flip = true;
      if (verbose) std::cout << "Tile flipping not available for snes_mode7: converting with no-flip enabled\n";
    }

  } catch (const std::exception& e) {
    std::cerr << "Error: " << e.what() << '\n';
    return 1;
  }

  try {
    if (settings.in_image.empty() && settings.in_data.empty()) throw std::runtime_error("Input image or native data required");

    sfc::Tileset tileset;

    if (!settings.in_data.empty()) {
      // Native data input
      tileset = sfc::Tileset(sfc::read_binary(settings.in_data), settings.mode, settings.bpp, settings.tile_w, settings.tile_h, settings.no_flip);
      if (verbose) std::cout << "Loaded tiles from \"" << settings.in_data << "\" (" << tileset.size() << " tiles)\n";

    } else {
      // Image input
      sfc::Image image(settings.in_image);
      if (verbose) std::cout << "Loaded image from \"" << settings.in_image << "\" (" << image << ")\n";

      std::vector<sfc::Image> crops = image.crops(settings.tile_w, settings.tile_h);
      if (verbose) std::cout << "Image sliced into " << crops.size() << " " << settings.tile_w << "x" << settings.tile_h << " tiles\n";

      sfc::Palette palette;
      tileset = sfc::Tileset(settings.mode, settings.bpp, settings.tile_w, settings.tile_h, settings.no_discard,
                             settings.no_flip, settings.no_remap, settings.max_tiles);

      if (settings.no_remap) {
        if (image.palette_size() == 0) throw std::runtime_error("\"--no-remap\" requires indexed color image");
        if (verbose) std::cout << "Creating tile data straight from color indices\n";

      } else {
        if (settings.in_palette.empty()) throw std::runtime_error("Input palette required (except in --no-remap mode)");
        palette = sfc::Palette(settings.in_palette, settings.mode, sfc::palette_size_at_bpp(settings.bpp));
        if (verbose) std::cout << "Remapping tile data from palette \"" << settings.in_palette << "\" (" << palette << ")\n";
      }

      for (auto& img : crops) tileset.add(img, &palette);
      if (verbose) {
        if (settings.no_discard) {
          std::cout << "Created tileset with " << tileset.size() << " tiles\n";
        } else {
          std::cout << "Created optimized tileset with " << tileset.size()
                    << " tiles (discarded " << tileset.discarded_tiles << " redudant tiles)\n";
        }
      }
    }

    if (!settings.out_data.empty()) {
      tileset.save(settings.out_data);
      if (verbose) std::cout << "Saved native tile data to \"" << settings.out_data << "\"\n";
    }

    if (!settings.out_image.empty()) {
      sfc::Image tileset_image(tileset);
      if (!settings.in_data.empty()) {
        tileset_image.save_indexed(settings.out_image);
      } else {
        tileset_image.save(settings.out_image);
      }
      if (verbose) std::cout << "Saved tileset image to \"" << settings.out_image << "\"\n";
    }

  } catch (const std::exception& e) {
    std::cerr << "Error: " << e.what() << '\n';
    return 1;
  }

  return 0;
}
