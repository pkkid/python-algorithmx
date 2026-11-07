//! Bitboard solver.
//!
//! Every playable square gets one bit of a `u64` (boards up to 64 squares), and
//! every placement of every piece is precomputed as a bitmask. The search always
//! fills the lowest empty square, so it only needs to look at the placements
//! whose lowest square is that one. Testing a placement is one AND, placing it
//! is one OR, and undoing it is free (the old mask is still on the stack).
//!
//! Squares are laid out as a grid with one padding column (`bit = r * W + c`)
//! so neighbours are bit shifts. That makes a cheap dead-end check possible:
//! after each placement, every enclosed pocket of empty squares must have a
//! size that some combination of the remaining pieces adds up to.
//!
//! It can also solve every date in a single search: add one 1x1 "hole" piece
//! per date region (month, day and weekday squares) that may only sit inside
//! its own region. Every complete tiling then leaves exactly one date showing,
//! so one pass counts the solutions for all dates at once.
//!
//! When only counts are needed (year mode), results are memoized: since the
//! search always fills the lowest empty square, the number of ways to finish
//! depends only on (filled squares, used pieces), whatever date led there.
use crate::*;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Fill order used by [`BitSolver::new`]: start at the bottom-right corner and
/// go row by row. Measured fastest of the 8 orders on both Dragonfjord and
/// Guanglu (both boards have their ragged edge there, which constrains early).
pub const BEST_ORDER: u8 = 3;

/// Date regions by square number (1-based, row-major over playable squares).
const REGIONS: [(u32, u32); 3] = [(1, 12), (13, 43), (44, 50)];

pub struct BitSolver {
    /// Placement list and rendering are shared with the DLX solver.
    pub base: Solver,
    pub(crate) bit: Vec<Vec<u8>>,
    pub(crate) width: u32,
    pub(crate) full: u64,
    /// For each set of used flags: bit `n` set if the unused pieces/holes can
    /// cover exactly `n` squares.
    pub(crate) sums: Vec<u64>,
    pub prune: bool,
    /// Bitmask of every piece and hole flag that must be used.
    pub(crate) all_used: u32,
    /// Hole pieces present on this board (one per date region).
    pub(crate) hole_flags: u32,
    /// Candidates grouped by their lowest square: `start[b]..start[b+1]`.
    pub(crate) start: Vec<u32>,
    pub(crate) masks: Vec<u64>,
    pub(crate) flags: Vec<u32>,
    pub(crate) ids: Vec<u32>,
}

/// Fast hasher for (u64, u32) memo keys (std's SipHash is overkill here).
#[derive(Default)]
struct MixHasher(u64);

impl std::hash::Hasher for MixHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.write_u64(b as u64);
        }
    }
    fn write_u64(&mut self, x: u64) {
        self.0 = (self.0.rotate_left(26) ^ x).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
    fn write_u32(&mut self, x: u32) {
        self.write_u64(x as u64);
    }
}

type Memo = std::collections::HashMap<(u64, u32), u64, std::hash::BuildHasherDefault<MixHasher>>;

/// Solution counts for every (month, day, weekday) combination.
pub struct DateCounts {
    counts: Vec<usize>,
}

impl DateCounts {
    fn index(month: u32, day: u32, weekday: u32) -> usize {
        (month as usize * 32 + day as usize) * 8 + weekday as usize
    }

    /// Weekday is ignored on boards without weekday squares.
    pub fn get(&self, month: u32, day: u32, weekday: u32) -> usize {
        self.counts
            .get(Self::index(month, day, weekday))
            .copied()
            .unwrap_or(0)
    }
}

impl BitSolver {
    pub fn new(puzzle: &Puzzle, pieces: &[Piece], reflections: bool) -> Self {
        Self::with_order(puzzle, pieces, reflections, BEST_ORDER)
    }

