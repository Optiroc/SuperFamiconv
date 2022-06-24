#include "Map.h"

namespace sfc {

void Map::add(const sfc::Image& image, const sfc::Tileset& tileset, const sfc::Palette& palette, unsigned bpp, unsigned pos_x, unsigned pos_y) {
  if (((pos_y * _map_width) + pos_x) > _entries.size())
    throw std::runtime_error("Map entry out of bounds");

  int tileset_index = -1;
  int palette_index = -1;
  Tile matched_tile;
  {
    // search all viable palette mappings of image in tileset
    const auto spv = palette.subpalettes_matching(image);
    for (const auto& p : spv) {
      const Image remapped_image = Image(image, *p);
      Tile remapped_tile(remapped_image, _mode, bpp, true);
      tileset_index = tileset.index_of(remapped_tile);
      if (tileset_index != -1) {
        palette_index = palette.index_of(*p);
        matched_tile = remapped_tile;
        break;
      }
    }
  }

  if (tileset_index == -1) {
    fmt::print(stderr, "  No matching tile for position {},{}\n", image.src_coord_x(), image.src_coord_y());
    _entries[(pos_y * _map_width) + pos_x] = Mapentry(0, 0, false, false);

  } else if (tileset_index >= (int)max_tile_count_for_mode(_mode)) {
    fmt::print(stderr, "  Mapped tile exceeds allowed map index at position {},{}\n", image.src_coord_x(), image.src_coord_y());
    _entries[(pos_y * _map_width) + pos_x] = Mapentry(0, 0, false, false);

  } else {
    const TileFlipped flipped = tileset.tiles()[tileset_index].is_flipped(matched_tile);
    _entries[(pos_y * _map_width) + pos_x] =
      Mapentry(tileset_index, palette_index, flipped.h, flipped.v);
  }
}

Mapentry Map::entry_at(unsigned x, unsigned y) const {
  if (x > _map_width)
    x = _map_width;
  if (y > _map_height)
    y = _map_height;
  if (((y * _map_width) + x) > _entries.size()) {
    return Mapentry();
  } else {
    Mapentry entry = _entries[(y * _map_width) + x];
    if (_tile_width == 8 && _tile_height == 8)
      return entry;

    // snes non-8x8 tilemap
    unsigned tile_col = entry.tile_index % 8;
    unsigned tile_row = entry.tile_index / 8;
    entry.tile_index = tile_col * (_tile_width == 8 ? 1 : 2) + tile_row * (_tile_height == 8 ? 16 : 32);
    return entry;
  }
}

void Map::add_base_offset(int offset) {
  for (auto& e : _entries) {
    e.tile_index = (unsigned)std::max(0, (int)e.tile_index + offset);
  }
}

void Map::add_palette_base_offset(int offset) {
  for (auto& e : _entries) {
    e.palette_index = (unsigned)std::max(0, (int)e.palette_index + offset);
  }
}

byte_vec_t Map::native_data(bool column_order, unsigned split_w, unsigned split_h) const {
  byte_vec_t data;
  for (const auto& vm : collect_entries(column_order, split_w, split_h)) {
    for (const auto& m : vm) {
      auto nd = sfc::pack_native_mapentry(m, _mode);
      data.insert(data.end(), nd.begin(), nd.end());
    }
  }
  return data;
}

byte_vec_t Map::palette_map(bool column_order, unsigned split_w, unsigned split_h) const {
  byte_vec_t data;
  for (const auto& vm : collect_entries(column_order, split_w, split_h)) {
    for (const auto& m : vm) {
      data.push_back(m.palette_index & 0xFF);
      data.push_back(m.palette_index >> 8);
    }
  }
  return data;
}

byte_vec_t Map::snes_mode7_interleaved_data(const Tileset& tileset) const {
  auto map_data = native_data();
  auto tile_data = tileset.native_data();

  size_t sz = (tile_data.size() > map_data.size()) ? tile_data.size() : map_data.size();
  byte_vec_t data = byte_vec_t(sz * 2);
  for (unsigned i = 0; i < map_data.size(); ++i)
    data[(i << 1)] = map_data[i];
  for (unsigned i = 0; i < tile_data.size(); ++i)
    data[(i << 1) + 1] = tile_data[i];

  return data;
}

byte_vec_t Map::gbc_banked_data() const {
  if ((width() % 32) || (height() % 32))
    throw std::runtime_error("gbc/out-gbc-bank requires map dimensions to be multiples of 32");

  const auto linear_data = native_data();
  auto banked_data = byte_vec_t(linear_data.size());
  for (unsigned i = 0; i < linear_data.size() >> 1; ++i) banked_data[i] = linear_data[i << 1];
  for (unsigned i = 0; i < linear_data.size() >> 1; ++i) banked_data[i + (linear_data.size() >> 1)] = linear_data[(i << 1) + 1];
  return banked_data;
}

void Map::save(const std::string& path, bool column_order, unsigned split_w, unsigned split_h) const {
  sfc::write_file(path, native_data(column_order, split_w, split_h));
}

void Map::save_pal_map(const std::string& path, bool column_order, unsigned split_w, unsigned split_h) const {
  sfc::write_file(path, palette_map(column_order, split_w, split_h));
}

const std::string Map::to_json(bool column_order, unsigned split_w, unsigned split_h) const {
  nlohmann::json j;
  const auto vmm = collect_entries(column_order, split_w, split_h);

  for (const auto& vm : vmm) {
    nlohmann::json ja = nlohmann::json::array();

    if (tile_flipping_allowed_for_mode(_mode) && default_palette_count_for_mode(_mode) > 1) {
      for (const auto& m : vm) {
        ja.push_back(
          {{"tile", m.tile_index}, {"palette", m.palette_index}, {"flip_h", (int)m.flip_h}, {"flip_v", (int)m.flip_v}});
      }
    } else if (tile_flipping_allowed_for_mode(_mode)) {
      for (const auto& m : vm) {
        ja.push_back({{"tile", m.tile_index}, {"flip_h", (int)m.flip_h}, {"flip_v", (int)m.flip_v}});
      }
    } else if (default_palette_count_for_mode(_mode) > 1) {
      for (const auto& m : vm) {
        ja.push_back({{"tile", m.tile_index}, {"palette", m.palette_index}});
      }
    } else {
      for (const auto& m : vm) {
        ja.push_back({{"tile", m.tile_index}});
      }
    }

    if (vmm.size() > 1) {
      j["maps"].emplace_back(ja);
    } else {
      j["map"] = ja;
    }
  }
  return j.dump(2);
}

std::vector<std::vector<Mapentry>> Map::collect_entries(bool column_order, unsigned split_w, unsigned split_h) const {
  std::vector<std::vector<Mapentry>> vvm;

  if (split_w > _map_width || split_w == 0)
    split_w = _map_width;
  if (split_h > _map_height || split_h == 0)
    split_h = _map_height;

  if (split_w == _map_width && split_h == _map_height) {
    vvm.push_back(_entries);
  } else {
    unsigned columns = (div_ceil(_map_width, split_w) == 0) ? 1 : div_ceil(_map_width, split_w);
    unsigned rows = (div_ceil(_map_height, split_h) == 0) ? 1 : div_ceil(_map_height, split_h);
    for (unsigned col = 0; col < columns; ++col) {
      for (unsigned row = 0; row < rows; ++row) {
        std::vector<Mapentry> vm;
        for (unsigned pos = 0; pos < split_w * split_h; ++pos) {
          vm.push_back(entry_at((col * split_w) + (pos % split_w), (row * split_h) + (pos / split_w)));
        }
        vvm.push_back(vm);
      }
    }
  }

  if (column_order) {
    std::vector<std::vector<Mapentry>> vvmo;
    for (unsigned vmi = 0; vmi < vvm.size(); ++vmi) {
      vvmo.push_back(std::vector<Mapentry>());
      for (unsigned pos = 0; pos < vvm[vmi].size(); ++pos) {
        vvmo.back().push_back(vvm[vmi][((pos * split_w) + (pos / split_h)) % (split_w * split_h)]);
      }
    }
    return vvmo;
  }

  return vvm;
}

} /* namespace sfc */
