# M2 aday B — `llmslot`: dil modeli + doğrulanmış yuvalar

**Yalnız karşılaştırma için.** Ürün dil modeli içermez (K3, K21). Bu deney, motorun
doğruladığı yuvalara bir dil modelinin yazmasının kurallı bir cümleciden daha iyi olup
olmayacağını ölçer. Model: **`sonnet`** (`claude -p --model sonnet`; çağrılar
`claude-sonnet-5-5` olarak raporlandı). Diğer aday ve `ALTERNATIVES.md` görülmedi.

## Tasarım

```
SentenceInput (harness m2 inputs) ──► slots.py (deterministik) ──► claude -p (sonnet)
                                                     │                    │
                                                     └──► verify.py ◄─────┘ ret → gerekçeli yeniden üretim (≤ 3 çağrı)
```

1. **Yuvalar (`slots.py`, LLM'siz, saf fonksiyon).** Girdiden: enstrüman, tür + tür terimi
   (TR/EN), yön, zaman türü, **zaman sınıfı** (olay / süren durum / bitmiş durum / beklenti),
   aşama + anlamı, son geçişin nedeni ve kaç bar önce olduğu, `bars_ago`, **izinli zaman
   ifadeleri** (`when_phrases`: `3 bar önce`/`3 bars ago`, 0 → `bugün`/`today`, 1 →
   `dün`/`yesterday`), `valid_now` (kapanmış aşama → hayır, yoksa `still_holds`), koşul
   (özne, ilişki, anahtar, `within_bars`), `redacted`, ve **`allowed_numbers`**: modelin
   yazabileceği her sayı TR (`1.345.000`, `196,96`) ve EN (`1,345,000`, `196.96`) biçiminde
   önceden biçimlenmiş, etiketli. Kaynaklar: baş/son olgunun kanıt değerleri, güncel değer,
   karne n / gerçekleşen / yüzde, `within_bars`, `bars_ago`. Ham `Story` modele verilmez.
   - `redacted` (lisanslı, K33): `allowed_numbers` boş, karne yok, zaman ifadesi rakamsız
     (`bugün`, `dün`, 5 bar → `bir hafta önce`/`a week ago`, 30 bar → `bir aydan uzun süre
     önce`/`over a month ago`; günlük bar = işlem günü), `within_bars` → `birkaç bar içinde`.
2. **Model.** Durum başına **bir çağrı, iki dil birden** (`{"tr": …, "en": …}`). Sistem
   istemi Claude Code'un varsayılanının yerine geçer (`--system-prompt`); `--tools ""`,
   `--strict-mcp-config`, `--setting-sources ""`, `--no-session-persistence`, boş geçici
   çalışma dizini. İstemler aynen `prompts/` altında: `system.txt` (kurallar + 7 TR/EN
   örnek; örneklerin enstrüman ve değerleri test kümesinde yok, K26 örneği de farklı
   değerlerle: ENKAI, RSI 30'un altı, şimdi 41,2), `user.txt`, `retry.txt`.
   `selfcheck.py`: istemdeki 7 örnek çıktının kendisi doğrulayıcıdan geçer (14/14).
3. **Doğrulayıcı (`verify.py`, deterministik).** Kurallar:
   - Biçim: tek satır, ≤ 2 cümle, ≤ 45 sözcük, işaretleme yok, yuva adı sızmaz, tavsiye
     sözcüğü yok (`tavsiye`, `öneri`, `should`, `recommend`, …).
   - Sayı: her rakam belirteci (TR ve EN binlik/ondalık okumaları) bir yuva değerinin
     `round(v, k)`'sine eşit olmalı (k = yazılan ondalık); **yazıyla sayı hiç yasak**
     (rakam yuvadan kopyalanır).
   - `redacted`: hiç rakam, hiç sayı sözcüğü (`buçuk`, `yüzde`, `half`, `percent` dahil),
     `%` yok.
   - Zaman ifadesi: "N bar önce / N bars ago" → N = `bars_ago` (ya da son geçiş); her
     "N bar" → N ∈ {bars_ago, son geçiş, within_bars}; `bugün/today` yalnız 0, `dün/yesterday`
     yalnız 1 olan bir zaman yuvası varsa; olay ve bitmiş durumda `when_phrases`'tan biri
     harfiyen bulunmalı.
   - Kip (zaman sınıfına göre): olay → geçmiş; süren durum → şimdiki zaman, "artık/no
     longer" yok; bitmiş durum → geçmiş + "artık … değil"/"no longer"; beklenti → koşul
     (`-sa/-se`, `eğer`; `if`).
   - Tür terimi geçmeli (kök listesi).
4. **Yeniden deneme / yedek.** Reddedilen dil, doğrulayıcının gerekçeleri ve önceki metinle
   yeniden istenir; durum başına en çok 3 çağrı. Üçüncüden sonra hâlâ ret → asgari
   **yuva dökümü** (`THYAO · extreme · up · state · weakened · 3 bar önce · valid_now=no`;
   düzyazı şablon değil). **Yedek hiç devreye girmedi (0/292 cümle).**

## Sonuçlar

İstem sürümü v2 (aşağıda v1 → v2). Tam küme iki kez, bağımsız üretildi:
`sentences.json` (+ `calls.json`, `log.run1.json`) ve `sentences.run2.json`
(+ `calls.run2.json`, `log.run2.json`). Düzenekte `llmslot` (1. üretim) ve yalnız
çalıştan çalışa kıyas için `llmslot_run2` kayıtlı.

### Otomatik ölçüler (`harness m2 measure`, iki üretimde aynı)

| ölçü | tr | en |
|---|---:|---:|
| zaman doğru (73) | 1,00 | 1,00 |
| — olay / süren durum / bitmiş durum / beklenti | 1,00 / 1,00 / 1,00 / 1,00 | 1,00 / 1,00 / 1,00 / 1,00 |
| K26 zaman doğru | evet | evet |
| tam zaman başvurusu | 0,83 | 0,83 |
| sızıntı / redacted | 0/18 | 0/18 |
| uydurma sayılı cümle / açık | 0/55 | 0/55 |
| boş cümle | 0 | 0 |
| düzenek belirleyiciliği (dosya sarmalayıcı) | 1,00 | 1,00 — anlamsız, aşağıya bakın |

"Tam zaman başvurusu" 0,83: kaçan 6/36 durumun hepsi `redacted` ve 5 ya da 30 bar önce
(rakam yasak olduğu için "bir hafta önce", "bir aydan uzun süre önce"). Tasarım gereği.

**Uyarı (dürüstlük):** doğrulayıcının kip kuralları K26/K27'yi kodladığı için düzeneğin
zaman sınıflayıcısıyla büyük ölçüde örtüşür; otomatik zaman puanının 1,00 olması kısmen
doğrulayıcının o kuralı zorlamasının sonucudur (bağımsız kanıt değil). Bağımsız sınama
anlama testidir (henüz koşulmadı; orkestratör koşacak).

### Belirleyicilik (çalıştan çalışa)

| | tr | en |
|---|---:|---:|
| bayt eşit cümle, 1. üretim ↔ 2. üretim | **6/73 = 0,08** | **10/73 = 0,14** |
| aynı sayı kümesi (rakam belirteçleri) | 68/73 | 68/73 |

Aynı girdi farklı cümle verir (`claude -p` sıcaklık/tohum denetimi vermez). Sayılar ve
zaman ifadeleri doğrulayıcı sayesinde her iki üretimde doğru; değişen ifade biçimi ve
hangi izinli sayıların seçildiği. Örnek, K26 2. üretim: "… aşırı alım zayıfladı ve artık
geçerli değil." (1. üretim: "… aşırı alım sona erdi."). Düzeneğin determinism ölçüsü
yalnız dosyayı okur (1,00) — gerçek değer bu tablodur.

### Ret / yeniden deneme

| | v1 istem (atıldı) | v2 1. üretim | v2 2. üretim |
|---|---:|---:|---:|
| çağrı (73 durum) | 77 | 74 | 75 |
| reddedilen metin (146 içinde) | 5 | 1 | 3 |
| gerekçeler | 3× > 45 sözcük, 1× yuva adı sızdı, 2× beklentide koşul eki yok | 1× > 45 sözcük | 2× çıktıda o dil yok (ayrıştırma), 1× > 45 sözcük |
| yedek (yuva dökümü) | 0 | 0 | 0 |

İkinci denemede her ret düzeldi; üçüncü çağrı hiç gerekmedi.

### Maliyet ve gecikme (model `sonnet`, liste fiyatı `total_cost_usd`)

| | 1. üretim | 2. üretim |
|---|---:|---:|
| toplam | 1,03 $ | 0,78 $ |
| cümle başına (146) | 0,0070 $ | 0,0054 $ |
| çağrı süresi medyan / p90 / en çok | 9,8 / 14,5 / 17,4 s | 6,6 / 11,1 / 15,2 s |
| durum başına süre (yeniden deneme dahil) en çok | 17,4 s | 26,6 s |
| çağrı başına girdi / çıktı belirteci (ort.) | 6.240 / 796 | 6.244 / 837 |

- Girdi belirteçlerinin çoğu önbellekten okunan sistem istemi (~5 bin); ilk çağrı önbelleği
  yazsın diye tek başına, kalanlar 6 paralel. Çıktı belirteçleri iki kısa cümleye göre
  yüksek; modelin iç düşünme belirteçlerini de içeriyor olabilir (CLI ayrıntı vermiyor).
- `calls.json`: harness biçimi + `attempt`, `cost_usd`, `shared_call_langs`. İki dil tek
  çağrıda yazıldığı için her dil satırında `seconds` çağrının **tam** süresidir (kullanıcının
  bekleyeceği süre), belirteç ve maliyet ikiye bölünmüştür. Düzeneğin `cost.json`'u
  1. üretimin `calls.json`'unu özetler (147 satır, ort. 10,1 s).
- Karşılaştırma: kurallı bir cümleci yüzlerce ns'de, ağsız ve bedava çalışır; bu aday cümle
  başına ~0,005–0,007 $ ve 7–15 s ister ve ağa bağlıdır.
- Bu görevin toplam LLM harcaması ≈ **3,0 $** (deneme 0,21 + v1 0,99 + v2 1,03 + 0,78).

### K26 cümleleri (1. üretim)

- TR: "THYAO'da RSI 3 bar önce 70'in üstüne çıkarak aşırı alıma girdi; şu an 55 ve artık
  70'in üstünde değil, aşırı alım sona erdi. Bu tür sinyal geçmişte 140 örneğin 87'sinde
  gerçekleşti."
- EN: "THYAO's RSI rose above 70 into overbought territory 3 bars ago; it is now 55 and no
  longer above 70, so the overbought state has ended. This kind of signal realized in 87 of
  140 past cases."

## v1 → v2 (tek istem yinelemesi)

v1'in 146 cümlesi elle okundu (`v1/`, sistem istemi `v1/system.v1.txt`). Doğrulayıcının
yakalamadığı sorunlar: beklenti cümlesinde sonuç yan cümlesinin düşmesi (s71 EN: "If … fell
below … within 3 bars, the … signal expired" — dilbilgisi bozuk), aşamanın iki kez
söylenmesi (s61 TR "… teyit etti ve teyit edildi aşamasında"). v2'de iki kural değişti:
beklenti için "önce eksiksiz eğer-ise yan cümlesi, kapanmış aşamada geçmiş biçimi
(`… teyit olacaktı` / `was to be confirmed if`)" ve "aşamayı bir kez söyle". Ölçülen
cümleler v2'dir; v1 yalnız kayıt.

## Zayıflıklar

1. **Belirleyici değil.** Aynı girdi → farklı bayt (TR 0,08, EN 0,14 eşitlik). Ürün ilkesine
   (aynı girdi, aynı çıktı) aykırı; önbellekleme yalnız *aynı* cümlenin tekrarını sağlar,
   cümlenin girdiden türetilebilirliğini değil. Denetim izi için cümlenin kendisi saklanmalı.
2. **Doğrulayıcı anlamı denetlemez.** Sayıların ve zaman ifadelerinin *doğru yuvadan*
   geldiğini değil, *bir* yuvaya eşit olduğunu denetler. Model bir değeri yanlış etikete
   bağlayabilir (ör. "ortalama 452.000" yerine kapanış değerini ortalama diye yazmak) ve
   geçer. Yön (yukarı/aşağı) ve "hâlâ geçerli" iddiası da sözcük düzeyinde denetlenmiyor;
   elle okumada yanlış yön görmedim ama bu bir kanıt değil.
3. **Dilbilgisi ve akıcılık denetlenmez.** v1'de bozuk beklenti cümlesi doğrulayıcıdan geçti.
4. **Yuva etiketi zayıflığı (motor tarafı).** Akış (`flow`) aşağı yönde koşul "net, ortalamanın
   altında" diye geliyor; model bunu sadakatle "net satış ortalamanın altına indi" diye
   yazıyor (s47), ki okura "satış zayıftı" gibi gelir. Model yuvaya sadık; sorun yuva
   anlamında. Kurallı adayı da aynı şekilde etkiler.
5. **`redacted` zaman ifadesi kaba.** 5 bar → "bir hafta önce", 30 bar → "bir aydan uzun
   süre önce"; tam bar sayısı kaybolur (anlama testinde `when_exact` düşebilir).
6. **Maliyet / gecikme / bağımlılık.** Cümle başına ~0,6 sent ve ~7–15 s; ağ ve sağlayıcı
   gerekir; cihazda çevrimdışı çalışmaz; WASM yansısında üretilemez.
7. **Uzunluk.** Ortalama TR 30, EN 35 sözcük (en çok 39 / 45); karne cümlesi uzatıyor.
   45 sözcük sınırı en sık ret nedeni.
8. **Ölçünün öz-doğrulaması.** Bkz. yukarıdaki uyarı: otomatik zaman ölçüsü bu aday için
   bağımsız değildir.

## Dosyalar

- `slots.py` yuva kurucu · `verify.py` doğrulayıcı · `generate.py` üretici ·
  `selfcheck.py` istem örneklerinin doğrulanması · `stats.py` özet (ret, maliyet, gecikme,
  çalıştan çalışa eşitlik)
- `prompts/system.txt`, `prompts/user.txt`, `prompts/retry.txt` — istemler aynen
- `inputs.json` — `harness m2 inputs` çıktısı (sentetik)
- `sentences.json`, `calls.json`, `log.run1.json` — 1. üretim (ölçülen)
- `sentences.run2.json`, `calls.run2.json`, `log.run2.json` — 2. üretim (belirleyicilik için)
- `v1/` — atılan v1 istemi ve çıktıları
- Ölçüm kaydı: `../raw/llmslot/`, `../raw/llmslot_run2/` (`auto.json`, `auto.md`, `cost.json`)

Yeniden üretim: `python3 -I generate.py --run 1` / `--run 2`, sonra
`harness m2 measure --alt llmslot` ve `python3 -I stats.py`. Okuyucu testi koşulmadı.
