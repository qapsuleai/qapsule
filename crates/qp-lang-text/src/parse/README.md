# parse/

Sözcüklerden deyimlere (`docs/PROGRAMS.md` §12.1, §12.4): `split.rs` (akışı satır başında
deyimlere keser; `{ … }` bloğu satırlara yayılabilir; her deyim ayrı ayrıştırılır, bir hata
sonraki satırları durdurmaz), `cursor.rs` (deyim içi konum, "beklenen …, bulunan …"
hataları), `stmt.rs` (deyimler), `expr.rs` (ifadeler, işleç önceliği).
