//! A primordial soup of BFF programs, after Agüera y Arcas et al.,
//! "Computational Life: How Well-formed, Self-replicating Programs Emerge
//! from Simple Interaction" (arXiv:2406.19108).
//!
//! The soup is a pile of 64-byte tapes. Each epoch pairs them up at random,
//! concatenates each pair into one 128-byte tape, runs it as a BFF program
//! (code and data share the tape, so programs can rewrite each other), then
//! splits it back. Nothing selects for anything, yet replicators show up.

use rayon::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};

pub const PROG: usize = 64;
pub const TAPE: usize = 2 * PROG;
pub const MAX_STEPS: usize = 1 << 13;

/// Experimental rule change: when set, bracket matching stops at the seam
/// between the two programs, so a loop can't open in one and close in the
/// other (an unmatched bracket halts, as usual).
pub static SEALED: AtomicBool = AtomicBool::new(false);

#[inline]
fn sealed() -> bool {
    SEALED.load(Ordering::Relaxed)
}

/// The paper's other BFF variant (cubff `bff`, as opposed to `bff_noheads`):
/// the first two bytes of the tape set head0 and head1, and execution
/// starts at byte 2.
pub static HEADS: AtomicBool = AtomicBool::new(false);

#[inline]
fn start(tape: &[u8; TAPE]) -> (usize, usize, isize) {
    if HEADS.load(Ordering::Relaxed) {
        (tape[0] as usize % TAPE, tape[1] as usize % TAPE, 2)
    } else {
        (0, 0, 0)
    }
}

// ---------------------------------------------------------------- rng

/// xoshiro256** seeded through splitmix64. Small, fast, good enough.
#[derive(Clone)]
pub struct Rng([u64; 4]);

impl Rng {
    pub fn new(seed: u64) -> Self {
        let mut z = seed;
        let mut next = || {
            z = z.wrapping_add(0x9e3779b97f4a7c15);
            let mut x = z;
            x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
            x ^ (x >> 31)
        };
        Rng([next(), next(), next(), next()])
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let s = &mut self.0;
        let out = s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = s[1] << 17;
        s[2] ^= s[0];
        s[3] ^= s[1];
        s[1] ^= s[2];
        s[0] ^= s[3];
        s[2] ^= t;
        s[3] = s[3].rotate_left(45);
        out
    }

    /// Uniform in 0..n (Lemire's multiply-shift; the bias is negligible here).
    #[inline]
    pub fn below(&mut self, n: usize) -> usize {
        ((self.next_u64() as u128 * n as u128) >> 64) as usize
    }

    #[inline]
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    pub fn fill(&mut self, buf: &mut [u8]) {
        for chunk in buf.chunks_mut(8) {
            let r = self.next_u64().to_le_bytes();
            chunk.copy_from_slice(&r[..chunk.len()]);
        }
    }

    pub fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            v.swap(i, self.below(i + 1));
        }
    }

    /// Poisson sample via Knuth for small means, normal approximation above.
    pub fn poisson(&mut self, mean: f64) -> usize {
        if mean <= 0.0 {
            return 0;
        }
        if mean < 30.0 {
            let l = (-mean).exp();
            let mut k = 0;
            let mut p = 1.0;
            loop {
                p *= self.unit();
                if p <= l {
                    return k;
                }
                k += 1;
            }
        }
        let u1 = self.unit().max(1e-300);
        let u2 = self.unit();
        let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        (mean + z * mean.sqrt()).round().max(0.0) as usize
    }
}

// ---------------------------------------------------------------- bff

pub fn is_op(b: u8) -> bool {
    matches!(b, b'<' | b'>' | b'{' | b'}' | b'+' | b'-' | b'.' | b',' | b'[' | b']')
}

