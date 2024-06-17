// tile/tileset representation
//
// david lindecrantz <optiroc@me.com>

#pragma once

#include "Common.h"
#include "Image.h"
#include "Palette.h"

namespace sfc {

struct Image;
struct Palette;

struct TileFlipped {
  bool h = false;
  bool v = false;
};

struct Tile {
  Tile(const Image& image, Mode mode = Mode::snes, unsigned bpp = 4, bool no_flip = false);

  Tile(const byte_vec_t& native_data, Mode mode = Mode::snes, unsigned bpp = 4, bool no_flip = false, unsigned width = 8,
       unsigned height = 8);

  Tile(const std::vector<Tile>& metatile, bool no_flip, unsigned width, unsigned height);

  Tile(Mode mode, unsigned bpp, unsigned width, unsigned height) : _mode(mode), _bpp(bpp), _width(width), _height(height) {
    _data.resize(width * height);
    _palette.resize(palette_size_at_bpp(bpp));
  };

  Tile(){};

  const index_vec_t& data() const { return _data; }
  const rgba_vec_t& palette() const { return _palette; }
  byte_vec_t native_data() const;
  rgba_vec_t rgba_data() const;

  bool operator==(const Tile& other) const;

  TileFlipped is_flipped(const Tile& other) const;

  Tile crop(unsigned x, unsigned y, unsigned width, unsigned height) const;
  std::vector<Tile> crops(unsigned tile_width, unsigned tile_height) const;

private:
  Mode _mode = Mode::snes;
  unsigned _bpp = 4;
  unsigned _width = 8;
  unsigned _height = 8;
  index_vec_t _data;
  std::vector<index_vec_t> _mirrors;
  rgba_vec_t _palette;
};

struct Tileset {
  Tileset(Mode mode = Mode::snes, unsigned bpp = 4, unsigned tile_width = 8, unsigned tile_height = 8, bool no_discard = false,
          bool no_flip = false, bool no_remap = false, unsigned max_tiles = 0)
      : _mode(mode), _bpp(bpp), _tile_width(tile_width), _tile_height(tile_height), _no_discard(no_discard), _no_flip(no_flip),
        _no_remap(no_remap), _max_tiles(max_tiles){};

  Tileset(const byte_vec_t& native_data, Mode mode = Mode::snes, unsigned bpp = 4, unsigned tile_width = 8,
          unsigned tile_height = 8, bool no_flip = false);

  unsigned tile_width() const { return _tile_width; }
  unsigned tile_height() const { return _tile_height; }
  unsigned size() const { return (unsigned)_tiles.size(); }
  unsigned max() const { return _max_tiles; }
  bool is_full() const { return _max_tiles > 0 && _tiles.size() > _max_tiles; }

  const std::vector<Tile>& tiles() const { return _tiles; }

  int index_of(const Tile& tile) const;
  void add(const Image& image, const Palette* palette = nullptr);

  byte_vec_t native_data() const;
  void save(const std::string& path) const;

  unsigned discarded_tiles = 0;

private:
  Mode _mode = Mode::snes;
  unsigned _bpp = 4;
  unsigned _tile_width = 8;
  unsigned _tile_height = 8;
  bool _no_discard = false;
  bool _no_flip = false;
  bool _no_remap = false;
  unsigned _max_tiles = 0;

  std::vector<Tile> _tiles;

  std::vector<Tile> remap_tiles_for_output(const std::vector<Tile>& tiles, Mode mode) const;
  std::vector<Tile> remap_tiles_for_input(const std::vector<Tile>& tiles, Mode mode) const;
};

// mirror raw image data
template <typename T>
inline std::vector<T> mirror(const std::vector<T>& source, unsigned width, bool horizontal, bool vertical) {
  auto m = source;
  unsigned height = (unsigned)(source.size() / width);
  if ((source.size() % width != 0) || (source.size() != width * height))
    throw std::runtime_error("Can't mirror non-square image vector");

  if (horizontal) {
    for (unsigned row = 0; row < height; ++row)
      std::reverse(m.begin() + (row * width), m.begin() + (row * width) + width);
  }

  if (vertical) {
    std::vector<T> mv;
    for (int row = height - 1; row >= 0; --row) {
      for (unsigned column = 0; column < width; ++column)
        mv.push_back(m[(row * width) + column]);
    }
    m = mv;
  }

  return m;
}

} /* namespace sfc */
