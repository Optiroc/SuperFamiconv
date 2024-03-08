.PHONY: clean

SRC_DIR := src
INC_DIR := include
BIN_DIR := bin
OBJ_DIR := .build

CXX := g++

FLAGS     := -Wall -Wextra -I$(INC_DIR)
CXX_FLAGS := -std=c++20
LD_FLAGS  :=

ifeq ($(DEBUG), 1)
  CXX_FLAGS += -O0 -g
else
  CXX_FLAGS += -O3 -flto
endif

COMMON_OBJ  := $(OBJ_DIR)/Image.o $(OBJ_DIR)/Palette.o $(OBJ_DIR)/Tiles.o $(OBJ_DIR)/Map.o
LIBRARY_OBJ := $(OBJ_DIR)/LodePNG/lodepng.o $(OBJ_DIR)/fmt/format.o
HEADERS     := $(wildcard $(SRC_DIR)/*.h)
HEADERS     += $(INC_DIR)/Options.h $(INC_DIR)/nlohmann/json.hpp $(wildcard $(INC_DIR)/fmt/*.h)

superfamiconv: $(BIN_DIR)/superfamiconv

$(BIN_DIR)/superfamiconv : $(OBJ_DIR)/superfamiconv.o $(OBJ_DIR)/sfc_palette.o $(OBJ_DIR)/sfc_tiles.o $(OBJ_DIR)/sfc_map.o $(COMMON_OBJ) $(LIBRARY_OBJ) | $(BIN_DIR)
	$(CXX) $(LD_FLAGS) $^ -o $@

$(OBJ_DIR)/%.o : ./**/%.cpp $(HEADERS)
	@mkdir -pv $(dir $@)
	$(CXX) $(CXX_FLAGS) $(FLAGS) -c $< -o $@

$(BIN_DIR):
	@mkdir -pv $@

clean:
	@rm -rf $(OBJ_DIR) $(BIN_DIR)
