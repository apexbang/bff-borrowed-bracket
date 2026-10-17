//! Find and explain the birth of life in a 2D soup, automatically.
//!
//! Runs the grid keeping the last `ring` epochs (soup + pairings) in memory.
//! When high-order entropy passes 1 bit, it finds the first epoch with a
//! heritable copier, then walks that lineage back one interaction at a time:
//! as long as the tile's previous self, or the partner that rewrote it, was
//! already a copier, keep going. The first interaction where neither was a
//! copier is the birth. Report it, including any loop that closed across the
//! seam between the two programs.

use clap::Parser;
use rayon::prelude::*;
use soup::*;
use std::collections::VecDeque;

#[derive(Parser)]
struct Args {
    #[arg(long, default_value_t = 1)]
    seed: u64,
    #[arg(long, default_value_t = 240)]
    w: usize,
    #[arg(long, default_value_t = 135)]
    h: usize,
    #[arg(long, default_value_t = 30000)]
    epochs: u64,
    #[arg(long, default_value_t = 0.00024)]
    mutation: f64,
    /// Epochs of history kept in memory (2 MB each at full size).
    #[arg(long, default_value_t = 400)]
    ring: usize,
    #[arg(long, default_value_t = 2)]
    threads: usize,
    /// Rule change: loops can't close across the seam between programs.
    #[arg(long)]
    sealed: bool,
    /// Fraction of itself a program must copy to count as a copier.
    #[arg(long, default_value_t = 0.9)]
    min_fidelity: f64,
    /// Use the BFF variant where bytes 0 and 1 set the heads.
    #[arg(long)]
    heads: bool,
    /// Intervention: every epoch, replace a leading `]` with a random other byte.
    #[arg(long)]
    strip_armour: bool,
    /// Save each traced birth's pre-interaction tape (128 bytes, then one byte:
    /// 0 if the newborn is the first half, 1 if second) to DIR/s<seed>_e<epoch>.bin.
    #[arg(long)]
    save_birth: Option<String>,
    /// Start from this soup snapshot instead of random bytes (epochs count from 0).
    #[arg(long)]
    load: Option<String>,
    /// Write the soup to <file>_<epoch>.bin at this epoch, then stop.
    #[arg(long, num_args = 2, value_names = ["EPOCH", "FILE"])]
    dump_at: Vec<String>,
}

struct Frame {
    epoch: u64,
    data: Vec<u8>,
    pairs: Vec<(usize, usize)>,
}

fn prog(d: &[u8], i: usize) -> &[u8] {
    &d[i * PROG..(i + 1) * PROG]
}

static MIN_FIDELITY: std::sync::OnceLock<f64> = std::sync::OnceLock::new();

fn min_fidelity() -> f64 {
    *MIN_FIDELITY.get().unwrap_or(&0.9)
}

fn is_copier(p: &[u8], salt: u64) -> bool {
    copy_score(p, &mut Rng::new(salt ^ 0x5eed), 3) >= min_fidelity()
}

fn heritable_tiles(d: &[u8], salt: u64) -> Vec<usize> {
    (0..d.len() / PROG)
        .into_par_iter()
        .filter(|&i| {
            let mut rng = Rng::new(salt ^ (i as u64).wrapping_mul(0x9e3779b97f4a7c15));
            let p = prog(d, i);
            let f = min_fidelity();
            copy_score(p, &mut rng, 1) >= f / 2.0 && copy_score(p, &mut rng, 3) >= f && heritable_at(p, &mut rng, f)
        })
        .collect()
}

