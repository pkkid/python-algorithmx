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
> python3 puzzle-a-day.py --date=2025-05-29
🟦🟦🟦🟦⬛🟧⬛
🟨🟦🟨🟧🟧🟧⬛
🟨🟨🟨🟧⬜⬜⬜
🟩🟩🟩⬜⬜🟫🟫
🟩🟪🟪🟥🟫🟫🟫
🟩🟪🟪🟥🟥🟥🟥
⬛🟪🟪⬛⬛⬛⬛

Found 66 solutions after 5.5s.
```

To view all available options, run `python3 puzzle-a-day.py --help`. Available
options for `--puzzle` are `dragonfjord` and `guanglu`, see `puzzle-a-day.ini`
for the puzzle layouts.

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
