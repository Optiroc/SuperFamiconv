#include "Tiles.h"

namespace sfc {

Tile::Tile(const Image& image, Mode mode, unsigned bpp, bool no_flip) {
  if (image.indexed_data().empty()) throw std::runtime_error("Can't create tile without indexed data");

  _mode = mode;
  _bpp = bpp;
  _width = image.width();
  _height = image.height();
  _palette = image.palette();

  index_t mask = bitmask_at_bpp(_bpp);
  for (index_t ip : image.indexed_data()) _data.push_back(ip & mask);

  if (!no_flip) {
    _mirrors.push_back(mirror(_data, _width, true, false));
    _mirrors.push_back(mirror(_data, _width, false, true));
    _mirrors.push_back(mirror(_data, _width, true, true));
  }
}

Tile::Tile(const std::vector<uint8_t>& native_data, Mode mode, unsigned bpp, bool no_flip, unsigned width, unsigned height) {
  _mode = mode;
  _bpp = bpp;
  _width = width;
  _height = height;
  _data = unpack_native_tile(native_data, mode, bpp, width, height);

  _palette.resize(palette_size_at_bpp(bpp));
  channel_t add = 0x100 / _palette.size();
  for (unsigned i = 0; i < _palette.size(); ++i) {
    channel_t value = add * i;
    _palette[i] = (rgba_t)(0xff000000 + value + (value << 8) + (value << 16));
  }

  if (!no_flip) {
    _mirrors.push_back(mirror(_data, _width, true, false));
    _mirrors.push_back(mirror(_data, _width, false, true));
    _mirrors.push_back(mirror(_data, _width, true, true));
  }
}

Tile::Tile(const std::vector<Tile>& metatile, bool no_flip, unsigned width, unsigned height) {
  if (metatile.empty()) return;

  _mode = metatile[0]._mode;
  _bpp = metatile[0]._bpp;
  _palette = metatile[0]._palette;
  _width = width;
  _height = height;
  _data.resize(width * height);

  const unsigned metatile_dim = metatile[0]._width;
  const unsigned metatiles_h = width / metatile_dim;
  const unsigned metatiles_v = height / metatile_dim;

  unsigned metatile_index = 0;
  for (unsigned my = 0; my < metatiles_v; ++my) {
    for (unsigned mx = 0; mx < metatiles_h; ++mx) {
      for (unsigned blit_y = 0; blit_y < metatile_dim; ++blit_y) {
        std::memcpy(&_data[((blit_y + (my * metatile_dim)) * width) + (mx * metatile_dim)],
                    &metatile[metatile_index]._data[blit_y * metatile_dim], metatile_dim);
      }
      ++metatile_index;
    }
  }

  if (!no_flip) {
    _mirrors.push_back(mirror(_data, _width, true, false));
    _mirrors.push_back(mirror(_data, _width, false, true));
    _mirrors.push_back(mirror(_data, _width, true, true));
  }
}

bool Tile::is_h_flipped(const Tile& other) const {
  if (other._data == _data) return false;
  if (_mirrors.empty()) throw std::runtime_error("Programmer error");
  if (other._data == _mirrors[0] || other._data == _mirrors[2]) return true;
  return false;
}

bool Tile::is_v_flipped(const Tile& other) const {
  if (other._data == _data) return false;
  if (_mirrors.empty()) throw std::runtime_error("Programmer error");
  if (other._data == _mirrors[1] || other._data == _mirrors[2]) return true;
  return false;
}

bool Tile::operator==(const Tile& other) const {
  if (other._data == _data) return true;
  if (!_mirrors.empty()) {
    for (auto& m : _mirrors) {
      if (other._data == m) return true;
    }
  }
  return false;
}

Tile Tile::crop(unsigned x, unsigned y, unsigned crop_width, unsigned crop_height) const {
  Tile t;
  t._mode = _mode;
  t._bpp = _bpp;
  t._width = crop_width;
  t._height = crop_height;
  t._palette = _palette;
  t._data.resize(crop_width * crop_height);

  if (x > _width || y > _height) {
    // no blit
  } else {
    unsigned blit_width = (x + crop_width > _width) ? _width - x : crop_width;
    unsigned blit_height = (y + crop_height > _height) ? _height - y : crop_height;
    for (unsigned iy = 0; iy < blit_height; ++iy) {
      std::memcpy(&t._data[iy * t._width], &_data[(x) + ((iy + y) * _width)], blit_width);
    }
  }

  if (_mirrors.size()) {
    t._mirrors.push_back(mirror(t._data, t._width, true, false));
    t._mirrors.push_back(mirror(t._data, t._width, false, true));
    t._mirrors.push_back(mirror(t._data, t._width, true, true));
  }

  return t;
}

std::vector<Tile> Tile::crops(unsigned tile_width, unsigned tile_height) const {
  std::vector<Tile> tv;
  unsigned x = 0;
  unsigned y = 0;
  while (y < _height) {
    while (x < _width) {
      tv.push_back(crop(x, y, tile_width, tile_height));
      x += tile_width;
    }
    x = 0;
    y += tile_height;
  }
  return tv;
}

std::vector<uint8_t> Tile::native_data() const {
  return pack_native_tile(_data, _mode, _bpp, _width, _height);
}

std::vector<rgba_t> Tile::rgba_data() const {
  std::vector<rgba_t> v(_data.size());
  for (unsigned i = 0; i < _data.size(); ++i) {
    v[i] = (_data[i] == 0) ? transparent_color : _palette[_data[i]];
  }
  return v;
}


Tileset::Tileset(const std::vector<uint8_t> native_data, Mode mode, unsigned bpp,
                 unsigned tile_width, unsigned tile_height, bool no_flip) {
  _mode = mode;
  _bpp = bpp;
  _tile_width = tile_width;
  _tile_height = tile_height;
  _no_flip = no_flip;

  if (_mode == Mode::snes || _mode == Mode::snes_mode7) {
    unsigned bytes_per_tile = bpp << 3;
    if (native_data.size() % bytes_per_tile != 0) {
      throw std::runtime_error("Tile data can't be deserialized (size doesn't match bpp setting)");
    }

    unsigned tiles = (unsigned)native_data.size() / bytes_per_tile;
    for (unsigned i = 0; i < tiles; ++i) {
      _tiles.push_back(
        Tile(std::vector<uint8_t>(&native_data[i * bytes_per_tile], &native_data[(i + 1) * bytes_per_tile]),
             mode, bpp, no_flip, 8, 8));
    }
  }

  if (_tile_width != 8 || _tile_height != 8) _tiles = remap_tiles_for_input(_tiles, _mode);
}

void Tileset::add(const Image& image, const Palette* palette) {
  Tile tile;

  if (_no_remap) {
    tile = Tile(image, _mode, _bpp, _no_flip);
  } else {
    if (palette == nullptr) throw std::runtime_error("Can't remap tile without palette");
    Image remapped_image = Image(image, palette->subpalette_matching(image));
    tile = Tile(remapped_image, _mode, _bpp, _no_flip);
  }

  if (_no_discard) {
    if (is_full()) throw std::runtime_error("Tileset reached maximum size");
    _tiles.push_back(tile);
  } else {
    if (std::find(_tiles.begin(), _tiles.end(), tile) == _tiles.end()) {
      if (is_full()) throw std::runtime_error("Tileset reached maximum size");
      _tiles.push_back(tile);
    } else {
      ++discarded_tiles;
    }
  }
}

int Tileset::index_of(const Tile& tile) const {
  auto index = std::find(_tiles.begin(), _tiles.end(), tile);
  if (index != _tiles.end()) {
    return (int)std::distance(_tiles.begin(), index);
  } else {
    return -1;
  }
}

void Tileset::save(const std::string& path) const {
  write_file(path, native_data());
}

std::vector<uint8_t> Tileset::native_data() const {
  std::vector<Tile> tv;
  if (_tile_width != 8 || _tile_height != 8) {
    tv = remap_tiles_for_output(_tiles, _mode);
  } else {
    tv = _tiles;
  }

  std::vector<uint8_t> data;
  for (const auto& t : tv) {
    auto nt = t.native_data();
    data.insert(data.end(), nt.begin(), nt.end());
  }

  return data;
}

std::vector<Tile> Tileset::remap_tiles_for_output(const std::vector<Tile>& tiles, Mode mode) const {
  std::vector<Tile> tv;

  if (mode == Mode::snes) {
    if (!(_tile_width == 8 || _tile_width == 16 || _tile_height == 8 || _tile_height == 16)) {
      throw std::runtime_error("Programmer error (remap_tiles_for_output Mode::snes with not 16x16, 16x8 or 8x16)");
    }

    const unsigned cells_per_tile_h = _tile_width / 8;
    const unsigned cells_per_tile_v = _tile_height / 8;
    const unsigned cells_per_row = 16;
    const unsigned tiles_per_row = cells_per_row / cells_per_tile_h;
    const unsigned cell_rows = div_ceil((int)tiles.size(), tiles_per_row) * cells_per_tile_v;

    tv.resize(cells_per_row * cell_rows);

    for (unsigned i = 0; i < tiles.size(); ++i) {
      unsigned base_pos = (((i / tiles_per_row) * cells_per_tile_v) * cells_per_row) +
                          ((i % tiles_per_row) << (cells_per_tile_h - 1));
      auto ct = tiles[i].crops(8, 8);
      for (unsigned cy = 0; cy < cells_per_tile_v; ++cy) {
        for (unsigned cx = 0; cx < cells_per_tile_h; ++cx) {
          tv[base_pos + (cy * cells_per_row) + cx] = ct[(cy * cells_per_tile_v) + cx];
        }
      }
    }

  } else if (mode == Mode::snes_mode7) {
    throw std::runtime_error("This can't be!");
  }

  return tv;
}

std::vector<Tile> Tileset::remap_tiles_for_input(const std::vector<Tile>& tiles, Mode mode) const {
  std::vector<Tile> tv;

  if (mode == Mode::snes) {
    if (!(_tile_width == 8 || _tile_width == 16 || _tile_height == 8 || _tile_height == 16)) {
      throw std::runtime_error("Programmer error (remap_tiles_for_input Mode::snes with not 16x16, 16x8 or 8x16)");
    }

    const unsigned cells_per_tile_h = _tile_width / 8;
    const unsigned cells_per_tile_v = _tile_height / 8;

    for (unsigned i = 0; i < _tiles.size(); ++i) {
      std::vector<Tile> metatile;
      for (unsigned yo = 0; yo < cells_per_tile_v; ++yo) {
        for (unsigned xo = 0; xo < cells_per_tile_h; ++xo) {
          if ((i + (yo * 16) + xo) < tiles.size()) metatile.push_back(tiles[i + (yo * 16) + xo]);
        }
      }
      if (metatile.size() == cells_per_tile_h * cells_per_tile_v)
        tv.push_back(Tile(metatile, _no_flip, _tile_width, _tile_height));
    }

  } else if (mode == Mode::snes_mode7) {
    throw std::runtime_error("Programmer error");
  }

  return tv;
}


} /* namespace sfc */
