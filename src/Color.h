// color types and functions
//
// david lindecrantz <optiroc@me.com>

#pragma once

#include "Common.h"

namespace sfc {

const rgba_t transparent_color = 0x00000000;

// vector<channel_t> to vector<rgba_t>
inline rgba_vec_t to_rgba(const channel_vec_t& data) {
  if (data.size() % 4 != 0)
    throw std::runtime_error("RGBA vector size not a multiple of 4");
  rgba_vec_t v(data.size() >> 2);
  for (unsigned i = 0; i < v.size(); ++i) {
    v[i] = (data[i * 4]) + (data[(i * 4) + 1] << 8) + (data[(i * 4) + 2] << 16) + (data[(i * 4) + 3] << 24);
  }
  return v;
}

// swap bytes between network order and little endian
constexpr rgba_t reverse_bytes(rgba_t v) {
  return ((v >> 24) & 0xff) | ((v << 8) & 0xff0000) | ((v >> 8) & 0xff00) | ((v << 24) & 0xff000000);
}

// rgba value to css style hex string
inline std::string to_hexstring(rgba_t value, bool pound = true, bool alpha = false) {
  return fmt::format("{1}{0:0{2}x}", reverse_bytes(value) >> (alpha ? 0 : 8), pound ? "#" : "", alpha ? 8 : 6);
}

// css style hex string to rgba value
inline rgba_t from_hexstring(const std::string& str) {
  std::string s = str;
  s.erase(std::remove(s.begin(), s.end(), '#'), s.end());
  s.erase(std::remove(s.begin(), s.end(), '"'), s.end());
  s.erase(std::remove(s.begin(), s.end(), '\''), s.end());

  if (s.size() == 6)
    s.insert(6, 2, 'f');
  if (s.size() != 8)
    throw std::runtime_error("Argument color-zero not a 6 or 8 character hex-string");

  uint32_t i;
  if (sscanf(s.c_str(), "%x", &i) == 1) {
    return reverse_bytes(i);
  } else {
    throw std::runtime_error("Failed to interpret color string");
  };
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

//
// rgba_color / hsva_color
//

struct rgba_color final {
  channel_t r = {};
  channel_t g = {};
  channel_t b = {};
  channel_t a = {};

  rgba_color(){};
  rgba_color(rgba_t c) : r(c & 0x000000ff), g((c & 0x0000ff00) >> 8), b((c & 0x00ff0000) >> 16), a((c & 0xff000000) >> 24) {
    if (a < 0x80)
      r = g = b = a = 0;
  };

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
  float h = {};
  float s = {};
  float v = {};
  float a = {};

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

  if (h < 0)
    h = 360 + h;
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

// aesthetically pleasing color sorting
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
  std::sort(colors.begin(), colors.end(), [](const rgba_t& a, const rgba_t& b) -> bool { return rgba_color(a) > rgba_color(b); });
}

} /* namespace sfc */
