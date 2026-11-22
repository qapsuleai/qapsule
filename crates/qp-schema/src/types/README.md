# types/

Şema v0'ın belge tipleri, iki konuda: `series.rs` (çubuk, çalışma çıktısı, enstrüman,
takvim, `Series`, kaynak bilgisi) ve `document.rs` (olgu, bulgu, kapsül, niyet, defter,
`Document`, eski `Command`/`Op` taslağı). Dışa açık yollar `qp_schema::X` olarak
`types.rs` ve `lib.rs`'teki `pub use` ile korunur. Protokol mesajları burada değildir
(`protocol/`).
