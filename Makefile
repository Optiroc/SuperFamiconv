SRC_DIR := src
INC_DIR := include
BIN_DIR := bin
OBJ_DIR := .build

CC  := gcc
CXX := g++

FLAGS     := -Wextra -I$(INC_DIR)
CXX_FLAGS := -std=gnu++11 $(FLAGS)
CC_FLAGS  := -std=gnu99 $(FLAGS)
LD_FLAGS  :=

ifeq ($(OS),Windows_NT)
  CC := x86_64-w64-mingw32-gcc
  CXX := x86_64-w64-mingw32-g++
  LD_FLAGS += -static -static-libgcc -static-libstdc++
endif

ifeq ($(DEBUG), 1)
  CXX_FLAGS += -O0 -g
  CC_FLAGS += -O0 -g
else
  ifeq ($(OS),Windows_NT)
    CXX_FLAGS += -O3
    CC_FLAGS += -O3
  else
    CXX_FLAGS += -O3 -flto
    CC_FLAGS += -O3 -flto
  endif
endif


COMMON_OBJ  := $(OBJ_DIR)/Image.o $(OBJ_DIR)/Palette.o $(OBJ_DIR)/Tiles.o $(OBJ_DIR)/Map.o
COMMON_OBJ  += $(OBJ_DIR)/LodePNG/lodepng.o
HEADERS     := $(wildcard $(SRC_DIR)/*.h)

.PHONY: clean sfc_palette sfc_tiles sfc_map

default: superfamiconv

all: sfc_palette sfc_tiles sfc_map superfamiconv

sfc_palette: $(BIN_DIR)/sfc_palette
sfc_tiles: $(BIN_DIR)/sfc_tiles
sfc_map: $(BIN_DIR)/sfc_map
superfamiconv: $(BIN_DIR)/superfamiconv

$(BIN_DIR)/sfc_palette : $(OBJ_DIR)/sfc_palette.o $(COMMON_OBJ) | $(BIN_DIR)
	$(CXX) $(LD_FLAGS) $^ -o $@

$(BIN_DIR)/sfc_tiles : $(OBJ_DIR)/sfc_tiles.o $(COMMON_OBJ) | $(BIN_DIR)
	$(CXX) $(LD_FLAGS) $^ -o $@

$(BIN_DIR)/sfc_map : $(OBJ_DIR)/sfc_map.o $(COMMON_OBJ) | $(BIN_DIR)
	$(CXX) $(LD_FLAGS) $^ -o $@

$(BIN_DIR)/superfamiconv : $(OBJ_DIR)/superfamiconv.o $(OBJ_DIR)/sfc_palette.om $(OBJ_DIR)/sfc_tiles.om $(OBJ_DIR)/sfc_map.om $(COMMON_OBJ) | $(BIN_DIR)
	$(CXX) $(LD_FLAGS) -D SFC_MONOLITH $^ -o $@

$(OBJ_DIR)/%.o : ./**/%.cpp $(HEADERS)
	@mkdir -pv $(dir $@)
	$(CXX) $(CXX_FLAGS) -c $< -o $@

$(OBJ_DIR)/%.om : ./**/%.cpp $(HEADERS)
	@mkdir -pv $(dir $@)
	$(CXX) -D SFC_MONOLITH $(CXX_FLAGS) -c $< -o $@

$(OBJ_DIR)/%.o : ./**/%.c $(HEADERS)
	@mkdir -pv $(dir $@)
	$(CC) $(CC_FLAGS) -c $< -o $@

$(BIN_DIR):
	@mkdir -pv $@

clean:
	@rm -rf $(OBJ_DIR) $(BIN_DIR)
