//! Forensics for the birth of a replicator in the 2D soup: given snapshots
//! of epoch e-1 and e and the tile where a copier first shows up, find the
//! neighbour pairing at epoch e that turns the old tile into the new one.

use soup::*;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (before, after) = (std::fs::read(&a[1]).unwrap(), std::fs::read(&a[2]).unwrap());
    let (x, y): (usize, usize) = (a[3].parse().unwrap(), a[4].parse().unwrap());
    let w = 240;
    let c = y * w + x;
    let prog = |d: &[u8], i: usize| d[i * PROG..(i + 1) * PROG].to_vec();
    let target = prog(&after, c);
    println!("tile ({x},{y}) before: {}", show(&prog(&before, c)));
    println!("tile ({x},{y}) after:  {}\n", show(&target));

    for dy in -2isize..=2 {
        for dx in -2isize..=2 {
            let (nx, ny) = (x as isize + dx, y as isize + dy);
            if (dx, dy) == (0, 0) || nx < 0 || ny < 0 || nx >= w as isize || ny >= 135 {
                continue;
            }
            let n = ny as usize * w + nx as usize;
            for c_first in [true, false] {
                let mut t = [0u8; TAPE];
                let (ci, ni) = if c_first { (0, PROG) } else { (PROG, 0) };
                t[ci..ci + PROG].copy_from_slice(&prog(&before, c));
                t[ni..ni + PROG].copy_from_slice(&prog(&before, n));
                let ops = run(&mut t);
                let got = &t[ci..ci + PROG];
                let same = got.iter().zip(&target).filter(|(p, q)| p == q).count();
                // The neighbour should also match its own after-state if it was the partner.
                let n_same = t[ni..ni + PROG].iter().zip(&after[n * PROG..(n + 1) * PROG]).filter(|(p, q)| p == q).count();
                if same >= 60 {
                    println!(
                        "partner ({nx},{ny}) {} — tile matches {same}/64, partner matches {n_same}/64, {ops} ops",
                        if c_first { "second" } else { "first" }
                    );
                    let (first, second) = if c_first { (c, n) } else { (n, c) };
                    println!("  tape before: {}{}", show(&prog(&before, first)), show(&prog(&before, second)));
                    println!("  tape after:  {}", show(&t));
                }
            }
        }
    }
}
