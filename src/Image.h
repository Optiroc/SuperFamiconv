// image representation
//
// david lindecrantz <optiroc@gmail.com>

#pragma once

#include <LodePNG/lodepng.h>

#include "Common.h"
#include "Color.h"
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

  rgba_u32_vec rgba_data() const;
  rgba_u32_vec palette() const { return _palette; };
  index_vec indexed_data() const { return _indexed_data; }
  rgba_u32_set colors() const { return _colors; }

  rgba_u32 rgba_u32_at(unsigned index) const {
    return (_data[index * 4]) + (_data[(index * 4) + 1] << 8) + (_data[(index * 4) + 2] << 16) + (_data[(index * 4) + 3] << 24);
  }

  Image quantize(unsigned num_colors, Mode mode = Mode::none, dither::Mode dither = dither::Mode::none) const;
  Image crop(unsigned x, unsigned y, unsigned width, unsigned height, Mode mode) const;
  std::vector<Image> crops(unsigned tile_width, unsigned tile_height, Mode mode) const;

  void save(const std::string& path) const;
  void save_indexed(const std::string& path);

  const std::string description() const;

private:
  unsigned _width;
  unsigned _height;
  unsigned _src_coord_x;
  unsigned _src_coord_y;
  channel_vec _data;
  index_vec _indexed_data;
  rgba_u32_vec _palette;
  rgba_u32_set _colors;

  void set_pixel(const rgba_u32 color, const unsigned index);
  void set_pixel(const rgba_u32 color, const unsigned x, const unsigned y);
  void blit(const rgba_u32_vec& rgba_data, const unsigned x, const unsigned y, const unsigned width);
  void set_pixel_indexed(const index_t color, const unsigned index);
  void set_pixel_indexed(const index_t color, const unsigned x, const unsigned y);
  void blit_indexed(const index_vec& data, const unsigned x, const unsigned y, const unsigned width);

  void set_default_palette(const unsigned indices = 256);
};

} /* namespace sfc */
