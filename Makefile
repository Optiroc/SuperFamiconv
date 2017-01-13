SRC_DIR     := src
INC_DIR     := include
BIN_DIR     := bin
OBJ_DIR     := .build

CXX_FLAGS   := -std=c++14 -O3 -Wall -I$(INC_DIR)
CC_FLAGS    := -std=gnu99 -O3 -Wall -I$(INC_DIR)
LD_FLAGS    :=

COMMON_OBJ  := $(OBJ_DIR)/Image.o $(OBJ_DIR)/Palette.o $(OBJ_DIR)/Palette.o $(OBJ_DIR)/Tiles.o $(OBJ_DIR)/Map.o
INC_SRC		:= $(wildcard $(INC_DIR)/**/*.cpp)
INC_OBJ     += $(patsubst $(INC_DIR)%,$(OBJ_DIR)%,$(patsubst %.cpp,%.o,$(INC_SRC)))
HEADERS     := $(wildcard $(SRC_DIR)/*.h)

.PHONY: clean sfc_palette sfc_tiles sfc_map

default: superfamiconv

all: sfc_palette sfc_tiles sfc_map superfamiconv

sfc_palette: $(BIN_DIR)/sfc_palette
sfc_tiles: $(BIN_DIR)/sfc_tiles
sfc_map: $(BIN_DIR)/sfc_map
superfamiconv: $(BIN_DIR)/superfamiconv

$(BIN_DIR)/sfc_palette : $(OBJ_DIR)/sfc_palette.o  $(COMMON_OBJ) $(INC_OBJ) | $(BIN_DIR)
	g++ $(LD_FLAGS) $^ -o $@

$(BIN_DIR)/sfc_tiles : $(OBJ_DIR)/sfc_tiles.o $(COMMON_OBJ) $(INC_OBJ) | $(BIN_DIR)
	g++ $(LD_FLAGS) $^ -o $@

$(BIN_DIR)/sfc_map : $(OBJ_DIR)/sfc_map.o $(COMMON_OBJ) $(INC_OBJ) | $(BIN_DIR)
	g++ $(LD_FLAGS) $^ -o $@

$(BIN_DIR)/superfamiconv : $(OBJ_DIR)/superfamiconv.o $(OBJ_DIR)/sfc_palette.om $(OBJ_DIR)/sfc_tiles.om $(OBJ_DIR)/sfc_map.om $(COMMON_OBJ) $(INC_OBJ) | $(BIN_DIR)
	g++ $(LD_FLAGS) -D SFC_MONOLITH $^ -o $@

$(BIN_DIR):
	@mkdir -pv $@

$(OBJ_DIR)/%.o : ./**/%.cpp $(HEADERS)
	@mkdir -pv $(dir $@)
	g++ $(CXX_FLAGS) -c $< -o $@

$(OBJ_DIR)/%.om : ./**/%.cpp $(HEADERS)
	@mkdir -pv $(dir $@)
	g++ -D SFC_MONOLITH $(CXX_FLAGS) -c $< -o $@

$(OBJ_DIR)/%.o : ./**/%.c $(HEADERS)
	@mkdir -pv $(dir $@)
	gcc $(CC_FLAGS) -c $< -o $@

clean:
	@rm -rf $(OBJ_DIR) $(BIN_DIR)
