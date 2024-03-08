// palette representation
//
// david lindecrantz <optiroc@me.com>

#pragma once

#include "Common.h"
#include "Image.h"
#include "Mode.h"

namespace sfc {

struct Image;

struct Subpalette final {
  Subpalette(Mode mode, unsigned max_colors) : _mode(mode), _max_colors(max_colors){};

  Mode mode() const { return _mode; }
  bool is_full() const { return _colors.size() == _max_colors; }

  rgba_t color_at(unsigned index) const { return _colors[index]; }
  const rgba_vec_t colors() const { return _colors; }
  const rgba_vec_t normalized_colors() const { return normalize_colors(_colors, _mode); }

  void add(rgba_t color, bool add_duplicates = false);
  void add(const rgba_vec_t& new_colors, bool add_duplicates = false, bool overwrite = false);
  void set(unsigned index, const rgba_t color);

  Subpalette padded() const;
  unsigned diff(const rgba_set_t& new_colors) const;
  void sort();
  bool check_col0_duplicates();

private:
  Mode _mode = Mode::snes;
  unsigned _max_colors;

  rgba_vec_t _colors;
  rgba_set_t _colors_set;
};

struct Palette final {
  Palette(Mode mode = Mode::snes, unsigned max_subpalettes = 0, unsigned max_colors = 0)
      : _mode(mode), _max_subpalettes(max_subpalettes), _max_colors_per_subpalette(max_colors){};

  Palette(const byte_vec_t& native_data, Mode in_mode = Mode::snes, unsigned colors_per_subpalette = 16);
  Palette(const std::string& path, Mode in_mode = Mode::snes, unsigned colors_per_subpalette = 16);

  unsigned max_colors_per_subpalette() const { return _max_colors_per_subpalette; }
  unsigned size() const;
  const std::vector<rgba_vec_t> colors() const;
  const std::vector<rgba_vec_t> normalized_colors() const;

  void set_color(unsigned index, const rgba_t color);
  void prime_col0(const rgba_t color);
  void check_col0_duplicates();

  void add_images(std::vector<sfc::Image>);
  void add_colors(const rgba_vec_t& colors, bool reduce_depth = true);

  int index_of(const Subpalette& subpalette) const;
  const Subpalette& subpalette_matching(const Image& image) const;
  std::vector<const Subpalette*> subpalettes_matching(const Image& image) const;

  void sort();

  const std::string description() const;
  const std::string to_json() const;
  void save(const std::string& path) const;
  void save_act(const std::string& path) const;

private:
  Mode _mode = Mode::snes;
  unsigned _max_subpalettes = 0;
  unsigned _max_colors_per_subpalette = 0;
  std::vector<Subpalette> _subpalettes;

  rgba_t _col0 = 0;
  bool _col0_is_shared = false;

  Subpalette& add_subpalette();
  unsigned subpalettes_free() const { return _max_subpalettes - (unsigned)_subpalettes.size(); }

  const rgba_set_vec_t optimized_palettes(const rgba_set_vec_t& colors) const;
};

} /* namespace sfc */
