# M17 — Kapsül dili ölçümü: sonuçlar

Ölçen: R5 `m17-measure` alt ajanı (dal `r5/m17-measure`), 2026-11-18. Düzenek ve seçim kuralı
alternatiflerden önce yazılmış olan `harness/lang/SPEC.md`'dir; bu ölçümde **değiştirilmedi** (§8 ek yok).
Ölçülen diller: `selvi`, `tunca`, `marla` (üç aday; dallar `r5/m17-selvi` e544d28, `r5/m17-tunca` 0a37561,
`r5/m17-marla` 9b4e60d, `main` c414e44 üzerine birleştirildi), başvuru `r4` (sıralanır, seçilmez) ve
gizli `bozuk`.

> **Ajan insan değildir; bu ölçümler ön elemedir.** İlk iki aday gösterimde seçilebilir kalır; son söz
> Arkin'in telefonu ve insan testi v3'tür (SPEC §0.4).

## Özet

| | `selvi` | `tunca` | `marla` | `r4` | `bozuk` |
|---|---:|---:|---:|---:|---:|
| 13 tarama toplamı (531 × 2) | **0** | **0** | **0** | 9266 | 34006 |
| birincil (haiku) | 0,898 [0,866; 0,924] | 0,914 [0,883; 0,937] | 0,889 [0,856; 0,915] | 0,869 [0,834; 0,898] | 0,500 [0,453; 0,547] |
| eleştirmen BT log güç (haiku) | 1,685 [1,411; 2,012] | 1,345 [1,110; 1,602] | 1,409 [1,141; 1,723] | −0,750 [−1,102; −0,465] | −3,687 [−4,069; −3,321] |
| başlık+ayrıntı ort. karakter (TR+EN) | 234,9 | 236,0 | **222,4** | 258,5 | 443,8 |
| tavsiye okuması (al/sat seçimi) | 0 | 0 | 0 | 0 | 0 |

- Üç aday da sert eşikleri geçiyor (bütün taramalar 0).
- **Anlama doğruluğunda üç aday ayırt edilemedi** (aralıklar ikişer ikişer örtüşüyor). Başvuru `r4` de
  birincilde adaylardan ayırt edilemedi (`r4` üst sınırı 0,898 ≥ adayların alt sınırları).
- **Eleştirmen BT'de üç aday ayırt edilemedi** (aralıklar örtüşüyor; üç aday çiftinin hepsinde çelişki
  > %30). Üç aday BT'de `r4`'ün ayrık biçimde önünde.
- `bozuk` birincilde ve BT'de **son**, aralığı ötekilerden ayrık → **düzenek geçerli** (SPEC §5.6).
- Seçim kuralı 4. adıma (kısa olan) düştü → bütünleştirmeye girecek varsayılan: **`marla`**.
  Gösterimde seçilebilir ilk iki: **`marla`** ve **`selvi`** (§6; yorum belirsizliği açıkça yazılı).
- Maliyet: haiku tam koşu **0,6655 $** + sonnet alt küme **2,8049 $** = **3,4704 $** (≤ 10 $).

## 1. Ne koşuldu (SPEC'e göre)

