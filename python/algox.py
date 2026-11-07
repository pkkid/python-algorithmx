#!/usr/bin/env python3
"""
Author: Ali Assaf <ali.assaf.mail@gmail.com>
Copyright: (c) 2010 Ali Assaf
License: GNU General Public License <http://www.gnu.org/licenses/>

References:
https://www.cs.mcgill.ca/~aassaf9/python/algorithm_x.html
https://en.wikipedia.org/wiki/Exact_cover
"""


def solve(X, Y, solution=None):
    if not X:
        yield list(solution)
    else:
        c = min(X, key=lambda c: len(X[c]))
        for r in list(X[c]):
            solution.append(r)
            cols = select(X, Y, r)
            for s in solve(X, Y, solution):
                yield s
            deselect(X, Y, r, cols)
            solution.pop()


def select(X, Y, r):
    cols = []
    for j in Y[r]:
        for i in X[j]:
            for k in Y[i]:
                if k != j:
                    X[k].remove(i)
        cols.append(X.pop(j))
    return cols


def deselect(X, Y, r, cols):
    for j in reversed(Y[r]):
        X[j] = cols.pop()
        for i in X[j]:
            for k in Y[i]:
                if k != j:
                    X[k].add(i)


def exact_cover(X, Y):
    X = {j: set() for j in X}
    for i, row in Y.items():
        for j in row:
            X[j].add(i)
    return X


if __name__ == '__main__':
    X = {1, 2, 3, 4, 5, 6, 7}
    Y = {'A':[1,4,7], 'B':[1,4], 'C':[4,5,7], 'D':[3,5,6], 'E':[2,3,6,7], 'F':[2,7]}
    X = exact_cover(X, Y)
    solutions = solve(X, Y, [])
    print(list(solutions))
