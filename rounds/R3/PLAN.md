# R3 — Orkestrasyon planı (orkestratörün çalışma notu)

Bütünleştirme dalı `r3/int`. Her alt görev kendi dalında (`r3/<görev>`), kendi worktree'sinde;
`--no-ff` ile `r3/int`'e. `main`'e yalnız round sonunda, verify yeşilken.

## Evre 1 — makine
- T1 (orkestratör): sözlük, CAPSULE §9, şema `qp-schema::capsule`, sözleşme iskeletleri
  (`qp-facts`, `qp-capsule`, `qp-grammar::glyph`), 20 ajan görevi. → `r3/int`
- T2 (alt ajan, `r3/t2-data`): `qp-source` rejimli sentetik + sentetik lisanslı akış;
  `scripts/fetch-bars.sh`; `fixtures/local` okuyucu; DATA_ACCESS doğrulamaları.
- T3 (alt ajan, `r3/t3-facts`): `qp-facts` dedektörleri; `qp-compute` ek hesaplar.
- T4+T5 (alt ajan, `r3/t45-capsule`): `qp-capsule` motoru (yaşam döngüsü, aday, karne,
  ileriye bakma testi, hassasiyet süzgeci), iskele stratejiler yalnız test için.
- Doğrulama: ayrı denetçi alt ajan (T2–T5).
- T6/T7 Evre 2 S ile birleşik (seçimden sonra): protokol eki ve gramer `glyph` işareti.

## Evre 2 — ölçüm
- H: düzenekler (alternatiflerden önce, ayrı commit) `rounds/R3/experiments/harness/`.
- M1a–c, M2a–b, M3a–c, M4a–b, M5a–b, M6a–b, M7: her biri ayrı dal + ayrı alt ajan.
- S: seçimlerin bütünleştirilmesi; T6/T7; altın görseller; dökümler; hassasiyet benzetimi; performans.
- E: ajan yazabilirlik; HT: insan testi paketi; A: son denetim.
- [ ] HT generate.sh yeniden: S-C 'Sinyalın' düzeltmesinden sonra
