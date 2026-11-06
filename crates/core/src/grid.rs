//! Rectangular grid helpers shared by genres (I5).
//!
//! ```
//! use pencil_puzzles_core::grid::{flood, neighbours};
//!
//! let nb = neighbours(2, 3);
//! assert_eq!(nb[1], [0, 2, 4]);
//! let shaded = [true, true, false, false, true, true];
//! assert_eq!(flood(0, |i| shaded[i], &nb), [0, 1, 4, 5]);
//! assert!(flood(2, |i| shaded[i], &nb).is_empty());
//! ```

/// The orthogonal neighbours of every cell of a `rows` by `cols` grid,
/// cells counted row-major, each in the order up, left, right, down.
pub fn neighbours(rows: usize, cols: usize) -> Vec<Vec<usize>> {
    (0..rows * cols)
        .map(|i| {
            let (r, c) = (i / cols, i % cols);
            let mut n = Vec::with_capacity(4);
            if r > 0 {
                n.push(i - cols);
            }
            if c > 0 {
                n.push(i - 1);
            }
            if c + 1 < cols {
                n.push(i + 1);
            }
            if r + 1 < rows {
                n.push(i + cols);
            }
            n
        })
        .collect()
}

/// The cells connected to `start` through cells where `allowed` holds, in
/// breadth-first order starting with `start`; empty if `start` itself is
/// not allowed. Uses no recursion (G7).
pub fn flood(
    start: usize,
    allowed: impl Fn(usize) -> bool,
    neighbours: &[Vec<usize>],
) -> Vec<usize> {
    if !allowed(start) {
        return Vec::new();
    }
    flood_from(&[start], allowed, neighbours, usize::MAX)
}

/// The cells connected to `starts` through cells where `allowed` holds, in
/// breadth-first order: the starts first, then the cells they reach. The
/// starts are included whether or not they are allowed. Stops once `limit`
/// cells are found, so a caller that only needs to know whether there are
/// at least `limit` cells does no more work than that.
pub fn flood_from(
    starts: &[usize],
    allowed: impl Fn(usize) -> bool,
    neighbours: &[Vec<usize>],
    limit: usize,
) -> Vec<usize> {
    let mut seen = vec![false; neighbours.len()];
    let mut out = Vec::new();
    for &i in starts {
        if !seen[i] {
            seen[i] = true;
            out.push(i);
        }
    }
    let mut at = 0;
    while at < out.len() && out.len() < limit {
        for &j in &neighbours[out[at]] {
            if !seen[j] && allowed(j) {
                seen[j] = true;
                out.push(j);
            }
        }
        at += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floods_stop_at_the_limit_and_keep_breadth_first_order() {
        let nb = neighbours(3, 3);
        assert_eq!(nb[4], [1, 3, 5, 7]);
        assert_eq!(flood(4, |_| true, &nb), [4, 1, 3, 5, 7, 0, 2, 6, 8]);
        assert_eq!(flood_from(&[0, 8, 0], |_| true, &nb, 4), [0, 8, 1, 3]);
        assert_eq!(flood_from(&[4], |_| false, &nb, 9), [4]);
        assert!(neighbours(0, 5).is_empty());
    }
}
