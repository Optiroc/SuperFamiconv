BUILD_DIR := build

.PHONY: clean

release:
	@cmake -B$(BUILD_DIR)/release -DCMAKE_BUILD_TYPE=Release
	@cmake --build $(BUILD_DIR)/release --parallel 4

debug:
	@cmake -B$(BUILD_DIR)/debug -DCMAKE_BUILD_TYPE=Debug
	@cmake --build $(BUILD_DIR)/debug --parallel 4

all: clean release

clean:
	@rm -rf $(BUILD_DIR)