/// Run one concatenated tape in place. Returns the number of real
/// instructions executed (no-op bytes are read but not counted), matching
/// cubff's accounting. The step budget counts every byte read.
pub fn run(tape: &mut [u8; TAPE]) -> usize {
    const MASK: usize = TAPE - 1;
    let (mut h0, mut h1, mut pc) = start(tape);
    let mut ops = 0;
    for _ in 0..MAX_STEPS {
        let mut real = true;
        match tape[pc as usize] {
            b'<' => h0 = h0.wrapping_sub(1) & MASK,
            b'>' => h0 = (h0 + 1) & MASK,
            b'{' => h1 = h1.wrapping_sub(1) & MASK,
            b'}' => h1 = (h1 + 1) & MASK,
            b'+' => tape[h0] = tape[h0].wrapping_add(1),
            b'-' => tape[h0] = tape[h0].wrapping_sub(1),
            b'.' => tape[h1] = tape[h0],
            b',' => tape[h0] = tape[h1],
            b'[' => {
                if tape[h0] == 0 {
                    let hi = if sealed() && (pc as usize) < PROG { PROG } else { TAPE };
                    let mut depth = 1;
                    while depth > 0 {
                        pc += 1;
                        if pc as usize >= hi {
                            return ops + 1;
                        }
                        match tape[pc as usize] {
                            b'[' => depth += 1,
                            b']' => depth -= 1,
                            _ => {}
                        }
                    }
                }
            }
            b']' => {
                if tape[h0] != 0 {
                    let lo = if sealed() && pc as usize >= PROG { PROG as isize } else { 0 };
                    let mut depth = 1;
                    while depth > 0 {
                        pc -= 1;
                        if pc < lo {
                            return ops + 1;
                        }
                        match tape[pc as usize] {
                            b']' => depth += 1,
                            b'[' => depth -= 1,
                            _ => {}
                        }
                    }
                }
            }
            _ => real = false,
        }
        ops += real as usize;
        pc += 1;
        if pc as usize >= TAPE {
            break;
        }
    }
    ops
}

// ---------------------------------------------------------------- tests of copying

/// Best fraction of `p` copied into the partner (direct or mirrored) over a
/// few random partners, trying `p` as both first and second half.
pub fn copy_score(p: &[u8], rng: &mut Rng, trials: usize) -> f64 {
    copy_score_with(p, rng, trials, None)
}

/// `copy_score`, optionally forcing the partner's first byte (e.g. to `]`).
pub fn copy_score_with(p: &[u8], rng: &mut Rng, trials: usize, lead: Option<u8>) -> f64 {
    let mut best: f64 = 0.0;
    for _ in 0..trials {
        for first in [true, false] {
            let mut t = [0u8; TAPE];
            rng.fill(&mut t);
            if let Some(b) = lead {
                t[if first { PROG } else { 0 }] = b;
            }
            let (me, other) = if first { (0, PROG) } else { (PROG, 0) };
            t[me..me + PROG].copy_from_slice(p);
            run(&mut t);
            best = best.max(similarity(p, &t[other..other + PROG]) as f64 / PROG as f64);
        }
    }
    best
}

/// Follow a lineage: run `p` against noise, take whatever landed in the
/// partner as the child (whichever half `p` was in), repeat. Heritable if
/// the 4th-generation descendant still carries 90% of `p` (direct or mirrored).
pub fn heritable(p: &[u8], rng: &mut Rng) -> bool {
    heritable_at(p, rng, 0.9)
}

/// `heritable` with a custom similarity threshold (fraction of 64 bytes).
pub fn heritable_at(p: &[u8], rng: &mut Rng, threshold: f64) -> bool {
    let mut x = p.to_vec();
    for _ in 0..4 {
        let mut best: Option<(usize, Vec<u8>)> = None;
        for first in [true, false] {
            let mut t = [0u8; TAPE];
            rng.fill(&mut t);
            let (me, other) = if first { (0, PROG) } else { (PROG, 0) };
            t[me..me + PROG].copy_from_slice(&x);
            run(&mut t);
            let child = t[other..other + PROG].to_vec();
            let score = similarity(p, &child);
            if best.as_ref().is_none_or(|b| score > b.0) {
                best = Some((score, child));
            }
        }
        x = best.unwrap().1;
    }
    similarity(p, &x) as f64 >= threshold * PROG as f64
}

