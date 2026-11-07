//! Explore piece sets for the Dragonfjord board.
//!
//!   explore analyze [--set set1] [--trials 200] [--out FILE]
//!       Solutions and difficulty for every date of an existing set.
//!   explore search [--trials 10] [--out FILE]
//!       Every set of 7 pentominoes + 1 hexomino: which work all 366 dates,
//!       and how hard each one is.
//!   explore rate "H02 Y V U Z L P N" [--trials 200]
//!       Summary for one set of named pieces (pentomino letters, H01..H35).
//!   explore sides --from results/dragonfjord-piece-sets.csv [--out FILE]
//!       One-sided pieces: paint one face red, the other blue, and never flip.
//!       For each set that works with flipping, try every way of painting the
//!       mirror-asymmetric pieces and report paintings that solve every date
//!       red side up, and ones that also solve every date blue side up.
//!   explore onesided [--out FILE]
//!       The full one-sided family: 7 of the 18 one-sided pentominoes (mirror
//!       images count as different pieces) + 1 of the 60 one-sided hexominoes,
//!       no flipping. Finds sets that solve every date, and whether their
//!       mirror image (the blue side) does too.
//!
//! Difficulty is the mean number of wrong placements made by the simulated
//! human solver in `puzzle_a_day::difficulty`.
use puzzle_a_day::difficulty::{Human, spearman};
use puzzle_a_day::polyomino::{art, free_polyominoes, name};
use puzzle_a_day::*;
use std::io::Write;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

const COLORS: [char; 10] = ['🟪', '🟦', '🟩', '🟨', '🟧', '🟥', '🟫', '⬜', '🔵', '🔴'];

fn arg(name: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1).cloned())
}

fn threads() -> usize {
    arg("--threads")
        .and_then(|t| t.parse().ok())
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(4, |n| n.get()))
}

/// Every month/day pair, including Feb 29.
fn dates() -> Vec<(u32, u32)> {
    (1..=12).flat_map(|m| (1..=days_in_month(2024, m)).map(move |d| (m, d))).collect()
}

fn pieces_from(shapes: &[Vec<Cell>]) -> Vec<Piece> {
    shapes
        .iter()
        .enumerate()
        .map(|(i, cells)| Piece { color: COLORS[i % COLORS.len()], cells: cells.clone() })
        .collect()
}

fn names(pieces: &[Piece]) -> String {
    pieces.iter().map(|p| name(&p.cells)).collect::<Vec<_>>().join(" ")
}

fn board() -> Puzzle {
    builtin_puzzles().into_iter().find(|p| p.name == "dragonfjord").unwrap()
}

/// Run `f(i)` for i in 0..n on all threads, returning results in order.
fn par_map<R: Send>(n: usize, f: impl Fn(usize) -> R + Sync) -> Vec<R> {
    let next = AtomicUsize::new(0);
    let out: Mutex<Vec<Option<R>>> = Mutex::new((0..n).map(|_| None).collect());
    std::thread::scope(|s| {
        for _ in 0..threads() {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    if i >= n {
                        break;
                    }
                    let r = f(i);
                    out.lock().unwrap()[i] = Some(r);
                }
            });
        }
    });
    out.into_inner().unwrap().into_iter().map(Option::unwrap).collect()
}

struct Report {
    names: String,
    fail_dates: usize,
    min: usize,
    mean: f64,
    unique_days: usize,
    /// Per-date mean wrong placements (only when every date is solvable).
    wrong: Vec<f64>,
    forced: f64,
}

impl Report {
    fn wrong_mean(&self) -> f64 {
        self.wrong.iter().sum::<f64>() / self.wrong.len().max(1) as f64
    }
    fn wrong_pct(&self, p: f64) -> f64 {
        let mut w = self.wrong.clone();
        w.sort_by(|a, b| a.partial_cmp(b).unwrap());
        w.get(((w.len() as f64 - 1.0) * p).round() as usize).copied().unwrap_or(0.0)
    }
}

fn seed(m: u32, d: u32) -> u64 {
    0x5EED_0000 + m as u64 * 100 + d as u64
}

