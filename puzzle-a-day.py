#!/usr/bin/env python3
"""
Author: Michael Shepanski <michael.shepanski@gmail.com>
Copyright: (c) 2022 Michael Shepanski
License: GNU General Public License <http://www.gnu.org/licenses/>

References:
https://www.dragonfjord.com/product/a-puzzle-a-day/
"""
import argparse, datetime, random, time
import configparser, multiprocessing
import pentomino
from os.path import abspath

PUZZLEBOOK = abspath(__file__).replace('.py', '.ini')
BASEDIFFICULTY = 24341


def rgb(text, color='#aaa'):
    if color is None: return text
    r,g,b = tuple(int(x*2, 16) for x in color.lstrip('#'))
    return f'\033[38;2;{r};{g};{b}m{text}\033[00m'


def find_solutions_for_year(puzzle, year, num_procs=4):
    solutions = {}
    try:
        procs = {}
        with multiprocessing.Pool(processes=num_procs) as multipool:
            date = datetime.date(year, 1, 1)
            while date.year == year:
                procs[date] = multipool.apply_async(find_solutions_for_date, (puzzle, date))
                date += datetime.timedelta(days=1)
            for date, proc in procs.items():
                _solutions, count = proc.get(timeout=300)
                color = None if count else '#c00'
                print(rgb(f'{date.strftime("%b %d")}: Found {count} solutions', color))
                solutions[date] = _solutions
    except KeyboardInterrupt:
        print(' KeyboardInterrupt; Stopping..')
    # Print a Summary of our findings
    numdays = len(solutions)
    total = sum([len(d) for d in solutions.values()])
    print(f'Average: {round(total/numdays, 2)}')
    print(f'Total: {total}')
    print(f'No Solutuion: {sum([1 for d in solutions.values() if len(d) == 0])}')
    print(f'Difficulty: {round(total/BASEDIFFICULTY, 2)}x Dragonfjord')
    return solutions, total


def find_solutions_for_date_test(puzzle, date):
    board, pieces = puzzle['board'].strip().split('\n\n')
    board = pentomino.Shape.fromstr(board, 'board') if isinstance(board, str) else board
    solver = pentomino.Puzzle(board, pieces, allow_reflections=True, allow_duplicates=False, allow_empty_squares=True)
    solutions = list(solver.find_solutions())
    return solutions, len(solutions)


def find_solutions_for_date(puzzle, date):
    try:
        board, pieces = puzzle['board'].strip().split('\n\n')
        allow_reflections = puzzle.getboolean('allow_reflections')
        allow_duplicates = puzzle.getboolean('allow_duplicates')
        board = pentomino.Shape.fromstr(board, 'board') if isinstance(board, str) else board
        if puzzle.getboolean('set_date'):
            board = set_date(board, date)
        solver = pentomino.Puzzle(board, pieces, allow_reflections, allow_duplicates,
            showx=opts.showx, showy=opts.showy)
        solutions = list(solver.find_solutions())
        return solutions, len(solutions)
    except KeyboardInterrupt:
        return [], 0


def set_date(board, date=None):
    """ Place the occupied markers on the specified month and day. If the number
        of open squares is large enough, it will also occupy the weekday starting
        with Sunday as the first slot.
    """
    date = date or datetime.date.today()
    weekday = ((date.weekday() + 1) % 7) + 1
    count = 0
    for r,c in board.coords():
        count += 1
        if date.month == count: board.shape[r][c] = pentomino.Shape.EMPTY
        if date.day == count-12: board.shape[r][c] = pentomino.Shape.EMPTY
        if weekday == count-12-31: board.shape[r][c] = pentomino.Shape.EMPTY
    return board


if __name__ == '__main__':
    # Parse the command line options
    puzzles = configparser.ConfigParser()
    puzzles.read(PUZZLEBOOK)
    parsedate = lambda datestr: datetime.datetime.strptime(datestr, r'%Y-%m-%d').date()
    parsepuzzle = lambda puzzlestr: puzzles[puzzlestr]
    parser = argparse.ArgumentParser(description='Solve the Dragonfjord puzzle.')
    parser.add_argument('puzzle', type=parsepuzzle, default='dragonfjord', help='Name of the puzzle to solve.')
    parser.add_argument('-d', '--date', type=parsedate, help='Date to solve for (YYYY-MM-DD)')
    parser.add_argument('-y', '--year', type=int, help='Run for every day of the year.')
    parser.add_argument('--test', default=False, action='store_true', help='Run test method.')
    parser.add_argument('--showx', default=False, action='store_true', help='Show the X-Contraint values.')
    parser.add_argument('--showy', default=False, action='store_true', help='Show the Y-Universe values.')
    parser.add_argument('--procs', type=int, default=4, help='Num procs to use for year mode.')
    opts = parser.parse_args()
    # Run the program and print the results
    starttime = time.time()
    print(f'{opts.puzzle["board"]}\n')
    # Run the correct function
    if opts.test: solutions, total = find_solutions_for_date_test(opts.puzzle, opts.date)
    elif opts.year: solutions, total = find_solutions_for_year(opts.puzzle, opts.year, opts.procs)
    else: solutions, total = find_solutions_for_date(opts.puzzle, opts.date)
    # Print the results
    if not opts.year and total: print(random.choice(solutions))
    runtime = round(time.time() - starttime, 1)
    print(f'Found {total} solutions after {runtime}s')
    
