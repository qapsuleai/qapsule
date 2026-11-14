# RP1 / E1 — Shared sub-computations inside a sweep

Branch `rp1/e1-share` from `caab6e3` (RP1/T1 baseline). Only `crates/qp-compute/` and this
file changed. Machine: Apple M4 Max (16 threads), shared with other agents' benchmarks;
load average 3–6 during all runs (stated per table).

## Hypotheses (written before implementing)

Baseline profile A on this machine (load ~5, 7 reps, median ms): sma 14.8, ema 8.2,
rsi 25.2, **macd 42.2**, **bollinger 45.9**.

Cost model per variant, 10k bars: one EMA step is a ~7-cycle dependent chain
(`x*a + prev*(1-a)`); Bollinger's step has a `sqrt`, two divisions and a rolling M2;
RSI has four divisions per bar (two smoothing, two in `rsi_from_avg`).

- **H1 MACD (fast, slow) sharing.** For a run of consecutive variants with the same
  `(fast, slow)`, compute the macd line once (both EMAs stepped together, exactly the
  `Macd::commit1` operations) and keep it in column 0 of the lane buffers; per variant
  only the signal EMA over `macd[slow-1..]` and `histogram = macd - signal` run, several
  signals stepped as lanes. Work per variant drops from 3 EMAs to ~1 EMA + 1/10 of 2 EMAs,
  and 1/3 of the column writes disappear (macd column copied once per run per lane, not
  rewritten per variant). Expected: 42 → ~15–18 ms (2.3–2.8×).
- **H2 EMA dedupe across (fast, slow) pairs.** On top of H1, cache EMA columns by
  period: the bench grid has 100 pairs but only 10 fast + 28 slow distinct periods
  (200 → 38 EMA passes). Expected: a further 1–3 ms; may not pay for the extra column
  pass (macd = f - s as a separate loop) and cache memory.
- **H3 Bollinger per-period sharing.** For a run with the same period, step the
  middle/std state once (`Bollinger::step1` split into `step_std` + `bands`, same ops),
  keep `middle` in column 0, keep `std` in a scratch column; per `k` run a
  dependency-free column pass `band = k*std; upper = m+band; lower = m-band;
  bandwidth = m==0 ? 0 : (2*band)/m`. Expected: 46 → ~8–12 ms (4–5×): the shared state is
  1/10 of the work and the tail is throughput-bound.
- **H4 RSI shared gain/loss split.** Precompute `(gain, loss)` of `x - prev` once per
  sweep and feed every period's Wilder smoothing from it. The split is cheap and off the
  critical path; RSI is division-bound. Expected: small, 0–10%.
- **H5 RSI tail as a column pass** (`rsi_from_avg` moved out of the chain into a pass
  over `avg_gain/avg_loss`): the chains keep 2 divisions per bar, the other 2 run as a
  dependency-free pass. Expected: 10–25%, but it needs two extra columns written and read
  per variant.
- Grouping must be general: any list, any order. Runs of consecutive equal keys use the
  shared path; stretches of singletons fall back to the R1 lane kernels (same visit
  order, `visit` index = input position). Lists sorted by the shared key benefit; a
  shuffled list runs at baseline speed (no regression by construction).
- SMA/EMA: bench grid has 1,000 distinct periods; nothing exact to share (prefix sums
  would change the rounding). Not attempted.

## What was built

New private module `crates/qp-compute/src/sweep/share.rs` (child of `sweep.rs`):

- `segments(keys)`: splits the parameter list into maximal runs of **consecutive** equal
  shared keys (length ≥ 2 → shared path) and the plain stretches between them, in input
  order. Plain stretches go through the unchanged R1 `sweep_kernel` (now taking a `base`
  index offset). `visit` is still called in input order with the input index; nothing is
  reordered, so `sweep()` (which ignores the index) stays correct. Unsorted lists simply
  have more/longer plain stretches.
