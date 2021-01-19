#!/usr/bin/env python3

import sys
import os

checker = [
    [0, 2],
    [2, 0]
]

bayer2x2 = [
    [0, 2],
    [3, 1]
]

bayer4x4 = [
    [0,  8,  2,  10],
    [12, 4,  14, 6],
    [3,  11, 1,  9],
    [15, 7,  13, 5]
]

bayer8x8 = [
    [0,  48, 12, 60, 3,  51, 15, 63],
    [32, 16, 44, 28, 35, 19, 47, 31],
    [8,  56, 4,  52, 11, 59, 7,  55],
    [40, 24, 36, 20, 43, 27, 39, 23],
    [2,  50, 14, 62, 1,  49, 13, 61],
    [34, 18, 46, 30, 33, 17, 45, 29],
    [10, 58, 6,  54, 9,  57, 5,  53],
    [42, 26, 38, 22, 41, 25, 37, 21]
]

stippleH = [
    [0, 4, 1, 5],
    [6, 2, 7, 3]
]

stippleV = [
    [0, 4],
    [6, 2],
    [1, 5],
    [7, 3]
]

lineH = [
    [0, 2, 1, 3],
    [7, 6, 7, 7]
]

lineV = [
    [0, 7],
    [2, 6],
    [1, 7],
    [3, 7]
]

def printm(m):
    print("{")
    for r in m:
        print("  " + ", ".join(str(i) for i in r) + ",")
    print("}")

def main():
    try:
        matrices = [checker, bayer2x2, bayer4x4, bayer8x8, stippleH, stippleV, lineH, lineV]
        for matrix in matrices:
            dim = (len(matrix[0]), len(matrix))
            t = [[((n+1) / (dim[0]*dim[1]) - 0.5) for n in row] for row in matrix]
            printm(t)
            print("")

        return 0

    # except Exception as err:
    except IOError as err:
        print("error - {}".format(err))
        sys.exit(1)

if __name__ == "__main__":
    sys.exit(main())
