// sfc_palette
// part of superfamiconv
//
// david lindecrantz <optiroc@gmail.com>

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
  std::string color_zero;
};
};

int sfc_palette(int argc, char* argv[]) {
  SfcPalette::Settings settings = {};
  bool verbose = false;
  bool col0_forced = false;
  rgba_t col0 = 0;

  try {
    bool help = false;
    bool license = false;
    std::string mode_str;
    bool dummy = false;

    Options options;
    options.IndentDescription = sfc::Constants::options_indent;

#ifdef SFC_MONOLITH
    int min_args = 2;
    options.Header =
      "Usage: superfamiconv palette [<options>]\n";
#else
    int min_args = 1;
    options.Header =
      "SuperFamiconv/sfc_palette - Create palette data from image\n"
      "Usage: sfc_palette [<options>]\n";
#endif

    // clang-format off
    options.Add(settings.in_image,           'i', "in-image",       "Input: image");
    options.Add(settings.out_data,           'd', "out-data",       "Output: native data");
    options.Add(settings.out_act,            'a', "out-act",        "Output: act palette");
    options.Add(settings.out_json,           'j', "out-json",       "Output: json");
    options.Add(settings.out_image,          'o', "out-image",      "Output: image");

    options.Add(mode_str,                    'M', "mode",           "Mode",                             std::string("snes"), "Settings");
    options.Add(settings.palettes,           'P', "palettes",       "Number of subpalettes",            unsigned(8),         "Settings");
    options.Add(settings.colors,             'C', "colors",         "Colors per subpalette",            unsigned(16),        "Settings");
    options.Add(settings.tile_w,             'W', "tile-width",     "Tile width",                       unsigned(8),         "Settings");
    options.Add(settings.tile_h,             'H', "tile-height",    "Tile height",                      unsigned(8),         "Settings");
    options.AddSwitch(settings.no_remap,     'R', "no-remap",       "Don't remap colors",               false,               "Settings");
    options.Add(settings.color_zero,         '0', "color-zero",     "Set color #0 <default: color at 0,0>", std::string(),   "Settings");

    options.AddSwitch(verbose, 'v', "verbose", "Verbose logging", false, "_");
#ifndef SFC_MONOLITH
    options.AddSwitch(license, 'L', "license", "Show license",    false, "_");
#endif
    options.AddSwitch(help,    'h', "help",    "Show this help",  false, "_");

    options.AddSwitch(help,    '?', std::string(), std::string(), false);
    options.AddSwitch(dummy,   '9', std::string(), std::string(), false);
    // clang-format on

    if (argc <= min_args || !options.Parse(argc, argv) || help) {
      fmt::print(options.Usage());
      return 0;
    }

    if (license) {
      fmt::print("\nSuperFamiconv/sfc_palette {}\n{}\n\n{}\n", sfc::VERSION, sfc::COPYRIGHT, sfc::LICENSE);
      return 0;
    }

    settings.mode = sfc::mode(mode_str);

    // Mode-specific defaults
    if (!options.WasSet("palettes")) settings.palettes = sfc::max_palette_count_for_mode(settings.mode);
    if (!options.WasSet("colors")) settings.colors = sfc::palette_size_at_bpp(sfc::default_bpp_for_mode(settings.mode));

    if (!settings.color_zero.empty()) {
      col0 = sfc::from_hexstring(settings.color_zero);
      col0_forced = true;
    }

  } catch (const std::exception& e) {
    fmt::print(stderr, "Error: {}\n", e.what());
    return 1;
  }

  try {
    if (settings.in_image.empty()) throw std::runtime_error("Input image required");

    sfc::Image image(settings.in_image);
    if (verbose) fmt::print("Loaded image from \"{}\" ({})\n", settings.in_image, image.description());

    sfc::Palette palette;

    if (settings.no_remap) {
      if (image.palette_size() == 0) throw std::runtime_error("no-remap requires indexed color image");
      if (verbose) fmt::print("Mapping palette straight from indexed color image\n");

      palette = sfc::Palette(settings.mode, 1, (unsigned)image.palette_size());
      palette.add_noremap(image.palette());

    } else {
      if (verbose) fmt::print("Mapping optimized palette ({}x{} entries for {}x{} tiles)\n",
                              settings.palettes, settings.colors, settings.tile_w, settings.tile_h);

      palette = sfc::Palette(settings.mode, settings.palettes, settings.colors);

      col0 = col0_forced ? col0 : image.crop(0, 0, 1, 1).rgba_data()[0];
      if (verbose) fmt::print("Setting color zero to {}\n", sfc::to_hexstring(col0, true, true));
      palette.add(col0);

      palette.add(image.image_crops(settings.tile_w, settings.tile_h));
    }

    if (verbose) fmt::print("Generated palette with {}\n", palette.description());

    if (!settings.no_remap) {
      palette.sort();
    }

    if (!settings.out_data.empty()) {
      palette.save(settings.out_data);
      if (verbose) fmt::print("Saved native palette data to \"{}\"\n", settings.out_data);
    }

    if (!settings.out_act.empty()) {
      palette.save_act(settings.out_act);
      if (verbose) fmt::print("Saved ACT palette to \"{}\"\n", settings.out_act);
    }

    if (!settings.out_image.empty()) {
      sfc::Image palette_image(palette);
      palette_image.save(settings.out_image);
      if (verbose) fmt::print("Saved palette image to \"{}\"\n", settings.out_image);
    }

    if (!settings.out_json.empty()) {
      sfc::write_file(settings.out_json, palette.to_json());
      if (verbose) fmt::print("Saved json data to \"{}\"\n", settings.out_json);
    }

  } catch (const std::exception& e) {
    fmt::print(stderr, "Error: {}\n", e.what());
    return 1;
  }

  return 0;
}

#ifndef SFC_MONOLITH
int main(int argc, char* argv[]) {
  return sfc_palette(argc, argv);
}
#endif
