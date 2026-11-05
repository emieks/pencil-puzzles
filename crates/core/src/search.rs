//! The generator loop shared by every genre (`docs/design/framework.md`).
//!
//! A genre builds candidates and proposes changes to them. The loop screens
//! each candidate with the human solver capped at the requested level, keeps
//! changes that do not make the candidate worse, and certifies the first
//! candidate at exactly the requested level. Seeds, attempt counts and every
//! choice are deterministic.
use crate::{Budget, BudgetExceeded, Difficulty, Rating, SplitMix64};

/// How a candidate fared when solved with techniques up to the cap.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Solved: the level of the hardest technique used and the total proof
    /// size. Because easier techniques are always tried first, the level is
    /// the puzzle's exact level. Genres may use the proof size to prefer
    /// candidates; the loop compares levels only.
    Solved(Difficulty, u64),
    /// Stuck with these variables unresolved: too hard for the cap, or
    /// not unique.
    Stuck(Vec<usize>),
}

impl Outcome {
    /// Larger is closer to a puzzle at the cap: any solved candidate beats
    /// any stuck one, fewer unresolved variables beat more, and a harder
    /// solved level beats an easier one.
    pub fn rank(&self) -> (bool, i64) {
        match self {
            Self::Solved(level, _) => (true, *level as i64),
            Self::Stuck(unresolved) => (false, -(unresolved.len() as i64)),
        }
    }
}

/// What to do next with the current candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Move<C> {
    /// Screen this changed candidate and keep it unless it is worse.
    Try(C),
    /// Continue from this candidate even if it is worse, such as putting
    /// clues back to escape a dead end.
    Force(C),
    /// Abandon the current candidate and build a new one.
    Restart,
}

/// A genre's way of building and changing candidates.
pub trait Construct {
    /// A candidate puzzle with its solution and any search memory.
    type Candidate: Clone;
    /// The genre's puzzle.
    type Puzzle;

    /// A fresh candidate.
    fn build(
        &self,
        rng: &mut SplitMix64,
        budget: &Budget,
    ) -> Result<Self::Candidate, BudgetExceeded>;

    /// The puzzle a candidate presents.
    fn puzzle(&self, candidate: &Self::Candidate) -> Self::Puzzle;

    /// Solve with techniques up to `cap`.
    fn screen(
        &self,
        puzzle: &Self::Puzzle,
        cap: Difficulty,
        budget: &Budget,
    ) -> Result<Outcome, BudgetExceeded>;

    /// Propose the next change. The candidate may update its own search
    /// memory, which is kept even if the proposal is rejected. Unresolved
    /// variables in a stuck outcome show where the difficulty is. Keep it
    /// cheap, about linear in the grid; it has no budget. Expensive work
    /// belongs in `build` or `screen`.
    fn change(
        &self,
        candidate: &mut Self::Candidate,
        outcome: &Outcome,
        cap: Difficulty,
        rng: &mut SplitMix64,
    ) -> Move<Self::Candidate>;

    /// Whether a tried change is kept. By default it is kept unless its
    /// outcome ranks worse ([`Outcome::rank`]).
    fn keep(&self, before: &Outcome, after: &Outcome) -> bool {
        after.rank() >= before.rank()
    }

    /// Whether a candidate at the requested level is ready to return.
    /// Genres that keep refining a puzzle at its level return false until done.
    fn finished(&self, _candidate: &Self::Candidate) -> bool {
        true
    }

    /// Consecutive rejected proposals before starting over.
    fn patience(&self) -> u32;

    /// Full rating with an independent uniqueness check, or `None` if the
    /// candidate fails certification. The loop then starts over. A candidate
    /// that screened as solved should never fail: sound techniques that
    /// solve a puzzle prove it unique.
    fn certify(
        &self,
        puzzle: &Self::Puzzle,
        budget: &Budget,
    ) -> Result<Option<Rating>, BudgetExceeded>;
}

/// A certified puzzle.
#[derive(Clone, Debug)]
pub struct Found<C> {
    /// The candidate, including its solution.
    pub candidate: C,
    /// Its full rating.
    pub rating: Rating,
    /// Candidates screened, including this one: every fresh build, every
    /// tried or forced change.
    pub attempts: u32,
}

/// Why no puzzle was found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchError {
    /// The work budget ran out.
    BudgetExceeded,
    /// The attempt limit was reached; carries the number of attempts.
    AttemptsExhausted(u32),
}

