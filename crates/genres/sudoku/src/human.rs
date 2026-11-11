//! Sudoku's human techniques, plugged into the shared solve loop
//! (`pencil_puzzles_core::human`). Each technique is deterministic and never
//! consults the completed answer.
//!
//! Cell sets are `u128` bitboards in row-major order; the positions of each
//! candidate digit and the peers of each cell are masks, so a technique's
//! scans are a few bit operations per pattern examined.
use crate::board::Topology;
use crate::{Budget, BudgetExceeded, Difficulty, MAX_SIZE, Puzzle, Shape};
use pencil_puzzles_core::human::{Candidates, Find, HumanStep, Kind, Profile, Technique, Trace};
use std::sync::OnceLock;

/// A set of cells, one bit per cell in row-major order.
type Cells = u128;

const fn cell(i: usize) -> Cells {
    1 << i
}

/// The cells of a set in ascending order.
fn cells(mut set: Cells) -> impl Iterator<Item = usize> {
    std::iter::from_fn(move || {
        if set == 0 {
            None
        } else {
            let i = set.trailing_zeros() as usize;
            set &= set - 1;
            Some(i)
        }
    })
}

fn count(set: Cells) -> usize {
    set.count_ones() as usize
}

pub(crate) fn initial_masks(puzzle: &Puzzle) -> Vec<u16> {
    let all = (1u16 << puzzle.shape().size()) - 1;
    puzzle
        .givens()
        .iter()
        .map(|&v| if v == 0 { all } else { 1 << (v - 1) })
        .collect()
}

/// Cells whose digit is written in before solving starts: the printed givens.
pub(crate) fn given_cells(puzzle: &Puzzle) -> Vec<bool> {
    puzzle.givens().iter().map(|&v| v != 0).collect()
}

/// What Sudoku techniques read besides candidates.
pub(crate) struct Grid {
    n: usize,
    /// Rows, columns and boxes, as cell lists and as masks.
    units: Vec<Vec<usize>>,
    unit_bits: Vec<Cells>,
    /// Line and box pairs that share at least two cells.
    pairs: Vec<(usize, usize)>,
    /// The cells sharing a unit with each cell, excluding the cell itself.
    peers: Vec<Cells>,
    /// Digit subsets of each size from 2 to 4, in ascending numeric order.
    selections: [Vec<u16>; 5],
}

impl Grid {
    /// The grid of a puzzle's shape, built once per shape.
    pub(crate) fn of(shape: Shape) -> &'static Grid {
        static GRIDS: [OnceLock<Grid>; 3] = [OnceLock::new(), OnceLock::new(), OnceLock::new()];
        GRIDS[shape.box_size() - 1].get_or_init(|| Grid::new_for(shape))
    }

    #[cfg(test)]
    pub(crate) fn new(puzzle: &Puzzle) -> Self {
        Self::new_for(puzzle.shape())
    }

    fn new_for(shape: Shape) -> Self {
        let topology = Topology::new(shape);
        let n = topology.n;
        let unit_bits: Vec<Cells> = topology
            .units
            .iter()
            .map(|u| u.iter().fold(0, |m, &i| m | cell(i)))
            .collect();
        let peers = (0..n * n)
            .map(|i| {
                unit_bits
                    .iter()
                    .filter(|&&u| u & cell(i) != 0)
                    .fold(0, |m, &u| m | u)
                    & !cell(i)
            })
            .collect();
        let mut selections: [Vec<u16>; 5] = Default::default();
        for (size, list) in selections.iter_mut().enumerate().skip(2) {
            list.extend((1u16..1u16 << n).filter(|s| s.count_ones() as usize == size));
        }
        Self {
            n,
            units: topology.units,
            unit_bits,
            pairs: topology.pairs,
            peers,
            selections,
        }
    }
}

/// A Sudoku view of a position, with the helpers techniques share.
struct Engine<'a> {
    view: &'a Candidates<'a, Grid>,
    /// Shorthand for `view.masks`.
    masks: &'a [u16],
    /// Shorthand for `view.placed`: written-in digits (givens, player marks,
    /// found singles), whose peers are cleared as bookkeeping.
    placed: &'a [bool],
    g: &'a Grid,
    /// Cells where each digit is still a candidate.
    digits: [Cells; MAX_SIZE],
}

impl<'a> Engine<'a> {
    fn new(view: &'a Candidates<'a, Grid>) -> Self {
        let mut digits = [0; MAX_SIZE];
        for (i, &mask) in view.masks.iter().enumerate() {
            let mut m = mask;
            while m != 0 {
                digits[m.trailing_zeros() as usize] |= cell(i);
                m &= m - 1;
            }
        }
        Self {
            view,
            masks: view.masks,
            placed: view.placed,
            g: view.context,
            digits,
        }
    }

    fn n(&self) -> usize {
        self.g.n
    }

    fn sees(&self, a: usize, b: usize) -> bool {
        self.g.peers[a] & cell(b) != 0
    }

    /// Cells of the set that hold the digit `bit`.
    fn holding(&self, set: Cells, bit: u16) -> Cells {
        set & self.digits[bit.trailing_zeros() as usize]
    }

    /// Remove the digit `bit` from every cell of the set that holds it.
    fn removal(&self, set: Cells, bit: u16) -> Vec<(usize, u16)> {
        cells(self.holding(set, bit))
            .map(|i| (i, self.masks[i] & !bit))
            .collect()
    }

    fn charge(&self) -> Result<(), BudgetExceeded> {
        self.view.charge()
    }

    fn charge_many(&self, operations: u64) -> Result<(), BudgetExceeded> {
        if self.view.budget.spend(operations) {
            Ok(())
        } else {
            Err(BudgetExceeded)
        }
    }

    fn step(
        &self,
        technique: &'static str,
        support: Vec<usize>,
        units: Vec<usize>,
        changes: Vec<(usize, u16)>,
    ) -> Option<HumanStep> {
        self.view.step(technique, support, units, changes)
    }

