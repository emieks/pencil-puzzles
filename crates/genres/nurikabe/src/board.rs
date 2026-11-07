//! Rule instances, bounded propagation and complete binary search, on
//! bitboards (`bits`).
//!
//! Propagation narrows each rule instance: a cell loses a value when
//! writing that value in would break the instance. The instances are
//! narrowed until none changes anything, so the result is the unique
//! fixpoint and does not depend on the order in which cells are tested.
//! That lets the tests skip cells whose value cannot affect the instance.
use crate::bits::{self, Bits, bit, count, first};
use crate::*;
use std::rc::Rc;

pub(crate) const LAND: u16 = 1;
pub(crate) const SEA: u16 = 2;

#[derive(Clone, Debug)]
pub(crate) enum Kind {
    /// A numbered cell and its number.
    Island(usize, u32),
    Numbers,
    Sea,
    Pool,
}

#[derive(Clone, Debug)]
pub(crate) struct Instance {
    pub kind: Kind,
    /// The cells the instance can involve.
    pub scope: Bits,
    pub place: Place,
    pub rule: u32,
}

pub(crate) struct Topology {
    pub grid: bits::Grid,
    /// Numbered cells.
    pub clues: Bits,
    pub instances: Vec<Instance>,
}

impl Topology {
    pub fn new(p: &Puzzle) -> Self {
        let grid = bits::Grid::new(p.shape());
        let cols = grid.cols;
        let mut clues = 0;
        let mut instances = Vec::new();
        for (i, &n) in p.clues().iter().enumerate().filter(|(_, n)| **n > 0) {
            clues |= bit(i);
            instances.push(Instance {
                kind: Kind::Island(i, n),
                scope: grid.ball(i, n as usize),
                place: cell(i, cols),
                rule: 1,
            });
        }
        for (kind, rule) in [(Kind::Numbers, 2), (Kind::Sea, 3)] {
            instances.push(Instance {
                kind,
                scope: grid.all,
                place: Place::Board,
                rule,
            });
        }
        for i in bits::cells(grid.pools(grid.all)) {
            instances.push(Instance {
                kind: Kind::Pool,
                scope: grid.block(i),
                place: cell(i, cols),
                rule: 4,
            });
        }
        Self {
            grid,
            clues,
            instances,
        }
    }
}

pub(crate) fn cell(i: usize, cols: usize) -> Place {
    Place::Cell {
        row: (i / cols) as u32,
        col: (i % cols) as u32,
    }
}

/// The candidates of every cell: `land` holds the cells that cannot be sea,
/// `sea` the cells that cannot be land. A cell in both has no value left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct State {
    pub land: Bits,
    pub sea: Bits,
}

impl State {
    /// The state of candidate masks (`LAND`, `SEA`, both or neither).
    pub fn from_masks(masks: &[u16]) -> Self {
        let mut s = Self { land: 0, sea: 0 };
        for (i, &m) in masks.iter().enumerate() {
            if m & SEA == 0 {
                s.land |= bit(i);
            }
            if m & LAND == 0 {
                s.sea |= bit(i);
            }
        }
        s
    }

    /// Whether some cell has no value left.
    pub fn dead(self) -> bool {
        self.land & self.sea != 0
    }

    /// Cells with both values.
    pub fn open(self, all: Bits) -> Bits {
        all & !self.land & !self.sea
    }

    /// Cells that may still be land.
    pub fn possible_land(self, all: Bits) -> Bits {
        all & !self.sea
    }

    /// Cells that may still be sea.
    pub fn possible_sea(self, all: Bits) -> Bits {
        all & !self.land
    }

    /// The mask of one cell.
    pub fn mask(self, i: usize) -> u16 {
        let b = bit(i);
        (if self.sea & b == 0 { LAND } else { 0 }) | (if self.land & b == 0 { SEA } else { 0 })
    }
}

#[derive(Clone)]
pub(crate) struct Board<'a> {
    pub puzzle: &'a Puzzle,
    pub budget: &'a Budget,
    pub topology: Rc<Topology>,
    pub state: State,
}

