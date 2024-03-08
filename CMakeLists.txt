cmake_minimum_required (VERSION 3.3)
project (superfamiconv)

set(CMAKE_CXX_STANDARD 20)
include_directories(include)

if(NOT CMAKE_BUILD_TYPE)
  set(CMAKE_BUILD_TYPE MinSizeRel)
endif(NOT CMAKE_BUILD_TYPE)

if(MSVC)
  set(SOURCES include/fmt/format.cpp include/fmt/posix.cpp include/LodePNG/lodepng.cpp include/getoptw/getoptw.c src/superfamiconv.cpp src/sfc_palette.cpp src/sfc_tiles.cpp src/sfc_map.cpp src/Image.cpp src/Map.cpp src/Palette.cpp src/Tiles.cpp)
  add_definitions(-DNOMINMAX)
  set(CMAKE_CXX_FLAGS_RELEASE "${CMAKE_CXX_FLAGS_RELEASE} /MT")
  set(CMAKE_CXX_FLAGS_DEBUG "${CMAKE_CXX_FLAGS_DEBUG} /MTd")
else(MSVC)
# gcc, clang
  set(SOURCES include/fmt/format.cpp include/fmt/posix.cpp include/LodePNG/lodepng.cpp src/superfamiconv.cpp src/sfc_palette.cpp src/sfc_tiles.cpp src/sfc_map.cpp src/Image.cpp src/Map.cpp src/Palette.cpp src/Tiles.cpp)
  set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -Wall -Wextra")
  set(CMAKE_CXX_FLAGS_DEBUG "-O0 -g")

  if(WIN32) # mingw/cygwin
    set(CMAKE_CXX_FLAGS_RELWITHDEBINFO "-O3 -g")
    set(CMAKE_CXX_FLAGS_MINSIZEREL "-O3 -s")
    set(CMAKE_CXX_FLAGS_RELEASE "-O3 -s")
    set(CMAKE_EXE_LINKER_FLAGS "${CMAKE_LD_FLAGS} -static -static-libgcc -static-libstdc++")
  else(WIN32)
    set(CMAKE_CXX_FLAGS_RELWITHDEBINFO "-O3 -g")
    set(CMAKE_CXX_FLAGS_MINSIZEREL "-O3 -s -flto")
    set(CMAKE_CXX_FLAGS_RELEASE "-O3 -s -flto")
  endif(WIN32)

  set(CMAKE_EXE_LINKER_FLAGS_RELEASE "-s")
  set(CMAKE_EXE_LINKER_FLAGS_MINSIZEREL "-s")
endif(MSVC)

add_executable(superfamiconv ${SOURCES})
install(TARGETS superfamiconv DESTINATION bin)
