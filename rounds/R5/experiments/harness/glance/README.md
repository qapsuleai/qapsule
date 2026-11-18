# R5/T2 — tek bakış düzeneği (M16, M18, M20, M21)

Bağlayıcı şartname: [`SPEC.md`](SPEC.md) (§G0–§G11). Kuru koşu sonucu:
[`DRYRUN.md`](DRYRUN.md). Bu klasör yalnız düzenektir; hiçbir alternatif burada yaşamaz.

## Gerekenler

- `qp` (sürüm derlemesi) ve kabuğun WASM paketi: `QP_DEMO_PACK_ONLY=1 scripts/demo.sh`, sonra
  `cargo build --release -p qp-cli` (worktree'de kendi `CARGO_TARGET_DIR`'ın). İkili yolu
  `QP_BIN` ya da `--qp` ile verilir (eşzamanlı derlemeler ortak ikiliyi ezmesin diye bir kopyası
  önerilir).
- `shells/web/node_modules` (Playwright; ana ağaçtakine sembolik bağ yeterli).
- Python 3 + Pillow (numpy gerekmez). `claude` komut satırı (okuyucu/eleştirmen).
- Koşu klasörü (`--run DIR`) depo dışında ya da izlenmeyen bir `raw/` altında olmalı: ekran
  görüntüleri ve kayıtlar depoya girmez.

## Nasıl koşulur

```sh
H=rounds/R5/experiments/harness/glance
RUN=/path/outside/repo/run1            # ya da $H/raw/run1 (izlenmez)
export QP_DEMO_PORT=8461
# Bozuk tasarım bir görünüş kaydıdır ama ürüne girmez (SPEC §G8 Ek G2): yamalı bir kopyada derlenir
# ve o koşunun BÜTÜN adayları bu derlemeyle çekilir (aday dallarının kaydı da bu kopyaya yama
# olarak eklenebilir).
$H/broken/build.sh /path/outside/repo/brokenbuild     # ≈ 5 dk; QP=... WEB=... yazar
export QP_BIN=/path/outside/repo/brokenbuild/qp QP_WEB=/path/outside/repo/brokenbuild/src/shells/web

python3 $H/glance.py probe  --run $RUN             # okurun akışı, 64 adım × 2 okur (~30 s)
python3 $H/glance.py states --run $RUN [--charts]  # durum kümesi → states.json
# designs.json: [{"name": "<ad>", "look": "<look>", "face": "<face>"}, ...,
#                {"name": "kestim", "look": "kestim", "face": "vadrel"}]   # bozuk (gizli ad)
python3 $H/glance.py shoot  --run $RUN --designs designs.json [--langs tr,en] [--themes light,dark] [--voice marla]
# --voice (Ek G4-1; probe, chartstates, shoot, k52): adrese `?voice=`; verilmezse ürünün gösterim
# varsayılanı. M16 her adayda aynı metinle: --voice marla. Kayıtta `voice` {requested, used}.
python3 $H/glance.py derive --run $RUN            # kısma, gri, yarım boy, hareket çiftleri, eleştirmen çiftleri
python3 $H/glance.py compute --run $RUN           # sert eşikler → computed.json + computed.md
python3 $H/glance.py k52    --run $RUN --designs designs.json [--reps 3] [--voice marla]
python3 $H/glance.py read   --run $RUN --model haiku  [--personas F,H] [--reps 3] [--sets ...] [--states ...]
python3 $H/glance.py read   --run $RUN --model sonnet --sets face --lang tr --theme light   # boylar arası uyum
python3 $H/glance.py critic --run $RUN --model sonnet [--reps 3]
python3 $H/glance.py score  --run $RUN --broken <gizli ad>     # scores.json + scores.md
python3 $H/glance.py broken-check --run $RUN --broken <gizli ad>
python3 -m unittest discover -s $H/tests          # deterministik kısımların birim testleri
```

Her aşama yeniden başlatılabilir: okuyucu/eleştirmen çağrıları kod adıyla kaydedilir, var olan
kayıt yeniden çağrılmaz.

## Neyi nerede bulursun (koşu klasörü)

| Yol | İçerik |
|---|---|
| `probe/probe.json`, `states.json` | okurun akışı ve öğeleri (motor verisi); durum kümesi |
| `key.json`, `critic_key.json` | kod ↔ tasarım ve çift ↔ A/B eşlemesi — okuyucuya gitmez |
| `shots/<kod>.png/.json` | 402×874 @1× ekran ve kaydı (motor verisi, DOM, dokunma hedefleri) |
| `stim/` | okuyucuya giden türev görüntüler (kısma, gri, yarım boy, hareket, çift) |
| `readers/<kod>.json`, `critics/<kod>.json` | körlük kaydı: istem, görüntü kimliği, geçici ad, ham cevap, kesilmiş cevap, araçlar, tur, maliyet, model |
| `computed.json/.md`, `scores.json/.md`, `broken-check.json`, `k52/` | sonuçlar |

## Maliyet tahmini (ölçülen birim fiyatlarla)

Kuru koşuda ölçülen ortalama çağrı maliyeti: **haiku okuyucu ≈ 0,0009 $**, **sonnet okuyucu ≈
0,020 $**, **sonnet eleştirmen ≈ 0,0145 $** (`DRYRUN.md`). Süre: haiku ≈ 6–10 s/çağrı, 8 paralel.

Tam M16 koşusu, D aday (4 yön + `r4` + bozuk = 6), 2 dil × 2 tema:

| Kalem | Hesap | Çağrı | ≈ $ |
|---|---|---:|---:|
| Görüntü başına uyaranlar | 8 yüz durumu × (yüz + yarım + kısma + gri) + akış + 2 kanıt + 2 × 2 hareket = 39 | | |
| Okuyucu (haiku) | 6 aday × 4 dil/tema × 2 persona × (8 kısma × **6 tekrar** + 31 öteki × 3 tekrar) (Ek G3) | 6 768 | 6,1 |
| Boylar arası uyum (sonnet) | 6 × yüz kümesi 8 durum × 2 persona × 1 (tr/açık) | 96 | 1,9 |
| Eleştirmen (sonnet) | C(6,2) = 15 çift × 8 durum × 2 sıra × 3 tekrar (tr/açık) | 720 | 10,4 |
| **Toplam M16** | | ≈ 7 580 | **≈ 18,4** |

M18 (3 aday + `r4` + bozuk = 5; 3 derinlik × 2 dil): okuyucu ≈ 5 × 6 × 2 × 3 = 180 haiku (0,2 $) +
eleştirmen 10 çift × 3 × 2 × 3 = 180 sonnet (2,5 $). M20 (2 + `r4` + bozuk): ≈ 1 $. M21 (2 + hareketsiz
+ bozuk): ≈ 1 $. Hepsi birlikte ≈ 26 $ ve birkaç saat duvar saati (eleştirmen en yavaşı).

## Bilinen sınırlar

`SPEC.md` §G0 (ajan insan değildir; tek model ailesi), §G11 (açık riskler) ve `DRYRUN.md`
"Bulgu" bölümü (okuyucu doğruluğu bozuk tasarımı ayırmadı; kural buna göre düzeltildi).
