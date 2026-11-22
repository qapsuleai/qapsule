üretilmiştir: cargo run -p qp-schema --example params_doc (elle düzenlemeyin)

# Parameters (generated)

Source: `qp_schema::params::system::REGISTRY`. Resolution order: object > user > plan > system (`docs/ENGINEERING.md` §6). Inventory: `docs/PARAMETERS.md`.

| Key | Kind | Unit | Range | Default | Scopes | Changed by | Entitlement | Description | Since |
|---|---|---|---|---|---|---|---|---|---|
| `scorecard.min_n` | int | count | 1.. | 20 | system | release only | no | Ledger sample size from which the scorecard sentence states counts plainly. | RG1 |
| `glyph.min_ledger_n` | int | count | 1.. | 20 | system | release only | no | Closed ledger rows a story needs at birth before its glyph shows a confidence fill. | RG1 |
| `glyph.freshness_half_bars` | float | bars | 0.. | 5.0 | system | release only | no | Bars after the last transition at which a glyph's freshness is one half. | RG1 |
| `story.theme.gap_bars` | int | bars | 0.. | 10 | system | release only | no | Join tolerance for facts of the same kind and direction. | RG1 |
| `story.theme.flow_confirm_bars` | int | bars | 0.. | 1 | system | release only | no | Window in which a flow fact confirms a price-meaning story. | RG1 |
| `outcome.barrier.c_dir` | float | ratio | 0.. | 0.3 | system | release only | no | Directional barrier constant: k = c_dir · √h, in ATR(fast) units. | RG1 |
| `outcome.barrier.c_neutral` | float | ratio | 0.. | 0.65 | system | release only | no | Neutral barrier constant: k_n = c_neutral · √h, in ATR(slow) units. | RG1 |
| `outcome.atr_fast` | int | bars | 1.. | 14 | system | release only | no | ATR period for directional stories. | RG1 |
| `outcome.atr_slow` | int | bars | 1.. | 100 | system | release only | no | ATR period for neutral stories (normal volatility). | RG1 |
| `outcome.intraday_max_step_ms` | int | ms | 1.. | 43200000 | system | release only | no | Bars at least this far apart are not intraday and are judged as daily. | RG1 |
| `watch.max_per_person` | int | count | 0.. | 32 | system, plan | admin | yes | Active watches per person at most. | RG1 |
| `dispute.max_open_per_conn` | int | count | 1.. | 16 | system | release only | no | Open disputes per connection at most. | RG1 |
| `dispute.max_per_conn` | int | count | 1.. | 256 | system | release only | no | Disputes accepted per connection over its life at most. | RG1 |
| `dispute.max_digests` | int | count | 1.. | 4 | system | release only | no | Different digests per (connection, claim, t) at most. | RG1 |
| `gesture.story_zoom_min_bars` | int | bars | 1.. | 20 | system | release only | no | Fewest bars a tap on a story cluster shows. | RG1 |
| `depth.chart.lookback_bars` | int | bars | 0.. | 30 | system | release only | no | Bars shown before the fact bar in the chart depth view. | RG1 |
