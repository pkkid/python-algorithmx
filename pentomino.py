#!/usr/bin/env python3
"""
Author: Michael Shepanski <michael.shepanski@gmail.com>
Copyright: (c) 2022 Michael Shepanski
License: GNU General Public License <http://www.gnu.org/licenses/>

References:
https://en.wikipedia.org/wiki/Exact_cover
"""
import argparse, copy, itertools, time, uuid
from algox import exact_cover, solve

# All shapes need to be an even 2d array. All pieces need to be the same
# size 2d array. The dark black square represents an empty space allowing
# for unique shapes and sizes.
EXAMPLE_BOARD = """
⬛⬛🟫🟫🟫⬛⬛
🟫🟫🟫🟫🟫🟫🟫
🟫🟫🟫🟫🟫🟫🟫
🟫🟫🟫⬛⬛⬛⬛
"""
EXAMPLE_PIECES = """
🟪🟪🟪 ⬛🟦⬛ ⬛🟥🟥 🟨🟨⬛ 🟩🟩🟩
⬛🟪⬛ 🟦🟦🟦 🟥🟥⬛ 🟨🟨⬛ ⬛⬛🟩
"""


class Shape:
    EMPTY = '⬛'

    def __init__(self, shape, name):
        self.shape = list(list(r) for r in shape)  # List of list of chars
        self.name = name        # Shape name used for constrints
        self.trim()             # Trim empty space around the shape
    
    def __hash__(self):
        return hash(tuple(tuple(r) for r in self.shape))
    
    def __eq__(self, other):
        return self.shape == other.shape
    
    def __repr__(self):
        return str(tuple(''.join(row) for row in self.shape)).replace(' ','')
    
    def __str__(self):
        return '\n'.join(''.join(row) for row in self.shape) + '\n'
    
    @classmethod
    def fromstr(self, shapestr, nametmpl=None):
        """ Factory function return a Shape object or list of Shape objects from a
            string. use <i> in nametmpl to represent the piece number if needed. If
            nametmpl is not passed in, a random 5 char hex string will be generated.
        """
        nametmpl = nametmpl or uuid.uuid4().hex[-5:]
        shapestr = shapestr.strip()
        if ' ' in shapestr:
            # If there is still a space in the shapestr, we can
            # assume they want a liast of Shape objects returned.
            shapes = []
            for i, shapelist in enumerate(zip(*[r.split() for r in shapestr.split('\n')])):
                name = nametmpl.replace('<i>', str(i))
                shapes.append(Shape(shapelist, name))
            return shapes
        # No space in shapestr, return a single Shape.
        return Shape(shapestr.split('\n'), nametmpl)

    @property
    def rows(self):
        """ Return number of rows in this shape. """
        return range(len(self.shape))
    
    @property
    def cols(self):
        """ Return number of cols in this shape. """
        return range(len(self.shape[0]))
    
    def coords(self):
        """ Iterate the coordinates of this shape. """
        for r,c in itertools.product(self.rows, self.cols):
            if self.shape[r][c] != Shape.EMPTY:
                yield r,c

    def positions(self, other, r, c):
        """ Returns the coordinates of the other shape inside this one with a starting
            position of (r,c). If the other shape is out of bounds, this returns None.
        """
        positions = set()
        for pr, pc in other.coords():
            if r+pr >= len(self.shape): return None
            if c+pc >= len(self.shape[0]): return None
            if self.shape[r+pr][c+pc] == Shape.EMPTY: return None
            positions.add((r+pr, c+pc))
        return positions

    def reflect(self):
        """ Return a new piece reflected. """
        return Shape([row[::-1] for row in self.shape], self.name)

    def rotate(self):
        """ Return a new piece rotated 90'. """
        return Shape(list(zip(*self.shape[::-1])), self.name)
    
    def rotations(self, allow_reflections=False):
        """ Iterate all rotations for the specified piece. """
        rotations = set()
        rotation = self
        for i in range(8 if allow_reflections else 4):
            rotation = rotation.reflect() if i == 4 else rotation.rotate()
            if rotation not in rotations:
                rotations.add(rotation)
                yield rotation
    
    def trim(self):
        """ Trim empty space around the shape. """
        trimming = True
        while trimming:
            trimming = False
            if all(row[-1] == Shape.EMPTY for row in self.shape):
                self.shape = [row[:-1] for row in self.shape]
                trimming = True
            if all(x == Shape.EMPTY for x in self.shape[-1]):
                self.shape = self.shape[:-1]
                trimming = True
    
    def update(self, other, r, c):
        """ Update this shape by placing the other shape inside this one at the specified
            location. There is no error checking here, we assume it will fit.
        """
        for pr, pc in itertools.product(other.rows, other.cols):
            if other.shape[pr][pc] != Shape.EMPTY:
                self.shape[r+pr][c+pc] = other.shape[pr][pc]


