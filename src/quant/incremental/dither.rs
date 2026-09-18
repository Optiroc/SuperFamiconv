//! Dither variants for the incremental k-means quantization method.

use crate::dither::Dither;

pub(super) const DITHER_WEIGHT: f32 = 0.5;
pub(super) const MAX_CANDIDATES: usize = 4;

type DitherMatrix = [[usize; 2]; 2];

#[rustfmt::skip]
const BAYER4: DitherMatrix = [
    [0, 2],
    [3, 1],
];

#[rustfmt::skip]
const CHECKER: DitherMatrix = [
    [0, 1],
    [1, 0],
];

#[rustfmt::skip]
const STIPPLE_V: DitherMatrix = [
    [0, 1],
    [3, 2],
];

#[rustfmt::skip]
const STIPPLE_H: DitherMatrix = [
    [0, 3],
    [1, 2],
];

#[rustfmt::skip]
const LINE_V: DitherMatrix = [
    [0, 0],
    [1, 1],
];

#[rustfmt::skip]
const LINE_H: DitherMatrix = [
    [0, 1],
    [0, 1],
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum IncrementalDither {
    Bayer2x2,
    Checker,
    StippleV,
    StippleH,
    LineV,
    LineH,
}

impl IncrementalDither {
    pub fn for_dither(dither: Dither) -> Result<Option<Self>, String> {
        match dither {
            Dither::Off => Ok(None),
            Dither::Bayer2x2 => Ok(Some(IncrementalDither::Bayer2x2)),
            Dither::Checker => Ok(Some(IncrementalDither::Checker)),
            Dither::StippleV => Ok(Some(IncrementalDither::StippleV)),
            Dither::StippleH => Ok(Some(IncrementalDither::StippleH)),
            Dither::LineV => Ok(Some(IncrementalDither::LineV)),
            Dither::LineH => Ok(Some(IncrementalDither::LineH)),
            Dither::Bayer4x4 | Dither::Bayer8x8 | Dither::Atkinson | Dither::FloydSteinberg => Err(format!(
                "{dither} dithering is not supported by the incremental quantizer"
            )),
        }
    }

    fn table(self) -> DitherMatrix {
        match self {
            IncrementalDither::Bayer2x2 => BAYER4,
            IncrementalDither::Checker => CHECKER,
            IncrementalDither::StippleV => STIPPLE_V,
            IncrementalDither::StippleH => STIPPLE_H,
            IncrementalDither::LineV => LINE_V,
            IncrementalDither::LineH => LINE_H,
        }
    }

    /// Number of candidates tested per pixel.
    pub(super) fn n_candidates(self) -> usize {
        match self {
            IncrementalDither::Bayer2x2 | IncrementalDither::StippleH | IncrementalDither::StippleV => 4,
            IncrementalDither::Checker | IncrementalDither::LineH | IncrementalDither::LineV => 2,
        }
    }

    /// Rank of the candidate to pick at `(x, y)`.
    pub(super) fn rank(
        self,
        x: u32,
        y: u32,
    ) -> usize {
        self.table()[(x & 1) as usize][(y & 1) as usize]
    }
}
