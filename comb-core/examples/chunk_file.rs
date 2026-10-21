//! Small executable for testing the chunking code. Given a file and
//! a window size, it prints the number of chunks and some useful
//! statistics: mean/median/p99 chunk size, a size histogram, the
//! clamp-cut rate and throughput.
//!
//! Usage: `chunk_file <path> <h>`
#![allow(unsafe_code)]

use comb_core::boundary::BoundaryFinder;
use comb_core::boundary::Extremum;
use comb_core::chunk::chunk_range;
use memmap2::Advice;
use memmap2::Mmap;
use std::error::Error;
use std::fs::File;
use std::time::Duration;
use std::time::Instant;

const USAGE: &str = "usage: chunk_file <path> <h>";

/// Width in characters of the longest histogram bar.
const BAR_WIDTH: usize = 40;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let file_path = args.get(1).ok_or(USAGE)?;
    let h = args.get(2).ok_or(USAGE)?.parse::<usize>()?;
    let file = File::open(file_path)?;

    // Mapping a zero-length file is not portable, and there is nothing to chunk anyway.
    if file.metadata()?.len() == 0 {
        println!("{file_path}: empty file, no chunks");
        return Ok(());
    }

    // SAFETY: the file is opened read-only and is not modified or truncated while the map
    // is alive. This is a measurement tool run against corpus files nothing else writes to.
    let mmap = unsafe { Mmap::map(&file)? };
    if let Err(e) = mmap.advise(Advice::Sequential) {
        eprintln!("warning: madvise(SEQUENTIAL) failed: {e}");
    }

    let finder = Extremum::new(h);
    let start = Instant::now();
    let (chunks, stats) = chunk_range(&mmap, (0..mmap.len()).into(), &finder);
    let elapsed = start.elapsed();

    let mut lens: Vec<usize> = chunks.iter().map(|c| c.len).collect();
    lens.sort_unstable();

    // Under D2 the final chunk ends at the range end, which is not a cut.
    let cuts = lens.len() as u64 - 1;
    assert_eq!(
        stats.scan_stats.rule_cuts + stats.clamp_cuts,
        cuts,
        "rule cuts + clamp cuts must equal chunks - 1"
    );

    print_summary(&Summary {
        path: file_path,
        h,
        file_len: mmap.len(),
        lens: &lens,
        expected_mean: finder.mean_chunk_size(),
        rule_cuts: stats.scan_stats.rule_cuts,
        clamp_cuts: stats.clamp_cuts,
        elapsed,
    });
    println!();
    print_histogram(&histogram(&lens), finder.max_chunk_size());

    Ok(())
}

/// Everything the summary block prints.
struct Summary<'a> {
    path: &'a str,
    h: usize,
    file_len: usize,
    /// Chunk lengths, sorted ascending, never empty.
    lens: &'a [usize],
    expected_mean: usize,
    rule_cuts: u64,
    clamp_cuts: u64,
    elapsed: Duration,
}

#[allow(
    clippy::cast_precision_loss,
    reason = "statistics; precision loss past 2^53 is irrelevant"
)]
fn print_summary(s: &Summary) {
    let n = s.lens.len();
    let mean = s.file_len as f64 / n as f64;
    let median = s.lens[n / 2];
    // Nearest-rank p99: the smallest length with at least 99% of chunks at or below it.
    let p99 = s.lens[(99 * n).div_ceil(100) - 1];

    let cuts = s.rule_cuts + s.clamp_cuts;
    let clamp_rate = if cuts == 0 {
        0.0
    } else {
        s.clamp_cuts as f64 / cuts as f64
    };

    let secs = s.elapsed.as_secs_f64();
    let mib_per_sec = s.file_len as f64 / (1024.0 * 1024.0) / secs;

    println!("file        {} ({} bytes)", s.path, s.file_len);
    println!("h           {}", s.h);
    println!("chunks      {n}");
    println!(
        "mean        {mean:.0} B  (expected {}, ratio {:.3})",
        s.expected_mean,
        mean / s.expected_mean as f64
    );
    println!("median      {median} B");
    println!("p99         {p99} B");
    println!(
        "cuts        {cuts}  (rule {}, clamp {})",
        s.rule_cuts, s.clamp_cuts
    );
    println!("clamp rate  {clamp_rate:.4}");
    println!("time        {secs:.3} s");
    println!("throughput  {mib_per_sec:.1} MiB/s  (includes Blake3)");
}

/// Counts per power-of-two bucket: index `k` holds lengths in `[2^k, 2^(k+1))`.
/// `lens` must be sorted ascending and contain no zeros.
fn histogram(lens: &[usize]) -> Vec<u64> {
    let Some(&max) = lens.last() else {
        return Vec::new();
    };

    let mut counts = vec![0u64; max.ilog2() as usize + 1];
    for &len in lens {
        counts[len.ilog2() as usize] += 1;
    }
    counts
}

fn print_histogram(counts: &[u64], max_chunk_size: usize) {
    let Some(first) = counts.iter().position(|&c| c > 0) else {
        return;
    };
    let peak = counts.iter().copied().max().unwrap_or(1);
    let clamp_bucket = max_chunk_size.ilog2() as usize;

    for (k, &count) in counts.iter().enumerate().skip(first) {
        // At least one '#' for a non-empty bucket, so it can't look empty.
        let width = if count == 0 {
            0
        } else {
            (count * BAR_WIDTH as u64 / peak).max(1)
        };
        let width = usize::try_from(width).expect("width is at most BAR_WIDTH");
        let marker = if k == clamp_bucket { "  <- clamp" } else { "" };
        println!(
            "{:>8} .. {:>8} | {count:>10} | {:<bar$}{marker}",
            human(1 << k),
            human(1 << (k + 1)),
            "#".repeat(width),
            bar = BAR_WIDTH,
        );
    }
}

/// Formats a byte count with a binary unit, e.g. `4 KiB`.
fn human(bytes: usize) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];

    let mut value = bytes;
    let mut unit = 0;
    while value >= 1024 && value.is_multiple_of(1024) && unit < UNITS.len() - 1 {
        value /= 1024;
        unit += 1;
    }
    format!("{value} {}", UNITS[unit])
}