pub fn similarity(p: &[u8], q: &[u8]) -> usize {
    // Copies often come out shifted by a byte or two (a loop whose first
    // pass writes in place), so take the best alignment within +-8.
    let mut best = 0;
    for shift in -8isize..=8 {
        let (mut direct, mut mirror) = (0, 0);
        for i in 0..PROG {
            let j = i as isize + shift;
            if j < 0 || j >= PROG as isize {
                continue;
            }
            let j = j as usize;
            direct += (q[j] == p[i]) as usize;
            mirror += (q[PROG - 1 - j] == p[i]) as usize;
        }
        best = best.max(direct).max(mirror);
    }
    best
}

/// The self-replication detector of Knierim et al. 2026 ("BFF: Simple
/// explanations for complex phenomena", arXiv:2607.01483), as described:
/// 9 chains; each chain starts with `p` in the second half and 5 times copies
/// the second half into the first, fills the second half with noise and runs.
/// Score = min(first-half bytes matching `p` in >= 3 final tapes, second-half
/// bytes identical across >= 3 final tapes). They call >= 48 a replicator.
/// (Noise is unspecified in the paper; uniform bytes here.)
pub fn knierim_score(p: &[u8], rng: &mut Rng) -> usize {
    let finals: Vec<[u8; TAPE]> = (0..9)
        .map(|_| {
            let mut t = [0u8; TAPE];
            t[PROG..].copy_from_slice(p);
            for _ in 0..5 {
                t.copy_within(PROG.., 0);
                rng.fill(&mut t[PROG..]);
                run(&mut t);
            }
            t
        })
        .collect();
    let first = (0..PROG).filter(|&i| finals.iter().filter(|t| t[i] == p[i]).count() >= 3).count();
    let second = (0..PROG)
        .filter(|&i| {
            let mut c = [0u8; 256];
            finals.iter().any(|t| {
                let v = &mut c[t[PROG + i] as usize];
                *v += 1;
                *v >= 3
            })
        })
        .count();
    first.min(second)
}

/// Like `run`, but also reports every loop that closed across the seam
/// between the two programs: a `]` in one half jumping back to a `[` in the
/// other. Returns (ops, [(bracket_pos, target_pos, times)]).
pub fn run_seam(tape: &mut [u8; TAPE]) -> (usize, Vec<(usize, usize, usize)>) {
    const MASK: usize = TAPE - 1;
    let (mut h0, mut h1, mut pc) = start(tape);
    let mut ops = 0;
    let mut seams: Vec<(usize, usize, usize)> = Vec::new();
    for _ in 0..MAX_STEPS {
        let mut real = true;
        match tape[pc as usize] {
            b'<' => h0 = h0.wrapping_sub(1) & MASK,
            b'>' => h0 = (h0 + 1) & MASK,
            b'{' => h1 = h1.wrapping_sub(1) & MASK,
            b'}' => h1 = (h1 + 1) & MASK,
            b'+' => tape[h0] = tape[h0].wrapping_add(1),
            b'-' => tape[h0] = tape[h0].wrapping_sub(1),
            b'.' => tape[h1] = tape[h0],
            b',' => tape[h0] = tape[h1],
            b'[' => {
                if tape[h0] == 0 {
                    let hi = if sealed() && (pc as usize) < PROG { PROG } else { TAPE };
                    let mut depth = 1;
                    while depth > 0 {
                        pc += 1;
                        if pc as usize >= hi {
                            return (ops + 1, seams);
                        }
                        match tape[pc as usize] {
                            b'[' => depth += 1,
                            b']' => depth -= 1,
                            _ => {}
                        }
                    }
                }
            }
            b']' => {
                if tape[h0] != 0 {
                    let from = pc as usize;
                    let lo = if sealed() && pc as usize >= PROG { PROG as isize } else { 0 };
                    let mut depth = 1;
                    while depth > 0 {
                        pc -= 1;
                        if pc < lo {
                            return (ops + 1, seams);
                        }
                        match tape[pc as usize] {
                            b']' => depth += 1,
                            b'[' => depth -= 1,
                            _ => {}
                        }
                    }
                    let to = pc as usize;
                    if (from < PROG) != (to < PROG) {
                        match seams.iter_mut().find(|s| s.0 == from && s.1 == to) {
                            Some(s) => s.2 += 1,
                            None => seams.push((from, to, 1)),
                        }
                    }
                }
            }
            _ => real = false,
        }
        ops += real as usize;
        pc += 1;
        if pc as usize >= TAPE {
            break;
        }
    }
    (ops, seams)
}