| | değer |
|---|---|
| Döküm | 531 durum × 2 dil (TR, EN), her dil için `langh dump --lang <ad>` |
| Okuyucu planı | `langh plan --run runs/m17 --langs r4,bozuk,selvi,tunca,marla --readers 3 --reps 3 --seed 20261118 --model haiku` |
| Okuyucular | 5 dil × 2 persona (F, H) × 2 koşul (`yuz`, `acik`) × 2 sayfa × 3 okuyucu = **120** koşu; S12 kümesi (12 durum) |
| Eleştirmen | C(5,2) = 10 çift × 3 tekrar × 2 sıra = **60** koşu (12 durum, koşul `acik`) |
| Model | `haiku` → `claude-haiku-5-5` (cevaplardaki `modelUsage`'den doğrulandı) |
| Tohum | plan **20261118**, önyükleme (BT, 1000 kez) **20261118** (`score --seed 20261118`) |
| Körlük kodları | `01b4c7` tunca, `0e4d81` selvi, `45b8e1` r4, `65a09e` marla, `eafe88` bozuk (`runs/m17/key.json`; okuyucuya gitmedi) |
| İkinci boy | `runs/m17-sonnet`: aynı tohum, `--readers 1 --reps 1 --model sonnet` → 40 okuyucu + 20 eleştirmen; `claude-sonnet-5-5` |
| Çözümlenemeyen cevap | 0 / 180 (haiku), 0 / 60 (sonnet); başarısız çağrı 0 |

**Körlük denetimi (kayıt).** `run_readers.sh` her istemi göndermeden önce `langh check-prompt` ile
yasak listeye (bütün kayıtlı dil adları dahil `selvi tunca marla r4 bozuk`; `eski başvuru referans
"yeni dil" "bugünkü dil" …`) bakar. Ayrıca koşudan önce elle:

```
$ for p in runs/m17/readers/*/prompt.txt runs/m17/critics/*/prompt.txt; do langh check-prompt $p || bad=$((bad+1)); done
checked 180 prompts, forbidden hits in 0
$ grep -liE 'selvi|tunca|marla|bozuk|\br4\b|\beski\b|yeni dil|başvuru' runs/m17/*/*/prompt.txt | wc -l
0
$ DRY_RUN=1 ./run_readers.sh runs/m17        → done: 0 answered (çıkış 0; hiçbir istem reddedilmedi)
```

Sonnet planının 60 istemi haiku planındaki karşılıklarıyla (okuyucu 1, tekrar 1) **bayt bayt aynı**
(`cmp`: 60 aynı, 0 farklı); aynı denetimden geçti.

## 2. Deterministik taramalar (13; 531 durum × 2 dil)

| dil | vague | advice | repeat | internal | unglossed | length | threshold | ledger_expect | contradiction | equivalence | redacted_digit | determinism | empty |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `selvi` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| `tunca` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| `marla` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| `r4` | 756 | 0 | 76 | 2344 | 1557 | 1294 | 1002 | 1581 | 636 | 20 | 0 | 0 | 0 |
| `bozuk` | 7681 | 0 | 0 | 15088 | 2414 | 3184 | 1854 | 3375 | 396 | 14 | 0 | 0 | 0 |

Birleşik ağaçta yeniden üretildi (`out/<ad>/scan.json`, izlenmez). `r4` ve `bozuk` SPEC §2'deki başvuru
satırlarıyla aynı; adayların yeniden üretilen `scan.json` ve `TABLE.md` dosyaları dallarında commit
edilmiş olanlarla bayt eşit (`M17/selvi/`, `M17/marla/`, `harness/lang/results/tunca/`).

Ortalama karakter (TR+EN, 531 durum): başlık / ayrıntı / karne — `selvi` 72,4 / 162,5 / 62,6;
`tunca` 66,6 / 169,4 / 55,8; `marla` 71,5 / 150,9 / 60,5; `r4` 59,7 / 198,8 / 27,7; `bozuk` 145,4 / 298,4 / 63,5.

## 3. Anlama doğruluğu

### 3.1 Birincil (bütün puanlanan sorular, iki koşul; F 2, H 1; Wilson %95 persona ağırlıklı)

| haiku sırası | dil | haiku (120 koşu) | sonnet (40 koşu, alt küme) |
|---:|---|---|---|
| 1 | `tunca` | 0,914 [0,883; 0,937] | 0,927 [0,872; 0,960] |
| 2 | `selvi` | 0,898 [0,866; 0,924] | 0,911 [0,853; 0,947] |
| 3 | `marla` | 0,889 [0,856; 0,915] | 0,930 [0,875; 0,961] |
| 4 | `r4` | 0,869 [0,834; 0,898] | 0,847 [0,779; 0,897] |
| 5 | `bozuk` | 0,500 [0,453; 0,547] | 0,495 [0,414; 0,577] |

Haiku'da `tunca`–`selvi`, `tunca`–`marla`, `selvi`–`marla` aralıkları örtüşür → **ayırt edilemedi**.
Sonnet'te sıra `marla` > `tunca` > `selvi` (yine örtüşüyor); iki modelde sıranın değişmesi farkın gürültü
düzeyinde olduğunu destekliyor.

### 3.2 Soru × persona × koşul (haiku; oran [Wilson %95]; n = 36, S5'te 33)

| dil | persona | koşul | S1 | S2 | S3 | S4 | S5 | S6 |
|---|---|---|---|---|---|---|---|---|
| `selvi` | F | yuz | 0.69 [0.53; 0.82] | 0.67 [0.50; 0.80] | 1.00 [0.90; 1.00] | 0.97 [0.86; 1.00] | 0.94 [0.80; 0.98] | 0.97 [0.86; 1.00] |
| `selvi` | F | acik | 0.69 [0.53; 0.82] | 0.94 [0.82; 0.98] | 1.00 [0.90; 1.00] | 1.00 [0.90; 1.00] | 0.94 [0.80; 0.98] | 1.00 [0.90; 1.00] |
| `selvi` | H | yuz | 0.64 [0.48; 0.78] | 0.67 [0.50; 0.80] | 1.00 [0.90; 1.00] | 1.00 [0.90; 1.00] | 0.91 [0.76; 0.97] | 1.00 [0.90; 1.00] |
| `selvi` | H | acik | 0.69 [0.53; 0.82] | 0.92 [0.78; 0.97] | 0.97 [0.86; 1.00] | 1.00 [0.90; 1.00] | 0.91 [0.76; 0.97] | 1.00 [0.90; 1.00] |
| `tunca` | F | yuz | 0.94 [0.82; 0.98] | 0.44 [0.30; 0.60] | 1.00 [0.90; 1.00] | 1.00 [0.90; 1.00] | 0.94 [0.80; 0.98] | 1.00 [0.90; 1.00] |
| `tunca` | F | acik | 0.69 [0.53; 0.82] | 1.00 [0.90; 1.00] | 1.00 [0.90; 1.00] | 1.00 [0.90; 1.00] | 1.00 [0.90; 1.00] | 1.00 [0.90; 1.00] |
| `tunca` | H | yuz | 0.94 [0.82; 0.98] | 0.39 [0.25; 0.55] | 1.00 [0.90; 1.00] | 0.97 [0.86; 1.00] | 0.97 [0.85; 0.99] | 1.00 [0.90; 1.00] |
| `tunca` | H | acik | 0.75 [0.59; 0.86] | 0.89 [0.75; 0.96] | 1.00 [0.90; 1.00] | 1.00 [0.90; 1.00] | 0.97 [0.85; 0.99] | 1.00 [0.90; 1.00] |
| `marla` | F | yuz | 0.83 [0.68; 0.92] | 0.50 [0.34; 0.66] | 0.97 [0.86; 1.00] | 0.97 [0.86; 1.00] | 0.91 [0.76; 0.97] | 1.00 [0.90; 1.00] |
| `marla` | F | acik | 0.78 [0.62; 0.88] | 0.86 [0.71; 0.94] | 1.00 [0.90; 1.00] | 0.92 [0.78; 0.97] | 0.94 [0.80; 0.98] | 1.00 [0.90; 1.00] |
| `marla` | H | yuz | 0.78 [0.62; 0.88] | 0.47 [0.32; 0.63] | 0.97 [0.86; 1.00] | 0.97 [0.86; 1.00] | 0.91 [0.76; 0.97] | 1.00 [0.90; 1.00] |
| `marla` | H | acik | 0.75 [0.59; 0.86] | 0.92 [0.78; 0.97] | 1.00 [0.90; 1.00] | 0.94 [0.82; 0.98] | 0.94 [0.80; 0.98] | 1.00 [0.90; 1.00] |
| `r4` | F | yuz | 1.00 [0.90; 1.00] | 0.08 [0.03; 0.22] | 1.00 [0.90; 1.00] | 1.00 [0.90; 1.00] | 0.97 [0.85; 0.99] | 1.00 [0.90; 1.00] |
| `r4` | F | acik | 1.00 [0.90; 1.00] | 0.64 [0.48; 0.78] | 0.94 [0.82; 0.98] | 0.94 [0.82; 0.98] | 0.91 [0.76; 0.97] | 1.00 [0.90; 1.00] |
| `r4` | H | yuz | 1.00 [0.90; 1.00] | 0.08 [0.03; 0.22] | 0.86 [0.71; 0.94] | 1.00 [0.90; 1.00] | 0.94 [0.80; 0.98] | 1.00 [0.90; 1.00] |
| `r4` | H | acik | 1.00 [0.90; 1.00] | 0.67 [0.50; 0.80] | 0.94 [0.82; 0.98] | 0.94 [0.82; 0.98] | 0.91 [0.76; 0.97] | 1.00 [0.90; 1.00] |
| `bozuk` | F | yuz | 1.00 [0.90; 1.00] | 0.08 [0.03; 0.22] | 0.00 [0.00; 0.10] | 0.39 [0.25; 0.55] | 0.00 [0.00; 0.10] | 1.00 [0.90; 1.00] |
| `bozuk` | F | acik | 0.89 [0.75; 0.96] | 0.08 [0.03; 0.22] | 0.72 [0.56; 0.84] | 0.72 [0.56; 0.84] | 0.00 [0.00; 0.10] | 1.00 [0.90; 1.00] |
| `bozuk` | H | yuz | 0.97 [0.86; 1.00] | 0.08 [0.03; 0.22] | 0.00 [0.00; 0.10] | 0.42 [0.27; 0.58] | 0.00 [0.00; 0.10] | 1.00 [0.90; 1.00] |
| `bozuk` | H | acik | 0.89 [0.75; 0.96] | 0.08 [0.03; 0.22] | 0.69 [0.53; 0.82] | 0.81 [0.65; 0.90] | 0.03 [0.01; 0.15] | 1.00 [0.90; 1.00] |

Şans düzeyi: S1 0,09, S2 ≈ 0, S3 0,20, S4 0,17, S5 ≈ 0, S6 0,25. Ham k/n tabloları:
`harness/lang/runs/m17/RESULTS.md`.

**Okuma.**
- Adayların `r4`'e göre kazancı **S2** (eşik adı ve değeri): `yuz`'da `r4` 0,08 → adaylar 0,39–0,67 (eşik
  değeri artık bildirimde görünen kısımda); `acik`'ta `r4` 0,64–0,67 → adaylar 0,86–1,00.
- Adayların `r4`'e göre kaybı **S1** (ne oldu): `r4` 1,00, adaylar 0,64–0,94. Hata dökümü
  (`M17/analysis/s1.py`): üç adayda da **ortalama dizilimi aşağı** (s195, s432; doğru "Düşüş sürüyor")
  okuyucuların çoğunda "Fiyat bir sınırın altına indi" diye okunuyor ("20 günlük ortalamasının altına
  yerleşti", "ortalamanın altında düşüşe geçti", "düşüş eğilimine girdi"); **hacim** (s239) "bilmiyorum"
  ya da "çok yükseldi"; `selvi`'de ayrıca **RSI aşırılık** (s190) "dönüş / sınırın altına indi"; `tunca`'da
  **uyumsuzluk** (s202) yarı yarıya "düşüş sürüyor". `r4`'ün S1 tavanı olayı düz adlandırmasından
  ("düşüşünü sürdürdü"). Bu bir dil tasarımı geri bildirimidir (§10).
- S3, S4, S6 adaylarda tavana yakın; tavsiye okuması hiçbir dilde yok (her dilde 0 / 144).
- `yuz`'da S2 `r4` ve `bozuk` 3/36: S12'deki yetkisiz okur maddesi (rakam yoksa doğru sayılır).

### 3.3 Sonnet alt kümesi (oran [Wilson %95]; n = 12, S5'te 11)

| dil | persona | koşul | S1 | S2 | S3 | S4 | S5 | S6 |
|---|---|---|---|---|---|---|---|---|
| `selvi` | F | yuz | 0.75 [0.47; 0.91] | 0.58 [0.32; 0.81] | 1.00 [0.76; 1.00] | 1.00 [0.76; 1.00] | 1.00 [0.74; 1.00] | 1.00 [0.76; 1.00] |
| `selvi` | F | acik | 0.75 [0.47; 0.91] | 0.92 [0.65; 0.99] | 1.00 [0.76; 1.00] | 1.00 [0.76; 1.00] | 1.00 [0.74; 1.00] | 1.00 [0.76; 1.00] |
| `selvi` | H | yuz | 0.75 [0.47; 0.91] | 0.58 [0.32; 0.81] | 1.00 [0.76; 1.00] | 1.00 [0.76; 1.00] | 0.91 [0.62; 0.98] | 1.00 [0.76; 1.00] |
| `selvi` | H | acik | 0.92 [0.65; 0.99] | 0.75 [0.47; 0.91] | 1.00 [0.76; 1.00] | 1.00 [0.76; 1.00] | 0.91 [0.62; 0.98] | 1.00 [0.76; 1.00] |
| `tunca` | F | yuz | 0.92 [0.65; 0.99] | 0.42 [0.19; 0.68] | 1.00 [0.76; 1.00] | 1.00 [0.76; 1.00] | 0.91 [0.62; 0.98] | 1.00 [0.76; 1.00] |
| `tunca` | F | acik | 0.92 [0.65; 0.99] | 1.00 [0.76; 1.00] | 1.00 [0.76; 1.00] | 1.00 [0.76; 1.00] | 0.91 [0.62; 0.98] | 1.00 [0.76; 1.00] |
| `tunca` | H | yuz | 1.00 [0.76; 1.00] | 0.50 [0.25; 0.75] | 1.00 [0.76; 1.00] | 1.00 [0.76; 1.00] | 0.91 [0.62; 0.98] | 1.00 [0.76; 1.00] |
| `tunca` | H | acik | 0.92 [0.65; 0.99] | 1.00 [0.76; 1.00] | 1.00 [0.76; 1.00] | 1.00 [0.76; 1.00] | 0.91 [0.62; 0.98] | 1.00 [0.76; 1.00] |
| `marla` | F | yuz | 0.92 [0.65; 0.99] | 0.58 [0.32; 0.81] | 1.00 [0.76; 1.00] | 1.00 [0.76; 1.00] | 0.91 [0.62; 0.98] | 1.00 [0.76; 1.00] |
| `marla` | F | acik | 0.83 [0.55; 0.95] | 1.00 [0.76; 1.00] | 1.00 [0.76; 1.00] | 1.00 [0.76; 1.00] | 0.91 [0.62; 0.98] | 1.00 [0.76; 1.00] |
| `marla` | H | yuz | 0.83 [0.55; 0.95] | 0.58 [0.32; 0.81] | 1.00 [0.76; 1.00] | 1.00 [0.76; 1.00] | 0.91 [0.62; 0.98] | 1.00 [0.76; 1.00] |
| `marla` | H | acik | 0.83 [0.55; 0.95] | 1.00 [0.76; 1.00] | 1.00 [0.76; 1.00] | 1.00 [0.76; 1.00] | 1.00 [0.74; 1.00] | 1.00 [0.76; 1.00] |
| `r4` | F | yuz | 1.00 [0.76; 1.00] | 0.08 [0.01; 0.35] | 0.75 [0.47; 0.91] | 1.00 [0.76; 1.00] | 0.91 [0.62; 0.98] | 1.00 [0.76; 1.00] |
| `r4` | F | acik | 1.00 [0.76; 1.00] | 0.58 [0.32; 0.81] | 1.00 [0.76; 1.00] | 0.92 [0.65; 0.99] | 0.91 [0.62; 0.98] | 1.00 [0.76; 1.00] |
| `r4` | H | yuz | 1.00 [0.76; 1.00] | 0.08 [0.01; 0.35] | 0.75 [0.47; 0.91] | 1.00 [0.76; 1.00] | 0.91 [0.62; 0.98] | 1.00 [0.76; 1.00] |
| `r4` | H | acik | 1.00 [0.76; 1.00] | 0.67 [0.39; 0.86] | 1.00 [0.76; 1.00] | 0.92 [0.65; 0.99] | 0.91 [0.62; 0.98] | 1.00 [0.76; 1.00] |
| `bozuk` | F | yuz | 1.00 [0.76; 1.00] | 0.08 [0.01; 0.35] | 0.00 [0.00; 0.24] | 0.33 [0.14; 0.61] | 0.00 [0.00; 0.26] | 1.00 [0.76; 1.00] |
| `bozuk` | F | acik | 0.83 [0.55; 0.95] | 0.08 [0.01; 0.35] | 0.75 [0.47; 0.91] | 0.75 [0.47; 0.91] | 0.00 [0.00; 0.26] | 1.00 [0.76; 1.00] |
| `bozuk` | H | yuz | 0.92 [0.65; 0.99] | 0.08 [0.01; 0.35] | 0.00 [0.00; 0.24] | 0.58 [0.32; 0.81] | 0.00 [0.00; 0.26] | 1.00 [0.76; 1.00] |
| `bozuk` | H | acik | 0.83 [0.55; 0.95] | 0.08 [0.01; 0.35] | 0.75 [0.47; 0.91] | 0.67 [0.39; 0.86] | 0.00 [0.00; 0.26] | 1.00 [0.76; 1.00] |

## 4. Kör eleştirmen (Bradley–Terry, önyükleme 1000, tohum 20261118)

| dil | haiku log güç [%95] | sonnet log güç [%95] |
|---|---|---|
| `selvi` | 1,685 [1,411; 2,012] | 2,070 [1,626; 2,510] |
| `marla` | 1,409 [1,141; 1,723] | 1,959 [1,620; 2,353] |
| `tunca` | 1,345 [1,110; 1,602] | 0,572 [0,200; 0,923] |
| `r4` | −0,750 [−1,102; −0,465] | −1,319 [−1,624; −1,035] |
| `bozuk` | −3,687 [−4,069; −3,321] | −3,282 [−3,575; −2,922] |

**Çelişki oranı** (birim = çift × tekrar × durum; iki sırada farklı kazanan; > %30 → çift ayırt edilemedi):

| çift | haiku | sonnet |
|---|---:|---:|
| `tunca`–`marla` | 16/36 = 0,44 → ayırt edilemedi | 4/12 = 0,33 → ayırt edilemedi |
| `selvi`–`tunca` | 14/36 = 0,39 → ayırt edilemedi | 0/12 = 0,00 |
| `selvi`–`marla` | 13/36 = 0,36 → ayırt edilemedi | 6/12 = 0,50 → ayırt edilemedi |
| `r4`–`tunca` | 0/36 = 0,00 | 0/12 |
| `r4`–`selvi` | 6/36 = 0,17 | 0/12 |
| `r4`–`marla` | 3/36 = 0,08 | 0/12 |
| `r4`–`bozuk` | 1/36 = 0,03 | 0/12 |
| `bozuk`–her aday | 0/36 | 0/12 |

Haiku'da üç aday çiftinin üçünde de çelişki > %30: eleştirmen adaylar arasında sıra etkisinin ötesinde
bir tercih göstermiyor. Adaylar ↔ `r4` ve herkes ↔ `bozuk` kararlı.

**Uzunluk yanlılığı (U2; `meta.lens`, eleştirmenin gördüğü metin uzunluğu).** Kararlı birimlerde uzun
metin kazandı: haiku **216/307 = 0,704**, korelasyon −0,001; sonnet 77/110 = 0,700, korelasyon −0,043.
Yalnız aday çiftlerinde (`M17/analysis/len.py`): haiku **28/65 = 0,431**, sonnet 10/26 = 0,385 → adaylar
arasında uzunluk yanlılığı görülmüyor. Bütün çiftlerdeki 0,70, eleştirmen metninde (sözlük dahil)
adayların `r4`'ten uzun olmasından ve neredeyse hep kazanmasından geliyor; `r4`↔aday farkının bir kısmı
uzunluk tercihi olabilir — tercih ≠ performans (U9), BT seçimde ikincildir.

## 5. haiku ↔ sonnet uyumu

Aynı 60 istem (okuyucu 1, tekrar 1) iki modele gitti (`M17/analysis/s1.py`):

| ölçü | uyum |
|---|---:|
| okuyucu harf cevabı S1 | 220/240 = 0,917 |
| S3 | 224/240 = 0,933 |
| S4 | 232/240 = 0,967 |
| S6 | 240/240 = 1,000 |
| eleştirmen seçimi (aynı istem, aynı madde), bütün çiftler | 209/240 = 0,871 |
| eleştirmen seçimi, yalnız üç aday arasındaki çiftler | 47/72 = 0,653 |

İki model **büyük resimde aynı**: `bozuk` son, `r4` dördüncü, üç aday birincilde örtüşüyor, BT'de adaylar
`r4`'ün önünde. Adaylar arasındaki ince sırada **uyuşmuyorlar** (birincil: haiku `tunca`, sonnet `marla`
önde; BT: sonnet `tunca`'yı ayrık biçimde üçüncü koyuyor, haiku koymuyor; aday çiftlerinde seçim uyumu
0,65, yazı tura 0,50). Bu da "ayırt edilemedi" sonucunu destekliyor. Seçim SPEC varsayılanı olan haiku tam
koşusuyla yapılır; sonnet yalnız ikinci okuyucudur.

## 6. Seçim kuralı (SPEC §5) adım adım

0. **Geçerlilik (§5.6):** `bozuk` birincilde son (0,500 [0,453; 0,547]; en yakın `r4` [0,834; 0,898] ile
   ayrık) ve BT'de son (ayrık) → **düzenek geçerli**, seçim yapılabilir. Sonnet'te de aynı.
1. **Sert eşikler (§5.1):** `selvi`, `tunca`, `marla` bütün dökümde 13 taramada 0 → üçü de geçer.
   `r4` sıralanır, seçilmez.
2. **Birincil (§5.2):** ilk iki `tunca` 0,914 [0,883; 0,937] ↔ `selvi` 0,898 [0,866; 0,924] → örtüşüyor →
   **ayırt edilemedi**. (`marla` 0,889 [0,856; 0,915] de `tunca` ile örtüşüyor: üç aday birbirinden ayırt
   edilemedi.)
3. **Eleştirmen BT (§5.3):** ilk iki `selvi` [1,411; 2,012] ↔ `marla` [1,141; 1,723] → örtüşüyor; ayrıca
   çelişki 0,36 > 0,30 → **ayırt edilemedi**. (`selvi`↔`tunca` [1,110; 1,602] de örtüşüyor, çelişki 0,39.)
4. **Kısa olan (§5.4):** TR+EN ortalama (başlık + ayrıntı) karakter: `marla` **222,4**, `selvi` 234,9,
   `tunca` 236,0 → **varsayılan: `marla`**.
5. **İlk iki (§5.5), gösterimde seçilebilir:** **`marla`** ve **`selvi`**.

**Yorum notu (açık soru; orkestratörün onayı gerekir).** §5.4 "kısa olan"ın hangi adaylar arasından
seçileceğini adlandırmıyor. Uygulanan yorum: 2. ve 3. adımlarda **birbirinden ayırt edilemeyen bütün
adaylar** (burada üçü) arasında kısa olan — çünkü SPEC §3.7 ve §4.3 örtüşen aralıklarda nokta tahminiyle
kazanan ilan etmeyi yasaklıyor; `marla`'yı yalnız nokta tahmini üçüncü diye dışarıda bırakmak bunu yapmak
olurdu. Diğer okumalar:
- "Birincilin ilk ikisi arasında kısa olan" → `selvi` (234,9) ↔ `tunca` (236,0): **`selvi`** (fark 1,1 karakter).
- "BT'nin ilk ikisi arasında kısa olan" → `selvi` ↔ `marla`: **`marla`**.

`selvi` her okumada ilk ikidedir; `marla` üç okumanın ikisinde varsayılandır. Üç aday da anlama ölçüsünde
ve eleştirmende ayırt edilemediği için galeriye üçünün de konması bu ölçümle çelişmez.

**Başvuruya göre.** Adaylar `r4`'ten birincilde ayrık biçimde iyi **değil** (aralıklar örtüşüyor; S2'deki
kazanç ile S1'deki kayıp birbirini kısmen götürüyor); taramalarda (9266 → 0) ve eleştirmen BT'de ayrık
biçimde iyiler.

## 7. Maliyet

| koşu | koşu sayısı | model | maliyet |
|---|---:|---|---:|
| `runs/m17` (tam ölçüm) | 180 | claude-haiku-5-5 | 0,6655 $ |
| `runs/m17-sonnet` (alt küme) | 60 | claude-sonnet-5-5 | 2,8049 $ |
| **toplam** | 240 | | **3,4704 $** |

Haiku koşu başına ≈ 0,0037 $, sonnet ≈ 0,047 $ (≈ 13×). Taramalar ücretsiz. Haiku 5, sonnet 3 eşzamanlı
süreçle, aynı anda koşuldu (süre ölçülmedi).

## 8. Ham kayıtlar (commit edildi)

- `harness/lang/runs/m17/` — 120 okuyucu + 60 eleştirmen dizini (`prompt.txt`, `answer.json`, `meta.json`),
  `key.json` (kod → ad), `set.json` (S12), `scores.json`, `RESULTS.md`; **3,3 MB**.
- `harness/lang/runs/m17-sonnet/` — aynı biçim, 60 dizin; **1,1 MB**.
- `M17/analysis/` — `s1.py` (S1 hata dökümü + haiku↔sonnet uyumu), `len.py` (aday çiftlerinde uzunluk),
  `wilson.py` (hücre Wilson aralıkları); yalnız commit edilen ham kayıtları ve `out/r4/dump.json`'u okur.
- Döküm (`out/<ad>/dump.json`, `findings.json`) izlenmez; `langh dump --lang <ad>` ile bayt bayt yeniden
  üretilir.

Yeniden üretmek: `harness/lang/README.md` komutları; `langh score --run runs/m17 --seed 20261118`.

## 9. Sınırlar ve sorunlar

- Tek model ailesi (Claude); sonnet yalnız alt küme (1 okuyucu, 1 tekrar). Sıcaklık ayarlanamıyor.
- Tavan: S3, S4, S6 adaylarda ≈ 1,00; adaylar arası farkın tamamı S1, S2, S5'ten. 3 okuyucuyla birincil
  aralık ≈ ±0,03; adaylar arası nokta farkları (≤ 0,025) bunun içinde.
- §5.4'ün aday kümesi belirsiz (§6 yorum notu). SPEC değiştirilmedi; karar orkestratörde.
- Ölçüm sırasında `main` ilerledi (c414e44 → 0e85cb6); bu dal c414e44 tabanlıdır.

## 10. Öneriler (kapsam dışı; yapılmadı)

- **S1 (olay adı):** üç adayda ortalama dizilimi aşağı "sınırın altına indi" okunuyor. Başlıkta olay
  fiilini önce ve düz söylemek ("düşüş sürüyor"; RESEARCH F1 "önden yüklü başlık") ve hacim olayında
  "alışılmadık alım" demek S1'i `r4` düzeyine çıkarabilir. Bütünleştirmede seçilen dile uygulanırsa
  yeniden ölçülmeli.
- SPEC §5.4'e tarihli ek: "kısa olan, 2.–3. adımda lidere göre ayırt edilemeyen bütün adaylar arasından"
  (ya da hangisi isteniyorsa) — sonraki ölçümler için belirsizliği kapatır.
- Aday çiftlerini ayırmak için okuyucu sayısı artırılabilir (3 → 9 ≈ 2 $ haiku ile), ama eleştirmen
  çelişkisi %36–44 olduğundan insan testi (v3) daha bilgilendirici olur.
