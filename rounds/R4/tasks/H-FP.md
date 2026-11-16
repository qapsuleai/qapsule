# R4/H-FP — Yüzün başlık şablonu için arayüz (M8 öncesi)

Önce `rounds/R4/tasks/COMMON.md`; `rounds/R4/experiments/harness/SPEC.md` §2 ve §8 (E1, E5);
`rounds/R4/experiments/harness/m8910/README.md` (M8 arayüzü); `rounds/R4/experiments/T3.md` §1.4;
`crates/qp-capsule/src/realize/parts.rs`; `crates/qp-grammar/src/faces.rs`, `layer.rs`
(`headline` metin alanı), `provider.rs`; `crates/qp-session/src/provider.rs`, `reading.rs`,
`present.rs`, `interact.rs`; `crates/qp-wasm/src/shell.rs` (yansının sağlayıcısı).

Sorun: M8 yüz alternatifleri kendi **başlık şablonlarını** yazmalı (SPEC E5; sert eşik başlık
≤ 80 karakter TR/EN). Bugün yüz yalnız kapsülün tam cümlesini (`headline`, R3 cümlesi, en çok
~205 karakter) ve karne satırını (`ledger`) görebiliyor; T3'ün `realize::parts::Parts` yapı
taşları gramere hiç ulaşmıyor.

Yapılacak (tasarım yazmadan; yalnız arayüz):
1. `qp-grammar`'da sade bir `HeadlineParts` (yalnız metinler + birkaç bayrak: okuyucunun
   dilinde olay adı, yön kelimesi, ne zaman (süre ifadesi, takvime göre), son hareket,
   geçerlilik ifadesi, kısaltılmış karne, enstrüman etiketi, aşama, yetkili mi (rakamsız mı),
   karne n ve az örnekli mi) ve `DataProvider::headline_parts(finding) -> Option<HeadlineParts>`
   (varsayılan `None`). Oturum (otorite okuma/sunum/etkileşim yolları) ve yansının sağlayıcısı
   bunu `qp_capsule::realize::parts` ile, `headlines`'ı doldurdukları aynı yerde, aynı dil ve
   kitle kuralıyla doldurur (yetkisize rakamsız). İç terim yok.
2. `FaceDesign`'a isteğe bağlı başlık şablonu: `headline: Option<fn(&HeadlineParts) -> String>`
   (tek dilde; `HeadlineParts` dili taşır). Etkin yüzün şablonu varsa, o yüzün sahnesinde
   bulgunun `headline` metin alanı şablonun çıktısıdır (düzenek başlığı oradan ölçer); yoksa
   bugünkü cümle. Etkin yüzün nasıl bilineceğini sen seç (ör. `CompileOptions.face` ya da yüz
   görünüm işlevine sağlayıcı sarmalayıcısı); yalnız yüz sahnesi etkilenir: protokoldeki
   kapsül `headline`'ı, açık kapsül, grafik ve dökümler **bayt eşit** kalır. `r3`: `None`.
3. Testte örnek bir şablon (test içinde, kayda girmeyen) S8'in birkaç durumu için TR/EN
   çıktı verir; yetkisiz okuyucuda rakam yok. Belirleyici.
4. Düzenek (`rounds/R4/experiments/harness/m8910/`): SPEC E1 (S4 "az örnek" kovası
   `n < GLYPH_MIN_LEDGER_N`); `s8.json` ve başvuru `r3` ölçülerini (`M8/raw/r3/`) yeniden üret;
   README'nin M8 bölümüne şablon arayüzünü ekle (alternatifin yazacağı yer yine `faces.rs`).
5. `docs/CONCEPTS.md` "Başlık yapı taşları" satırını gerekiyorsa güncelle.

Yazdığın yerler: `crates/qp-grammar/`, `crates/qp-session/`, `crates/qp-wasm/src/shell.rs`
(yalnız sağlayıcı), `crates/qp-capsule/src/realize/parts.rs` (eksik yapı taşı gerekiyorsa),
düzenek klasörü, `docs/CONCEPTS.md`, `rounds/R4/experiments/H-FP.md` (not).
Var olan altınlar/dökümler değişmez. `QP_VERIFY_BROWSER=1 scripts/verify.sh` yeşil (demo
adımındaki `feed-visitor.page.png` dalgalanması bilinen sorundur; tekrar koşunca geçiyorsa yaz).
Son mesajda: dal, commit, M8 alternatiflerine verilecek şablon arayüzünün tam tarifi.
