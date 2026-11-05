//! The human solve loop shared by every genre (`docs/design/framework.md`).
//!
//! A genre supplies a [`Profile`]: its techniques in policy order, each with
//! a level and a [`Kind`]. The loop always applies the first step of the
//! first technique that makes progress, so the same candidates always give
//! the same trace. Techniques must be sound and must never consult the
//! solution.
//!
//! A genre's context (puzzle, units, regions) is owned by the genre and built
//! once per solve; cloning the puzzle into it is fine.
use crate::search::Outcome;
use crate::{
    Budget, BudgetExceeded, CandidateFact, Difficulty, Literal, Metrics, PatternProof, Rating, Step,
};
use std::collections::BTreeSet;

/// Whether a technique counts as skill.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Routine consequences of written-in values; never counts as skill.
    Bookkeeping,
    /// A deduction a solver has to spot.
    Skill,
}

/// One recognized pattern and the candidates it keeps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HumanStep {
    /// Name of the pattern found, such as `hidden_single`.
    pub technique: &'static str,
    /// Level of the technique that found it.
    pub level: Difficulty,
    /// Kind of the technique that found it.
    pub kind: Kind,
    /// Every variable needed to establish the pattern, including negative premises.
    pub support: Vec<usize>,
    /// Genre-defined rule instances (rows, regions, lines) the pattern uses.
    pub units: Vec<usize>,
    /// Variables and the candidate masks they keep.
    pub changes: Vec<(usize, u16)>,
    /// Candidate snapshots the proof depends on.
    pub premises: Vec<(usize, u16)>,
    /// Earlier step indices that established those snapshots.
    pub depends_on: Vec<usize>,
    /// Size of the explanation.
    pub proof_nodes: u32,
    /// The variable and value this step writes in; it then counts as placed.
    pub places: Option<(usize, u16)>,
}

/// The candidates as a technique sees them. Techniques only read them.
#[derive(Clone, Copy, Debug)]
pub struct Candidates<'a, C> {
    /// The genre's read-only context.
    pub context: &'a C,
    /// Candidate mask for each variable.
    pub masks: &'a [u16],
    /// Variables whose value is written in (givens, player marks, found
    /// placements). Clearing their consequences is bookkeeping.
    pub placed: &'a [bool],
    /// The work budget, charged one unit per examined pattern.
    pub budget: &'a Budget,
    /// Level given to steps built here; set by the running technique.
    pub level: Difficulty,
    /// Kind given to steps built here; set by the running technique.
    pub kind: Kind,
}

impl<'a, C> Candidates<'a, C> {
    /// Candidates for running techniques outside a profile, as tests do.
    /// Steps built from them are labeled Easy skill steps.
    pub fn new(context: &'a C, masks: &'a [u16], placed: &'a [bool], budget: &'a Budget) -> Self {
        Self {
            context,
            masks,
            placed,
            budget,
            level: Difficulty::Easy,
            kind: Kind::Skill,
        }
    }

    /// Charge one unit of work.
    pub fn charge(&self) -> Result<(), BudgetExceeded> {
        if self.budget.spend(1) {
            Ok(())
        } else {
            Err(BudgetExceeded)
        }
    }

    /// Build a step from its support and changes, dropping changes that
    /// remove nothing. Returns `None` when the pattern is unproductive.
    pub fn step(
        &self,
        technique: &'static str,
        mut support: Vec<usize>,
        units: Vec<usize>,
        mut changes: Vec<(usize, u16)>,
    ) -> Option<HumanStep> {
        changes.retain(|&(i, keep)| self.masks[i] & keep != self.masks[i]);
        if changes.is_empty() {
            return None;
        }
        changes.sort_unstable();
        support.extend(changes.iter().map(|&(i, _)| i));
        support.sort_unstable();
        support.dedup();
        let premises = support.iter().map(|&i| (i, self.masks[i])).collect();
        Some(HumanStep {
            technique,
            level: self.level,
            kind: self.kind,
            proof_nodes: support.len() as u32,
            support,
            units,
            changes,
            premises,
            depends_on: Vec::new(),
            places: None,
        })
    }
}

/// A function finding every productive step of one technique, in a fixed order.
pub type Find<C> = for<'a, 'b> fn(&'a Candidates<'b, C>) -> Result<Vec<HumanStep>, BudgetExceeded>;