    /// Record a step; true when the technique can stop because the caller
    /// applies only the first step found.
    fn push(&self, steps: &mut Vec<HumanStep>, step: HumanStep) -> bool {
        steps.push(step);
        self.view.first_only
    }

    /// Whether a chain search can stop: the caller applies only the first
    /// step found, and one was.
    fn done(&self, steps: &[HumanStep]) -> bool {
        self.view.first_only && !steps.is_empty()
    }

    /// Write in a single-candidate cell and remove its digit from its peers.
    fn clear_peers(
        &self,
        technique: &'static str,
        cell: usize,
        mut support: Vec<usize>,
        units: Vec<usize>,
    ) -> Option<HumanStep> {
        let mask = self.masks[cell];
        let changes = self.removal(self.g.peers[cell], mask);
        support.push(cell);
        let mut step = self.step(technique, support, units, changes)?;
        step.places = Some((cell, mask));
        Some(step)
    }

    /// Bookkeeping: peers of written-in digits lose that digit.
    fn bookkeeping(&self) -> Result<Vec<HumanStep>, BudgetExceeded> {
        let mut steps = Vec::new();
        for cell in 0..self.masks.len() {
            self.charge()?;
            if self.placed[cell]
                && self.masks[cell].count_ones() == 1
                && let Some(step) =
                    self.clear_peers("peer_elimination", cell, Vec::new(), Vec::new())
                && self.push(&mut steps, step)
            {
                return Ok(steps);
            }
        }
        Ok(steps)
    }

    /// Unplaced cells left with one candidate by elimination.
    fn naked_singles(&self) -> Result<Vec<HumanStep>, BudgetExceeded> {
        let mut steps = Vec::new();
        for cell in 0..self.masks.len() {
            self.charge()?;
            if !self.placed[cell]
                && self.masks[cell].count_ones() == 1
                && let Some(step) = self.clear_peers("naked_single", cell, Vec::new(), Vec::new())
                && self.push(&mut steps, step)
            {
                return Ok(steps);
            }
        }
        Ok(steps)
    }

    fn hidden_singles(&self) -> Result<Vec<HumanStep>, BudgetExceeded> {
        let mut steps = Vec::new();
        for (unit, cells) in self.g.units.iter().enumerate() {
            for digit in 0..self.n() {
                self.charge()?;
                let bit = 1 << digit;
                let possible = self.g.unit_bits[unit] & self.digits[digit];
                if count(possible) != 1 {
                    continue;
                }
                let cell = possible.trailing_zeros() as usize;
                let step = if self.masks[cell] != bit {
                    self.step(
                        "hidden_single",
                        cells.clone(),
                        vec![unit],
                        vec![(cell, bit)],
                    )
                    .map(|mut step| {
                        step.places = Some((cell, bit));
                        step
                    })
                } else if !self.placed[cell] {
                    // Already single by elimination but also the only spot in
                    // this unit: a solver sees the hidden single first, so it
                    // is written in here and its peers cleared.
                    self.clear_peers("hidden_single", cell, cells.clone(), vec![unit])
                } else {
                    None
                };
                if let Some(step) = step
                    && self.push(&mut steps, step)
                {
                    return Ok(steps);
                }
            }
        }
        Ok(steps)
    }

    fn locked(&self) -> Result<Vec<HumanStep>, BudgetExceeded> {
        let mut steps = Vec::new();
        for &(a, b) in &self.g.pairs {
            for (source, target) in [(a, b), (b, a)] {
                for digit in 0..self.n() {
                    self.charge()?;
                    let bit = 1 << digit;
                    let possible = self.g.unit_bits[source] & self.digits[digit];
                    if count(possible) < 2 || possible & !self.g.unit_bits[target] != 0 {
                        continue;
                    }
                    let changes =
                        self.removal(self.g.unit_bits[target] & !self.g.unit_bits[source], bit);
                    if let Some(step) = self.step(
                        if source >= self.n() * 2 {
                            "pointing"
                        } else {
                            "claiming"
                        },
                        self.g.units[source].clone(),
                        vec![source, target],
                        changes,
                    ) && self.push(&mut steps, step)
                    {
                        return Ok(steps);
                    }
                }
            }
        }
        Ok(steps)
    }

    fn subsets(&self, size: u32, hidden: bool) -> Result<Vec<HumanStep>, BudgetExceeded> {
        let mut steps = Vec::new();
        let technique = match (hidden, size) {
            (false, 2) => "naked_pair",
            (false, 3) => "naked_triple",
            (false, _) => "naked_quad",
            (true, 2) => "hidden_pair",
            (true, 3) => "hidden_triple",
            (true, _) => "hidden_quad",
        };
        let n = self.n();
        for (unit, cells) in self.g.units.iter().enumerate() {
            let mut local = [0u16; MAX_SIZE];
            for (j, &i) in cells.iter().enumerate() {
                local[j] = self.masks[i];
            }
            let local = &local[..n];
            // Which digits occur in the unit at all.
            let present = local.iter().fold(0u16, |m, &x| m | x);
            // Unit positions with at least two candidates.
            let wide = local
                .iter()
                .enumerate()
                .filter(|&(_, &m)| m.count_ones() >= 2)
                .fold(0u16, |p, (j, _)| p | 1 << j);
            // Unit positions of each digit.
            let mut pos = [0u16; MAX_SIZE];
            for (j, &m) in local.iter().enumerate() {
                for (d, p) in pos.iter_mut().enumerate() {
                    if m & (1 << d) != 0 {
                        *p |= 1 << j;
                    }
                }
            }
            let chosen = |selection: u16| (0..n).filter(move |&j| selection & (1 << j) != 0);
            for &selection in &self.g.selections[size as usize] {
                self.charge()?;
                let (support, changes) = if hidden {
                    // Every selected digit must occur; a unit with no such digit is contradictory.
                    if selection & !present != 0 {
                        continue;
                    }
                    let positions = chosen(selection).fold(0u16, |p, d| p | pos[d]);
                    if positions.count_ones() != size {
                        continue;
                    }
                    (
                        cells.clone(),
                        chosen(positions)
                            .map(|j| (cells[j], local[j] & selection))
                            .collect(),
                    )
                } else {
                    if selection & !wide != 0 {
                        continue;
                    }
                    let digits = chosen(selection).fold(0u16, |mask, j| mask | local[j]);
                    if digits.count_ones() != size {
                        continue;
                    }
                    let changes = (0..n)
                        .filter(|&j| selection & (1 << j) == 0)
                        .map(|j| (cells[j], local[j] & !digits))
                        .collect();
                    (chosen(selection).map(|j| cells[j]).collect(), changes)
                };
                if let Some(step) = self.step(technique, support, vec![unit], changes)
                    && self.push(&mut steps, step)
                {
                    return Ok(steps);
                }
            }
        }
        Ok(steps)
    }

