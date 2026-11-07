//! Small executable meant to predict how effective the tier ladder may be by
//! measuring the basic hypothesis that stable regions tend to stay stable and
//! volatile regions stay volatile.
//! It does this by chunking at the same tier on multiple versions of a file
//! and measuring how many chunks get changed version to version.
//!
//! Versions are taken in argument order, oldest first. For every anchor version
//! `vt` and lag `k`, all in `vt`'s coordinates:
//!
//! - survivors: `vt` chunks still present in `v(t+k-1)` (all of them when `k == 1`)
//! - new: survivors absent from `v(t-1)`
//! - gone: survivors absent from `v(t+k)`
//! - base rate = bytes(gone) / bytes(survivors)
//! - conditional rate = bytes(new ∩ gone) / bytes(new)
//! - ratio = conditional / base
//! - max = 1 / base, the largest ratio possible (conditional cannot exceed 1); a ratio near
//!   its max is capped by churn, not evidence of a weak effect
//!
//! At lag 1 this is the plain question: does a chunk that just changed change again
//! next step more often than average? At lag `k > 1` it asks the same of new chunks that
//! then survived `k - 1` further versions, which shows how long volatility lasts (the
//! aging horizon of DESIGN.md §4.3). Its `new %` column is the bytes that survived, so a
//! lag whose new chunks all died early shows a tiny sample rather than a real ratio.
//!
//! The pooled row per lag sums numerators and denominators over all non-excluded
//! anchors and divides once. One high-churn anchor dominates those sums, so a median row of
//! the per-anchor ratios is printed as well. Output is a Markdown table for `benches/RESULTS.md`.
//!
//! Usage: `autocorr <h> <v1> <v2> <v3> [<v4> ...]`
#![allow(unsafe_code)]

use comb_core::boundary::Extremum;
use comb_core::chunk::chunk_range;
use comb_core::chunk_id::ChunkId;
use memmap2::Advice;
use memmap2::Mmap;
use std::collections::HashSet;
use std::error::Error;
use std::fs::File;

const USAGE: &str = "usage: autocorr <h> <v1> <v2> <v3> [<v4> ...]";

/// Largest lag measured. Lags beyond `versions - 2` have no data and are skipped.
const MAX_LAG: usize = 3;

/// An anchor whose new fraction is above this is treated as rebuilt or compressed
/// content and kept out of the pooled row.
const MAX_NEW_FRACTION: f64 = 0.95;

/// One version, chunked at tier 0. Only ids and lengths are kept, not the data.
struct Version {
    path: String,
    file_len: usize,
    /// `(id, len)` in file order.
    chunks: Vec<(ChunkId, usize)>,
    ids: HashSet<ChunkId>,
}

/// Byte counts for one `(anchor, lag)` measurement.
#[derive(Default, Clone, Copy)]
struct Counts {
    /// Bytes of survivors.
    survivors: u64,
    /// Bytes of survivors absent from `v(t+k)`.
    gone: u64,
    /// Bytes of survivors absent from `v(t-1)`.
    new: u64,
    /// Bytes of survivors both new and gone.
    new_gone: u64,
}

impl Counts {
    fn add(&mut self, other: Self) {
        self.survivors += other.survivors;
        self.gone += other.gone;
        self.new += other.new;
        self.new_gone += other.new_gone;
    }
}

/// One measured `(anchor, lag)` pair.
struct Row {
    lag: usize,
    /// 0-based index of the anchor version.
    anchor: usize,
    counts: Counts,
    /// Length of the anchor version in bytes.
    anchor_len: usize,
    /// Bytes of the whole anchor absent from `v(t-1)`, over the anchor's length.
    /// Independent of the lag; decides exclusion.
    new_fraction: f64,
}

impl Row {
    fn exclusion(&self) -> Option<String> {
        if self.new_fraction == 0.0 {
            Some("excluded: nothing new".to_string())
        } else if self.new_fraction > MAX_NEW_FRACTION {
            Some(format!(
                "excluded: {:.1}% new (rebuilt or compressed?)",
                self.new_fraction * 100.0
            ))
        } else {
            None
        }
    }

