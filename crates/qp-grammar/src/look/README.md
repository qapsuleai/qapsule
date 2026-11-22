# look

Görünüşler (R5/T3): gramerin tasarım kayıtları, veri olarak. `look.rs` tipleri ve `Look`
yöntemlerini tutar; `fonts.rs` yazı tipleri, `r4.rs`/`ornek.rs`/`oltan.rs`/`zemre.rs`/`tavek.rs` birer
görünüş, `registry.rs` kayıt ve ada göre arama, `css.rs` kabuk CSS değişkenleri, `chart.rs`
(RD2/S1) yoğunluk profili, köken renk rolleri ve grafik alışkanlıkları (`ChartLook`).
Ne değildir: tema renk hesabı (`theme.rs`) ya da çizim.

**Profiller (RD2):** bir tasarım yönü iki görünüş kaydeder: `<ad>` (`Profile::Dense`,
masaüstü) ve `<ad>-tel` (`Profile::Touch`, telefon; `..<AD>` ile yazılır, tip, krom, boşluk
ve işaret değişir). RD2'den önceki görünüşler `Profile::Legacy`'dir; `scripts/design-check`
yalnız legacy olmayanları sınar (`docs/design/CONTRACT.md`).
