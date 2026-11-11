"""Integration tests run against an installed wheel, not the source directory."""
import _thread
from dataclasses import FrozenInstanceError
import threading
import time
import unittest

import pencil_puzzles as pp


class PackageTests(unittest.TestCase):
    def test_all_genres_generate_solve_rate_check_and_round_trip(self):
        for genre, shape in (("sudoku", (4, 4)), ("nurikabe", (3, 3)), ("kakurasu", (3, 4)),
                             ("fillomino", (3, 4)), ("hashi", (5, 5)), ("hitori", (4, 5)), ("kakuro", (5, 6))):
            with self.subTest(genre=genre):
                g = pp.generate(genre, shape, difficulty="easy", seed=42)
                self.assertEqual(g.rating.level, pp.Difficulty.EASY)
                self.assertEqual(g.puzzle.shape, shape)
                self.assertEqual(pp.solve(g.puzzle), g.solution)
                self.assertEqual(pp.rate(g.puzzle), g.rating)
                self.assertEqual(pp.check(g.puzzle, g.solution), [])
                self.assertIsNone(pp.hint(g.puzzle, g.solution))
                self.assertEqual(pp.Puzzle.from_id(g.puzzle.id), g.puzzle)
                self.assertEqual(pp.Puzzle.from_text(g.puzzle.to_text()), g.puzzle)
                self.assertEqual(g.solution.shape, shape)
                self.assertIn(genre, pp.genres())

    def test_standard_sizes_and_random_seeds(self):
        for genre, shape in (("sudoku", (9, 9)), ("nurikabe", (10, 10)), ("kakurasu", (7, 7)),
                             ("fillomino", (10, 10)), ("hashi", (10, 10)), ("hitori", (8, 8)),
                             ("kakuro", (10, 10))):
            with self.subTest(genre=genre):
                self.assertEqual(pp.generate(genre, difficulty="easy").puzzle.shape, shape)
        first = pp.generate("sudoku", (4, 4))
        self.assertEqual(pp.generate("sudoku", (4, 4), seed=first.seed), first)
        self.assertEqual(len({pp.generate("sudoku", (4, 4)).seed for _ in range(5)}), 5)

    def test_print_shows_the_grid(self):
        puzzle = pp.Puzzle.from_text("genre: sudoku\ngrid:\n1..4\n.4..\n..4.\n4..1")
        self.assertEqual(str(puzzle), "1 . | . 4\n. 4 | . .\n----+----\n. . | 4 .\n4 . | . 1")
        for genre, shape in (("nurikabe", (3, 3)), ("kakurasu", (3, 4)), ("fillomino", (3, 4)),
                             ("hashi", (5, 5)), ("hitori", (4, 5)), ("kakuro", (5, 6))):
            with self.subTest(genre=genre):
                text = str(pp.generate(genre, shape, difficulty="easy", seed=42).puzzle)
                self.assertTrue(text.strip())
                self.assertNotIn("genre:", text)

    def test_clues_and_deprecated_names(self):
        sudoku = pp.Puzzle.from_text("genre: sudoku\ngrid:\n1..4\n.4..\n..4.\n4..1")
        self.assertEqual(sudoku.clues[0], (1, 0, 0, 4))
        kakurasu = pp.Puzzle.from_id("kakurasu:2x2:1.2/1.2")
        self.assertEqual(kakurasu.clues, pp.KakurasuClues((1, 2), (1, 2)))
        self.assertEqual(kakurasu.clues.rows, (1, 2))
        hitori = pp.Puzzle.from_text("genre: hitori\ngrid:\n11\n12")
        kakuro = pp.Puzzle.from_text("genre: kakuro\ngrid:\n# 4/ 6/\n/3 . .\n/7 . .")
        for puzzle, old, value in ((sudoku, "givens", sudoku.clues), (hitori, "numbers", hitori.clues),
                                   (kakuro, "cells", kakuro.clues),
                                   (kakurasu, "row_clues", (1, 2)), (kakurasu, "col_clues", (1, 2))):
            with self.subTest(old=old), self.assertWarns(DeprecationWarning):
                self.assertEqual(getattr(puzzle, old), value)

    def test_fillomino_numbers_and_two_digit_sizes(self):
        puzzle = pp.Puzzle.from_text("genre: fillomino\ngrid:\n2.\n..\n.1")
        self.assertIsInstance(puzzle, pp.FillominoPuzzle)
        self.assertEqual(puzzle.clues, ((2, 0), (0, 0), (0, 1)))
        solution = pp.solve(puzzle)
        self.assertEqual(solution.cells, ((2, 2), (3, 3), (3, 1)))
        partial = pp.Solution("fillomino", ((2, 0), (0, 0), (0, 1)))
        self.assertEqual(pp.hint(puzzle, partial).level, pp.Difficulty.EASY)
        wrong = pp.Solution("fillomino", ((2, 2), (3, 3), (1, 1)))
        self.assertEqual({v.rule for v in pp.check(puzzle, wrong)}, {1})
        big = pp.Puzzle.from_id("fillomino:1x10:-10i")
        self.assertEqual(pp.solve(big).cells, ((10,) * 10,))

    def test_hashi_bridges_steps_and_finished_checks(self):
        puzzle = pp.Puzzle.from_text("genre: hashi\ngrid:\n2.2\n...\n1.1")
        self.assertIsInstance(puzzle, pp.HashiPuzzle)
        self.assertEqual(puzzle.clues, ((2, 0, 2), (0, 0, 0), (1, 0, 1)))
        solution = pp.solve(puzzle)
        self.assertEqual(solution.cells, (("2", "-", "2"), ("|", ".", "|"), ("1", ".", "1")))
        self.assertEqual(pp.check(puzzle, solution, finished=True), [])
        empty = pp.Solution("hashi", (("2", ".", "2"), (".", ".", "."), ("1", ".", "1")))
        # Undrawn bridges are undecided unless the answer is finished.
        self.assertEqual(pp.check(puzzle, empty), [])
        self.assertEqual({v.rule for v in pp.check(puzzle, empty, finished=True)}, {1, 3})
        step = pp.hint(puzzle, empty)
        self.assertTrue(all(len(r) == 5 for r in step.removed))
        crossing = pp.Puzzle.from_text("genre: hashi\ngrid:\n.1.\n1.1\n.1.")
        crossed = pp.Solution("hashi", ((".", "1", "."), ("1", "-|", "1"), (".", "1", ".")))
        self.assertIn(2, {v.rule for v in pp.check(crossing, crossed)})
        with self.assertRaises(pp.ParseError):
            pp.check(puzzle, pp.Solution("hashi", ((2, ".", 2), (".", ".", "."), (1, ".", 1))))

    def test_hitori_numbers_and_marks(self):
        puzzle = pp.Puzzle.from_text("genre: hitori\ngrid:\n11\n12")
        self.assertIsInstance(puzzle, pp.HitoriPuzzle)
        self.assertEqual(puzzle.clues, ((1, 1), (1, 2)))
        S, U = pp.Mark.SHADED, pp.Mark.UNSHADED
        self.assertEqual(pp.solve(puzzle).cells, ((S, U), (U, U)))
        self.assertEqual(pp.hint(puzzle).technique, "corner")
        wrong = pp.Solution("hitori", ((U, S), (S, U)))
        self.assertEqual({v.rule for v in pp.check(puzzle, wrong)}, {3})
        wide = pp.Puzzle.from_id("hitori:1x11:123456789ab")
        self.assertEqual(wide.clues, (tuple(range(1, 12)),))

    def test_kakuro_blocks_sums_and_digits(self):
        puzzle = pp.Puzzle.from_text("genre: kakuro\ngrid:\n# 4/ 6/\n/3 . .\n/7 . .")
        self.assertIsInstance(puzzle, pp.KakuroPuzzle)
        self.assertEqual(puzzle.clues, (((0, 0), (4, 0), (6, 0)), ((0, 3), None, None),
                                        ((0, 7), None, None)))
        self.assertEqual(puzzle.id, "kakuro:3x3:DbAb/4.6.3.7")
        solution = pp.solve(puzzle)
        self.assertEqual(solution.cells, ((None, None, None), (None, 1, 2), (None, 3, 4)))
        partial = pp.Solution("kakuro", ((None, None, None), (None, 1, 0), (None, 0, 0)))
        self.assertIsNotNone(pp.hint(puzzle, partial))
        wrong = pp.Solution("kakuro", ((None, None, None), (None, 2, 1), (None, 3, 4)))
        violations = pp.check(puzzle, wrong)
        self.assertEqual({v.rule for v in violations}, {1})
        self.assertEqual(violations[0].place, {"cell": {"row": 0, "col": 1}})

    def test_generation_is_reproducible(self):
        first = pp.generate("sudoku", (4, 4), difficulty=pp.Difficulty.EASY, seed=(1 << 64) - 1)
        again = pp.generate("sudoku", (4, 4), difficulty="easy", seed=(1 << 64) - 1)
        self.assertEqual(first, again)
        self.assertEqual(first.seed, (1 << 64) - 1)

    def test_ratings_hints_and_violations(self):
        g = pp.generate("sudoku", (9, 9), difficulty="medium", seed=3)
        self.assertEqual(g.rating.level, pp.Difficulty.MEDIUM)
        self.assertTrue(g.rating.techniques)
        self.assertGreater(g.rating.steps, 0)
        step = pp.hint(g.puzzle)
        self.assertIsInstance(step, pp.Step)
        # Hints skip routine bookkeeping and remove only wrong candidates.
        self.assertNotEqual(step.technique, "peer_elimination")
        for row, col, value in step.removed:
            self.assertNotEqual(int(value), g.solution.cells[row][col])
        wrong = [list(row) for row in g.solution.cells]
        wrong[0][0], wrong[0][1] = wrong[0][1], wrong[0][0]
        violations = pp.check(g.puzzle, pp.Solution("sudoku", tuple(map(tuple, wrong))))
        self.assertTrue(violations)
        self.assertTrue(all(isinstance(v, pp.Violation) for v in violations))

    def test_typed_failures(self):
        with self.assertRaises(pp.UnsupportedRequestError):
            pp.generate("sudoku", (7, 7))
        with self.assertRaises(pp.UnsupportedRequestError):
            pp.generate("chess", (8, 8))
        with self.assertRaises(ValueError):
            pp.generate("sudoku", (4, 4), difficulty="extreme")
        with self.assertRaises(pp.AttemptsExhaustedError) as attempts:
            pp.generate("sudoku", (4, 4), limits=pp.Limits(attempts=0))
        self.assertEqual(attempts.exception.attempts, 0)
        with self.assertRaises(pp.BudgetExceededError):
            pp.generate("sudoku", (4, 4), limits=pp.Limits(work=0))
        with self.assertRaises(pp.MultipleSolutionsError) as multiple:
            pp.solve(pp.Puzzle.from_id("sudoku:4x4:................"))
        self.assertEqual(len(multiple.exception.witnesses), 2)
        self.assertNotEqual(*multiple.exception.witnesses)
        with self.assertRaises(pp.NoSolutionError):
            pp.solve(pp.Puzzle.from_id("sudoku:4x4:11.............."))
        with self.assertRaises(pp.ParseError):
            pp.Puzzle.from_id("sudoku:4x4:not-a-grid")

    def test_precancelled_and_live_cancellation(self):
        token = pp.CancellationToken()
        token.cancel()
        with self.assertRaises(pp.CancelledError):
            pp.generate("sudoku", (4, 4), cancel=token)
        token = pp.CancellationToken()
        started = threading.Event()
        outcomes = []

        def run():
            started.set()
            try:
                pp.generate("kakurasu", (32, 32), difficulty="expert",
                            limits=pp.Limits(work=(1 << 64) - 1, attempts=1_000_000), cancel=token)
                outcomes.append("returned")
            except pp.CancelledError:
                outcomes.append("cancelled")
        worker = threading.Thread(target=run)
        worker.start()
        self.assertTrue(started.wait(5))
        time.sleep(0.05)
        token.cancel()
        worker.join(5)
        self.assertFalse(worker.is_alive(), "work did not observe cancellation")
        self.assertEqual(outcomes, ["cancelled"])

    def test_keyboard_interrupt_while_native_work_runs(self):
        timer = threading.Timer(0.05, _thread.interrupt_main)
        timer.start()
        try:
            with self.assertRaises(KeyboardInterrupt):
                pp.generate("kakurasu", (32, 32), difficulty="expert",
                            limits=pp.Limits(work=(1 << 64) - 1, attempts=1_000_000))
        finally:
            timer.cancel()
            timer.join()

    def test_validation_and_immutable_objects(self):
        with self.assertRaises(TypeError):
            pp.generate("sudoku", (4, 4), seed=True)
        with self.assertRaises(TypeError):
            pp.generate("sudoku", [4, 4])
        with self.assertRaises(ValueError):
            pp.Limits(work=-1)
        g = pp.generate("sudoku", (1, 1), difficulty="easy")
        with self.assertRaises(FrozenInstanceError):
            g.seed = 4

    def test_public_names(self):
        for name in pp.__all__:
            self.assertTrue(hasattr(pp, name), name)


if __name__ == "__main__":
    unittest.main()
