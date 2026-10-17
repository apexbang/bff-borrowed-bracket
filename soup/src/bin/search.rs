//! How rare are replicators? Draw random 64-byte programs, either uniform
//! or with the byte frequencies of a soup snapshot, and count the ones that
//! copy most of themselves into a random partner. Or, with --census, test
//! every program actually present in a snapshot.

use clap::Parser;
use rayon::prelude::*;
use soup::*;

#[derive(Parser)]
struct Args {
    /// Programs to sample (ignored with --census).
    #[arg(long, default_value_t = 100_000_000)]
    samples: u64,
    /// Draw bytes i.i.d. from this snapshot's byte histogram instead of uniform.
    #[arg(long)]
    like: Option<String>,
    /// Test every program in this snapshot instead of sampling.
    #[arg(long)]
    census: Option<String>,
    #[arg(long, default_value_t = 8)]
    threads: usize,
    #[arg(long, default_value_t = 7)]
    seed: u64,
    /// Use the Knierim et al. detector (score >= 48) instead of my copier tests.
    #[arg(long)]
    knierim: bool,
    /// With --census, also count latent copiers: programs that copy themselves
    /// only when the partner's first byte is `]`.
    #[arg(long)]
    latent: bool,
    /// With --census, print the index of every copier found.
    #[arg(long)]
    list: bool,
    /// Fraction of the program's bytes that must turn up in the partner.
    #[arg(long, default_value_t = 0.9)]
    min_fidelity: f64,
}

/// Cumulative byte distribution for inverse-CDF sampling.
fn cdf_of(path: &str) -> Vec<f64> {
    let data = std::fs::read(path).unwrap();
    let mut c = [0f64; 256];
    for &b in &data {
        c[b as usize] += 1.0;
    }
    let mut acc = 0.0;
    c.iter()
        .map(|x| {
            acc += x / data.len() as f64;
            acc
        })
        .collect()
}

fn main() {
    let a = Args::parse();
    rayon::ThreadPoolBuilder::new().num_threads(a.threads).build_global().unwrap();

    if let Some(path) = &a.census {
        let data = std::fs::read(path).unwrap();
        let hits: Vec<(usize, f64, bool)> = data
            .par_chunks_exact(PROG)
            .enumerate()
            .filter_map(|(i, p)| {
                let mut rng = Rng::new(a.seed ^ (i as u64).wrapping_mul(0x9e37));
                let s = copy_score(p, &mut rng, 3);
                (s >= a.min_fidelity).then(|| (i, s, heritable(p, &mut rng)))
            })
            .collect();
        if a.latent {
            let latent: Vec<usize> = data
                .par_chunks_exact(PROG)
                .enumerate()
                .filter_map(|(i, p)| {
                    let mut rng = Rng::new(a.seed ^ (i as u64).wrapping_mul(0x51ed));
                    let with = copy_score_with(p, &mut rng, 3, Some(b']'));
                    (with >= a.min_fidelity && copy_score(p, &mut rng, 3) < a.min_fidelity).then_some(i)
                })
                .collect();
            println!("{path}: {} latent copiers (need a partner starting with ])", latent.len());
            for i in latent.iter().take(5) {
                println!("  L {}", show(&data[i * PROG..][..PROG]));
            }
        }
        let ops = data.iter().filter(|&&b| is_op(b)).count() as f64 / data.len() as f64;
        let her = hits.iter().filter(|h| h.2).count();
        println!(
            "{path}: {} copiers, {} heritable, of {} programs; op fraction {:.3}",
            hits.len(), her, data.len() / PROG, ops
        );
        if a.list {
            for (i, _, h) in &hits {
                println!("hit {i} {}", *h as u8);
            }
        }
        for (i, s, h) in hits.iter().take(5) {
            println!("  {:.2} {} {}", s, if *h { "H" } else { " " }, show(&data[i * PROG..][..PROG]));
        }
        return;
    }

    let cdf = a.like.as_deref().map(cdf_of);
    let chunk = 1_000_000u64;
    let found: Vec<(Vec<u8>, f64, bool)> = (0..a.samples.div_ceil(chunk))
        .into_par_iter()
        .flat_map_iter(|c| {
            let mut rng = Rng::new(a.seed.wrapping_mul(1_000_003) ^ c);
            let mut out = Vec::new();
            let mut p = [0u8; PROG];
            for _ in 0..chunk {
                match &cdf {
                    None => rng.fill(&mut p),
                    Some(cdf) => {
                        for b in p.iter_mut() {
                            let u = rng.unit();
                            *b = cdf.partition_point(|&x| x < u).min(255) as u8;
                        }
                    }
                }
                if a.knierim {
                    // Cheap filter: a replicator in their sense copies itself
                    // (as first half, against noise) in most single runs, so
                    // require one decent copy in three tries before the
                    // 45-run detector.
                    let promising = (0..3).any(|_| {
                        let mut t = [0u8; TAPE];
                        rng.fill(&mut t[PROG..]);
                        t[..PROG].copy_from_slice(&p);
                        run(&mut t);
                        similarity(&p, &t[PROG..]) >= 40
                    });
                    if promising {
                        let k = knierim_score(&p, &mut rng);
                        if k >= 48 {
                            out.push((p.to_vec(), k as f64 / 64.0, true));
                        }
                    }
                    continue;
                }
                // One cheap trial first; only promising ones get more.
                if copy_score(&p, &mut rng, 1) >= a.min_fidelity * 0.5 {
                    let s = copy_score(&p, &mut rng, 4);
                    if s >= a.min_fidelity {
                        out.push((p.to_vec(), s, heritable(&p, &mut rng)));
                    }
                }
            }
            out
        })
        .collect();
    let her = found.iter().filter(|f| f.2).count();
    println!(
        "{} copiers ({:.3e}/program), {} heritable ({:.3e}/program), in {} samples from {}",
        found.len(),
        found.len() as f64 / a.samples as f64,
        her,
        her as f64 / a.samples as f64,
        a.samples,
        a.like.as_deref().unwrap_or("uniform bytes")
    );
    for (p, s, h) in found.iter().filter(|f| f.2).chain(found.iter().filter(|f| !f.2)).take(10) {
        println!("  {:.2} {} {}", s, if *h { "H" } else { " " }, show(p));
    }
}
