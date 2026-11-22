# `query` yüzeyinin parçaları

`../query.rs` yüzeyin kendisidir (`Query`, `ToolSurface`, hata ve `qp/0` G/Ç tipleri); bu
klasör onun konu parçalarını tutar: argümanlar (`args`), sözlük ve anahtar denetimi
(`check`), kapsül öğeleri (`capsule`), `find` isimleri (`find`, `find_records`), biçimleme
(`shape`) ve iki araç (`ask`, `act`). Ayrı bir yüzey değildir; kayıt `../registry.rs`'tedir.