    fn fish(&self, size: u32) -> Result<Vec<HumanStep>, BudgetExceeded> {
        let n = self.n();
        let full = ((1u32 << n) - 1) as u16;
        let mut steps = Vec::new();
        for orientation in 0..2 {
            for digit in 0..n {
                let bit = 1 << digit;
                // Where the digit can go in each base line, as a mask over
                // cover lines.
                let mut lines = [0u16; MAX_SIZE];
                if orientation == 0 {
                    for (r, line) in lines.iter_mut().enumerate().take(n) {
                        *line = (self.digits[digit] >> (r * n)) as u16 & full;
                    }
                } else {
                    for i in cells(self.digits[digit]) {
                        lines[i % n] |= 1 << (i / n);
                    }
                }
                for &bases in &self.g.selections[size as usize] {
                    self.charge()?;
                    let mut covers = 0u16;
                    let mut valid = true;
                    for u in (0..n).filter(|&u| bases & (1 << u) != 0) {
                        let positions = lines[u];
                        if positions.count_ones() < 2 || positions.count_ones() > size {
                            valid = false;
                            break;
                        }
                        covers |= positions;
                    }
                    if !valid || covers.count_ones() != size {
                        continue;
                    }
                    let changes = cells(self.digits[digit])
                        .filter(|&i| {
                            let (base, cover) = if orientation == 0 {
                                (i / n, i % n)
                            } else {
                                (i % n, i / n)
                            };
                            bases & (1 << base) == 0 && covers & (1 << cover) != 0
                        })
                        .map(|i| (i, self.masks[i] & !bit))
                        .collect();
                    let units: Vec<_> = (0..n)
                        .filter(|&u| bases & (1 << u) != 0)
                        .map(|u| orientation * n + u)
                        .collect();
                    let support = units
                        .iter()
                        .flat_map(|&u| self.g.units[u].iter().copied())
                        .collect();
                    if let Some(step) = self.step(
                        match size {
                            2 => "x_wing",
                            3 => "swordfish",
                            _ => "jellyfish",
                        },
                        support,
                        units,
                        changes,
                    ) && self.push(&mut steps, step)
                    {
                        return Ok(steps);
                    }
                }
            }
        }
        Ok(steps)
    }

    /// Conjugate pairs of one digit: units where it has exactly two places.
    fn strong_links(&self, bit: u16) -> Result<Vec<(usize, usize, usize)>, BudgetExceeded> {
        let mut links = Vec::new();
        for (unit, &set) in self.g.unit_bits.iter().enumerate() {
            self.charge()?;
            let positions = self.holding(set, bit);
            if count(positions) == 2 {
                let a = positions.trailing_zeros() as usize;
                let b = (positions & (positions - 1)).trailing_zeros() as usize;
                links.push((a, b, unit));
            }
        }
        Ok(links)
    }

    /// Skyscraper, two-string kite and turbot fish: A=B-C=D on one digit, so
    /// A or D holds it, and cells seeing both lose it.
    fn turbot(&self) -> Result<Vec<HumanStep>, BudgetExceeded> {
        let n = self.n();
        let mut steps = Vec::new();
        for digit in 0..n {
            let bit = 1 << digit;
            let links = self.strong_links(bit)?;
            for (x, &(a0, b0, u)) in links.iter().enumerate() {
                for &(c0, d0, v) in &links[x + 1..] {
                    for (a, b) in [(a0, b0), (b0, a0)] {
                        for (c, d) in [(c0, d0), (d0, c0)] {
                            self.charge()?;
                            let four = [a, b, c, d];
                            if (0..4).any(|i| (i + 1..4).any(|j| four[i] == four[j]))
                                || !self.sees(b, c)
                            {
                                continue;
                            }
                            let used = cell(a) | cell(b) | cell(c) | cell(d);
                            let changes =
                                self.removal(self.g.peers[a] & self.g.peers[d] & !used, bit);
                            let kind = |w: usize| w / n;
                            let technique = if kind(u) == kind(v) && kind(u) < 2 {
                                "skyscraper"
                            } else if kind(u) < 2 && kind(v) < 2 {
                                "two_string_kite"
                            } else {
                                "turbot_fish"
                            };
                            if let Some(step) =
                                self.step(technique, four.to_vec(), vec![u, v], changes)
                                && self.push(&mut steps, step)
                            {
                                return Ok(steps);
                            }
                        }
                    }
                }
            }
        }
        Ok(steps)
    }

    /// W-wing: two unconnected {x,y} cells joined by a strong link on x through
    /// cells they see, so one of them is y.
    fn w_wing(&self) -> Result<Vec<HumanStep>, BudgetExceeded> {
        let total = self.masks.len();
        let mut steps = Vec::new();
        let mut links = Vec::new();
        for digit in 0..self.n() {
            links.push(self.strong_links(1 << digit)?);
        }
        for p in 0..total {
            let mask = self.masks[p];
            if mask.count_ones() != 2 {
                continue;
            }
            for q in p + 1..total {
                if self.masks[q] != mask || self.sees(p, q) {
                    continue;
                }
                for x in (0..self.n()).filter(|&d| mask & (1 << d) != 0) {
                    let y = mask & !(1 << x);
                    for &(a0, b0, unit) in &links[x] {
                        self.charge()?;
                        let joined = [(a0, b0), (b0, a0)].into_iter().find(|&(a, b)| {
                            ![p, q].contains(&a)
                                && ![p, q].contains(&b)
                                && self.sees(p, a)
                                && self.sees(q, b)
                        });
                        let Some((a, b)) = joined else {
                            continue;
                        };
                        let changes = self.removal(self.g.peers[p] & self.g.peers[q], y);
                        if let Some(step) =
                            self.step("w_wing", vec![p, q, a, b], vec![unit], changes)
                            && self.push(&mut steps, step)
                        {
                            return Ok(steps);
                        }
                    }
                }
            }
        }
        Ok(steps)
    }