- `macd`: per shared run the macd line is `ema(fast) - ema(slow)`; EMA columns of every
  fast/slow period used by any shared run are computed once up front (`ema_cache`,
  4 EMAs interleaved, capped at 16 MB; above the cap each run computes its own line,
  `macd_line`, covered by a unit test with cap 0). The line is copied once per run into
  the lane buffers' column 0; signal EMAs run 2 lanes (`signal_lanes`), first through the
  regular `peek1/commit1` until every lane is past its seed bar, then the steady
  expression `Ema::steady_next` with lane states in registers.
- `bollinger`: per shared run `Bollinger::step_std` gives `(middle, std)` once (`middle`
  written straight into column 0), then each `k` runs a column pass
  `Bollinger::bands(k, middle, std)` writing upper/lower/bandwidth.
- `rsi` (all lists, no grouping needed): split computed once
  (`rsi_split = rsi::split`), each period stepped by `Rsi::step_split` (fused peek+commit,
  same operations) through its seed bar, then the steady Wilder expression `Rsi::smooth`
  with lane states in registers, 8 lanes, in blocks of 128 bars into small stack buffers;
  `rsi_from_avg_pass` runs per lane per block (out of the dependency chains) and each lane
  writes one sequential stream.
- Study additions (all `pub(crate)`, single-run code paths unchanged except
  `rsi_from_avg`, see below): `Ema::steady`, `Ema::steady_next`; `Rsi::step_split`,
  `Rsi::smooth`, `Rsi::steady`, `rsi_from_avg_pass`; `Bollinger::std_of`, `bands`,
  `peek_std`, `step_std`, `step_std_slow` (copies of the corresponding parts of
  `value`/`peek1`/`step1`/`commit1`, same operations in the same order).

Bit-identity argument: every shared value is produced by the very expression the single
run evaluates, with the same inputs, once; floating point is deterministic, so computing
it once and reading it N times is identical to computing it N times. No reassociation,
no FMA, no reciprocal swaps were introduced (the only new expression forms are
`steady_next`/`smooth`, which are the single-run expressions with the state passed in).

## Measurements

### Development steps (native aarch64, profile A, 11 reps, median ms; not interleaved,
same session, load 3–5)

| Step | rsi | macd | bollinger | Note |
|---|---|---|---|---|
| baseline `caab6e3` | 24.4 | 41.1 | 44.9 | |
| H1+H3 first version (signal lanes 4, generic `peek1/commit1` in lanes) | 23.0 | 20.3 | 8.6 | |
| H1 signal lanes 1 / 2 / 4 / 8 (generic step) | | 17.7 / 22.6 / 20.1 / 56.9 | | |
| H1 + steady phase in registers, lanes 1 / 2 / 4 / 8 | | 15.3 / 8.9 / 9.9 / 46.1 | | lanes 2 kept |
| H1 macd line also steady-phase | | 9.1 | | line was 1.55 ms of it (measured by computing it 10×) |
| H1 blocked signal loop (256-bar stack blocks), lanes 1/2/4/8 | | 16.3 / 10.8 / 10.1 / 9.0 | | fixes 8 lanes but not faster than 2 unblocked → **alma** |
| H2 EMA cache | | 7.7 | | −1.05 ms (−12%) → **al** |
| H4 split shared + fused step_split, lanes 4 / 8 | 18.5 / 14.9 | | | baseline at 8 lanes: 24.4 |
| H4 + steady phase in registers (8 lanes) | 13.8 | | | |
| H5 unblocked (2 scratch columns), lanes 4 / 8 | 13.5 / 62.8 | | | 16 write streams thrash |
| H5 blocked (128-bar stack blocks), lanes 4 / 8 | 12.9 / 9.1 | | | → **al** |
| H3b: step 2/4/8 Bollinger states of consecutive runs as lanes | | | G=1 9.2, 2 8.7, 4 9.7, 8 13.4 | no gain → **alma** |

### Final: baseline `caab6e3` vs `rp1/e1-share`, interleaved back-to-back, 3 alternations,
11 reps each; median of the 3 run-medians (min..max of run-medians). Digest column:
"same" = every digest printed by both binaries identical (profile A per-study digests
d9a393c2f397ae9e, f9be271e91103539, 9515526d5288453b, d5a6c407256992e6,
7e683c082fb95dc9 and all B/C/D lines).

**Native aarch64** (load 3.9–6.1)

