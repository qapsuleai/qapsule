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
| `program.max_nodes` | int | count | 1.. | 200 | system | release only | no | Nodes per program at most (`E_TOO_MANY_NODES`). | RL1 |
| `program.max_window` | int | bars | 1..100000 | 2000 | system | release only | no | Largest `period`, `n` or `bars` in a program (`E_WINDOW_LIMIT`); at most `study.max_period`. | RL1 |
| `program.max_outputs` | int | count | 1.. | 4 | system | release only | no | Outputs per program at most (`E_TOO_LARGE`). | RL1 |
| `program.max_params` | int | count | 0.. | 16 | system | release only | no | Parameters per program at most (`E_TOO_LARGE`). | RL1 |
| `program.max_ir_bytes` | int | bytes | 256.. | 8192 | system | release only | no | Canonical IR size at most; bounds log growth (`E_TOO_LARGE`). | RL1 |
| `program.max_cost_per_bar` | int | unit/bar | 1.. | 666 | system | release only | no | Static cost per bar of one program at most (`E_COST_LIMIT`); ≈ RX4's 2 000 000 units for 3000 bars. | RL1 |
| `program.signal.max_horizon_bars` | int | bars | 1.. | 500 | system | release only | no | Largest `max_bars` of a signal outcome rule (`E_BAD_RULE`). | RL1 |
| `program.check.max_errors` | int | count | 1.. | 50 | system | release only | no | Errors returned by one check at most; the rest is `truncated`. | RL1 |
| `program.live.doc.max_instances` | int | count | 0.. | 32 | system, plan | admin | yes | Per document, per owner: program instances at most (`E_BUDGET`). | RL1 |
| `program.live.doc.max_cost_per_bar` | int | unit/bar | 0.. | 2000 | system, plan | admin | yes | Per document, per owner: sum of the live cost per bar of the owner's instances at most (`E_BUDGET`). | RL1 |
| `program.live.doc.max_log_bytes` | int | bytes | 0.. | 4194304 | system, plan | admin | yes | Per document, per owner: bytes of program changes in the log at most; bounds log growth by repeated edits (`E_BUDGET`). | RL1 |
| `program.try.max_bars` | int | bars | 1.. | 20000 | system, plan | admin | yes | Closed bars one try or replay run reads at most (the latest ones). | RL1 |
| `program.try.max_work` | int | unit | 1.. | 50000000 | system, plan | admin | yes | Work of one try or replay run at most; the run stops with `stopped: budget`. | RL1 |
| `program.try.max_items` | int | count | 1..100 | 10 | system | release only | no | Values, bars or records listed per output in a try summary. | RL1 |
| `program.work.record` | int | unit | 0.. | 16 | system | release only | no | Work charged per emitted record (event bar, signal record, state key). | RL1 |
| `program.work.open_record` | int | unit | 0.. | 1 | system | release only | no | Work charged per open signal record per bar. | RL1 |
| `program.work.read` | int | unit | 0.. | 1 | system | release only | no | Work charged per output cursor read. | RL1 |
| `program.text.max_source_chars` | int | chars | 1.. | 65536 | system | release only | no | Characters of a program text at most (`E_TOO_LARGE`); eight times `program.max_ir_bytes` for comments and spelling. | RL1 |
| `program.text.max_token_chars` | int | chars | 1.. | 1024 | system | release only | no | Characters of one token (name, number, string) of a program text at most (`E_TOO_LARGE`). | RL1 |
| `program.text.max_depth` | int | count | 1.. | 64 | system | release only | no | Expression nesting of a program text at most, parentheses included (`E_TOO_LARGE`). | RL1 |
| `program.max_ops_per_command` | int | count | 1..64 | 8 | system | release only | no | Program operations (`add_program`, `set_program_params`, `remove_program`) in one command at most (`E_TOO_LARGE`). | RL1 |