impl<'a> Board<'a> {
    pub fn new(puzzle: &'a Puzzle, budget: &'a Budget) -> Self {
        let topology = Topology::new(puzzle);
        let state = State {
            land: topology.clues,
            sea: 0,
        };
        Self {
            puzzle,
            budget,
            topology: Rc::new(topology),
            state,
        }
    }

    pub fn spend(&self) -> Result<(), BudgetExceeded> {
        if self.budget.spend(1) {
            Ok(())
        } else {
            Err(BudgetExceeded)
        }
    }

    fn grid(&self) -> &bits::Grid {
        &self.topology.grid
    }

    /// Set one cell's candidates.
    #[cfg(test)]
    pub fn set_mask(&mut self, i: usize, mask: u16) {
        let b = bit(i);
        self.state.land &= !b;
        self.state.sea &= !b;
        if mask & SEA == 0 {
            self.state.land |= b;
        }
        if mask & LAND == 0 {
            self.state.sea |= b;
        }
    }

    /// The smallest and largest size an island's land can still have.
    fn island_bounds(&self, k: usize, s: State) -> (usize, usize) {
        let instance = &self.topology.instances[k];
        let Kind::Island(origin, _) = instance.kind else {
            unreachable!()
        };
        let g = self.grid();
        let fixed = s.land & !s.sea & instance.scope;
        let possible = s.possible_land(instance.scope);
        (
            count(g.flood(bit(origin), fixed)),
            count(g.flood(bit(origin), possible)),
        )
    }

    /// Whether instance `k` is broken: some cell of its scope has no value,
    /// or no completion of the candidates can satisfy it.
    pub fn broken(&self, k: usize, s: State) -> bool {
        let instance = &self.topology.instances[k];
        if s.land & s.sea & instance.scope != 0 {
            return true;
        }
        let g = self.grid();
        match instance.kind {
            Kind::Island(_, n) => {
                let (min, max) = self.island_bounds(k, s);
                min > n as usize || max < n as usize
            }
            Kind::Pool => instance.scope & s.sea == instance.scope,
            Kind::Sea => {
                let possible = s.possible_sea(g.all);
                if possible == 0 {
                    return true;
                }
                let required = s.sea;
                required != 0 && g.flood(bit(first(required)), possible) & required != required
            }
            Kind::Numbers => {
                let fixed = s.land;
                let possible = s.possible_land(g.all);
                let mut remaining = fixed;
                while remaining != 0 {
                    let component = g.flood(bit(first(remaining)), fixed);
                    remaining &= !component;
                    if count(component & self.topology.clues) > 1
                        || g.flood(component, possible) & self.topology.clues == 0
                    {
                        return true;
                    }
                }
                false
            }
        }
    }

