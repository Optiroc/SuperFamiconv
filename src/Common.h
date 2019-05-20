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
#include <iterator>
#include <memory>
#include <set>
#include <stdexcept>
#include <string>
#include <vector>

#include <fmt/format.h>
#include <nlohmann/json.hpp>
#include "Mode.h"

typedef uint8_t index_t;   // color index (typedefd in case more than 8 bits are needed down the road)
typedef uint8_t channel_t; // rgba color channel
typedef uint32_t rgba_t;   // rgba color stored in little endian order

typedef std::vector<uint8_t> byte_vec_t;
typedef std::vector<index_t> index_vec_t;
typedef std::vector<channel_t> channel_vec_t;
typedef std::vector<rgba_t> rgba_vec_t;
typedef std::set<rgba_t> rgba_set_t;
typedef std::vector<rgba_set_t> rgba_set_vec_t;

namespace sfc {

constexpr const char* VERSION =
  "0.8.3";

constexpr const char* COPYRIGHT =
  "Copyright (c) 2017-2019 David Lindecrantz";

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

  "LodePNG version 20190210"
  "\n"
  "Copyright (c) 2005-2019 Lode Vandevennen"
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
  "|  |  |__   |  |  | | | |  version 3.6.1\n"
  "|_____|_____|_____|_|___|  https://github.com/nlohmann/json\n"
  "\n"
  "Licensed under the MIT License <http://opensource.org/licenses/MIT>.\n"
  "SPDX-License-Identifier: MIT\n"
  "Copyright (c) 2013-2019 Niels Lohmann <http://nlohmann.me>.\n"
  "\n"
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

  "\n\n\n"

  "{fmt} version 5.3.0\n"
  "Copyright (c) 2012 - 2016, Victor Zverovich"
  "\n\n"
  "All rights reserved."
  "\n\n"
  "Redistribution and use in source and binary forms, with or without "
  "modification, are permitted provided that the following conditions are met:"
  "\n\n"
  "1. Redistributions of source code must retain the above copyright notice, this "
  "   list of conditions and the following disclaimer.\n"
  "2. Redistributions in binary form must reproduce the above copyright notice, "
  "   this list of conditions and the following disclaimer in the documentation "
  "   and/or other materials provided with the distribution."
  "\n\n"
  "THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS \"AS IS\" AND "
  "ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED "
  "WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE "
  "DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR "
  "ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES "
  "(INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; "
  "LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND "
  "ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT "
  "(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS "
  "SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE."
  "\n";

enum Constants {
  options_indent = 24
};

const rgba_t transparent_color = 0x00000000;

constexpr unsigned palette_size_at_bpp(unsigned bpp) {
  unsigned s = 1;
  for (unsigned i = 0; i < bpp; ++i) s = s << 1;
  return s;
}

constexpr index_t bitmask_at_bpp(unsigned bpp) {
  unsigned m = 1;
  for (unsigned i = 1; i < bpp; ++i) m |= m << 1;
  return (index_t)m;
}

inline rgba_vec_t to_rgba(const channel_vec_t& data) {
  if (data.size() % 4 != 0) throw std::runtime_error("RGBA vector size not a multiple of 4");
  rgba_vec_t v(data.size() >> 2);
  for (unsigned i = 0; i < v.size(); ++i) {
    v[i] = (data[i * 4]) + (data[(i * 4) + 1] << 8) + (data[(i * 4) + 2] << 16) + (data[(i * 4) + 3] << 24);
  }
  return v;
}

inline byte_vec_t to_bytes(const rgba_vec_t& data) {
  byte_vec_t v(data.size() << 2);
  for (unsigned i = 0; i < data.size(); ++i) {
    v[(i << 2) + 0] = data[i] & 0x000000ff;
    v[(i << 2) + 1] = (data[i] & 0x0000ff00) >> 8;
    v[(i << 2) + 2] = (data[i] & 0x00ff0000) >> 16;
    v[(i << 2) + 3] = (data[i] & 0xff000000) >> 24;
  }
  return v;
}

//
// color transformations
//

struct rgba_color final {
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

struct hsva_color final {
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
  float rgb_max = (float)fmax(fmax(rgba.r, rgba.g), rgba.b);
  float rgb_min = (float)fmin(fmin(rgba.r, rgba.g), rgba.b);
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
    rgba.r = (channel_t)c;
    rgba.g = (channel_t)x;
    rgba.b = 0;
  } else if (1.0f <= p && p < 2.0f) {
    rgba.r = (channel_t)x;
    rgba.g = (channel_t)c;
    rgba.b = 0;
  } else if (2.0f <= p && p < 3.0f) {
    rgba.r = 0;
    rgba.g = (channel_t)c;
    rgba.b = (channel_t)x;
  } else if (3.0f <= p && p < 4.0f) {
    rgba.r = 0;
    rgba.g = (channel_t)x;
    rgba.b = (channel_t)c;
  } else if (4.0f <= p && p < 5.0f) {
    rgba.r = (channel_t)x;
    rgba.g = 0;
    rgba.b = (channel_t)c;
  } else if (5.0f <= p && p < 6.0f) {
    rgba.r = (channel_t)c;
    rgba.g = 0;
    rgba.b = (channel_t)x;
  } else {
    rgba.r = 0;
    rgba.g = 0;
    rgba.b = 0;
  }

