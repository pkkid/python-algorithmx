//! Estimate how hard a date is for a person to solve.
//!
//! The model is a randomized solver that plays the way people do:
//!
//! * It notices dead ends: an empty square nothing can cover, or an enclosed
//!   pocket whose size no combination of the remaining pieces adds up to.
//! * It works on the most constrained square first (the one with the fewest
//!   ways to cover it). If there is only one way, that move is *forced*.
//! * Otherwise it guesses, trying the options in random order, and takes
//!   pieces back when it hits a dead end.
//!
//! Difficulty is the average number of *wrong placements* (pieces put down and
//! later taken back) before the puzzle is solved, over many random runs.
//! `naive` mode instead fills squares in a fixed order with no lookahead, a
//! stand-in for someone just trying pieces.
use crate::BitSolver;

/// Small, fast, seedable random number generator (SplitMix64).
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn below(&mut self, n: u32) -> u32 {
        ((self.next_u64() >> 32) * n as u64 >> 32) as u32
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Trial {
    pub placements: u32,
    pub forced: u32,
    pub solved: bool,
}

/// Summary of many trials on one date.
#[derive(Clone, Copy, Debug, Default)]
pub struct Rating {
    /// Mean pieces placed and later removed before solving.
    pub wrong: f64,
    /// Fraction of placements that were forced (only one option).
    pub forced: f64,
    /// Trials that hit the placement cap without solving.
    pub gave_up: u32,
}

pub struct Human<'a> {
    bits: &'a BitSolver,
    /// For each square bit: candidate indexes (real pieces only) covering it.
    cover: Vec<Vec<u32>>,
    pieces: u32,
    pub cap: u32,
}

impl<'a> Human<'a> {
    pub fn new(bits: &'a BitSolver) -> Self {
        let nplacements = bits.base.placements.len() as u32;
        let mut cover = vec![Vec::new(); 64];
        for k in 0..bits.masks.len() {
            if bits.ids[k] < nplacements {
                let mut m = bits.masks[k];
                while m != 0 {
                    cover[m.trailing_zeros() as usize].push(k as u32);
                    m &= m - 1;
                }
            }
        }
        Human { bits, cover, pieces: bits.base.pieces.len() as u32, cap: 1_000_000 }
    }

    fn fits(&self, k: u32, filled: u64, used: u32) -> bool {
        let k = k as usize;
        used & self.bits.flags[k] == 0 && self.bits.masks[k] & filled == 0
    }

    /// One randomized attempt at solving with `blocked` squares left uncovered.
    pub fn trial(&self, blocked: u64, rng: &mut Rng, naive: bool) -> Trial {
        let mut t = Trial::default();
        t.solved = self.dfs(blocked, self.bits.hole_flags, rng, &mut t, naive);
        t
    }

    fn dfs(&self, filled: u64, used: u32, rng: &mut Rng, t: &mut Trial, naive: bool) -> bool {
        let b = self.bits;
        if filled == b.full {
            return used == b.all_used;
        }
        if t.placements >= self.cap {
            return false;
        }
        let (square, count) = if naive {
            let sq = (!filled & b.full).trailing_zeros();
            let n = self.cover[sq as usize].iter().filter(|&&k| self.fits(k, filled, used)).count();
            (sq, n as u32)
        } else {
            if !b.pockets_ok(filled, used) {
                return false;
            }
            // Most constrained square, ties broken at random.
            let (mut best, mut best_sq, mut ties) = (u32::MAX, 0, 0);
            let mut empty = !filled & b.full;
            while empty != 0 {
                let sq = empty.trailing_zeros();
                empty &= empty - 1;
                let n = self.cover[sq as usize].iter().filter(|&&k| self.fits(k, filled, used)).count() as u32;
                if n == 0 {
                    return false;
                }
                if n < best {
                    (best, best_sq, ties) = (n, sq, 1);
                } else if n == best {
                    ties += 1;
                    if rng.below(ties) == 0 {
                        best_sq = sq;
                    }
                }
            }
            (best_sq, best)
        };
        let mut options: Vec<u32> =
            self.cover[square as usize].iter().copied().filter(|&k| self.fits(k, filled, used)).collect();
        for i in (1..options.len()).rev() {
            options.swap(i, rng.below(i as u32 + 1) as usize);
        }
        for k in options {
            t.placements += 1;
            if count == 1 {
                t.forced += 1;
            }
            let k = k as usize;
            if self.dfs(filled | b.masks[k], used | b.flags[k], rng, t, naive) {
                return true;
            }
        }
        false
    }

    /// Average over `trials` random runs.
    pub fn rate(&self, blocked: u64, trials: u32, seed: u64, naive: bool) -> Rating {
        let mut rng = Rng::new(seed);
        let (mut wrong, mut placed, mut forced, mut gave_up) = (0f64, 0f64, 0f64, 0);
        for _ in 0..trials {
            let t = self.trial(blocked, &mut rng, naive);
            if !t.solved {
                gave_up += 1;
            }
            wrong += (t.placements.saturating_sub(if t.solved { self.pieces } else { 0 })) as f64;
            placed += t.placements as f64;
            forced += t.forced as f64;
        }
        Rating { wrong: wrong / trials as f64, forced: forced / placed.max(1.0), gave_up }
    }
}

/// Spearman rank correlation.
pub fn spearman(a: &[f64], b: &[f64]) -> f64 {
    fn ranks(v: &[f64]) -> Vec<f64> {
        let mut idx: Vec<usize> = (0..v.len()).collect();
        idx.sort_by(|&i, &j| v[i].partial_cmp(&v[j]).unwrap());
        let mut r = vec![0.0; v.len()];
        let mut i = 0;
        while i < idx.len() {
            let mut j = i;
            while j + 1 < idx.len() && v[idx[j + 1]] == v[idx[i]] {
                j += 1;
            }
            for &k in &idx[i..=j] {
                r[k] = (i + j) as f64 / 2.0;
            }
            i = j + 1;
        }
        r
    }
    let (ra, rb) = (ranks(a), ranks(b));
    let n = a.len() as f64;
    let (ma, mb) = (ra.iter().sum::<f64>() / n, rb.iter().sum::<f64>() / n);
    let cov: f64 = ra.iter().zip(&rb).map(|(x, y)| (x - ma) * (y - mb)).sum();
    let va: f64 = ra.iter().map(|x| (x - ma).powi(2)).sum();
    let vb: f64 = rb.iter().map(|y| (y - mb).powi(2)).sum();
    cov / (va * vb).sqrt()
}
