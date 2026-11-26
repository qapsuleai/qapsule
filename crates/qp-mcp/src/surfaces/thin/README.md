# `thin` yüzeyinin parçaları

`../thin.rs` yüzeyin kendisidir (şemadan üretilmiş araçlar, karar araçlarının ajan ucunda
gizlenmesi, kapsül biçimi). Bu klasör onun konu parçalarını tutar: `program` (RL1,
`docs/PROGRAMS-SESSION.md` §3.3) — ajan ucunda üretilmiş araçlardan sonra listelenen
`program_check`, `program_try`, `program_explain` (metin ya da ara gösterim → program
sorgusu) ve `program_add` (metin → ara gösterim, tek `add_program` komutu; ajandan öneri).
Ayrı bir yüzey değildir; kayıt `../registry.rs`'tedir.
