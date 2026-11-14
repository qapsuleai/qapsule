# T6b encoding bench — platform: wasm32-wasip1 (node)
corpus: 10000 bars; RSI nulls 14; ticks per second 60

## Sizes and times (median; per message)

| case | codec | bytes | enc µs | dec µs | gzip-6 B | brotli-6 B | note |
|---|---|---:|---:|---:|---:|---:|---|
| tick set_last_bar | json (float_roundtrip) | 234 | 0.39 | 1.11 | - | - |  |
| tick set_last_bar | cbor ciborium | 183 | 0.44 | 2.00 | - | - |  |
| tick set_last_bar | cbor minicbor-serde | 183 | 0.23 | 1.22 | - | - |  |
| tick set_last_bar | msgpack rmp (struct-map) | 183 | 0.32 | 0.85 | - | - |  |
| tick set_last_bar | msgpack rmp (struct-array) | 115 | 0.19 | - | - | - | decode error: invalid type: sequence, expected a string |
| tick set_last_bar | postcard (same types) | 100 | 0.12 | - | - | - | decode error: This is a feature that PostCard will never implement |
| tick set_last_bar | postcard (mirror types) | 78 | 0.16 | 0.10 | - | - |  |
| tick set_last_bar | json SoA bars | 234 | 0.43 | 1.09 | - | - |  |
| tick set_last_bar | qcol (LE f64 blocks) | 83 | 0.07 | 0.09 | - | - |  |
| tick append_bars(1) | json (float_roundtrip) | 236 | 0.39 | 1.14 | - | - |  |
| tick append_bars(1) | cbor ciborium | 184 | 0.45 | 2.07 | - | - |  |
| tick append_bars(1) | cbor minicbor-serde | 184 | 0.23 | 1.23 | - | - |  |
| tick append_bars(1) | msgpack rmp (struct-map) | 184 | 0.34 | 0.88 | - | - |  |
| tick append_bars(1) | msgpack rmp (struct-array) | 115 | 0.20 | - | - | - | decode error: invalid type: sequence, expected a string |
| tick append_bars(1) | postcard (same types) | 100 | 0.12 | - | - | - | decode error: This is a feature that PostCard will never implement |
| tick append_bars(1) | postcard (mirror types) | 79 | 0.17 | 0.12 | - | - |  |
| tick append_bars(1) | json SoA bars | 246 | 0.52 | 1.45 | - | - |  |
| tick append_bars(1) | qcol (LE f64 blocks) | 88 | 0.08 | 0.17 | - | - |  |
| values RSI 10k | json (float_roundtrip) | 321991 | 333 | 549 | - | - |  |
| values RSI 10k | cbor ciborium | 179986 | 156 | 768 | - | - |  |
| values RSI 10k | cbor minicbor-serde | 179986 | 82 | 363 | - | - |  |
| values RSI 10k | msgpack rmp (struct-map) | 179986 | 132 | 251 | - | - |  |
| values RSI 10k | msgpack rmp (struct-array) | 179931 | 124 | 250 | - | - |  |
| values RSI 10k | postcard (same types) | 154643 | 85 | - | - | - | decode error: This is a feature that PostCard will never implement |
| values RSI 10k | postcard (mirror types) | 154631 | 88 | 45 | - | - |  |
| values RSI 10k | json SoA bars | 321991 | 334 | 544 | - | - |  |
| values RSI 10k | qcol (LE f64 blocks) | 161296 | 22 | 9.68 | - | - |  |
| values MACD(3) 10k | json (float_roundtrip) | 733728 | 1166 | 1376 | - | - |  |
| values MACD(3) 10k | cbor ciborium | 359388 | 319 | 1568 | - | - |  |
| values MACD(3) 10k | cbor minicbor-serde | 359388 | 124 | 768 | - | - |  |
| values MACD(3) 10k | msgpack rmp (struct-map) | 359388 | 255 | 577 | - | - |  |
| values MACD(3) 10k | msgpack rmp (struct-array) | 359333 | 258 | 535 | - | - |  |
| values MACD(3) 10k | postcard (same types) | 334043 | 110 | - | - | - | decode error: This is a feature that PostCard will never implement |
| values MACD(3) 10k | postcard (mirror types) | 334031 | 126 | 64 | - | - |  |
| values MACD(3) 10k | json SoA bars | 733728 | 1188 | 1372 | - | - |  |
| values MACD(3) 10k | qcol (LE f64 blocks) | 323816 | 50 | 25 | - | - |  |
| append_bars 10k | json (float_roundtrip) | 767220 | 1642 | 3056 | - | - |  |
| append_bars 10k | cbor ciborium | 659189 | 1060 | 5709 | - | - |  |
| append_bars 10k | cbor minicbor-serde | 670119 | 256 | 3015 | - | - |  |
| append_bars 10k | msgpack rmp (struct-map) | 670119 | 670 | 1830 | - | - |  |
| append_bars 10k | msgpack rmp (struct-array) | 550062 | 325 | - | - | - | decode error: invalid type: sequence, expected a string |
| append_bars 10k | postcard (same types) | 464772 | 112 | - | - | - | decode error: This is a feature that PostCard will never implement |
| append_bars 10k | postcard (mirror types) | 464751 | 123 | 67 | - | - |  |
| append_bars 10k | json SoA bars | 507256 | 1178 | 1248 | - | - |  |
| append_bars 10k | qcol (LE f64 blocks) | 480040 | 50 | 34 | - | - |  |
| snapshot Document 10k | json (float_roundtrip) | 769516 | 1639 | 3083 | - | - |  |
| snapshot Document 10k | cbor ciborium | 660958 | 1045 | 5759 | - | - |  |
| snapshot Document 10k | cbor minicbor-serde | 671948 | 263 | 3012 | - | - |  |
| snapshot Document 10k | msgpack rmp (struct-map) | 671948 | 692 | 1847 | - | - |  |
| snapshot Document 10k | msgpack rmp (struct-array) | 551059 | - | - | - | - | decode error: invalid type: sequence, expected a string |
| snapshot Document 10k | postcard (same types) | 465636 | - | - | - | - | decode error: This is a feature that PostCard will never implement |
| snapshot Document 10k | postcard (mirror types) | - | - | - | - | - | n/a (no Document mirror) |
| snapshot Document 10k | json SoA bars | 509577 | 1191 | 1029 | - | - |  |
| snapshot Document 10k | qcol (LE f64 blocks) | 482480 | 53 | 44 | - | - |  |