    /// `order` (0..8) picks which corner the search starts filling from and
    /// whether it goes by rows or columns.
    pub fn with_order(puzzle: &Puzzle, pieces: &[Piece], reflections: bool, order: u8) -> Self {
        let base = Solver::new(puzzle, pieces, reflections);
        let open = puzzle.open_cells();
        let (rows, cols) = (base.rows, base.cols);
        let transpose = order & 4 != 0;
        let (vrows, vcols) = if transpose {
            (cols, rows)
        } else {
            (rows, cols)
        };
        let width = vcols + 1;
        assert!(
            vrows * width <= 64,
            "bitboard solver supports boards up to 8x7 squares"
        );
        let to_bit = |r: usize, c: usize| {
            let r = if order & 1 != 0 { rows - 1 - r } else { r };
            let c = if order & 2 != 0 { cols - 1 - c } else { c };
            let (r, c) = if transpose { (c, r) } else { (r, c) };
            r * width + c
        };
        assert!(pieces.len() + REGIONS.len() <= 16, "too many pieces");
        let mut bit = vec![vec![u8::MAX; base.cols]; base.rows];
        let mut full = 0u64;
        for &(r, c) in &open {
            let b = to_bit(r, c);
            bit[r][c] = b as u8;
            full |= 1 << b;
        }
        let nsquares = open.len();
        let npieces = pieces.len() as u32;

        // (lowest bit, mask, used-flag, id) for every piece placement and hole.
        let mut cands: Vec<(u32, u64, u32, u32)> = Vec::new();
        for (id, placement) in base.placements.iter().enumerate() {
            let mask = placement
                .cells
                .iter()
                .fold(0u64, |m, &(r, c)| m | 1 << bit[r][c]);
            cands.push((mask.trailing_zeros(), mask, 1 << placement.piece, id as u32));
        }
        let mut hole_flags = 0;
        for (region, &(lo, hi)) in REGIONS.iter().enumerate() {
            if nsquares < hi as usize {
                continue; // e.g. Dragonfjord has no weekday squares
            }
            let flag = 1 << (npieces + region as u32);
            hole_flags |= flag;
            for (i, &(r, c)) in open.iter().enumerate() {
                let n = i as u32 + 1;
                if (lo..=hi).contains(&n) {
                    let b = bit[r][c] as u32;
                    cands.push((b, 1 << b, flag, base.placements.len() as u32 + n - 1));
                }
            }
        }
        // Stable sort keeps piece order inside each square's group.
        cands.sort_by_key(|c| c.0);
        let mut start = vec![0u32; 65];
        for c in &cands {
            start[c.0 as usize + 1] += 1;
        }
        for b in 0..64 {
            start[b + 1] += start[b];
        }
        // Subset sums of the unused items, for every combination of used flags.
        let mut sizes: Vec<u32> = pieces.iter().map(|p| p.cells.len() as u32).collect();
        sizes.extend(std::iter::repeat_n(1, hole_flags.count_ones() as usize));
        let nflags = sizes.len();
        let mut sums = vec![0u64; 1 << nflags];
        let all = (1usize << nflags) - 1;
        for used in (0..=all).rev() {
            // Unused set U = all & !used; sums(U) = sums(U - p) | sums(U - p) << size(p).
            let unused = all & !used;
            sums[used] = if unused == 0 {
                1
            } else {
                let p = unused.trailing_zeros() as usize;
                let rest = sums[used | 1 << p];
                rest | rest << sizes[p]
            };
        }
        // Hole flags sit right after the piece flags, so `sums` lines up with `used`.
        BitSolver {
            base,
            bit,
            width: width as u32,
            full,
            sums,
            prune: true,
            all_used: (1 << npieces) - 1 | hole_flags,
            hole_flags,
            start,
            masks: cands.iter().map(|c| c.1).collect(),
            flags: cands.iter().map(|c| c.2).collect(),
            ids: cands.iter().map(|c| c.3).collect(),
        }
    }

    fn search(
        &self,
        filled: u64,
        used: u32,
        stack: &mut Vec<u32>,
        found: &mut dyn FnMut(&[u32]) -> bool,
    ) -> bool {
        if filled == self.full {
            return used != self.all_used || found(stack);
        }
        let b = (!filled & self.full).trailing_zeros() as usize;
        for k in self.start[b] as usize..self.start[b + 1] as usize {
            let (mask, flag) = (self.masks[k], self.flags[k]);
            if used & flag != 0 || mask & filled != 0 {
                continue;
            }
            if self.prune && !self.pockets_ok(filled | mask, used | flag) {
                continue;
            }
            stack.push(self.ids[k]);
            let keep_going = self.search(filled | mask, used | flag, stack, found);
            stack.pop();
            if !keep_going {
                return false;
            }
        }
        true
    }

