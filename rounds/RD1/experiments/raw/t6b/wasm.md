# T6b encoding bench — platform: wasm32-wasip1 (node)
corpus: 10000 bars; RSI nulls 14; ticks per second 60

## Sizes and times (median; per message)

| case | codec | bytes | enc µs | dec µs | gzip-6 B | brotli-6 B | note |
|---|---|---:|---:|---:|---:|---:|---|
| tick set_last_bar | json (float_roundtrip) | 234 | 0.39 | 1.11 | - | - |  |
| tick set_last_bar | cbor ciborium | 183 | 0.43 | 2.02 | - | - |  |
| tick set_last_bar | cbor minicbor-serde | 183 | 0.24 | 1.18 | - | - |  |
| tick set_last_bar | msgpack rmp (struct-map) | 183 | 0.32 | 0.84 | - | - |  |
| tick set_last_bar | msgpack rmp (struct-array) | 115 | 0.19 | - | - | - | decode error: invalid type: sequence, expected a string |
| tick set_last_bar | postcard (same types) | 100 | 0.12 | - | - | - | decode error: This is a feature that PostCard will never implement |
| tick set_last_bar | postcard (mirror types) | 78 | 0.16 | 0.11 | - | - |  |
| tick set_last_bar | json SoA bars | 234 | 0.43 | 1.12 | - | - |  |
| tick set_last_bar | qcol (LE f64 blocks) | 83 | 0.08 | 0.09 | - | - |  |
| tick append_bars(1) | json (float_roundtrip) | 236 | 0.39 | 1.16 | - | - |  |
| tick append_bars(1) | cbor ciborium | 184 | 0.45 | 2.10 | - | - |  |
| tick append_bars(1) | cbor minicbor-serde | 184 | 0.24 | 1.23 | - | - |  |
| tick append_bars(1) | msgpack rmp (struct-map) | 184 | 0.34 | 0.88 | - | - |  |
| tick append_bars(1) | msgpack rmp (struct-array) | 115 | 0.20 | - | - | - | decode error: invalid type: sequence, expected a string |
| tick append_bars(1) | postcard (same types) | 100 | 0.12 | - | - | - | decode error: This is a feature that PostCard will never implement |
| tick append_bars(1) | postcard (mirror types) | 79 | 0.17 | 0.12 | - | - |  |
| tick append_bars(1) | json SoA bars | 246 | 0.52 | 1.45 | - | - |  |
| tick append_bars(1) | qcol (LE f64 blocks) | 88 | 0.10 | 0.17 | - | - |  |
| values RSI 10k | json (float_roundtrip) | 321991 | 333 | 550 | - | - |  |
| values RSI 10k | cbor ciborium | 179986 | 156 | 766 | - | - |  |
| values RSI 10k | cbor minicbor-serde | 179986 | 81 | 361 | - | - |  |
| values RSI 10k | msgpack rmp (struct-map) | 179986 | 123 | 254 | - | - |  |
| values RSI 10k | msgpack rmp (struct-array) | 179931 | 126 | 269 | - | - |  |
| values RSI 10k | postcard (same types) | 154643 | 91 | - | - | - | decode error: This is a feature that PostCard will never implement |
| values RSI 10k | postcard (mirror types) | 154631 | 89 | 46 | - | - |  |
| values RSI 10k | json SoA bars | 321991 | 333 | 544 | - | - |  |
| values RSI 10k | qcol (LE f64 blocks) | 161296 | 22 | 9.76 | - | - |  |
| values MACD(3) 10k | json (float_roundtrip) | 733728 | 1181 | 1392 | - | - |  |
| values MACD(3) 10k | cbor ciborium | 359388 | 323 | 1554 | - | - |  |
| values MACD(3) 10k | cbor minicbor-serde | 359388 | 124 | 744 | - | - |  |
| values MACD(3) 10k | msgpack rmp (struct-map) | 359388 | 241 | 528 | - | - |  |
| values MACD(3) 10k | msgpack rmp (struct-array) | 359333 | 245 | 522 | - | - |  |
| values MACD(3) 10k | postcard (same types) | 334043 | 110 | - | - | - | decode error: This is a feature that PostCard will never implement |
| values MACD(3) 10k | postcard (mirror types) | 334031 | 121 | 63 | - | - |  |
| values MACD(3) 10k | json SoA bars | 733728 | 1176 | 1404 | - | - |  |
| values MACD(3) 10k | qcol (LE f64 blocks) | 323816 | 51 | 25 | - | - |  |
| append_bars 10k | json (float_roundtrip) | 767220 | 1682 | 3037 | - | - |  |
| append_bars 10k | cbor ciborium | 659189 | 1040 | 5781 | - | - |  |
| append_bars 10k | cbor minicbor-serde | 670119 | 271 | 2973 | - | - |  |
| append_bars 10k | msgpack rmp (struct-map) | 670119 | 671 | 1785 | - | - |  |
| append_bars 10k | msgpack rmp (struct-array) | 550062 | 317 | - | - | - | decode error: invalid type: sequence, expected a string |
| append_bars 10k | postcard (same types) | 464772 | 116 | - | - | - | decode error: This is a feature that PostCard will never implement |
| append_bars 10k | postcard (mirror types) | 464751 | 127 | 67 | - | - |  |
| append_bars 10k | json SoA bars | 507256 | 1187 | 1275 | - | - |  |
| append_bars 10k | qcol (LE f64 blocks) | 480040 | 48 | 33 | - | - |  |
| snapshot Document 10k | json (float_roundtrip) | 769516 | 1674 | 3054 | - | - |  |
| snapshot Document 10k | cbor ciborium | 660958 | 1036 | 5789 | - | - |  |
| snapshot Document 10k | cbor minicbor-serde | 671948 | 266 | 2994 | - | - |  |
| snapshot Document 10k | msgpack rmp (struct-map) | 671948 | 681 | 1810 | - | - |  |
| snapshot Document 10k | msgpack rmp (struct-array) | 551059 | - | - | - | - | decode error: invalid type: sequence, expected a string |
| snapshot Document 10k | postcard (same types) | 465636 | - | - | - | - | decode error: This is a feature that PostCard will never implement |
| snapshot Document 10k | postcard (mirror types) | - | - | - | - | - | n/a (no Document mirror) |
| snapshot Document 10k | json SoA bars | 509577 | 1192 | 1054 | - | - |  |
| snapshot Document 10k | qcol (LE f64 blocks) | 482480 | 54 | 44 | - | - |  |

