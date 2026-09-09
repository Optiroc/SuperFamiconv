use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

use crate::binpack::prng::Prng;
use crate::color::ReducedColor as Symbol;

/// Optimizes palettes using overload-and-remove bin packing.
///
/// References:
/// - <https://arxiv.org/abs/1605.00558>
/// - <https://git.sr.ht/~issotm/pagination-rs/tree/master/item/src/lib.rs>
/// - <https://github.com/gbdev/rgbds/tree/master/src/gfx/pal_packing.cpp>
pub fn pack(
    requirements: &[BTreeSet<Symbol>],
    capacity: usize,
    seed: Option<u64>,
) -> Vec<BTreeSet<Symbol>> {
    let mut pages: Vec<Page> = Vec::new();

    // Place each tile into pages, largest-first
    let mut order: Vec<Tile> = (0..requirements.len()).map(Tile::new).collect();

    // If `seed` was given shuffle tiles of equal size
    if let Some(seed) = seed {
        Prng::new(seed).shuffle(&mut order);
    }

    order.sort_by_key(|t| std::cmp::Reverse(requirements[t.idx].len()));
    let mut queue: VecDeque<Tile> = order.into();

    while let Some(tile) = queue.pop_front() {
        // Find page where this tile would take the least relative space
        let colors = &requirements[tile.idx];
        let best_page_idx = pages
            .iter()
            .enumerate()
            .filter(|(page_idx, _)| !tile.forbidden_from.contains(page_idx))
            // Keep pages with some shared colors, pair with relative size if added
            .filter_map(|(page_idx, page)| {
                let rel = page.relative_size_if_added(colors);
                (rel < colors.len() as f64).then_some((page_idx, rel))
            })
            // Select page where the tile would occupy the least relative size
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(page_idx, _)| page_idx);

        let Some(best_page_idx) = best_page_idx else {
            // No page found, add tile to new page
            pages.push(Page::from_tile(tile, colors));
            continue;
        };

        // Add tile to selected page
        pages[best_page_idx].add(tile, colors);

        // If page is overloaded, try to remove worst fit until it's within capacity
        loop {
            if pages[best_page_idx].volume() <= capacity {
                break;
            }

            // Color count vs relative size ratio per tile
            let ratios: Vec<(usize, f64)> = pages[best_page_idx]
                .tiles
                .iter()
                .map(|t| {
                    let colors = &requirements[t.idx];
                    let rel_size = pages[best_page_idx].relative_size_of(colors);
                    (t.idx, colors.len() as f64 / rel_size)
                })
                .collect();

            // If all tiles on page have ~equal ratio leave page overloaded
            let (min_ratio, max_ratio) = ratios
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &(_, r)| {
                    (lo.min(r), hi.max(r))
                });
            if max_ratio - min_ratio < 1e-9 {
                break;
            }

            // Put tile with lowest ratio back on queue and forbid it from reentering
            let min_idx = ratios.iter().find(|&&(_, r)| r == min_ratio).unwrap().0;
            let mut removed = pages[best_page_idx].remove(min_idx, &requirements[min_idx]);
            removed.forbidden_from.insert(best_page_idx);
            queue.push_back(removed);
        }
    }

    // Remove pages still overloaded
    let mut overloaded_tiles: Vec<usize> = Vec::new();
    pages.retain(|page| {
        if page.volume() <= capacity {
            true
        } else {
            overloaded_tiles.extend(page.tiles.iter().map(|t| t.idx));
            false
        }
    });

    // Insert overloaded tiles into the first page they fit in, or a new one
    for tile_idx in overloaded_tiles {
        let tile = &requirements[tile_idx];
        let fit = pages.iter().position(|p| p.volume_if_added(tile) <= capacity);
        match fit {
            Some(i) => pages[i].add(Tile::new(tile_idx), tile),
            None => pages.push(Page::from_tile(Tile::new(tile_idx), tile)),
        }
    }

    // Decant
    pages.sort_by_key(|p| std::cmp::Reverse(p.volume()));
    let decanted = decant(pages, capacity, requirements);

    let mut result: Vec<BTreeSet<Symbol>> = decanted
        .into_iter()
        .map(|page| page.mult.into_keys().collect())
        .collect();
    result.sort_by_key(|b| std::cmp::Reverse(b.len()));
    result
}

struct Tile {
    idx: usize,
    /// Page indices tile is forbidden from reentering.
    forbidden_from: HashSet<usize>,
}

