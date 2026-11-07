//! Cell sets of a grid with at most 64 cells, one bit per cell in row-major
//! order. Neighbours, floods and 2x2 blocks are a few shifts and masks.
use crate::Shape;

/// A set of cells.
pub(crate) type Bits = u64;

/// The cell with index `i`.
pub(crate) const fn bit(i: usize) -> Bits {
    1 << i
}

/// The cells of a set in ascending order.
pub(crate) fn cells(mut bits: Bits) -> impl Iterator<Item = usize> {
    std::iter::from_fn(move || {
        if bits == 0 {
            None
        } else {
            let i = bits.trailing_zeros() as usize;
            bits &= bits - 1;
            Some(i)
        }
    })
}

/// The number of cells in a set.
pub(crate) fn count(bits: Bits) -> usize {
    bits.count_ones() as usize
}

/// The lowest cell of a nonempty set.
pub(crate) fn first(bits: Bits) -> usize {
    debug_assert_ne!(bits, 0);
    bits.trailing_zeros() as usize
}

/// A grid's shape as bit masks.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Grid {
    pub cols: usize,
    /// Every cell.
    pub all: Bits,
    /// Cells with a cell to their left.
    not_left: Bits,
    /// Cells with a cell to their right.
    not_right: Bits,
    /// Cells that are the top-left corner of a 2x2 block.
    corners: Bits,
}

impl Grid {
    pub fn new(shape: Shape) -> Self {
        let (rows, cols) = (shape.rows(), shape.cols());
        let cells = rows * cols;
        debug_assert!(cells <= 64);
        let all = if cells == 64 { !0 } else { bit(cells) - 1 };
        let mut not_left = 0;
        let mut not_right = 0;
        let mut corners = 0;
        for i in 0..cells {
            let (r, c) = (i / cols, i % cols);
            if c > 0 {
                not_left |= bit(i);
            }
            if c + 1 < cols {
                not_right |= bit(i);
            }
            if r + 1 < rows && c + 1 < cols {
                corners |= bit(i);
            }
        }
        Self {
            cols,
            all,
            not_left,
            not_right,
            corners,
        }
    }

    /// Cells whose neighbour below is in the set.
    fn above(&self, b: Bits) -> Bits {
        b >> self.cols
    }

    /// Cells whose neighbour above is in the set.
    fn below(&self, b: Bits) -> Bits {
        (b << self.cols) & self.all
    }

    /// Cells whose neighbour to the right is in the set.
    fn left_of(&self, b: Bits) -> Bits {
        (b & self.not_left) >> 1
    }

    /// Cells whose neighbour to the left is in the set.
    fn right_of(&self, b: Bits) -> Bits {
        (b & self.not_right) << 1
    }

    /// Cells with a side in common with a cell of the set, including cells
    /// of the set itself.
    pub fn adjacent(&self, b: Bits) -> Bits {
        self.above(b) | self.below(b) | self.left_of(b) | self.right_of(b)
    }

    /// Cells outside the set with a side in common with a cell of the set.
    pub fn border(&self, b: Bits) -> Bits {
        self.adjacent(b) & !b
    }

    /// The cells of `allowed` connected to `seed` through `allowed`, plus
    /// `seed` itself. Unlimited distance.
    pub fn flood(&self, seed: Bits, allowed: Bits) -> Bits {
        self.flood_steps(seed, allowed, usize::MAX)
    }

    /// Like [`Grid::flood`], but at most `steps` cells away from `seed`.
    pub fn flood_steps(&self, seed: Bits, allowed: Bits, steps: usize) -> Bits {
        let mut region = seed;
        let mut frontier = seed;
        let mut left = steps;
        while frontier != 0 && left > 0 {
            frontier = self.adjacent(frontier) & allowed & !region;
            region |= frontier;
            left -= 1;
        }
        region
    }

    /// Top-left corners of the 2x2 blocks that lie entirely in the set.
    pub fn pools(&self, b: Bits) -> Bits {
        let right = self.left_of(b);
        b & right & self.above(b) & self.above(right) & self.corners
    }

    /// The four cells of the 2x2 block with top-left corner `i`.
    pub fn block(&self, i: usize) -> Bits {
        let top = bit(i) | bit(i + 1);
        top | top << self.cols
    }

    /// Cells of the set within Manhattan distance `d` of cell `i`.
    pub fn ball(&self, i: usize, d: usize) -> Bits {
        self.flood_steps(bit(i), self.all, d)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neighbours_floods_and_pools_follow_the_grid() {
        let g = Grid::new(Shape::new(3, 4).unwrap());
        // Cell 5 is the middle of the second row.
        assert_eq!(g.adjacent(bit(5)), bit(1) | bit(4) | bit(6) | bit(9));
        // Corners and edges do not wrap.
        assert_eq!(g.adjacent(bit(3)), bit(2) | bit(7));
        assert_eq!(g.adjacent(bit(8)), bit(4) | bit(9));
        assert_eq!(g.border(bit(0) | bit(1)), bit(2) | bit(4) | bit(5));
        let allowed = bit(0) | bit(1) | bit(5) | bit(9) | bit(3);
        assert_eq!(g.flood(bit(0), allowed), bit(0) | bit(1) | bit(5) | bit(9));
        assert_eq!(g.flood_steps(bit(0), allowed, 1), bit(0) | bit(1));
        // The seed is included whether or not it is allowed.
        assert_eq!(g.flood(bit(2), bit(6) | bit(11)), bit(2) | bit(6));
        let sea = bit(0) | bit(1) | bit(4) | bit(5) | bit(6) | bit(10);
        assert_eq!(g.pools(sea), bit(0));
        assert_eq!(g.block(5), bit(5) | bit(6) | bit(9) | bit(10));
        assert_eq!(g.ball(0, 1), bit(0) | bit(1) | bit(4));
        assert_eq!(cells(bit(3) | bit(9)).collect::<Vec<_>>(), [3, 9]);
        let full = Grid::new(Shape::new(8, 8).unwrap());
        assert_eq!(full.all, u64::MAX);
        assert_eq!(count(full.pools(u64::MAX)), 49);
    }
}
