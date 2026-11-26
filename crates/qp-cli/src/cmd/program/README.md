# `qp program` parçaları

`../program.rs` komutun kendisidir: argüman yapıları, dosyayı okuma (JSON nesnesi → ara
gösterim, değilse metin; boyut sınırı okumadan önce, bozuk JSON `E_PARSE`), denetim, `check` gövdesi, çıkış kodları. `full.rs`: `run` (try
özeti ya da `--full` bar başına çıktı, kapalı pencere) ve `explain` gövdeleri
(`docs/PROGRAMS-SESSION.md` §3.3).
