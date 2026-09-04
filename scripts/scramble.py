# scrambles an image by:
# - shuffling the pixels within each 8x8 tile
# - shuffling the 8x8 tiles within each row across the image
# - all tiles all scrambed in the same way, so tile deduplication is unaffected

import random
import sys
from datetime import datetime

from PIL import Image

TILE_SIZE = 8

def scramble(img):
    width, height = img.size
    assert width % TILE_SIZE == 0, f"width {width} is not divisible by {TILE_SIZE}"
    assert height % TILE_SIZE == 0, f"height {height} is not divisible by {TILE_SIZE}"
    pixels = img.load()

    tile_offsets = [(dx, dy) for dy in range(TILE_SIZE) for dx in range(TILE_SIZE)]
    pixel_order = list(range(len(tile_offsets)))
    random.shuffle(pixel_order)

    tile_rows = [
        [(tile_x, tile_y) for tile_x in range(0, width, TILE_SIZE)]
        for tile_y in range(0, height, TILE_SIZE)
    ]

    # scramble tiles
    for row in tile_rows:
        for tile_x, tile_y in row:
            coords = [(tile_x + dx, tile_y + dy) for dx, dy in tile_offsets]
            values = [pixels[coords[i]] for i in pixel_order]
            for coord, value in zip(coords, values):
                pixels[coord] = value

    # scramble tile positions within each row
    src = img.copy()
    src_pixels = src.load()
    for row in tile_rows:
        shuffled_row = row[:]
        random.shuffle(shuffled_row)
        for (dst_x, dst_y), (src_x, src_y) in zip(row, shuffled_row):
            for dx, dy in tile_offsets:
                pixels[dst_x + dx, dst_y + dy] = src_pixels[src_x + dx, src_y + dy]

if __name__ == "__main__":
    if len(sys.argv) != 3:
        print(f"usage: {sys.argv[0]} input.png output.png", file=sys.stderr)
        sys.exit(1)

    random.seed(datetime.now().timestamp())
    img = Image.open(sys.argv[1])
    scramble(img)
    img.save(sys.argv[2])
