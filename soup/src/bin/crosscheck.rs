//! Check the JS port (web/sim.js) against `run`: tapes in, tapes and op counts out.
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let inp = std::fs::read(&a[1]).unwrap();
    let out = std::fs::read(&a[2]).unwrap();
    let cnt = std::fs::read(&a[3]).unwrap();
    let (mut bad_tape, mut bad_ops) = (0, 0);
    for (i, (t, o)) in inp.chunks_exact(128).zip(out.chunks_exact(128)).enumerate() {
        let mut tape: [u8; 128] = t.try_into().unwrap();
        let ops = soup::run(&mut tape);
        bad_tape += (tape[..] != *o) as usize;
        bad_ops += (ops as u32 != u32::from_le_bytes(cnt[i * 4..i * 4 + 4].try_into().unwrap())) as usize;
    }
    println!("{} tapes: {} differ, {} op counts differ", inp.len() / 128, bad_tape, bad_ops);
}
