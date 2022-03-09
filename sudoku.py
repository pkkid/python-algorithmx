#!/usr/bin/env python3
"""
Author: Ali Assaf <ali.assaf.mail@gmail.com>
Copyright: (c) 2010 Ali Assaf
License: GNU General Public License <http://www.gnu.org/licenses/>
"""
import math
from itertools import product
from algox import exact_cover, select, solve


def solve_sudoku(grid):
    """ An efficient Sudoku solver using Algorithm X.
        grid = [
            [5,3,0, 0,7,0, 0,0,0],
            [6,0,0, 1,9,5, 0,0,0],
            [0,9,8, 0,0,0, 0,6,0],
            [8,0,0, 0,6,0, 0,0,3],
            [4,0,0, 8,0,3, 0,0,1],
            [7,0,0, 0,2,0, 0,0,6],
            [0,6,0, 0,0,0, 2,8,0],
            [0,0,0, 4,1,9, 0,0,5],
            [0,0,0, 0,8,0, 0,7,9]]
        for solution in solve_sudoku(grid):
            print(*solution, sep='\n')
    """
    gridsize = len(grid)                    # Size of the grid
    boxsize = int(math.sqrt(gridsize))      # Size of a single box
    rows = range(gridsize)                  # All rows in the grid 0-8
    cols = range(gridsize)                  # All columns in the grid 0-8
    boxes = range(gridsize)                 # All boxnums in the grid 0-8
    nums = range(1, gridsize+1)             # All numbers in the grid 1-9
    # X-Constraints (from the Wikipedia example)
    # rc: Row-Column: Each intersection of a row-column must contain exactly one number.
    # rn: Row-Number: Each row must contain each number exactly once
    # cn: Column-Number: Each column must contain each number exactly once.
    # bn: Box-Number: Each box must contain each number exactly once.
    X = (
        [('rc', rc) for rc in product(rows, cols)] +
        [('rn', rn) for rn in product(rows, nums)] +
        [('cn', cn) for cn in product(cols, nums)] +
        [('bn', bn) for bn in product(boxes, nums)]
    )
    # Y-Universe
    # For every Row-Column-Number set, list which constraints it satisfies
    # in the definition of X above.
    Y = dict()
    for row, col, num in product(rows, cols, nums):
        box = (row // boxsize) * boxsize + (col // boxsize)  # Box number
        Y[(row,col,num)] = [('rc',(row,col)), ('rn',(row,num)), ('cn',(col,num)), ('bn',(box,num))]
    # Convert X to a dictionary per the changes to Algorithm X as mentioned here:
    # https://www.cs.mcgill.ca/~aassaf9/python/algorithm_x.html
    X = exact_cover(X, Y)
    # Pre-populate the algorithm
    # Select known positions in the grid
    for row in rows:
        for col in cols:
            num = grid[row][col]
            if num != 0:
                selection = (row, col, num)
                select(X, Y, selection)
    # Iterate all solutions
    # Update the grid in place and yeild the result
    for solution in solve(X, Y, []):
        for (r, c, n) in solution:
            grid[r][c] = n
        yield grid


if __name__ == '__main__':
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