    /// Narrow instance `k`: remove every value whose writing in would break
    /// it. Only cells whose value can affect the instance are tested.
    fn narrow(&mut self, k: usize) {
        let s = self.state;
        let instance = &self.topology.instances[k];
        let scope = instance.scope;
        if self.broken(k, s) {
            self.state.land |= scope;
            self.state.sea |= scope;
            return;
        }
        let g = self.grid();
        let clues = self.topology.clues;
        let open = s.open(scope);
        // Values that go: cells that cannot be land, cells that cannot be sea.
        let mut no_land = 0;
        let mut no_sea = 0;
        match instance.kind {
            Kind::Island(origin, n) => {
                let n = n as usize;
                let fixed = s.land & scope;
                let possible = s.possible_land(scope);
                let fixed_component = g.flood(bit(origin), fixed);
                let reach = g.flood(bit(origin), possible);
                // Land next to the fixed island may make it too large.
                for i in bits::cells(open & g.adjacent(fixed_component)) {
                    if count(g.flood(bit(origin), fixed | bit(i))) > n {
                        no_land |= bit(i);
                    }
                }
                // Sea within reach may leave the island too little room. A
                // cell with one neighbour in the reach cannot cut it.
                let size = count(reach);
                for i in bits::cells(open & reach) {
                    let broken = if size - 1 < n {
                        true
                    } else if count(g.adjacent(bit(i)) & reach) == 1 {
                        false
                    } else {
                        count(g.flood(bit(origin), reach & !bit(i))) < n
                    };
                    if broken {
                        no_sea |= bit(i);
                    }
                }
            }
            Kind::Pool => {
                for i in bits::cells(open) {
                    if scope & !bit(i) & !s.sea == 0 {
                        no_sea |= bit(i);
                    }
                }
            }
            Kind::Sea => {
                let possible = s.possible_sea(g.all);
                let required = s.sea;
                if count(possible) == 1 {
                    // The only cell that can be sea must be.
                    no_land |= possible & open;
                }
                if required != 0 {
                    let reach = g.flood(bit(first(required)), possible);
                    // Sea outside the connected area could never join it.
                    no_sea |= open & !reach;
                    // Land inside it may cut it. A cell with one neighbour
                    // in the area cannot.
                    for i in bits::cells(open & reach) {
                        if count(g.adjacent(bit(i)) & reach) > 1
                            && g.flood(bit(first(required)), reach & !bit(i)) & required != required
                        {
                            no_land |= bit(i);
                        }
                    }
                }
            }
            Kind::Numbers => {
                let fixed = s.land;
                let possible = s.possible_land(g.all);
                // Fixed land components with their number of clues, and the
                // areas of possible land, each with whether it holds a clue.
                let mut components = Vec::new();
                let mut remaining = fixed;
                while remaining != 0 {
                    let component = g.flood(bit(first(remaining)), fixed);
                    remaining &= !component;
                    components.push((component, count(component & clues)));
                }
                let mut clueless_areas = 0;
                let mut remaining = possible;
                while remaining != 0 {
                    let area = g.flood(bit(first(remaining)), possible);
                    remaining &= !area;
                    if area & clues == 0 {
                        clueless_areas |= area;
                    }
                }
                for i in bits::cells(open) {
                    // Land here joins the fixed components around it; the
                    // result may hold two clues or be unable to reach one.
                    let around = g.adjacent(bit(i));
                    let numbers: usize = components
                        .iter()
                        .filter(|(component, _)| component & around != 0)
                        .map(|&(_, n)| n)
                        .sum();
                    if numbers > 1 || clueless_areas & bit(i) != 0 {
                        no_land |= bit(i);
                    }
                }
                // Sea here may cut unnumbered land off from every clue. A
                // cell with one neighbour among the possible land cannot.
                for &(component, numbers) in &components {
                    if numbers > 0 {
                        continue;
                    }
                    let area = g.flood(component, possible);
                    for i in bits::cells(open & area) {
                        if count(g.adjacent(bit(i)) & possible) > 1
                            && g.flood(component, possible & !bit(i)) & clues == 0
                        {
                            no_sea |= bit(i);
                        }
                    }
                }
            }
        }
        self.state.sea |= no_land;
        self.state.land |= no_sea;
    }

    pub fn answer(&self) -> Answer {
        let mut a = Answer::new(self.puzzle.shape());
        let cols = self.puzzle.shape().cols();
        for i in 0..self.puzzle.shape().cells() {
            a.set(
                i / cols,
                i % cols,
                if self.state.mask(i) == SEA {
                    Mark::Shaded
                } else {
                    Mark::Unshaded
                },
            );
        }
        a
    }

    /// Whether some cell has no candidate left.
    pub fn contradictory(&self) -> bool {
        self.state.dead()
    }

    /// Narrow every instance until nothing changes. Returns whether the
    /// candidates are still consistent.
    pub fn propagate(&mut self) -> Result<bool, BudgetExceeded> {
        loop {
            if self.contradictory() {
                return Ok(false);
            }
            let before = self.state;
            for k in 0..self.topology.instances.len() {
                self.spend()?;
                self.narrow(k);
                if self.contradictory() {
                    return Ok(false);
                }
            }
            if self.state == before {
                return Ok(true);
            }
        }
    }
}