class PentominoPuzzle:

    def __init__(self, board, pieces, allow_reflections=False, allow_duplicates=False):
        """ Solve a pentomino placement puzzle using Algorithm X by Donald Knuth. Using
            the implementation of that algorithm by Ali Assaf.
            > solver = PentominoPuzzle(BOARD, PIECES)
            > for board in solver.find_solutions():
            >     print(board)
        """
        self.board = Shape.fromstr(board, 'board') if isinstance(board, str) else board
        self.pieces = Shape.fromstr(pieces, 'p<i>') if isinstance(pieces, str) else pieces
        self.allow_reflections = allow_reflections  # Set true to allow piece reflections
        self.allow_duplicates = allow_duplicates    # Set true to allow piece duplicates
        
    def init_pieces(self, pieces):
        """ Convert the single pieces string into a list of Piece objects. """
        shapes = list(zip(*[r.split() for r in pieces.strip().split('\n')]))
        return [Shape(shape, f'p{i+1}') for i, shape in enumerate(shapes)]
    
    def update_board(self, piece, r, c):
        """ Place the piece in the board at the specified starting r,c. """
        for pr, pc in piece.coords():
            if piece.shape[pr][pc] != Shape.EMPTY:
                self.board[r+pr][c+pc] = piece.shape[pr][pc]

    def find_solutions(self):
        # X-Constraints
        # Board: For each of the board squares, there is the constraint that it must be
        #    covered by a pentomino exactly once. Name these constraints after the
        #    corresponding squares in the board: ij, where i is the rank and j is the file.
        # Pieces: For each of the pieces, there is the constraint that it must be placed
        #    exactly once. Name these constraints after their piece names: p1, p2, p3, ...
        X = [('b',(r,c)) for r,c in self.board.coords()]
        if not self.allow_duplicates:
            X += [('p',piece.name) for piece in self.pieces]
        # Y-Universe
        # For every Row-Column-PieceRotation set, list which constraints it satisfies
        # in the definition of X above.
        Y = {}
        for i, piece in enumerate(self.pieces):
            for rotation in piece.rotations(self.allow_reflections):
                for r, c in itertools.product(self.board.rows, self.board.cols):
                    if positions := self.board.positions(rotation, r, c):
                        Y[(rotation,r,c)] = [('b',(r,c)) for r,c in positions]
                        if not self.allow_duplicates:
                            Y[(rotation,r,c)] += [('p',piece.name)]
        # Solve it!
        X = exact_cover(X, Y)
        for solution_keys in solve(X, Y, []):
            solution = copy.deepcopy(self.board)
            for rotation, r, c in solution_keys:
                solution.update(rotation, r, c)
            yield solution


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description='Solve a pentomino placement puzzle.')
    parser.add_argument('-r', '--allow-reflections', default=False, action='store_true', help='Allow piece reflections.')
    parser.add_argument('-d', '--allow-duplicates', default=False, action='store_true', help='Allow duplicate pieces.')
    opts = dict(vars(parser.parse_args()))
    # Run the example puzzle
    starttime = time.time()
    solver = PentominoPuzzle(EXAMPLE_BOARD, EXAMPLE_PIECES, **opts)
    solutions = list(solver.find_solutions())
    for board in solutions:
        print(board)
    runtime = round(time.time() - starttime, 1)
    print(f'Found {len(solutions)} solutions in {runtime}s.')
