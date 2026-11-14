# T6b plain JSON (no float_roundtrip) — platform: native

| case | codec | bytes | enc µs | dec µs | note |
|---|---|---:|---:|---:|---|
| warmup | json (no float_roundtrip) | 321991 | 205.38 | 377.25 | bit diffs 448 |
| tick set_last_bar | json (no float_roundtrip) | 234 | 0.23 | 0.87 | bit diffs 0 |
| tick append_bars(1) | json (no float_roundtrip) | 236 | 0.23 | 0.90 | bit diffs 0 |
| values RSI 10k | json (no float_roundtrip) | 321991 | 204.53 | 372.52 | bit diffs 448 |
| values MACD(3) 10k | json (no float_roundtrip) | 733728 | 866.67 | 863.88 | bit diffs 3337 |
| append_bars 10k | json (no float_roundtrip) | 767220 | 852.17 | 2338.50 | bit diffs 0 |
| snapshot Document 10k | json (no float_roundtrip) | 769516 | 852.96 | 2367.71 | bit diffs 0 |

## f64 exactness without float_roundtrip (100001 values)

append_bars bit diffs: 23132 ; values bit diffs: 23132 ; tricky-only: 5/5
tricky failures: ["1.7976931348623155e+308 -> 1.7976931348623157e308", "123456789.12345679 -> 1.234567891234568e8", "9007199254740991.0 -> 9.00719925474099e15", "3.4028234663852886e+38 -> 3.402823466385289e38", "0.9999999999999999 -> 1e0"]
