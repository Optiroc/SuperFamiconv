// image representation
//
// david lindecrantz <optiroc@gmail.com>

#pragma once

#include <LodePNG/lodepng.h>
#include "Common.h"
#include "Palette.h"
#include "Tiles.h"

namespace sfc {

struct Subpalette;
struct Palette;
struct Tileset;

struct Image {
  Image(){};
  Image(const std::string& path);
  Image(const sfc::Palette& palette);
  Image(const sfc::Tileset& tileset);
  Image(const Image& image, const sfc::Subpalette& subpalette);

  unsigned width() const { return _width; }
  unsigned height() const { return _height; }
  unsigned palette_size() const { return (unsigned)_palette.size(); }
  std::vector<rgba_t> palette() const { return _palette; };
  std::vector<index_t> indexed_data() const { return _indexed_data; }

  rgba_t rgba_color_at(unsigned index) const {
    return (_data[index * 4]) + (_data[(index * 4) + 1] << 8) +
           (_data[(index * 4) + 2] << 16) + (_data[(index * 4) + 3] << 24);
  }

  std::vector<rgba_t> rgba_data() const;

  Image crop(unsigned x, unsigned y, unsigned width, unsigned height) const;
  std::vector<Image> crops(unsigned tile_width, unsigned tile_height) const;
  std::vector<std::vector<rgba_t>> rgba_crops(unsigned tile_width, unsigned tile_height) const;
  std::vector<std::vector<index_t>> indexed_crops(unsigned tile_width, unsigned tile_height) const;

  void save(const std::string& path) const;
  void save_indexed(const std::string& path);

private:
  unsigned _width;
  unsigned _height;
  std::vector<channel_t> _data;
  std::vector<index_t> _indexed_data;
  std::vector<rgba_t> _palette;

  void set_pixel(const rgba_t color, const unsigned index);
  void set_pixel(const rgba_t color, const unsigned x, const unsigned y);
  void blit(const std::vector<rgba_t>& rgba_data, const unsigned x, const unsigned y, const unsigned width);
  void set_pixel_indexed(const index_t color, const unsigned index);
  void set_pixel_indexed(const index_t color, const unsigned x, const unsigned y);
  void blit_indexed(const std::vector<index_t>& data, const unsigned x, const unsigned y, const unsigned width);

  void set_default_palette(const unsigned indices = 256);
};

std::ostream& operator<<(std::ostream& os, const Image& img);

} /* namespace sfc */
