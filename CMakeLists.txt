cmake_minimum_required(VERSION 3.20)
project(superfamiconv LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 20)

include_directories(include)

if(NOT CMAKE_BUILD_TYPE)
  set(CMAKE_BUILD_TYPE Debug)
endif()

if(CMAKE_BUILD_TYPE STREQUAL "Release")
  set(CMAKE_OSX_ARCHITECTURES arm64 x86_64)
endif()

if(MSVC)
  add_definitions(-DNOMINMAX)
  set(CMAKE_CXX_FLAGS_RELEASE "${CMAKE_CXX_FLAGS_RELEASE} /MT")
  set(CMAKE_CXX_FLAGS_DEBUG "${CMAKE_CXX_FLAGS_DEBUG} /MTd")
endif()

if(MSVC)
  set(SOURCES include/LodePNG/lodepng.cpp include/getopt-win/getopt.c src/superfamiconv.cpp src/sfc_palette.cpp src/sfc_tiles.cpp src/sfc_map.cpp src/Image.cpp src/Map.cpp src/Palette.cpp src/Tiles.cpp)
else()
  set(SOURCES include/LodePNG/lodepng.cpp src/superfamiconv.cpp src/sfc_palette.cpp src/sfc_tiles.cpp src/sfc_map.cpp src/Image.cpp src/Map.cpp src/Palette.cpp src/Tiles.cpp)
endif()

add_executable(superfamiconv ${SOURCES})

include(FetchContent)

FetchContent_Declare(
  fmt
  GIT_REPOSITORY https://github.com/fmtlib/fmt
  GIT_TAG        407c905e45ad75fc29bf0f9bb7c5c2fd3475976f) # 12.1.0
FetchContent_MakeAvailable(fmt)

target_link_libraries(superfamiconv fmt::fmt)
