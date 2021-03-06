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
โฌโฌ๐ซ๐ซ๐ซโฌโฌ
๐ซ๐ซ๐ซ๐ซ๐ซ๐ซ๐ซ
๐ซ๐ซ๐ซ๐ซ๐ซ๐ซ๐ซ
๐ซ๐ซ๐ซโฌโฌโฌโฌ
"""
PIECES = """
๐ช๐ช๐ช โฌ๐ฆโฌ โฌ๐ฅ๐ฅ ๐จ๐จโฌ ๐ฉ๐ฉ๐ฉ
โฌ๐ชโฌ ๐ฆ๐ฆ๐ฆ ๐ฅ๐ฅโฌ ๐จ๐จโฌ โฌโฌ๐ฉ
"""

solver = Puzzle(BOARD, PIECES, allow_reflections=True)
solutions = list(solver.find_solutions())
for board in solutions:
    print(board)

>>>
โฌโฌ๐ช๐ฉ๐ฉโฌโฌ
๐ฆ๐ช๐ช๐ช๐ฉ๐จ๐จ
๐ฆ๐ฆ๐ฅ๐ฅ๐ฉ๐จ๐จ
๐ฆ๐ฅ๐ฅโฌโฌโฌโฌ
```

## Example Dragonfjord Command Line Solver
Only show a single random solution is displayed when running from the command line
as seeing all solutions was a bit unwieldy.
```bash
> python3 dragonfjord.py --month=5 --day=29
๐ฆ๐ฆ๐ฆ๐ฆโฌ๐งโฌ
๐จ๐ฆ๐จ๐ง๐ง๐งโฌ
๐จ๐จ๐จ๐งโฌโฌโฌ
๐ฉ๐ฉ๐ฉโฌโฌ๐ซ๐ซ
๐ฉ๐ช๐ช๐ฅ๐ซ๐ซ๐ซ
๐ฉ๐ช๐ช๐ฅ๐ฅ๐ฅ๐ฅ
โฌ๐ช๐ชโฌโฌโฌโฌ

Found 66 solutions after 5.5s.
```

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