fn search(
    mut b: Board<'_>,
    limit: usize,
    found: &mut usize,
    mut visit: impl FnMut(Answer),
) -> Result<(), BudgetExceeded> {
    // Explicit recursive visitor shares output storage; recursion is at most one per cell.
    fn dfs(
        b: &mut Board<'_>,
        limit: usize,
        found: &mut usize,
        visit: &mut dyn FnMut(Answer),
    ) -> Result<(), BudgetExceeded> {
        if *found >= limit {
            return Ok(());
        }
        b.spend()?;
        if !b.propagate()? {
            return Ok(());
        }
        let open = b.state.open(b.topology.grid.all);
        if open != 0 {
            let i = bit(first(open));
            for value in [LAND, SEA] {
                let mut next = b.clone();
                if value == LAND {
                    next.state.land |= i;
                } else {
                    next.state.sea |= i;
                }
                dfs(&mut next, limit, found, visit)?;
                if *found >= limit {
                    break;
                }
            }
        } else {
            *found += 1;
            visit(b.answer());
        }
        Ok(())
    }
    dfs(&mut b, limit, found, &mut visit)
}

/// Prove zero, one or multiple solutions; budget exhaustion is an error.
pub fn solve(p: &Puzzle, budget: &Budget) -> Result<Solutions<Answer>, BudgetExceeded> {
    let mut answers = Vec::new();
    search(Board::new(p, budget), 2, &mut 0, |a| answers.push(a))?;
    Ok(match answers.len() {
        0 => Solutions::None,
        1 => Solutions::Unique(answers.remove(0)),
        _ => Solutions::Multiple(answers.remove(0), answers.remove(0)),
    })
}

/// Count solutions up to a caller-supplied limit without retaining witnesses.
pub fn count_solutions(p: &Puzzle, limit: usize, budget: &Budget) -> Result<usize, BudgetExceeded> {
    let mut n = 0;
    search(Board::new(p, budget), limit, &mut n, |_| {})?;
    Ok(n)
}

/// Report broken instances under complete or partial marks, using a deterministic budget.
pub fn check(
    p: &Puzzle,
    a: &Answer,
    budget: &Budget,
) -> Result<Vec<Violation>, Box<dyn std::error::Error + Send + Sync>> {
    if a.shape() != p.shape() {
        return Err(ParseError::new("the answer has the wrong shape").into());
    }
    let mut b = Board::new(p, budget);
    let cols = p.shape().cols();
    let mut violations = Vec::new();
    for i in 0..p.shape().cells() {
        let mark = a.get(i / cols, i % cols);
        if p.clues()[i] > 0 && mark == Mark::Shaded {
            violations.push(Violation::new(
                5,
                cell(i, cols),
                vec![("clue", p.clues()[i] as i64)],
            ));
        }
        if p.clues()[i] == 0 {
            match mark {
                Mark::Unmarked => {}
                Mark::Unshaded => b.state.land |= bit(i),
                Mark::Shaded => b.state.sea |= bit(i),
            }
        }
    }
    for (k, inst) in b.topology.instances.iter().enumerate() {
        b.spend()?;
        if b.broken(k, b.state) {
            let values = match inst.kind {
                Kind::Island(_, n) => {
                    let (min, max) = b.island_bounds(k, b.state);
                    vec![
                        ("minimum", min as i64),
                        ("maximum", max as i64),
                        ("clue", n as i64),
                    ]
                }
                Kind::Pool => vec![("shaded", 4)],
                Kind::Sea => vec![(
                    "components",
                    component_count(&b, b.state.sea & !b.state.land) as i64,
                )],
                Kind::Numbers => vec![(
                    "components",
                    component_count(&b, b.state.land & !b.state.sea) as i64,
                )],
            };
            violations.push(Violation::new(inst.rule, inst.place, values));
        }
    }
    Ok(violations)
}

/// The number of connected groups in a set of cells.
fn component_count(b: &Board<'_>, mut cells: Bits) -> usize {
    let mut n = 0;
    while cells != 0 {
        cells &= !b.topology.grid.flood(bit(first(cells)), cells);
        n += 1;
    }
    n
}
