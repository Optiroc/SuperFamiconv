// color types and functions
//
// david lindecrantz <optiroc@gmail.com>

#pragma once

#include <cmath>

namespace sfc {

typedef uint8_t index_t;   // color index (typedefd in case more than 8 bits are needed down the road)
typedef uint8_t channel_t; // rgba color channel
typedef uint32_t rgba_u32; // rgba color stored in little endian order

typedef std::vector<index_t> index_vec;
typedef std::vector<channel_t> channel_vec;
typedef std::vector<rgba_u32> rgba_u32_vec;
typedef std::set<rgba_u32> rgba_u32_set;
typedef std::vector<rgba_u32_set> rgba_u32_set_vec;

const rgba_u32 transparent_color = 0x00000000;

//
// color utility functions
//

constexpr unsigned palette_size_at_bpp(unsigned bpp) {
  unsigned s = 1;
  for (unsigned i = 0; i < bpp; ++i)
    s = s << 1;
  return s;
}

constexpr index_t bitmask_at_bpp(unsigned bpp) {
  unsigned m = 1;
  for (unsigned i = 1; i < bpp; ++i)
    m |= m << 1;
  return (index_t)m;
}

inline byte_vec to_bytes(const rgba_u32_vec& colors) {
  byte_vec v(colors.size() << 2);
  for (unsigned i = 0; i < colors.size(); ++i) {
    v[(i << 2) + 0] = colors[i] & 0x000000ff;
    v[(i << 2) + 1] = (colors[i] & 0x0000ff00) >> 8;
    v[(i << 2) + 2] = (colors[i] & 0x00ff0000) >> 16;
    v[(i << 2) + 3] = (colors[i] & 0xff000000) >> 24;
  }
  return v;
}

// swap bytes between network order and little endian
constexpr rgba_u32 reverse_bytes(rgba_u32 v) {
  return ((v >> 24) & 0xff) | ((v << 8) & 0xff0000) | ((v >> 8) & 0xff00) | ((v << 24) & 0xff000000);
}

// rgba value to css style hex string
inline std::string to_hexstring(rgba_u32 value, bool pound = true, bool alpha = false) {
  return fmt::format("{1}{0:0{2}x}", reverse_bytes(value) >> (alpha ? 0 : 8), pound ? "#" : "", alpha ? 8 : 6);
}

// css style hex string to rgba value
inline rgba_u32 from_hexstring(const std::string& str) {
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

constexpr channel_t channel_clamp(double value) {
  return value > 255 ? 255 : (value < 0 ? 0 : value);
}

//
// rgba_color / hsva_color / lab_color types
// conversions based on https://www.easyrgb.com/en/math.php
// deltaE 2000 from http://www.brucelindbloom.com/javascript/ColorDiff.js
//

struct hsva_color;
struct lab_color;

// rgba components in range 0..255
struct rgba_color final {
  channel_t r = {};
  channel_t g = {};
  channel_t b = {};
  channel_t a = {};

  rgba_color(){};
  rgba_color(unsigned r, unsigned g, unsigned b, unsigned a)
  : r(r), g(g), b(b), a(a){};

  rgba_color(rgba_u32 c)
  : r(c & 0x000000ff), g((c & 0x0000ff00) >> 8), b((c & 0x00ff0000) >> 16), a((c & 0xff000000) >> 24) {
    if (a < 0x80) r = g = b = a = 0;
  };

  operator rgba_u32() const { return (a << 24) + (b << 16) + (g << 8) + r; }
  operator hsva_color() const;
  operator lab_color() const;

  bool operator>(const rgba_color& other) const;
  bool operator==(const rgba_color& other) const {
    return r == other.r && g == other.g && b == other.b && a == other.a;
  }

  rgba_color add(const rgba_color& c) const {
    return rgba_color{
      channel_clamp(r + c.r), channel_clamp(g + c.g), channel_clamp(b + c.b), a
    };
  }

  rgba_color add(const double f) const {
    return rgba_color{
      channel_clamp(r + f), channel_clamp(g + f), channel_clamp(b + f), a
    };
  }
};

inline bool r_cmp(const rgba_color& a, const rgba_color& b) {
  return a.r < b.r;
}

inline bool g_cmp(const rgba_color& a, const rgba_color& b) {
  return a.g < b.g;
}

inline bool b_cmp(const rgba_color& a, const rgba_color& b) {
  return a.b < b.b;
}

inline bool a_cmp(const rgba_color& a, const rgba_color& b) {
  return a.a < b.a;
}

// vector<rgba_t> to vector<rgba_color>
inline std::vector<rgba_color> to_rgba_color_vec(const rgba_u32_vec& colors) {
  std::vector<rgba_color> v(colors.size());
  for (unsigned i = 0; i < v.size(); ++i) v[i] = colors[i];
  return v;
}

// vector<rgba_color> to vector<rgba_t>
inline rgba_u32_vec to_rgba_u32_vec(const std::vector<rgba_color>& colors) {
  rgba_u32_vec v(colors.size());
  for (unsigned i = 0; i < v.size(); ++i) v[i] = colors[i];
  return v;
}

// vector<channel_t> to vector<rgba_u32>
inline rgba_u32_vec to_rgba(const channel_vec& data) {
  if (data.size() % 4 != 0)
    throw std::runtime_error("RGBA vector size not a multiple of 4");
  rgba_u32_vec v(data.size() >> 2);
  for (unsigned i = 0; i < v.size(); ++i) {
    v[i] = (data[i * 4]) + (data[(i * 4) + 1] << 8) + (data[(i * 4) + 2] << 16) + (data[(i * 4) + 3] << 24);
  }
  return v;
}

// hsva components in range 0..1
struct hsva_color final {
  double h = {};
  double s = {};
  double v = {};
  double a = {};

  hsva_color(double h = 0.0, double s = 0.0, double v = 0.0, double a = 0.0)
  : h(h), s(s), v(v), a(a){};

  operator rgba_color() const;
};

// L*ab components
struct lab_color final {
  double L = {};
  double a = {};
  double b = {};
  double A = {};

  lab_color(double L = 0, double a = 0.0, double b = 0.0, double A = 0.0)
  : L(L), a(a), b(b), A(A){};

  operator rgba_color() const;
};

// rgba to hsva
inline rgba_color::operator hsva_color() const {
  hsva_color hsva;
  hsva.a = a / 255.0;
  double _r = r / 255.0;
  double _g = g / 255.0;
  double _b = b / 255.0;

  double max = fmax(fmax(_r, _g), _b);
  double min = fmin(fmin(_r, _g), _b);
  double delta = max - min;

  if (delta > 0) {
    if (max == _r) {
      hsva.h = 60.0 * (fmod(((_g - _b) / delta), 6.0));
    } else if (max == _g) {
      hsva.h = 60.0 * (((_b - _r) / delta) + 2.0);
    } else if (max == _b) {
      hsva.h = 60.0 * (((_r - _g) / delta) + 4.0);
    }
    hsva.s = (max > 0.0) ? delta / max : 0.0;
    hsva.v = max;
  } else {
    hsva.h = 0.0;
    hsva.s = 0.0;
    hsva.v = max;
  }

  if (hsva.h < 0.0) hsva.h += 360.0;
  return hsva;
}

// hsva to rgba
inline hsva_color::operator rgba_color() const {
  double r, g, b;
  double c = v * s;
  double p = fmod(h / 60.0, 6.0);
  double x = c * (1.0 - fabs(fmod(p, 2.0) - 1.0));
  double m = v - c;

  if (0.0 <= p && p < 1.0) {
    r = c; g = x; b = 0.0;
  } else if (1.0 <= p && p < 2.0) {
    r = x; g = c; b = 0.0;
  } else if (2.0 <= p && p < 3.0) {
    r = 0.0; g = c; b = x;
  } else if (3.0 <= p && p < 4.0) {
    r = 0.0; g = x; b = c;
  } else if (4.0 <= p && p < 5.0) {
    r = x; g = 0.0; b = c;
  } else if (5.0 <= p && p < 6.0) {
    r = c; g = 0.0; b = x;
  } else {
    r = 0.0; g = 0.0; b = 0.0;
  }

  return rgba_color((r + m) * 255.0, (g + m) * 255.0, (b + m) * 255.0, a * 255.0);
}

// rgba to lab
inline rgba_color::operator lab_color() const {
  double _r = r / 255.0;
  double _g = g / 255.0;
  double _b = b / 255.0;

  _r = _r > 0.04045 ? pow((_r + 0.055) / 1.055, 2.4) : _r / 12.92;
  _g = _g > 0.04045 ? pow((_g + 0.055) / 1.055, 2.4) : _g / 12.92;
  _b = _b > 0.04045 ? pow((_b + 0.055) / 1.055, 2.4) : _b / 12.92;

  double x = (_r * 0.412453 + _g * 0.357580 + _b * 0.180423) / 0.95047;
  double y = (_r * 0.212671 + _g * 0.715160 + _b * 0.072169) / 1.00000;
  double z = (_r * 0.019334 + _g * 0.119193 + _b * 0.950227) / 1.08883;

  x = x > 0.008856 ? pow(x, 1.0 / 3.0) : (7.787 * x) + 16.0 / 116.0;
  y = y > 0.008856 ? pow(y, 1.0 / 3.0) : (7.787 * y) + 16.0 / 116.0;
  z = z > 0.008856 ? pow(z, 1.0 / 3.0) : (7.787 * z) + 16.0 / 116.0;

  return lab_color((116.0 * y) - 16.0, 500.0 * (x - y), 200.0 * (y - z), a);
}

// lab to rgba
inline lab_color::operator rgba_color() const {
  double y = (L + 16.0) / 116.0;
  double x = (a / 500.0) + y;
  double z = y - (b / 200.0);

  x = 0.95047 * ((x * x * x > 0.008856) ? x * x * x : (x - 16.0 / 116.0) / 7.787);
  y = 1.00000 * ((y * y * y > 0.008856) ? y * y * y : (y - 16.0 / 116.0) / 7.787);
  z = 1.08883 * ((z * z * z > 0.008856) ? z * z * z : (z - 16.0 / 116.0) / 7.787);

  double r = x *  3.2406 + y * -1.5372 + z * -0.4986;
  double g = x * -0.9689 + y *  1.8758 + z *  0.0415;
  double b = x *  0.0557 + y * -0.2040 + z *  1.0570;

  r = r > 0.0031308 ? (1.055 * pow(r, 1.0 / 2.4)) - 0.055 : r * 12.92;
  g = g > 0.0031308 ? (1.055 * pow(g, 1.0 / 2.4)) - 0.055 : g * 12.92;
  b = b > 0.0031308 ? (1.055 * pow(b, 1.0 / 2.4)) - 0.055 : b * 12.92;

  return rgba_color(fmax(0.0, fmin(1.0, r)) * 255.0, fmax(0.0, fmin(1.0, g)) * 255.0, fmax(0.0, fmin(1.0, b)) * 255.0, A);
}

// CIE DeltaE 76 color difference
inline double cie_de76(const lab_color& lab_a, const lab_color& lab_b) {
  return sqrt(pow(lab_a.L - lab_b.L, 2.0) + pow(lab_a.a - lab_b.a, 2) + pow(lab_a.b - lab_b.b, 2));
}

inline const rgba_color closest_de76(const rgba_color& c, const std::vector<rgba_color>& palette) {
  return *std::min_element(palette.begin(), palette.end(), [c](const rgba_color& a, const rgba_color& b) {
    return cie_de76(c, a) < cie_de76(c, b);
  });
}

// CIE DeltaE 2000 color difference
inline double cie_de2000(const lab_color& lab_a, const lab_color& lab_b) {
  const double kL = 1.0;
  const double kC = 1.0;
  const double kH = 1.0;

  const double mean_L = 0.5 * (lab_a.L + lab_b.L);
  const double c1 = sqrt(lab_a.a * lab_a.a + lab_a.b * lab_a.b);
  const double c2 = sqrt(lab_b.a * lab_b.a + lab_b.b * lab_b.b);
  const double mean_c = 0.5 * (c1 + c2);
  const double g = 0.5 * (1.0 - sqrt(pow(mean_c, 7.0) / (pow(mean_c, 7.0) + pow(25.0, 7.0))));

  const double a1p = lab_a.a * (1.0 + g);
  const double a2p = lab_b.a * (1.0 + g);
  const double c1p = sqrt(a1p * a1p + lab_a.b * lab_a.b);
  const double c2p = sqrt(a2p * a2p + lab_b.b * lab_b.b);

  const double mean_cp = 0.5 * (c1p + c2p);

  double h1p = (atan2(lab_a.b, a1p) * 180.0) / M_PI;
  if (h1p < 0.0) h1p += 360.0;
  double h2p = (atan2(lab_b.b, a2p) * 180.0) / M_PI;
  if (h2p < 0.0) h2p += 360.0;
  const double mean_hp = abs(h1p - h2p) > 180.0 ? 0.5 * (h1p + h2p + 360.0) : 0.5 * (h1p + h2p);

  double T = 1.0 - 0.17 * cos(M_PI * (mean_hp - 30.0) / 180.0) + 0.24 * cos(M_PI * (2.0 * mean_hp) / 180.0) +
             0.32 * cos(M_PI * (3.0 * mean_hp + 6.0) / 180.0) - 0.20 * cos(M_PI * (4.0 * mean_hp - 63.0) / 180.0);

  double dhp;
  if (abs(h2p - h1p) <= 180.0) dhp = h2p - h1p;
  else dhp = (h2p <= h1p) ? (h2p - h1p + 360.0) : (h2p - h1p - 360.0);

  double dL = lab_b.L - lab_a.L;
  double dC = c2p - c1p;
  double dH = 2.0 * sqrt(c1p * c2p) * sin(M_PI * (0.5 * dhp) / 180.0);
  double dT = 30.0 * exp(-((mean_hp - 275.0) / 25.0) * ((mean_hp - 275.0) / 25.0));

  double sL = 1.0 + ((0.015 * (mean_L - 50.0) * (mean_L - 50.0)) / sqrt(20.0 + (mean_L - 50.0) * (mean_L - 50.0)));
  double sC = 1.0 + 0.045 * mean_cp;
  double sH = 1.0 + 0.015 * mean_cp * T;

  double rC = sqrt(pow(mean_cp, 7.0) / (pow(mean_cp, 7.0) + pow(25.0, 7.0)));
  double rT = -2.0 * rC * sin(M_PI * (2.0 * dT) / 180.0);

  return sqrt((dL / (kL * sL)) * (dL / (kL * sL)) + (dC / (kC * sC)) * (dC / (kC * sC)) +
              (dH / (kH * sH)) * (dH / (kH * sH)) + (dC / (kC * sC)) * (dH / (kH * sH)) * rT);
}

inline const rgba_color closest_de2000(const rgba_color& c, const std::vector<rgba_color>& palette) {
  return *std::min_element(palette.begin(), palette.end(), [c](const rgba_color& a, const rgba_color& b) {
    return cie_de2000(c, a) < cie_de2000(c, b);
  });
}

/*
struct closest_de2000_memoizer final {
  closest_de2000_memoizer(const std::vector<rgba_color>& palette)
  : _palette(palette){};

private:
  std::unordered_map<rgba_u32, rgba_u32> _cache;
  const std::vector<rgba_color>& _palette;
};
*/

// aesthetically pleasing color sorting
inline bool rgba_color::operator>(const rgba_color& o) const {
  const int segments = 8;
  const hsva_color hsva = *this;
  const hsva_color hsva_o = o;

  int h = (int)(segments * hsva.h);
  int l = (int)(segments * sqrt(0.241f * r + 0.691f * g + 0.068f * b));
  int v = (int)(segments * hsva.v);

  int ho = (int)(segments * hsva_o.h);
  int lo = (int)(segments * sqrt(0.241f * o.r + 0.691f * o.g + 0.068f * o.b));
  int vo = (int)(segments * hsva_o.v);

  return std::tie(h, l, v) > std::tie(ho, lo, vo);
}

inline void sort_colors(rgba_u32_vec& colors) {
  std::sort(colors.begin(), colors.end(), [](const rgba_u32& a, const rgba_u32& b) -> bool {
    return (rgba_color)(a) > (rgba_color)(b);
  });
}

/*
inline double color_diff(const rgba_color& a, const rgba_color& b) {
  return std::fabs(std::sqrt(std::pow(b.r - a.r, 2) + std::pow(b.g - a.g, 2) + std::pow(b.b - a.b, 2) + std::pow(b.a - a.a, 2)));
}
*/

/*
// get average palette step value
inline double palette_step(const std::vector<rgba_color>& palette) {
  fmt::print("PS: {}\n", palette.size());

  auto p = std::vector<lab_color>();
  for (const auto& c : palette) p.push_back(c);
  std::sort(p.begin(), p.end(), [](const lab_color& a, const lab_color& b) -> bool {
    return a.L > b.L;
  });

  fmt::print("F: {}\n", p.front().L);
  fmt::print("B: {}\n", p.back().L);
  return ((p.front().L - p.back().L) * (255.0 / 100.0)) / palette.size();

  //auto p = std::vector<hsva_color>();
  //for (const auto& c : palette) p.push_back(c);
  //std::sort(p.begin(), p.end(), [](const hsva_color& a, const hsva_color& b) -> bool {
  //  return a.v > b.v;
  //});
  //return ((p.front().v - p.back().v) * 255.0) / palette.size();
}
*/

//
// median cut color quantization
// https://indiegamedev.net/2020/01/17/median-cut-with-floyd-steinberg-dithering-in-c/
//

// TODO: exclude alpha option

inline void mediancut_sort(std::vector<rgba_color>& box, channel_t& rr, channel_t& gr, channel_t& br, channel_t& ar) {
  rr = (*std::max_element(box.begin(), box.end(), r_cmp)).r - (*std::min_element(box.begin(), box.end(), r_cmp)).r;
  gr = (*std::max_element(box.begin(), box.end(), g_cmp)).g - (*std::min_element(box.begin(), box.end(), g_cmp)).g;
  br = (*std::max_element(box.begin(), box.end(), b_cmp)).b - (*std::min_element(box.begin(), box.end(), b_cmp)).b;
  ar = (*std::max_element(box.begin(), box.end(), a_cmp)).a - (*std::min_element(box.begin(), box.end(), a_cmp)).a;

  if (rr >= gr && rr >= br && rr >= ar) {
    std::sort(box.begin(), box.end(), r_cmp);
  } else if (gr >= rr && gr >= br && gr >= ar) {
    std::sort(box.begin(), box.end(), g_cmp);
  } else if (br >= rr && br >= gr && br >= ar) {
    std::sort(box.begin(), box.end(), b_cmp);
  } else {
    std::sort(box.begin(), box.end(), a_cmp);
  }
}

inline std::vector<rgba_color> mediancut_palette(const std::vector<rgba_color>& source, const unsigned num_colors) {
  typedef std::vector<rgba_color> Box;
  typedef std::pair<channel_t, Box> RangeBox;

  std::vector<RangeBox> boxes;
  Box init = source;
  boxes.push_back(RangeBox(0, init));

  while (boxes.size() < num_colors) {
    // for each box, sort the boxes pixels according to the color it has the most range in
    for (RangeBox& box_data : boxes) {
      channel_t r_range;
      channel_t g_range;
      channel_t b_range;
      channel_t a_range;
      if (std::get<0>(box_data) == 0) {
        mediancut_sort(std::get<1>(box_data), r_range, g_range, b_range, a_range);
        if (r_range >= g_range && r_range >= b_range && r_range >= a_range) {
          std::get<0>(box_data) = r_range;
        } else if (g_range >= r_range && g_range >= b_range && g_range >= a_range) {
          std::get<0>(box_data) = g_range;
        } else if (b_range >= r_range && b_range >= g_range && b_range >= a_range) {
          std::get<0>(box_data) = b_range;
        } else {
          std::get<0>(box_data) = a_range;
        }
      }
    }

    std::sort(boxes.begin(), boxes.end(), [](const RangeBox& a, const RangeBox& b) { return std::get<0>(a) < std::get<0>(b); });

    std::vector<RangeBox>::iterator itr = std::prev(boxes.end());
    Box biggest_box = std::get<1>(*itr);
    boxes.erase(itr);

    // the box is sorted already, so split at median
    Box split_a(biggest_box.begin(), biggest_box.begin() + biggest_box.size() / 2);
    Box split_b(biggest_box.begin() + biggest_box.size() / 2, biggest_box.end());

    boxes.push_back(RangeBox(0, split_a));
    boxes.push_back(RangeBox(0, split_b));
  }

  // average each box to determine the color
  std::vector<rgba_color> palette;
  for (const RangeBox& box_data : boxes) {
    Box box = std::get<1>(box_data);
    unsigned r_acc = 0, g_acc = 0, b_acc = 0, a_acc = 0;
    std::for_each(box.begin(), box.end(), [&](const rgba_color& c) {
      r_acc += c.r;
      g_acc += c.g;
      b_acc += c.b;
      a_acc += c.a;
    });
    r_acc /= box.size();
    g_acc /= box.size();
    b_acc /= box.size();
    a_acc /= box.size();

    palette.push_back(rgba_color(std::min(r_acc, 255u), std::min(g_acc, 255u), std::min(b_acc, 255u), std::min(a_acc, 255u)));
  }
  return palette;
}

//
// ordered dithering
//

namespace dither {

enum class Mode {
  none,
  checker,
  bayer2x2,
  bayer4x4,
  bayer8x8,
  stippleV,
  stippleH,
  lineV,
  lineH
};

inline Mode mode(const std::string& str) {
  if (str == "checkerboard" || str == "checkered" || str == "checker") {
    return Mode::checker;
  } else if (str == "bayer" || str == "bayer2" || str == "bayer2x2") {
    return Mode::bayer2x2;
  } else if (str == "bayer4" || str == "bayer4x4") {
    return Mode::bayer4x4;
  } else if (str == "bayer8" || str == "bayer8x8") {
    return Mode::bayer8x8;
  } else if (str == "stippleV" || str == "stipplev" || str == "stipple_v") {
    return Mode::stippleV;
  } else if (str == "stippleH" || str == "stippleh" || str == "stipple_h") {
    return Mode::stippleH;
  } else if (str == "lineV" || str == "linev" || str == "line_v") {
    return Mode::lineV;
  } else if (str == "lineH" || str == "lineh" || str == "line_h") {
    return Mode::lineH;
  } else
  return Mode::none;
}

inline std::string mode(Mode mode) {
  switch (mode) {
  case Mode::checker:
    return std::string("checkerboard");
  case Mode::bayer2x2:
    return std::string("bayer2x2");
  case Mode::bayer4x4:
    return std::string("bayer4x4");
  case Mode::bayer8x8:
    return std::string("bayer8x8");
  case Mode::stippleV:
    return std::string("stippleV");
  case Mode::stippleH:
    return std::string("stippleH");
  case Mode::lineV:
    return std::string("lineV");
  case Mode::lineH:
    return std::string("lineH");
  case Mode::none:
    return std::string("none");
  }
}

inline std::string descripton(Mode mode) {
  switch (mode) {
  case Mode::checker:
    return std::string("checkerboard dithering");
  case Mode::bayer2x2:
    return std::string("2x2 bayer dithering");
  case Mode::bayer4x4:
    return std::string("4x4 bayer dithering");
  case Mode::bayer8x8:
    return std::string("8x8 bayer dithering");
  case Mode::stippleV:
    return std::string("vertical stippled dithering");
  case Mode::stippleH:
    return std::string("horizontal stippled dithering");
  case Mode::lineV:
    return std::string("vertical line dithering");
  case Mode::lineH:
    return std::string("horizontal line dithering");
  case Mode::none:
    return std::string("no dithering");
  }
}

typedef struct Matrix {
    unsigned width, height;
    double range, offset;
    double map[64];
} Matrix;

constexpr const Matrix mtx_checker = {
  2, 2,
  1.5, 0.0,
  {
    -0.25,  0.25,
     0.25, -0.25
  }
};

constexpr const Matrix mtx_bayer2x2 = {
  2, 2,
  1.0, 0.0,
  {
    -0.25, 0.25,
     0.5,  0.0
  }
};

constexpr const Matrix mtx_bayer4x4 = {
  4, 4,
  1.0, 0.0,
  {
    -0.4375,  0.0625, -0.3125,  0.1875,
     0.3125, -0.1875,  0.4375, -0.0625,
    -0.25,    0.25,   -0.375,   0.125,
     0.5,     0.0,     0.375,  -0.125
  }
};

constexpr const Matrix mtx_bayer8x8 = {
  8, 8,
  1.0, 0.0,
  {
    -0.484375,  0.265625, -0.296875,  0.453125, -0.4375,   0.3125,  -0.25,     0.5,
     0.015625, -0.234375,  0.203125, -0.046875,  0.0625,  -0.1875,   0.25,     0.0,
    -0.359375,  0.390625, -0.421875,  0.328125, -0.3125,   0.4375,  -0.375,    0.375,
     0.140625, -0.109375,  0.078125, -0.171875,  0.1875,  -0.0625,   0.125,   -0.125,
    -0.453125,  0.296875, -0.265625,  0.484375, -0.46875,  0.28125, -0.28125,  0.46875,
     0.046875, -0.203125,  0.234375, -0.015625,  0.03125, -0.21875,  0.21875, -0.03125,
    -0.328125,  0.421875, -0.390625,  0.359375, -0.34375,  0.40625, -0.40625,  0.34375,
     0.171875, -0.078125,  0.109375, -0.140625,  0.15625, -0.09375,  0.09375, -0.15625
  }
};

constexpr const Matrix mtx_stippleV = {
  4, 2,
  1.0, 0.0,
  {
    -0.375,  0.125, -0.25, 0.25,
     0.375, -0.125,  0.5,  0.0
  }
};

constexpr const Matrix mtx_stippleH = {
  2, 4,
  1.0, 0.0,
  {
    -0.375,  0.125,
     0.375, -0.125,
    -0.25,   0.25,
     0.5,    0.0
  }
};

constexpr const Matrix mtx_lineV = {
  2, 4,
  1.0, 0.0,
  {
    -0.375, 0.5,
    -0.125, 0.375,
    -0.25,  0.5,
     0.0,   0.5
  }
};

constexpr const Matrix mtx_lineH = {
  4, 2,
  1.0, 0.0,
  {
    -0.375, -0.125, -0.25, 0.0,
     0.5,    0.375,  0.5,  0.5
  }
};

constexpr const Matrix mtx_none = {
  1, 1,
  1.0, 0.0,
  { 0 }
};

inline const Matrix& matrix(Mode mode) {
  switch (mode) {
  case Mode::checker:
    return mtx_checker;
  case Mode::bayer2x2:
    return mtx_bayer2x2;
  case Mode::bayer4x4:
    return mtx_bayer4x4;
  case Mode::bayer8x8:
    return mtx_bayer8x8;
  case Mode::stippleV:
    return mtx_stippleV;
  case Mode::stippleH:
    return mtx_stippleH;
  case Mode::lineV:
    return mtx_lineV;
  case Mode::lineH:
    return mtx_lineH;
  case Mode::none:
    return mtx_none;
  }
}

} /* namespace dither */

} /* namespace sfc */
