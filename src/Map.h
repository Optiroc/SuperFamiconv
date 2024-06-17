// map representation
//
// david lindecrantz <optiroc@me.com>

#pragma once

#include "Common.h"

#include "Image.h"
#include "Palette.h"
#include "Tiles.h"

namespace sfc {

struct Image;
struct Palette;
struct Tileset;

struct Mapentry final {
  Mapentry(unsigned tile_index = 0, unsigned palette_index = 0, bool flip_h = false, bool flip_v = false)
  : tile_index(tile_index), palette_index(palette_index), flip_h(flip_h), flip_v(flip_v){};

  unsigned tile_index = 0;
  unsigned palette_index = 0;
  bool flip_h = false;
  bool flip_v = false;
};


struct Map final {
  Map(Mode mode = Mode::snes, unsigned map_width = 32, unsigned map_height = 32, unsigned tile_width = 8, unsigned tile_height = 8)
  : _mode(mode), _map_width(map_width), _map_height(map_height), _tile_width(tile_width), _tile_height(tile_height) {
    _entries.resize(map_width * map_height);
  };

  unsigned width() const { return _map_width; }
  unsigned height() const { return _map_height; }

  void add(const Image& image, const Tileset& tileset, const Palette& palette, unsigned bpp, unsigned pos_x, unsigned pos_y);
  Mapentry entry_at(unsigned x, unsigned y) const;

  void add_base_offset(int offset);
  void add_palette_base_offset(int offset);

  byte_vec_t native_data(bool column_order = false, unsigned split_w = 0, unsigned split_h = 0) const;
  byte_vec_t palette_map(bool column_order = false, unsigned split_w = 0, unsigned split_h = 0) const;
  byte_vec_t snes_mode7_interleaved_data(const Tileset& tileset) const;
  byte_vec_t gbc_banked_data() const;

  void save(const std::string& path, bool column_order = false, unsigned split_w = 0, unsigned split_h = 0) const;
  void save_pal_map(const std::string& path, bool column_order = false, unsigned split_w = 0, unsigned split_h = 0) const;
  const std::string to_json(bool column_order = false, unsigned split_w = 0, unsigned split_h = 0) const;

private:
  Mode _mode = Mode::snes;
  unsigned _map_width = 32;
  unsigned _map_height = 32;
  unsigned _tile_width = 8;
  unsigned _tile_height = 8;

  std::vector<Mapentry> _entries;

  std::vector<std::vector<Mapentry>> collect_entries(bool column_order = false, unsigned split_w = 0, unsigned split_h = 0) const;
};


inline byte_vec_t pack_native_mapentry(const Mapentry& entry, Mode mode) {
  byte_vec_t v;
  switch (mode) {
  case Mode::snes:
    v.push_back(entry.tile_index & 0xff);
    v.push_back(((entry.tile_index >> 8) & 0x03) | ((entry.palette_index << 2) & 0x1c) | (entry.flip_h << 6) | (entry.flip_v << 7));
    break;

  case Mode::snes_mode7:
    v.push_back(entry.tile_index & 0xff);
    break;

  case Mode::gb:
    v.push_back(entry.tile_index & 0xff);
    break;

  case Mode::gbc:
    v.push_back(entry.tile_index & 0xff);
    v.push_back(((entry.palette_index) & 0x07) | ((entry.tile_index >> 5) & 0x08) | (entry.flip_h << 5) | (entry.flip_v << 6));
    break;

  case Mode::gba:
    v.push_back(entry.tile_index & 0xff);
    v.push_back(((entry.tile_index >> 8) & 0x03) | (entry.flip_h << 2) | (entry.flip_v << 3) | ((entry.palette_index << 4) & 0xf0));
    break;

  case Mode::gba_affine:
    v.push_back(entry.tile_index & 0xff);
    break;

  case Mode::md:
    v.push_back(entry.tile_index & 0xff);
    v.push_back(((entry.tile_index >> 8) & 0x07) | (entry.flip_h << 3) | (entry.flip_v << 4) | ((entry.palette_index << 5) & 0x60));
    break;

  case Mode::pce:
    v.push_back(entry.tile_index & 0xff);
    v.push_back(((entry.tile_index >> 8) & 0x0f) | ((entry.palette_index << 4) & 0xf0));
    break;

  case Mode::ws:
  case Mode::wsc:
  case Mode::wsc_packed:
    v.push_back(entry.tile_index & 0xff);
    v.push_back(((entry.tile_index >> 8) & 0x01) | ((entry.palette_index << 1) & 0x1e) | ((entry.tile_index >> 4) & 0x20) | (entry.flip_h << 6) | (entry.flip_v << 7));
	break;

  case Mode::pce_sprite:
  case Mode::none:
    break;
  }
  return v;
}

} /* namespace sfc */
