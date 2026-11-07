//! A Puzzle a Day solver.
//!
//! Rust port of `python/pentomino.py` / `python/puzzle-a-day.py`. Pieces are placed on the board
//! by solving an exact cover problem with Knuth's Algorithm X, implemented with
//! Dancing Links (DLX). The same library builds as a native CLI (`src/main.rs`)
//! and as a WebAssembly module (`src/wasm.rs`) for the browser demo.

mod bits;
pub mod difficulty;
pub mod polyomino;
pub use bits::{BitSolver, DateCounts};

#[cfg(target_arch = "wasm32")]
mod wasm;

/// Character that marks an empty (non-board / non-piece) square.
pub const EMPTY: char = '⬛';

/// The puzzle definitions shipped with this repo, embedded at compile time.
pub const PUZZLES_INI: &str = include_str!("../../puzzle-a-day.ini");

/// A (row, col) coordinate.
pub type Cell = (usize, usize);

#[derive(Clone, Debug)]
pub struct Piece {
    pub color: char,
    pub cells: Vec<Cell>,
}

#[derive(Clone, Debug)]
pub struct Puzzle {
    pub name: String,
    /// `true` where the board has a playable square.
    pub board: Vec<Vec<bool>>,
    /// Named piece sets, in the order they appear in the ini file.
    pub sets: Vec<(String, Vec<Piece>)>,
}

impl Puzzle {
    pub fn rows(&self) -> usize {
        self.board.len()
    }

    pub fn cols(&self) -> usize {
        self.board.iter().map(Vec::len).max().unwrap_or(0)
    }

    /// Playable squares in row-major order (the order dates are assigned in).
    pub fn open_cells(&self) -> Vec<Cell> {
        let mut cells = Vec::new();
        for (r, row) in self.board.iter().enumerate() {
            for (c, &open) in row.iter().enumerate() {
                if open {
                    cells.push((r, c));
                }
            }
        }
        cells
    }

    pub fn set(&self, name: &str) -> Option<&[Piece]> {
        self.sets
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, p)| p.as_slice())
    }
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Parse the emoji puzzle definitions from an ini file (same format as
/// `puzzle-a-day.ini`, read by Python's configparser).
pub fn parse_ini(text: &str) -> Result<Vec<Puzzle>, String> {
    let mut sections: Vec<(String, Vec<(String, String)>)> = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            sections.push((trimmed[1..trimmed.len() - 1].to_string(), Vec::new()));
            continue;
        }
        let Some((_, keys)) = sections.last_mut() else {
            return Err(format!("value outside of a section: {line}"));
        };
        if line.starts_with(char::is_whitespace) {
            // Continuation line of a multi-line value.
            let (_, value) = keys.last_mut().ok_or("continuation line without key")?;
            value.push('\n');
            value.push_str(trimmed);
        } else if let Some((key, value)) = line.split_once('=') {
            keys.push((key.trim().to_string(), value.trim().to_string()));
        } else {
            return Err(format!("could not parse line: {line}"));
        }
    }
    let mut puzzles = Vec::new();
    for (name, keys) in sections {
        let board_text = keys
            .iter()
            .find(|(k, _)| k == "board")
            .map(|(_, v)| v.as_str())
            .ok_or_else(|| format!("puzzle {name} has no board"))?;
        let board = parse_board(board_text);
        let mut sets = Vec::new();
        for (key, value) in &keys {
            if key != "board" {
                sets.push((key.clone(), parse_pieces(value)?));
            }
        }
        puzzles.push(Puzzle { name, board, sets });
    }
    Ok(puzzles)
}

/// Load the puzzles embedded in the binary.
pub fn builtin_puzzles() -> Vec<Puzzle> {
    parse_ini(PUZZLES_INI).expect("embedded puzzle-a-day.ini is valid")
}

fn grid_lines(text: &str) -> impl Iterator<Item = &str> {
    text.lines().map(str::trim).filter(|l| !l.is_empty())
}

/// Parse a board drawn as one emoji string per line.
pub fn parse_board(text: &str) -> Vec<Vec<bool>> {
    grid_lines(text)
        .map(|line| line.chars().map(|ch| ch != EMPTY).collect())
        .collect()
}

