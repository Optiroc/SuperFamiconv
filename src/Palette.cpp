#include "Palette.h"

namespace sfc {

void Palette::add(const rgba_t color) {
  std::vector<rgba_t> v = {color};
  add(v);
}

// add colors by appending to first subpalette with enough free colors
void Palette::add(const std::vector<rgba_t>& colors) {
  auto rc = reduce_colors(colors, _mode);
  std::set<rgba_t> cs(rc.begin(), rc.end());
  cs.erase(transparent_color);
  if (cs.size() > _max_colors_per_subpalette) throw std::runtime_error("Colors don't fit in palette"); // TODO: catch and report position

  Subpalette* best_candidate = nullptr;
  unsigned best_diff = UINT_MAX;
  for (Subpalette& sp : _subpalettes) {
    unsigned diff = sp.diff(cs);
    if (diff > 0 && sp.is_full()) continue;
    if (diff < best_diff && diff <= sp.capacity_left()) {
      best_diff = diff;
      best_candidate = &sp;
    }
  }

  if (best_diff == 0) {
    return;
  } else if (!best_candidate || best_candidate->is_full()) {
    Subpalette& new_sp = add_subpalette();
    new_sp.add(std::vector<rgba_t>(cs.begin(), cs.end()));
  } else {
    best_candidate->add(std::vector<rgba_t>(cs.begin(), cs.end()));
  }
}

// add colors without reordering or discarding duplicates
void Palette::add_noremap(const std::vector<rgba_t>& colors, bool reduce) {
  auto rc = colors;
  if (reduce) rc = reduce_colors(rc, _mode);
  auto splits = split_vector(rc, _max_colors_per_subpalette);
  for (auto& sv : splits) {
    Subpalette sp(_mode, _max_colors_per_subpalette);
    sp.add(sv, true);
    _subpalettes.push_back(sp);
  }
}

void Palette::sort() {
  for (auto& sp : _subpalettes) sp.sort();
}

void Palette::pad() {
  for (auto& sp : _subpalettes) sp.pad();
}

const Subpalette& Palette::subpalette_at(unsigned index) const {
  if (index > _subpalettes.size()) throw std::runtime_error("Subpalette doesn't exist");
  return _subpalettes.at(index);
}

// get first subpalette containing all colors in image
const Subpalette& Palette::subpalette_matching(const Image& image) const {
  auto rc = reduce_colors(image.rgba_data(), _mode);
  std::set<rgba_t> cs(rc.begin(), rc.end());
  cs.erase(transparent_color);
  if (cs.size() > _max_colors_per_subpalette) throw std::runtime_error("Colors don't fit in palette"); // TODO: catch and report position

  const Subpalette* matching_sp = nullptr;
  for (const Subpalette& sp : _subpalettes) {
    if (sp.diff(cs) == 0) {
      matching_sp = &sp;
      break;
    }
  }

  if (matching_sp == nullptr) throw std::runtime_error("No matching palette for image"); // TODO: catch and report position
  return *matching_sp;
}

std::vector<const Subpalette*> Palette::subpalettes_matching(const Image& image) const {
  std::vector<const Subpalette*> sv;

  auto rc = reduce_colors(image.rgba_data(), _mode);
  std::set<rgba_t> cs(rc.begin(), rc.end());
  if (cs.size() > _max_colors_per_subpalette) throw std::runtime_error("Colors don't fit in palette"); // TODO: catch and report position

  for (const Subpalette& sp : _subpalettes) {
    if (sp.diff(cs) == 0) sv.push_back(&sp);
  }

  return sv;
}

Subpalette& Palette::first_nonempty_subpalette() {
  if (_subpalettes.empty() || _subpalettes[_subpalettes.size() - 1].is_full()) return add_subpalette();
  return _subpalettes[_subpalettes.size() - 1];
}

int Palette::index_of(const Subpalette& subpalette) const {
  for (int i = 0; i < (int)_subpalettes.size(); ++i) {
    if (subpalette.colors() == _subpalettes[i].colors()) return i;
  }
  return -1;
}

Subpalette& Palette::add_subpalette() {
  if (_max_subpalettes - _subpalettes.size() == 0) throw std::runtime_error("Colors don't fit in palette");
  _subpalettes.emplace_back(Subpalette(_mode, _max_colors_per_subpalette));
  Subpalette& sp = _subpalettes.back();

  // add color 0 from first palette to any subsequent palettes (make as option?)
  if (_subpalettes.size() > 1 && _subpalettes[0].size() > 0) sp.add(_subpalettes[0].color_at(0));

  return sp;
}

const std::vector<std::vector<rgba_t>> Palette::colors() const {
  std::vector<std::vector<rgba_t>> v;
  for (auto sp : _subpalettes) v.push_back(sp.colors());
  return v;
}

const std::vector<std::vector<rgba_t>> Palette::normalized_colors() const {
  auto v = colors();
  for (auto& i : v) i = normalize_colors(i, _mode);
  return v;
}


Palette::Palette(const std::vector<uint8_t> native_data, Mode in_mode, unsigned colors_per_subpalette) {
  _mode = in_mode;
  _max_colors_per_subpalette = colors_per_subpalette;
  _max_subpalettes = 64;
  add_noremap(unpack_native_colors(native_data, in_mode), false);
}

// deserialize palette from json or binary
Palette::Palette(const std::string& path, Mode in_mode, uint32_t colors_per_subpalette) {
  _mode = in_mode;
  _max_colors_per_subpalette = colors_per_subpalette;
  _max_subpalettes = 64;

  try {
    // load json
    auto j = read_json_file(path);
    auto jp = j["palettes"];
    for (auto jsp : jp) {
      std::vector<rgba_t> colors;
      for (auto jcs : jsp)
        if (jcs.is_string()) colors.push_back(reduce_color(from_hexstring(jcs), in_mode));
      if (colors.size() > _max_colors_per_subpalette) throw std::runtime_error("Palette in JSON doesn't match color depth / colors per subpalette");
      add_noremap(colors, false);
    }
  } catch (...) {
    // load binary
    add_noremap(unpack_native_colors(read_binary(path), in_mode), false);
  }

  if (_subpalettes.empty()) throw std::runtime_error("No palette data in JSON");
}

const std::string Palette::to_json() const {
  auto v = normalized_colors();
  std::vector<std::vector<std::string>> vj;

  for (auto i : v) {
    std::vector<std::string> vs;
    for (auto j : i) vs.push_back(to_hexstring(j));
    vj.push_back(vs);
  }

  nlohmann::json j;
  j["palettes"] = vj;
  return j.dump(2);
}

void Palette::save(const std::string& path) const {
  std::vector<uint8_t> data;

  for (auto& sp : _subpalettes) {
    std::vector<rgba_t> colors = sp.colors();
    for (auto c : colors) {
      auto nc = pack_native_color(c, _mode);
      data.insert(data.end(), nc.begin(), nc.end());
    }
  }

  write_file(path, data);
}

void Palette::save_act(const std::string& path) const {
  std::vector<uint8_t> data((256 * 3) + 4);
  int count = 0;

  for (auto& sp : _subpalettes) {
    std::vector<rgba_t> colors = sp.get_normalized_colors();
    for (auto c : colors) {
      rgba_color rgba(c);
      data[count * 3 + 0] = rgba.r;
      data[count * 3 + 1] = rgba.g;
      data[count * 3 + 2] = rgba.b;
      if (++count > 256) goto done;
    }
  }

  done:
  data[0x300] = 0x00;
  data[0x301] = count & 0xff;
  data[0x302] = data[0x303] = 0xff;
  write_file(path, data);
}

// print info
std::ostream& operator<<(std::ostream& os, Palette const& p) {
  auto v = p.colors();
  int total = 0;
  std::stringstream ss;
  ss << "[";

  for (auto i : v) {
    total += i.size();
    ss << i.size() << ",";
  }

  if (total > 0) {
    ss.seekp(-1, std::ios_base::end);
    ss << "] colors, " << total << " total";
    return os << ss.str();
  } else {
    return os << "zero colors";
  }
}

} /* namespace sfc */
