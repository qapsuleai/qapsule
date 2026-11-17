# Tarama — `r4`

531 durum × 2 dil. Bulgu sayısı (hedef: hepsi 0).

| tarama | toplam | durum | başlık | ayrıntı | karne | sözlük | ekran | TR | EN | TR+EN |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| vague | 756 | 227 | 756 | 0 | 0 | 0 | 0 | 311 | 445 | 0 |
| advice | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| repeat | 76 | 76 | 0 | 0 | 0 | 0 | 76 | 76 | 0 | 0 |
| internal | 2344 | 506 | 0 | 2344 | 0 | 0 | 0 | 1181 | 1163 | 0 |
| unglossed | 1557 | 454 | 37 | 1501 | 19 | 0 | 0 | 819 | 738 | 0 |
| length | 1294 | 404 | 0 | 1294 | 0 | 0 | 0 | 654 | 640 | 0 |
| threshold | 1002 | 389 | 0 | 0 | 0 | 0 | 1002 | 523 | 479 | 0 |
| ledger_expect | 1581 | 355 | 0 | 0 | 1581 | 0 | 0 | 882 | 699 | 0 |
| contradiction | 636 | 198 | 0 | 396 | 0 | 0 | 240 | 313 | 323 | 0 |
| equivalence | 20 | 20 | 0 | 20 | 0 | 0 | 0 | 0 | 0 | 20 |
| redacted_digit | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| determinism | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| empty | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

## Örnekler (tarama başına en çok 3 farklı ayrıntı)

- **vague**: s001 tr headline: «büyük (sayısız cümlede)»; s001 en headline: «unusually (sayısız cümlede)»; s001 en headline: «heavy (sayısız cümlede)»
- **repeat**: s033 tr screen: «headline = detail: Hâlâ geçerli.»; s054 tr screen: «headline = detail: Artık geçerli değil.»
- **internal**: s002 tr detail: «sinyali»; s002 en detail: «signal»; s005 tr detail: «bar»
- **unglossed**: s001 tr detail: «kurumsal (kurumsal)»; s001 tr detail: «karnesi (karne\w*)»; s001 en detail: «institutional (institutional)»
- **length**: s001 tr detail: «4 cümle > 3»; s005 tr detail: «249 > 200»; s005 tr detail: «5 cümle > 3»
- **threshold**: s001 tr screen: «dedektör parametresi yok (institutional_multiple)»; s002 tr screen: «eşiğin adı yok (institutional_multiple)»; s033 tr screen: «dedektör parametresi yok (range_extreme)»
- **ledger_expect**: s020 tr ledger: «beklentinin süresi (10) yok»; s020 tr ledger: «beklentinin yönü yok»; s034 tr ledger: «yasak: beklendiği gibi»
- **contradiction**: s001 tr detail: «geçerlilik unknown ≠ valid»; s001 tr screen: «başlık ["valid"] ↔ ayrıntı ["unknown"]»; s011 tr detail: «geçerlilik no_longer ≠ closed:expired»
- **equivalence**: s223 tr+en detail: «sayılar [] ↔ [0.0]»; s230 tr+en detail: «sayılar [1.0, 1.0] ↔ [0.0, 1.0, 1.0]»; s247 tr+en detail: «sayılar [5.0, 21.0] ↔ [0.0, 5.0, 21.0]»