// ---------------------------------------------------------------- 2D grid

/// The 2D soup: programs on a w x h grid (no wraparound), each pairing with
/// a random neighbour within 2 cells per epoch.
pub struct Grid {
    pub w: usize,
    pub h: usize,
    pub data: Vec<u8>,
    pub mutation: f64,
    pub rng: Rng,
    pub epoch: u64,
    order: Vec<usize>,
    claimed: Vec<bool>,
}

impl Grid {
    pub fn new(w: usize, h: usize, mutation: f64, seed: u64) -> Self {
        let n = w * h;
        let mut rng = Rng::new(seed);
        let mut data = vec![0u8; n * PROG];
        rng.fill(&mut data);
        Grid { w, h, data, mutation, rng, epoch: 0, order: (0..n).collect(), claimed: vec![false; n] }
    }

    /// One epoch. Returns the pairs that ran, (first half, second half).
    pub fn step(&mut self) -> Vec<(usize, usize)> {
        let (w, h, n) = (self.w, self.h, self.w * self.h);
        let rng = &mut self.rng;
        // Pick pairs: visit in random order, each tries one random neighbour.
        rng.shuffle(&mut self.order);
        self.claimed.iter_mut().for_each(|c| *c = false);
        let mut pairs = Vec::with_capacity(n / 2);
        for &i in &self.order {
            if self.claimed[i] {
                continue;
            }
            let (x, y) = ((i % w) as isize, (i / w) as isize);
            let dx = rng.below(5) as isize - 2;
            let dy = rng.below(5) as isize - 2;
            let (nx, ny) = (x + dx, y + dy);
            if (dx, dy) == (0, 0) || nx < 0 || ny < 0 || nx >= w as isize || ny >= h as isize {
                continue;
            }
            let j = ny as usize * w + nx as usize;
            if self.claimed[j] {
                continue;
            }
            self.claimed[i] = true;
            self.claimed[j] = true;
            pairs.push((i, j));
        }

        let data = &self.data;
        let results: Vec<[u8; TAPE]> = pairs
            .par_iter()
            .map(|&(i, j)| {
                let mut t = [0u8; TAPE];
                t[..PROG].copy_from_slice(&data[i * PROG..(i + 1) * PROG]);
                t[PROG..].copy_from_slice(&data[j * PROG..(j + 1) * PROG]);
                run(&mut t);
                t
            })
            .collect();
        for (&(i, j), t) in pairs.iter().zip(&results) {
            self.data[i * PROG..(i + 1) * PROG].copy_from_slice(&t[..PROG]);
            self.data[j * PROG..(j + 1) * PROG].copy_from_slice(&t[PROG..]);
        }
        for _ in 0..self.rng.poisson(self.mutation * self.data.len() as f64) {
            let k = self.rng.below(self.data.len());
            self.data[k] = self.rng.next_u64() as u8;
        }
        self.epoch += 1;
        pairs
    }
}

