| payload | bytes | JSON.parse µs | JSON.stringify µs | TextDecoder+JSON.parse µs |
|---|---:|---:|---:|---:|
| tick | 234 | 0.8 | 0.6 | 0.8 |
| rsi | 321991 | 487.0 | 997.9 | 506.6 |
| macd | 733728 | 1136.9 | 2229.2 | 1180.6 |
| append10k | 767220 | 2274.2 | 3170.3 | 2304.3 |
| doc | 769516 | 2301.6 | 3538.1 | 2965.7 |

V8 JSON number round trip on MACD values: 0 diffs of 29909
qcol append10k: 480040 bytes; Float64Array view creation µs: 0.021 (offset illustrative)
