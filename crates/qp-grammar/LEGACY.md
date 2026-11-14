# qp-grammar — what came from the legacy viewport/LOD math (R2/T7)

Sources read (read-only): `../Services/quantex-charts-core/core/src/rendering/viewport.rs`,
`lod.rs`, `vertex.rs`, `hit_test.rs`; `../Docs/q-charts/03-PERFORMANCE-REQUIREMENTS.md` and
`analysis/core-layer-design.md` (LOD sections); `docs/LEGACY.md` §5. No code was copied; no
`.env` was opened.

## Taken

| Legacy idea | Where it lives now | Notes |
|-------------|-------------------|-------|
| X axis by **bar index**, not time (`index_to_x = margin_left + (idx − start + 0.5) · bar_width`): session gaps take no space | `XMap`, `scale::slot_x` | Same formula, but snapped with `floor` (R0 layout rule) so the candle column is a whole pixel; `from_bar` is an integer slot. |
| Inverse transforms `x_to_index`, `y_to_price` | `scale::slot_at`, `scale::value_at` | Used by the data → pixel → data property test (`tests/roundtrip.rs`), which the legacy code lacked (`docs/LEGACY.md` L14). |
| Price ↔ pixel `margin_top + (high − p) · h / span` | `scale::value_y` | Identical shape; R0's expression order is kept bit for bit. |
| 5 % vertical padding when fitting (`fit_bars`, `fit_price_to_visible`) | `compile::domain` | Fitted to the **visible** range (legacy `fit_price_to_visible`), per panel, over every drawn value (rules and bands included), not only bar highs/lows. |
| Visible range as `floor(start) .. ceil(end)` clipped to the data (`visible_range`) | `Frame::from/to` | Slots are integers here; the transient viewport is clamped to existing slots. |
| Aggregating candles to OHLC when zoomed out (`Aggregated` level) | `layer::prepare` (candle buckets) | First `o`, max `h`, min `l`, last `c` per bucket. |
| Hit-testing in O(1) by index | `slot_at` | Plus per-item hit boxes in the scene (`Item.hit`) for marks other than candles. |

## Not taken, and why

| Legacy | Why not |
|--------|---------|
| LOD **levels by visible bar count** (200 / 1000 / 5000 / 10000 thresholds; `Simplified`, `Line`, `Heatmap`) | A bar-count threshold ignores the frame width: 1000 bars are fine at 4K and unreadable on a phone. GRAMMAR §15 triggers on **pixels** (`slot < 1`) with bucket size `k = ceil(slots / plot width)`, plus the profile's `max_points`. Switching candles to a close line (`Line`) would also change *what* is drawn (a different mark) behind the agent's back; aggregation keeps the mark and its meaning. `Heatmap` had no implementation. |
| Fractional `start_idx`/`end_idx` (half-bar padding, smooth pan) | The persistent range and the viewport are bar slots (integers) in the protocol; sub-bar panning is an interaction concern (R4/R7). |
| `start_idx < 0` meaning "auto-fit" (`wasm/src/engine.rs`) | Ambiguous with a legitimately negative left pad (`docs/LEGACY.md` L12). Here "no range" is `None`. |
| Body width factors 0.8 / 0.4 by level | R0's rule is kept (70 % of the slot, odd above 2 px) so the R0 scene stays byte-identical; under LOD the bucket width is used. |
| Line LOD = close only | Lines keep first/min/max/last per bucket (a vertical stroke), so spikes stay visible; the item carries `<field>.min`/`<field>.max`. |
| Margins (right 70, bottom 30, top 10, left 5) duplicated in TS | Layout margins are R0's (left 8, right W − 72, top 34, bottom H − 24) and live in one place (`compile.rs`). |
| `f32` prices, `dpr` stored but unused, time-based "legacy compat" helpers | `f64` rule; device pixel ratio belongs to the shell; time-based X was replaced by slots. |
| Volume drawn in the bottom 20 % of the price chart | Volume is a `bar` layer in its own panel (or any panel) in the grammar. |
