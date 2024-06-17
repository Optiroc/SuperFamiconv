// image representation
//
// david lindecrantz <optiroc@me.com>

#pragma once

#include <LodePNG/lodepng.h>

#include "Common.h"
#include "Mode.h"
#include "Palette.h"
#include "Tiles.h"

namespace sfc {

struct Subpalette;
struct Palette;
struct Tileset;

struct Image final {
  Image(){};
  Image(const std::string& path);
  Image(const sfc::Palette& palette);
  Image(const sfc::Tileset& tileset, unsigned width = 128);
  Image(const Image& image, const sfc::Subpalette& subpalette);

  unsigned width() const { return _width; }
  unsigned height() const { return _height; }
  unsigned src_coord_x() const { return _src_coord_x; }
  unsigned src_coord_y() const { return _src_coord_y; }

  unsigned palette_size() const { return (unsigned)_palette.size(); }

  rgba_vec_t rgba_data() const;
  rgba_vec_t palette() const { return _palette; };
  index_vec_t indexed_data() const { return _indexed_data; }
  rgba_set_t colors() const { return _colors; }

  rgba_t rgba_color_at(unsigned index) const {
    return (_data[index * 4]) + (_data[(index * 4) + 1] << 8) + (_data[(index * 4) + 2] << 16) + (_data[(index * 4) + 3] << 24);
  }

  Image crop(unsigned x, unsigned y, unsigned width, unsigned height, Mode mode) const;
  std::vector<Image> crops(unsigned tile_width, unsigned tile_height, Mode mode) const;

  void save(const std::string& path) const;
  void save_indexed(const std::string& path);
  void save_scaled(const std::string& path, Mode mode);

  const std::string description() const;

private:
  unsigned _width;
  unsigned _height;
  unsigned _src_coord_x;
  unsigned _src_coord_y;
  channel_vec_t _data;
  index_vec_t _indexed_data;
  rgba_vec_t _palette;
  rgba_set_t _colors;

  void set_pixel(const rgba_t color, const unsigned index);
  void set_pixel(const rgba_t color, const unsigned x, const unsigned y);
  void blit(const rgba_vec_t& rgba_data, const unsigned x, const unsigned y, const unsigned width);
  void set_pixel_indexed(const index_t color, const unsigned index);
  void set_pixel_indexed(const index_t color, const unsigned x, const unsigned y);
  void blit_indexed(const index_vec_t& data, const unsigned x, const unsigned y, const unsigned width);

  void set_default_palette(const unsigned indices = 256);
};

} /* namespace sfc */
