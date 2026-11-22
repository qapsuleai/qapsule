# `recovered/worktree-agent/*` dalları — özet (RG1/W, salt okuma)

Yöntem: her dal için `git merge-base --is-ancestor`, `git rev-list --count main..dal`, `git cherry main dal`,
`git diff --stat main...dal`; `+` görünen commit'ler main'deki aynı başlıklı commit'le satır satır karşılaştırıldı.
Hiçbir dal silinmedi ya da değiştirilmedi.

| Dal (`recovered/worktree-agent/…`) | Konu | main'de olmayan commit · cherry +/− | Bulgu | Öneri |
|---|---|---|---|---|
| 0474490800 | R5/t2-lang M17/M19 düzenekleri | 1 · +1/−0 | main `649110f`; fark yalnız commit edilmiş `target/` çıktısı | sil |
| 155233fafa | R5/T3 ornek s1 accent | 6 · +0/−6 | hepsi main'de (`8650c2a`) | sil |
| 15641d59c6 | R4 kapanış belgeleri | 1 · +1/−0 | main `5af4a14`; dalda eski yer tutucular | sil |
| 1b4f22a61f | R6/T5 LiveFeed, `qp replay --raw` | 1 · +1/−0 | main `acdc834` | sil |
| 1c67ddb67f | R5 M16 oltan | 1 · +0/−1 | main'de | sil |
| 1c860dac35 | R5 M19 | 4 · +0/−4 | main'de | sil |
| 25c9a676bf | R3 eval sim Cargo.lock | 1 · +1/−0 | main'de geçildi (`21053f2`) | sil |
| 31b9e3b089 | R5 M16 zemre | 3 · +0/−3 | main'de | sil |
| 4f5213ef43 | RP3 birleştirme | 1 (merge) · 0/0 | iki ebeveyn main'de, boş | sil |
| 4f710f649b | RP1/E4 | 1 · +1/−0 | main `1de91f5` (rebase farkı) | sil |
| 5175f67c62 | R5/T3b | 2 · +0/−2 | main'de | sil |
| 5361b219f1 | R3/M1b rozet | 1 · +1/−0 | main `70caea9`; fark commit edilmiş `target/` (~1500 dosya) | sil |
| 5b2f82cdb2 | R6/T1 Cargo.lock birleşimi | 1 (merge) · 0/0 | ağacı main `a88a808` ile aynı | sil |
| 5d339288b6 | RP1/E1 | 1 · +1/−0 | main `cf6d438` ile aynı satırlar | sil |
| 618f5a53ec | R6/T1 (5b2f82cdb2'nin kopyası) | 1 (merge) · 0/0 | ağacı `a88a808` ile aynı | sil |
| 72cec3080d | RP1/E8 | 1 · +1/−0 | main `36213a6`, 1 satır rebase farkı | sil |
| 89a998e380 | R5 rapor taslağı | 1 · +0/−1 | main'de | sil |
| 8c3593435e | RP1 birleştirmeleri | 4 (merge) · 0/0 | eşleri main'de | sil |
| bba97ec414 | R5 M16 bertan (elenen) | 2 · +2/−0 | patch'ler `r5/m16-bertan` dalında birebir | sil (adlı dal kalır) |
| c2f41fe759 | R6/T5 ilk durum zamanı | 1 · +1/−0 | main `5983b8c` | sil |
| c3621b72f7 | R5/t0-small | 2 · +0/−2 | main'de | sil |
| ddc99df301 | R5 M18 protokol | 4 · +0/−4 | main'de | sil |
| f4ae3368b5 | RP1/E7 | 1 · +1/−0 | main `c824a4a`, 2 satır biçim farkı | sil |
| f6fa700a4f | R5 M16 kivra (elenen) | 4 · +4/−0 | patch'ler `r5/m16-kivra` dalında birebir | sil (adlı dal kalır) |

Sonuç: 24/24 dalda benzersiz iş yok. Elenen alternatifler (`bertan`, `kivra`) adlı `r5/m16-*` dallarında
korunuyor; o dallar kalmalı. Karar Arkin'in.
