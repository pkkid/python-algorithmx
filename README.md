# AlgorithmX Problems and Solvers

Trying to efficiently write a program to provide me solutions to [Dragonfjord's
"A Puzzle a Day"](https://www.dragonfjord.com/product/a-puzzle-a-day/) has lead
me down a slightly mind boggling path of learning what about Donald Knuth's
[AlgorithmX](https://en.wikipedia.org/wiki/Knuth%27s_Algorithm_X), as well as
[Exact Cover](https://en.wikipedia.org/wiki/Exact_cover), and
[Dancing Link](https://en.wikipedia.org/wiki/Dancing_Links)
algorithms. I am definitely not an expert in this algorithm, but was able to get
something working with the help of Ali Assaf's post and implementation of AlgorithmX.

The examples in this repository can solve Sudoku and Pentomino puzzles relativly
quickly. While all these examples work in Python3, I highly recomend using
[pypy](https://www.pypy.org/download.html) as it is much faster for these types
of problems.

The Python solver lives in `python/`. Run the examples below from that folder.

## Example Pentomino Solver
```python
from pentomino import Puzzle

BOARD = """
⬛⬛🟫🟫🟫⬛⬛
🟫🟫🟫🟫🟫🟫🟫
🟫🟫🟫🟫🟫🟫🟫
🟫🟫🟫⬛⬛⬛⬛
"""
PIECES = """
🟪🟪🟪 ⬛🟦⬛ ⬛🟥🟥 🟨🟨⬛ 🟩🟩🟩
⬛🟪⬛ 🟦🟦🟦 🟥🟥⬛ 🟨🟨⬛ ⬛⬛🟩
"""

solver = Puzzle(BOARD, PIECES, allow_reflections=True)
solutions = list(solver.find_solutions())
for board in solutions:
    print(board)

>>>
⬛⬛🟪🟩🟩⬛⬛
🟦🟪🟪🟪🟩🟨🟨
🟦🟦🟥🟥🟩🟨🟨
🟦🟥🟥⬛⬛⬛⬛
```

## Example Dragonfjord Command Line Solver
Only a single random solution is displayed when running from the command line
as seeing all solutions was a bit unwieldy.
```bash
> python3 python/puzzle-a-day.py --date=2025-05-29
🟦🟦🟦🟦⬛🟧⬛
🟨🟦🟨🟧🟧🟧⬛
🟨🟨🟨🟧⬜⬜⬜
🟩🟩🟩⬜⬜🟫🟫
🟩🟪🟪🟥🟫🟫🟫
🟩🟪🟪🟥🟥🟥🟥
⬛🟪🟪⬛⬛⬛⬛

Found 66 solutions after 5.5s.
```

To view all available options, run `python3 python/puzzle-a-day.py --help`. Available
options for `--puzzle` are `dragonfjord` and `guanglu`, see `puzzle-a-day.ini`
for the puzzle layouts.

## Rust Solver
`rust/` is a Rust rewrite of the puzzle-a-day solver with no dependencies. It
reads the same `puzzle-a-day.ini` (embedded at compile time, or pass `--ini`) and
accepts the same options as the Python script, so the two are interchangeable.
```bash
cd rust
cargo build --release
./target/release/puzzle-a-day --date=2025-05-29
./target/release/puzzle-a-day -p guanglu -s set1 -y 2025
./target/release/puzzle-a-day --help
```

It includes two solvers, picked with `--algo`:

* **`dlx`**: the same algorithm as the Python code. Knuth's Algorithm X, always
  branching on the constraint with the fewest options, implemented with Dancing
  Links instead of Python dicts of sets. This isolates how much of the speedup
  comes from the language alone.
* **`bits`** (default): a different algorithm built for this kind of puzzle.
  * The board fits in one 64-bit integer, and every placement of every piece is
    precomputed as a bitmask, so testing whether a piece fits is a single AND.
  * The search always fills the first empty square, starting from the
    bottom-right corner (measured fastest of the 8 corner/direction choices).
  * After each placement it prunes the branch if any enclosed pocket of empty
    squares has a size no combination of the remaining pieces can fill.
  * Year mode solves dates together instead of one at a time. Without weekday
    squares, it adds 1x1 "hole" pieces restricted to the month and day squares,
    so one search counts the solutions for every date at once. With weekdays, it
    caches the count of each partial board (filled squares + pieces used) and
    shares the cache across all the dates in a month.

Both solvers find exactly the same solutions; every day of 2025 was checked
against the Python solver.

### Speed comparison
Single core, solve time inside the process (no interpreter startup), on an
Intel i7-13700K with CPython 3.15 and Rust 1.99. "~" marks Python times estimated
from every 7th day of the year. Run `python3 rust/bench.py` to reproduce.

| Task | Python | Rust `dlx` | Rust `bits` |
|---|---:|---:|---:|
| Dragonfjord, one date | 0.39s | 0.009s (43x) | 0.003s (130x) |
| Guanglu, one date | 26.3s | 0.35s (74x) | 0.069s (381x) |
| Dragonfjord, every day of 2025 | 147s | 1.9s (77x) | 0.076s (1,900x) |
| Guanglu, every day of 2025 | ~3,440s | 60s (57x) | 3.8s (900x) |

Using all 24 cores (`--procs`, on by default), Rust `bits` counts a full year of
Guanglu in 0.45s and Dragonfjord in 0.006s. For comparison, the Python run in
`results/guanglu-set1-2025-results.txt` took 2,205s on 16 processes.

`cargo run --release --example compare` shows what each optimization contributes:

| Every day of 2025, single core | Dragonfjord | Guanglu |
|---|---:|---:|
| Rust `dlx` | 1.87s | 61.9s |
| Bitboard, one search per date | 0.88s | 39.9s |
| + prune unfillable pockets | 0.42s | 17.6s |
| + fill from the bottom-right corner | 0.42s | 11.3s |
| + solve dates together (holes / shared cache) | 0.074s | 3.8s |

## Web Demo
`web/` is a playable browser version: drag, rotate and flip pieces, pick any date,
and press Solve to animate a solution into place. It uses the same Rust code
compiled to an ~100 KB WebAssembly module, which solves a Dragonfjord date in
about 1-2 ms in the browser.

Run `web/build.sh` (needs the `wasm32-unknown-unknown` Rust target), then either
serve `web/index.html` and `web/puzzle_a_day.wasm` from any static web host, or
open `web/standalone.html`, which has the WASM inlined into one file.

## Exploring New Piece Sets
`rust/src/bin/explore.rs` searches for new Dragonfjord piece sets and rates how
hard they are, using a simulated solver that plays like a person (fills forced
spots first, works on the tightest square, guesses, and backtracks). Difficulty is
the average number of pieces it places and later has to take back.
```bash
cd rust
./target/release/explore search --out sets.csv     # every 7 pentominoes + 1 hexomino set
./target/release/explore rate "H29 L Y P U V N Z"  # rate one set
./target/release/explore analyze --set set1        # difficulty of every date
./target/release/explore onesided                  # pieces that may never be flipped
```

Of the 27,720 sets of 7 pentominoes + 1 hexomino, only 356 solve every date of
the year (including Feb 29). The interesting ones are in `puzzle-a-day.ini`:

* **easy**: set1 with the Z swapped for an F. Easiest of the 356, about 22%
  easier than set1.
* **hard**: set1 with the 2x3 block swapped for a zigzag hexomino. Hardest of
  the 356, about 3.5x harder than set1.
* **interesting**: the weirdest-shaped set that still works, scored by counting
  corners on each piece's outline.
* **onesided**: works every date with pieces never flipped (play with
  `--no-reflections`), and again with every piece flipped over. It needs two L's
  and two P's, painted as mirror images.

Raw results are in `results/dragonfjord-*.csv`.

## Example Sudoku Solver
This solver is not my code, but Ali Assaf's. Copied from his posted example,
cleaned up Python 3 styles, and commented to help me understand. I included
it in this repo to hopefully help other's learn this algorithm usage as well.

```python
from sudoku import solve_sudoku

grid = [
    [0,1,0, 0,0,0, 0,3,0],
    [9,0,0, 0,2,0, 1,5,0],
    [0,0,0, 1,0,0, 0,6,4],
    [7,0,0, 0,0,0, 0,0,0],
    [8,0,0, 3,9,0, 5,0,6],
    [0,0,0, 0,0,0, 0,4,9],
    [5,0,0, 0,7,1, 0,0,0],
    [0,0,8, 0,0,0, 0,9,1],
    [0,4,0, 2,6,0, 0,0,5]]
for solution in solve_sudoku(grid):
    print(*solution, sep='\n')
    print()

>>>
[4, 1, 2, 7, 5, 6, 9, 3, 8]
[9, 8, 6, 4, 2, 3, 1, 5, 7]
[3, 5, 7, 1, 8, 9, 2, 6, 4]
[7, 9, 1, 6, 4, 5, 8, 2, 3]
[8, 2, 4, 3, 9, 7, 5, 1, 6]
[6, 3, 5, 8, 1, 2, 7, 4, 9]
[5, 6, 3, 9, 7, 1, 4, 8, 2]
[2, 7, 8, 5, 3, 4, 6, 9, 1]
[1, 4, 9, 2, 6, 8, 3, 7, 5]
```


## Thanks To
* [Donald Knuth](https://en.wikipedia.org/wiki/Donald_Knuth) for coming up with
  [AlgorithmX](https://en.wikipedia.org/wiki/Knuth%27s_Algorithm_X).
* [Ali Assaf](https://www.cs.mcgill.ca/~aassaf9/index.html) for the short implementation
  of AlgorithX as well as a [great write up and example](https://www.cs.mcgill.ca/~aassaf9/python/algorithm_x.html).

## License
GNU General Public License <http://www.gnu.org/licenses/>