| row | base ms | new ms | speedup | digest |
|---|---|---|---|---|
| A sma | 14.42 (14.38..14.88) | 14.38 (14.24..14.50) | 1.00× | same |
| A ema | 9.17 (8.86..9.67) | 10.20 (9.19..11.07) | 0.90× * | same |
| A rsi | 24.37 (24.16..24.37) | **9.13** (9.08..9.20) | **2.67×** | same |
| A macd | 40.87 (40.84..40.88) | **7.63** (7.62..8.09) | **5.36×** | same |
| A bollinger | 44.96 (44.47..45.68) | **8.91** (8.76..8.94) | **5.05×** | same |
| B sma | 14.17 | 13.89 | 1.02× | same |
| B ema | 23.85 | 24.48 | 0.97× | same |
| B rsi | 55.39 | **41.60** | **1.33×** | same |
| B macd | 43.57 | 43.16 | 1.01× | same |
| B bollinger | 42.41 | 42.20 | 1.00× | same |
| C ema>rsi | 99.60 | 99.41 | 1.00× | same |
| C rsi>bollinger | 172.88 | 170.56 | 1.01× | same |
| C sma>ema>rsi | 163.16 | 162.75 | 1.00× | same |
| D tick (µs) | 1.79 | 1.71 | 1.05× | same |
| D new bar (µs) | 2.62 | 2.58 | 1.02× | same |
| D attach | 26.37 | 26.73 | 0.99× | same |

\* A ema: the EMA sweep code is byte-for-byte the same function as at baseline (same
size and instruction mix; only its address/alignment moved). Its per-rep times are
bimodal (min 7.6–10, median up to 13 in some runs, also seen on intermediate commits
26113aa / e60d3bc and not on others). Built for diagnosis with
`RUSTFLAGS="-C llvm-args=-align-loops=64"` for both base and new, A ema is equal
(base 9.05–9.55, new 8.9–10.3 median). So it is a code-layout effect, not a cost of this
change; it should be expected to move again with any other change.

**wasm32 (Node 24.3, V8 13.6)** (load 3.0–3.9)

| row | base ms | new ms | speedup | digest |
|---|---|---|---|---|
| A sma | 15.70 | 15.88 | 0.99× | same |
| A ema | 26.05 | 26.15 | 1.00× | same |
| A rsi | 55.89 (55.80..56.86) | **18.56** (18.24..18.59) | **3.01×** | same |
| A macd | 75.56 (75.29..75.66) | **10.24** (9.89..10.88) | **7.38×** | same |
| A bollinger | 69.08 (69.07..69.29) | **10.95** (10.86..11.18) | **6.31×** | same |
| B sma / ema | 15.21 / 17.08 | 15.19 / 17.17 | 1.00× / 0.99× | same |
| B rsi | 60.29 | **46.70** | **1.29×** | same |
| B macd / bollinger | 67.64 / 64.80 | 68.85 / 64.61 | 0.98× / 1.00× | same |
| C ema>rsi | 189.21 | 193.18 | 0.98× | same |
| C rsi>bollinger | 335.09 | 337.04 | 0.99× | same |
| C sma>ema>rsi | 377.32 | 385.68 | 0.98× | same |
| D tick / new bar (µs) | 4.21 / 5.08 | 4.25 / 5.12 | 0.99× / 0.99× | same |
| D attach | 50.55 | 51.15 | 0.99× | same |

**x86_64 under Rosetta 2 (informational)** (load 2.7–3.3)

| row | base ms | new ms | speedup | digest |
|---|---|---|---|---|
| A sma / ema | 15.11 / 20.77 | 15.18 / 20.95 | 1.00× / 0.99× | same |
| A rsi | 27.24 | **14.78** | **1.84×** | same |
| A macd | 69.92 | **9.01** | **7.76×** | same |
| A bollinger | 63.48 | **10.53** | **6.03×** | same |
| B sma / ema | 16.02 / 36.61 | 15.77 / 36.29 | 1.02× / 1.01× | same |
| B rsi | 38.49 | 47.34 | **0.81×** | same |
| B macd / bollinger | 66.25 / 60.94 | 66.33 / 60.95 | 1.00× / 1.00× | same |
| C ema>rsi | 148.89 | 156.98 | 0.95× | same |
| C rsi>bollinger | 243.38 | 258.03 | 0.94× | same |
| C sma>ema>rsi | 237.15 | 241.35 | 0.98× | same |
| D tick / new bar / attach | 2.75 / 3.92 / 39.15 | 2.83 / 3.92 / 39.08 | 0.97× / 1.00× / 1.00× | same |

