# RF1 — Yeniden yazılan öğeler

`PLAN.md` §0: taşınan öğe `moved.py` ile "değişmedi" çıkar; gövdesi ya da imzası değişen her
öğe burada sayılır.

## S2 — `qp-ops`, `qp-plane` (B7: sağlayıcının `&Session` bağı)

| Öğe | Ne değişti |
|---|---|
| `qp_ops::provider::DocProvider` (yapı) | `parts` alanı `Option<(&Session, Language)>` → `Option<(&dyn HeadlineSource, Language)>` (alan belgesi de) |
| `DocProvider::with_parts` | parametre `&Session` → `&dyn HeadlineSource` (gövde aynı; çağıranlar `with_parts(self, lang)` değişmedi) |
| `impl DataProvider for DocProvider` → `headline_parts` | `s.doc` / `s.proj` ile `reader_parts` çağrısı → `s.headline_parts(finding, lang)` |
| `impl DataProvider for DocProvider` → `voice` | `s.doc` / `s.proj` ile `reader_voice` çağrısı → `s.voice(finding, lang, v, self.clock)` |

Yeni öğeler (eski gövdeler buraya taşındı):

- `qp_ops::provider::HeadlineSource` (trait; iki yöntem).
- `impl qp_ops::HeadlineSource for Session` (`qp-session` `capsule/headsource.rs`): eski iki gövde
  aynen, `s` → `self`.

- `qp_plane::values::Computed::is_empty` (yeni, kullanılmıyor): `Computed` ve `len` crate
  sınırını geçince clippy `len_without_is_empty` istedi; modül/crate düzeyinde susturma yerine
  eklendi. `Computed` ve `impl`'i `values.rs`'ten `values/computed.rs`'e saf taşımayla gitti
  (`new` ve `main_ix` `pub(super)`), böylece `values.rs` taban çizgisi değerini aşmadı.

Toplam: 4 yeniden yazılan öğe, 3 yeni öğe. `moved.py`'nin bir CHANGED'ı daha var:
`qp-shellcore` `drive/viewer_less.rs` `fn deliver` — yalnız yol (`qp_session::headparts` →
`qp_plane::headparts`) ve rustfmt'in kısalan satırı yeniden sarması; gövde aynı.

## S4 — `expand_op` kol başına fonksiyon (`qp-ops` `expand/op.rs` + `expand/op/`)

`expand_op` bir dağıtıcı `match` olarak kaldı (`op.rs` 121 satır, fonksiyon 101 satır); 22 kol
kendi fonksiyonuna gitti. Her fonksiyonun gövdesi kolun gövdesidir: değerlendirme sırası, hata
iletileri ve erken dönüşler aynı; `return` kolda da `expand_op`'tan dönüyordu.

- **Kol fonksiyonları alanları parametre olarak alır.** Dağıtıcı alanları açar, örneğin
  `Op::PutSeries { series } => series::put_series(doc, series, p, ctx)`. Kalıpta
  `unreachable!` yoktur.
- **Yedi parametre sınırı:** alanı çok olan dört kol için `Arm { doc, p, ctx }` yapısı kullanılır
  (yeni öğe): `add_study`, `add_fact`, `add_finding`, `add_comment`.
- **`ptr_arg` izni:** alan bağları `&String` / `&Vec<_>` olduğundan, gövde değişmesin diye on
  kol fonksiyonuna öğe düzeyinde `#[allow(clippy::ptr_arg)]` kondu. Bu, `arch-check`'in saydığı
  beş lint'ten biri değil.
- **`ctx` ve `doc` almayanlar:** `ctx` kullanmayanlar onu almaz (`set_params`, `remove_layer`,
  `append_bars`, `update_last_bar`); `set_audience` `doc` almaz.
- **Satır içinde kalan kollar:** `add_watch`/`remove_watch`, `undo`, gramer işlemleri,
  `accept_proposal`.
- **Dağıtıcı 101 satır** (alanları açılmış 23 kol, `rustfmt` biçimi). Arkin'in kararıyla
  (`DECISION.md`, uygulama notları) `fn.lines` taban çizgisinde 101 satırla durur ve
  `too_many_lines` izni kalır (`lint.allow` `qp-ops` 21). `main`'e göre borç 903 → 101;
  `cognitive_complexity` izni kalktı.

| Yeni fonksiyon | Dosya |
|---|---|
| `put_series`, `put_series_source`, `add_study`, `set_params`, `append_bars`, `update_last_bar` | `op/series.rs` |
| `put_view`, `add_layer`, `remove_layer`, `set_view_range` | `op/view.rs` |
| `add_fact`, `add_finding`, `update_finding`, `decide_finding` (onay/ret), `add_narrative`, `show_finding` | `op/finding.rs` |
| `add_intent`, `set_intent_active`, `add_comment`, `remove`, `set_audience`, `set_sensitivity`, `reject_proposal` | `op/people.rs` |

- **`put_series` ikiye bölündü:** 80 satırı aştığı için kaynağa göre denetim (`match &series.source`,
  `let src` ile) `put_series_source(doc, series, &mut s, &sp, ctx)` adımına gitti. Orada `sp`
  `&str` olduğundan 6 yerde `&sp` → `sp` (clippy `needless_borrow`); `s.source = …` ataması
  `&mut` üstünden.
- **Sayım:** yeniden yazılan öğe `expand_op` (1); yeni öğeler 23 kol fonksiyonu ve `struct Arm`.
  `expand_op` dışında yeni fonksiyonların hiçbiri 80 satırı aşmıyor; dosyalar ≤ 337 satır.
- **İ6:** `op.rs` kalemi yapılandırmadan silindi.

## Not: crate düzeyi izin

`qp-ops` ve `qp-plane`'in `lib.rs`'inde `#![allow(clippy::result_large_err)]` var. Bu,
`qp-session`'da zaten bulunan iznin kopyası: protokol hatası soğuk yolda kutusuz dönüyor,
gerekçe `qp-session` `lib.rs`'teki yorum. `arch-check`'in `lint.inner` kuralı bu lint'e bakmıyor.
Taşınan kodla birlikte geldi; yeni bir susturma değil.
