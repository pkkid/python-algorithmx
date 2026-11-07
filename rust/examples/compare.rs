//! Time each optimization step over a full year of dates.
//!
//!   cargo run --release --example compare [year]
//!   QUICK=1 cargo run --release --example compare     # skip the slow DLX run
use puzzle_a_day::*;
use std::time::Instant;

fn main() {
    let year: i64 = std::env::args().nth(1).and_then(|y| y.parse().ok()).unwrap_or(2025);
    let quick = std::env::var("QUICK").is_ok();
    let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
    let dates: Vec<(u32, u32)> =
        (1..=12).flat_map(|m| (1..=days_in_month(year, m)).map(move |d| (m, d))).collect();
    for puzzle in builtin_puzzles() {
        let pieces = &puzzle.sets[0].1;
        let blocked: Vec<Vec<Cell>> =
            dates.iter().map(|&(m, d)| date_cells(&puzzle, m, d, weekday(year, m, d))).collect();
        println!("{} {}, every day of {year}:", puzzle.name, puzzle.sets[0].0);
        let time = |label: &str, f: &dyn Fn() -> usize| {
            let t = Instant::now();
            let total = f();
            println!("  {label:<46} {:8.3}s  ({total} solutions)", t.elapsed().as_secs_f64());
        };
        if !quick {
            let dlx = Solver::new(&puzzle, pieces, true);
            time("1 core: Algorithm X / DLX (Python's algorithm)", &|| {
                blocked.iter().map(|b| dlx.count(b)).sum()
            });
        }
        let mut bits = BitSolver::with_order(&puzzle, pieces, true, 0);
        bits.prune = false;
        time("1 core: bitboard, one search per date", &|| {
            blocked.iter().map(|b| bits.count(b)).sum()
        });
        bits.prune = true;
        time("1 core:  + prune unfillable pockets", &|| {
            blocked.iter().map(|b| bits.count(b)).sum()
        });
        let bits = BitSolver::new(&puzzle, pieces, true);
        time("1 core:  + fill from bottom-right corner", &|| {
            blocked.iter().map(|b| bits.count(b)).sum()
        });
        let strategy = if puzzle.open_cells().len() >= 50 {
            "memo shared per month"
        } else {
            "all dates in one search"
        };
        time(&format!("1 core:  + {strategy}"), &|| {
            bits.count_year(&puzzle, year, 1).iter().map(|(_, n)| n).sum()
        });
        time(&format!("{cores} cores: same"), &|| {
            bits.count_year(&puzzle, year, cores).iter().map(|(_, n)| n).sum()
        });
    }
}