/// Parse a row of pieces drawn side by side, separated by spaces.
pub fn parse_pieces(text: &str) -> Result<Vec<Piece>, String> {
    let rows: Vec<Vec<Vec<char>>> = grid_lines(text)
        .map(|line| {
            line.split_whitespace()
                .map(|tok| tok.chars().collect())
                .collect()
        })
        .collect();
    let count = rows.first().map_or(0, Vec::len);
    if rows.iter().any(|r| r.len() != count) {
        return Err("every line of a piece set needs the same number of pieces".into());
    }
    let mut pieces = Vec::with_capacity(count);
    for i in 0..count {
        let mut cells = Vec::new();
        let mut color = EMPTY;
        for (r, row) in rows.iter().enumerate() {
            for (c, &ch) in row[i].iter().enumerate() {
                if ch != EMPTY {
                    cells.push((r, c));
                    color = ch;
                }
            }
        }
        if cells.is_empty() {
            return Err(format!("piece {i} is empty"));
        }
        pieces.push(Piece {
            color,
            cells: normalize(&cells),
        });
    }
    Ok(pieces)
}

// ---------------------------------------------------------------------------
// Shapes
// ---------------------------------------------------------------------------

/// Shift cells so the bounding box starts at (0,0), and sort them.
pub fn normalize(cells: &[Cell]) -> Vec<Cell> {
    let minr = cells.iter().map(|c| c.0).min().unwrap_or(0);
    let minc = cells.iter().map(|c| c.1).min().unwrap_or(0);
    let mut out: Vec<Cell> = cells.iter().map(|&(r, c)| (r - minr, c - minc)).collect();
    out.sort_unstable();
    out
}

fn transform(cells: &[Cell], f: impl Fn(i64, i64) -> (i64, i64)) -> Vec<Cell> {
    let moved: Vec<(i64, i64)> = cells.iter().map(|&(r, c)| f(r as i64, c as i64)).collect();
    let minr = moved.iter().map(|c| c.0).min().unwrap_or(0);
    let minc = moved.iter().map(|c| c.1).min().unwrap_or(0);
    let mut out: Vec<Cell> = moved
        .iter()
        .map(|&(r, c)| ((r - minr) as usize, (c - minc) as usize))
        .collect();
    out.sort_unstable();
    out
}

/// Every unique orientation of a piece (4 rotations, plus 4 mirrored ones if
/// reflections are allowed).
pub fn orientations(cells: &[Cell], reflections: bool) -> Vec<Vec<Cell>> {
    let mut out: Vec<Vec<Cell>> = Vec::new();
    let mut shape = normalize(cells);
    for i in 0..if reflections { 8 } else { 4 } {
        shape = if i == 4 {
            transform(&shape, |r, c| (r, -c)) // mirror
        } else {
            transform(&shape, |r, c| (c, -r)) // rotate 90 degrees
        };
        if !out.contains(&shape) {
            out.push(shape.clone());
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Dates
// ---------------------------------------------------------------------------

pub fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

pub fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if is_leap(year) => 29,
        _ => 28,
    }
}

/// Days since 1970-01-01 (Howard Hinnant's algorithm).
pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Inverse of [`days_from_civil`].
pub fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + if m <= 2 { 1 } else { 0 }, m, d)
}

/// Day of the week with Sunday = 1 ... Saturday = 7 (matches puzzle-a-day.py).
pub fn weekday(y: i64, m: u32, d: u32) -> u32 {
    ((days_from_civil(y, m, d) + 4).rem_euclid(7) + 1) as u32
}

/// The squares left uncovered for a date. Board squares are numbered in
/// row-major order: 1-12 are months, 13-43 days, 44-50 weekdays (if present).
/// Pass `weekday` 0 to ignore weekday squares.
pub fn date_cells(puzzle: &Puzzle, month: u32, day: u32, weekday: u32) -> Vec<Cell> {
    let mut cells = Vec::new();
    for (i, cell) in puzzle.open_cells().into_iter().enumerate() {
        let n = i as u32 + 1;
        if n == month || n == day + 12 || (weekday > 0 && n == weekday + 43) {
            cells.push(cell);
        }
    }
    cells
}

// ---------------------------------------------------------------------------
// Solver
// ---------------------------------------------------------------------------

/// One way to put a piece on the board.
#[derive(Clone, Debug)]
pub struct Placement {
    pub piece: usize,
    pub cells: Vec<Cell>,
}