// ---------------------------------------------------------------- soup

pub struct Soup {
    pub data: Vec<u8>,
    pub n: usize,
    pub mutation: f64,
    pub rng: Rng,
    pub epoch: u64,
    order: Vec<u32>,
}

pub struct EpochStats {
    pub ops: u64,
}

impl Soup {
    pub fn new(n: usize, mutation: f64, seed: u64) -> Self {
        assert!(n % 2 == 0);
        let mut rng = Rng::new(seed);
        let mut data = vec![0u8; n * PROG];
        rng.fill(&mut data);
        Soup { data, n, mutation, rng, epoch: 0, order: (0..n as u32).collect() }
    }

    /// The permutation used by the last `step`: programs order[2k] and
    /// order[2k+1] were run together, in that order.
    pub fn order(&self) -> &[u32] {
        &self.order
    }

    pub fn program(&self, i: usize) -> &[u8] {
        &self.data[i * PROG..(i + 1) * PROG]
    }

    pub fn step(&mut self) -> EpochStats {
        self.rng.shuffle(&mut self.order);

        // Pairs are disjoint, so: gather, run in parallel, scatter.
        let pairs: Vec<(u32, u32)> =
            self.order.chunks_exact(2).map(|c| (c[0], c[1])).collect();
        let data = &self.data;
        let results: Vec<([u8; TAPE], usize)> = pairs
            .par_iter()
            .map(|&(a, b)| {
                let mut tape = [0u8; TAPE];
                let (a, b) = (a as usize, b as usize);
                tape[..PROG].copy_from_slice(&data[a * PROG..(a + 1) * PROG]);
                tape[PROG..].copy_from_slice(&data[b * PROG..(b + 1) * PROG]);
                let ops = run(&mut tape);
                (tape, ops)
            })
            .collect();
        let mut ops = 0u64;
        for (&(a, b), (tape, o)) in pairs.iter().zip(results.iter()) {
            let (a, b) = (a as usize, b as usize);
            self.data[a * PROG..(a + 1) * PROG].copy_from_slice(&tape[..PROG]);
            self.data[b * PROG..(b + 1) * PROG].copy_from_slice(&tape[PROG..]);
            ops += *o as u64;
        }

        let k = self.rng.poisson(self.mutation * self.data.len() as f64);
        for _ in 0..k {
            let i = self.rng.below(self.data.len());
            self.data[i] = self.rng.next_u64() as u8;
        }

        self.epoch += 1;
        EpochStats { ops }
    }
}

// ---------------------------------------------------------------- metrics

/// Shannon entropy of the byte distribution, in bits per byte.
pub fn shannon(data: &[u8]) -> f64 {
    let mut counts = [0u64; 256];
    for &b in data {
        counts[b as usize] += 1;
    }
    let n = data.len() as f64;
    counts
        .iter()
        .filter(|&&c| c > 0)
        .map(|&c| {
            let p = c as f64 / n;
            -p * p.log2()
        })
        .sum()
}

/// Brotli (quality 2, as in the paper) compressed size, in bits per byte.
pub fn compressed_bpb(data: &[u8]) -> f64 {
    let mut out = Vec::with_capacity(data.len());
    let params = brotli::enc::BrotliEncoderParams { quality: 2, lgwin: 24, ..Default::default() };
    brotli::BrotliCompress(&mut &data[..], &mut out, &params).unwrap();
    out.len() as f64 * 8.0 / data.len() as f64
}

/// Shannon entropy minus normalised Kolmogorov complexity: near zero for
/// random noise, large when the soup is full of copies of a few things.
pub fn high_order_entropy(data: &[u8]) -> f64 {
    shannon(data) - compressed_bpb(data)
}

