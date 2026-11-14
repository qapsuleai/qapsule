# T6b plain JSON (no float_roundtrip) — platform: wasm32-wasip1 (node)

| case | codec | bytes | enc µs | dec µs | note |
|---|---|---:|---:|---:|---|
| warmup | json (no float_roundtrip) | 321991 | 335.71 | 493.33 | bit diffs 448 |
| tick set_last_bar | json (no float_roundtrip) | 234 | 0.39 | 1.49 | bit diffs 0 |
| tick append_bars(1) | json (no float_roundtrip) | 236 | 0.39 | 1.20 | bit diffs 0 |
| values RSI 10k | json (no float_roundtrip) | 321991 | 341.81 | 482.75 | bit diffs 448 |
| values MACD(3) 10k | json (no float_roundtrip) | 733728 | 1191.92 | 1097.71 | bit diffs 3337 |
| append_bars 10k | json (no float_roundtrip) | 767220 | 1632.12 | 3167.62 | bit diffs 0 |
| snapshot Document 10k | json (no float_roundtrip) | 769516 | 1647.33 | 3176.08 | bit diffs 0 |

## f64 exactness without float_roundtrip (100001 values)

append_bars bit diffs: 23132 ; values bit diffs: 23132 ; tricky-only: 5/5
tricky failures: ["1.7976931348623155e+308 -> 1.7976931348623157e308", "123456789.12345679 -> 1.234567891234568e8", "9007199254740991.0 -> 9.00719925474099e15", "3.4028234663852886e+38 -> 3.402823466385289e38", "0.9999999999999999 -> 1e0"]
