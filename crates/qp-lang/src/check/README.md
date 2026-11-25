# check/

Denetimin adımları (`docs/PROGRAMS.md` §2–§5, §8.1, §11): `top.rs` kök (ret ya da program,
bölümler, boyut, kimlikler, `Checked`), `fields.rs` katı nesne okuyucu (bilinmeyen alan),
`params.rs` parametre tanımı ve bağlanan değerler, `args.rs` düğüm alanları (başvuru,
tamsayı/ondalık alan, sabit; iki katlı aralık denetiminin statik yarısı), `nodes.rs` düğüm
başına okuma, `types.rs` tip ve saat, `clock.rs` periyot yazımı ve üst periyot kuralı,
`outputs.rs` çıktılar, sonuç kuralı, durum boyutları, `cost.rs` statik maliyet, `warn.rs`
uyarılar. Dışa açık giriş üst dosya `check.rs`'tedir.
