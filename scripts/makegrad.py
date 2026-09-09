import numpy as np
from PIL import Image

SIZE = 256
START_RGB = (127, 127, 127)
STOP_RGB = (191, 191, 191)
PREFIX = "quarter"

def save_gradient(path, values):
    start = np.array(START_RGB, dtype=np.float64)
    stop = np.array(STOP_RGB, dtype=np.float64)
    t = values.astype(np.float64)[..., None] / 255.0
    img = (start + (stop - start) * t).astype(np.uint8)
    Image.fromarray(img, mode="RGB").save(path)

# make horizontal gradient
row = np.arange(SIZE, dtype=np.uint8)
horizontal = np.tile(row, (SIZE, 1))
save_gradient(f"{PREFIX}_gradient_h.png", horizontal)

# make vertical gradient
col = np.arange(SIZE, dtype=np.uint8).reshape(SIZE, 1)
vertical = np.tile(col, (1, SIZE))
save_gradient(f"{PREFIX}_gradient_v.png", vertical)

# make diagonal gradient
x, y = np.meshgrid(np.arange(SIZE), np.arange(SIZE))
diagonal = np.clip((x + y) // 2, 0, 255).astype(np.uint8)
save_gradient(f"{PREFIX}_gradient_d.png", diagonal)