## Live tick stream: 60 messages/second

| stream | codec | avg B/msg | B/s raw | ws-deflate B/msg (context takeover) | B/s ws-deflate | enc+dec µs/s (CPU) |
|---|---|---:|---:|---:|---:|---:|
| set_last_bar | json (float_roundtrip) | 233.8 | 14030 | NaN | NaN | 92.3 |
| set_last_bar | cbor ciborium | 182.8 | 10970 | NaN | NaN | 150.9 |
| set_last_bar | cbor minicbor-serde | 183.0 | 10980 | NaN | NaN | 87.1 |
| set_last_bar | msgpack rmp (struct-map) | 183.0 | 10980 | NaN | NaN | 71.1 |
| set_last_bar | msgpack rmp (struct-array) | decode fails | | | | |
| set_last_bar | postcard (same types) | decode fails | | | | |
| set_last_bar | postcard (mirror types) | 78.0 | 4680 | NaN | NaN | 17.2 |
| set_last_bar | json SoA bars | 233.8 | 14030 | NaN | NaN | 96.2 |
| set_last_bar | qcol (LE f64 blocks) | 83.0 | 4980 | NaN | NaN | 11.1 |
| append_bars(1) | json (float_roundtrip) | 236.3 | 14181 | NaN | NaN | 104.9 |
| append_bars(1) | cbor ciborium | 182.7 | 10964 | NaN | NaN | 158.4 |
| append_bars(1) | cbor minicbor-serde | 184.0 | 11040 | NaN | NaN | 92.5 |
| append_bars(1) | msgpack rmp (struct-map) | 184.0 | 11040 | NaN | NaN | 74.2 |
| append_bars(1) | msgpack rmp (struct-array) | decode fails | | | | |
| append_bars(1) | postcard (same types) | decode fails | | | | |
| append_bars(1) | postcard (mirror types) | 79.0 | 4740 | NaN | NaN | 18.6 |
| append_bars(1) | json SoA bars | 246.3 | 14781 | NaN | NaN | 119.3 |
| append_bars(1) | qcol (LE f64 blocks) | 88.0 | 5280 | NaN | NaN | 15.3 |

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
