//! Dense 2-D grid with deterministic neighbour iteration.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Grid<T> {
    pub w: usize,
    pub h: usize,
    pub data: Vec<T>,
}

/// 8-neighbour offsets in a fixed order (E, SE, S, SW, W, NW, N, NE).
pub const N8: [(i32, i32); 8] = [
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
];
/// 4-neighbour offsets (E, S, W, N).
pub const N4: [(i32, i32); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];

impl<T: Clone> Grid<T> {
    pub fn new(w: usize, h: usize, fill: T) -> Self {
        Grid {
            w,
            h,
            data: vec![fill; w * h],
        }
    }
}

impl<T> Grid<T> {
    #[inline]
    pub fn idx(&self, x: usize, y: usize) -> usize {
        y * self.w + x
    }
    #[inline]
    pub fn xy(&self, i: usize) -> (usize, usize) {
        (i % self.w, i / self.w)
    }
    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
    #[inline]
    pub fn get(&self, i: usize) -> &T {
        &self.data[i]
    }
    #[inline]
    pub fn set(&mut self, i: usize, v: T) {
        self.data[i] = v;
    }
    pub fn is_edge(&self, i: usize) -> bool {
        let (x, y) = self.xy(i);
        x == 0 || y == 0 || x + 1 == self.w || y + 1 == self.h
    }
    /// Neighbour index for an offset, if in bounds.
    #[inline]
    pub fn offset(&self, i: usize, d: (i32, i32)) -> Option<usize> {
        let (x, y) = self.xy(i);
        let nx = x as i64 + d.0 as i64;
        let ny = y as i64 + d.1 as i64;
        if nx < 0 || ny < 0 || nx >= self.w as i64 || ny >= self.h as i64 {
            None
        } else {
            Some(ny as usize * self.w + nx as usize)
        }
    }
    pub fn neighbors8(&self, i: usize) -> impl Iterator<Item = usize> + '_ {
        N8.iter().filter_map(move |d| self.offset(i, *d))
    }
    pub fn neighbors4(&self, i: usize) -> impl Iterator<Item = usize> + '_ {
        N4.iter().filter_map(move |d| self.offset(i, *d))
    }
}

/// Euclidean distance between two cells.
pub fn cell_dist<T>(g: &Grid<T>, a: usize, b: usize) -> f64 {
    let (ax, ay) = g.xy(a);
    let (bx, by) = g.xy(b);
    let dx = ax as f64 - bx as f64;
    let dy = ay as f64 - by as f64;
    mandate_core::math::sqrt(dx * dx + dy * dy)
}

/// Min-heap entry ordered by cost, then index (deterministic tie-break).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeapItem {
    pub cost: f64,
    pub idx: usize,
}

impl Eq for HeapItem {}
impl Ord for HeapItem {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        o.cost
            .total_cmp(&self.cost)
            .then_with(|| o.idx.cmp(&self.idx))
    }
}
impl PartialOrd for HeapItem {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}
