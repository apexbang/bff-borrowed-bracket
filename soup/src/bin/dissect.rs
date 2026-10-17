//! Take a soup snapshot apart: what are the common programs, do they
//! replicate, which of their bytes matter, and how do they do it.

use clap::Parser;
use soup::*;
use std::collections::HashMap;

#[derive(Parser)]
struct Args {
    /// Soup snapshot (.bin, concatenated 64-byte programs).
    soup: String,
    #[arg(long, default_value_t = 5)]
    top: usize,
    #[arg(long, default_value_t = 200)]
    trials: usize,
    /// Print an execution trace of the top program run against noise.
    #[arg(long)]
    trace: bool,
    /// Use the BFF variant where bytes 0 and 1 set the heads.
    #[arg(long)]
    heads: bool,
}

/// How often `p` ends up copied verbatim over a random partner, with `p`
/// in the first half and in the second half of the tape.
fn replication_rate(p: &[u8], trials: usize, rng: &mut Rng) -> (f64, f64) {
    let (mut first, mut second) = (0, 0);
    for _ in 0..trials {
        let mut t = [0u8; TAPE];
        rng.fill(&mut t);
        t[..PROG].copy_from_slice(p);
        run(&mut t);
        first += (t[PROG..] == *p) as usize;

        rng.fill(&mut t);
        t[PROG..].copy_from_slice(p);
        run(&mut t);
        second += (t[..PROG] == *p) as usize;
    }
    (first as f64 / trials as f64, second as f64 / trials as f64)
}

/// Mean fraction of `p`'s bytes found in the partner afterwards, direct or
/// mirrored (copying with h1 walking backwards produces a mirror image).
fn fidelity(p: &[u8], trials: usize, rng: &mut Rng) -> f64 {
    let mut total = 0.0;
    for _ in 0..trials {
        let mut t = [0u8; TAPE];
        rng.fill(&mut t);
        t[..PROG].copy_from_slice(p);
        run(&mut t);
        let direct = (0..PROG).filter(|&i| t[PROG + i] == p[i]).count();
        let mirror = (0..PROG).filter(|&i| t[TAPE - 1 - i] == p[i]).count();
        total += direct.max(mirror) as f64 / PROG as f64;
    }
    total / trials as f64
}

/// Bytes whose knockout (replace with an inert byte) wrecks copying.
fn essential(p: &[u8], rng: &mut Rng) -> Vec<bool> {
    let base = fidelity(p, 50, rng);
    (0..PROG)
        .map(|i| {
            let mut q = p.to_vec();
            q[i] = if p[i] == 1 { 2 } else { 1 };
            fidelity(&q, 50, rng) < base * 0.5
        })
        .collect()
}

fn trace(p: &[u8], rng: &mut Rng) {
    let mut t = [0u8; TAPE];
    rng.fill(&mut t);
    t[..PROG].copy_from_slice(p);
    // A tiny re-implementation with printing; kept separate from the fast path.
    let (mut h0, mut h1, mut pc) = if HEADS.load(std::sync::atomic::Ordering::Relaxed) {
        (t[0] as usize % TAPE, t[1] as usize % TAPE, 2isize)
    } else {
        (0usize, 0usize, 0isize)
    };
    let mut lines = 0;
    for _ in 0..MAX_STEPS {
        let c = t[pc as usize];
        if is_op(c) && lines < 400 {
            println!("pc {:3} {}  h0 {:3} [{:3}]  h1 {:3} [{:3}]", pc, c as char, h0, t[h0], h1, t[h1]);
            lines += 1;
        }
        match c {
            b'<' => h0 = h0.wrapping_sub(1) & (TAPE - 1),
            b'>' => h0 = (h0 + 1) & (TAPE - 1),
            b'{' => h1 = h1.wrapping_sub(1) & (TAPE - 1),
            b'}' => h1 = (h1 + 1) & (TAPE - 1),
            b'+' => t[h0] = t[h0].wrapping_add(1),
            b'-' => t[h0] = t[h0].wrapping_sub(1),
            b'.' => t[h1] = t[h0],
            b',' => t[h0] = t[h1],
            b'[' if t[h0] == 0 => {
                let mut d = 1;
                while d > 0 {
                    pc += 1;
                    if pc as usize >= TAPE {
                        println!("unmatched [ -> halt");
                        return;
                    }
                    d += (t[pc as usize] == b'[') as i32 - (t[pc as usize] == b']') as i32;
                }
            }
            b']' if t[h0] != 0 => {
                let mut d = 1;
                while d > 0 {
                    pc -= 1;
                    if pc < 0 {
                        println!("unmatched ] -> halt");
                        return;
                    }
                    d += (t[pc as usize] == b']') as i32 - (t[pc as usize] == b'[') as i32;
                }
            }
            _ => {}
        }
        pc += 1;
        if pc as usize >= TAPE {
            println!("ran off the end");
            break;
        }
    }
    println!("after: {}", show(&t[..PROG]));
    println!("       {}", show(&t[PROG..]));
}

fn main() {
    let a = Args::parse();
    HEADS.store(a.heads, std::sync::atomic::Ordering::Relaxed);
    let data = std::fs::read(&a.soup).unwrap();
    let n = data.len() / PROG;
    let mut rng = Rng::new(42);

    let mut counts: HashMap<&[u8], usize> = HashMap::new();
    for p in data.chunks_exact(PROG) {
        *counts.entry(p).or_default() += 1;
    }
    let mut v: Vec<_> = counts.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    println!("{} programs, {} distinct; HOE {:.3}", n, v.len(), high_order_entropy(&data));

    // How much of the soup replicates at all, sampled.
    let sample = 2000.min(n);
    let mut reps = 0;
    for _ in 0..sample {
        let i = rng.below(n);
        reps += (fidelity(&data[i * PROG..][..PROG], 4, &mut rng) >= 0.9) as usize;
    }
    println!("{:.1}% of sampled programs copy >=90% of themselves (direct or mirrored) as first half\n", 100.0 * reps as f64 / sample as f64);

    for (p, c) in v.iter().take(a.top) {
        let (f, s) = replication_rate(p, a.trials, &mut rng);
        let fid = fidelity(p, a.trials, &mut rng);
        let ess = essential(p, &mut rng);
        let mask: String = ess.iter().map(|&e| if e { '^' } else { ' ' }).collect();
        println!(
            "{:6}x  exact copy as first {:4.0}% / as second {:4.0}%  fidelity {:4.0}%",
            c, f * 100.0, s * 100.0, fid * 100.0
        );
        println!("   {}", show(p));
        println!("   {}", mask);
        let core: String = p.iter().zip(&ess).filter(|(_, e)| **e).map(|(&b, _)| show(&[b])).collect();
        println!("   essential: {}\n", core);
    }
    if a.trace {
        trace(v[0].0, &mut rng);
    }
}
