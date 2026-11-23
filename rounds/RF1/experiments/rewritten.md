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

`expand_op` dağıtıcı `match`'i olarak kaldı (56 satırlık dosya); 22 kol kendi fonksiyonuna
gitti. Her fonksiyonun gövdesi kolun gövdesidir (aynen; değerlendirme sırası, hata iletileri,
erken dönüşler aynı; `return` kolda da `expand_op`'tan dönüyordu). Her fonksiyon `op`'u
`let Op::X { … } = op else { unreachable!(…) };` ile açar (`expand_watch`'ın kalıbı);
dağıtıcı yalnız o kolu o fonksiyona gönderir. `ctx` kullanmayanlar onu almaz
(`set_params`, `remove_layer`, `append_bars`, `update_last_bar`), `set_audience` `doc` almaz.
Kalan satır içi kollar: `add_watch`/`remove_watch`, `undo`, gramer işlemleri,
`accept_proposal`. `expand_op`'un `too_many_lines` ve `cognitive_complexity` izinleri kalktı.

| Yeni fonksiyon | Dosya |
|---|---|
| `put_series`, `put_series_source`, `add_study`, `set_params`, `append_bars`, `update_last_bar` | `op/series.rs` |
| `put_view`, `add_layer`, `remove_layer`, `set_view_range` | `op/view.rs` |
| `add_fact`, `add_finding`, `update_finding`, `decide_finding` (onay/ret), `add_narrative`, `show_finding` | `op/finding.rs` |
| `add_intent`, `set_intent_active`, `add_comment`, `remove`, `set_audience`, `set_sensitivity`, `reject_proposal` | `op/people.rs` |

- `put_series` 80 satırı aştığı için ikiye bölündü: kaynağa göre denetim (`match &series.source`,
  `let src` ile) `put_series_source(doc, series, &mut s, &sp, ctx)` adımına gitti; orada `sp`
  `&str` olduğundan `&sp` → `sp` (6 yer; clippy `needless_borrow`), `s.source = …` `&mut` üstünden.
- Yeniden yazılan öğe sayısı: `expand_op` (1) + 23 yeni fonksiyon. Yeni fonksiyonların hiçbiri
  80 satırı aşmıyor (arch-check `fn.lines` taban çizgisinde `expand/op` kalemi yok); dosyalar ≤ 337.
- İ6'nın `op.rs` kalemi yapılandırmadan silindi; `expand_op` `fn.lines` kalemi kalktı.