    /// X-chain: a single-digit chain that alternates strong and weak links and
    /// starts and ends with a strong link, with at most `max_links` strong links.
    /// One end holds the digit, so cells seeing both ends lose it.
    fn x_chain(&self, max_links: usize) -> Result<Vec<HumanStep>, BudgetExceeded> {
        let total = self.masks.len();
        let mut steps = Vec::new();
        for digit in 0..self.n() {
            let bit = 1 << digit;
            let mut strong = vec![Vec::new(); total];
            for (a, b, _) in self.strong_links(bit)? {
                strong[a].push(b);
                strong[b].push(a);
            }
            // A weak link must lead on to a strong link, so only linked cells matter.
            let holders: Vec<_> = (0..total).filter(|&i| !strong[i].is_empty()).collect();
            let mut path = Vec::new();
            for &start in &holders {
                path.clear();
                path.push(start);
                self.x_chain_from(bit, max_links, &strong, &holders, &mut path, &mut steps)?;
                if self.done(&steps) {
                    return Ok(steps);
                }
            }
        }
        Ok(steps)
    }

    fn x_chain_from(
        &self,
        bit: u16,
        max_links: usize,
        strong: &[Vec<usize>],
        holders: &[usize],
        path: &mut Vec<usize>,
        steps: &mut Vec<HumanStep>,
    ) -> Result<(), BudgetExceeded> {
        let last = *path.last().unwrap();
        let on_path = path.iter().fold(0, |m, &i| m | cell(i));
        for &next in &strong[last] {
            self.charge()?;
            if on_path & cell(next) != 0 {
                continue;
            }
            path.push(next);
            let links = path.len() / 2;
            // Chains of two strong links are turbot fish, found at Hard.
            if links >= 3 && path[0] < next {
                let (a, d) = (path[0], next);
                let changes = self.removal(
                    self.g.peers[a] & self.g.peers[d] & !(on_path | cell(next)),
                    bit,
                );
                if let Some(step) = self.step("x_chain", path.clone(), Vec::new(), changes)
                    && self.push(steps, step)
                {
                    path.pop();
                    return Ok(());
                }
            }
            if links < max_links {
                let reach = self.g.peers[next] & !(on_path | cell(next));
                for &weak in holders {
                    if reach & cell(weak) == 0 {
                        continue;
                    }
                    self.charge()?;
                    path.push(weak);
                    self.x_chain_from(bit, max_links, strong, holders, path, steps)?;
                    path.pop();
                    if self.done(steps) {
                        path.pop();
                        return Ok(());
                    }
                }
            }
            path.pop();
        }
        Ok(())
    }

    /// XY-chain: bivalue cells, each seeing the next and sharing the digit
    /// passed along, whose free digits at both ends are the same z. One end is
    /// z, so cells seeing both ends lose it.
    fn xy_chain(&self, max_cells: usize) -> Result<Vec<HumanStep>, BudgetExceeded> {
        let total = self.masks.len();
        let bivalue: Vec<_> = (0..total)
            .filter(|&i| self.masks[i].count_ones() == 2)
            .collect();
        let mut steps = Vec::new();
        let mut path = Vec::new();
        for &start in &bivalue {
            for z in (0..self.n()).filter(|&d| self.masks[start] & (1 << d) != 0) {
                let z = 1u16 << z;
                path.clear();
                path.push(start);
                let carried = self.masks[start] & !z;
                self.xy_chain_from(z, carried, max_cells, &bivalue, &mut path, &mut steps)?;
                if self.done(&steps) {
                    return Ok(steps);
                }
            }
        }
        Ok(steps)
    }

    fn xy_chain_from(
        &self,
        z: u16,
        carried: u16,
        max_cells: usize,
        bivalue: &[usize],
        path: &mut Vec<usize>,
        steps: &mut Vec<HumanStep>,
    ) -> Result<(), BudgetExceeded> {
        let last = *path.last().unwrap();
        let on_path = path.iter().fold(0, |m, &i| m | cell(i));
        let reach = self.g.peers[last] & !on_path;
        for &next in bivalue {
            if reach & cell(next) == 0 || self.masks[next] & carried == 0 {
                continue;
            }
            self.charge()?;
            path.push(next);
            let free = self.masks[next] & !carried;
            // Three-cell chains are XY-wings, found at Hard.
            if free == z && path.len() >= 4 && path[0] < next {
                let (a, d) = (path[0], next);
                let changes = self.removal(
                    self.g.peers[a] & self.g.peers[d] & !(on_path | cell(next)),
                    z,
                );
                if let Some(step) = self.step("xy_chain", path.clone(), Vec::new(), changes)
                    && self.push(steps, step)
                {
                    path.pop();
                    return Ok(());
                }
            }
            if path.len() < max_cells {
                self.xy_chain_from(z, free, max_cells, bivalue, path, steps)?;
                if self.done(steps) {
                    path.pop();
                    return Ok(());
                }
            }
            path.pop();
        }
        Ok(())
    }

