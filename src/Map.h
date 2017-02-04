// map representation
//
// david lindecrantz <optiroc@gmail.com>

#pragma once

#include "Common.h"
#include "Image.h"
#include "Palette.h"
#include "Tiles.h"

namespace sfc {

struct Image;
struct Palette;
struct Tileset;

struct Mapentry {
  Mapentry(unsigned tile_index = 0, unsigned palette_index = 0, bool flip_h = false, bool flip_v = false)
  : tile_index(tile_index),
    palette_index(palette_index),
    flip_h(flip_h),
    flip_v(flip_v){};

  unsigned tile_index = 0;
  unsigned palette_index = 0;
  bool flip_h = false;
  bool flip_v = false;
};

struct Map {
  Map(Mode mode = Mode::snes, unsigned map_width = 32, unsigned map_height = 32)
  : _mode(mode),
    _map_width(map_width),
    _map_height(map_height) {
    _entries.resize(map_width * map_height);
  };

  void add(const Image& image, const Tileset& tileset, const Palette& palette, unsigned bpp, unsigned pos_x, unsigned pos_y);
  Mapentry entry_at(unsigned x, unsigned y) const;

  std::vector<uint8_t> native_data(bool column_order = false, unsigned split_w = 0, unsigned split_h = 0) const;
  void save(const std::string& path, bool column_order = false, unsigned split_w = 0, unsigned split_h = 0) const;
  const std::string to_json(bool column_order = false, unsigned split_w = 0, unsigned split_h = 0) const;

private:
  Mode _mode = Mode::snes;
  unsigned _map_width = 32;
  unsigned _map_height = 32;

  std::vector<Mapentry> _entries;

  std::vector<std::vector<Mapentry>> collect_entries(bool column_order = false, unsigned split_w = 0, unsigned split_h = 0) const;
};

inline std::vector<uint8_t> pack_native_mapentry(const Mapentry& entry, Mode mode) {
  std::vector<uint8_t> v;
  switch (mode) {
  case Mode::snes:
    v.push_back(entry.tile_index & 0xff);
    v.push_back(((entry.tile_index >> 8) & 0xff) | ((entry.palette_index << 2) & 0x1c) | (entry.flip_h << 6) | (entry.flip_v << 7));
    break;

  case Mode::snes_mode7:
    v.push_back(entry.tile_index & 0xff);
    break;
  }
  return v;
}

} /* namespace sfc */
