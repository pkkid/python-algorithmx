#!/usr/bin/env python3
"""
Author: Michael Shepanski <michael.shepanski@gmail.com>
Copyright: (c) 2022 Michael Shepanski
License: GNU General Public License <http://www.gnu.org/licenses/>
https://www.dragonfjord.com/product/a-puzzle-a-day/
"""
import argparse, datetime, random, time
import configparser, multiprocessing
import pentomino


def rgb(text, color='#aaa'):
    """ Return the text with the specified color. """
    if color is None: return text
    r,g,b = tuple(int(x*2, 16) for x in color.lstrip('#'))
    return f'\033[38;2;{r};{g};{b}m{text}\033[00m'


def print_value(date, count):
    """ Print the value for the specified date in a calendar format. """
    color = None if count else '#c00'
    if date.day == 1:
        print()
        print(f'\n{date.strftime("%B")}')
        print('     '*date.weekday(), end='')
    elif date.weekday() == 0:
        print()
    print(rgb(f'{count:4} ', color), end='')


def print_summary(solutions):
    """ Print a Summary of our findings. """
    numdays = len(solutions)
    total = sum([len(d) for d in solutions.values()])
    avgcount = round(total/numdays, 2)
    mincount = min([len(d) for d in solutions.values()])
    maxcount = max([len(d) for d in solutions.values()])
    zerocount = sum([1 for d in solutions.values() if len(d) == 0])
    print('\n')
    print(f'total:{total}, avg:{avgcount}, min:{mincount}, max:{maxcount}, zero:{zerocount}')


def find_solutions_for_year(board, pieces, year, num_procs=4):
    """ This function finds solutions for a given puzzle for each day of a
        specified year. It uses multiprocessing to speed up the process, with
        the number of processes defaulting to 4 but customizable.
    """
    solutions = {}
    try:
        procs = {}
        with multiprocessing.Pool(processes=num_procs) as multipool:
            date = datetime.date(year, 1, 1)
            while date.year == year:
                procs[date] = multipool.apply_async(find_solutions_for_date, (board, pieces, date))
                date += datetime.timedelta(days=1)
            for date, proc in procs.items():
                _solutions, count = proc.get(timeout=300)
                print_value(date, count)
                solutions[date] = _solutions
    except KeyboardInterrupt:
        print('\n\nKeyboardInterrupt; Stopping..')
    print_summary(solutions)
    total = sum([len(d) for d in solutions.values()])
    return solutions, total


def find_solutions_for_date(board, pieces, date):
    """ Find the solutions for the specified date. """
    try:
        board = pentomino.Shape.fromstr(board, 'board') if isinstance(board, str) else board
        board = set_date(board, date)
        solver = pentomino.Puzzle(board, pieces,
            allow_reflections=not opts.no_reflections,
            allow_duplicates=opts.allow_duplicates,
            showx=opts.showx,
            showy=opts.showy)
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
    parser = argparse.ArgumentParser(description='Solve the Dragonfjord puzzle.')
    parser.add_argument('-p', '--puzzle', default='dragonfjord', help='Name of the puzzle to solve.')
    parser.add_argument('-s', '--set', default='set1', help='Piece set to use.')
    parser.add_argument('-d', '--date', help='Date to solve for (YYYY-MM-DD)')
    parser.add_argument('-y', '--year', type=int, help='Run for every day of the year.')
    parser.add_argument('--no-reflections', default=False, action='store_true', help='Dont allow pieces to be flipped.')
    parser.add_argument('--allow-duplicates', default=False, action='store_true', help='Allow duplicate pieces.')
    parser.add_argument('--showx', default=False, action='store_true', help='Show the X-Contraint values.')
    parser.add_argument('--showy', default=False, action='store_true', help='Show the Y-Universe values.')
    parser.add_argument('--procs', type=int, default=4, help='Num procs to use for year mode.')
    opts = parser.parse_args()
    # Validate the arguments
    starttime = time.time()
    puzzles = configparser.ConfigParser()
    puzzles.read('puzzle-a-day.ini')
    board = puzzles[opts.puzzle]['board']
    pieces = puzzles[opts.puzzle][opts.set]
    print(f'{board}\n{pieces}')
    # Process for the full year or a single date
    if opts.year:
        solutions, total = find_solutions_for_year(board, pieces, opts.year, opts.procs)
    else:
        date = datetime.datetime.strptime(opts.date, r'%Y-%m-%d').date() if opts.date else None
        solutions, total = find_solutions_for_date(board, pieces, date)
        print(random.choice(solutions))
    runtime = round(time.time() - starttime, 1)
    print(f'Found {total} solutions after {runtime}s')
    
