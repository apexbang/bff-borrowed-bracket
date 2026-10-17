//! 2D BFF soup: programs live on a torus-free grid and only pair up with
//! neighbours within 2 cells, so replicators spread as visible fronts.
//! Frames are piped straight into ffmpeg; each program is an 8x8 tile.

use clap::Parser;
use rayon::prelude::*;
use soup::*;
use std::io::Write;
use std::process::{Command, Stdio};

#[derive(Parser)]
struct Args {
    #[arg(long, default_value_t = 240)]
    w: usize,
    #[arg(long, default_value_t = 135)]
    h: usize,
    #[arg(long, default_value_t = 20000)]
    epochs: u64,
    #[arg(long, default_value_t = 0.00024)]
    mutation: f64,
    #[arg(long, default_value_t = 1)]
    seed: u64,
    #[arg(long, default_value_t = 8)]
    threads: usize,
    /// Write a frame every this many epochs.
    #[arg(long, default_value_t = 10)]
    every: u64,
    #[arg(long, default_value = "grid.mp4")]
    video: String,
    /// Stop this many epochs after high-order entropy passes 1 bit.
    #[arg(long, default_value_t = 4000)]
    stop_after: u64,
    /// Save the soup (as <video>_<epoch>.bin) at these epochs.
    #[arg(long, value_delimiter = ',')]
    snapshot_at: Vec<u64>,
}

fn color(b: u8) -> [u8; 3] {
    match b {
        b'<' => [230, 80, 80],
        b'>' => [250, 150, 60],
        b'{' => [80, 120, 240],
        b'}' => [60, 190, 250],
        b'+' => [80, 210, 100],
        b'-' => [30, 140, 70],
        b'.' => [250, 230, 80],
        b',' => [190, 110, 230],
        b'[' => [255, 255, 255],
        b']' => [200, 200, 200],
        0 => [0, 0, 0],
        _ => {
            let g = 20 + b / 8;
            [g, g, g + 4]
        }
    }
}

fn main() {
    let a = Args::parse();
    rayon::ThreadPoolBuilder::new().num_threads(a.threads).build_global().unwrap();
    let (w, h) = (a.w, a.h);
    let mut g = Grid::new(w, h, a.mutation, a.seed);

    let (fw, fh) = (w * 8, h * 8);
    let mut ff = Command::new("ffmpeg")
        .args(["-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgb24"])
        .args(["-s", &format!("{fw}x{fh}"), "-r", "30", "-i", "-"])
        .args(["-c:v", "libopenh264", "-b:v", "12M", "-pix_fmt", "yuv420p", &a.video])
        .stdin(Stdio::piped())
        .spawn()
        .expect("ffmpeg");
    let mut pipe = ff.stdin.take().unwrap();
    let mut frame = vec![0u8; fw * fh * 3];
    let mut transition = None;

    for epoch in 1..=a.epochs {
        g.step();
        let data = &g.data;
        if a.snapshot_at.contains(&epoch) {
            std::fs::write(a.video.replace(".mp4", &format!("_{epoch}.bin")), data).unwrap();
        }
        if epoch % a.every == 0 {
            frame.par_chunks_mut(fw * 3 * 8).enumerate().for_each(|(gy, rows)| {
                for gx in 0..w {
                    let p = &data[(gy * w + gx) * PROG..][..PROG];
                    for (k, &b) in p.iter().enumerate() {
                        let (px, py) = (gx * 8 + k % 8, k / 8);
                        rows[(py * fw + px) * 3..][..3].copy_from_slice(&color(b));
                    }
                }
            });
            pipe.write_all(&frame).unwrap();
        }
        if epoch % 100 == 0 {
            let hoe = high_order_entropy(data);
            println!("epoch {epoch:6} HOE {hoe:.3}");
            if transition.is_none() && hoe > 1.0 {
                transition = Some(epoch);
                println!("TRANSITION at {epoch}");
            }
            if transition.is_some_and(|t| epoch >= t + a.stop_after) {
                break;
            }
        }
    }
    drop(pipe);
    ff.wait().unwrap();
    std::fs::write(a.video.replace(".mp4", ".bin"), &g.data).unwrap();
}
