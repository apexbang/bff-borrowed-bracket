//! Birth forensics for the well-mixed soup: snapshots of epoch e-1 and e,
//! the pairing used at e, and a program index. Replays that one interaction.
//! usage: origin_mixed <run dir> <epoch> <index> [generations back]

use soup::*;

fn load(dir: &str, what: &str, e: u64) -> Vec<u8> {
    std::fs::read(format!("{dir}/{what}_{e:06}.bin")).unwrap()
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let dir = &a[1];
    let mut e: u64 = a[2].parse().unwrap();
    let mut idx: usize = a[3].parse().unwrap();
    let back: usize = a.get(4).map_or(1, |s| s.parse().unwrap());
    let prog = |d: &[u8], i: usize| d[i * PROG..(i + 1) * PROG].to_vec();

    for _ in 0..back {
        let (before, after) = (load(dir, "soup", e - 1), load(dir, "soup", e));
        let order: Vec<usize> = load(dir, "pairs", e)
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes(c.try_into().unwrap()) as usize)
            .collect();
        let pos = order.iter().position(|&x| x == idx).unwrap();
        let (first, second) = (order[pos & !1], order[pos | 1]);
        let partner = if first == idx { second } else { first };
        let mut t = [0u8; TAPE];
        t[..PROG].copy_from_slice(&prog(&before, first));
        t[PROG..].copy_from_slice(&prog(&before, second));
        let ops = run(&mut t);
        let mine = if first == idx { &t[..PROG] } else { &t[PROG..] };
        let ok = mine.iter().zip(prog(&after, idx)).filter(|(p, q)| **p == *q).count();
        println!("epoch {e}: program {idx} was {} half, partner {partner}; {ops} ops; replay matches {ok}/64",
            if first == idx { "first" } else { "second" });
        println!("  first  {}", show(&prog(&before, first)));
        println!("  second {}", show(&prog(&before, second)));
        println!("  after  {}{}", show(&t[..PROG]), show(&t[PROG..]));
        // Follow whichever half held the code that did the work: the partner,
        // if the program was rewritten, else the program itself.
        let changed = prog(&before, idx) != prog(&after, idx);
        println!("  program {idx} {}\n", if changed { "was rewritten" } else { "unchanged" });
        if changed && ops > 0 {
            idx = partner;
        }
        e -= 1;
        if !std::path::Path::new(&format!("{dir}/pairs_{e:06}.bin")).exists() {
            break;
        }
    }
}