fn evaluate(board: &Puzzle, pieces: &[Piece], trials: u32) -> Report {
    let bits = BitSolver::new(board, pieces, true);
    let all = bits.count_all_dates(1);
    let counts: Vec<usize> = dates().iter().map(|&(m, d)| all.get(m, d, 0)).collect();
    let mut report = Report {
        names: names(pieces),
        fail_dates: counts.iter().filter(|&&c| c == 0).count(),
        min: *counts.iter().min().unwrap(),
        mean: counts.iter().sum::<usize>() as f64 / counts.len() as f64,
        unique_days: counts.iter().filter(|&&c| c == 1).count(),
        wrong: Vec::new(),
        forced: 0.0,
    };
    if report.fail_dates == 0 && trials > 0 {
        let human = Human::new(&bits);
        let mut forced = 0.0;
        for &(m, d) in &dates() {
            let r = human.rate(bits.blocked_mask(&date_cells(board, m, d, 0)), trials, seed(m, d), false);
            report.wrong.push(r.wrong);
            forced += r.forced;
        }
        report.forced = forced / dates().len() as f64;
    }
    report
}

fn analyze() {
    let board = board();
    let set = arg("--set").unwrap_or("set1".into());
    let trials: u32 = arg("--trials").and_then(|t| t.parse().ok()).unwrap_or(200);
    let pieces = board.set(&set).expect("unknown set").to_vec();
    let bits = BitSolver::new(&board, &pieces, true);
    let all = bits.count_all_dates(threads());
    let human = Human::new(&bits);
    let dates = dates();
    eprintln!("Rating {} dates for {set} ({}), {trials} trials each...", dates.len(), names(&pieces));
    let rows = par_map(dates.len(), |i| {
        let (m, d) = dates[i];
        let blocked = bits.blocked_mask(&date_cells(&board, m, d, 0));
        let skilled = human.rate(blocked, trials, seed(m, d), false);
        let naive = human.rate(blocked, (trials / 4).max(1), seed(m, d), true);
        (m, d, all.get(m, d, 0), skilled, naive)
    });
    let mut out: Box<dyn Write> = match arg("--out") {
        Some(path) => Box::new(std::fs::File::create(path).unwrap()),
        None => Box::new(std::io::sink()),
    };
    writeln!(out, "month,day,solutions,wrong,forced,naive_wrong").unwrap();
    for (m, d, n, s, v) in &rows {
        writeln!(out, "{m},{d},{n},{:.2},{:.4},{:.1}", s.wrong, s.forced, v.wrong).unwrap();
    }
    let sols: Vec<f64> = rows.iter().map(|r| r.2 as f64).collect();
    let wrong: Vec<f64> = rows.iter().map(|r| r.3.wrong).collect();
    let forced: Vec<f64> = rows.iter().map(|r| r.3.forced).collect();
    let naive: Vec<f64> = rows.iter().map(|r| r.4.wrong).collect();
    println!("Spearman correlation over {} dates:", rows.len());
    println!("  solutions vs wrong placements (skilled): {:+.2}", spearman(&sols, &wrong));
    println!("  solutions vs wrong placements (naive):   {:+.2}", spearman(&sols, &naive));
    println!("  solutions vs forced-move share:          {:+.2}", spearman(&sols, &forced));
    println!("  forced-move share vs wrong (skilled):    {:+.2}", spearman(&forced, &wrong));
    let mut by: Vec<usize> = (0..rows.len()).collect();
    by.sort_by(|&a, &b| wrong[b].partial_cmp(&wrong[a]).unwrap());
    let show = |i: usize| {
        let (m, d, n, s, v) = &rows[i];
        println!("  {m:>2}/{d:<2}  {n:>4} solutions  {:>6.1} wrong  {:>4.0}% forced  naive {:>8.0}", s.wrong, s.forced * 100.0, v.wrong);
    };
    println!("Hardest dates:");
    by.iter().take(6).for_each(|&i| show(i));
    println!("Easiest dates:");
    by.iter().rev().take(6).for_each(|&i| show(i));
    println!("Mean wrong placements: {:.1}", wrong.iter().sum::<f64>() / wrong.len() as f64);
}

