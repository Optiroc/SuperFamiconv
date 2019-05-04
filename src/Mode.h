// mode specific defaults and constraints
//
// david lindecrantz <optiroc@gmail.com>

#pragma once

#include "Common.h"

namespace sfc {

enum class Mode {
  none,
  snes,
  snes_mode7,
  gb,
  gbc
};

inline Mode mode(const std::string& str) {
  if (str == "snes") {
    return Mode::snes;
  } else if (str == "snes_mode7") {
    return Mode::snes_mode7;
  } else if (str == "gb") {
    //TODO:
    //return Mode::gb;
    return Mode::gbc;
  } else if (str == "gbc") {
    return Mode::gbc;
  }
  return Mode::none;
}

inline std::string mode(Mode mode) {
  switch (mode) {
  case Mode::snes:
    return std::string("snes");
  case Mode::snes_mode7:
    return std::string("snes_mode7");
  case Mode::gb:
    //TODO:
    //return std::string("gb");
  case Mode::gbc:
    return std::string("gbc");
  default:
    return std::string("none");
  }
}

constexpr unsigned default_bpp_for_mode(Mode mode) {
  switch (mode) {
    case Mode::snes:
      return 4;
    case Mode::snes_mode7:
      return 8;
    case Mode::gb:
    case Mode::gbc:
      return 2;
    default:
      return 4;
  }
}

constexpr bool bpp_allowed_for_mode(unsigned bpp, Mode mode) {
  switch (mode) {
  case Mode::snes:
    return bpp == 2 || bpp == 4 || bpp == 8;
  case Mode::snes_mode7:
    return bpp == 8;
  case Mode::gb:
  case Mode::gbc:
    return bpp == 2;
  default:
    return false;
  }
}

constexpr unsigned default_tile_size_for_mode(Mode mode) {
  switch (mode) {
  case Mode::snes:
    return 8;
  default:
    return 8;
  }
}

constexpr bool tile_width_allowed_for_mode(unsigned width, Mode mode) {
  switch (mode) {
  case Mode::snes:
    return width == 8 || width == 16;
  case Mode::snes_mode7:
    return width == 8;
  case Mode::gb:
  case Mode::gbc:
    return width == 8;
  default:
    return false;
  }
}

constexpr bool tile_height_allowed_for_mode(unsigned height, Mode mode) {
  switch (mode) {
  case Mode::snes:
    return height == 8 || height == 16;
  case Mode::snes_mode7:
    return height == 8;
  case Mode::gb:
  case Mode::gbc:
    return height == 8;
  default:
    return false;
  }
}

constexpr bool tile_flipping_allowed_for_mode(Mode mode) {
  switch (mode) {
  case Mode::snes:
  case Mode::gbc:
    return true;
  case Mode::snes_mode7:
  case Mode::gb:
    return false;
  default:
    return false;
  }
}

constexpr unsigned default_map_size_for_mode(Mode mode) {
  switch (mode) {
  case Mode::snes:
    return 32;
  case Mode::snes_mode7:
    return 128;
  case Mode::gb:
  case Mode::gbc:
    return 32;
  default:
    return 32;
  }
}

constexpr unsigned default_palette_count_for_mode(Mode mode) {
  switch (mode) {
    case Mode::snes:
      return 8;
    case Mode::snes_mode7:
      return 1;
    case Mode::gb:
      return 1;
    case Mode::gbc:
      return 8;
    default:
      return 8;
  }
}

constexpr bool col0_is_shared_for_mode(Mode mode) {
  switch (mode) {
  case Mode::snes:
  case Mode::snes_mode7:
    return true;
  case Mode::gb:
  case Mode::gbc:
    return false;
  default:
    return true;
  }
}

} /* namespace sfc */