    /// Every connected pocket of empty squares must be fillable by some subset
    /// of the remaining pieces (and holes).
    #[inline]
    pub(crate) fn pockets_ok(&self, filled: u64, used: u32) -> bool {
        let sums = self.sums[used as usize];
        let w = self.width;
        let mut empty = !filled & self.full;
        while empty != 0 {
            let mut region = empty & empty.wrapping_neg();
            loop {
                let grown =
                    (region | region << 1 | region >> 1 | region << w | region >> w) & empty;
                if grown == region {
                    break;
                }
                region = grown;
            }
            if sums >> region.count_ones() & 1 == 0 {
                return false;
            }
            empty &= !region;
        }
        true
    }

    /// Count solutions for several dates, sharing one memo table between them.
    pub fn count_dates_memo(&self, dates: &[Vec<Cell>]) -> Vec<usize> {
        let mut memo = Memo::default();
        dates
            .iter()
            .map(|b| self.count_rec(self.blocked_mask(b), self.hole_flags, &mut memo) as usize)
            .collect()
    }

    fn count_rec(&self, filled: u64, used: u32, memo: &mut Memo) -> u64 {
        if filled == self.full {
            return (used == self.all_used) as u64;
        }
        let key = (filled, used);
        if let Some(&n) = memo.get(&key) {
            return n;
        }
        let b = (!filled & self.full).trailing_zeros() as usize;
        let mut n = 0;
        for k in self.start[b] as usize..self.start[b + 1] as usize {
            let (mask, flag) = (self.masks[k], self.flags[k]);
            if used & flag != 0 || mask & filled != 0 {
                continue;
            }
            if self.prune && !self.pockets_ok(filled | mask, used | flag) {
                continue;
            }
            n += self.count_rec(filled | mask, used | flag, memo);
        }
        memo.insert(key, n);
        n
    }

    pub fn blocked_mask(&self, blocked: &[Cell]) -> u64 {
        blocked.iter().fold(0, |m, &(r, c)| m | 1 << self.bit[r][c])
    }

    /// Same contract as [`Solver::solve`]: placement indexes into `base.placements`.
    pub fn solve(&self, blocked: &[Cell], limit: usize) -> Vec<Vec<usize>> {
        let mut out = Vec::new();
        // Holes are marked used, so only real pieces can be placed.
        self.search(
            self.blocked_mask(blocked),
            self.hole_flags,
            &mut Vec::new(),
            &mut |s| {
                out.push(s.iter().map(|&i| i as usize).collect());
                limit == 0 || out.len() < limit
            },
        );
        out
    }

    pub fn count(&self, blocked: &[Cell]) -> usize {
        let mut n = 0;
        self.search(
            self.blocked_mask(blocked),
            self.hole_flags,
            &mut Vec::new(),
            &mut |_| {
                n += 1;
                true
            },
        );
        n
    }

    /// Count solutions for every date at once, split over `threads` threads.
    pub fn count_all_dates(&self, threads: usize) -> DateCounts {
        // Expand the first few levels into independent jobs for the threads.
        let mut jobs: Vec<(u64, u32, Vec<u32>)> = vec![(0, 0, Vec::new())];
        while jobs.len() < threads.max(1) * 16 {
            let mut next = Vec::new();
            for (filled, used, stack) in &jobs {
                if *filled == self.full {
                    next.push((*filled, *used, stack.clone()));
                    continue;
                }
                let b = (!filled & self.full).trailing_zeros() as usize;
                for k in self.start[b] as usize..self.start[b + 1] as usize {
                    if used & self.flags[k] == 0 && self.masks[k] & filled == 0 {
                        let mut s = stack.clone();
                        s.push(self.ids[k]);
                        next.push((filled | self.masks[k], used | self.flags[k], s));
                    }
                }
            }
            if next.len() <= jobs.len() {
                break;
            }
            jobs = next;
        }
        let nplacements = self.base.placements.len() as u32;
        let per_thread = parallel(jobs.len(), threads, |next| {
            let mut counts = vec![0usize; 13 * 32 * 8];
            let mut tally = |stack: &[u32]| {
                let (mut m, mut d, mut w) = (0, 0, 0);
                for &id in stack {
                    if id >= nplacements {
                        match id - nplacements + 1 {
                            n @ 1..=12 => m = n,
                            n @ 13..=43 => d = n - 12,
                            n => w = n - 43,
                        }
                    }
                }
                counts[DateCounts::index(m, d, w)] += 1;
                true
            };
            while let Some(i) = next() {
                let (filled, used, stack) = &jobs[i];
                self.search(*filled, *used, &mut stack.clone(), &mut tally);
            }
            counts
        });
        let mut counts = vec![0usize; 13 * 32 * 8];
        for c in per_thread {
            counts.iter_mut().zip(c).for_each(|(t, c)| *t += c);
        }
        DateCounts { counts }
    }