/// One entry of a profile.
pub struct Technique<C> {
    /// Stable identifier of the entry.
    pub id: &'static str,
    /// The lowest level cap that may use it.
    pub level: Difficulty,
    /// Whether it counts as skill.
    pub kind: Kind,
    /// Whether it tests hypotheses with [`Profile::refute`]. Such techniques
    /// are never used inside another hypothesis.
    pub probes: bool,
    /// Finds its productive steps.
    pub find: Find<C>,
}

impl<C> Technique<C> {
    /// Every productive step, labeled with this entry's level and kind.
    pub fn run(&self, candidates: &Candidates<'_, C>) -> Result<Vec<HumanStep>, BudgetExceeded> {
        let labeled = Candidates {
            level: self.level,
            kind: self.kind,
            ..*candidates
        };
        (self.find)(&labeled)
    }
}

/// A genre's techniques in policy order: bookkeeping first, then easiest to
/// hardest.
pub struct Profile<C: 'static> {
    /// Stable profile name, such as `sudoku-human`.
    pub id: &'static str,
    /// Version of the profile and its policy.
    pub version: u32,
    /// Techniques in policy order.
    pub techniques: &'static [Technique<C>],
    /// Whether the candidates already break a rule. It must never be true
    /// while some assignment within the candidates satisfies every rule,
    /// because [`Profile::refute`] treats it as proof. It may miss broken
    /// partial states (refute then finds fewer contradictions), but with one
    /// candidate everywhere it must be exactly "some rule is broken". It
    /// runs every step, so keep it cheap.
    pub broken: fn(&C, &[u16]) -> bool,
}

/// The canonical trace of one capped solve.
#[derive(Clone, Debug)]
pub struct Trace {
    /// Candidates when the loop stopped.
    pub masks: Vec<u16>,
    /// Every applied step, in order.
    pub steps: Vec<HumanStep>,
    /// Distinct productive steps of the chosen technique before each step.
    pub choices: Vec<u32>,
    /// Distinct placements offered by Easy skill techniques before each
    /// step; zero for bookkeeping. Empty unless metrics were requested.
    pub placements: Vec<u32>,
    /// Variables with one candidate before each step.
    pub fixed: Vec<u32>,
    /// Whether the candidates were solved consistently.
    pub solved: bool,
}

impl Trace {
    /// Solved at the level of the hardest step, with the total proof size, or
    /// stuck with the number of unresolved variables. A contradiction counts
    /// as stuck.
    pub fn outcome(&self) -> Outcome {
        if self.solved {
            Outcome::Solved(
                self.steps
                    .iter()
                    .map(|s| s.level)
                    .max()
                    .unwrap_or(Difficulty::Easy),
                self.steps.iter().map(|s| u64::from(s.proof_nodes)).sum(),
            )
        } else {
            Outcome::Stuck(
                (0..self.masks.len())
                    .filter(|&i| self.masks[i].count_ones() != 1)
                    .collect(),
            )
        }
    }
}

impl<C> Profile<C> {
    /// Productive steps of the first technique within `cap` that has any,
    /// keeping the first explanation of any duplicate changes.
    pub fn next(
        &self,
        candidates: &Candidates<'_, C>,
        cap: Difficulty,
    ) -> Result<Vec<HumanStep>, BudgetExceeded> {
        self.first(candidates, cap, true)
    }

    fn first(
        &self,
        candidates: &Candidates<'_, C>,
        cap: Difficulty,
        probes: bool,
    ) -> Result<Vec<HumanStep>, BudgetExceeded> {
        for technique in self
            .techniques
            .iter()
            .filter(|t| t.level <= cap && (probes || !t.probes))
        {
            let mut steps = technique.run(candidates)?;
            if !steps.is_empty() {
                let mut seen = BTreeSet::new();
                steps.retain(|step| seen.insert(step.changes.clone()));
                return Ok(steps);
            }
        }
        Ok(Vec::new())
    }

