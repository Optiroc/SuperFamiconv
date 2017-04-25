// common types and utility functions
//
// david lindecrantz <optiroc@gmail.com>

#pragma once

#include <algorithm>
#include <climits>
#include <cmath>
#include <cstdint>
#include <fstream>
#include <functional>
#include <iomanip>
#include <iostream>
#include <iterator>
#include <memory>
#include <set>
#include <stdexcept>
#include <string>
#include <unordered_set>
#include <vector>

#include <json.hpp>

typedef uint32_t rgba_t;   // rgba color stored in little endian order
typedef uint8_t channel_t; // rgba color channel
typedef uint8_t index_t;   // color index (typedefd in case more than 8 bits are needed down the road)

namespace sfc {

constexpr const char* LICENSE =
  "Permission is hereby granted, free of charge, to any person obtaining a copy "
  "of this software and associated documentation files (the \"Software\"), to deal "
  "in the Software without restriction, including without limitation the rights "
  "to use, copy, modify, merge, publish, distribute, sublicense, and/or sell "
  "copies of the Software, and to permit persons to whom the Software is "
  "furnished to do so, subject to the following conditions:"
  "\n\n"
  "The above copyright notice and this permission notice shall be included in "
  "all copies or substantial portions of the Software."
  "\n\n"
  "THE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR "
  "IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, "
  "FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE "
  "AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER "
  "LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, "
  "OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN "
  "THE SOFTWARE."

  "\n\n\n"

  "LodePNG version 20161127"
  "\n"
  "Copyright (c) 2005-2016 Lode Vandevennen"
  "\n\n"
  "This software is provided 'as-is', without any express or implied "
  "warranty. In no event will the authors be held liable for any damages "
  "arising from the use of this software. "
  "\n\n"
  "Permission is granted to anyone to use this software for any purpose, "
  "including commercial applications, and to alter it and redistribute it "
  "freely, subject to the following restrictions:"
  "\n\n"
  "1. The origin of this software must not be misrepresented; you must not "
  "claim that you wrote the original software. If you use this software "
  "in a product, an acknowledgment in the product documentation would be "
  "appreciated but is not required."
  "\n\n"
  "2. Altered source versions must be plainly marked as such, and must not be "
  "misrepresented as being the original software."
  "\n\n"
  "3. This notice may not be removed or altered from any source distribution."

  "\n\n\n"