Cross-target digest (`examples/digest`): `7b1e99d40889f83c` on aarch64, x86_64 and
wasm32 (unchanged).

### Code-generation side effects found and fixed (all measured, all same digests)

Adding sweep-only callers to small single-run functions changed LLVM's inlining of the
single-run paths, which showed up in profiles B–D even though their source was
unchanged:

| Change | Effect | Fix kept |
|---|---|---|
| `Bollinger::step1` rewritten as `step_std().map(bands)` | wasm B bollinger 0.79× | `peek1`/`value`/`step1` restored verbatim; sweep uses separate copies |
| `step_std` calling `commit1` (third call site) | native: `commit1` outlined from `Streaming::push`, D attach 0.94–0.95× | `step_std_slow`: out-of-line copy of `peek_std + commit1` body; symbol diff vs baseline now only adds new functions |
| `rsi_from_avg` branch form shared with the sweep pass | wasm C ema>rsi 0.87× (also with `#[inline(always)]` or a duplicated pass fn) | single-run `rsi_from_avg` in select form (same value; wasm C 0.98×), sweep pass uses a branch-form copy `rsi_from_avg_pass` (wasm A rsi 23.2 → 18.4 ms vs the select form) |
| macd `line` buffer allocated per call | B macd (one variant per call, 2,500 calls) ~+5% on wasm | allocated only when a shared run exists |

Remaining on x86-Rosetta: B rsi 0.81× (one-variant lists through the split path; native
1.33×, wasm 1.29×) and C 0.94–0.95× (select-form `rsi_from_avg` in the single run). Not
fixed: Rosetta is not the CI runner; real x86_64 (GitHub Linux) must be measured before
deciding. If it reproduces there, two cheap switches exist: route RSI lists with fewer
than ~4 variants to the R1 kernel, and choose the `rsi_from_avg` form per target.

### Bit-equality evidence

- All profile digests above identical on aarch64, wasm32, x86_64; cross-target digest
  unchanged.
- Existing tests unchanged and green (`tests/sweep.rs` lane counts 1/2/3/4/8 etc.).
- New `tests/share.rs`: macd, bollinger, rsi sweeps vs single runs (`qp_compute::run`)
  bit for bit, every lane count (None/1/2/3/4/8), checking visit order and `sweep()`
  output order, on: benchmark-grid slices, a deterministically **shuffled** grid slice,
  **mixed** lists (runs of length 2–11 interleaved with singletons, **duplicate**
  parameter sets, `fast = 1`, `period = 1`, defaults filled in), a flat stretch and a
  zero-price stretch (Bollinger exact-flat and `middle == 0` branches; RSI no-move and
  no-loss), and **series shorter than the periods** (0, 1, 2, 5, 12, 15, 27, 31 bars).
- Unit tests in `share.rs`: `segments` splitting; macd with EMA cache budget 0 (the
  uncached line path) vs single runs.
- Mutation checks (temporary, reverted): perturbing `k` by one ulp in the Bollinger pass,
  swapping the histogram subtraction, and flipping a sign in `step_std_slow`'s M2 removal
  each make `tests/share.rs` fail.

## Decisions