impl Tile {
    fn new(idx: usize) -> Self {
        Self {
            idx,
            forbidden_from: HashSet::new(),
        }
    }
}

#[derive(Default)]
struct Page {
    /// Tiles assigned to this page.
    tiles: Vec<Tile>,
    /// Multiplicites (count of this page's tiles that contain a given color).
    mult: HashMap<Symbol, u32>,
}

impl Page {
    fn from_tile(
        tile: Tile,
        symbols: &BTreeSet<Symbol>,
    ) -> Self {
        Self {
            tiles: vec![tile],
            mult: symbols.iter().map(|&c| (c, 1)).collect(),
        }
    }

    fn tile_indices(&self) -> Vec<usize> {
        self.tiles.iter().map(|t| t.idx).collect()
    }

    fn volume(&self) -> usize {
        self.mult.len()
    }

    fn volume_if_added<'a, I: IntoIterator<Item = &'a Symbol>>(
        &self,
        symbols: I,
    ) -> usize {
        self.mult.len() + symbols.into_iter().filter(|c| !self.mult.contains_key(c)).count()
    }

    fn relative_size_of(
        &self,
        symbols: &BTreeSet<Symbol>,
    ) -> f64 {
        symbols.iter().map(|c| 1.0 / f64::from(self.mult[c])).sum()
    }

    fn relative_size_if_added(
        &self,
        symbols: &BTreeSet<Symbol>,
    ) -> f64 {
        symbols
            .iter()
            .map(|s| 1.0 / f64::from(self.mult.get(s).copied().unwrap_or(0) + 1))
            .sum()
    }

    fn add(
        &mut self,
        tile: Tile,
        symbols: &BTreeSet<Symbol>,
    ) {
        for &s in symbols {
            *self.mult.entry(s).or_insert(0) += 1;
        }
        self.tiles.push(tile);
    }

    fn remove(
        &mut self,
        tile_idx: usize,
        symbols: &BTreeSet<Symbol>,
    ) -> Tile {
        let pos = self.tiles.iter().position(|t| t.idx == tile_idx).unwrap();
        let tile = self.tiles.remove(pos);
        for &s in symbols {
            if let Some(m) = self.mult.get_mut(&s) {
                *m -= 1;
                if *m == 0 {
                    self.mult.remove(&s);
                }
            }
        }
        tile
    }

    fn merge(
        &mut self,
        mut other: Page,
    ) {
        self.tiles.append(&mut other.tiles);
        for (s, n) in other.mult {
            *self.mult.entry(s).or_insert(0) += n;
        }
    }
}

fn decant(
    pages: Vec<Page>,
    capacity: usize,
    requirements: &[BTreeSet<Symbol>],
) -> Vec<Page> {
    // Pass 1: Whole pages
    let mut decanted: Vec<Page> = Vec::new();
    for page in pages {
        let fits_page_idx = decanted
            .iter()
            .position(|p| p.volume_if_added(page.mult.keys()) <= capacity);
        match fits_page_idx {
            Some(page_idx) => decanted[page_idx].merge(page),
            None => decanted.push(page),
        }
    }

    // Phase 2: Components (tiles sharing symbols)
    for from_page_idx in (1..decanted.len()).rev() {
        let mut from_page = std::mem::take(&mut decanted[from_page_idx]);
        for component in connected_components(&from_page.tile_indices(), requirements) {
            let component_symbols: BTreeSet<Symbol> = component
                .iter()
                .flat_map(|&tile_idx| requirements[tile_idx].iter().copied())
                .collect();
            let fits_page_idx = decanted[..from_page_idx]
                .iter()
                .position(|p| p.volume_if_added(&component_symbols) <= capacity);
            if let Some(to_page_idx) = fits_page_idx {
                for &fits_tile_idx in &component {
                    let tile = from_page.remove(fits_tile_idx, &requirements[fits_tile_idx]);
                    decanted[to_page_idx].add(tile, &requirements[fits_tile_idx]);
                }
            }
        }
        if from_page.tiles.is_empty() {
            decanted.remove(from_page_idx);
            continue;
        }
        decanted[from_page_idx] = from_page;
    }

    // Phase 3: Individual tiles
    for from_page_idx in (1..decanted.len()).rev() {
        let mut from_page = std::mem::take(&mut decanted[from_page_idx]);
        for from_tile_idx in from_page.tile_indices() {
            let symbols = &requirements[from_tile_idx];
            let fits_page_idx = decanted[..from_page_idx]
                .iter()
                .position(|p| p.volume_if_added(symbols) <= capacity);
            if let Some(to_page_idx) = fits_page_idx {
                let tile = from_page.remove(from_tile_idx, symbols);
                decanted[to_page_idx].add(tile, symbols);
            }
        }
        if from_page.tiles.is_empty() {
            decanted.remove(from_page_idx);
            continue;
        }
        decanted[from_page_idx] = from_page;
    }

    decanted
}