/// Precomputes every legal placement of every piece on the empty board. Each
/// date then only needs to drop the placements that cover that date's squares.
pub struct Solver {
    pub rows: usize,
    pub cols: usize,
    pub open: Vec<Vec<bool>>,
    pub pieces: Vec<Piece>,
    pub placements: Vec<Placement>,
}

impl Solver {
    pub fn new(puzzle: &Puzzle, pieces: &[Piece], reflections: bool) -> Self {
        let (rows, cols) = (puzzle.rows(), puzzle.cols());
        let open: Vec<Vec<bool>> = (0..rows)
            .map(|r| {
                (0..cols)
                    .map(|c| puzzle.board[r].get(c).copied().unwrap_or(false))
                    .collect()
            })
            .collect();
        let mut placements = Vec::new();
        for (p, piece) in pieces.iter().enumerate() {
            for shape in orientations(&piece.cells, reflections) {
                for r in 0..rows {
                    for c in 0..cols {
                        let cells: Vec<Cell> =
                            shape.iter().map(|&(pr, pc)| (r + pr, c + pc)).collect();
                        if cells
                            .iter()
                            .all(|&(cr, cc)| cr < rows && cc < cols && open[cr][cc])
                        {
                            placements.push(Placement { piece: p, cells });
                        }
                    }
                }
            }
        }
        Solver {
            rows,
            cols,
            open,
            pieces: pieces.to_vec(),
            placements,
        }
    }

    fn build(&self, blocked: &[Cell]) -> Dlx {
        // Columns: one per uncovered board square, then one per piece.
        let mut index = vec![vec![usize::MAX; self.cols]; self.rows];
        let mut ncols = 0;
        for r in 0..self.rows {
            for c in 0..self.cols {
                if self.open[r][c] && !blocked.contains(&(r, c)) {
                    index[r][c] = ncols;
                    ncols += 1;
                }
            }
        }
        let piece_col = ncols;
        let mut dlx = Dlx::new(ncols + self.pieces.len());
        let mut row = Vec::with_capacity(8);
        'next: for (i, placement) in self.placements.iter().enumerate() {
            row.clear();
            for &(r, c) in &placement.cells {
                if index[r][c] == usize::MAX {
                    continue 'next;
                }
                row.push(index[r][c]);
            }
            row.push(piece_col + placement.piece);
            dlx.add_row(i as u32, &row);
        }
        dlx
    }

    /// Find solutions (up to `limit`, 0 for all) leaving `blocked` uncovered.
    /// Each solution is a list of placement indexes into `self.placements`.
    pub fn solve(&self, blocked: &[Cell], limit: usize) -> Vec<Vec<usize>> {
        let mut dlx = self.build(blocked);
        let mut out = Vec::new();
        dlx.search(&mut Vec::new(), &mut |sol| {
            out.push(sol.iter().map(|&r| r as usize).collect());
            limit == 0 || out.len() < limit
        });
        out
    }

    /// Count solutions without storing them.
    pub fn count(&self, blocked: &[Cell]) -> usize {
        let mut dlx = self.build(blocked);
        let mut n = 0;
        dlx.search(&mut Vec::new(), &mut |_| {
            n += 1;
            true
        });
        n
    }

    /// Draw a solution with each piece's emoji color.
    pub fn render(&self, solution: &[usize]) -> String {
        let mut grid = vec![vec![EMPTY; self.cols]; self.rows];
        for &i in solution {
            let placement = &self.placements[i];
            for &(r, c) in &placement.cells {
                grid[r][c] = self.pieces[placement.piece].color;
            }
        }
        grid.iter()
            .map(|row| row.iter().collect::<String>() + "\n")
            .collect()
    }
}

/// Knuth's Dancing Links. Node 0 is the root, nodes 1..=ncols are column
/// headers, and every placement adds one node per column it covers.
struct Dlx {
    left: Vec<u32>,
    right: Vec<u32>,
    up: Vec<u32>,
    down: Vec<u32>,
    col: Vec<u32>,
    row: Vec<u32>,
    size: Vec<u32>,
}

