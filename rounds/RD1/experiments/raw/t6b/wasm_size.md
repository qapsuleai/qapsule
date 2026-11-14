| features | .wasm bytes (wasm32-unknown-unknown, opt-level s, lto fat, strip) | gzip -9 bytes |
|---|---:|---:|
| json-none | 13331 | 5134 |
| json (baseline) | 822239 | 206689 |
| cbor-ciborium | 1684550 | 394134 |
| cbor-minicbor | 1718544 | 405246 |
| msgpack | 1699210 | 397590 |
| postcard-mirror | 827084 | 207156 |
| qcol | 839920 | 214244 |
| json-none,qcol | 64420 | 25209 |
