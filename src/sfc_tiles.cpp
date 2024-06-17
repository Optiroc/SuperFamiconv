// sfc_tiles
// part of superfamiconv
//
// david lindecrantz <optiroc@me.com>

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
  bool sprite_mode;
  unsigned max_tiles;
  unsigned out_image_width;
};
}; // namespace SfcTiles

int sfc_tiles(int argc, char* argv[]) {
  SfcTiles::Settings settings = {};
  bool verbose = false;

  try {
    bool help = false;
    std::string mode_str;

    Options options;
    options.IndentDescription = sfc::Constants::options_indent;
    options.Header = "Usage: superfamiconv tiles [<options>]\n";

    // clang-format off
    options.Add(settings.in_image,           'i', "in-image",       "Input: image");
    options.Add(settings.in_data,            'n', "in-data",        "Input: native data");
    options.Add(settings.in_palette,         'p', "in-palette",     "Input: palette (native/json)");
    options.Add(settings.out_data,           'd', "out-data",       "Output: native data");
    options.Add(settings.out_image,          'o', "out-image",      "Output: image");

    options.Add(mode_str,                    'M', "mode",           "Mode <default: snes>",              std::string("snes"), "Settings");
    options.Add(settings.bpp,                'B', "bpp",            "Bits per pixel",                    unsigned(4),         "Settings");
    options.Add(settings.tile_w,             'W', "tile-width",     "Tile width",                        unsigned(8),         "Settings");
    options.Add(settings.tile_h,             'H', "tile-height",    "Tile height",                       unsigned(8),         "Settings");
    options.AddSwitch(settings.no_remap,     'R', "no-remap",       "Don't remap colors",                false,               "Settings");
    options.AddSwitch(settings.no_discard,   'D', "no-discard",     "Don't discard redundant tiles",     false,               "Settings");
    options.AddSwitch(settings.no_flip,      'F', "no-flip",        "Don't discard using tile flipping", false,               "Settings");
    options.AddSwitch(settings.sprite_mode,  'S', "sprite-mode",    "Apply sprite output settings",      false,               "Settings");
    options.Add(settings.max_tiles,          'T', "max-tiles",      "Maximum number of tiles",           unsigned(),          "Settings");
    options.Add(settings.out_image_width,   '\0', "out-image-width","Width of out-image",                unsigned(),          "Settings");

    options.AddSwitch(verbose,               'v', "verbose",        "Verbose logging", false, "_");
    options.AddSwitch(help,                  'h', "help",           "Show this help",  false, "_");
    // clang-format on

    if (!options.Parse(argc, argv))
      return 1;

    if (argc <= 2 || help) {
      std::cout << options.Usage();
      return 0;
    }

    settings.mode = sfc::mode(mode_str);

    // Set pce_sprite mode and sprite_mode interchangeably
    if (settings.sprite_mode && settings.mode == sfc::Mode::pce)
      settings.mode = sfc::Mode::pce_sprite;
    if (settings.mode == sfc::Mode::pce_sprite)
      settings.sprite_mode = true;

    // Mode-specific defaults
    if (!options.WasSet("bpp"))
      settings.bpp = sfc::default_bpp_for_mode(settings.mode);
    if (!options.WasSet("tile-width"))
      settings.tile_w = sfc::default_tile_size_for_mode(settings.mode);
    if (!options.WasSet("tile-height"))
      settings.tile_h = sfc::default_tile_size_for_mode(settings.mode);
    if (!options.WasSet("no-flip"))
      settings.no_flip = !sfc::tile_flipping_allowed_for_mode(settings.mode);
    if (!options.WasSet("max-tiles"))
      settings.max_tiles = sfc::max_tile_count_for_mode(settings.mode);

    if (!sfc::tile_width_allowed_for_mode(settings.tile_w, settings.mode)) {
      settings.tile_w = sfc::default_tile_size_for_mode(settings.mode);
      if (verbose)
        fmt::print("Tile width not allowed for specified mode, using default ({})\n", settings.tile_w);
    }

    if (!sfc::tile_height_allowed_for_mode(settings.tile_h, settings.mode)) {
      settings.tile_h = sfc::default_tile_size_for_mode(settings.mode);
      if (verbose)
        fmt::print("Tile height not allowed for specified mode, using default ({})\n", settings.tile_h);
    }

    // Sprite mode defaults
    if (settings.sprite_mode) {
      settings.no_discard = settings.no_flip = true;
    }

    if (!sfc::bpp_allowed_for_mode(settings.bpp, settings.mode))
      throw std::runtime_error("bpp setting not allowed for specified mode");

  } catch (const std::exception& e) {
    fmt::print(stderr, "Error: {}\n", e.what());
    return 1;
  }

  try {
    if (settings.in_image.empty() && settings.in_data.empty())
      throw std::runtime_error("Input image or native data required");

    if (verbose)
      fmt::print("Performing tiles operation in \"{}\" mode\n", sfc::mode(settings.mode));

    sfc::Tileset tileset;

    if (!settings.in_data.empty()) {
      // Native data input
      tileset = sfc::Tileset(sfc::read_binary(settings.in_data), settings.mode, settings.bpp, settings.tile_w, settings.tile_h,
                             settings.no_flip);
      if (verbose)
        fmt::print("Loaded tiles from \"{}\" ({} tiles)\n", settings.in_data, tileset.size());

    } else {
      // Image input
      sfc::Image image(settings.in_image);
      std::vector<sfc::Image> crops = image.crops(settings.tile_w, settings.tile_h, settings.mode);
      if (verbose)
        fmt::print("Loaded image from \"{}\" ({})\n", settings.in_image, image.description());

      if (settings.mode == sfc::Mode::pce && settings.sprite_mode) {
        if (image.width() % 16 || image.height() % 16)
          throw std::runtime_error("pce/sprite-mode requires image dimensions to be a multiple of 16");
      }

      if (verbose)
        fmt::print("Image sliced into {} {}x{}px tiles\n", crops.size(), settings.tile_w, settings.tile_h);

      sfc::Palette palette;
      tileset = sfc::Tileset(settings.mode, settings.bpp, settings.tile_w, settings.tile_h, settings.no_discard, settings.no_flip,
                             settings.no_remap, settings.max_tiles);

      if (settings.no_remap) {
        if (image.palette_size() == 0)
          throw std::runtime_error("\"--no-remap\" requires indexed color image");
        if (verbose)
          fmt::print("Creating tile data straight from color indices\n");

      } else {
        if (settings.in_palette.empty())
          throw std::runtime_error("Input palette required (except in --no-remap mode)");
        palette = sfc::Palette(settings.in_palette, settings.mode, sfc::palette_size_at_bpp(settings.bpp));
        if (palette.size() < 1)
          throw std::runtime_error("Input palette size is zero");
        if (verbose)
          fmt::print("Remapping tile data from palette \"{}\" ({})\n", settings.in_palette, palette.description());
      }

      for (auto& img : crops)
        tileset.add(img, &palette);
      if (tileset.is_full()) {
        throw std::runtime_error(
          fmt::format("Tileset exceeds maximum size ({} entries generated, {} maximum)", tileset.size(), tileset.max()));
      }
      if (verbose) {
        if (settings.no_discard) {
          fmt::print("Created tileset with {} entries\n", tileset.size());
        } else {
          fmt::print("Created optimized tileset with {} entries (discarded {} redundant tiles)\n", tileset.size(),
                     tileset.discarded_tiles);
        }
      }
    }

    // Write data
    if (!settings.out_data.empty()) {
      tileset.save(settings.out_data);
      if (verbose)
        fmt::print("Saved native tile data to \"{}\"\n", settings.out_data);
    }

    if (!settings.out_image.empty()) {
      sfc::Image tileset_image(tileset, settings.out_image_width);
      if (!settings.in_data.empty()) {
        tileset_image.save_indexed(settings.out_image);
      } else {
        tileset_image.save(settings.out_image);
      }
      if (verbose)
        fmt::print("Saved tileset image to \"{}\"\n", settings.out_image);
    }

  } catch (const std::exception& e) {
    fmt::print(stderr, "Error: {}\n", e.what());
    return 1;
  }

  return 0;
}