    /// Assume `variable` keeps only `keep`, then apply up to `max_steps`
    /// steps of non-probing techniques within `cap`. Returns the steps if
    /// they break a rule, which proves the assumption false. An assumed
    /// single value counts as written in, so its consequences follow as
    /// bookkeeping. `keep` must leave the variable some current candidate.
    pub fn refute(
        &self,
        candidates: &Candidates<'_, C>,
        variable: usize,
        keep: u16,
        cap: Difficulty,
        max_steps: usize,
    ) -> Result<Option<Vec<HumanStep>>, BudgetExceeded> {
        let context = candidates.context;
        let mut masks = candidates.masks.to_vec();
        let mut placed = candidates.placed.to_vec();
        debug_assert_ne!(
            masks[variable] & keep,
            0,
            "the assumed value is already ruled out"
        );
        masks[variable] &= keep;
        if masks[variable].count_ones() == 1 {
            placed[variable] = true;
        }
        let mut chain = Vec::new();
        loop {
            if (self.broken)(context, &masks) {
                return Ok(Some(chain));
            }
            if chain.len() >= max_steps {
                return Ok(None);
            }
            let view = Candidates::new(context, &masks, &placed, candidates.budget);
            let Some(step) = self.first(&view, cap, false)?.into_iter().next() else {
                return Ok(None);
            };
            for &(i, keep) in &step.changes {
                masks[i] &= keep;
            }
            if let Some((i, _)) = step.places {
                placed[i] = true;
            }
            chain.push(step);
        }
    }

    /// The "what if" technique: if a variable had one of its candidate
    /// values, non-probing techniques within `cap` would break a rule within
    /// `max_steps` steps, so that value goes. Only the shortest chains found
    /// are returned, lowest variable first; each step's support covers its
    /// whole chain. Once a chain of some length is found, later variables
    /// are only searched that far.
    pub fn contradictions(
        &self,
        candidates: &Candidates<'_, C>,
        technique: &'static str,
        cap: Difficulty,
        max_steps: usize,
    ) -> Result<Vec<HumanStep>, BudgetExceeded> {
        let mut best = max_steps;
        let mut found = Vec::new();
        for variable in 0..candidates.masks.len() {
            let mask = candidates.masks[variable];
            if mask.count_ones() < 2 {
                continue;
            }
            for bit in (0..16).map(|v| 1u16 << v).filter(|bit| mask & bit != 0) {
                candidates.charge()?;
                let Some(chain) = self.refute(candidates, variable, bit, cap, best)? else {
                    continue;
                };
                let mut support = vec![variable];
                let mut units = Vec::new();
                for step in &chain {
                    support.extend(&step.support);
                    units.extend(&step.units);
                }
                units.sort_unstable();
                units.dedup();
                let changes = vec![(variable, mask & !bit)];
                if let Some(mut step) = candidates.step(technique, support, units, changes) {
                    step.proof_nodes += chain.iter().map(|s| s.proof_nodes).sum::<u32>();
                    best = best.min(chain.len());
                    found.push((chain.len(), step));
                }
                // In a consistent state at most one value per variable falls.
                break;
            }
        }
        Ok(found
            .into_iter()
            .filter(|&(length, _)| length == best)
            .map(|(_, step)| step)
            .collect())
    }

    /// Distinct placements offered by Easy skill techniques: the direct moves
    /// a solver could make.
    pub fn placements(&self, candidates: &Candidates<'_, C>) -> Result<u32, BudgetExceeded> {
        let mut found = BTreeSet::new();
        for technique in self
            .techniques
            .iter()
            .filter(|t| t.kind == Kind::Skill && t.level == Difficulty::Easy)
        {
            for step in technique.run(candidates)? {
                found.extend(step.places);
            }
        }
        Ok(found.len() as u32)
    }

    /// Solve from `masks` using techniques up to `cap`, until solved,
    /// contradictory or stuck. `metrics` also records placements before each
    /// step, which costs extra work.
    pub fn analyze(
        &self,
        context: &C,
        mut masks: Vec<u16>,
        mut placed: Vec<bool>,
        budget: &Budget,
        cap: Difficulty,
        metrics: bool,
    ) -> Result<Trace, BudgetExceeded> {
        let mut steps = Vec::new();
        let mut choices = Vec::new();
        let mut placements = Vec::new();
        let mut fixed = Vec::new();
        let mut dependencies = vec![None; masks.len()];
        loop {
            if !budget.spend(1) {
                return Err(BudgetExceeded);
            }
            if masks.iter().all(|m| m.count_ones() == 1) || (self.broken)(context, &masks) {
                break;
            }
            let candidates = Candidates::new(context, &masks, &placed, budget);
            let found = self.next(&candidates, cap)?;
            let Some(mut step) = found.first().cloned() else {
                break;
            };
            choices.push(found.len() as u32);
            if metrics {
                placements.push(if step.kind == Kind::Bookkeeping {
                    0
                } else {
                    self.placements(&candidates)?
                });
            }
            fixed.push(masks.iter().filter(|m| m.count_ones() == 1).count() as u32);
            step.depends_on = step
                .support
                .iter()
                .filter_map(|&i| dependencies[i])
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            for &(i, keep) in &step.changes {
                masks[i] &= keep;
                dependencies[i] = Some(steps.len());
            }
            if let Some((i, _)) = step.places {
                placed[i] = true;
            }
            steps.push(step);
        }
        let solved = masks.iter().all(|m| m.count_ones() == 1) && !(self.broken)(context, &masks);
        Ok(Trace {
            masks,
            steps,
            choices,
            placements,
            fixed,
            solved,
        })
    }
}