fn search() {
    let board = board();
    let trials: u32 = arg("--trials").and_then(|t| t.parse().ok()).unwrap_or(10);
    let pents = free_polyominoes(5);
    let hexes = free_polyominoes(6);
    let mut sets: Vec<Vec<Vec<Cell>>> = Vec::new();
    for mask in 0u32..1 << 12 {
        if mask.count_ones() != 7 {
            continue;
        }
        for hex in &hexes {
            let mut shapes = vec![hex.clone()];
            shapes.extend((0..12).filter(|i| mask & 1 << i != 0).map(|i| pents[i].clone()));
            sets.push(shapes);
        }
    }
    eprintln!("Testing {} sets on {} threads...", sets.len(), threads());
    let done = AtomicUsize::new(0);
    let start = std::time::Instant::now();
    let reports = par_map(sets.len(), |i| {
        let r = evaluate(&board, &pieces_from(&sets[i]), trials);
        let n = done.fetch_add(1, Ordering::Relaxed) + 1;
        if n % 1000 == 0 {
            eprintln!("  {n}/{} sets, {:.0}s", sets.len(), start.elapsed().as_secs_f64());
        }
        r
    });
    let mut out: Box<dyn Write> = match arg("--out") {
        Some(path) => Box::new(std::fs::File::create(path).unwrap()),
        None => Box::new(std::io::stdout()),
    };
    writeln!(out, "pieces,fail_dates,min_solutions,mean_solutions,unique_days,wrong_mean,wrong_p90,wrong_max,forced").unwrap();
    for r in &reports {
        writeln!(
            out,
            "{},{},{},{:.1},{},{:.2},{:.2},{:.2},{:.4}",
            r.names,
            r.fail_dates,
            r.min,
            r.mean,
            r.unique_days,
            r.wrong_mean(),
            r.wrong_pct(0.9),
            r.wrong_pct(1.0),
            r.forced
        )
        .unwrap();
    }
    let passing = reports.iter().filter(|r| r.fail_dates == 0).count();
    eprintln!("{passing} of {} sets solve every date. Took {:.0}s", reports.len(), start.elapsed().as_secs_f64());
}

fn rate(list: &str) {
    let board = board();
    let trials: u32 = arg("--trials").and_then(|t| t.parse().ok()).unwrap_or(200);
    let mut library: Vec<Vec<Cell>> = Vec::new();
    for n in 4..=6 {
        library.extend(free_polyominoes(n));
    }
    let shapes: Vec<Vec<Cell>> = list
        .split_whitespace()
        .map(|n| library.iter().find(|s| name(s) == n).unwrap_or_else(|| panic!("unknown piece {n}")).clone())
        .collect();
    let pieces = pieces_from(&shapes);
    let r = evaluate(&board, &pieces, trials);
    for p in &pieces {
        println!("  {:>3}  {}", name(&p.cells), art(&p.cells));
    }
    println!(
        "{}: fails {} dates, solutions min {} mean {:.1}, {} days with a unique solution",
        r.names, r.fail_dates, r.min, r.mean, r.unique_days
    );
    if !r.wrong.is_empty() {
        println!(
            "wrong placements: mean {:.1}, p90 {:.1}, max {:.1}; forced moves {:.0}%",
            r.wrong_mean(),
            r.wrong_pct(0.9),
            r.wrong_pct(1.0),
            r.forced * 100.0
        );
    }
}

/// Solve every date without flipping. Returns (unsolvable dates, min, mean solutions).
fn one_sided(board: &Puzzle, pieces: &[Piece]) -> (usize, usize, f64) {
    let all = BitSolver::new(board, pieces, false).count_all_dates(1);
    let counts: Vec<usize> = dates().iter().map(|&(m, d)| all.get(m, d, 0)).collect();
    let fails = counts.iter().filter(|&&c| c == 0).count();
    (fails, *counts.iter().min().unwrap(), counts.iter().sum::<usize>() as f64 / counts.len() as f64)
}

fn mirror(cells: &[Cell]) -> Vec<Cell> {
    let w = cells.iter().map(|c| c.1).max().unwrap_or(0);
    normalize(&cells.iter().map(|&(r, c)| (r, w - c)).collect::<Vec<_>>())
}

fn is_chiral(cells: &[Cell]) -> bool {
    orientations(cells, true).len() == 2 * orientations(cells, false).len()
}

