# Başvuru ajanları (R1–R3) — sentetik ölçüm

Düzenek: `agents/` (rol talimatları, `tasks.json`, motordan doğru cevaplar `expected.json`,
`run.py`, `score.py`). Yüzey `thin`, kapsül biçimi `card` (sürücünün varsayılanı; M29 sonucu
`json` olarak geldi, sonraki ölçümler için not). Sentetik R5B oturumu; `claude` = Arkın
adına, `claude-guest` = yetkisiz konuk adına. Her model bir Claude Code alt ajanı.
Puanlama deterministik: doğruluk motor cevabına göre; her yorum, cevap ve bildirim taze bir
denetçi sunucuda motorun **kendi doğrulayıcısından** yeniden geçirilir (`checks.jsonl`).

| koşu | n | doğru | atıf oranı | uydurma sayı* | tavsiye dili | sızıntı | ort. çağrı | ort. belirteç/görev | yorum ilk denemede / düzeltmeyle | kaçırılan zorunlu olay | gereksiz bildirim |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|---:|---:|
| yorumcu-haiku | 8 | 8 | 1,00 | 0 | 0 | 0 | 2,6 | 10 799 | 7 / 1 | — | — |
| yorumcu-sonnet | 8 | 8 | 1,00 | 0 | 0 | 0 | 2,4 | 9 085 | 7 / 1 | — | — |
| yorumcu-opus | 8 | 8 | 1,00 | 0 | 0 | 0 | 3,4 | 10 291 | 7 / 1 | — | — |
| soru-haiku | 7 | 7 | 1,00 | 10 | 0 | 0 | 1,7 | 12 902 | — | — | — |
| soru-sonnet | 7 | 7 | 1,00 | 2 | 0 | 0 | 3,7 | 12 355 | — | — | — |
| soru-opus | 7 | 6 | 1,00 | 3 | 0 | 0 | 4,7 | 12 468 | — | — | — |
| nobetci-haiku | 4 | 4 | 1,00 | 0 | 0 | 0 | 1,3 | 29 416 | — | 0 | 0 |
| nobetci-sonnet | 4 | 4 | 1,00 | 0 | 0 | 0 | 1,3 | 28 416 | — | 0 | 0 |
| nobetci-opus | 4 | 4 | 1,00 | 0 | 0 | 0 | 0,8 | 29 484 | — | 0 | 0 |

Ajan karar denemesi (accept/reject) 0; replay denetimi denemesi 0 (bütün koşularda).
Belirteç toplamı: Yorumcu 241 398, Soru 264 071, Nöbetçi 349 265 (iki evre) = 854 734.

**\* "Uydurma sayı" (yalnız Soru cevaplarında; doğrulayıcıdan geçen yorumlarda 0):** elle
okundu, hiçbiri uydurma değil:
- haiku'nun 10 sayısı **bar numaraları** ("bar 158'de doğdu", "149–158. barlar"): kapsülün
  `born_bar`/`last_bar` alanından; doğrulayıcı bu alanları kanıt değeri saymıyor.
- konuk adına S07 cevaplarında 2–3 sayı: konuğa ret nedeni kör söylenir; aynı cümleler ayrı
  ayrı denendiğinde (143,85 / 141,02 / "15 seferin 9'u" / "26 seferin 8'i") konuk için de
  kabul edildi; kalanlar madde numaraları "(1)", "(2)" ve "6 bar önce" gibi bar sayıları.
  Sızıntı denetimi (lisanslı değer kümesi) 0.
→ Öneri: doğrulayıcı kapsülün bar alanlarını ve liste numaralarını tanısın (REPORT "Öneriler").

Nöbetçi: 10 günlük pencere (M31 penceresi) barındırıcı tarafında adımlanır (ajan kimliği
replay'i ilerletemez: `forbidden`); ajanın kurduğu izlemeler (3 görev → 5 izleme) aynı
`add_watch` ile yeniden oynatılır; 17 olay gelen kutusu. Üç model de 8 bildirim yazdı,
zorunlu olayların hiçbirini kaçırmadı, gereksiz bildirim 0.

## Ölçüt 9: Matriks bağlamıyla (gerçek veri, yalnız toplu sayı)

`--source local` (yerel gerçek barlar, `fixtures/local/`), `--matriks`, model sonnet,
Soru (ISCTR, ASELS) ve Yorumcu (ISCTR, THYAO). Dökümler `fixtures/local/agent-runs/`
altında; depoya girmedi.

| koşu | görev | doğru | atıf | Matriks ağ sorgusu | dış sayı `external`'da kaynağıyla | doğrulayıcı |
|---|---:|---:|---:|---:|---|---|
| ilk koşu ("gerekirse") | 4 | 4 | 4/4 | 1 | — (ajanlar bağlam almadı) | 4/4 kabul |
| ikinci koşu ("mutlaka") | 4 | 3 | 4/4 | 5 (+3 önbellekten) | 4/4 cevap/yorum, kaynak adıyla (`Matriks MCP newsAndEvents`, `marketSnapshot`) | yorum 2/2 kabul (biri düzeltmeyle); Soru cevabı 0/2 |

- Toplam Matriks ağ sorgusu (round boyunca): **7** (kurulumda 1 `tools/list`, 3
  `newsAndEvents`, 3 `marketSnapshot`); önbellekten 4. Sayaç: `fixtures/local/agent-cache/matriks/count.jsonl`.
- İlk koşuda görev "gerekirse" diyordu; ajanlar Matriks'i hiç çağırmadı. İkinci koşuda
  görev bağlamı açıkça istiyor (`agents/run.py`).
- Soru cevaplarının ikisi doğrulayıcıda reddedildi: dış değer işaretli yazılmış (`-1,41`)
  ama alıntıda "%1,41 düşüş" — `external_value_not_in_quote`; ve "2018" yılı (tarih maskesi
  yalnız "2018'de"/"2018 yılı" biçimini tanıyor). Uydurma değil; doğrulayıcı kullanılabilirliği
  (REPORT "Öneriler").
- Yorumcu THYAO'da "en önemli açık hikâye" olarak motorun seçtiğinden başka bir açık
  hikâyeyi yorumladı (doğruluk 1/2); yorum doğrulandı.
- Ajanlar replay tarihi (2018) ile Matriks verisinin tarihi (2026) arasındaki farkı her
  cevapta söyledi.
