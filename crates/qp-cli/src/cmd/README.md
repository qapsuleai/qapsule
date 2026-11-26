# cmd/

`qp` komutlarının gövdeleri, konu başına bir dosya: `replay`, `scene` (sahne, açıklama,
bakış, çizim), `compute`, `schema`, `serve` (`serve`, `conform`),
`program` (RL1: `qp program check | run | explain`; argüman yapıları da burada, `run`/`explain`
gövdesi `program/full.rs`). Argüman yapıları ve
dağıtım `main.rs`'tedir; barındırma komutları (`demo`, `sim`, `live`, `mcp serve`)
`qp-host`'tadır.