/// Render a tape the way the paper does: ops as themselves, zero as '0',
/// everything else as a space-ish dot.
pub fn show(prog: &[u8]) -> String {
    prog.iter()
        .map(|&b| {
            if is_op(b) {
                b as char
            } else if b == 0 {
                '0'
            } else {
                '·'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Epoch 7904 of 2D seed 1: tile (176,41) followed by tile (177,43).
    const PARENT: [u8; 64] = [91,118,91,118,91,118,91,118,91,118,91,118,199,118,44,118,205,118,143,118,226,118,86,118,37,118,161,118,37,118,37,118,37,118,171,118,133,118,210,118,60,118,133,118,44,118,124,118,66,118,154,118,143,118,125,118,125,118,31,31,31,31,31,31];
    const PARTNER: [u8; 64] = [93,164,59,206,59,68,59,100,59,85,88,2,38,135,118,16,117,157,59,242,59,180,59,48,17,123,59,2,59,206,232,2,55,207,59,2,238,59,59,2,59,6,59,10,37,61,52,68,59,162,59,41,59,91,59,216,59,29,0,0,0,0,0,165];

    // One test, because SEALED is global and tests run in parallel.
    #[test]
    fn sealed_seam_blocks_the_birth_but_not_a_replicator() {
        let born = |sealed: bool| {
            SEALED.store(sealed, Ordering::Relaxed);
            let mut t = [0u8; TAPE];
            t[..PROG].copy_from_slice(&PARENT);
            t[PROG..].copy_from_slice(&PARTNER);
            let (_, seams) = run_seam(&mut t);
            (copy_score(&t[PROG..], &mut Rng::new(3), 5) >= 0.9, seams.len())
        };
        assert_eq!(born(false), (true, 1));
        let (copier, seams) = born(true);
        assert_eq!(seams, 0);
        assert!(!copier, "sealed seam should prevent the birth");

        // The replicator that birth produced still copies itself when sealed.
        SEALED.store(false, Ordering::Relaxed);
        let mut t = [0u8; TAPE];
        t[..PROG].copy_from_slice(&PARENT);
        t[PROG..].copy_from_slice(&PARTNER);
        run(&mut t);
        let child: Vec<u8> = t[PROG..].to_vec();
        SEALED.store(true, Ordering::Relaxed);
        assert!(copy_score(&child, &mut Rng::new(5), 5) >= 0.9);
        SEALED.store(false, Ordering::Relaxed);

        // Heads variant: bytes 0,1 aim head0 at 0 (128 % 128, but nonzero so
        // the loop is entered) and head1 at 64; `[.>}]` then copies forward.
        let mut fwd = [1u8; PROG];
        fwd[..7].copy_from_slice(&[128, 64, b'[', b'.', b'>', b'}', b']']);
        HEADS.store(true, Ordering::Relaxed);
        let mut t = [0u8; TAPE];
        Rng::new(9).fill(&mut t);
        t[..PROG].copy_from_slice(&fwd);
        run(&mut t);
        assert_eq!(&t[PROG..], &fwd[..], "forward copier should copy itself exactly");
        HEADS.store(false, Ordering::Relaxed);
        let mut t = [0u8; TAPE];
        Rng::new(9).fill(&mut t);
        t[..PROG].copy_from_slice(&fwd);
        run(&mut t);
        assert_ne!(&t[PROG..], &fwd[..]);

        // Knierim et al.'s detector: replicators score high, noise doesn't.
        let mut hand = [1u8; PROG];
        hand[..5].copy_from_slice(b"[{.>]");
        hand[59..].copy_from_slice(b"]>.{[");
        let mut rng = Rng::new(11);
        assert!(knierim_score(&hand, &mut rng) >= 48);
        assert!(knierim_score(&child, &mut rng) >= 48, "evolved replicator: {}", knierim_score(&child, &mut rng));
        let mut noise = [0u8; PROG];
        rng.fill(&mut noise);
        assert!(knierim_score(&noise, &mut rng) < 20);
    }
}