    /// New bytes this row measures (the survivors at lag `k > 1`), as a percentage of
    /// the anchor.
    #[allow(
        clippy::cast_precision_loss,
        reason = "statistics; precision loss past 2^53 is irrelevant"
    )]
    fn new_percent(&self) -> f64 {
        if self.anchor_len == 0 {
            0.0
        } else {
            self.counts.new as f64 * 100.0 / self.anchor_len as f64
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let h = args.get(1).ok_or(USAGE)?.parse::<usize>()?;
    let paths = args.get(2..).unwrap_or_default();
    if paths.len() < 3 {
        return Err(USAGE.into());
    }

    let finder = Extremum::new(h);
    let mut summaries = Vec::with_capacity(paths.len());
    let mut rows = Vec::new();

    // Sliding window: when v(j) loads it is the `v(t+k)` of every anchor `t = j - k`,
    // so only the last `MAX_LAG + 2` versions are ever needed at once.
    let mut window: Vec<Option<Version>> = Vec::with_capacity(paths.len());
    for (j, path) in paths.iter().enumerate() {
        eprintln!("chunking v{} {path}", j + 1);
        let version = load(path, &finder)?;
        summaries.push((version.path.clone(), version.file_len, version.chunks.len()));
        window.push(Some(version));

        for lag in 1..=MAX_LAG {
            let Some(anchor) = j.checked_sub(lag).filter(|&t| t >= 1) else {
                continue;
            };
            let get = |i: usize| window[i].as_ref().expect("inside the sliding window");
            let (counts, new_fraction) =
                measure(get(anchor - 1), get(anchor), get(anchor + lag - 1), get(j));
            rows.push(Row {
                lag,
                anchor,
                counts,
                anchor_len: get(anchor).file_len,
                new_fraction,
            });
        }

        if let Some(oldest) = j.checked_sub(MAX_LAG + 1) {
            window[oldest] = None;
        }
    }

    print_report(h, &summaries, &rows);
    Ok(())
}

/// Maps and chunks one file at tier 0.
fn load(path: &str, finder: &Extremum) -> Result<Version, Box<dyn Error>> {
    let file = File::open(path)?;

    // Mapping a zero-length file is not portable, and there is nothing to chunk anyway.
    if file.metadata()?.len() == 0 {
        return Ok(Version {
            path: path.to_string(),
            file_len: 0,
            chunks: Vec::new(),
            ids: HashSet::new(),
        });
    }

    // SAFETY: the file is opened read-only and is not modified or truncated while the map
    // is alive. This is a measurement tool run against corpus files nothing else writes to.
    let mmap = unsafe { Mmap::map(&file)? };
    if let Err(e) = mmap.advise(Advice::Sequential) {
        eprintln!("warning: madvise(SEQUENTIAL) failed: {e}");
    }

    let (chunks, _) = chunk_range(&mmap, (0..mmap.len()).into(), finder);
    let chunks: Vec<(ChunkId, usize)> = chunks.iter().map(|c| (c.id, c.len)).collect();
    let ids = chunks.iter().map(|&(id, _)| id).collect();

    Ok(Version {
        path: path.to_string(),
        file_len: mmap.len(),
        chunks,
        ids,
    })
}

/// Counts in `anchor`'s coordinates; see the module docs. `mid` is `v(t+k-1)`, which is
/// the anchor itself at lag 1. Also returns the anchor's whole-file new fraction.
#[allow(
    clippy::cast_precision_loss,
    reason = "statistics; precision loss past 2^53 is irrelevant"
)]
fn measure(prev: &Version, anchor: &Version, mid: &Version, next: &Version) -> (Counts, f64) {
    let mut counts = Counts::default();
    let mut anchor_new = 0u64;

    for &(id, len) in &anchor.chunks {
        let len = len as u64;
        let is_new = !prev.ids.contains(&id);
        if is_new {
            anchor_new += len;
        }
        if !mid.ids.contains(&id) {
            continue;
        }

        let is_gone = !next.ids.contains(&id);
        counts.survivors += len;
        if is_gone {
            counts.gone += len;
        }
        if is_new {
            counts.new += len;
            if is_gone {
                counts.new_gone += len;
            }
        }
    }

    let new_fraction = if anchor.file_len == 0 {
        0.0
    } else {
        anchor_new as f64 / anchor.file_len as f64
    };
    (counts, new_fraction)
}