fn sides() {
    let board = board();
    let path = arg("--from").expect("--from <search csv>");
    let text = std::fs::read_to_string(&path).expect("read --from");
    let mut library: Vec<Vec<Cell>> = Vec::new();
    for n in 4..=6 {
        library.extend(free_polyominoes(n));
    }
    let lookup = |n: &str| library.iter().find(|s| name(s) == n).unwrap().clone();
    // Sets that work with flipping allowed (fail_dates == 0).
    let sets: Vec<Vec<Vec<Cell>>> = text
        .lines()
        .skip(1)
        .filter(|l| l.split(',').nth(1) == Some("0"))
        .map(|l| l.split(',').next().unwrap().split_whitespace().map(lookup).collect())
        .collect();
    eprintln!("Trying every painting of {} sets...", sets.len());
    // For each set: (mask, min, mean) for every painting that works red side up.
    let results = par_map(sets.len(), |i| {
        let shapes = &sets[i];
        let chiral: Vec<usize> = (0..shapes.len()).filter(|&k| is_chiral(&shapes[k])).collect();
        let mut works = Vec::new();
        let mut fails_by_mask = Vec::new();
        for mask in 0u32..1 << chiral.len() {
            let mut painted = shapes.clone();
            for (bit, &k) in chiral.iter().enumerate() {
                if mask & 1 << bit != 0 {
                    painted[k] = mirror(&painted[k]);
                }
            }
            let (fails, min, mean) = one_sided(&board, &pieces_from(&painted));
            if fails == 0 {
                works.push((mask, min, mean));
            }
            fails_by_mask.push(fails);
        }
        (chiral, works, fails_by_mask)
    });
    let mut out: Box<dyn Write> = match arg("--out") {
        Some(path) => Box::new(std::fs::File::create(path).unwrap()),
        None => Box::new(std::io::sink()),
    };
    writeln!(out, "pieces,red_side_up,min_solutions,mean_solutions,blue_side_also_works").unwrap();
    let (mut one, mut both) = (0, 0);
    // Closest misses: fewest unsolvable dates red side up, and for both sides.
    let paint = |shapes: &Vec<Vec<Cell>>, chiral: &[usize], mask: u32| -> String {
        let mut painted = shapes.clone();
        for (bit, &k) in chiral.iter().enumerate() {
            if mask & 1 << bit != 0 {
                painted[k] = mirror(&painted[k]);
            }
        }
        painted.iter().map(|c| art(c)).collect::<Vec<_>>().join(" ")
    };
    let mut red: Vec<(usize, String, String)> = Vec::new();
    let mut duo: Vec<(usize, usize, String, String)> = Vec::new();
    for (shapes, (chiral, _, fails)) in sets.iter().zip(&results) {
        let full = (1u32 << chiral.len()) - 1;
        let (mask, f) = fails.iter().enumerate().min_by_key(|x| *x.1).unwrap();
        red.push((*f, names(&pieces_from(shapes)), paint(shapes, chiral, mask as u32)));
        let (mask, worst) = (0..fails.len())
            .map(|m| (m, fails[m].max(fails[!(m as u32) as usize & full as usize])))
            .min_by_key(|x| x.1)
            .unwrap();
        let other = fails[!(mask as u32) as usize & full as usize];
        duo.push((worst, fails[mask].min(other), names(&pieces_from(shapes)), paint(shapes, chiral, mask as u32)));
    }
    red.sort();
    duo.sort();
    println!("Closest red-side-up paintings (unsolvable dates, set, red faces):");
    for r in red.iter().take(8) {
        println!("  {:>3}  {:<22} {}", r.0, r.1, r.2);
    }
    println!("Closest paintings for red AND blue (worse side, better side, set, red faces):");
    for d in duo.iter().take(8) {
        println!("  {:>3} {:>3}  {:<22} {}", d.0, d.1, d.2, d.3);
    }
    for (shapes, (chiral, works, _)) in sets.iter().zip(&results) {
        if works.is_empty() {
            continue;
        }
        one += 1;
        let full = (1u32 << chiral.len()) - 1;
        let mut set_both = false;
        for &(mask, min, mean) in works {
            let blue = works.iter().any(|w| w.0 == !mask & full);
            set_both |= blue;
            let mut painted = shapes.clone();
            for (bit, &k) in chiral.iter().enumerate() {
                if mask & 1 << bit != 0 {
                    painted[k] = mirror(&painted[k]);
                }
            }
            let red: Vec<String> = painted.iter().map(|c| art(c)).collect();
            writeln!(out, "{},{},{min},{mean:.1},{blue}", names(&pieces_from(shapes)), red.join(" ")).unwrap();
        }
        if set_both {
            both += 1;
        }
    }
    println!("{one} of {} sets have a painting that solves every date red side up", sets.len());
    println!("{both} sets have a painting that solves every date red side up AND blue side up");
}

/// Every shape of `n` squares with mirror images counted separately, plus
/// the index of each shape's mirror image in the same list.
fn one_sided_shapes(n: usize) -> (Vec<Vec<Cell>>, Vec<usize>) {
    let mut shapes: Vec<Vec<Cell>> = Vec::new();
    for free in free_polyominoes(n) {
        let rot = orientations(&free, false).into_iter().min().unwrap();
        shapes.push(rot.clone());
        let flipped = orientations(&mirror(&rot), false).into_iter().min().unwrap();
        if flipped != rot {
            shapes.push(flipped);
        }
    }
    let key = |c: &Vec<Cell>| orientations(c, false).into_iter().min().unwrap();
    let mirrors = shapes
        .iter()
        .map(|s| {
            let m = key(&mirror(s));
            shapes.iter().position(|t| key(t) == m).unwrap()
        })
        .collect();
    (shapes, mirrors)
}

