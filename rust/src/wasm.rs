//! WebAssembly exports for the browser demo.
//!
//! No wasm-bindgen: every function takes plain integers and writes its result
//! as UTF-8 JSON into an output buffer. JavaScript reads it with
//! `new Uint8Array(memory.buffer, out_ptr(), len)`.
use crate::*;
use std::cell::RefCell;
use std::fmt::Write;

thread_local! {
    static OUT: RefCell<String> = RefCell::new(String::new());
    static PUZZLES: Vec<Puzzle> = builtin_puzzles();
    static SOLVER: RefCell<Option<((u32, u32, bool), BitSolver)>> = const { RefCell::new(None) };
}

fn set_output(s: String) -> u32 {
    OUT.with(|out| {
        let len = s.len() as u32;
        *out.borrow_mut() = s;
        len
    })
}

fn cells_json(out: &mut String, cells: &[Cell]) {
    out.push('[');
    for (i, (r, c)) in cells.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let _ = write!(out, "[{r},{c}]");
    }
    out.push(']');
}

/// Pointer to the output of the last call.
#[unsafe(no_mangle)]
pub extern "C" fn out_ptr() -> *const u8 {
    OUT.with(|out| out.borrow().as_ptr())
}

/// Describe the built-in puzzles. Returns the JSON length:
/// `[{"name":..,"board":[[r,c]..],"sets":[{"name":..,"pieces":[{"color":..,"cells":[[r,c]..]}]}]}]`
#[unsafe(no_mangle)]
pub extern "C" fn puzzles_json() -> u32 {
    let mut out = String::from("[");
    PUZZLES.with(|puzzles| {
        for (i, p) in puzzles.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            let _ = write!(
                out,
                "{{\"name\":\"{}\",\"rows\":{},\"cols\":{},\"board\":",
                p.name,
                p.rows(),
                p.cols()
            );
            cells_json(&mut out, &p.open_cells());
            out.push_str(",\"sets\":[");
            for (j, (name, pieces)) in p.sets.iter().enumerate() {
                if j > 0 {
                    out.push(',');
                }
                let _ = write!(out, "{{\"name\":\"{name}\",\"pieces\":[");
                for (k, piece) in pieces.iter().enumerate() {
                    if k > 0 {
                        out.push(',');
                    }
                    let _ = write!(out, "{{\"color\":\"{}\",\"cells\":", piece.color);
                    cells_json(&mut out, &piece.cells);
                    out.push('}');
                }
                out.push_str("]}");
            }
            out.push_str("]}");
        }
    });
    out.push(']');
    set_output(out)
}

/// Solve a date. `weekday` is Sunday=1..Saturday=7 (ignored by boards without
/// weekday squares). `limit` of 0 returns every solution. Returns the JSON
/// length: `{"count":N,"solutions":[[cells of piece 0, cells of piece 1, ..], ..]}`
#[unsafe(no_mangle)]
pub extern "C" fn solve(
    puzzle: u32,
    set: u32,
    month: u32,
    day: u32,
    weekday: u32,
    reflections: u32,
    limit: u32,
) -> u32 {
    let reflections = reflections != 0;
    let json = PUZZLES.with(|puzzles| {
        let Some(p) = puzzles.get(puzzle as usize) else {
            return "{\"error\":\"unknown puzzle\"}".to_string();
        };
        let Some((_, pieces)) = p.sets.get(set as usize) else {
            return "{\"error\":\"unknown set\"}".to_string();
        };
        SOLVER.with(|cache| {
            let mut cache = cache.borrow_mut();
            let key = (puzzle, set, reflections);
            if cache.as_ref().is_none_or(|(k, _)| *k != key) {
                *cache = Some((key, BitSolver::new(p, pieces, reflections)));
            }
            let solver = &cache.as_ref().unwrap().1;
            let solutions = solver.solve(&date_cells(p, month, day, weekday), limit as usize);
            let mut out = format!("{{\"count\":{},\"solutions\":[", solutions.len());
            for (i, solution) in solutions.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                // Order the placements by piece index.
                let mut by_piece: Vec<&Placement> = solution
                    .iter()
                    .map(|&s| &solver.base.placements[s])
                    .collect();
                by_piece.sort_by_key(|pl| pl.piece);
                out.push('[');
                for (k, pl) in by_piece.iter().enumerate() {
                    if k > 0 {
                        out.push(',');
                    }
                    cells_json(&mut out, &pl.cells);
                }
                out.push(']');
            }
            out.push_str("]}");
            out
        })
    });
    set_output(json)
}

/// Count solutions for every day of `year` (for benchmarking). `algo` 0 uses
/// the bitboard solver, 1 the original Algorithm X (DLX) port.
#[unsafe(no_mangle)]
pub extern "C" fn count_year(puzzle: u32, set: u32, year: i32, reflections: u32, algo: u32) -> u32 {
    PUZZLES.with(|puzzles| {
        let p = &puzzles[puzzle as usize];
        let pieces = &p.sets[set as usize].1;
        if algo == 0 {
            let bits = BitSolver::new(p, pieces, reflections != 0);
            return bits
                .count_year(p, year as i64, 1)
                .iter()
                .map(|(_, n)| n)
                .sum::<usize>() as u32;
        }
        let solver = Solver::new(p, pieces, reflections != 0);
        let mut total = 0;
        for m in 1..=12 {
            for d in 1..=days_in_month(year as i64, m) {
                total += solver.count(&date_cells(p, m, d, crate::weekday(year as i64, m, d)));
            }
        }
        total as u32
    })
}