/// Walk the lineage of `tile` back from ring[k] to its birth and report it.
fn trace(ring: &VecDeque<Frame>, mut k: usize, mut tile: usize, w: usize, save: Option<(&str, u64)>) {
    let xy = |i: usize| format!("({},{})", i % w, i / w);
    let mut steps = 0;
    loop {
        if k == 0 {
            println!("  ran out of history after {steps} steps");
            return;
        }
        let (before, now) = (&ring[k - 1].data, &ring[k]);
        let e = now.epoch;
        let Some(&(i, j)) = now.pairs.iter().find(|&&(i, j)| i == tile || j == tile) else {
            if prog(before, tile) != prog(&now.data, tile) && !is_copier(prog(before, tile), e) {
                println!("  BIRTH epoch {e}: tile {} changed without interacting: mutation", xy(tile));
                println!("    before {}", show(prog(before, tile)));
                println!("    after  {}", show(prog(&now.data, tile)));
                let diff: Vec<String> = (0..PROG)
                    .filter(|&b| before[tile * PROG + b] != now.data[tile * PROG + b])
                    .map(|b| format!("byte {b}: {} -> {}", show(&[before[tile * PROG + b]]), show(&[now.data[tile * PROG + b]])))
                    .collect();
                println!("    changed {}  |  was copier before: {}", diff.join(", "), is_copier(prog(before, tile), e));
                return;
            }
            k -= 1;
            continue;
        };
        let partner = if i == tile { j } else { i };
        if prog(before, tile) == prog(&now.data, tile) {
            k -= 1;
            continue;
        }
        steps += 1;
        if is_copier(prog(before, tile), e) {
            k -= 1;
            continue;
        }
        if is_copier(prog(before, partner), e) {
            tile = partner;
            k -= 1;
            continue;
        }

        // Birth: neither the tile's old self nor its partner was a copier.
        let mut t = [0u8; TAPE];
        t[..PROG].copy_from_slice(prog(before, i));
        t[PROG..].copy_from_slice(prog(before, j));
        if let Some((dir, seed)) = save {
            let mut rec = t.to_vec();
            rec.push((i != tile) as u8);
            std::fs::write(format!("{dir}/s{seed}_e{e}.bin"), rec).unwrap();
        }
        let (ops, seams) = run_seam(&mut t);
        let child = if i == tile { &t[..PROG] } else { &t[PROG..] };
        let matches = child.iter().zip(prog(&now.data, tile)).filter(|(p, q)| p == q).count();
        println!(
            "  BIRTH epoch {e}: tile {} rewritten while {} half, partner {}; {ops} ops; replay {matches}/64; \
             lineage steps {steps}",
            xy(tile), if i == tile { "first" } else { "second" }, xy(partner)
        );
        println!("    first  {}", show(prog(before, i)));
        println!("    second {}", show(prog(before, j)));
        println!("    after  {}{}", show(&t[..PROG]), show(&t[PROG..]));
        println!(
            "    child copier: {}  |  seam loops: {}",
            is_copier(child, 1),
            if seams.is_empty() {
                "none".to_string()
            } else {
                seams.iter().map(|(f, to, n)| format!("]@{f}->[@{to} x{n}")).collect::<Vec<_>>().join(", ")
            }
        );
        return;
    }
}

fn main() {
    let a = Args::parse();
    rayon::ThreadPoolBuilder::new().num_threads(a.threads).build_global().unwrap();
    SEALED.store(a.sealed, std::sync::atomic::Ordering::Relaxed);
    HEADS.store(a.heads, std::sync::atomic::Ordering::Relaxed);
    MIN_FIDELITY.set(a.min_fidelity).unwrap();
    let mut g = Grid::new(a.w, a.h, a.mutation, a.seed);
    if let Some(f) = &a.load {
        g.data = std::fs::read(f).unwrap();
        assert_eq!(g.data.len(), a.w * a.h * PROG);
    }
    let mut strip_rng = Rng::new(a.seed ^ 0xa4a0);
    let mut ring: VecDeque<Frame> = VecDeque::with_capacity(a.ring + 1);
    let w = a.w;
    let xy = |i: usize| format!("({},{})", i % w, i / w);
    let has = |f: &Frame| !heritable_tiles(&f.data, f.epoch).is_empty();
    // A heritable lineage is "active" from the census that first sees it
    // until a census finds none; each new active period gets traced once.
    let mut active = false;

    loop {
        let pairs = g.step();
        if a.strip_armour {
            for p in g.data.chunks_exact_mut(PROG) {
                while p[0] == b']' {
                    p[0] = strip_rng.next_u64() as u8;
                }
            }
        }
        if a.dump_at.len() == 2 && g.epoch == a.dump_at[0].parse::<u64>().unwrap() {
            std::fs::write(format!("{}_{}.bin", a.dump_at[1], g.epoch), &g.data).unwrap();
            println!("seed {} | dumped at epoch {}", a.seed, g.epoch);
            return;
        }
        ring.push_back(Frame { epoch: g.epoch, data: g.data.clone(), pairs });
        if ring.len() > a.ring {
            ring.pop_front();
        }
        if g.epoch % 100 == 0 {
            let present = has(ring.back().unwrap());
            if present && !active {
                // Step back while copiers are present, then forward to the first.
                let mut k = ring.len() - 1;
                while k >= 10 && has(&ring[k - 10]) {
                    k -= 10;
                }
                while k > 0 && has(&ring[k - 1]) {
                    k -= 1;
                }
                let first = heritable_tiles(&ring[k].data, ring[k].epoch);
                println!("seed {} | heritable copier at epoch {} tile {} ({} tiles)",
                    a.seed, ring[k].epoch, xy(first[0]), first.len());
                trace(&ring, k, first[0], w, a.save_birth.as_deref().map(|d| (d, a.seed)));
            }
            if !present && active {
                println!("  lineage gone by epoch {}", g.epoch);
            }
            active = present;
            if high_order_entropy(&g.data) > 1.0 {
                println!("seed {} | TRANSITION at epoch {}", a.seed, g.epoch);
                return;
            }
        }
        if g.epoch >= a.epochs {
            let instr = g.data.iter().filter(|&&b| is_op(b)).count() as f64 / g.data.len() as f64;
            println!("seed {} | no transition in {} epochs | instr {:.2}%", a.seed, a.epochs, instr * 100.0);
            return;
        }
    }
}