impl From<BudgetExceeded> for SearchError {
    fn from(_: BudgetExceeded) -> Self {
        Self::BudgetExceeded
    }
}

/// Find a puzzle at exactly `target`, or at any level up to Expert when
/// `target` is `None`. Every screened candidate counts as one attempt.
pub fn search<G: Construct>(
    genre: &G,
    target: Option<Difficulty>,
    seed: u64,
    max_attempts: u32,
    budget: &Budget,
) -> Result<Found<G::Candidate>, SearchError> {
    let cap = target.unwrap_or(Difficulty::Expert);
    let wanted = |level: Difficulty| target.is_none_or(|t| level == t);
    let mut rng = SplitMix64::new(seed);
    let mut attempts = 0;
    let screen = |candidate: &G::Candidate, attempts: &mut u32| {
        if *attempts >= max_attempts {
            return Err(SearchError::AttemptsExhausted(*attempts));
        }
        *attempts += 1;
        Ok(genre.screen(&genre.puzzle(candidate), cap, budget)?)
    };
    'restart: loop {
        let mut candidate = genre.build(&mut rng, budget)?;
        let mut outcome = screen(&candidate, &mut attempts)?;
        let mut rejected = 0;
        loop {
            if let Outcome::Solved(level, _) = outcome
                && wanted(level)
                && genre.finished(&candidate)
            {
                match genre.certify(&genre.puzzle(&candidate), budget)? {
                    Some(rating) if wanted(rating.level) => {
                        return Ok(Found {
                            candidate,
                            rating,
                            attempts,
                        });
                    }
                    _ => continue 'restart,
                }
            }
            match genre.change(&mut candidate, &outcome, cap, &mut rng) {
                Move::Restart => continue 'restart,
                Move::Force(next) => {
                    outcome = screen(&next, &mut attempts)?;
                    candidate = next;
                    rejected = 0;
                }
                Move::Try(next) => {
                    let next_outcome = screen(&next, &mut attempts)?;
                    if genre.keep(&outcome, &next_outcome) {
                        candidate = next;
                        outcome = next_outcome;
                        rejected = 0;
                    } else {
                        rejected += 1;
                        if rejected >= genre.patience() {
                            continue 'restart;
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A toy genre: a number that is "solved at level n" when below 4, and
    /// stuck otherwise. Changes step it by one.
    struct Counter;

    impl Construct for Counter {
        type Candidate = u8;
        type Puzzle = u8;
        fn build(&self, rng: &mut SplitMix64, _: &Budget) -> Result<u8, BudgetExceeded> {
            Ok(rng.below(6) as u8)
        }
        fn puzzle(&self, c: &u8) -> u8 {
            *c
        }
        fn screen(&self, p: &u8, cap: Difficulty, _: &Budget) -> Result<Outcome, BudgetExceeded> {
            Ok(match p {
                n @ 0..=3 if Difficulty::ALL[*n as usize] <= cap => {
                    Outcome::Solved(Difficulty::ALL[*n as usize], 0)
                }
                n => Outcome::Stuck(vec![0; usize::from(*n)]),
            })
        }
        fn change(
            &self,
            c: &mut u8,
            outcome: &Outcome,
            _: Difficulty,
            _: &mut SplitMix64,
        ) -> Move<u8> {
            match outcome {
                Outcome::Solved(..) => Move::Try(*c + 1),
                Outcome::Stuck(_) => Move::Try(c.saturating_sub(1)),
            }
        }
        fn patience(&self) -> u32 {
            3
        }
        fn certify(&self, p: &u8, _: &Budget) -> Result<Option<Rating>, BudgetExceeded> {
            Ok(Some(Rating::new(Difficulty::ALL[usize::from(*p)], 1)))
        }
    }

    /// A toy genre whose candidate is (value, counter), always built as
    /// (0, 0), with scripted changes. Values 0-3 solve at that level.
    type Change = fn(&mut (u8, u32), &Outcome) -> Move<(u8, u32)>;

    struct Script {
        change: Change,
        finished: fn(&(u8, u32)) -> bool,
        certifies: bool,
        patience: u32,
    }

    impl Construct for Script {
        type Candidate = (u8, u32);
        type Puzzle = u8;
        fn build(&self, _: &mut SplitMix64, _: &Budget) -> Result<(u8, u32), BudgetExceeded> {
            Ok((0, 0))
        }
        fn puzzle(&self, c: &(u8, u32)) -> u8 {
            c.0
        }
        fn screen(&self, p: &u8, cap: Difficulty, b: &Budget) -> Result<Outcome, BudgetExceeded> {
            Counter.screen(p, cap, b)
        }
        fn change(
            &self,
            c: &mut (u8, u32),
            outcome: &Outcome,
            _: Difficulty,
            _: &mut SplitMix64,
        ) -> Move<(u8, u32)> {
            (self.change)(c, outcome)
        }
        fn finished(&self, c: &(u8, u32)) -> bool {
            (self.finished)(c)
        }
        fn patience(&self) -> u32 {
            self.patience
        }
        fn certify(&self, p: &u8, b: &Budget) -> Result<Option<Rating>, BudgetExceeded> {
            Ok(if self.certifies {
                Counter.certify(p, b)?
            } else {
                None
            })
        }
    }

    fn script(change: Change) -> Script {
        Script {
            change,
            finished: |_| true,
            certifies: true,
            patience: u32::MAX,
        }
    }

    fn run(genre: &Script, target: Difficulty, max: u32) -> Result<Found<(u8, u32)>, SearchError> {
        search(genre, Some(target), 0, max, &Budget::unlimited())
    }

    #[test]
    fn forced_changes_are_kept_even_when_worse() {
        // Easy (0) is forced to stuck (5), from which Hard (2) is reached.
        let genre = script(|c, outcome| match outcome {
            Outcome::Solved(..) => Move::Force((5, c.1)),
            Outcome::Stuck(_) => Move::Try((2, c.1)),
        });
        let found = run(&genre, Difficulty::Hard, 10).unwrap();
        assert_eq!(found.candidate.0, 2);
        assert_eq!(found.attempts, 3);
    }

    #[test]
    fn worse_tries_are_rejected_until_patience_restarts() {
        // Every try is worse; two rejections restart, so the 7th screen is
        // a fresh build and the 8th is refused.
        let mut genre = script(|c, _| Move::Try((5, c.1)));
        genre.patience = 2;
        assert_eq!(
            run(&genre, Difficulty::Hard, 7).unwrap_err(),
            SearchError::AttemptsExhausted(7)
        );
    }

    #[test]
    fn failed_certification_and_restart_moves_start_over() {
        let mut genre = script(|_, _| Move::Restart);
        genre.certifies = false;
        assert_eq!(
            run(&genre, Difficulty::Easy, 4).unwrap_err(),
            SearchError::AttemptsExhausted(4)
        );
        let genre = script(|_, _| Move::Restart);
        assert_eq!(
            run(&genre, Difficulty::Hard, 3).unwrap_err(),
            SearchError::AttemptsExhausted(3)
        );
    }

    #[test]
    fn unfinished_candidates_keep_changing_at_the_target() {
        let mut genre = script(|c, _| Move::Try((c.0, c.1 + 1)));
        genre.finished = |c| c.1 >= 2;
        let found = run(&genre, Difficulty::Easy, 10).unwrap();
        assert_eq!(found.candidate, (0, 2));
        assert_eq!(found.attempts, 3);
    }

    #[test]
    fn fewer_unresolved_variables_rank_higher() {
        assert!(Outcome::Stuck(vec![1]).rank() > Outcome::Stuck(vec![1, 2]).rank());
        assert!(Outcome::Solved(Difficulty::Easy, 9).rank() > Outcome::Stuck(Vec::new()).rank());
        assert!(
            Outcome::Solved(Difficulty::Hard, 0).rank()
                > Outcome::Solved(Difficulty::Easy, 9).rank()
        );
    }

    #[test]
    fn finds_the_target_and_is_deterministic() {
        let budget = Budget::unlimited();
        let found = search(&Counter, Some(Difficulty::Hard), 7, 100, &budget).unwrap();
        assert_eq!(found.candidate, 2);
        let again = search(&Counter, Some(Difficulty::Hard), 7, 100, &budget).unwrap();
        assert_eq!(again.attempts, found.attempts);
        let exact = search(&Counter, Some(Difficulty::Hard), 7, found.attempts, &budget).unwrap();
        assert_eq!(exact.candidate, 2);
        assert_eq!(
            search(&Counter, Some(Difficulty::Hard), 7, 0, &budget).unwrap_err(),
            SearchError::AttemptsExhausted(0)
        );
    }
}