fn onesided() {
    let board = board();
    let (pents, pmir) = one_sided_shapes(5);
    let (hexes, hmir) = one_sided_shapes(6);
    eprintln!("{} one-sided pentominoes, {} one-sided hexominoes", pents.len(), hexes.len());
    let mirror_set = |mask: u32, hex: usize| -> (u32, usize) {
        let m = (0..pents.len()).filter(|&i| mask & 1 << i != 0).fold(0u32, |m, i| m | 1 << pmir[i]);
        (m, hmir[hex])
    };
    // One job per set, skipping a set when its mirror image comes first (same answer).
    let mut jobs: Vec<(u32, usize)> = Vec::new();
    for mask in 0u32..1 << pents.len() {
        if mask.count_ones() == 7 {
            for hex in 0..hexes.len() {
                if (mask, hex) <= mirror_set(mask, hex) {
                    jobs.push((mask, hex));
                }
            }
        }
    }
    let build = |mask: u32, hex: usize| -> Vec<Piece> {
        let mut shapes = vec![hexes[hex].clone()];
        shapes.extend((0..pents.len()).filter(|&i| mask & 1 << i != 0).map(|i| pents[i].clone()));
        pieces_from(&shapes)
    };
    let all_dates = dates();
    // Does every date have a solution? Stops at the first date that doesn't, and
    // moves that date to the front so later sets fail fast.
    let solves_all = |pieces: &[Piece], order: &mut Vec<usize>| -> bool {
        let bits = BitSolver::new(&board, pieces, false);
        for k in 0..order.len() {
            let (m, d) = all_dates[order[k]];
            if bits.solve(&date_cells(&board, m, d, 0), 1).is_empty() {
                let i = order.remove(k);
                order.insert(0, i);
                return false;
            }
        }
        true
    };
    eprintln!("Testing {} sets (mirror pairs counted once) on {} threads...", jobs.len(), threads());
    let start = std::time::Instant::now();
    let chunks = threads() * 64;
    let per = jobs.len().div_ceil(chunks);
    let done = AtomicUsize::new(0);
    let found: Vec<Vec<(u32, usize)>> = par_map(chunks, |c| {
        let mut order: Vec<usize> = (0..all_dates.len()).collect();
        let mut hits = Vec::new();
        for &(mask, hex) in jobs.iter().skip(c * per).take(per) {
            if solves_all(&build(mask, hex), &mut order) {
                hits.push((mask, hex));
            }
        }
        let n = done.fetch_add(1, Ordering::Relaxed) + 1;
        if n % (chunks / 10).max(1) == 0 {
            eprintln!("  {}% {:.0}s", n * 100 / chunks, start.elapsed().as_secs_f64());
        }
        hits
    });
    let hits: Vec<(u32, usize)> = found.concat();
    let mut out: Box<dyn Write> = match arg("--out") {
        Some(path) => Box::new(std::fs::File::create(path).unwrap()),
        None => Box::new(std::io::sink()),
    };
    writeln!(out, "red_side_up,min_solutions,mean_solutions,blue_side_also_works").unwrap();
    let mut both = 0;
    for &(mask, hex) in &hits {
        let pieces = build(mask, hex);
        let (_, min, mean) = one_sided(&board, &pieces);
        let (mm, mh) = mirror_set(mask, hex);
        let blue = (mm, mh) == (mask, hex) || solves_all(&build(mm, mh), &mut (0..all_dates.len()).collect());
        both += blue as usize;
        let red: Vec<String> = pieces.iter().map(|p| art(&p.cells)).collect();
        writeln!(out, "{},{min},{mean:.1},{blue}", red.join(" ")).unwrap();
        println!("  {} (min {min}, mean {mean:.1}){}", red.join(" "), if blue { "  BLUE SIDE TOO" } else { "" });
    }
    println!(
        "{} sets solve every date red side up ({} also blue side up). Took {:.0}s",
        hits.len(),
        both,
        start.elapsed().as_secs_f64()
    );
}

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("analyze") => analyze(),
        Some("search") => search(),
        Some("sides") => sides(),
        Some("onesided") => onesided(),
        Some("rate") => rate(&std::env::args().nth(2).expect("piece list")),
        _ => eprintln!("usage: explore analyze|search|rate|sides|onesided (see source for options)"),
    }
}