## Live tick stream: 60 messages/second

| stream | codec | avg B/msg | B/s raw | ws-deflate B/msg (context takeover) | B/s ws-deflate | enc+dec µs/s (CPU) |
|---|---|---:|---:|---:|---:|---:|
| set_last_bar | json (float_roundtrip) | 233.8 | 14030 | NaN | NaN | 91.8 |
| set_last_bar | cbor ciborium | 182.8 | 10970 | NaN | NaN | 152.8 |
| set_last_bar | cbor minicbor-serde | 183.0 | 10980 | NaN | NaN | 87.2 |
| set_last_bar | msgpack rmp (struct-map) | 183.0 | 10980 | NaN | NaN | 70.9 |
| set_last_bar | msgpack rmp (struct-array) | decode fails | | | | |
| set_last_bar | postcard (same types) | decode fails | | | | |
| set_last_bar | postcard (mirror types) | 78.0 | 4680 | NaN | NaN | 16.9 |
| set_last_bar | json SoA bars | 233.8 | 14030 | NaN | NaN | 93.7 |
| set_last_bar | qcol (LE f64 blocks) | 83.0 | 4980 | NaN | NaN | 10.3 |
| append_bars(1) | json (float_roundtrip) | 236.3 | 14181 | NaN | NaN | 94.1 |
| append_bars(1) | cbor ciborium | 182.7 | 10964 | NaN | NaN | 153.7 |
| append_bars(1) | cbor minicbor-serde | 184.0 | 11040 | NaN | NaN | 89.4 |
| append_bars(1) | msgpack rmp (struct-map) | 184.0 | 11040 | NaN | NaN | 73.1 |
| append_bars(1) | msgpack rmp (struct-array) | decode fails | | | | |
| append_bars(1) | postcard (same types) | decode fails | | | | |
| append_bars(1) | postcard (mirror types) | 79.0 | 4740 | NaN | NaN | 18.1 |
| append_bars(1) | json SoA bars | 246.3 | 14781 | NaN | NaN | 118.3 |
| append_bars(1) | qcol (LE f64 blocks) | 88.0 | 5280 | NaN | NaN | 14.9 |

## f64 exactness (100001 values: 45 tricky + random finite bit patterns + price-like)

| codec | append_bars bit diffs | values bit diffs | tricky-only diffs |
|---|---:|---:|---:|
| json (float_roundtrip) | 0 | 0 | 0/0 |
| cbor ciborium | 0 | 0 | 0/0 |
| cbor minicbor-serde | 0 | 0 | 0/0 |
| msgpack rmp (struct-map) | 0 | 0 | 0/0 |
| msgpack rmp (struct-array) | err: invalid type: sequence, expected a string | 0 | err: invalid type: sequence, expected a string/0 |
| postcard (same types) | err: This is a feature that PostCard will never impleme | err: This is a feature that PostCard will never impleme | err: This is a feature that PostCard will never impleme/err: This is a feature that PostCard will never impleme |
| postcard (mirror types) | 0 | 0 | 0/0 |
| json SoA bars | 0 | 0 | 0/0 |
| qcol (LE f64 blocks) | 0 | 0 | 0/0 |

## Schema evolution probe (Bar has deny_unknown_fields)

| codec | extra field -> Bar (deny) | extra field -> Bar w/o deny | missing field -> Bar |
|---|---|---|---|
| json | error: unknown field `x`, expected one of `t`, `o`, `h`, `l`, `c`,  | OK BarLoose { t: 1, o: 2.0, h: 3.0, l: 1.0, c: 2.5, v: 10.0  | error: missing field `v` at line 1 column 39 |
| cbor ciborium | error: Semantic(None, "unknown field `x`, expected one of `t`, `o`, | OK BarLoose { t: 1, o: 2.0, h: 3.0, l: 1.0, c: 2.5, v: 10.0  | error: Semantic(None, "missing field `v`") |
| cbor minicbor-serde | error: decode error: unknown field `x`, expected one of `t`, `o`, ` | OK BarLoose { t: 1, o: 2.0, h: 3.0, l: 1.0, c: 2.5, v: 10.0  | error: decode error: missing field `v` |
| msgpack rmp (struct-map) | error: unknown field `x`, expected one of `t`, `o`, `h`, `l`, `c`,  | OK BarLoose { t: 1, o: 2.0, h: 3.0, l: 1.0, c: 2.5, v: 10.0  | error: missing field `v` |
| postcard | OK Bar { t: 1, o: 2.0, h: 3.0, l: 1.0, c: 2.5, v: 10.0 } | OK BarLoose { t: 1, o: 2.0, h: 3.0, l: 1.0, c: 2.5, v: 10.0  | error: Hit the end of buffer, expected more data |
