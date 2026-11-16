# M3 — Önem sıralaması: sonuçlar ve seçim

Düzenek `harness-sim` (`176f77e`). Alternatifler ayrı dallarda, birbirini görmeden: **M3a
`rarity`** (`r3/m3a-rarity` `3546a94`), **M3b `ledger`** (`r3/m3b-ledger` `9489154`), **M3c
`confirm`** (`r3/m3c-confirm` `38147c9`). Değerlendirme dalı `r3/eval-sim`.

Üç savunucu da aynı ölçü kusurunu bağımsız bildirdi: başvuru sonuç kuralı yönsüz sıkışmayı
%96–100 "gerçekleşti" sayıyor; sıkışmayı yukarı iten her sıralayıcı "sonradan gerçekleşen"
ölçüsünde kazanıyor. Bu yüzden karar **bütünleşik yeniden koşuma** göre verildi (tema
`kinddir`, sonuç `barrier`; `raw-chosen/`); başvuru yapılandırmasındaki sonuçlar `raw/`'da.

## Bütünleşik sonuçlar (kinddir + barrier)

Sentetik 30 × 500 × 3 tohum, havuz; seviye `important` (kademe 1):

| Ölçü | başvuru | rarity | ledger | confirm |
|---|---:|---:|---:|---:|
| Günlük ort. / medyan / p90 / en çok | 1,57 / 1 / 3 / 7 | 1,07 / 1 / 3 / 5 | 1,00 / 0 / 4 / 13 | 1,79 / 2 / 4 / 8 |
| Sıfır gün payı | 0,196 | 0,364 | 0,687 | **0,150** |
| Değişim katsayısı | 0,78 | 1,00 | 1,91 | **0,74** |
| Rejim oranı (en yüksek/en düşük) | 3,35 | **1,42** | 2,38 | 2,82 |
| Tohumlar arası (s2 `important`) | 1,55 | 1,07 | **0,09** (çöküş) | 1,92 |
| 5 gün içinde aynı enstrümanda tekrar | 0,190 | **0,186** | 0,450 | 0,222 |
| K1 − K3 gerçekleşen payı (taban 0,460) | 0,019 | −0,015 | 0,027 | 0,018 |
| `balanced` günlük ort. | 4,60 | 5,29 | 6,51 | 3,78 |

Gerçek veri (6 sembol; toplu):

| Ölçü | başvuru | rarity | ledger | confirm |
|---|---:|---:|---:|---:|
| Günlük: `important`/gün | 0,05 | 0,18 | 0,01 | 0,09 |
| Günlük: K1 − K3 (K1 kapanan) | 0,033 (105) | 0,015 (388) | — (K3 yok) | **0,088** (200) |
| Saatlik: `important`/gün | 0,84 | 1,32 | 0,01 | 1,19 |
| Saatlik: K1 − K3 | −0,026 | −0,008 | — | 0,000 |
| İleriye bakma (kademeler) | geçti | geçti | geçti | geçti |

## Seçim: **M3c — teyit merkezli birleşim**

- Dikkat bütçesinin en düzgünü: en düşük değişim katsayısı ve en az sıfır gün; tohumlar ve
  rejimler arasında kararlı (rarity rejimde daha kararlı ama günlerin %36'sı boş).
- Gerçek günlük veride gerçekleşen payını kademe 1'de belirgin yükselten tek kural (+0,088,
  200 kapanış; küçük örneklem); saatlikte fark yok. Sentetik veride hiçbir kural bilgi
  taşımıyor (beklenen; veri öngörülemez).
- K30 ile tutarlı: önem önce bağımsız aile sayısından; nadirlik ve karne ikincil terim olarak
  içinde.
- `ledger` soğuk başlangıçta çöküyor (gerçek veride ~2 yıl kademe 1 yok; s2'de 0,09/gün).

## Sınırlar

- Gerçek veride akış ailesi yok (kurumsal akış lisanslı, yerelde yok); teyit merkezli kuralın
  asıl gücü (çok aileli hikâye) gerçek veride sınanamadı.
- `RankCtx` yakın geçmiş (son aynı tür hikâyeden beri bar) ve aynı bardaki pazar genişliği
  taşımıyor; tekrarlar bastırılamıyor (öneri).
- Kademe eşikleri bar cinsinden; saatlik seri günlüğün ~8 katı bütçe alıyor (öneri: periyot).
- Karne anahtarında yön yok (gerçek günlükte kırılım yukarı ~0,60, aşağı ~0,40; öneri).
