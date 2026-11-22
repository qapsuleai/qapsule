# qp-schema

Qapsule şemasının tek kaynağı: belge tipleri (`types/`, `grammar`, `capsule`, `data`, `scene`,
`market`, `comment`) ve `qp/0` protokol mesajları (`protocol/`). JSON Schema bu Rust
tiplerinden üretilir (`export.rs`, `qp schema export`); elle yazılmaz. `validation.rs` örneği
üretilen şemaya karşı ve çubuk serisini anlamca sınar. Hesap, çizim ya da otorite mantığı
burada değildir.