| Idea | Decision | Reason |
|---|---|---|
| H1 macd line shared per (fast, slow) run, signal lanes with steady phase | **al** | 5.4× native, 7.4× wasm, 7.8× x86; bit-identical |
| H2 EMA dedupe cache across pairs | **al** | −12% on top of H1; 3 MB for profile A, capped at 16 MB with tested fallback |
| H3 Bollinger middle/std shared per period, k pass | **al** | 5.0× native, 6.3× wasm, 6.0× x86 |
| H3b Bollinger states of several runs as lanes | **alma** | no gain (G=2 equal, G≥4 slower) |
| H4 RSI split shared + fused step + steady registers, 8 lanes | **al** | 24.4 → 13.8 ms alone |
| H5 RSI `rsi_from_avg` as blocked pass | **al** | 13.8 → 9.1 ms native; 2.7× total native, 3.0× wasm; also B rsi 1.33× native / 1.29× wasm |
| Blocked MACD signal loop | **alma** | equal to 2 unblocked lanes; kept the simpler form |
| Out-of-order (sorted) grouping with a column cache for unsorted lists | **sonra** | today only consecutive runs share; a list sorted by key gets everything, a shuffled one runs at R1 speed. Sorting internally needs either out-of-order `visit` (API change; `sweep()` relies on order) or caching shared columns (memory ∝ distinct keys × bars). Worth it only if agents send unsorted grids |
| SMA/EMA sharing | **alma** | nothing exact to share with distinct periods |

Invariant-breaking: none needed or tried; every gain here is bit-identical.

## What is now the slowest part, and why (studies touched)

- **MACD (7.6 ms native):** the per-variant signal EMA, a 7-cycle `mul → add` chain per
  bar (2 lanes overlap two chains), plus writing two 80 KB columns per variant (160 MB of
  stores for the grid). Without the stores the 4/8-lane compute was ~6 ms, so the column
  writes are now ~1/3 of the time. The shared part (38 EMA columns + 100 line passes) is
  ~0.5 ms.
- **Bollinger (8.9 ms):** ~3.5 ms in the 100 shared state passes (the rolling-sum/M2
  chain plus branches for flat-window detection, `sqrt`, a division) and ~4.6 ms in the
  1,000 `k` passes, which are bound by one division per bar (`bandwidth`) and three
  column stores per bar (240 MB of stores). Laning the shared states did not help.
- **RSI (9.1 ms):** division throughput: four divisions per variant-bar (two in the
  smoothing chains, two in `rsi_from_avg`) = 40 M divisions; ~1 division per 1.5 cycles.
  None of them can be replaced by a reciprocal multiply without changing bits.
- Across all three, the next limit is **materializing every output column** for every
  variant (the `SweepColumns` visitor contract): ~150–250 MB of stores per 1,000-variant
  sweep. An API that lets the caller ask for only the outputs/bars it needs (e.g. only
  `histogram`, only the last N bars, or a reduction) would remove most of the remaining
  time; that is an API/schema change and is left as a proposal.
- Observation useful for SIMD/lane experiments: on M4, more than ~12 concurrent
  interleaved write streams (e.g. 8 lanes × 2 output columns) collapse throughput by
  5–7× (MACD 8 lanes 46–57 ms vs 9 ms at 2; RSI 16 streams 63 ms vs 9 ms blocked).
  Blocking through small stack buffers and writing each lane's block sequentially fixes
  it.

## Conflicts with other experiments

- `sweep.rs`: `sweep_kernel` gained a `base: usize` index parameter; the `macd`,
  `bollinger` and `rsi` arms of `sweep_each_lanes` now dispatch to `sweep/share.rs`
  (the R1 lane kernels are still used for sma/ema and for macd/bollinger plain
  stretches; the `Kernel for Rsi` impl is now unused by the sweep). `RSI_LANES` no longer
  affects RSI sweeps (`RSI_SPLIT_LANES = 8` in `share.rs`); the `lanes` argument still
  overrides all paths.
- Studies: added `pub(crate)` helpers in `ema.rs`, `rsi.rs` (`split` became
  `pub(crate)`; `rsi_from_avg` now select form; new `rsi_from_avg_pass`), `bollinger.rs`
  (copies only; single-run functions verbatim). Any SIMD/lane experiment that rewrites
  `step_lanes`/`Kernel::step` for macd/bollinger/rsi will mostly not affect profile A
  anymore on this branch (those grids take the shared path); it still matters for profile
  B and for unsorted lists. A threads experiment can parallelize over shared runs
  (`segments`) or over RSI lane chunks without changing results.
- The steady-phase-in-registers trick (EMA signal, RSI) overlaps with lane/SIMD
  experiments; measure them on top of this branch rather than on the baseline.
