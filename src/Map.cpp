#include "Map.h"

namespace sfc {

void Map::add(const sfc::Image& image, const sfc::Tileset& tileset, const sfc::Palette& palette, unsigned bpp, unsigned pos_x, unsigned pos_y) {
  if (((pos_y * _map_width) + pos_x) > _entries.size()) throw std::runtime_error("Map entry out of bounds");

  int tileset_index = -1;
  int palette_index = -1;
  Tile matched_tile;
  {
    // search all viable palette mappings of image in tileset
    auto spv = palette.subpalettes_matching(image);
    for (auto p : spv) {
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
    _entries[(pos_y * _map_width) + pos_x] = Mapentry(0, 0, false, false);
  } else {
    _entries[(pos_y * _map_width) + pos_x] = Mapentry(tileset_index, palette_index,
                                                      tileset.tiles()[tileset_index].is_h_flipped(matched_tile),
                                                      tileset.tiles()[tileset_index].is_v_flipped(matched_tile));
  }
}

Mapentry Map::entry_at(unsigned x, unsigned y) const {
  if (x > _map_width) x = _map_width;
  if (y > _map_height) y = _map_height;
  if (((y * _map_width) + x) > _entries.size()) {
    return Mapentry();
  } else {
    return _entries[(y * _map_width) + x];
  }
}

std::vector<uint8_t> Map::native_data(bool column_order, unsigned split_w, unsigned split_h) const {
  std::vector<uint8_t> data;
  for (auto& vm : collect_entries(column_order, split_w, split_h)) {
    for (auto& m : vm) {
      auto nd = sfc::pack_native_mapentry(m, _mode);
      data.insert(data.end(), nd.begin(), nd.end());
    }
  }
  return data;
}

void Map::save(const std::string& path, bool column_order, unsigned split_w, unsigned split_h) const {
  sfc::write_file(path, native_data(column_order, split_w, split_h));
}

const std::string Map::to_json(bool column_order, unsigned split_w, unsigned split_h) const {
  nlohmann::json j;
  auto vmm = collect_entries(column_order, split_w, split_h);
  for (auto& vm : vmm) {
    nlohmann::json ja = nlohmann::json::array();
    for (auto& m : vm) {
      ja.push_back({{"tile", m.tile_index},
                    {"palette", m.palette_index},
                    {"flip_h", (int)m.flip_h},
                    {"flip_v", (int)m.flip_v}});
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

  if (split_w > _map_width || split_w == 0) split_w = _map_width;
  if (split_h > _map_height || split_h == 0) split_h = _map_height;

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
