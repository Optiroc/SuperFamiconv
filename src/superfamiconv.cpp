// superfamiconv
//
// david lindecrantz <optiroc@me.com>

// TODO: unpack_native_tile() for pce_sprite data
// TODO: Check "shorthand" path for 16x16 tile conversion
// TODO: Check map output with tiles using duplicate colors (#8)
// TODO: Don't always pad native palette output? (Pad every palette but the last? Option?)

#include <Options.h>
#include "About.h"
#include "Color.h"
#include "Common.h"
#include "Image.h"
#include "Map.h"
#include "Palette.h"
#include "Tiles.h"

extern int sfc_palette(int argc, char* argv[]);
extern int sfc_tiles(int argc, char* argv[]);
extern int sfc_map(int argc, char* argv[]);

struct Settings {
  std::string in_image;
  std::string out_palette;
  std::string out_tiles;
  std::string out_map;
  std::string out_palette_image;
  std::string out_palette_act;
  std::string out_tiles_image;
  std::string out_scaled_image;

  sfc::Mode mode;
  unsigned bpp;
  unsigned tile_w;
  unsigned tile_h;

  bool no_remap;
  bool no_discard;
  bool no_flip;
  int tile_base_offset;
  int palette_base_offset;
  bool sprite_mode;
  std::string color_zero;
};

