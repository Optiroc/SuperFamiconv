#include "Palette.h"

namespace sfc {

// add color
void Subpalette::add(rgba_t color, bool add_duplicates) {
  if (add_duplicates) {
    if (is_full())
      throw std::runtime_error("Colors don't fit in palette");
    _colors.push_back(color);
  } else if (_colors_set.find(color) == _colors_set.end()) {
    if (is_full())
      throw std::runtime_error("Colors don't fit in palette");
    _colors.push_back(color);
  }
  _colors_set.insert(color);
}

// add vector of colors
void Subpalette::add(const rgba_vec_t& new_colors, bool add_duplicates, bool overwrite) {
  if (overwrite) {
    _colors.clear();
    _colors_set.clear();
  }
  for (auto c : new_colors)
    add(c, add_duplicates);
}

// set color at index
void Subpalette::set(unsigned index, const rgba_t color) {
  if (_colors.size() > index) {
    _colors[index] = color;
    _colors_set = rgba_set_t(_colors.begin(), _colors.end());
  }
}

// return subpalette padded to max_color count
Subpalette Subpalette::padded() const {
  Subpalette sp = Subpalette(*this);
  while (sp._colors.size() < sp._max_colors)
    sp.add(0, true);
  return sp;
}

// number of colors in new_colors not in subpalette
unsigned Subpalette::diff(const rgba_set_t& new_colors) const {
  rgba_set_t ds;
  std::set_difference(new_colors.begin(), new_colors.end(), _colors_set.begin(), _colors_set.end(),
                      std::inserter(ds, ds.begin()));
  return (unsigned)ds.size();
}

// sort colors, keeping color at index 0
void Subpalette::sort() {
  if (_colors.size() < 3)
    return;
  rgba_vec_t vc(_colors.begin() + 1, _colors.end());
  _colors.resize(1);
  sort_colors(vc);
  std::reverse(vc.begin(), vc.end());
  _colors.insert(_colors.end(), vc.begin(), vc.end());
}

// if there are duplicates of color zero, set alpha of color zero to 0
bool Subpalette::check_col0_duplicates() {
  if (_colors.size() <= 1)
    return false;
  if (std::find(std::next(_colors.begin()), _colors.end(), _colors[0]) != _colors.end()) {
    _colors[0] = _colors[0] & 0x00ffffff;
    _colors_set = rgba_set_t(_colors.begin(), _colors.end());
    return true;
  }
  return false;
}


// construct Palette by deserializing json or binary
Palette::Palette(const std::string& path, Mode in_mode, uint32_t colors_per_subpalette) {
  _mode = in_mode;
  _max_colors_per_subpalette = colors_per_subpalette;
  _max_subpalettes = 64;

  try {
    // load json
    auto j = read_json_file(path);
    auto jp = j["palettes"];
    for (const auto& jsp : jp) {
      rgba_vec_t colors;
      for (const auto& jcs : jsp) {
        if (jcs.is_string())
          colors.push_back(reduce_color(from_hexstring(jcs), in_mode));
      }
      if (colors.size() > _max_colors_per_subpalette)
        throw std::runtime_error("Palette in JSON doesn't match color depth / colors per subpalette");
      add_colors(colors, false);
    }
  } catch (...) {
    // load binary
    add_colors(unpack_native_colors(read_binary(path), in_mode), false);
    check_col0_duplicates();
  }

  if (_subpalettes.empty())
    throw std::runtime_error("No palette data in JSON");
}

// construct Palette from native data
Palette::Palette(const byte_vec_t& native_data, Mode in_mode, unsigned colors_per_subpalette) {
  _mode = in_mode;
  _max_colors_per_subpalette = colors_per_subpalette;
  _max_subpalettes = default_palette_count_for_mode(_mode);
  add_colors(unpack_native_colors(native_data, in_mode), false);
  check_col0_duplicates();
}


// total number of colors
unsigned Palette::size() const {
  unsigned count = 0;
  for (const auto& sp : _subpalettes)
    count += sp.colors().size();
  return count;
}

// get colors
const std::vector<rgba_vec_t> Palette::colors() const {
  std::vector<rgba_vec_t> v;
  for (const auto& sp : _subpalettes)
    v.push_back(sp.colors());
  return v;
}

// get colors normalized to RGBA8888 range
const std::vector<rgba_vec_t> Palette::normalized_colors() const {
  auto v = colors();
  for (auto& i : v)
    i = normalize_colors(i, _mode);
  return v;
}

// set color at index all subpalettes
void Palette::set_color(unsigned index, const rgba_t color) {
  for (auto& sp : _subpalettes)
    sp.set(index, color);
}

// set color to be used at index 0 for subsequently created SubPalettes
void Palette::prime_col0(const rgba_t color) {
  _col0 = reduce_color(color, _mode) == transparent_color ? transparent_color : color;
  _col0_is_shared = true;
}

// if there are duplicates of color zero, set alpha of color zero to 0
void Palette::check_col0_duplicates() {
  if (sfc::col0_is_shared_for_mode(_mode)) {
    bool fixed = false;
    for (auto& sp : _subpalettes)
      fixed |= sp.check_col0_duplicates();
    if (fixed)
      fmt::print("Palette contains duplicates of color zero, treating color zero as transparent\n");
  }
}

// add optimized subpalettes containing colors in palette_tiles
void Palette::add_images(std::vector<sfc::Image> palette_tiles) {

  // make vector of sets of all tiles' colors
  rgba_set_vec_t palettes = rgba_set_vec_t();
  for (const auto& c : palette_tiles) {

    if (c.colors().size() > _max_colors_per_subpalette) {
      fmt::print(stderr, "  Tile with too many ({} > {}) unique colors at {},{} in source image\n", c.colors().size(), _max_colors_per_subpalette, c.src_coord_x(), c.src_coord_y());
    }

    if (_col0_is_shared) {
      auto colors = c.colors();
      colors.insert(_col0);
      palettes.push_back(reduce_colors(colors, _mode));
    } else {
      palettes.push_back(reduce_colors(c.colors(), _mode));
    }
  }

  // optimize
  auto optimized = optimized_palettes(palettes);

  // TODO: if throw iterate all palette_tiles and report positions
  if (optimized.size() > _max_subpalettes)
    throw std::runtime_error("Colors in image do not fit in available palettes. Aborting.");

  // add subpalettes
  for (auto& cs : optimized) {
    auto& sp = add_subpalette();
    rgba_vec_t cv(cs.begin(), cs.end());

    if (_col0_is_shared) {
      auto p = std::find(cv.begin(), cv.end(), reduce_color(_col0, _mode));
      if (p != cv.end())
        std::iter_swap(p, cv.begin());
    }

    sp.add(cv);
  }
}

void Palette::add_colors(const rgba_vec_t& colors, bool reduce_depth) {
  auto rc = colors;
  if (reduce_depth)
    rc = reduce_colors(rc, _mode);
  auto splits = split_vector(rc, _max_colors_per_subpalette);
  for (auto& sv : splits) {
    Subpalette sp(_mode, _max_colors_per_subpalette);
    sp.add(sv, true);
    _subpalettes.push_back(sp);
  }
}


int Palette::index_of(const Subpalette& subpalette) const {
  for (int i = 0; i < (int)_subpalettes.size(); ++i) {
    if (subpalette.colors() == _subpalettes[i].colors())
      return i;
  }
  return -1;
}

// get first subpalette containing all colors in image
const Subpalette& Palette::subpalette_matching(const Image& image) const {
  auto rc = reduce_colors(image.rgba_data(), _mode);
  rgba_set_t cs(rc.begin(), rc.end());
  cs.erase(transparent_color);

  if (cs.size() > _max_colors_per_subpalette) {
    throw std::runtime_error(
      fmt::format("Tile with too many ({} > {}) unique colors at {},{} in source image", cs.size(), _max_colors_per_subpalette, image.src_coord_x(), image.src_coord_y()));
  }

  auto match = std::find_if(_subpalettes.begin(), _subpalettes.end(), [&](const auto& val) -> bool { return val.diff(cs) == 0; });

  if (match == _subpalettes.end()) {
    throw std::runtime_error(
      fmt::format("No matching palette for tile at {},{} in source image", image.src_coord_x(), image.src_coord_y()));
  }

  return *match;
}

std::vector<const Subpalette*> Palette::subpalettes_matching(const Image& image) const {
  std::vector<const Subpalette*> sv;

  auto rc = reduce_colors(image.rgba_data(), _mode);
  rgba_set_t cs(rc.begin(), rc.end());

  if (cs.size() > _max_colors_per_subpalette) {
    throw std::runtime_error(
      fmt::format("Tile with too many unique colors at {},{} in source image\n", image.src_coord_x(), image.src_coord_y()));
  }

  for (const Subpalette& sp : _subpalettes) {
    if (sp.diff(cs) == 0)
      sv.push_back(&sp);
  }

  return sv;
}


void Palette::sort() {
  for (auto& sp : _subpalettes)
    sp.sort();
}


const std::string Palette::description() const {
  auto v = colors();
  int total = 0;
  std::string s = "";

  for (const auto& i : v) {
    total += i.size();
    s += fmt::format("{},", i.size());
  }

  if (total > 0) {
    if (v.size() == 1) {
      return fmt::format("{} colors", total);
    } else {
      s.pop_back();
      return fmt::format("{} colors [{}]", total, s);
    }
  } else {
    return "zero colors";
  }
}

const std::string Palette::to_json() const {
  nlohmann::json j;

  {
    auto ps = normalized_colors();
    std::vector<std::vector<std::string>> jps;

    for (const auto& p : ps) {
      std::vector<std::string> jp;
      for (const auto& c : p)
        jp.push_back(to_hexstring(c));
      jps.push_back(jp);
    }

    j["palettes"] = jps;
  }

  {
    auto ps = colors();
    std::vector<std::vector<std::vector<unsigned>>> jps;

    for (const auto& p : ps) {
      std::vector<std::vector<unsigned>> jp;
      for (const auto& c : p) {
        std::vector<unsigned> jpt;
        auto rgb = rgba_color(c);
        jpt.push_back(rgb.r);
        jpt.push_back(rgb.g);
        jpt.push_back(rgb.b);
        jp.push_back(jpt);
      }
      jps.push_back(jp);
    }

    j["palettes_native_rgb"] = jps;
  }

  return j.dump(2);
}

void Palette::save(const std::string& path) const {
  byte_vec_t data;
  for (const auto& sp : _subpalettes) {
    Subpalette spp = sp.padded();
    auto nc = pack_native_colors(spp.colors(), _mode);
    data.insert(data.end(), nc.begin(), nc.end());
  }
  write_file(path, data);
}

void Palette::save_act(const std::string& path) const {
  byte_vec_t data((256 * 3) + 4);
  int count = 0;

  for (const auto& sp : _subpalettes) {
    Subpalette spp = sp.padded();
    rgba_vec_t colors = spp.normalized_colors();
    for (const auto& c : colors) {
      rgba_color rgba(c);
      data[count * 3 + 0] = rgba.r;
      data[count * 3 + 1] = rgba.g;
      data[count * 3 + 2] = rgba.b;
      if (++count > 256)
        goto done;
    }
  }

done:
  data[0x300] = 0x00;
  data[0x301] = count & 0xff;
  data[0x302] = data[0x303] = 0xff;
  write_file(path, data);
}


Subpalette& Palette::add_subpalette() {
  if (_max_subpalettes - _subpalettes.size() == 0)
    throw std::runtime_error("Colors don't fit in palette");
  _subpalettes.emplace_back(Subpalette(_mode, _max_colors_per_subpalette));
  Subpalette& sp = _subpalettes.back();
  return sp;
}

// functional form of old "greedy best fit" style palette optimizer
const rgba_set_vec_t Palette::optimized_palettes(const rgba_set_vec_t& colors) const {

  auto filter_subsets = [](const rgba_set_vec_t& v) {
    auto n = rgba_set_vec_t(v.size());
    auto it = std::copy_if(v.begin(), v.end(), n.begin(), [&](const auto& s) { return !has_superset(s, v); });
    n.resize(std::distance(n.begin(), it));
    return n;
  };

  auto filter_redundant = [](const rgba_set_vec_t& v) {
    auto n = rgba_set_vec_t(v.size());
    auto it = std::copy_if(v.begin(), v.end(), n.begin(),
                           [&](auto& s) { return s.size() < 1 ? false : std::find(n.begin(), n.end(), s) == n.end(); });
    n.resize(std::distance(n.begin(), it));
    return n;
  };

  auto best_fit = [&](const rgba_set_t& s, const rgba_set_vec_t& v) {
    int best = -1;
    unsigned i = 0;
    for (auto& cs : v) {
      rgba_set_t d;
      std::set_difference(s.begin(), s.end(), cs.begin(), cs.end(), std::inserter(d, d.begin()));
      if (d.size() + cs.size() <= _max_colors_per_subpalette)
        best = i;
      ++i;
    }
    return best;
  };

  auto sets = filter_redundant(colors);
  sets = filter_subsets(sets);
  std::sort(sets.begin(), sets.end(), [](auto& a, auto& b) { return a.size() < b.size(); });

  rgba_set_vec_t opt = rgba_set_vec_t();

  while (sets.size()) {
    auto set = vec_pop(sets);
    auto best_index = best_fit(set, opt);
    if (best_index == -1) {
      opt.push_back(set);
    } else {
      opt[best_index].insert(set.begin(), set.end());
    }
  }

  std::sort(opt.begin(), opt.end(), [](auto& a, auto& b) -> bool { return a.size() > b.size(); });
  return opt;
}

} /* namespace sfc */