    fn wings(&self, xyz: bool) -> Result<Vec<HumanStep>, BudgetExceeded> {
        let mut steps = Vec::new();
        for pivot in 0..self.masks.len() {
            if self.masks[pivot].count_ones() != if xyz { 3 } else { 2 } {
                continue;
            }
            let wings: Vec<_> = cells(self.g.peers[pivot])
                .filter(|&i| self.masks[i].count_ones() == 2)
                .collect();
            for (offset, &a) in wings.iter().enumerate() {
                for &b in &wings[offset + 1..] {
                    self.charge()?;
                    let (p, x, y) = (self.masks[pivot], self.masks[a], self.masks[b]);
                    let common = x & y;
                    if common.count_ones() != 1 || (p | x | y).count_ones() != 3 {
                        continue;
                    }
                    if xyz {
                        if x & !p != 0 || y & !p != 0 || x == y {
                            continue;
                        }
                    } else if p & common != 0
                        || (p & x).count_ones() != 1
                        || (p & y).count_ones() != 1
                        || p & x == p & y
                    {
                        continue;
                    }
                    let mut seen = self.g.peers[a] & self.g.peers[b];
                    if xyz {
                        seen &= self.g.peers[pivot];
                    }
                    let changes = self.removal(seen & !(cell(pivot) | cell(a) | cell(b)), common);
                    if let Some(step) = self.step(
                        if xyz { "xyz_wing" } else { "xy_wing" },
                        vec![pivot, a, b],
                        Vec::new(),
                        changes,
                    ) && self.push(&mut steps, step)
                    {
                        return Ok(steps);
                    }
                }
            }
        }
        Ok(steps)
    }

    fn coloring(&self, max_nodes: usize, extended: bool) -> Result<Vec<HumanStep>, BudgetExceeded> {
        let total = self.masks.len();
        let mut steps = Vec::new();
        // 0 = uncolored, 1 = false, 2 = true.
        let mut colors = vec![0u8; total];
        for digit in 0..self.n() {
            let bit = 1 << digit;
            let mut edges = vec![Vec::<(usize, usize)>::new(); total];
            for (unit, &set) in self.g.unit_bits.iter().enumerate() {
                self.charge()?;
                let positions = self.holding(set, bit);
                if count(positions) == 2 {
                    let a = positions.trailing_zeros() as usize;
                    let b = (positions & (positions - 1)).trailing_zeros() as usize;
                    edges[a].push((b, unit));
                    edges[b].push((a, unit));
                }
            }
            for start in 0..total {
                self.charge()?;
                if edges[start].is_empty() {
                    continue;
                }
                colors[start] = 1;
                let mut component = vec![start];
                let mut units = 0u32;
                let mut at = 0;
                let mut contradiction = false;
                while at < component.len() && component.len() <= max_nodes {
                    let cell = component[at];
                    for &(other, unit) in &edges[cell] {
                        self.charge()?;
                        let color = 3 - colors[cell];
                        if colors[other] != 0 {
                            if colors[other] != color {
                                contradiction = true;
                            }
                        } else {
                            colors[other] = color;
                            component.push(other);
                            units |= 1 << unit;
                        }
                    }
                    at += 1;
                }
                let colored = [
                    component
                        .iter()
                        .filter(|&&c| colors[c] == 1)
                        .fold(0, |m, &c| m | cell(c)),
                    component
                        .iter()
                        .filter(|&&c| colors[c] == 2)
                        .fold(0, |m, &c| m | cell(c)),
                ];
                let skip = contradiction
                    || component.len() > max_nodes
                    || component.iter().min() != Some(&start);
                if !skip {
                    let len = component.len() as u64;
                    self.charge_many(len * (len - 1) / 2)?;
                    let invalid = [0, 1]
                        .map(|c| cells(colored[c]).any(|a| self.g.peers[a] & colored[c] != 0));
                    self.charge_many(total as u64)?;
                    let mut changes = Vec::new();
                    for i in cells(self.digits[digit]) {
                        let eliminate = match colors[i] {
                            0 => {
                                self.g.peers[i] & colored[0] != 0
                                    && self.g.peers[i] & colored[1] != 0
                            }
                            color => invalid[usize::from(color) - 1],
                        };
                        if eliminate {
                            changes.push((i, self.masks[i] & !bit));
                        }
                    }
                    let units: Vec<_> = (0..self.g.units.len())
                        .filter(|&u| units & (1 << u) != 0)
                        .collect();
                    let support = units
                        .iter()
                        .flat_map(|&u| self.g.units[u].iter().copied())
                        .collect();
                    if let Some(step) = self.step(
                        if extended {
                            "extended_coloring"
                        } else {
                            "simple_coloring"
                        },
                        support,
                        units,
                        changes,
                    ) && self.push(&mut steps, step)
                    {
                        return Ok(steps);
                    }
                }
                for &c in &component {
                    colors[c] = 0;
                }
            }
        }
        Ok(steps)
    }
}

const fn entry(
    id: &'static str,
    level: Difficulty,
    kind: Kind,
    find: Find<Grid>,
) -> Technique<Grid> {
    Technique {
        id,
        level,
        kind,
        probes: false,
        find,
    }
}

use Difficulty::{Easy, Expert, Hard, Medium};
use Kind::{Bookkeeping, Skill};

/// Policy order: bookkeeping, hidden then naked singles, then each family
/// in turn. Every family is exhausted before the next is considered.
static TECHNIQUES: [Technique<Grid>; 21] = [
    entry("peer_elimination", Easy, Bookkeeping, |p| {
        Engine::new(p).bookkeeping()
    }),
    entry("hidden_single", Easy, Skill, |p| {
        Engine::new(p).hidden_singles()
    }),
    entry("naked_single", Easy, Skill, |p| {
        Engine::new(p).naked_singles()
    }),
    entry("locked_candidates", Medium, Skill, |p| {
        Engine::new(p).locked()
    }),
    entry("naked_pair", Medium, Skill, |p| {
        Engine::new(p).subsets(2, false)
    }),
    entry("hidden_pair", Hard, Skill, |p| {
        Engine::new(p).subsets(2, true)
    }),
    entry("naked_triple", Hard, Skill, |p| {
        Engine::new(p).subsets(3, false)
    }),
    entry("hidden_triple", Hard, Skill, |p| {
        Engine::new(p).subsets(3, true)
    }),
    entry("naked_quad", Hard, Skill, |p| {
        Engine::new(p).subsets(4, false)
    }),
    entry("hidden_quad", Hard, Skill, |p| {
        Engine::new(p).subsets(4, true)
    }),
    entry("x_wing", Expert, Skill, |p| Engine::new(p).fish(2)),
    entry("swordfish", Expert, Skill, |p| Engine::new(p).fish(3)),
    entry("turbot_family", Expert, Skill, |p| Engine::new(p).turbot()),
    entry("xy_wing", Expert, Skill, |p| Engine::new(p).wings(false)),
    entry("xyz_wing", Expert, Skill, |p| Engine::new(p).wings(true)),
    entry("w_wing", Expert, Skill, |p| Engine::new(p).w_wing()),
    entry("simple_coloring", Expert, Skill, |p| {
        Engine::new(p).coloring(6, false)
    }),
    entry("jellyfish", Expert, Skill, |p| Engine::new(p).fish(4)),
    entry("extended_coloring", Expert, Skill, |p| {
        Engine::new(p).coloring(12, true)
    }),
    entry("x_chain", Expert, Skill, |p| Engine::new(p).x_chain(3)),
    entry("xy_chain", Expert, Skill, |p| Engine::new(p).xy_chain(6)),
];