/// `(base, conditional, ratio)`, each `None` when its denominator is zero.
#[allow(
    clippy::cast_precision_loss,
    reason = "statistics; precision loss past 2^53 is irrelevant"
)]
fn rates(c: Counts) -> (Option<f64>, Option<f64>, Option<f64>) {
    let div = |n: u64, d: u64| (d > 0).then(|| n as f64 / d as f64);
    let base = div(c.gone, c.survivors);
    let conditional = div(c.new_gone, c.new);
    let ratio = match (base, conditional) {
        (Some(b), Some(c)) if b > 0.0 => Some(c / b),
        _ => None,
    };
    (base, conditional, ratio)
}

/// Median of the values, `None` when there are none.
fn median(mut values: Vec<f64>) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let mid = values.len() / 2;
    Some(if values.len().is_multiple_of(2) {
        f64::midpoint(values[mid - 1], values[mid])
    } else {
        values[mid]
    })
}

/// The largest possible ratio for a base rate: `1 / base`.
fn ceiling(base: Option<f64>) -> Option<f64> {
    base.filter(|&b| b > 0.0).map(|b| 1.0 / b)
}

fn fmt_opt(value: Option<f64>, decimals: usize) -> String {
    value.map_or_else(|| "n/a".to_string(), |v| format!("{v:.decimals$}"))
}

fn print_report(h: usize, summaries: &[(String, usize, usize)], rows: &[Row]) {
    println!("h = {h}, {} versions, lags 1..={MAX_LAG}", summaries.len());
    println!();
    for (i, (path, len, chunks)) in summaries.iter().enumerate() {
        println!("- v{}: `{path}` ({len} bytes, {chunks} chunks)", i + 1);
    }
    println!();
    println!("| lag | anchor | new % | base | conditional | ratio | max | note |");
    println!("|---|---|---|---|---|---|---|---|");

    let max_lag = rows.iter().map(|r| r.lag).max().unwrap_or(0);
    for lag in 1..=max_lag {
        let mut pooled = Counts::default();
        let mut used = 0;
        let mut excluded = 0;
        let mut ratios = Vec::new();

        for row in rows.iter().filter(|r| r.lag == lag) {
            let (base, conditional, ratio) = rates(row.counts);
            let note = row.exclusion();
            if note.is_some() {
                excluded += 1;
            } else {
                pooled.add(row.counts);
                used += 1;
                ratios.extend(ratio);
            }
            println!(
                "| {lag} | v{} | {:.2} | {} | {} | {} | {} | {} |",
                row.anchor + 1,
                row.new_percent(),
                fmt_opt(base, 4),
                fmt_opt(conditional, 4),
                fmt_opt(ratio, 2),
                fmt_opt(ceiling(base), 2),
                note.unwrap_or_default(),
            );
        }

        let (base, conditional, ratio) = rates(pooled);
        println!(
            "| {lag} | **pooled** | | {} | {} | **{}** | {} | {used} used, {excluded} excluded |",
            fmt_opt(base, 4),
            fmt_opt(conditional, 4),
            fmt_opt(ratio, 2),
            fmt_opt(ceiling(base), 2),
        );
        let count = ratios.len();
        println!(
            "| {lag} | **median** | | | | **{}** | | of {count} anchor ratios |",
            fmt_opt(median(ratios), 2),
        );
    }
}
