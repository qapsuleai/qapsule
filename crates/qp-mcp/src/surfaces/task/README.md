# `task` yüzeyinin parçaları

`../task.rs` yüzeyin kendisidir (`Task` durumu, araç tanımları, `ToolSurface`, `Out` metin
düzeni); bu klasör onun konu parçalarını tutar: argümanlar (`args`), sonuç ve kapsül satırları
(`text`), izlemeler (`watch`: `watch` aracı ve `whats_new`'un izleme kısmı) ve öbür araçlar (`find`,
`explain`, `instrument`, `write`, `glossary`). Ayrı bir yüzey değildir; kayıt `../registry.rs`'tedir.
