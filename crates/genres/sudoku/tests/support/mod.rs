#![allow(dead_code)]
// Independent row-major brute force. No production propagation, search or matching.
pub fn solutions(n: usize, givens: &[u8], limit: usize) -> Vec<Vec<u8>> {
    fn valid(n: usize, box_size: usize, grid: &[u8], i: usize, v: u8) -> bool {
        let (r, c) = (i / n, i % n);
        for (j, &value) in grid.iter().enumerate().take(i) {
            if value != v {
                continue;
            }
            let (a, b) = (j / n, j % n);
            if a == r || b == c || (a / box_size == r / box_size && b / box_size == c / box_size) {
                return false;
            }
        }
        true
    }
    fn visit(
        n: usize,
        b: usize,
        givens: &[u8],
        grid: &mut Vec<u8>,
        i: usize,
        limit: usize,
        result: &mut Vec<Vec<u8>>,
    ) {
        if result.len() == limit {
            return;
        }
        if i == n * n {
            result.push(grid.clone());
            return;
        }
        for v in 1..=n as u8 {
            if givens[i] != 0 && givens[i] != v {
                continue;
            }
            if valid(n, b, grid, i, v) {
                grid[i] = v;
                visit(n, b, givens, grid, i + 1, limit, result);
                grid[i] = 0;
            }
        }
    }
    let b = match n {
        1 => 1,
        4 => 2,
        9 => 3,
        _ => panic!("test shape"),
    };
    let mut result = Vec::new();
    if limit > 0 {
        visit(n, b, givens, &mut vec![0; n * n], 0, limit, &mut result);
    }
    result
}
pub fn satisfies(n: usize, givens: &[u8], answer: &[u8]) -> bool {
    if answer.iter().any(|&v| v == 0 || v as usize > n)
        || givens.iter().zip(answer).any(|(&a, &b)| a != 0 && a != b)
    {
        return false;
    }
    solutions(n, answer, 1) == vec![answer.to_vec()]
}