/// The public rating of a solved trace: its level is the hardest step's,
/// its score the total proof size, and it lists the skill techniques used,
/// the number of skill steps and the recorded metrics.
pub fn rating<C>(trace: &Trace, profile: &Profile<C>, rater_version: u32) -> Rating {
    let level = trace
        .steps
        .iter()
        .map(|s| s.level)
        .max()
        .unwrap_or(Difficulty::Easy);
    let skill: Vec<_> = trace
        .steps
        .iter()
        .filter(|s| s.kind != Kind::Bookkeeping)
        .collect();
    let mut rating = Rating::new(level, rater_version);
    rating.score = trace.steps.iter().map(|s| u64::from(s.proof_nodes)).sum();
    rating.techniques = skill
        .iter()
        .map(|s| s.technique.to_string())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    rating.steps = skill.len() as u32;
    rating.metrics = Some(Metrics {
        profile: profile.id.into(),
        profile_version: profile.version,
        available_moves: trace.placements.clone(),
        productive_choices: trace.choices.clone(),
        solved_cells: trace.fixed.clone(),
    });
    rating
}

/// The public steps of a trace that started from `masks`. `literal` names
/// value `v` (a candidate bit index) of variable `i` in the genre's format.
pub fn public_steps(
    trace: &Trace,
    mut masks: Vec<u16>,
    literal: impl Fn(usize, usize) -> Literal,
) -> Vec<Step> {
    let values = |mask: u16| (0..16).filter(move |v| mask & (1 << v) != 0);
    trace
        .steps
        .iter()
        .map(|d| {
            let removed = d
                .changes
                .iter()
                .flat_map(|&(i, keep)| values(masks[i] & !keep).map(move |v| (i, v)))
                .map(|(i, v)| literal(i, v))
                .collect();
            let proof = Some(PatternProof {
                facts: d
                    .premises
                    .iter()
                    .map(|&(i, m)| CandidateFact {
                        place: literal(i, 0).place,
                        candidates: values(m).map(|v| literal(i, v).value).collect(),
                    })
                    .collect(),
                predecessors: d.depends_on.iter().map(|&i| i as u32).collect(),
                inference_nodes: d.proof_nodes,
            });
            for &(i, keep) in &d.changes {
                masks[i] &= keep;
            }
            Step {
                level: d.level,
                technique: d.technique.into(),
                removed,
                proof,
            }
        })
        .collect()
}

