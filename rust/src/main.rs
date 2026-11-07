//! Command line solver, mirrors `puzzle-a-day.py`.
//!
//!   puzzle-a-day --date=2025-05-29
//!   puzzle-a-day -p guanglu -s set1 -y 2025 --procs=16
use puzzle_a_day::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

struct Opts {
    puzzle: String,
    set: String,
    date: Option<(i64, u32, u32)>,
    year: Option<i64>,
    reflections: bool,
    procs: usize,
    all: bool,
    csv: bool,
    dlx: bool,
    ini: Option<String>,
}

const HELP: &str = "Solve the Dragonfjord puzzle.

Options:
  -p, --puzzle NAME     Name of the puzzle to solve (default: dragonfjord)
  -s, --set NAME        Piece set to use (default: set1)
  -d, --date YYYY-MM-DD Date to solve for (default: today, UTC)
  -y, --year YEAR       Run for every day of the year
      --no-reflections  Dont allow pieces to be flipped
      --procs N         Threads to use for year mode (default: all cores)
      --all             Print every solution instead of a random one
      --csv             Year mode: print date,count lines instead of a calendar
      --algo NAME       bits (default, bitboard search) or dlx (Algorithm X, as in Python)
      --ini PATH        Read puzzles from this file instead of the built-in copy
  -h, --help            Show this help";

fn parse_args() -> Result<Opts, String> {
    let mut opts = Opts {
        puzzle: "dragonfjord".into(),
        set: "set1".into(),
        date: None,
        year: None,
        reflections: true,
        procs: std::thread::available_parallelism().map_or(4, |n| n.get()),
        all: false,
        csv: false,
        dlx: false,
        ini: None,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let (flag, inline) = match arg.split_once('=') {
            Some((f, v)) if f.starts_with("--") => (f.to_string(), Some(v.to_string())),
            _ => (arg.clone(), None),
        };
        let mut value = || {
            inline
                .clone()
                .or_else(|| args.next())
                .ok_or(format!("{flag} needs a value"))
        };
        match flag.as_str() {
            "-p" | "--puzzle" => opts.puzzle = value()?,
            "-s" | "--set" => opts.set = value()?,
            "-d" | "--date" => opts.date = Some(parse_date(&value()?)?),
            "-y" | "--year" => opts.year = Some(value()?.parse().map_err(|_| "bad year")?),
            "--procs" => opts.procs = value()?.parse().map_err(|_| "bad procs")?,
            "--ini" => opts.ini = Some(value()?),
            "--no-reflections" => opts.reflections = false,
            "--all" => opts.all = true,
            "--csv" => opts.csv = true,
            "--algo" => match value()?.as_str() {
                "dlx" => opts.dlx = true,
                "bits" => opts.dlx = false,
                other => return Err(format!("unknown algo {other}, expected dlx or bits")),
            },
            "-h" | "--help" => {
                println!("{HELP}");
                std::process::exit(0);
            }
            _ => return Err(format!("unknown argument: {arg}\n\n{HELP}")),
        }
    }
    Ok(opts)
}

fn parse_date(s: &str) -> Result<(i64, u32, u32), String> {
    let parts: Vec<&str> = s.split('-').collect();
    let bad = || format!("bad date {s}, expected YYYY-MM-DD");
    if parts.len() != 3 {
        return Err(bad());
    }
    let y: i64 = parts[0].parse().map_err(|_| bad())?;
    let m: u32 = parts[1].parse().map_err(|_| bad())?;
    let d: u32 = parts[2].parse().map_err(|_| bad())?;
    if !(1..=12).contains(&m) || d == 0 || d > days_in_month(y, m) {
        return Err(bad());
    }
    Ok((y, m, d))
}

fn today() -> (i64, u32, u32) {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    civil_from_days((secs / 86400) as i64)
}

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