/// Groups `tile_indices` into components of tiles that share at least one color.
fn connected_components(
    tile_indices: &[usize],
    requirements: &[BTreeSet<Symbol>],
) -> Vec<Vec<usize>> {
    fn find(
        parent: &mut [usize],
        x: usize,
    ) -> usize {
        if parent[x] != x {
            parent[x] = find(parent, parent[x]);
        }
        parent[x]
    }

    let mut parent: Vec<usize> = (0..tile_indices.len()).collect();

    let mut first_with_symbol: HashMap<Symbol, usize> = HashMap::new();
    for (pos, &tile_idx) in tile_indices.iter().enumerate() {
        for &s in &requirements[tile_idx] {
            match first_with_symbol.get(&s) {
                Some(&other_pos) => {
                    let (root_a, root_b) = (find(&mut parent, pos), find(&mut parent, other_pos));
                    parent[root_a] = root_b;
                }
                None => {
                    first_with_symbol.insert(s, pos);
                }
            }
        }
    }

    let mut components: HashMap<usize, Vec<usize>> = HashMap::new();
    for (pos, &tile_idx) in tile_indices.iter().enumerate() {
        let root = find(&mut parent, pos);
        components.entry(root).or_default().push(tile_idx);
    }
    components.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test with gnarly palette from
    /// https://github.com/Rangi42/tilemap-studio/issues/86
    #[test]
    fn tricky_4x4_palette() {
        fn tile(
            a: (u8, u8, u8),
            b: (u8, u8, u8),
        ) -> BTreeSet<Symbol> {
            [a, b].into_iter().map(|(r, g, b)| Symbol::new(r, g, b, 0xff)).collect()
        }

        let requirements = vec![
            tile((0x84, 0xff, 0x00), (0xff, 0x00, 0x00)),
            tile((0x00, 0xff, 0x42), (0xff, 0xbd, 0x00)),
            tile((0x00, 0xff, 0xff), (0xff, 0x00, 0x00)),
            tile((0x00, 0x42, 0xff), (0xff, 0xbd, 0x00)),
            tile((0x84, 0x00, 0xff), (0xff, 0x00, 0x00)),
            tile((0xff, 0x00, 0xbd), (0xff, 0xbd, 0x00)),
            tile((0x00, 0xff, 0xff), (0x84, 0xff, 0x00)),
            tile((0x00, 0x42, 0xff), (0x00, 0xff, 0x42)),
            tile((0x84, 0x00, 0xff), (0x84, 0xff, 0x00)),
            tile((0x00, 0xff, 0x42), (0xff, 0x00, 0xbd)),
            tile((0x00, 0xff, 0xff), (0x84, 0x00, 0xff)),
            tile((0x00, 0x42, 0xff), (0xff, 0x00, 0xbd)),
            tile((0x5a, 0xb5, 0x00), (0xb5, 0x00, 0x00)),
            tile((0x00, 0xb5, 0x29), (0xb5, 0x84, 0x00)),
            tile((0x00, 0xb5, 0xb5), (0xb5, 0x00, 0x00)),
            tile((0x00, 0x29, 0xb5), (0xb5, 0x84, 0x00)),
            tile((0x5a, 0x00, 0xb5), (0xb5, 0x00, 0x00)),
            tile((0xb5, 0x00, 0x84), (0xb5, 0x84, 0x00)),
            tile((0x00, 0xb5, 0xb5), (0x5a, 0xb5, 0x00)),
            tile((0x00, 0x29, 0xb5), (0x00, 0xb5, 0x29)),
            tile((0x5a, 0x00, 0xb5), (0x5a, 0xb5, 0x00)),
            tile((0x00, 0xb5, 0x29), (0xb5, 0x00, 0x84)),
            tile((0x00, 0xb5, 0xb5), (0x5a, 0x00, 0xb5)),
            tile((0x00, 0x29, 0xb5), (0xb5, 0x00, 0x84)),
        ];

        let pages = pack(&requirements, 4, None);

        assert_eq!(pages.len(), 4);
        assert!(pages.iter().all(|p| p.len() == 4));
    }
}
