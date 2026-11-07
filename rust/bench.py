#!/usr/bin/env python3
"""
Benchmark the Python solver against the Rust (native and WASM) solver.

  cd rust && cargo build --release && cargo build --release --target wasm32-unknown-unknown --lib
  python3 bench.py                 # single dates + full year (Python estimated from a sample)
  python3 bench.py --year 2025     # also run Python on every day (slow)

Every timing is solve time measured inside the process (no interpreter startup).
"""
import argparse, configparser, datetime, importlib.util, json, os, statistics
import subprocess, sys, time

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
RUST_BIN = os.path.join(HERE, 'target', 'release', 'puzzle-a-day')
WASM = os.path.join(HERE, 'target', 'wasm32-unknown-unknown', 'release', 'puzzle_a_day.wasm')
sys.path.insert(0, os.path.join(ROOT, 'python'))
spec = importlib.util.spec_from_file_location('pad', os.path.join(ROOT, 'python', 'puzzle-a-day.py'))
pad = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pad)


class Opts:
    no_reflections = False
    allow_duplicates = False
    showx = showy = False


def load(puzzle, pset):
    cfg = configparser.ConfigParser()
    cfg.read(os.path.join(ROOT, 'puzzle-a-day.ini'), encoding='utf-8')
    return cfg[puzzle]['board'], cfg[puzzle][pset]


def python_counts(puzzle, pset, dates):
    board, pieces = load(puzzle, pset)
    counts, start = {}, time.perf_counter()
    for date in dates:
        counts[date.isoformat()] = pad.find_solutions_for_date(board, pieces, date, Opts)[1]
    return counts, time.perf_counter() - start


def rust_year(puzzle, pset, year, procs, algo='bits'):
    start = time.perf_counter()
    out = subprocess.run([RUST_BIN, '-p', puzzle, '-s', pset, '-y', str(year), '--csv',
        f'--procs={procs}', f'--algo={algo}'], capture_output=True, text=True, check=True).stdout
    elapsed = time.perf_counter() - start
    counts = dict(l.split(',') for l in out.splitlines() if l[:1].isdigit())
    return {k: int(v) for k, v in counts.items()}, elapsed


def rust_date(puzzle, pset, date, repeat, algo='bits'):
    """ Median in-process time over `repeat` runs, reported by the binary. """
    times = []
    for _ in range(repeat):
        out = subprocess.run([RUST_BIN, '-p', puzzle, '-s', pset, f'--date={date}', f'--algo={algo}'],
            capture_output=True, text=True, check=True).stdout
        times.append(float(out.strip().split()[-1].rstrip('s')))
    return statistics.median(times)


WASM_JS = r"""
import fs from 'fs';
const [wasmPath, puzzle, set, year, month, day, weekday] = process.argv.slice(1);
const {instance} = await WebAssembly.instantiate(fs.readFileSync(wasmPath));
const w = instance.exports;
const t0 = performance.now();
for (let i = 0; i < 20; i++) w.solve(+puzzle, +set, +month, +day, +weekday, 1, 0);
const single = (performance.now() - t0) / 20;
const t1 = performance.now();
const total = w.count_year(+puzzle, +set, +year, 1, 0);
const year_bits = (performance.now() - t1) / 1000;
const t2 = performance.now();
w.count_year(+puzzle, +set, +year, 1, 1);
console.log(JSON.stringify({single, year_bits, year_dlx: (performance.now() - t2) / 1000, total}));
"""


def wasm(puzzle_index, set_index, date):
    weekday = ((date.weekday() + 1) % 7) + 1
    out = subprocess.run(['node', '--input-type=module', '-e', WASM_JS, WASM, str(puzzle_index),
        str(set_index), str(date.year), str(date.month), str(date.day), str(weekday)],
        capture_output=True, text=True, check=True).stdout
    return json.loads(out)


def days(year, step=1):
    d, out = datetime.date(year, 1, 1), []
    while d.year == year:
        out.append(d)
        d += datetime.timedelta(days=step)
    return out


def row(label, py, dlx, bits, est=False):
    py_str = f'{"~" if est else " "}{py:9.2f}s'
    print(f'  {label:<26} {py_str:>11} {dlx:9.3f}s {bits:9.3f}s {py / dlx:7.0f}x {py / bits:8.0f}x')


def header(title):
    print(f'\n{title}')
    print(f'  {"":<26} {"python":>11} {"rust dlx":>10} {"rust bits":>10} {"dlx":>7} {"bits":>8}')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--year', type=int, help='Also run Python on every day of this year (slow)')
    opts = parser.parse_args()
    date = datetime.date(2025, 5, 29)
    puzzles = [('dragonfjord', 'set1'), ('guanglu', 'set1')]
    print(f'Python {sys.version.split()[0]} ({sys.implementation.name}) vs Rust release build, single core')

    header('Single date (2025-05-29), median of runs:')
    for puzzle, pset in puzzles:
        py = statistics.median(python_counts(puzzle, pset, [date])[1] for _ in range(3))
        row(f'{puzzle} {pset}', py, rust_date(puzzle, pset, date, 10, 'dlx'),
            rust_date(puzzle, pset, date, 10))

    year = opts.year or 2025
    header(f'Every day of {year} (~ = Python estimated from every 7th day):')
    for puzzle, pset in puzzles:
        rsc, bits = rust_year(puzzle, pset, year, 1)
        dlxc, dlx = rust_year(puzzle, pset, year, 1, 'dlx')
        assert rsc == dlxc, 'rust dlx/bits count mismatch!'
        if opts.year and puzzle == 'dragonfjord':
            pyc, py = python_counts(puzzle, pset, days(year))
            est = False
        else:
            sample = days(year, 7)
            pyc, py = python_counts(puzzle, pset, sample)
            py, est = py * len(rsc) / len(sample), True
        assert all(rsc[k] == v for k, v in pyc.items()), 'python/rust count mismatch!'
        row(f'{puzzle} {pset}', py, dlx, bits, est)

    if os.path.exists(WASM):
        print('\nWASM in node (single core):')
        for i, (puzzle, pset) in enumerate(puzzles):
            res = wasm(i, 0, date)
            print(f'  {puzzle} {pset}: one date {res["single"]:.1f}ms, full year '
                  f'{res["year_bits"]:.2f}s (dlx {res["year_dlx"]:.2f}s), {res["total"]} sols')
