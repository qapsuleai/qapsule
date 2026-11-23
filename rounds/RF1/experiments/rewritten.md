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

Toplam: 4 yeniden yazılan öğe, 2 yeni öğe. `moved.py`'nin bir CHANGED'ı daha var:
`qp-shellcore` `drive/viewer_less.rs` `fn deliver` — yalnız yol (`qp_session::headparts` →
`qp_plane::headparts`) ve rustfmt'in kısalan satırı yeniden sarması; gövde aynı.
