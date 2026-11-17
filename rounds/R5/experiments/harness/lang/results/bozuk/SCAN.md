# Tarama — `bozuk`

531 durum × 2 dil. Bulgu sayısı (hedef: hepsi 0).

| tarama | toplam | durum | başlık | ayrıntı | karne | sözlük | ekran | TR | EN | TR+EN |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| vague | 7681 | 531 | 4010 | 3671 | 0 | 0 | 0 | 4074 | 3607 | 0 |
| advice | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| repeat | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| internal | 15088 | 531 | 7434 | 6592 | 1062 | 0 | 0 | 7553 | 7535 | 0 |
| unglossed | 2414 | 531 | 382 | 1501 | 531 | 0 | 0 | 1510 | 904 | 0 |
| length | 3184 | 531 | 1062 | 2122 | 0 | 0 | 0 | 1591 | 1593 | 0 |
| threshold | 1854 | 531 | 0 | 0 | 0 | 0 | 1854 | 949 | 905 | 0 |
| ledger_expect | 3375 | 531 | 0 | 0 | 3375 | 0 | 0 | 1779 | 1596 | 0 |
| contradiction | 396 | 198 | 0 | 396 | 0 | 0 | 0 | 198 | 198 | 0 |
| equivalence | 14 | 14 | 0 | 14 | 0 | 0 | 0 | 0 | 0 | 14 |
| redacted_digit | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| determinism | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| empty | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

## Örnekler (tarama başına en çok 3 farklı ayrıntı)

- **vague**: s001 tr headline: «önemli bir eşikte»; s001 tr headline: «güçlü sinyal»; s001 tr headline: «önemli (sayısız cümlede)»
- **internal**: s001 tr headline: «bar»; s001 tr headline: «teyit»; s001 tr headline: «aile»
- **unglossed**: s001 tr detail: «kurumsal (kurumsal)»; s001 tr detail: «karnesi (karne\w*)»; s001 tr ledger: «karne (karne\w*)»
- **length**: s001 tr headline: «144 > 80»; s001 tr detail: «293 > 200»; s001 tr detail: «7 cümle > 3»
- **threshold**: s001 tr screen: «eşiğin değeri yok (institutional_multiple)»; s001 tr screen: «dedektör parametresi yok (institutional_multiple)»; s002 tr screen: «eşiğin adı yok (institutional_multiple)»
- **ledger_expect**: s001 tr ledger: «yasak: beklendiği gibi»; s001 en ledger: «yasak: as expected»; s020 tr ledger: «sayılar (n, tutan) yok»
- **contradiction**: s001 tr detail: «geçerlilik unknown ≠ valid»; s011 tr detail: «geçerlilik no_longer ≠ closed:expired»; s013 tr detail: «geçerlilik no_longer ≠ closed:failed»
- **equivalence**: s518 tr+en detail: «geçerlilik ["closed:retracted", "no_longer"] ↔ ["no_longer"]»; s520 tr+en detail: «geçerlilik ["closed:retracted"] ↔ []»