/// Some cell has no candidate, or some row, column or box cannot hold every digit.
fn broken(grid: &Grid, masks: &[u16]) -> bool {
    let all = (1u16 << grid.n) - 1;
    masks.contains(&0)
        || grid
            .units
            .iter()
            .any(|unit| unit.iter().fold(0u16, |bits, &i| bits | masks[i]) != all)
}

/// The canonical Sudoku human profile.
pub(crate) static PROFILE: Profile<Grid> = Profile {
    id: "sudoku-human",
    techniques: &TECHNIQUES,
    broken,
};

pub(crate) fn next_steps(
    puzzle: &Puzzle,
    masks: &[u16],
    placed: &[bool],
    budget: &Budget,
    cap: Difficulty,
) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let grid = Grid::of(puzzle.shape());
    PROFILE.next(&Candidates::new(grid, masks, placed, budget), cap)
}

/// Solve from the printed puzzle; `metrics` also records placements.
pub(crate) fn analyze(
    puzzle: &Puzzle,
    budget: &Budget,
    cap: Difficulty,
    metrics: bool,
) -> Result<Trace, BudgetExceeded> {
    analyze_state(
        puzzle,
        initial_masks(puzzle),
        given_cells(puzzle),
        budget,
        cap,
        metrics,
    )
}

