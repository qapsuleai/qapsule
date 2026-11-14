# T6b encoding bench — platform: native
corpus: 10000 bars; RSI nulls 14; ticks per second 60

## Sizes and times (median; per message)

| case | codec | bytes | enc µs | dec µs | gzip-6 B | brotli-6 B | note |
|---|---|---:|---:|---:|---:|---:|---|
| tick set_last_bar | json (float_roundtrip) | 234 | 0.23 | 0.86 | 174 | 153 |  |
| tick set_last_bar | cbor ciborium | 183 | 0.29 | 1.65 | 190 | 163 |  |
| tick set_last_bar | cbor minicbor-serde | 183 | 0.24 | 0.82 | 190 | 163 |  |
| tick set_last_bar | msgpack rmp (struct-map) | 183 | 0.25 | 0.70 | 196 | 167 |  |
| tick set_last_bar | msgpack rmp (struct-array) | 115 | 0.15 | - | - | - | decode error: invalid type: sequence, expected a string |
| tick set_last_bar | postcard (same types) | 100 | 0.14 | - | - | - | decode error: This is a feature that PostCard will never implement |
| tick set_last_bar | postcard (mirror types) | 78 | 0.20 | 0.09 | 101 | 82 |  |
| tick set_last_bar | json SoA bars | 234 | 0.29 | 0.90 | 174 | 153 |  |
| tick set_last_bar | qcol (LE f64 blocks) | 83 | 0.10 | 0.09 | 106 | 87 |  |
| tick append_bars(1) | json (float_roundtrip) | 236 | 0.26 | 0.90 | 177 | 159 |  |
| tick append_bars(1) | cbor ciborium | 184 | 0.30 | 1.73 | 196 | 172 |  |
| tick append_bars(1) | cbor minicbor-serde | 184 | 0.24 | 0.85 | 196 | 172 |  |
| tick append_bars(1) | msgpack rmp (struct-map) | 184 | 0.25 | 0.72 | 201 | 172 |  |
| tick append_bars(1) | msgpack rmp (struct-array) | 115 | 0.16 | - | - | - | decode error: invalid type: sequence, expected a string |
| tick append_bars(1) | postcard (same types) | 100 | 0.13 | - | - | - | decode error: This is a feature that PostCard will never implement |
| tick append_bars(1) | postcard (mirror types) | 79 | 0.20 | 0.10 | 102 | 83 |  |
| tick append_bars(1) | json SoA bars | 246 | 0.37 | 1.13 | 181 | 165 |  |
| tick append_bars(1) | qcol (LE f64 blocks) | 88 | 0.11 | 0.15 | 111 | 92 |  |
| values RSI 10k | json (float_roundtrip) | 321991 | 198 | 391 | 122124 | 102907 |  |
| values RSI 10k | cbor ciborium | 179986 | 84 | 527 | 123468 | 101001 |  |
| values RSI 10k | cbor minicbor-serde | 179986 | 41 | 183 | 123468 | 101001 |  |
| values RSI 10k | msgpack rmp (struct-map) | 179986 | 70 | 169 | 123471 | 100998 |  |
| values RSI 10k | msgpack rmp (struct-array) | 179931 | 69 | 170 | 123419 | 100962 |  |
| values RSI 10k | postcard (same types) | 154643 | 59 | - | - | - | decode error: This is a feature that PostCard will never implement |
| values RSI 10k | postcard (mirror types) | 154631 | 65 | 48 | 115148 | 109993 |  |
| values RSI 10k | json SoA bars | 321991 | 204 | 395 | 122124 | 102907 |  |
| values RSI 10k | qcol (LE f64 blocks) | 161296 | 35 | 8.60 | 110946 | 105348 |  |
| values MACD(3) 10k | json (float_roundtrip) | 733728 | 883 | 1094 | 308099 | 290159 |  |
| values MACD(3) 10k | cbor ciborium | 359388 | 163 | 1058 | 280789 | 248834 |  |
| values MACD(3) 10k | cbor minicbor-serde | 359388 | 66 | 375 | 280789 | 248834 |  |
| values MACD(3) 10k | msgpack rmp (struct-map) | 359388 | 125 | 353 | 280782 | 248811 |  |
| values MACD(3) 10k | msgpack rmp (struct-array) | 359333 | 124 | 357 | 280734 | 248774 |  |
| values MACD(3) 10k | postcard (same types) | 334043 | 82 | - | - | - | decode error: This is a feature that PostCard will never implement |
| values MACD(3) 10k | postcard (mirror types) | 334031 | 81 | 69 | 270699 | 257637 |  |
| values MACD(3) 10k | json SoA bars | 733728 | 880 | 1055 | 308099 | 290159 |  |
| values MACD(3) 10k | qcol (LE f64 blocks) | 323816 | 82 | 23 | 261321 | 254522 |  |
| append_bars 10k | json (float_roundtrip) | 767220 | 870 | 2381 | 214460 | 209681 |  |
| append_bars 10k | cbor ciborium | 659189 | 531 | 4655 | 237900 | 218279 |  |
| append_bars 10k | cbor minicbor-serde | 670119 | 342 | 1746 | 235588 | 214648 |  |
| append_bars 10k | msgpack rmp (struct-map) | 670119 | 332 | 1509 | 235589 | 214769 |  |
| append_bars 10k | msgpack rmp (struct-array) | 550062 | 180 | - | - | - | decode error: invalid type: sequence, expected a string |
| append_bars 10k | postcard (same types) | 464772 | 75 | - | - | - | decode error: This is a feature that PostCard will never implement |
| append_bars 10k | postcard (mirror types) | 464751 | 91 | 61 | 213695 | 211059 |  |
| append_bars 10k | json SoA bars | 507256 | 615 | 886 | 174735 | 150849 |  |
| append_bars 10k | qcol (LE f64 blocks) | 480040 | 89 | 27 | 189167 | 177532 |  |
| snapshot Document 10k | json (float_roundtrip) | 769516 | 872 | 2397 | 215744 | 210717 |  |
| snapshot Document 10k | cbor ciborium | 660958 | 533 | 4721 | 238989 | 219264 |  |
| snapshot Document 10k | cbor minicbor-serde | 671948 | 336 | 1762 | 236712 | 215634 |  |
| snapshot Document 10k | msgpack rmp (struct-map) | 671948 | 331 | 1512 | 236730 | 215771 |  |
| snapshot Document 10k | msgpack rmp (struct-array) | 551059 | - | - | - | - | decode error: invalid type: sequence, expected a string |
| snapshot Document 10k | postcard (same types) | 465636 | - | - | - | - | decode error: This is a feature that PostCard will never implement |
| snapshot Document 10k | postcard (mirror types) | - | - | - | - | - | n/a (no Document mirror) |
| snapshot Document 10k | json SoA bars | 509577 | 606 | 629 | 175730 | 151512 |  |
| snapshot Document 10k | qcol (LE f64 blocks) | 482480 | 63 | 34 | 190549 | 178424 |  |