    /// Count solutions for every day of `year`, returned as ((month, day), count).
    /// Boards without weekday squares use the all-dates search (one hole per
    /// region gives ~372 month/day combos, close to the 365 we need). Boards with
    /// weekdays would have 2604 combos, so instead each month is counted with a
    /// shared memo table, months running in parallel.
    pub fn count_year(
        &self,
        puzzle: &Puzzle,
        year: i64,
        threads: usize,
    ) -> Vec<((u32, u32), usize)> {
        let has_weekday = puzzle.open_cells().len() >= 50;
        let months: Vec<Vec<(u32, u32)>> = (1..=12)
            .map(|m| (1..=days_in_month(year, m)).map(|d| (m, d)).collect())
            .collect();
        if !has_weekday {
            let all = self.count_all_dates(threads);
            return months
                .concat()
                .into_iter()
                .map(|(m, d)| ((m, d), all.get(m, d, 0)))
                .collect();
        }
        let results = Mutex::new(vec![Vec::new(); 12]);
        parallel(12, threads, |next| {
            while let Some(i) = next() {
                let blocked: Vec<Vec<Cell>> = months[i]
                    .iter()
                    .map(|&(m, d)| date_cells(puzzle, m, d, weekday(year, m, d)))
                    .collect();
                let counts = self.count_dates_memo(&blocked);
                results.lock().unwrap()[i] = months[i].iter().copied().zip(counts).collect();
            }
        });
        results.into_inner().unwrap().concat()
    }
}

/// Run `work` on `threads` threads. Each gets a `next()` that hands out job
/// indexes `0..njobs`. With one thread (or on wasm32, which has no threads)
/// it runs inline.
fn parallel<R: Send>(
    njobs: usize,
    threads: usize,
    work: impl Fn(&dyn Fn() -> Option<usize>) -> R + Sync,
) -> Vec<R> {
    let counter = AtomicUsize::new(0);
    let next = || Some(counter.fetch_add(1, Ordering::Relaxed)).filter(|&i| i < njobs);
    if threads <= 1 || cfg!(target_arch = "wasm32") {
        return vec![work(&next)];
    }
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..threads.min(njobs.max(1)))
            .map(|_| scope.spawn(|| work(&next)))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_dlx() {
        for p in builtin_puzzles() {
            for (_, pieces) in p.sets.iter().take(2) {
                let dlx = Solver::new(&p, pieces, true);
                let bits = BitSolver::new(&p, pieces, true);
                let all = bits.count_all_dates(4);
                for (m, d) in [(1, 1), (2, 29), (5, 29), (12, 31)] {
                    let wd = weekday(2024, m, d);
                    let blocked = date_cells(&p, m, d, wd);
                    let n = dlx.count(&blocked);
                    assert_eq!(bits.count(&blocked), n, "{} {m}/{d}", p.name);
                    assert_eq!(bits.count_dates_memo(&[blocked.clone()]), [n]);
                    assert_eq!(
                        all.get(m, d, if p.open_cells().len() >= 50 { wd } else { 0 }),
                        n
                    );
                }
            }
        }
    }

    #[test]
    fn year_matches_dlx() {
        let puzzles = builtin_puzzles();
        let p = &puzzles[0];
        let dlx = Solver::new(p, &p.sets[0].1, true);
        let bits = BitSolver::new(p, &p.sets[0].1, true);
        for ((m, d), n) in bits.count_year(p, 2024, 4) {
            assert_eq!(
                n,
                dlx.count(&date_cells(p, m, d, weekday(2024, m, d))),
                "{m}/{d}"
            );
        }
    }
}
