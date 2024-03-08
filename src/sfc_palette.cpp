// sfc_palette
// part of superfamiconv
//
// david lindecrantz <optiroc@me.com>

#include <Options.h>
#include "Common.h"
#include "Image.h"
#include "Palette.h"

namespace SfcPalette {
struct Settings {
  std::string in_image;
  std::string out_data;
  std::string out_act;
  std::string out_json;
  std::string out_image;

  sfc::Mode mode;
  unsigned palettes;
  unsigned colors;
  unsigned tile_w;
  unsigned tile_h;
  bool no_remap;
  bool sprite_mode;
  std::string color_zero;
};
}; // namespace SfcPalette

int sfc_palette(int argc, char* argv[]) {
  SfcPalette::Settings settings = {};
  bool verbose = false;
  bool col0_forced = false;
  rgba_t col0 = 0;

  try {
    bool help = false;
    std::string mode_str;

    Options options;
    options.IndentDescription = sfc::Constants::options_indent;
    options.Header = "Usage: superfamiconv palette [<options>]\n";

    // clang-format off
    options.Add(settings.in_image,           'i', "in-image",       "Input: image");
    options.Add(settings.out_data,           'd', "out-data",       "Output: native data");
    options.Add(settings.out_act,            'a', "out-act",        "Output: photoshop palette");
    options.Add(settings.out_json,           'j', "out-json",       "Output: json");
    options.Add(settings.out_image,          'o', "out-image",      "Output: image");

    options.Add(mode_str,                    'M', "mode",           "Mode <default: snes>",             std::string("snes"), "Settings");
    options.Add(settings.palettes,           'P', "palettes",       "Number of subpalettes",            unsigned(8),         "Settings");
    options.Add(settings.colors,             'C', "colors",         "Colors per subpalette",            unsigned(16),        "Settings");
    options.Add(settings.tile_w,             'W', "tile-width",     "Tile width",                       unsigned(8),         "Settings");
    options.Add(settings.tile_h,             'H', "tile-height",    "Tile height",                      unsigned(8),         "Settings");
    options.AddSwitch(settings.no_remap,     'R', "no-remap",       "Don't remap colors",               false,               "Settings");
    options.AddSwitch(settings.sprite_mode,  'S', "sprite-mode",    "Apply sprite output settings",     false,               "Settings");
    options.Add(settings.color_zero,         '0', "color-zero",     "Set color #0",                     std::string(),       "Settings");

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
    if (!options.WasSet("palettes"))
      settings.palettes = sfc::default_palette_count_for_mode(settings.mode);
    if (!options.WasSet("colors"))
      settings.colors = sfc::palette_size_at_bpp(sfc::default_bpp_for_mode(settings.mode));
    if (!options.WasSet("tile-width"))
      settings.tile_w = sfc::default_tile_size_for_mode(settings.mode);
    if (!options.WasSet("tile-height"))
      settings.tile_h = sfc::default_tile_size_for_mode(settings.mode);

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
      fmt::print("Performing palette operation in \"{}\" mode\n", sfc::mode(settings.mode));

    sfc::Image image(settings.in_image);
    if (verbose)
      fmt::print("Loaded image from \"{}\" ({})\n", settings.in_image, image.description());

    sfc::Palette palette;

    if (settings.no_remap) {
      if (image.palette_size() == 0)
        throw std::runtime_error("no-remap requires indexed color image");
      if (verbose)
        fmt::print("Mapping palette straight from indexed color image\n");

      palette = sfc::Palette(settings.mode, 1, (unsigned)image.palette_size());
      palette.add_colors(image.palette());

    } else {
      if (verbose)
        fmt::print("Mapping optimized palette ({}x{} entries)\n", settings.palettes, settings.colors);

      palette = sfc::Palette(settings.mode, settings.palettes, settings.colors);

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
    }

    if (verbose)
      fmt::print("Created palette with {}\n", palette.description());

    if (!settings.no_remap) {
      palette.sort();
    }

    // Write data
    if (!settings.out_data.empty()) {
      palette.save(settings.out_data);
      if (verbose)
        fmt::print("Saved native palette data to \"{}\"\n", settings.out_data);
    }

    if (!settings.out_act.empty()) {
      palette.save_act(settings.out_act);
      if (verbose)
        fmt::print("Saved photoshop palette to \"{}\"\n", settings.out_act);
    }

    if (!settings.out_image.empty()) {
      sfc::Image palette_image(palette);
      palette_image.save(settings.out_image);
      if (verbose)
        fmt::print("Saved palette image to \"{}\"\n", settings.out_image);
    }

    if (!settings.out_json.empty()) {
      sfc::write_file(settings.out_json, palette.to_json());
      if (verbose)
        fmt::print("Saved json data to \"{}\"\n", settings.out_json);
    }

  } catch (const std::exception& e) {
    fmt::print(stderr, "Error: {}\n", e.what());
    return 1;
  }

  return 0;
}
