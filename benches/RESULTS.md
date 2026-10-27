# Measurement results

## Step 2A.1 — chunk-size variance (Extremum)

**Decision rule, pinned 2026-10-27, before the sweep below was run:**
proceed if CV ≤ 0.6; schedule the Chonkers fine tier (Phase 8) early if 0.6 < CV ≤ 0.8;
stop and discuss the Gear contingency if CV > 0.8.

Measured with `cargo run --release --example chunk_file -- <path> <h>`. CV is the population
standard deviation of chunk lengths divided by their mean. Clamp rate is clamp cuts / all cuts.

| input | h | mean | expected 2h+1 | CV | clamp rate | note |
|---|---|---|---|---|---|---|
| 1 GiB random (`/dev/urandom`) | 2 KiB | 4,096 | 4,097 | 0.382 | 0 | 262,140 chunks; Linux x86-64, 2026-10-27 |
| 1 GiB random (`/dev/urandom`) | 8 KiB | 16,389 | 16,385 | 0.383 | 0 | 65,517 chunks |
| 1 GiB random (`/dev/urandom`) | 32 KiB | 65,680 | 65,537 | 0.387 | 0 | 16,348 chunks |
| 1 GiB random (`/dev/urandom`) | 128 KiB | 263,043 | 262,145 | 0.381 | 0 | 4,082 chunks |
| 1 GiB random (`/dev/urandom`) | 512 KiB | 1,026,522 | 1,048,577 | 0.388 | 0 | 1,046 chunks; mean −2.1% is sampling noise at n ≈ 1k |
| corpus raw files | — | — | — | — | — | pending: on the desktop (`D:\CombCorpus\<source>\raw\`) |

**Verdict on random data:** CV ≈ 0.38 and does not depend on scale, which is below the 0.6
threshold, so proceed. This beats the ~0.5 estimate in ALGORITHM.md §5a, and it compares with
plain Gear at 1.04–1.16 and Gear NC-3 at 0.24–0.59 (arXiv 2409.06066, μ = 8192).
