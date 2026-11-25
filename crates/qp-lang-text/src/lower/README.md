# lower/

Metin deyimlerinden ham JSON ara gösterime iniş (`docs/PROGRAMS.md` §12.3): `run.rs`
(sürücü: adlar, ret, deyim sırası), `state.rs` (ad tablosu, kurulan düğümler, kaynak haritası,
düğüm ekleme ve `_k` numarası), `stmt.rs` (parametre, bağlama, çıktı, ret), `expr.rs`
(ifade → düğüm; son-sıra numaralama, takma ad), `args.rs` (işlev çağrısı, sayı alanları).
Yalnız ara gösterimde yazılamayan şey burada hatadır; gerisi denetçinin (`qp-lang`) işidir.