int superfamiconv(int argc, char* argv[]) {
  Settings settings = {};
  bool verbose = false;
  bool col0_forced = false;
  rgba_t col0 = 0;

  try {
    bool help;
    bool license;
    std::string mode_str;

    Options options;
    options.IndentDescription = sfc::Constants::options_indent;

    // clang-format off
    options.Header =
      "Usage: superfamiconv <command> [<options>]\n\n"

      "Available commands: palette, tiles, map or blank for \"shorthand mode\"\n"
      "Invoke with <command> --help for further help\n\n"

      "Shorthand mode options:\n";

    options.Add(settings.in_image,            'i', "in-image",             "Input: image");
    options.Add(settings.out_palette,         'p', "out-palette",          "Output: palette data");
    options.Add(settings.out_tiles,           't', "out-tiles",            "Output: tile data");
    options.Add(settings.out_map,             'm', "out-map",              "Output: map data");
    options.Add(settings.out_palette_image,   '\0', "out-palette-image",    "Output: palette image");
    options.Add(settings.out_palette_act,     '\0', "out-palette-act",      "Output: photoshop palette");
    options.Add(settings.out_tiles_image,     '\0', "out-tiles-image",      "Output: tiles image");
    options.Add(settings.out_scaled_image,    '\0', "out-scaled-image",     "Output: image scaled to destination colorspace");

    options.Add(mode_str,                     'M', "mode",                 "Mode <default: snes>",              std::string("snes"), "Settings");
    options.Add(settings.bpp,                 'B', "bpp",                  "Bits per pixel",                    unsigned(4),         "Settings");
    options.Add(settings.tile_w,              'W', "tile-width",           "Tile width",                        unsigned(8),         "Settings");
    options.Add(settings.tile_h,              'H', "tile-height",          "Tile height",                       unsigned(8),         "Settings");
    options.AddSwitch(settings.no_remap,      'R', "no-remap",             "Don't remap colors",                false,               "Settings");
    options.AddSwitch(settings.no_discard,    'D', "no-discard",           "Don't discard redundant tiles",     false,               "Settings");
    options.AddSwitch(settings.no_flip,       'F', "no-flip",              "Don't discard using tile flipping", false,               "Settings");
    options.Add(settings.tile_base_offset,    'T', "tile-base-offset",     "Tile base offset for map data",     int(0),              "Settings");
    options.Add(settings.palette_base_offset, 'P', "palette-base-offset",  "Palette base offset for map data",  int(0),              "Settings");
    options.AddSwitch(settings.sprite_mode,   'S', "sprite-mode",          "Apply sprite output settings",      false,               "Settings");
    options.Add(settings.color_zero,          '\0', "color-zero",           "Set color #0", std::string(),                           "Settings");

    options.AddSwitch(verbose,                'v', "verbose",              "Verbose logging", false, "_");
    options.AddSwitch(license,                'l', "license",              "Show licenses",   false, "_");
    options.AddSwitch(help,                   'h', "help",                 "Show this help",  false, "_");
    // clang-format on

    if (!options.Parse(argc, argv))
      return 1;

    if (argc <= 1 || help) {
      std::cout << options.Usage();
      return 0;
    }

    if (license) {
      fmt::print("\nSuperFamiconv {}\n{}\n\n{}\n", sfc::about::VERSION, sfc::about::COPYRIGHT, sfc::about::LICENSE);
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

    // Sprite mode defaults
    if (settings.sprite_mode) {
      settings.no_discard = settings.no_flip = true;
    }

    if (!settings.color_zero.empty()) {
      col0 = sfc::from_hexstring(settings.color_zero);
      col0_forced = true;
    }

  } catch (const std::exception& e) {
    fmt::print(stderr, "Error: {}\n", e.what());
    return 1;
  }

  try {
    if (settings.in_image.empty())
      throw std::runtime_error("Input image required");

    if (verbose)
      fmt::print("Performing conversion in \"{}\" mode\n", sfc::mode(settings.mode));

    sfc::Image image(settings.in_image);
    if (verbose)
      fmt::print("Loaded image from \"{}\" ({})\n", settings.in_image, image.description());

    // Write color-scaled image
    if (!settings.out_scaled_image.empty()) {
      image.save_scaled(settings.out_scaled_image, settings.mode);
      if (verbose)
        fmt::print("Saved image scaled to destination colorspace to \"{}\"\n", settings.out_scaled_image);
    }

    if (settings.mode == sfc::Mode::pce_sprite) {
      if (image.width() % 16 || image.height() % 16)
        throw std::runtime_error("pce/sprite-mode requires image dimensions to be a multiple of 16");
    }

    // Make palette
    sfc::Palette palette;
    {
      unsigned palette_count = sfc::default_palette_count_for_mode(settings.mode);
      unsigned colors_per_palette = sfc::palette_size_at_bpp(settings.bpp);

      if (settings.no_remap) {
        if (image.palette_size() == 0)
          throw std::runtime_error("no-remap requires indexed color image");
        if (verbose)
          fmt::print("Mapping palette straight from indexed color image\n");

        palette = sfc::Palette(settings.mode, palette_count, colors_per_palette);
        palette.add_colors(image.palette());

      } else {
        if (verbose)
          fmt::print("Mapping optimized palette ({}x{} entries)\n", palette_count, colors_per_palette);

        palette = sfc::Palette(settings.mode, palette_count, colors_per_palette);

        col0 = col0_forced ? col0 : image.crop(0, 0, 1, 1, settings.mode).rgba_data()[0];

        if (settings.sprite_mode) {
          if (verbose)
            fmt::print("Setting color zero to transparent\n");
          palette.prime_col0(sfc::transparent_color);
        } else if (col0_forced || sfc::col0_is_shared_for_mode(settings.mode)) {
          if (verbose)
            fmt::print("Setting color zero to {}\n", sfc::to_hexstring(col0, true, true));
          palette.prime_col0(col0);
        }

        palette.add_images(image.crops(settings.tile_w, settings.tile_h, settings.mode));
        palette.sort();
      }
      if (verbose)
        fmt::print("Created palette with {}\n", palette.description());
    }

    // Make tileset
    sfc::Tileset tileset(settings.mode, settings.bpp, settings.tile_w, settings.tile_h, settings.no_discard, settings.no_flip,
                         settings.no_remap, sfc::max_tile_count_for_mode(settings.mode));
    {
      std::vector<sfc::Image> crops = image.crops(settings.tile_w, settings.tile_h, settings.mode);

      for (auto& crop : crops)
        tileset.add(crop, &palette);
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

    // Make map
    unsigned map_width = sfc::div_ceil(image.width(), settings.tile_w);
    unsigned map_height = sfc::div_ceil(image.height(), settings.tile_h);

    if (map_width * settings.tile_w != image.width() || map_height * settings.tile_h != image.height()) {
      image = image.crop(0, 0, map_width * settings.tile_w, map_height * settings.tile_h, settings.mode);
    }

    sfc::Map map(settings.mode, map_width, map_height, settings.tile_w, settings.tile_h);
    if (settings.mode != sfc::Mode::pce_sprite) {
      std::vector<sfc::Image> crops = image.crops(settings.tile_w, settings.tile_h, settings.mode);
      if (verbose)
        fmt::print("Mapping {} {}x{}px tiles from image\n", crops.size(), settings.tile_w, settings.tile_h);

      for (unsigned i = 0; i < crops.size(); ++i) {
        map.add(crops[i], tileset, palette, settings.bpp, i % map_width, i / map_width);
      }

      if (settings.tile_base_offset)
        map.add_base_offset(settings.tile_base_offset);

      if (settings.palette_base_offset)
        map.add_palette_base_offset(settings.palette_base_offset);
    }

    // Write data
    if (!settings.out_palette.empty()) {
      palette.save(settings.out_palette);
      if (verbose)
        fmt::print("Saved native palette data to \"{}\"\n", settings.out_palette);
    }

    if (!settings.out_tiles.empty()) {
      tileset.save(settings.out_tiles);
      if (verbose)
        fmt::print("Saved native tile data to \"{}\"\n", settings.out_tiles);
    }

    if (!settings.out_map.empty()) {
      if (settings.mode == sfc::Mode::pce_sprite) {
        fmt::print(stderr, "Map output not available in pce_sprite mode\n");
      } else {
        map.save(settings.out_map);
        if (verbose)
          fmt::print("Saved native map data to \"{}\"\n", settings.out_map);
      }
    }

    if (!settings.out_palette_act.empty()) {
      palette.save_act(settings.out_palette_act);
      if (verbose)
        fmt::print("Saved photoshop palette to \"{}\"\n", settings.out_palette_act);
    }

    if (!settings.out_palette_image.empty()) {
      sfc::Image palette_image(palette);
      palette_image.save(settings.out_palette_image);
      if (verbose)
        fmt::print("Saved palette image to \"{}\"\n", settings.out_palette_image);
    }

    if (!settings.out_tiles_image.empty()) {
      sfc::Image tileset_image(tileset);
      tileset_image.save(settings.out_tiles_image);
      if (verbose)
        fmt::print("Saved tileset image to \"{}\"\n", settings.out_tiles_image);
    }

  } catch (const std::exception& e) {
    fmt::print(stderr, "Error: {}\n", e.what());
    return 1;
  }

  return 0;
}

int main(int argc, char* argv[]) {
  // If first argument is a subcommand, remove it and pass along
  if (argc > 1 && std::strcmp(argv[1], "palette") == 0) {
    std::strcpy(argv[1], "");
    return sfc_palette(argc, argv);

  } else if (argc > 1 && std::strcmp(argv[1], "tiles") == 0) {
    std::strcpy(argv[1], "");
    return sfc_tiles(argc, argv);

  } else if (argc > 1 && std::strcmp(argv[1], "map") == 0) {
    std::strcpy(argv[1], "");
    return sfc_map(argc, argv);

  } else {
    return superfamiconv(argc, argv);
  }
}