## Live tick stream: 60 messages/second

| stream | codec | avg B/msg | B/s raw | ws-deflate B/msg (context takeover) | B/s ws-deflate | enc+dec µs/s (CPU) |
|---|---|---:|---:|---:|---:|---:|
| set_last_bar | json (float_roundtrip) | 233.8 | 14030 | 35.1 | 2107 | 71.1 |
| set_last_bar | cbor ciborium | 182.8 | 10970 | 33.4 | 2005 | 117.3 |
| set_last_bar | cbor minicbor-serde | 183.0 | 10980 | 33.4 | 2003 | 65.7 |
| set_last_bar | msgpack rmp (struct-map) | 183.0 | 10980 | 33.5 | 2008 | 58.5 |
| set_last_bar | msgpack rmp (struct-array) | decode fails | | | | |
| set_last_bar | postcard (same types) | decode fails | | | | |
| set_last_bar | postcard (mirror types) | 78.0 | 4680 | 33.9 | 2036 | 16.4 |
| set_last_bar | json SoA bars | 233.8 | 14030 | 35.1 | 2107 | 75.4 |
| set_last_bar | qcol (LE f64 blocks) | 83.0 | 4980 | 34.1 | 2048 | 11.1 |
| append_bars(1) | json (float_roundtrip) | 236.3 | 14181 | 52.8 | 3166 | 75.3 |
| append_bars(1) | cbor ciborium | 182.7 | 10964 | 60.3 | 3617 | 122.5 |
| append_bars(1) | cbor minicbor-serde | 184.0 | 11040 | 59.8 | 3587 | 68.8 |
| append_bars(1) | msgpack rmp (struct-map) | 184.0 | 11040 | 59.9 | 3593 | 60.3 |
| append_bars(1) | msgpack rmp (struct-array) | decode fails | | | | |
| append_bars(1) | postcard (same types) | decode fails | | | | |
| append_bars(1) | postcard (mirror types) | 79.0 | 4740 | 55.0 | 3300 | 17.9 |
| append_bars(1) | json SoA bars | 246.3 | 14781 | 53.3 | 3198 | 94.7 |
| append_bars(1) | qcol (LE f64 blocks) | 88.0 | 5280 | 55.7 | 3342 | 15.8 |

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