/// A hint: the first skill step of a trace, as a public step. Bookkeeping
/// before it is applied silently, so its removed values are those still
/// left after routine consequences.
pub fn first_skill_step(
    trace: &Trace,
    masks: Vec<u16>,
    literal: impl Fn(usize, usize) -> Literal,
) -> Option<Step> {
    public_steps(trace, masks, literal)
        .into_iter()
        .zip(&trace.steps)
        .find(|(_, step)| step.kind == Kind::Skill)
        .map(|(public, _)| public)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A toy genre like one Sudoku row: every variable has a different value.
    struct Line;

    /// The other variables' candidates without value `bit`.
    fn others(c: &Candidates<'_, Line>, i: usize, bit: u16) -> Vec<(usize, u16)> {
        (0..c.masks.len())
            .filter(|&j| j != i)
            .map(|j| (j, c.masks[j] & !bit))
            .collect()
    }

    /// Bookkeeping: a placed value leaves the other variables.
    fn clear(c: &Candidates<'_, Line>) -> Result<Vec<HumanStep>, BudgetExceeded> {
        let mut steps = Vec::new();
        for i in 0..c.masks.len() {
            c.charge()?;
            if c.placed[i] && c.masks[i].count_ones() == 1 {
                steps.extend(c.step("clear", vec![i], Vec::new(), others(c, i, c.masks[i])));
            }
        }
        Ok(steps)
    }

    /// An unplaced variable with one value is written in.
    fn naked(c: &Candidates<'_, Line>) -> Result<Vec<HumanStep>, BudgetExceeded> {
        let mut steps = Vec::new();
        for i in 0..c.masks.len() {
            c.charge()?;
            if c.placed[i] || c.masks[i].count_ones() != 1 {
                continue;
            }
            if let Some(mut step) = c.step("single", vec![i], Vec::new(), others(c, i, c.masks[i]))
            {
                step.places = Some((i, c.masks[i]));
                steps.push(step);
            }
        }
        Ok(steps)
    }

    /// Removes value 0 from variable 0, twice over (a duplicate explanation).
    fn drop_first(c: &Candidates<'_, Line>) -> Result<Vec<HumanStep>, BudgetExceeded> {
        c.charge()?;
        Ok(["drop", "drop_again"]
            .into_iter()
            .filter_map(|name| c.step(name, Vec::new(), Vec::new(), vec![(0, !1)]))
            .collect())
    }

    const fn entry(
        id: &'static str,
        level: Difficulty,
        kind: Kind,
        find: Find<Line>,
    ) -> Technique<Line> {
        Technique {
            id,
            level,
            kind,
            probes: false,
            find,
        }
    }

    static TECHNIQUES: [Technique<Line>; 3] = [
        entry("clear", Difficulty::Easy, Kind::Bookkeeping, clear),
        entry("single", Difficulty::Easy, Kind::Skill, naked),
        entry("drop", Difficulty::Medium, Kind::Skill, drop_first),
    ];

    /// Some variable is empty, or two variables share their only value.
    /// With one value everywhere this is exactly "not all different".
    fn broken(_: &Line, masks: &[u16]) -> bool {
        let singles: Vec<_> = masks.iter().filter(|m| m.count_ones() == 1).collect();
        let distinct: BTreeSet<_> = singles.iter().collect();
        masks.contains(&0) || distinct.len() < singles.len()
    }

    static PROFILE: Profile<Line> = Profile {
        id: "line",
        version: 1,
        techniques: &TECHNIQUES,
        broken,
    };

    fn run(masks: Vec<u16>, placed: Vec<bool>, cap: Difficulty) -> Trace {
        PROFILE
            .analyze(&Line, masks, placed, &Budget::unlimited(), cap, true)
            .unwrap()
    }

    fn names(trace: &Trace) -> Vec<&'static str> {
        trace.steps.iter().map(|s| s.technique).collect()
    }

    #[test]
    fn levels_cap_the_loop_and_placements_are_recorded() {
        let easy = run(vec![0b11, 0b11], vec![false; 2], Difficulty::Easy);
        assert!(!easy.solved);
        assert!(easy.steps.is_empty());
        assert_eq!(easy.outcome(), Outcome::Stuck(vec![0, 1]));
        let medium = run(vec![0b11, 0b11], vec![false; 2], Difficulty::Medium);
        assert!(medium.solved);
        assert!(matches!(
            medium.outcome(),
            Outcome::Solved(Difficulty::Medium, _)
        ));
        assert_eq!(names(&medium), ["drop", "single"]);
        assert_eq!(medium.steps[0].level, Difficulty::Medium);
        assert_eq!(medium.steps[1].level, Difficulty::Easy);
        assert_eq!(medium.steps[1].depends_on, [0]);
        assert_eq!(medium.choices, [1, 1]);
        assert_eq!(medium.placements, [0, 1]);
    }

    #[test]
    fn duplicate_changes_keep_the_first_explanation() {
        let budget = Budget::unlimited();
        let (masks, placed) = (vec![0b11, 0b11], vec![false; 2]);
        let found = PROFILE
            .next(
                &Candidates::new(&Line, &masks, &placed, &budget),
                Difficulty::Medium,
            )
            .unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].technique, "drop");
    }

    #[test]
    fn placed_values_are_cleared_as_bookkeeping_not_placed_again() {
        let trace = run(vec![0b01, 0b11], vec![true, false], Difficulty::Easy);
        assert_eq!(names(&trace), ["clear"]);
        assert_eq!(trace.steps[0].kind, Kind::Bookkeeping);
        assert_eq!(trace.placements, [0]);
        assert!(trace.solved);
    }

    #[test]
    fn contradictions_and_broken_rules_stop_the_loop() {
        // Writing in variable 0 leaves variables 1 and 2 both needing value 1.
        let broken = run(
            vec![0b0001, 0b0011, 0b0011, 0b1100],
            vec![false; 4],
            Difficulty::Easy,
        );
        assert_eq!(names(&broken), ["single"]);
        assert!(!broken.solved);
        assert_eq!(broken.outcome(), Outcome::Stuck(vec![3]));
        let complete_but_wrong = run(vec![0b01, 0b01], vec![true, true], Difficulty::Easy);
        assert!(complete_but_wrong.steps.is_empty());
        assert!(!complete_but_wrong.solved);
    }

    #[test]
    fn steps_drop_unproductive_changes_and_sort_support() {
        let budget = Budget::unlimited();
        let (masks, placed) = (vec![0b11, 0b01, 0b10], vec![false; 3]);
        let c = Candidates::new(&Line, &masks, &placed, &budget);
        let step = c
            .step("t", vec![2, 0, 2], Vec::new(), vec![(1, 0b11), (0, 0b10)])
            .unwrap();
        assert_eq!(step.changes, [(0, 0b10)]);
        assert_eq!(step.support, [0, 2]);
        assert_eq!(step.premises, [(0, 0b11), (2, 0b10)]);
        assert!(
            c.step("t", Vec::new(), Vec::new(), vec![(1, 0b01)])
                .is_none()
        );
    }

    #[test]
    fn refuting_an_assumption_returns_the_breaking_chain() {
        let budget = Budget::unlimited();
        let (masks, placed) = (vec![0b0001, 0b0011, 0b0111, 0b1100], vec![false; 4]);
        let c = Candidates::new(&Line, &masks, &placed, &budget);
        // Variable 1 = value 0 clashes with variable 0 at once.
        let chain = PROFILE.refute(&c, 1, 0b0001, Difficulty::Easy, 5).unwrap();
        assert_eq!(chain.map(|steps| steps.len()), Some(0));
        // Variable 3 = value 2 clears it from variable 2; writing in variable
        // 0 then leaves variables 1 and 2 both needing value 1.
        assert!(
            PROFILE
                .refute(&c, 3, 0b0100, Difficulty::Easy, 1)
                .unwrap()
                .is_none()
        );
        let chain = PROFILE
            .refute(&c, 3, 0b0100, Difficulty::Easy, 5)
            .unwrap()
            .unwrap();
        let names: Vec<_> = chain.iter().map(|s| s.technique).collect();
        assert_eq!(names, ["clear", "single"]);
        // Variable 3 = value 3 is consistent.
        assert!(
            PROFILE
                .refute(&c, 3, 0b1000, Difficulty::Easy, 5)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn public_steps_name_removed_values_and_facts() {
        let trace = run(vec![0b01, 0b11], vec![true, false], Difficulty::Easy);
        let literal = |i: usize, v: usize| Literal {
            place: crate::Place::Cell {
                row: 0,
                col: i as u32,
            },
            value: (v + 1).to_string(),
        };
        let steps = public_steps(&trace, vec![0b01, 0b11], literal);
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].level, Difficulty::Easy);
        assert_eq!(steps[0].removed, [literal(1, 0)]);
        let facts = &steps[0].proof.as_ref().unwrap().facts;
        assert_eq!(facts[1].candidates, ["1", "2"]);
    }

    #[test]
    fn hints_skip_bookkeeping() {
        // Bookkeeping clears variable 0's value, then variable 1 is a single.
        let trace = run(
            vec![0b001, 0b011, 0b111],
            vec![true, false, false],
            Difficulty::Easy,
        );
        assert_eq!(names(&trace)[..2], ["clear", "single"]);
        let literal = |i: usize, v: usize| Literal {
            place: crate::Place::Cell {
                row: 0,
                col: i as u32,
            },
            value: v.to_string(),
        };
        let hint = first_skill_step(&trace, vec![0b001, 0b011, 0b111], literal).unwrap();
        assert_eq!(hint.technique, "single");
        // Variable 2 lost value 0 to bookkeeping already; the hint removes value 1.
        assert_eq!(hint.removed, [literal(2, 1)]);
    }

    #[test]
    fn exhausted_budget_is_an_error() {
        assert!(
            PROFILE
                .analyze(
                    &Line,
                    vec![1, 3],
                    vec![false; 2],
                    &Budget::new(0),
                    Difficulty::Easy,
                    false
                )
                .is_err()
        );
    }
}
