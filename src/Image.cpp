#include "Image.h"

namespace sfc {

Image::Image(const std::string& path) {
  byte_vec buffer;
  unsigned w, h;

  unsigned error = lodepng::load_file(buffer, path);
  if (error)
    throw std::runtime_error(lodepng_error_text(error));

  lodepng::State state;
  state.decoder.color_convert = false;
  state.decoder.ignore_crc = true;

  error = lodepng::decode(_data, w, h, state, buffer);
  if (error)
    throw std::runtime_error(lodepng_error_text(error));

  bool needs_conversion = false;

  if (state.info_raw.colortype == LCT_PALETTE) {
    if (state.info_raw.bitdepth && state.info_raw.bitdepth < 8) {
      // unpack 2/4 bit data
      _indexed_data = index_vec(w * h);
      unsigned depth = state.info_raw.bitdepth;
      unsigned ppb = 8 / state.info_raw.bitdepth;
      index_t mask = 0;
      for (unsigned i = 0; i < depth; ++i)
        mask = (mask << 1) + 1;

      for (unsigned i = 0; i < _indexed_data.size(); ++i) {
        unsigned pack_shift = 8 - depth - ((i * depth) % 8);
        _indexed_data[i] = mask & (_data[i / ppb] >> pack_shift);
      }

    } else {
      _indexed_data = _data;
    }

    for (unsigned i = 0; i < state.info_raw.palettesize * 4; i += 4) {
      uint32_t color = (state.info_raw.palette[i]) + (state.info_raw.palette[i + 1] << 8) +
                       (state.info_raw.palette[i + 2] << 16) + (state.info_raw.palette[i + 3] << 24);
      _palette.push_back(color);
    }

    needs_conversion = true;
    state.info_raw.colortype = LCT_RGBA;
  }

  if (state.info_png.color.colortype == LCT_RGB || state.info_png.color.colortype == LCT_GREY ||
      state.info_png.color.colortype == LCT_GREY_ALPHA) {
    state.info_raw.colortype = LCT_RGBA;
    needs_conversion = true;
  }

  if (state.info_png.color.bitdepth != 8) {
    state.info_raw.bitdepth = 8;
    needs_conversion = true;
  }

  if (needs_conversion) {
    _data.clear();
    state.decoder.color_convert = true;
    error = lodepng::decode(_data, w, h, state, buffer);
    if (error)
      throw std::runtime_error(lodepng_error_text(error));
  }

  _width = w;
  _height = h;

  _src_coord_x = _src_coord_y = 0;

  auto rgba_v = rgba_data();
  _colors = rgba_u32_set(rgba_v.begin(), rgba_v.end());
}

Image::Image(const sfc::Palette& palette) {
  auto v = palette.normalized_colors();
  if (v.empty() || v[0].empty())
    throw std::runtime_error("No colors");

  _width = palette.max_colors_per_subpalette();
  _height = (unsigned)v.size();
  _data.resize(_width * _height * 4);
  std::fill(_data.begin(), _data.end(), 0);

  for (unsigned y = 0; y < v.size(); ++y) {
    auto vy = v[y];
    for (unsigned x = 0; x < vy.size(); ++x)
      set_pixel(sfc::rgba_color(vy[x]), x, y);
  }

  _src_coord_x = _src_coord_y = 0;

  auto rgba_v = rgba_data();
  _colors = rgba_u32_set(rgba_v.begin(), rgba_v.end());
}

Image::Image(const sfc::Tileset& tileset, unsigned image_width) {
  if (!image_width)
    image_width = 128;

  const auto tiles = tileset.tiles();
  const unsigned tile_width = tileset.tile_width();
  const unsigned tile_height = tileset.tile_height();
  const unsigned tiles_per_row = sfc::div_ceil(image_width, tile_width);
  const unsigned rows = sfc::div_ceil(tileset.size(), tiles_per_row);

  _width = image_width;
  _height = rows * tileset.tile_height();
  _data.resize(_width * _height * 4);
  std::fill(_data.begin(), _data.end(), 0);
  _indexed_data.resize(_width * _height);
  std::fill(_indexed_data.begin(), _indexed_data.end(), 0);
  if (_data.empty())
    return;

  _palette = tileset.tiles()[0].palette();

  for (unsigned tile_index = 0; tile_index < tiles.size(); ++tile_index) {
    auto tile_rgba = tiles[tile_index].rgba_data();
    blit(tile_rgba, (tile_index % tiles_per_row) * tile_width, (tile_index / tiles_per_row) * tile_height, tile_width);
    auto tile_data = tiles[tile_index].data();
    blit_indexed(tile_data, (tile_index % tiles_per_row) * tile_width, (tile_index / tiles_per_row) * tile_height, tile_width);
  }

  _src_coord_x = _src_coord_y = 0;

  auto rgba_v = rgba_data();
  _colors = rgba_u32_set(rgba_v.begin(), rgba_v.end());
}

// Make new normalized image with color indices mapped to palette
Image::Image(const Image& image, const sfc::Subpalette& subpalette)
    : _width(image.width()), _height(image.height()), _palette(subpalette.normalized_colors()) {
  if (_palette.empty())
    throw std::runtime_error("No colors");

  unsigned size = _width * _height;
  _indexed_data.resize(size);
  _data.resize(size * 4);

  for (unsigned i = 0; i < size; ++i) {
    rgba_u32 color = sfc::normalize_color(sfc::reduce_color(image.rgba_u32_at(i), subpalette.mode()), subpalette.mode());
    if (color == transparent_color) {
      _indexed_data[i] = 0;
      set_pixel(transparent_color, i);
    } else {
      size_t palette_index = std::find(_palette.begin(), _palette.end(), color) - _palette.begin();
      if (palette_index < _palette.size()) {
        _indexed_data[i] = (index_t)palette_index;
        set_pixel(sfc::rgba_color(_palette[palette_index]), i);
      } else {
        throw std::runtime_error("Color not in palette");
      }
    }
  }

  _src_coord_x = _src_coord_y = 0;

  auto rgba_v = rgba_data();
  _colors = rgba_u32_set(rgba_v.begin(), rgba_v.end());
}

rgba_u32_vec Image::rgba_data() const {
  return sfc::to_rgba(_data);
}

Image Image::quantize(unsigned num_colors, Mode mode, dither::Mode dither) const {
  // quantize scaled+normalized colors
  const rgba_u32_vec pv = mode == Mode::none ? rgba_data() : normalize_colors(reduce_colors(rgba_data(), mode), mode);
  std::vector<rgba_color> palette = mediancut_palette(to_rgba_color_vec(pv), num_colors);

  // remap and dither new image
  unsigned size = _width * _height;
  Image img;
  img._width = _width;
  img._height = _height;
  img._data.resize(size * 4);
  img._indexed_data.resize(size);

  if (dither == dither::Mode::none) {

  } else {
    // TODO: NOP for now...
  }

  return img;
  /*
  const dither::Matrix& mtx = dither::matrix(dither);
  double fudge = 2.5;
  double step = palette_step(palette) * fudge;
  fmt::print("Size: {}\n", step);

  for (unsigned i = 0; i < size; ++i) {
    unsigned x = i % _width;
    unsigned y = i / _width;
    rgba_color c = rgba_color(rgba_u32_at(i)).add(step * mtx.range * mtx.map[(x % mtx.width) + ((y % mtx.height) * mtx.width)] + (mtx.offset * fudge));
    img.set_pixel(closest_de2000(c, palette), i);
    img.set_pixel_indexed(0, i);
  }
  */
}

Image Image::crop(unsigned x, unsigned y, unsigned crop_width, unsigned crop_height, Mode mode) const {
  Image img;
  img._palette = _palette;
  img._width = crop_width;
  img._height = crop_height;
  img._src_coord_x = x;
  img._src_coord_y = y;
  img._data.resize(crop_width * crop_height * 4);

  uint32_t fillval = mode == Mode::gb ? 0xff000000 : transparent_color;
  size_t fillsize = img._data.size();
  for (size_t i = 0; i < fillsize; i += 4)
    std::memcpy(img._data.data() + i, &fillval, sizeof(fillval));

  if (x > _width || y > _height) {
    // Crop outside source image: return empty
    if (_indexed_data.size())
      img._indexed_data.resize(crop_width * crop_height);
    return img;
  }

  unsigned blit_width = (x + crop_width > _width) ? _width - x : crop_width;
  unsigned blit_height = (y + crop_height > _height) ? _height - y : crop_height;

  for (unsigned iy = 0; iy < blit_height; ++iy) {
    std::memcpy(&img._data[iy * img._width * 4], &_data[(x * 4) + ((iy + y) * _width * 4)], blit_width * 4);
  }

  if (_indexed_data.size()) {
    img._indexed_data.resize(crop_width * crop_height);
    for (unsigned iy = 0; iy < blit_height; ++iy) {
      std::memcpy(&img._indexed_data[iy * img._width], &_indexed_data[x + ((iy + y) * _width)], blit_width);
    }
  }

  auto rgba_v = img.rgba_data();
  img._colors = rgba_u32_set(rgba_v.begin(), rgba_v.end());
  return img;
}

std::vector<Image> Image::crops(unsigned tile_width, unsigned tile_height, Mode mode) const {
  std::vector<Image> v;
  unsigned x = 0;
  unsigned y = 0;
  while (y < _height) {
    while (x < _width) {
      v.push_back(crop(x, y, tile_width, tile_height, mode));
      x += tile_width;
    }
    x = 0;
    y += tile_width;
  }
  return v;
}

void Image::save(const std::string& path) const {
  unsigned error = lodepng::encode(path.c_str(), _data, _width, _height, LCT_RGBA, 8);
  if (error)
    throw std::runtime_error(lodepng_error_text(error));
}

void Image::save_indexed(const std::string& path) {
  if (_palette.empty())
    set_default_palette();

  lodepng::State state;
  for (const auto& c : _palette) {
    rgba_color rgba(c);
    lodepng_palette_add(&state.info_png.color, rgba.r, rgba.g, rgba.b, rgba.a);
    lodepng_palette_add(&state.info_raw, rgba.r, rgba.g, rgba.b, rgba.a);
  }
  state.info_png.color.colortype = state.info_raw.colortype = LCT_PALETTE;
  state.info_png.color.bitdepth = state.info_raw.bitdepth = 8;
  state.encoder.auto_convert = 0;

  byte_vec buffer;
  unsigned error = lodepng::encode(buffer, _indexed_data, _width, _height, state);
  if (error)
    throw std::runtime_error(lodepng_error_text(error));
  lodepng::save_file(buffer, path.c_str());
}

const std::string Image::description() const {
  return fmt::format("{}x{}px, {}", width(), height(), palette_size() ? "indexed color" : "RGB color");
}

inline void Image::set_pixel(const rgba_u32 color, const unsigned index) {
  const unsigned offset = index * 4;
  if ((offset + 3) > _data.size())
    return;
  _data[offset + 0] = (channel_t)(color & 0xff);
  _data[offset + 1] = (channel_t)((color >> 8) & 0xff);
  _data[offset + 2] = (channel_t)((color >> 16) & 0xff);
  _data[offset + 3] = (channel_t)((color >> 24) & 0xff);
  _colors.insert(color);
}

inline void Image::set_pixel(const rgba_u32 color, const unsigned x, const unsigned y) {
  const unsigned offset = ((y * _width) + x) * 4;
  if ((offset + 3) > _data.size())
    return;
  _data[offset + 0] = (channel_t)(color & 0xff);
  _data[offset + 1] = (channel_t)((color >> 8) & 0xff);
  _data[offset + 2] = (channel_t)((color >> 16) & 0xff);
  _data[offset + 3] = (channel_t)((color >> 24) & 0xff);
  _colors.insert(color);
}

void Image::blit(const rgba_u32_vec& rgba_data, const unsigned x, const unsigned y, const unsigned width) {
  for (unsigned i = 0; i < rgba_data.size(); ++i)
    set_pixel(rgba_data[i], (i % width) + x, (i / width) + y);
}

inline void Image::set_pixel_indexed(const index_t color, const unsigned index) {
  if ((index) > _indexed_data.size())
    return;
  _indexed_data[index] = color;
}

inline void Image::set_pixel_indexed(const index_t color, const unsigned x, const unsigned y) {
  const unsigned offset = (y * _width) + x;
  if (offset > _indexed_data.size())
    return;
  _indexed_data[offset] = color;
}

void Image::blit_indexed(const channel_vec& data, const unsigned x, const unsigned y, const unsigned width) {
  for (unsigned i = 0; i < data.size(); ++i)
    set_pixel_indexed(data[i], (i % width) + x, (i / width) + y);
}

void Image::set_default_palette(const unsigned indices) {
  _palette.resize(indices);
  channel_t add = 0x100 / _palette.size();
  for (unsigned i = 0; i < _palette.size(); ++i) {
    channel_t value = add * i;
    _palette[i] = (rgba_u32)(0xff000000 + value + (value << 8) + (value << 16));
  }
}

} /* namespace sfc */
