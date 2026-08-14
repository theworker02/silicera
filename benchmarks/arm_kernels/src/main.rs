//! Standalone kernel binary for dual-artifact comparisons.
//!
//! Built twice by Silicera with different `RUSTFLAGS` (`target-cpu=x86-64-v2` vs
//! `target-cpu=native`). Prints `median_ns=` for the lab harness to parse.
//!
//! Not a workspace member — keeps the primary crate count at four.

use std::env;
use std::hint::black_box;
use std::time::Instant;

fn main() {
    let mut kernel = "dot_f32".to_string();
    let mut iterations: usize = 40;
    let mut args = env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--kernel" => {
                if let Some(v) = args.next() {
                    kernel = v;
                }
            }
            "--iterations" => {
                if let Some(v) = args.next() {
                    iterations = v.parse().unwrap_or(40);
                }
            }
            _ => {}
        }
    }

    // Warmup
    for _ in 0..5 {
        run_kernel(&kernel);
    }

    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let t0 = Instant::now();
        let _ = black_box(run_kernel(&kernel));
        samples.push(t0.elapsed().as_nanos() as f64);
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Less));
    let median = if samples.is_empty() {
        0.0
    } else if samples.len() % 2 == 0 {
        (samples[samples.len() / 2 - 1] + samples[samples.len() / 2]) / 2.0
    } else {
        samples[samples.len() / 2]
    };

    println!("kernel={kernel}");
    println!("iterations={iterations}");
    println!("median_ns={median:.0}");
    println!("target_cpu_note=see RUSTFLAGS of parent build");
}

fn run_kernel(name: &str) -> u64 {
    match name {
        "dot_f32" => dot_f32(),
        "saxpy_f32" => saxpy_f32(),
        "checksum_u8" => checksum_u8(),
        "reduce_i32" => reduce_i32(),
        other => {
            eprintln!("unknown kernel {other}; using dot_f32");
            dot_f32()
        }
    }
}

fn dot_f32() -> u64 {
    const N: usize = 1 << 16;
    let a: Vec<f32> = (0..N).map(|i| (i as f32) * 0.001 + 1.0).collect();
    let b: Vec<f32> = (0..N).map(|i| (i as f32) * 0.0007 + 0.5).collect();
    let mut s = 0.0f32;
    for i in 0..N {
        s = black_box(s + a[i] * b[i]);
    }
    black_box(s.to_bits() as u64)
}

fn saxpy_f32() -> u64 {
    const N: usize = 1 << 16;
    let mut y: Vec<f32> = (0..N).map(|i| i as f32).collect();
    let x: Vec<f32> = (0..N).map(|i| (i as f32) * 0.5).collect();
    let a = 1.7f32;
    for i in 0..N {
        y[i] = black_box(a * x[i] + y[i]);
    }
    black_box(y[N - 1].to_bits() as u64)
}

fn checksum_u8() -> u64 {
    const N: usize = 1 << 20;
    let data: Vec<u8> = (0..N).map(|i| (i % 251) as u8).collect();
    let mut s = 0u64;
    for &b in &data {
        s = s.wrapping_add(b as u64);
    }
    black_box(s)
}

fn reduce_i32() -> u64 {
    const N: usize = 1 << 16;
    let data: Vec<i32> = (0..N as i32).map(|i| i.wrapping_mul(3).wrapping_add(1)).collect();
    let mut s = 0i64;
    for &x in &data {
        s = s.wrapping_add(x as i64);
    }
    black_box(s as u64)
}