  "    __ _____ _____ _____ \n"
  " __|  |   __|     |   | |  JSON for Modern C++\n"
  "|  |  |__   |  |  | | | |  version 2.0.10\n"
  "|_____|_____|_____|_|___|  https://github.com/nlohmann/json\n"
  "\n"
  "Licensed under the MIT License <http://opensource.org/licenses/MIT>.\n"
  "Copyright (c) 2013-2017 Niels Lohmann <http://nlohmann.me>."
  "\n\n"
  "Permission is hereby  granted, free of charge, to any  person obtaining a copy "
  "of this software and associated  documentation files (the \"Software\"), to deal "
  "in the Software  without restriction, including without  limitation the rights "
  "to  use, copy,  modify, merge,  publish, distribute,  sublicense, and/or  sell "
  "copies  of  the Software,  and  to  permit persons  to  whom  the Software  is "
  "furnished to do so, subject to the following conditions:"
  "\n\n"
  "The above copyright notice and this permission notice shall be included in all "
  "copies or substantial portions of the Software."
  "\n\n"
  "THE SOFTWARE  IS PROVIDED \"AS  IS\", WITHOUT WARRANTY  OF ANY KIND,  EXPRESS OR "
  "IMPLIED,  INCLUDING BUT  NOT  LIMITED TO  THE  WARRANTIES OF  MERCHANTABILITY, "
  "FITNESS FOR  A PARTICULAR PURPOSE AND  NONINFRINGEMENT. IN NO EVENT  SHALL THE "
  "AUTHORS  OR COPYRIGHT  HOLDERS  BE  LIABLE FOR  ANY  CLAIM,  DAMAGES OR  OTHER "
  "LIABILITY, WHETHER IN AN ACTION OF  CONTRACT, TORT OR OTHERWISE, ARISING FROM, "
  "OUT OF OR IN CONNECTION WITH THE SOFTWARE  OR THE USE OR OTHER DEALINGS IN THE "
  "SOFTWARE."
  "\n";

enum Constants {
  options_indent = 24
};

enum class Mode {
  snes,
  snes_mode7
};

const rgba_t transparent_color = 0x00000000;

inline Mode mode(std::string& str) {
  if (str == "snes") {
    return Mode::snes;
  } else if (str == "snes_mode7") {
    return Mode::snes_mode7;
  }
  return Mode::snes;
}

inline std::string mode(Mode mode) {
  switch (mode) {
  case Mode::snes:
    return std::string("snes");
  case Mode::snes_mode7:
    return std::string("snes_mode7");
  default:
    return std::string();
  }
}

inline bool bpp_allowed_for_mode(unsigned bpp, Mode mode) {
  switch (mode) {
  case Mode::snes:
    return bpp == 2 || bpp == 4 || bpp == 8;
  case Mode::snes_mode7:
    return bpp == 8;
  default:
    return false;
  }
}

inline bool tile_width_allowed_for_mode(unsigned width, Mode mode) {
  switch (mode) {
  case Mode::snes:
    return width == 8 || width == 16;
  case Mode::snes_mode7:
    return width == 8;
  default:
    return false;
  }
}

inline bool tile_height_allowed_for_mode(unsigned height, Mode mode) {
  switch (mode) {
  case Mode::snes:
    return height == 8 || height == 16;
  case Mode::snes_mode7:
    return height == 8;
  default:
    return false;
  }
}

inline unsigned default_tile_size_for_mode(Mode mode) {
  switch (mode) {
  default:
    return 8;
  }
}

inline unsigned default_bpp_for_mode(Mode mode) {
  switch (mode) {
    case Mode::snes:
      return 4;
    case Mode::snes_mode7:
      return 8;
    default:
      return 4;
  }
}

inline unsigned default_map_size_for_mode(Mode mode) {
  switch (mode) {
  case Mode::snes:
    return 32;
  case Mode::snes_mode7:
    return 128;
  default:
    return 32;
  }
}

inline unsigned palette_size_at_bpp(unsigned bpp) {
  unsigned s = 1;
  for (unsigned i = 0; i < bpp; ++i) s = s << 1;
  return s;
}

inline index_t bitmask_at_bpp(unsigned bpp) {
  unsigned m = 1;
  for (unsigned i = 1; i < bpp; ++i) m |= m << 1;
  return (index_t)m;
}

inline std::vector<rgba_t> to_rgba(std::vector<channel_t> data) {
  if (data.size() % 4 != 0) throw std::runtime_error("RGBA vector size not a multiple of 4");
  std::vector<rgba_t> v(data.size() >> 2);
  for (unsigned i = 0; i < v.size(); ++i) {
    v[i] = (data[i * 4]) + (data[(i * 4) + 1] << 8) + (data[(i * 4) + 2] << 16) + (data[(i * 4) + 3] << 24);
  }
  return v;
}

//
// color transformations
//

struct rgba_color {
  channel_t r;
  channel_t g;
  channel_t b;
  channel_t a;

  rgba_color(){};
  rgba_color(rgba_t c)
  : r(c & 0x000000ff),
    g((c & 0x0000ff00) >> 8),
    b((c & 0x00ff0000) >> 16),
    a((c & 0xff000000) >> 24){};

  operator rgba_t() {
    rgba_t i = r;
    i += g << 8;
    i += b << 16;
    i += a << 24;
    return i;
  }

  bool operator>(const rgba_color& other) const;
};

struct hsva_color {
  float h;
  float s;
  float v;
  float a;

