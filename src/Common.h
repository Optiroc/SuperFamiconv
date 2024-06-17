// common types and utility functions
//
// david lindecrantz <optiroc@me.com>

#pragma once

#include <cstdint>
#include <fstream>
#include <set>
#include <nlohmann/json.hpp>
#include <fmt/core.h>

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

enum Constants {
  options_indent = 28,
};

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

#ifndef M_PI
#define M_PI 3.14159265358979323846
#endif

constexpr double rad2deg(const double rad) {
  return rad * (180.0 / M_PI);
}

constexpr double deg2rad(const double deg) {
  return deg * (M_PI / 180.0);
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
  if (!v.size())
    throw std::range_error("vector empty");
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
    if (cmp_set == set)
      continue;
    if (is_subset(set, cmp_set))
      return true;
  }
  return false;
}

} /* namespace sfc */
