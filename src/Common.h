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

#include <fmt/core.h>
#include <nlohmann/json.hpp>

typedef uint8_t index_t;   // color index (typedefd in case more than 8 bits are needed down the road)
typedef uint8_t channel_t; // rgba color channel
typedef uint32_t rgba_t;   // rgba color stored in little endian order

typedef std::vector<uint8_t> byte_vec_t;
typedef std::vector<index_t> index_vec_t;
typedef std::vector<channel_t> channel_vec_t;
typedef std::vector<rgba_t> rgba_vec_t;
typedef std::set<rgba_t> rgba_set_t;
typedef std::vector<rgba_set_t> rgba_set_vec_t;

#include "Color.h"
#include "Mode.h"

namespace sfc {

constexpr const char* VERSION = "0.9.2";

constexpr const char* COPYRIGHT = "Copyright (c) 2017-2020 David Lindecrantz";

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

  "LodePNG version 20200306"
  "\n"
  "Copyright (c) 2005-2020 Lode Vandevennen"
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
  "|  |  |__   |  |  | | | |  version 3.7.3\n"
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

  "{fmt} version 6.1.2\n"
  "Copyright (c) 2012 - present, Victor Zverovich"
  "\n\n"
  "Permission is hereby granted, free of charge, to any person obtaining "
  "a copy of this software and associated documentation files (the "
  "\"Software\"), to deal in the Software without restriction, including "
  "without limitation the rights to use, copy, modify, merge, publish, "
  "distribute, sublicense, and/or sell copies of the Software, and to "
  "permit persons to whom the Software is furnished to do so, subject to "
  "the following conditions:"
  "\n\n"
  "The above copyright notice and this permission notice shall be "
  "included in all copies or substantial portions of the Software."
  "\n\n"
  "THE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, "
  "EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF "
  "MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND "
  "NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE "
  "LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION "
  "OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION "
  "WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE."
  "\n\n"
  "--- Optional exception to the license ---"
  "\n\n"
  "As an exception, if, as a result of your compiling your source code, portions "
  "of this Software are embedded into a machine-executable object form of such "
  "source code, you may redistribute such embedded portions in such object form "
  "without including the above copyright and permission notices."
  "\n";

enum Constants {
  options_indent = 24,
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
