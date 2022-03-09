#!/usr/bin/env python3
"""
Author: Michael Shepanski <michael.shepanski@gmail.com>
Copyright: (c) 2022 Michael Shepanski
License: GNU General Public License <http://www.gnu.org/licenses/>

References:
https://www.dragonfjord.com/product/a-puzzle-a-day/
"""
import argparse, random, time
import multiprocessing
from collections import defaultdict
from datetime import date, timedelta
from pentomino import PentominoPuzzle, Shape

THISMONTH = date.today().month
THISDAY = date.today().day
OCCUPIED = "⚫"
BOARD = """
🟫🟫🟫🟫🟫🟫⬛
🟫🟫🟫🟫🟫🟫⬛
🟫🟫🟫🟫🟫🟫🟫
🟫🟫🟫🟫🟫🟫🟫
🟫🟫🟫🟫🟫🟫🟫
🟫🟫🟫🟫🟫🟫🟫
🟫🟫🟫⬛⬛⬛⬛
"""
PIECES = """
🟪🟪⬛ 🟦⬛⬛ 🟩🟩🟩 🟨⬛🟨 🟧🟧⬛ 🟥🟥⬛ 🟫⬛⬛ ⬜⬛⬛
🟪🟪⬛ 🟦⬛⬛ 🟩⬛⬛ 🟨🟨🟨 ⬛🟧⬛ 🟥⬛⬛ 🟫🟫⬛ ⬜⬜⬛
🟪🟪⬛ 🟦🟦⬛ 🟩⬛⬛ ⬛⬛⬛ ⬛🟧🟧 🟥⬛⬛ 🟫🟫⬛ ⬛⬜⬛
⬛⬛⬛ 🟦⬛⬛ ⬛⬛⬛ ⬛⬛⬛ ⬛⬛⬛ 🟥⬛⬛ ⬛⬛⬛ ⬛⬜⬛
"""


def find_solutions_for_year(board, pieces, procs=4):
    results = [None] * 366
    metrics = {'total':[], 'day':defaultdict(list), 'month':defaultdict(list)}
    try:
        with multiprocessing.Pool(processes=procs) as multipool:
            for i in range(366):
                day = date(2000, 1, 1) + timedelta(days=i)
                newboard = Shape.fromstr(board, 'board')
                args = (newboard, pieces, day.month, day.day)
                results[i] = (day, multipool.apply_async(find_solutions_for_day, args))
            for i in range(366):
                day, proc = results[i]
                solutions = proc.get(timeout=60)
                print(f'{day.strftime("%b %d")}: Found {len(solutions)} solutions')
                metrics['total'].append(len(solutions))
                metrics['month'][day.strftime("%b")].append(len(solutions))
                metrics['day'][day.strftime("%d")].append(len(solutions))
    except KeyboardInterrupt:
        pass
    # Print a Summary of our findings
    print('---')
    for day in metrics['day']:
        avg = round(sum(metrics["day"][day])/len(metrics["day"][day]), 1)
        print(f'{day} Average: {avg}')
    print('---')
    for month in metrics['month']:
        avg = round(sum(metrics["month"][month])/len(metrics["month"][month]), 1)
        print(f'{month} Average: {avg}')
    print('---')
    print(f'Average: {sum(metrics["total"])/len(metrics["total"])}')
    print(f'Total: {sum(metrics["total"])}')


def find_solutions_for_day(board, pieces, month, day):
    try:
        board = Shape.fromstr(board, 'board') if isinstance(board, str) else board
        board = set_month_and_day(board, month, day)
        solver = PentominoPuzzle(board, pieces, allow_reflections=True)
        return list(solver.find_solutions())
    except KeyboardInterrupt:
        pass


def set_month_and_day(board, month, day):
    """ Place the occupied markers on the specified month and day. """
    assert 1 <= month <= 12, f'Unknown month {month}'
    assert 1 <= day <= 31, f'Unknown day {day}'
    count = 0
    for r,c in board.coords():
        count += 1
        if month == count: board.shape[r][c] = Shape.EMPTY
        if day == count-12: board.shape[r][c] = Shape.EMPTY
    return board


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description='Solve the Dragonfjord puzzle.')
    parser.add_argument('-m', '--month', default=THISMONTH, type=int, help='Month to solve for.')
    parser.add_argument('-d', '--day', default=THISDAY, type=int, help='Day to solve for.')
    parser.add_argument('-y', '--year', default=False, action='store_true', help='Run for every day of the year.')
    parser.add_argument('-p', '--procs', type=int, default=4, help='Num procs to use for year mode.')
    opts = parser.parse_args()
    # Run the program and print the results
    starttime = time.time()
    if opts.year:
        solutions = find_solutions_for_year(BOARD, PIECES, opts.procs)
        runtime = round(time.time() - starttime, 1)
        print(f'Finished after {runtime}s')
    else:
        solutions = find_solutions_for_day(BOARD, PIECES, opts.month, opts.day)
        if solutions: print(random.choice(solutions))
        runtime = round(time.time() - starttime, 1)
        print(f'Found {len(solutions)} solutions after {runtime}s.')
    