/// Analyze from a candidate state; `placed` marks digits already written in.
pub(crate) fn analyze_state(
    puzzle: &Puzzle,
    masks: Vec<u16>,
    placed: Vec<bool>,
    budget: &Budget,
    cap: Difficulty,
    metrics: bool,
) -> Result<Trace, BudgetExceeded> {
    PROFILE.analyze(
        Grid::of(puzzle.shape()),
        masks,
        placed,
        budget,
        cap,
        metrics,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use pencil_puzzles_core::SplitMix64;
    use std::collections::BTreeSet;

    static UNPLACED: [bool; 81] = [false; 81];

    fn empty(n: usize) -> Puzzle {
        Puzzle::new(Shape::new(n).unwrap(), vec![0; n * n]).unwrap()
    }

    /// A random valid grid of side n = k*k: a pattern grid with digits, rows
    /// within bands, columns within stacks, bands and stacks all shuffled.
    fn random_grid(k: usize, rng: &mut SplitMix64) -> Vec<u8> {
        let n = k * k;
        let mut perm: Vec<u8> = (1..=n as u8).collect();
        rng.shuffle(&mut perm);
        let shuffle_lines = |rng: &mut SplitMix64| {
            let mut bands: Vec<usize> = (0..k).collect();
            rng.shuffle(&mut bands);
            let mut lines = Vec::new();
            for b in bands {
                let mut within: Vec<usize> = (0..k).map(|j| b * k + j).collect();
                rng.shuffle(&mut within);
                lines.extend(within);
            }
            lines
        };
        let rows = shuffle_lines(rng);
        let cols = shuffle_lines(rng);
        (0..n * n)
            .map(|i| {
                let (r, c) = (rows[i / n], cols[i % n]);
                perm[(r * k + r / k + c) % n]
            })
            .collect()
    }

    fn random_puzzle(k: usize, rng: &mut SplitMix64) -> (Puzzle, Vec<u8>) {
        let n = k * k;
        let grid = random_grid(k, rng);
        let clues = if k == 2 {
            4 + rng.below(8) as usize
        } else {
            17 + rng.below(24) as usize
        };
        let mut order: Vec<usize> = (0..n * n).collect();
        rng.shuffle(&mut order);
        let mut givens = grid.clone();
        for &i in &order[clues..] {
            givens[i] = 0;
        }
        (Puzzle::new(Shape::new(n).unwrap(), givens).unwrap(), grid)
    }

    fn first_step(masks: &[u16], placed: &[bool]) -> HumanStep {
        let puzzle = empty(4);
        let budget = Budget::unlimited();
        next_steps(&puzzle, masks, placed, &budget, Easy).unwrap()[0].clone()
    }

    /// Screening stops each technique at its first step; the applied steps
    /// are the same as when every step is listed for the metrics.
    #[test]
    fn screening_applies_the_same_steps_as_rating() {
        let mut rng = SplitMix64::new(99);
        for trial in 0..200 {
            let k = if trial % 5 == 0 { 2 } else { 3 };
            let (puzzle, _) = random_puzzle(k, &mut rng);
            for cap in Difficulty::ALL.iter().copied().take(4) {
                let budget = Budget::unlimited();
                let full = analyze(&puzzle, &budget, cap, true).unwrap();
                let quick = analyze(&puzzle, &budget, cap, false).unwrap();
                assert_eq!(full.steps, quick.steps, "{} {cap:?}", puzzle.id());
                assert_eq!(full.masks, quick.masks);
                assert_eq!(full.solved, quick.solved);
                assert!(quick.choices.is_empty());
            }
        }
    }

    #[test]
    fn peers_and_selections_follow_the_grid() {
        let g = Grid::new(&empty(9));
        // r2c5 (cell 13) sees its row, column and box: 20 cells.
        assert_eq!(count(g.peers[13]), 20);
        assert_eq!(g.peers[13] & cell(13), 0);
        assert_ne!(g.peers[13] & cell(9), 0);
        assert_ne!(g.peers[13] & cell(76), 0);
        assert_ne!(g.peers[13] & cell(21), 0);
        assert_eq!(g.peers[13] & cell(27), 0);
        assert_eq!(g.selections[2].len(), 36);
        assert_eq!(g.selections[3].len(), 84);
        assert_eq!(g.selections[4].len(), 126);
        assert_eq!(g.selections[2][0], 0b11);
        assert_eq!(cells(cell(3) | cell(80)).collect::<Vec<_>>(), [3, 80]);
    }

    #[test]
    fn clearing_a_written_in_digit_is_bookkeeping_not_a_naked_single() {
        // r1c1 is 1; every unit through it still has 1 elsewhere, so it is
        // single only by elimination, not a hidden single.
        let mut masks = vec![15; 16];
        masks[0] = 1;
        let step = first_step(&masks, &UNPLACED[..16]);
        assert_eq!(step.technique, "naked_single");
        assert_eq!(step.places, Some((0, 1)));
        let mut placed = [false; 16];
        placed[0] = true;
        let step = first_step(&masks, &placed);
        assert_eq!(step.technique, "peer_elimination");
    }

    #[test]
    fn hidden_singles_come_before_naked_singles() {
        // r1c1 is the only 1 in row 1, and r4c4 is single by elimination.
        let mut masks = vec![15; 16];
        masks[1..4].fill(14);
        masks[15] = 8;
        let step = first_step(&masks, &UNPLACED[..16]);
        assert_eq!(step.technique, "hidden_single");
        assert_eq!(step.changes, vec![(0, 1)]);
        // Once r1c1 is written in, its peers are bookkeeping.
        masks[0] = 1;
        let mut placed = [false; 16];
        placed[0] = true;
        assert_eq!(first_step(&masks, &placed).technique, "peer_elimination");
    }

    #[test]
    fn a_cell_that_is_both_singles_counts_as_hidden() {
        // r1c1 is single by elimination and the only 1 in row 1.
        let mut masks = vec![15; 16];
        masks[0] = 1;
        masks[1..4].fill(14);
        let step = first_step(&masks, &UNPLACED[..16]);
        assert_eq!(step.technique, "hidden_single");
        assert_eq!(step.places, Some((0, 1)));
        assert!(
            step.changes
                .iter()
                .all(|&(i, keep)| i != 0 && keep & 1 == 0)
        );
    }

    #[test]
    fn a_found_single_is_placed_and_later_cleared_as_bookkeeping() {
        let mut masks = vec![15; 16];
        masks[1..4].fill(14);
        let a = analyze_state(
            &empty(4),
            masks,
            vec![false; 16],
            &Budget::unlimited(),
            Easy,
            true,
        )
        .unwrap();
        assert_eq!(a.steps[0].technique, "hidden_single");
        assert_eq!(a.steps[1].technique, "peer_elimination");
        assert_eq!(a.steps[1].places, Some((0, 1)));
        assert_eq!(a.placements[1], 0);
    }

    #[test]
    fn explicit_pair_removes_only_the_pair_digits() {
        let puzzle = empty(4);
        let budget = Budget::unlimited();
        let mut masks = vec![15; 16];
        masks[0] = 3;
        masks[1] = 3;
        let grid = Grid::new(&puzzle);
        let view = Candidates::new(&grid, &masks, &UNPLACED[..masks.len()], &budget);
        let e = Engine::new(&view);
        let steps = e.subsets(2, false).unwrap();
        let step = steps.iter().find(|s| s.units == [0]).unwrap();
        assert_eq!(step.technique, "naked_pair");
        assert_eq!(step.changes, vec![(2, 12), (3, 12)]);
    }

    #[test]
    fn xy_wing_has_three_explicit_premises() {
        let puzzle = empty(9);
        let budget = Budget::unlimited();
        let mut masks = vec![511; 81];
        masks[0] = 3;
        masks[1] = 5;
        masks[9] = 6;
        let grid = Grid::new(&puzzle);
        let view = Candidates::new(&grid, &masks, &UNPLACED[..masks.len()], &budget);
        let e = Engine::new(&view);
        let steps = e.wings(false).unwrap();
        assert!(steps.iter().any(|s| s.changes.contains(&(10, 507))));
        for s in steps {
            assert!(s.support.contains(&0));
            assert!(s.support.contains(&1));
            assert!(s.support.contains(&9));
        }
    }

    #[test]
    fn a_pattern_cannot_inherit_a_forcing_bundle() {
        let puzzle = empty(4);
        let budget = Budget::unlimited();
        let analysis = analyze(&puzzle, &budget, Expert, false).unwrap();
        assert!(!analysis.solved);
        assert!(analysis.steps.is_empty());
        assert_eq!(analysis.masks, vec![15; 16]);
    }

    #[test]
    fn budget_exhaustion_is_not_a_stalled_rating() {
        assert!(analyze(&empty(9), &Budget::new(0), Expert, false).is_err());
    }

    /// Every technique keeps a known solution in random sparse candidate
    /// states around it, so it never removes a true candidate.
    #[test]
    fn nine_by_nine_deductions_keep_a_witness_solution() {
        let puzzle = empty(9);
        let budget = Budget::unlimited();
        let witness: Vec<u16> = (0..81)
            .map(|i| {
                let (r, c) = (i / 9, i % 9);
                1 << ((r * 3 + r / 3 + c) % 9)
            })
            .collect();
        let mut random = 0x9e3779b97f4a7c15u64;
        let mut fired = BTreeSet::new();
        for _ in 0..300 {
            let masks: Vec<_> = witness
                .iter()
                .map(|&w| {
                    random ^= random << 13;
                    random ^= random >> 7;
                    random ^= random << 17;
                    let extra = (random & (random >> 9) & (random >> 18)) as u16 & 511;
                    w | if random >> 60 < 3 { 0 } else { extra }
                })
                .collect();
            let grid = Grid::new(&puzzle);
            let view = Candidates::new(&grid, &masks, &UNPLACED[..masks.len()], &budget);
            let e = Engine::new(&view);
            let mut steps = e.locked().unwrap();
            for size in [2, 3, 4] {
                steps.extend(e.subsets(size, false).unwrap());
                steps.extend(e.subsets(size, true).unwrap());
            }
            for size in [2, 3, 4] {
                steps.extend(e.fish(size).unwrap());
            }
            steps.extend(e.turbot().unwrap());
            steps.extend(e.w_wing().unwrap());
            steps.extend(e.wings(false).unwrap());
            steps.extend(e.wings(true).unwrap());
            steps.extend(e.coloring(12, true).unwrap());
            steps.extend(e.x_chain(3).unwrap());
            steps.extend(e.xy_chain(6).unwrap());
            for step in steps {
                fired.insert(step.technique);
                for &(i, keep) in &step.changes {
                    assert_ne!(
                        keep & witness[i],
                        0,
                        "{} removed a true candidate",
                        step.technique
                    );
                }
            }
        }
        for technique in [
            "skyscraper",
            "two_string_kite",
            "w_wing",
            "xy_wing",
            "x_wing",
            "x_chain",
            "xy_chain",
        ] {
            assert!(
                fired.contains(technique),
                "{technique} never fired: {fired:?}"
            );
        }
    }

    #[test]
    fn deductions_preserve_every_independently_enumerated_four_by_four_completion() {
        fn complete(cells: &mut [u8; 16], at: usize, out: &mut Vec<[u8; 16]>) {
            if at == 16 {
                out.push(*cells);
                return;
            }
            let r = at / 4;
            let c = at % 4;
            for v in 1..=4 {
                if (0..at).any(|i| {
                    cells[i] == v
                        && (i / 4 == r || i % 4 == c || (i / 4 / 2 == r / 2 && i % 4 / 2 == c / 2))
                }) {
                    continue;
                }
                cells[at] = v;
                complete(cells, at + 1, out);
            }
            cells[at] = 0;
        }
        let mut solutions = Vec::new();
        complete(&mut [0; 16], 0, &mut solutions);
        assert_eq!(solutions.len(), 288);
        let puzzle = empty(4);
        let budget = Budget::unlimited();
        let mut random = 0x12345678u64;
        for trial in 0..100 {
            let witness = &solutions[trial % solutions.len()];
            let masks: Vec<_> = witness
                .iter()
                .map(|&v| {
                    random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
                    ((random >> 32) as u16 & 15) | (1 << (v - 1))
                })
                .collect();
            let compatible: Vec<_> = solutions
                .iter()
                .filter(|s| {
                    s.iter()
                        .enumerate()
                        .all(|(i, &v)| masks[i] & (1 << (v - 1)) != 0)
                })
                .collect();
            let grid = Grid::new(&puzzle);
            let view = Candidates::new(&grid, &masks, &UNPLACED[..masks.len()], &budget);
            let e = Engine::new(&view);
            let mut steps = e.naked_singles().unwrap();
            steps.extend(e.hidden_singles().unwrap());
            steps.extend(e.locked().unwrap());
            for size in [2, 3, 4] {
                steps.extend(e.subsets(size, false).unwrap());
                steps.extend(e.subsets(size, true).unwrap());
                steps.extend(e.fish(size).unwrap());
            }
            steps.extend(e.turbot().unwrap());
            steps.extend(e.w_wing().unwrap());
            steps.extend(e.wings(false).unwrap());
            steps.extend(e.wings(true).unwrap());
            steps.extend(e.coloring(12, true).unwrap());
            for step in steps {
                for &(i, keep) in &step.changes {
                    for solution in &compatible {
                        assert_ne!(keep & (1 << (solution[i] - 1)), 0, "{}", step.technique);
                    }
                }
            }
        }
    }

    /// A valid grid of side n = k*k, with digits relabeled by `perm`.
    fn valid_grid(k: usize, perm: &[u8]) -> Vec<u8> {
        let n = k * k;
        (0..n * n)
            .map(|i| {
                let (r, c) = (i / n, i % n);
                perm[(r * k + r / k + c) % n]
            })
            .collect()
    }

    /// `broken` is exactly "not a solution" on full grids, by the
    /// independent answer checker, and never fires while a solution fits.
    #[test]
    fn broken_is_exact_on_full_grids_and_sound_on_partial_ones() {
        let mut rng = 0x2545f4914f6cdd1du64;
        let mut next = |m: u64| {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            rng % m
        };
        for k in [2usize, 3] {
            let n = k * k;
            let puzzle = empty(n);
            let grid = Grid::new(&puzzle);
            for trial in 0..200 {
                let mut perm: Vec<u8> = (1..=n as u8).collect();
                for i in (1..n).rev() {
                    perm.swap(i, next(i as u64 + 1) as usize);
                }
                let mut cells = valid_grid(k, &perm);
                if trial % 2 == 1 {
                    // Usually breaks the grid; sometimes harmlessly.
                    let (a, b) = (next((n * n) as u64) as usize, next((n * n) as u64) as usize);
                    cells.swap(a, b);
                }
                let mut answer = crate::Answer::new(puzzle.shape());
                for (i, &v) in cells.iter().enumerate() {
                    answer.set(i / n, i % n, v).unwrap();
                }
                let full: Vec<u16> = cells.iter().map(|&v| 1 << (v - 1)).collect();
                assert_eq!(broken(&grid, &full), !puzzle.is_solved_by(&answer));
                if trial % 2 == 0 {
                    // Widen random cells: the solution still fits.
                    let partial: Vec<u16> = full
                        .iter()
                        .map(|&m| {
                            if next(2) == 0 {
                                m | next(1 << n) as u16
                            } else {
                                m
                            }
                        })
                        .collect();
                    assert!(!broken(&grid, &partial));
                }
            }
        }
    }
}