  hsva_color(){};
  hsva_color(const rgba_color& rgba);
  operator rgba_color();
};

// rgba to hsva
inline hsva_color::hsva_color(const rgba_color& rgba) {
  float rgb_max = fmax(fmax(rgba.r, rgba.g), rgba.b);
  float rgb_min = fmin(fmin(rgba.r, rgba.g), rgba.b);
  float rgb_delta = rgb_max - rgb_min;

  if (rgb_delta > 0) {
    if (rgb_max == rgba.r) {
      h = 60.0f * (fmod(((rgba.g - rgba.b) / rgb_delta), 6.0f));
    } else if (rgb_max == rgba.g) {
      h = 60.0f * (((rgba.b - rgba.r) / rgb_delta) + 2.0f);
    } else if (rgb_max == rgba.b) {
      h = 60.0f * (((rgba.r - rgba.g) / rgb_delta) + 4.0f);
    }
    s = (rgb_max > 0) ? rgb_delta / rgb_max : 0;
    v = rgb_max;
  } else {
    h = 0;
    s = 0;
    v = rgb_max;
  }

  if (h < 0) h = 360 + h;
  a = rgba.a / 255.0f;
}

// hsva to rgba
inline hsva_color::operator rgba_color() {
  rgba_color rgba;
  float c = v * s;
  float p = fmod(h / 60.0f, 6.0f);
  float x = c * (1.0f - fabs(fmod(p, 2.0f) - 1.0f));
  float m = v - c;

  if (0.0f <= p && p < 1.0f) {
    rgba.r = c;
    rgba.g = x;
    rgba.b = 0.0f;
  } else if (1.0f <= p && p < 2.0f) {
    rgba.r = x;
    rgba.g = c;
    rgba.b = 0.0f;
  } else if (2.0f <= p && p < 3.0f) {
    rgba.r = 0.0f;
    rgba.g = c;
    rgba.b = x;
  } else if (3.0f <= p && p < 4.0f) {
    rgba.r = 0.0f;
    rgba.g = x;
    rgba.b = c;
  } else if (4.0f <= p && p < 5.0f) {
    rgba.r = x;
    rgba.g = 0.0f;
    rgba.b = c;
  } else if (5.0f <= p && p < 6.0f) {
    rgba.r = c;
    rgba.g = 0.0f;
    rgba.b = x;
  } else {
    rgba.r = 0.0f;
    rgba.g = 0.0f;
    rgba.b = 0.0f;
  }

  rgba.r += m;
  rgba.g += m;
  rgba.b += m;
  rgba.a = a * 255.0f;
  return rgba;
}

// color sorting
inline bool rgba_color::operator>(const rgba_color& o) const {
  const int segments = 8;
  hsva_color hsva = hsva_color(*this);
  hsva_color hsva_o = hsva_color(o);

  int h = (int)(segments * hsva.h);
  // int l = (int)(segments * hsva.s);
  int l = (int)(segments * sqrt(0.241f * r + 0.691f * g + 0.068f * b));
  int v = (int)(segments * hsva.v);

  int ho = (int)(segments * hsva_o.h);
  // int lo = (int)(segments * hsva_o.s);
  int lo = (int)(segments * sqrt(0.241f * o.r + 0.691f * o.g + 0.068f * o.b));
  int vo = (int)(segments * hsva_o.v);

  return std::tie(h, l, v) > std::tie(ho, lo, vo);
}

inline void sort_colors(std::vector<rgba_t>& colors) {
  std::sort(colors.begin(), colors.end(), [](const rgba_t& a, const rgba_t& b) -> bool {
    return rgba_color(a) > rgba_color(b);
  });
}

// swap bytes between network order and little endian
inline rgba_t reverse_bytes(rgba_t v) {
  return ((v >> 24) & 0xff) | ((v << 8) & 0xff0000) | ((v >> 8) & 0xff00) | ((v << 24) & 0xff000000);
}

// scale up using left bit replication
inline channel_t scale_up(channel_t value, unsigned shift) {
  return (value << shift) | (value >> (8 - shift));
}

// scale standard rgba color to system specific range
inline rgba_t reduce_color(const rgba_t color, Mode to_mode) {
  switch (to_mode) {
  case Mode::snes:
  case Mode::snes_mode7:
    if (((color & 0xff000000) >> 24) < 0x80) {
      return transparent_color;
    } else {
      rgba_color c(color);
      c.r >>= 3;
      c.g >>= 3;
      c.b >>= 3;
      rgba_t scaled = c;
      return (scaled & 0x00ffffff) + 0xff000000;
    }
    break;
  default:
    return 0;
  }
}

// scale standard rgba colors to system specific range
inline std::vector<rgba_t> reduce_colors(const std::vector<rgba_t>& colors, Mode to_mode) {
  auto vc = colors;
  for (rgba_t& color : vc) color = reduce_color(color, to_mode);
  return vc;
}

// scale color from system specific range to 8bpc RGBA range
inline rgba_t normalize_color(const rgba_t color, Mode from_mode) {
  rgba_color c(color);
  switch (from_mode) {
  case Mode::snes:
  case Mode::snes_mode7:
    c.r = scale_up(c.r, 3);
    c.g = scale_up(c.g, 3);
    c.b = scale_up(c.b, 3);
    c.a = scale_up(c.a, 3);
    return c;
  default:
    return 0;
  }
}

// scale colors from system specific range to 8bpc RGBA range
inline std::vector<rgba_t> normalize_colors(const std::vector<rgba_t>& colors, Mode from_mode) {
  auto vc = colors;
  for (rgba_t& color : vc) color = normalize_color(color, from_mode);
  return vc;
}

// pack scaled rgba color to native format
inline std::vector<uint8_t> pack_native_color(const rgba_t color, Mode mode) {
  std::vector<uint8_t> v;
  switch (mode) {
  case Mode::snes:
  case Mode::snes_mode7:
    v.push_back((color & 0x1f) | ((color >> 3) & 0xe0));
    v.push_back(((color >> 11) & 0x3) | ((color >> 14) & 0x7c));
    break;
  }
  return v;
}

// unpack native format color to (scaled) rgba color
inline std::vector<rgba_t> unpack_native_colors(const std::vector<uint8_t> colors, Mode mode) {
  std::vector<rgba_t> v;
  switch (mode) {
  case Mode::snes:
  case Mode::snes_mode7:
    if (colors.size() % 2 != 0) {
      throw std::runtime_error("SNES native color data vector size not a multiple of 2");
    }
    for (unsigned i = 0; i < colors.size(); i += 2) {
      uint16_t cw = (colors[i + 1] << 8) + colors[i];
      rgba_t nc = (cw & 0x1f) | ((cw & 0x3e0) << 3) | ((cw & 0x7c00) << 6) | 0xff000000;
      v.push_back(nc);
    }
    break;
  }
  return v;
}

// pack raw image data to native format tile data
inline std::vector<uint8_t> pack_native_tile(const std::vector<index_t>& data, Mode mode, unsigned bpp, unsigned width, unsigned height) {
  std::vector<uint8_t> nd;

  auto make_nintendo_2bit_data = [](const std::vector<index_t>& in_data, unsigned plane_index) {
    std::vector<uint8_t> p(16);
    if (in_data.empty()) return p;

    index_t mask0 = 1;
    unsigned shift0 = 0;
    for (unsigned i = 0; i < plane_index; ++i) {
      mask0 = mask0 << 1;
      ++shift0;
    }
    index_t mask1 = mask0 << 1;
    unsigned shift1 = shift0 + 1;

    for (unsigned y = 0; y < 8; ++y) {
      for (unsigned x = 0; x < 8; ++x) {
        p[y * 2 + 0] |= ((in_data[y * 8 + x] & mask0) >> shift0) << (7 - x);
        p[y * 2 + 1] |= ((in_data[y * 8 + x] & mask1) >> shift1) << (7 - x);
      }
    }
    return p;
  };

  if (mode == Mode::snes) {
    if (width != 8 || height != 8) throw std::runtime_error("Programmer error");
    unsigned planes = bpp >> 1;
    for (unsigned i = 0; i < planes; ++i) {
      auto plane = make_nintendo_2bit_data(data, i * 2);
      nd.insert(nd.end(), plane.begin(), plane.end());
    }

  } else if (mode == Mode::snes_mode7) {
    if (width != 8 || height != 8) throw std::runtime_error("Programmer error (fix mode 7 packing!)");
    nd = data;
  }

  return nd;
}

// unpack native format tile data to raw image data
inline std::vector<index_t> unpack_native_tile(const std::vector<uint8_t>& data, Mode mode, unsigned bpp, unsigned width, unsigned height) {
  std::vector<index_t> ud(width * height);

  auto add_nintendo_1bit_plane = [](
    std::vector<index_t>& out_data, const std::vector<uint8_t>& in_data, unsigned plane_index) {
    int plane_offset = ((plane_index >> 1) * 16) + (plane_index & 1);
    for (int y = 0; y < 8; ++y) {
      for (int x = 0; x < 8; ++x) {
        out_data[y * 8 + x] += ((in_data[plane_offset + (y * 2)] >> (7 - x)) & 1) << plane_index;
      }
    }
  };

  if (mode == Mode::snes) {
    if (width != 8 || height != 8) throw std::runtime_error("Programmer error");
    for (unsigned i = 0; i < bpp; ++i) add_nintendo_1bit_plane(ud, data, i);
  } else if (mode == Mode::snes_mode7) {
    ud = data;
  }

  return ud;
}

// mirror raw image data
template <typename T>
inline std::vector<T> mirror(const std::vector<T>& source, unsigned width, bool horizontal, bool vertical) {
  auto m = source;
  unsigned height = (unsigned)(source.size() / width);
  if ((source.size() % width != 0) || (source.size() != width * height))
    throw std::runtime_error("Can't mirror non-square image vector");

  if (horizontal) {
    for (unsigned row = 0; row < height; ++row) std::reverse(m.begin() + (row * width), m.begin() + (row * width) + width);
  }

  if (vertical) {
    std::vector<T> mv;
    for (int row = height - 1; row >= 0; --row) {
      for (unsigned column = 0; column < width; ++column) mv.push_back(m[(row * width) + column]);
    }
    m = mv;
  }

  return m;
}

// rgba value to css style hex string
inline std::string to_hexstring(rgba_t value, bool pound = true, bool alpha = false) {
  value = reverse_bytes(value);
  std::stringstream ss;
  if (pound) ss << '#';
  if (alpha) {
    ss << std::setfill('0') << std::setw(8) << std::hex << value;
  } else {
    ss << std::setfill('0') << std::setw(6) << std::hex << (value >> 8);
  }
  return ss.str();
}

// css style hex string to rgba value
inline rgba_t from_hexstring(std::string str) {
  if (str.at(0) == '#') str.erase(0, 1);
  if (str.size() == 6) str.insert(6, 2, 'f');
  if (str.size() != 8) throw;
  uint32_t i;
  sscanf(str.c_str(), "%x", &i);
  return reverse_bytes(i);
}

//
// utility i/o functions
//

// parse json at path
inline nlohmann::json read_json_file(const std::string& path) {
  nlohmann::json j;
  std::ifstream f(path);
  if (f.fail()) {
    std::stringstream ss;
    ss << "File \"" << path << "\" could not be opened";
    throw std::runtime_error(ss.str());
  }
  f >> j;
  return j;
}

// read text file at path
inline std::string read_file(const std::string& path) {
  std::ifstream ifs(path);
  if (ifs.fail()) {
    std::stringstream ss;
    ss << "File \"" << path << "\" could not be opened";
    throw std::runtime_error(ss.str());
  }
  return std::string((std::istreambuf_iterator<char>(ifs)), (std::istreambuf_iterator<char>()));
}

// read binary file at path
inline std::vector<uint8_t> read_binary(const std::string& path) {
  std::ifstream ifs(path, std::ios::binary);
  if (ifs.fail()) {
    std::stringstream ss;
    ss << "File \"" << path << "\" could not be opened";
    throw std::runtime_error(ss.str());
  }
  return std::vector<uint8_t>((std::istreambuf_iterator<char>(ifs)), (std::istreambuf_iterator<char>()));
}

// write text file
inline void write_file(const std::string& path, const std::string& contents) {
  std::ofstream ofs(path, std::ofstream::out | std::ofstream::trunc);
  ofs << contents;
}

// write binary file
template <typename T>
inline void write_file(const std::string& path, const std::vector<T>& data) {
  std::ofstream ofs(path, std::ios::out | std::ofstream::trunc | std::ios::binary);
  ofs.write(reinterpret_cast<const char*>(&data[0]), data.size() * sizeof(T));
}

//
// general misc
//

inline int div_ceil(int numerator, int denominator) {
  return (numerator / denominator) + (((numerator < 0) ^ (denominator > 0)) && (numerator % denominator));
}

template <typename T>
std::vector<T> split_vector(const T& vect, unsigned split_size) {
  std::vector<T> sv;
  typename T::const_iterator it = vect.cbegin();
  const typename T::const_iterator end = vect.cend();

  while (it != end) {
    T v;
    std::back_insert_iterator<T> inserter(v);
    const auto num_to_copy = std::min(static_cast<unsigned>(std::distance(it, end)), split_size);
    std::copy(it, it + num_to_copy, inserter);
    sv.push_back(std::move(v));
    std::advance(it, num_to_copy);
  }
  return sv;
}

} /* namespace sfc */