fn count_each_date(
    puzzle: &Puzzle,
    solver: &Solver,
    year: i64,
    dates: &[(u32, u32)],
    procs: usize,
) -> Vec<usize> {
    let counts: Vec<AtomicUsize> = dates.iter().map(|_| AtomicUsize::new(0)).collect();
    let next = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        for _ in 0..procs.max(1) {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(&(m, d)) = dates.get(i) else { break };
                    let blocked = date_cells(puzzle, m, d, weekday(year, m, d));
                    counts[i].store(solver.count(&blocked), Ordering::Relaxed);
                }
            });
        }
    });
    counts.iter().map(|c| c.load(Ordering::Relaxed)).collect()
}

fn run_year(
    puzzle: &Puzzle,
    solver: &Solver,
    bits: Option<&BitSolver>,
    year: i64,
    procs: usize,
    csv: bool,
) -> usize {
    let mut dates = Vec::new();
    for m in 1..=12 {
        for d in 1..=days_in_month(year, m) {
            dates.push((m, d));
        }
    }
    let counts: Vec<usize> = if let Some(bits) = bits {
        bits.count_year(puzzle, year, procs)
            .into_iter()
            .map(|(_, n)| n)
            .collect()
    } else {
        count_each_date(puzzle, solver, year, &dates, procs)
    };
    if csv {
        for (&(m, d), &count) in dates.iter().zip(&counts) {
            println!("{year}-{m:02}-{d:02},{count}");
        }
        return counts.iter().sum();
    }
    // Print a calendar of counts, same layout as puzzle-a-day.py (Monday first).
    for (&(m, d), &count) in dates.iter().zip(&counts) {
        let monday0 = (weekday(year, m, d) + 5) % 7;
        if d == 1 {
            print!(
                "\n\n{}\n{}",
                MONTHS[m as usize - 1],
                "     ".repeat(monday0 as usize)
            );
        } else if monday0 == 0 {
            println!();
        }
        if count == 0 {
            print!("\x1b[38;2;204;0;0m{count:4} \x1b[00m");
        } else {
            print!("{count:4} ");
        }
    }
    let total: usize = counts.iter().sum();
    let avg = total as f64 / counts.len() as f64;
    let min = counts.iter().min().unwrap();
    let max = counts.iter().max().unwrap();
    let zero = counts.iter().filter(|&&c| c == 0).count();
    println!("\n\ntotal:{total}, avg:{avg:.2}, min:{min}, max:{max}, zero:{zero}");
    total
}

fn main() {
    let opts = match parse_args() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    let start = Instant::now();
    let puzzles = match &opts.ini {
        Some(path) => std::fs::read_to_string(path)
            .map_err(|e| e.to_string())
            .and_then(|text| parse_ini(&text)),
        None => Ok(builtin_puzzles()),
    };
    let puzzles = puzzles.unwrap_or_else(|e| {
        eprintln!("error reading puzzles: {e}");
        std::process::exit(1);
    });
    let Some(puzzle) = puzzles.iter().find(|p| p.name == opts.puzzle) else {
        eprintln!("unknown puzzle {}", opts.puzzle);
        std::process::exit(1);
    };
    let Some(pieces) = puzzle.set(&opts.set) else {
        eprintln!("unknown set {} for puzzle {}", opts.set, opts.puzzle);
        std::process::exit(1);
    };
    let solver = Solver::new(puzzle, pieces, opts.reflections);
    let bits = (!opts.dlx).then(|| BitSolver::new(puzzle, pieces, opts.reflections));

    let total = if let Some(year) = opts.year {
        run_year(puzzle, &solver, bits.as_ref(), year, opts.procs, opts.csv)
    } else {
        let (y, m, d) = opts.date.unwrap_or_else(today);
        let blocked = date_cells(puzzle, m, d, weekday(y, m, d));
        let solutions = match &bits {
            Some(bits) => bits.solve(&blocked, 0),
            None => solver.solve(&blocked, 0),
        };
        if opts.all {
            for s in &solutions {
                println!("{}", solver.render(s));
            }
        } else if !solutions.is_empty() {
            let seed = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.subsec_nanos());
            println!(
                "{}",
                solver.render(&solutions[seed as usize % solutions.len()])
            );
        }
        solutions.len()
    };
    let secs = start.elapsed().as_secs_f64();
    println!("Found {total} solutions after {secs:.3}s");
}
