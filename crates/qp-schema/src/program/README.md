# program/

Hesap dilinin (`qp-ir/0`, `docs/PROGRAMS.md`) tipleri: `node.rs` (düğümler ve `op`
türleri), `output.rs` (çıktılar, sonuç kuralı), `param.rs` (program parametresi ve sayı
alanları), `canonical.rs` (kanonik JSON ve kimlik biçimi; özet `qp-lang`'de), `record.rs`
(sinyal kaydı, değer kökeni), `reply.rs` (`check_program`, `try_program`, `explain_program`
cevapları), `head.rs` (kimlik/ad alanı önde yazılan nesnelerin serde yardımı). Dışa açık
yollar `qp_schema::program::X`. Denetim, çalıştırıcı ve bütçe burada değildir (`qp-lang`).
