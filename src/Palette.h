// palette representation
//
// david lindecrantz <optiroc@gmail.com>

#pragma once

#include "Common.h"
#include "Image.h"

namespace sfc {

struct Image;

struct Subpalette {
  Subpalette(Mode mode, unsigned max_colors)
  : _mode(mode),
    _max_colors(max_colors){};

  rgba_t color_at(unsigned index) const { return _colors[index]; }
  const std::vector<rgba_t> colors() const { return _colors; }
  Mode mode() const { return _mode; }

  unsigned size() const { return (unsigned)_colors.size(); }
  unsigned capacity_left() const { return (unsigned)(_max_colors - _colors.size()); }
  bool is_full() const { return _colors.size() == _max_colors; }

  const std::vector<rgba_t> get_normalized_colors() const { return normalize_colors(_colors, _mode); }

  void add(rgba_t color, bool add_duplicates = false) {
    if (add_duplicates) {
      if (is_full()) throw std::runtime_error("Colors don't fit in palette");
      _colors.push_back(color);
    } else if (_colors_set.find(color) == _colors_set.end()) {
      if (is_full()) throw std::runtime_error("Colors don't fit in palette");
      _colors.push_back(color);
    }
    _colors_set.insert(color);
  }

  void add(const std::vector<rgba_t>& new_colors, bool add_duplicates = false, bool overwrite = false) {
    if (overwrite) {
      _colors.clear();
      _colors_set.clear();
    }
    for (auto c : new_colors) add(c, add_duplicates);
  }

  void pad() {
    while (_colors.size() < _max_colors) add(0, true);
  }

  // number of colors in new_colors not in _colors
  unsigned diff(const std::set<rgba_t>& new_colors) const {
    std::set<rgba_t> ds;
    std::set_difference(new_colors.begin(), new_colors.end(), _colors_set.begin(), _colors_set.end(), std::inserter(ds, ds.begin()));
    return (unsigned)ds.size();
  }

  // sort colors, keeping color at index 0
  void sort() {
    if (_colors.size() < 3) return;
    std::vector<rgba_t> vc(_colors.begin() + 1, _colors.end());
    _colors.resize(1);
    sort_colors(vc);
    std::reverse(vc.begin(), vc.end());
    _colors.insert(_colors.end(), vc.begin(), vc.end());
  }

private:
  Mode _mode = Mode::snes;
  unsigned _max_colors;

  std::vector<rgba_t> _colors;
  std::set<rgba_t> _colors_set;
};


struct Palette {
  Palette(Mode mode = Mode::snes, unsigned max_subpalettes = 0, unsigned max_colors = 0)
  : _mode(mode),
    _max_subpalettes(max_subpalettes),
    _max_colors_per_subpalette(max_colors){};

  Palette(const std::vector<uint8_t> native_data, Mode in_mode = Mode::snes, unsigned colors_per_subpalette = 16);
  Palette(const std::string& path, Mode in_mode = Mode::snes, unsigned colors_per_subpalette = 16);

  unsigned max_colors_per_subpalette() const { return _max_colors_per_subpalette; }

  void add(const rgba_t color);
  void add(const std::vector<rgba_t>& colors);
  void add_noremap(const std::vector<rgba_t>& colors, bool reduce = true);

  const Subpalette& subpalette_at(unsigned index) const;
  const Subpalette& subpalette_matching(const Image& image) const;
  std::vector<const Subpalette*> subpalettes_matching(const Image& image) const;
  Subpalette& first_nonempty_subpalette();
  int index_of(const Subpalette& subpalette) const;

  const std::vector<std::vector<rgba_t>> colors() const;
  const std::vector<std::vector<rgba_t>> normalized_colors() const;

  void sort();
  void pad();

  const std::string to_json() const;
  void save(const std::string& path) const;
  void save_act(const std::string& path) const;

private:
  Mode _mode = Mode::snes;
  unsigned _max_subpalettes = 0;
  unsigned _max_colors_per_subpalette = 0;
  std::vector<Subpalette> _subpalettes;

  Subpalette& add_subpalette();

  inline unsigned subpalettes_free() const { return _max_subpalettes - (unsigned)_subpalettes.size(); }
};

std::ostream& operator<<(std::ostream& os, Palette const& p);

} /* namespace sfc */