  rgba.r += (channel_t)m;
  rgba.g += (channel_t)m;
  rgba.b += (channel_t)m;
  rgba.a = (channel_t)(a * 255.0f);
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

inline void sort_colors(rgba_vec_t& colors) {
  std::sort(colors.begin(), colors.end(), [](const rgba_t& a, const rgba_t& b) -> bool {
    return rgba_color(a) > rgba_color(b);
  });
}

// swap bytes between network order and little endian
constexpr rgba_t reverse_bytes(rgba_t v) {
  return ((v >> 24) & 0xff) | ((v << 8) & 0xff0000) | ((v >> 8) & 0xff00) | ((v << 24) & 0xff000000);
}

// scale up value using left bit replication
constexpr channel_t scale_up(channel_t value, unsigned shift) {
  switch (shift) {
  case 7:
    return value ? 0xff : 0;
  case 6:
    return (value << 6) | ((value << 4) & 0x30) | ((value << 2) & 0xc) | (value & 0x3);
  case 5:
    return (value << 5) | ((value << 2) & 0x1c) | ((value >> 1) & 0x3);
  case 4:
    return (value << 4) | (value & 0xf);
  case 3:
    return (value << 3) | ((value >> 2) & 0x7);
  case 2:
    return (value << 2) | ((value >> 4) & 0x3);
  case 1:
    return (value << 1) | ((value >> 6) & 0x1);
  default:
    return (value << shift);
  }
}

// scale standard rgba color to system specific range
inline rgba_t reduce_color(const rgba_t color, Mode to_mode) {
  switch (to_mode) {
  case Mode::snes:
  case Mode::snes_mode7:
  case Mode::gbc:
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
  case Mode::gb:
    // TODO: GB
    // http://problemkaputt.de/pandocs.htm#lcdmonochromepalettes
    // http://problemkaputt.de/pandocs.htm#lcdcolorpalettescgbonly
    return 0;
  case Mode::pce:
  case Mode::pce_sprite:
    if (((color & 0xff000000) >> 24) < 0x80) {
      return transparent_color;
    } else {
      rgba_color c(color);
      c.r >>= 5;
      c.g >>= 5;
      c.b >>= 5;
      rgba_t scaled = c;
      return (scaled & 0x00ffffff) + 0xff000000;
    }
    break;
  default:
    return 0;
  }
}

// scale standard rgba colors to system specific range
inline rgba_vec_t reduce_colors(const rgba_vec_t& colors, Mode to_mode) {
  auto vc = colors;
  for (rgba_t& color : vc) color = reduce_color(color, to_mode);
  return vc;
}

inline rgba_set_t reduce_colors(const rgba_set_t& colors, Mode to_mode) {
  auto sc = rgba_set_t();
  for (const rgba_t& color : colors) sc.insert(reduce_color(color, to_mode));
  return sc;
}

// scale color from system specific range to 8bpc RGBA range
inline rgba_t normalize_color(const rgba_t color, Mode from_mode) {
  rgba_color c(color);
  switch (from_mode) {
  case Mode::snes:
  case Mode::snes_mode7:
  case Mode::gbc:
    c.r = scale_up(c.r, 3);
    c.g = scale_up(c.g, 3);
    c.b = scale_up(c.b, 3);
    c.a = scale_up(c.a, 3);
    return c;
  case Mode::gb:
    // TODO: GB
    return 0;
  case Mode::pce:
  case Mode::pce_sprite:
    c.r = scale_up(c.r, 5);
    c.g = scale_up(c.g, 5);
    c.b = scale_up(c.b, 5);
    c.a = scale_up(c.a, 5);
    return c;
  default:
    return 0;
  }
}

// scale colors from system specific range to 8bpc RGBA range
inline rgba_vec_t normalize_colors(const rgba_vec_t& colors, Mode from_mode) {
  auto vc = colors;
  for (rgba_t& color : vc) color = normalize_color(color, from_mode);
  return vc;
}

// pack scaled rgba color to native format
inline byte_vec_t pack_native_color(const rgba_t color, Mode mode) {
  byte_vec_t v;
  switch (mode) {
  case Mode::snes:
  case Mode::snes_mode7:
  case Mode::gbc:
    v.push_back((color & 0x1f) | ((color >> 3) & 0xe0));
    v.push_back(((color >> 11) & 0x03) | ((color >> 14) & 0x7c));
    break;
  case Mode::gb:
    // TODO: GB
    break;
  case Mode::pce:
  case Mode::pce_sprite:
    v.push_back(((color >> 16) & 0x07) | (color << 3 & 0x38) | ((color >> 2) & 0xc0));
    v.push_back((color >> 10) & 0x01);
    break;
  default:
    break;
  }
  return v;
}

// unpack native format color to (scaled) rgba color
inline rgba_vec_t unpack_native_colors(const byte_vec_t& colors, Mode mode) {
  rgba_vec_t v;
  switch (mode) {
  case Mode::snes:
  case Mode::snes_mode7:
  case Mode::gbc:
    if (colors.size() % 2 != 0) {
      throw std::runtime_error("snes/gbc native palette size not a multiple of 2");
    }
    for (unsigned i = 0; i < colors.size(); i += 2) {
      uint16_t cw = (colors[i + 1] << 8) + colors[i];
      rgba_t nc = (cw & 0x001f) | ((cw & 0x03e0) << 3) | ((cw & 0x7c00) << 6) | 0xff000000;
      v.push_back(nc);
    }
    break;
  case Mode::gb:
    // TODO: GB
    break;
  case Mode::pce:
  case Mode::pce_sprite:
    if (colors.size() % 2 != 0) {
      throw std::runtime_error("pce native palette size not a multiple of 2");
    }
    for (unsigned i = 0; i < colors.size(); i += 2) {
      uint16_t cw = (colors[i + 1] << 8) + colors[i];
      rgba_t nc = ((cw & 0x0038) >> 3) | ((cw & 0x0007) << 8) | ((cw & 0x1c00) << 10) | 0xff000000;
      v.push_back(nc);
    }
    break;
  default:
    break;
  }
  return v;
}

// pack raw image data to native format tile data
inline byte_vec_t pack_native_tile(const index_vec_t& data, Mode mode, unsigned bpp, unsigned width, unsigned height) {

  auto make_2bpp_tile = [](const index_vec_t& in_data, unsigned plane_index) {
    byte_vec_t p(16);
    if (in_data.empty()) return p;

    index_t mask0 = 1;
    for (unsigned i = 0; i < plane_index; ++i) mask0 <<= 1;
    index_t mask1 = mask0 << 1;

    unsigned shift0 = plane_index;
    unsigned shift1 = plane_index + 1;

    for (unsigned y = 0; y < 8; ++y) {
      for (unsigned x = 0; x < 8; ++x) {
        p[y * 2 + 0] |= ((in_data[y * 8 + x] & mask0) >> shift0) << (7 - x);
        p[y * 2 + 1] |= ((in_data[y * 8 + x] & mask1) >> shift1) << (7 - x);
      }
    }
    return p;
  };

  auto make_bitplane_data = [](const index_vec_t& in_data, unsigned plane) {
    if (in_data.size() % 8)
      throw std::runtime_error("programmer error (in_data not multiple of 8 in make_bitplane_data())");

    size_t plane_size = in_data.size() >> 3;
    byte_vec_t p(plane_size);

    index_t mask = 1;
    for (unsigned i = 0; i < plane; ++i) mask <<= 1;

    for (unsigned index_b = 0, index_i = 0; index_b < plane_size; ++index_b) {
      index_t byte = 0;
      for (unsigned b = 0; b < 8; ++b) {
        if (in_data[index_i + b] & mask) byte |= 1 << b;
      }
      p[index_b] = byte;
      index_i += 8;
    }
    return p;
  };

  byte_vec_t nd;

  if (mode == Mode::snes || mode == Mode::gb || mode == Mode::gbc || mode == Mode::pce) {
    if (width != 8 || height != 8)
      throw std::runtime_error(fmt::format("programmer error (tile size not 8x8 in pack_native_tile() for mode \"{}\")", sfc::mode(mode)));

    unsigned planes = bpp >> 1;
    for (unsigned i = 0; i < planes; ++i) {
      auto plane = make_2bpp_tile(data, i * 2);
      nd.insert(nd.end(), plane.begin(), plane.end());
    }

  } else if (mode == Mode::snes_mode7) {
    nd = data;

  } else if (mode == Mode::pce_sprite) {
    for (unsigned p = 0; p < 4; ++p) {
      auto plane = make_bitplane_data(data, p);
      nd.insert(nd.end(), plane.begin(), plane.end());
    }
  }

  return nd;
}

// unpack native format tile data to raw image data
inline index_vec_t unpack_native_tile(const byte_vec_t& data, Mode mode, unsigned bpp, unsigned width, unsigned height) {

  auto add_1bit_plane = [](
    index_vec_t& out_data, const byte_vec_t& in_data, unsigned plane_index) {
    int plane_offset = ((plane_index >> 1) * 16) + (plane_index & 1);
    for (int y = 0; y < 8; ++y) {
      for (int x = 0; x < 8; ++x) {
        out_data[y * 8 + x] += ((in_data[plane_offset + (y * 2)] >> (7 - x)) & 1) << plane_index;
      }
    }
  };

  index_vec_t ud(width * height);

  if (mode == Mode::snes || mode == Mode::gb || mode == Mode::gbc || mode == Mode::pce) {
    for (unsigned i = 0; i < bpp; ++i) add_1bit_plane(ud, data, i);
  } else if (mode == Mode::snes_mode7) {
    ud = data;
  } else if (mode == Mode::pce_sprite) {
    // TODO: read pce_sprite data
    throw std::runtime_error("Using pce_sprite native data as input not implemented");
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
  return fmt::format("{1}{0:0{2}x}", reverse_bytes(value) >> (alpha ? 0 : 8), pound ? "#" : "", alpha ? 8 : 6);
}

// css style hex string to rgba value
inline rgba_t from_hexstring(const std::string& str) {
  std::string s = str;
  if (s.at(0) == '#') s.erase(0, 1);
  if (s.size() == 6) s.insert(6, 2, 'f');
  if (s.size() != 8) throw std::runtime_error("Argument color-zero not a 6 or 8 character hex-string");
  uint32_t i;
  if (sscanf(s.c_str(), "%x", &i) == 1) {
    return reverse_bytes(i);
  } else {
    throw std::runtime_error("Failed to interpret argument color-zero");
  };
}

//
// utility i/o functions
//

// parse json at path
inline nlohmann::json read_json_file(const std::string& path) {
  nlohmann::json j;
  std::ifstream f(path);
  if (f.fail()) {
    throw std::runtime_error(fmt::format("File \"{}\" could not be opened", path));
  }
  f >> j;
  return j;
}

// read text file at path
inline std::string read_file(const std::string& path) {
  std::ifstream ifs(path);
  if (ifs.fail()) {
    throw std::runtime_error(fmt::format("File \"{}\" could not be opened", path));
  }
  return std::string((std::istreambuf_iterator<char>(ifs)), (std::istreambuf_iterator<char>()));
}

// read binary file at path
inline byte_vec_t read_binary(const std::string& path) {
  std::ifstream ifs(path, std::ios::binary);
  if (ifs.fail()) {
    throw std::runtime_error(fmt::format("File \"{}\" could not be opened", path));
  }
  return byte_vec_t((std::istreambuf_iterator<char>(ifs)), (std::istreambuf_iterator<char>()));
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

constexpr int div_ceil(int numerator, int denominator) {
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

template <typename T>
T vec_pop(std::vector<T>& v) {
  if (!v.size()) throw std::range_error("vector empty");
  T e = v.back();
  v.pop_back();
  return e;
}

template <typename T>
bool is_subset(const std::set<T>& set, const std::set<T>& superset) {
  return std::includes(superset.begin(), superset.end(), set.begin(), set.end());
}

template <typename T>
bool has_superset(const std::set<T>& set, const std::vector<std::set<T>>& super) {
  for (auto& cmp_set : super) {
    if (cmp_set == set) continue;
    if (is_subset(set, cmp_set)) return true;
  }
  return false;
}

} /* namespace sfc */
