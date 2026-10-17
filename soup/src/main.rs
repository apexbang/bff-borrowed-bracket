use clap::Parser;
use soup::*;
use std::collections::HashMap;
use std::io::Write;
use std::time::Instant;

#[derive(Parser)]
#[command(about = "Primordial BFF soup: watch self-replicators appear from noise")]
struct Args {
    /// Number of programs (log2).
    #[arg(long, default_value_t = 17)]
    log_n: u32,
    #[arg(long, default_value_t = 16384)]
    epochs: u64,
    /// Per-byte mutation probability per epoch.
    #[arg(long, default_value_t = 0.00024)]
    mutation: f64,
    #[arg(long, default_value_t = 1)]
    seed: u64,
    #[arg(long, default_value_t = 8)]
    threads: usize,
    /// Measure entropy every this many epochs.
    #[arg(long, default_value_t = 64)]
    every: u64,
    /// Stop this many epochs after high-order entropy first passes 1 bit.
    #[arg(long)]
    stop_after: Option<u64>,
    /// Save the soup every this many epochs (pre-life snapshots).
    #[arg(long)]
    snapshot_every: Option<u64>,
    /// Save the soup at these epochs.
    #[arg(long, value_delimiter = ',')]
    snapshot_at: Vec<u64>,
    /// Save the pairing (u32 little-endian permutation) used at these epochs.
    #[arg(long, value_delimiter = ',')]
    pairs_at: Vec<u64>,
    /// Directory for log.csv and soup snapshots.
    #[arg(long, default_value = "runs/default")]
    out: String,
    #[arg(long)]
    quiet: bool,
}

fn top_programs(s: &Soup, k: usize) -> Vec<(usize, Vec<u8>)> {
    let mut counts: HashMap<&[u8], usize> = HashMap::new();
    for i in 0..s.n {
        *counts.entry(s.program(i)).or_default() += 1;
    }
    let mut v: Vec<_> = counts.into_iter().map(|(p, c)| (c, p.to_vec())).collect();
    v.sort_by(|a, b| b.0.cmp(&a.0));
    v.truncate(k);
    v
}

fn main() {
    let a = Args::parse();
    rayon::ThreadPoolBuilder::new().num_threads(a.threads).build_global().unwrap();
    std::fs::create_dir_all(&a.out).unwrap();
    let mut log = std::fs::File::create(format!("{}/log.csv", a.out)).unwrap();
    writeln!(log, "epoch,ops_per_pair,shannon,compressed_bpb,hoe,top_count,secs").unwrap();

    let n = 1usize << a.log_n;
    let mut s = Soup::new(n, a.mutation, a.seed);
    let t0 = Instant::now();
    let mut ops_acc = 0u64;
    let mut transition: Option<u64> = None;

    while s.epoch < a.epochs {
        ops_acc += s.step().ops;
        if a.pairs_at.contains(&s.epoch) {
            let bytes: Vec<u8> = s.order().iter().flat_map(|x| x.to_le_bytes()).collect();
            std::fs::write(format!("{}/pairs_{:06}.bin", a.out, s.epoch), bytes).unwrap();
        }
        if a.snapshot_every.is_some_and(|k| s.epoch % k == 0) || a.snapshot_at.contains(&s.epoch) {
            std::fs::write(format!("{}/soup_{:06}.bin", a.out, s.epoch), &s.data).unwrap();
        }
        if s.epoch % a.every == 0 {
            let h = shannon(&s.data);
            let c = compressed_bpb(&s.data);
            let top = top_programs(&s, 1);
            let opp = ops_acc as f64 / (a.every as f64 * (n / 2) as f64);
            ops_acc = 0;
            let secs = t0.elapsed().as_secs_f64();
            writeln!(log, "{},{:.2},{:.4},{:.4},{:.4},{},{:.1}", s.epoch, opp, h, c, h - c, top[0].0, secs)
                .unwrap();
            if !a.quiet {
                println!(
                    "ep {:6} ops/pair {:7.1} H {:.3} K {:.3} HOE {:6.3} top {:5}x {}  [{:.0}s]",
                    s.epoch, opp, h, c, h - c, top[0].0, show(&top[0].1), secs
                );
            }
            if transition.is_none() && h - c > 1.0 {
                transition = Some(s.epoch);
                println!("TRANSITION seed {} at epoch {}", a.seed, s.epoch);
                std::fs::write(format!("{}/soup_at_transition.bin", a.out), &s.data).unwrap();
            }
            if let (Some(t), Some(k)) = (transition, a.stop_after) {
                if s.epoch >= t + k {
                    break;
                }
            }
        }
    }
    std::fs::write(format!("{}/soup_final.bin", a.out), &s.data).unwrap();
    let mut f = std::fs::File::create(format!("{}/top.txt", a.out)).unwrap();
    for (c, p) in top_programs(&s, 20) {
        writeln!(f, "{:6} {}", c, show(&p)).unwrap();
    }
    println!(
        "done seed {} epochs {} transition {:?} in {:.0}s",
        a.seed, s.epoch, transition, t0.elapsed().as_secs_f64()
    );
}