impl Dlx {
    fn new(ncols: usize) -> Self {
        let n = ncols + 1;
        let mut dlx = Dlx {
            left: (0..n as u32)
                .map(|i| if i == 0 { ncols as u32 } else { i - 1 })
                .collect(),
            right: (0..n as u32)
                .map(|i| if i as usize == ncols { 0 } else { i + 1 })
                .collect(),
            up: (0..n as u32).collect(),
            down: (0..n as u32).collect(),
            col: (0..n as u32).collect(),
            row: vec![u32::MAX; n],
            size: vec![0; n],
        };
        dlx.left.reserve(n * 6);
        dlx
    }

    fn add_row(&mut self, row_id: u32, cols: &[usize]) {
        let first = self.left.len() as u32;
        for (k, &c) in cols.iter().enumerate() {
            let c = c as u32 + 1;
            let node = first + k as u32;
            let last = cols.len() as u32 - 1;
            self.left.push(if k == 0 { first + last } else { node - 1 });
            self.right
                .push(if k as u32 == last { first } else { node + 1 });
            let above = self.up[c as usize];
            self.up.push(above);
            self.down.push(c);
            self.down[above as usize] = node;
            self.up[c as usize] = node;
            self.col.push(c);
            self.row.push(row_id);
            self.size[c as usize] += 1;
        }
    }

    fn cover(&mut self, c: u32) {
        let c = c as usize;
        let (l, r) = (self.left[c], self.right[c]);
        self.right[l as usize] = r;
        self.left[r as usize] = l;
        let mut i = self.down[c] as usize;
        while i != c {
            let mut j = self.right[i] as usize;
            while j != i {
                let (u, d) = (self.up[j], self.down[j]);
                self.down[u as usize] = d;
                self.up[d as usize] = u;
                self.size[self.col[j] as usize] -= 1;
                j = self.right[j] as usize;
            }
            i = self.down[i] as usize;
        }
    }

    fn uncover(&mut self, c: u32) {
        let c = c as usize;
        let mut i = self.up[c] as usize;
        while i != c {
            let mut j = self.left[i] as usize;
            while j != i {
                self.size[self.col[j] as usize] += 1;
                let (u, d) = (self.up[j], self.down[j]);
                self.down[u as usize] = j as u32;
                self.up[d as usize] = j as u32;
                j = self.left[j] as usize;
            }
            i = self.up[i] as usize;
        }
        let (l, r) = (self.left[c], self.right[c]);
        self.right[l as usize] = c as u32;
        self.left[r as usize] = c as u32;
    }

    /// Returns false once `found` asks to stop.
    fn search(&mut self, solution: &mut Vec<u32>, found: &mut dyn FnMut(&[u32]) -> bool) -> bool {
        if self.right[0] == 0 {
            return found(solution);
        }
        // Pick the column with the fewest remaining options.
        let mut best = self.right[0];
        let mut c = best;
        while c != 0 {
            if self.size[c as usize] < self.size[best as usize] {
                best = c;
                if self.size[c as usize] == 0 {
                    return true;
                }
            }
            c = self.right[c as usize];
        }
        if self.size[best as usize] == 0 {
            return true;
        }
        self.cover(best);
        let mut keep_going = true;
        let mut r = self.down[best as usize];
        while r != best && keep_going {
            solution.push(self.row[r as usize]);
            let mut j = self.right[r as usize];
            while j != r {
                self.cover(self.col[j as usize]);
                j = self.right[j as usize];
            }
            keep_going = self.search(solution, found);
            let mut j = self.left[r as usize];
            while j != r {
                self.uncover(self.col[j as usize]);
                j = self.left[j as usize];
            }
            solution.pop();
            r = self.down[r as usize];
        }
        self.uncover(best);
        keep_going
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weekday_matches_python() {
        // 2025-05-29 was a Thursday -> Sunday=1 numbering gives 5.
        assert_eq!(weekday(2025, 5, 29), 5);
        assert_eq!(weekday(2026, 11, 8), 1);
        assert_eq!(civil_from_days(days_from_civil(2024, 2, 29)), (2024, 2, 29));
    }

    #[test]
    fn readme_example_counts() {
        let puzzles = builtin_puzzles();
        let p = puzzles.iter().find(|p| p.name == "dragonfjord").unwrap();
        let solver = Solver::new(p, p.set("set1").unwrap(), true);
        assert_eq!(
            solver.count(&date_cells(p, 5, 29, weekday(2025, 5, 29))),
            66
        );
    }
}
