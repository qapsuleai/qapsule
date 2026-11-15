# R2 — Bağımsız Denetim (T11)

- **Denetçi:** bağımsız denetçi alt ajan (round'un hiçbir işini yapmadı).
- **Tarih:** 2026-11-15
- **Denetlenen:** `r2/int` @ `521d5f4` ("R2 close (pre-audit)"), temiz kopya:
  `git clone /Users/arkns/Works/Mchart/qapsule <scratchpad>/audit && git checkout 521d5f4`.
  Bütün komutlar bu kopyada çalıştırıldı. Gerçek depoya yazılan tek dosya bu dosyadır
  (dal `r2/audit`, `521d5f4`'ten).
- **Makine:** Apple silicon (aarch64), macOS; `qp` = kopyada `verify.sh`'ın derlediği
  `target/release/qp`.
- **Bağımsız hesaplar:** kendi Python kodum (scratchpad `tools/ind.py`: SMA, Bollinger (nüfus
  sapması), RSI Wilder, MACD (SMA tohumlu EMA); `tools/pix.py`: yerleşim ve log eşleme
  `docs/GRAMMAR.md` §3/§14'ten). Uygulama koduna bakarak yazılmadı.

## Sonuç: **KOŞULLU KABUL**

Round'un çekirdeği iddia edildiği gibi çalışıyor: `verify.sh` temiz kopyada 14/14 yeşil, CI
`r2/int`'te yeşil, sekiz görünüm dökümünden iki kez bayt-eşit ve altınla aynı çiziliyor, R0
altın görselleri değişmedi, altın görsellerde ölçtüğüm 40'tan fazla noktanın hepsi veriden
bağımsız hesapladığım konumla 0–2 px içinde, `hit` 65 isabette değerleri verimle birebir
veriyor, ajan yazabilirlik ham kayıtları yeniden puanlamada bayt-eşit, 100k kare raporla
uyumlu (2,683 ms, −%2), şartnameyi okuyarak kimlik bilmeden kurduğum görünüm ilk gerçek
denemede uygulandı ve çizildi. Hesap kodunda değişiklik yalnız denetim kalanları.

Kabulü koşullu kılan noktalar (ayrıntı "Bulgular"da):

1. **Tahmin yelpazesi (nokta serilerinin son bardan sonraki yuvaları) ekranlı hiçbir
   bağlantının `screen`/`hit`'inde görünmüyor.** Ekranlı bağlantının her zaman bir bakışı var
   (varsayılan son N bar) ve `look` son bardan ötesini reddediyor; V3 görünümünün
   masaüstündeki `screen` özetinde üç bandın her biri **1 öğe**. Altın görsel ve `qp explain`
   (bağlantısız) ise yelpazeyi gösteriyor. Ölçüt 7 ("`screen` her görünümü özetliyor") V3
   için ekranlı kişiler açısından sağlanmıyor. (Orta)
2. **Seçicili bir döküm adımı `send.msg` içinde yazılınca şemadan geçiyor ama koşucu
   reddediyor** (`qp schema validate --type Transcript` → `valid`; `qp conform` / `qp render
   --transcript` → `invalid type: map, expected a string`). Yalnız `send_raw` çalışıyor; bu
   `PROTOCOL.md` §15'te yazmıyor. (Düşük–orta; dilden bağımsız döküm biçimi için)
3. Yeni kavramlar **"seçici"** (`SeriesSel`…`ItemSel`, seçilen tasarımın çekirdeği) ve
   **"nokta serisi"** (`data.kind = "points"`) `docs/CONCEPTS.md`'de yok (CLAUDE.md kuralı). (Düşük)
4. `REPORT.md`, `STATUS.md`, `ROADMAP.md` denetimden önce "Tamamlandı, 14/14" diyor ve henüz
   olmayan `AUDIT.md`'yi kanıt gösteriyor (RD1 denetimindeki aynı durum); WASM sayıları
   belgeler arasında tutarsız. (Düşük)

Koşullar: (K1) Bulgu 1 için ya ekranlı bağlantının görünen aralığını nokta serisi yuvalarına
uzatmak (ör. bakış son bara dayalıysa uzatma) ya da şartnamede (GRAMMAR §3, PROTOCOL §4.6,
§5.4 `screen`) davranışı açıkça tanımlamak ve REPORT'ta ölçüt 7'yi ⚠️ olarak not etmek —
ana akış kararı; (K2) PROTOCOL §15'e "seçici içeren mesajlar `send_raw` ile yazılır" notu ya da
koşucunun seçicili `send.msg`'i kabul etmesi; (K3) `CONCEPTS.md`'ye seçici ve nokta serisi;
(K4) aşağıdaki "REPORT'ta abartılı ya da tutarsız" listesinin düzeltilmesi.

## VERIFY tablosu (16 satır)

**Toplam: 14 PASS, 2 koşullu PASS (7, 10), 0 FAIL.**

| # | Kontrol | Sonuç | Kanıt |
|---|---------|-------|-------|
| 1 | `QP_VERIFY_BROWSER=1 scripts/verify.sh` | **PASS** | Temiz kopyada `EXIT=0`, `verify: OK`, 14/14 PASS, 213 s: fmt, clippy, test (toplam 267+ test geçti, 0 başarısız, 1 yok sayılan = `presenter_timing`, süre ölçümü), schema "23 schemas, deterministic", fixtures, golden "byte-identical", views "8 views x2 byte-identical to goldens (V0 = sample.png)", wasm "gzip -9 595584 B", hygiene, browser "headless Chromium matches golden", compute, digest "arm64, x86_64, wasm32: 7b1e99d40889f83c", protocol "61 examples as expected", conform "27 transcripts x inproc, serve, mcp". `scripts/qapsule-gh.env` kopyada zaten **izleniyor** (`git ls-files` gösteriyor; `.gitignore` istisnası); ana kopyadan bir şey kopyalamaya gerek olmadı, içeriğini okumadım (hygiene adımı izinli dört anahtarı denetliyor, PASS). |
| 2 | `scripts/preflight.sh`; CI | **PASS** | `preflight: TAMAM` (origin, ssh, `qgh` hesabı, yazma izni, commit e-postası). `scripts/qgh run list --branch r2/int` → run `43658183855` "R2 close (pre-audit)" başta `in_progress`, bekledim: `completed success` (5m43s), `headSha 521d5f4d…`, iş `verify` success. Not: VERIFY satırı `--branch main` diyor; `main` henüz RD1'de (`506bdf0`, success), R2 birleştirilmedi. |
| 3 | `qp conform` üç taşımada | **PASS** | verify adım 14: 27 döküm × inproc/serve/mcp. `git diff 506bdf0 521d5f4 -- fixtures/protocol/conformance` yalnız 18–27'yi ekliyor; 01–17 değişmedi. Koşucunun ayırt ediciliği: döküm 21'de bir `no_origin` beklentisini `invalid` yaptım → üç taşımada da FAIL (doğru adım ve yol: `step 4 message 0 at /error/code`). (İlk denememde ilk `"no_origin"` metni bir açıklamadaydı ve döküm geçti — bu koşucu hatası değil, benim mutasyonumun yeri.) |
| 4 | Altı görünüm (+2 derinlik), dökümden iki kez | **PASS** | Kendi döngümle (`qp render --transcript fixtures/views/<v>.json …` ×2, sha256): v0-r0, v1, v2, v3, v4, v5, d1, d2 → **8/8 iki çizim aynı ve altınla aynı** (ör. v1 `558996b2be7925eb`, v3 `6236ed8ece850695`). Dökümler yalnız protokol mesajı: v0 `add_study`+`put_view`; v1 `add_study`×2, `put_view`, `insert_layer`×4; v2 `add_study`, `put_view`; v3 `put_series`×6 (21 noktalık satır içi nokta serileri), `put_view`; v4 `put_view`, `set_scale`; v5 `add_study`×2, `put_view`; d1/d2 `thyao` + `present`/`show_finding`. Başlangıç dokümanlarında (`base.json`, `growth.json`) görünüm yok. |
| 5 | R0 altın görselleri | **PASS** | `git diff --stat 506bdf0 521d5f4 -- fixtures/golden/sample.png fixtures/golden/stress.png` → **boş**. `fixtures/golden/` farkı yalnız 7 yeni dosya (`views/*.png`). |
| 6 | Görselleri açıp veriden noktaları karşılaştırma | **PASS** | Sekiz görseli Read aracıyla açtım (aşağıda "VERIFY 6"). Göstergeleri bağımsız hesapladım; `qp explain`'deki son değerlerle 1e-13 içinde aynı (SMA20 124.9705, BB 128.50979/121.43121, RSI 55.0999118, histogram −0.4127343, SMA50 120.8594). Otomatik alanları (%5 pay; log'da log uzayında) yeniden hesapladım: hepsi aynı. Görsel başına 2–6 nokta, toplam 40+ nokta: hepsi beklenen pikselden **0–2 px**. |
| 7 | Yalnız şartname + şemayla yeni görünüm | **koşullu PASS** | Hacim çubukları yeni panelde (fiyatın altında) + kapanış alanı, `thyao`'ya karşı yalnız seçicilerle; **ilk gerçek komut uygulandı**, iki çizim bayt-eşit. Belirsizlikler ve bir kusur (Bulgu 2) aşağıda "VERIFY 7". |
| 8 | Kökensiz işaret, olmayan seri, geçersiz kanal, belirsiz seçici | **PASS** | Hepsi reddedildi, `path` ile: `no_origin` ("layer `l5` of view `main`: constant 123.45 at /encoding/y has no origin", `hint`); RSI panelinde 31 → `candidates` yakınlık sırasıyla (`rsi.oversold`=30, `rsi.mid`=50, olgu alanları…); `rsi41` → `not_found` + `did_you_mean ["rsi14"]`; `{"study":"stochastic"}` → `not_found` + 4 aday; `colour` → `schema` + `expected` kanal listesi + `did_you_mean ["color"]`; `{"mark":"line"}` → `ambiguous`, 3 aday çizim sırasıyla; `{"output":"upper"}` → `invalid` + `split_layer` hazır dizisi; kökeninden farklı sabit (70 vs `rsi.oversold`) → `invalid`, `expected ["30"]`. Eksik: fiyat panelinde (eşik ve olgu yok) `no_origin` `candidates` vermiyor (Bulgu 3). |
| 9 | 18 eşzamanlı yazma | **PASS** | `cargo test --release -p qp-session --test concurrency -- --nocapture`: `base`'siz correct 18 / refused 0 / **WRONG 0**; `base: 3` ile correct 16 / refused 2 (S8 iki sıra) / **WRONG 0**. Kâhinleri okudum: kimlik düzeyinde niyet, son yazan kazanır; anlamlı. Not: 9 senaryonun yalnız 3'ü (S1–S3) gramer işlemi; gerisi RD1/T6c'nin seri/bulgu işlemleri. Katman düzeyi çakışma döküm 23'te. |
| 10 | `screen` ve `hit`: rastgele noktalar | **koşullu PASS** | Altı görünümde (v0–v5) **20 rastgele + 10 hedefli nokta/görünüm** (kendi tohumum 20261115), `headless` bağlantıdan: 176 sorgu, 65 isabet, 111 boş; **isabetlerin hepsinde bar = benim x'ten hesapladığım bar ve değerler verimle birebir** (mum o/h/l/c, SMA, RSI, histogram + taban 0, Bollinger üst/alt/orta, yelpaze `v`/`q65.v` vb.); uyumsuzluk 0. Boş olduğunu bildiğim alanlar (ısınma barları, log grafiğin sol üstü) "öğe yok". Koşul: **masaüstü** bağlantıdan V3'te yelpaze hiç görünmüyor (Bulgu 1). |
| 11 | Sunucu belirleyiciliği | **PASS** | `qp serve` üzerinden kendim: watch/phone/desktop × 1.000 `present fnd-overbought` → derinlik başına **1** ayrı çıktı (`sentence` {text,state}; `capsule` {+capsule_view, glyphs, spark, confidence}; `chart` {+view, facts}). `tests/presenter.rs` (1.000 × 3) de geçiyor. |
| 12 | Ajan yazabilirlik ham kayıtları | **PASS** | `harness/score.py v1` ve `score.py v2 A` yeniden koşuldu: `git status` temiz (score.json'lar **bayt-eşit**); sayılar REPORT ile aynı (base 7/20·2/20·20/20; A 19/19/19; B 17/10/20; A v2 19/16/20; belirteçler aynı). 20 ajan, 20 ayrı tohum = sha256(`r2-t9/<etiket>/<değişke>/<ajan>`)[:8] (kendim yeniden hesapladım), 400 rastgele kimliğin hepsi farklı, şartname örnek kimliklerinden (`px`, `rsi14`, `main`, `l0`…) hiçbiri yok. Her ajanda `sandbox.sb`, `prompt.txt` (RD1 O3 kapandı). Profiller: v1 base/A yalnız okuma reddi (yazma reddi 0 satır), v1 B ve v2 A okuma+yazma reddi (3 satır) — REPORT bunu söylüyor. Dökümlerdeki araç çağrılarında paket/iş klasörü dışında `file_path` 0. `isotest/os-check.txt`: depo, gizli doküman, `qp` ikilisi okunamıyor (`Operation not permitted`), depoya ve başka ajana yazılamıyor. |
| 13 | 100k bar kare | **PASS** | `cargo run --release -p qp-render --example frame_100k`: LOD **2,683 ms** (derleme 1,553 + tarama 1,207), tam 59,56 ms, fark 2.275/720.000 px (%0,316). Rapor 2,745 ms → **−%2,3** (±%20 içinde). |
| 14 | `qp-grammar`/`qp-render`'da hesap kodu | **PASS** | VERIFY'daki komut (sınırsız) 81 satır buluyor, ama hepsi alt dizge: `btreemap` 35 (`BTreeMap`: "tr**ema**p"), `qp_schema` 29 ("sch**ema**"), `smallest`/`small` 6 ("**sma**ll"), `persistent`/`versions`/`parsing` ("**rsi**"), `semantic` 2, `remainder` 2. Kelime sınırlı `grep -rn -i -E "\b(sma|ema|rsi|macd|bollinger)\b"` → **0 satır**. Elle tarama: `sum`/`mean`/`sqrt`/pencere araması yalnız yerleşim (panel ağırlıkları), `detmath.rs` (belirleyici `log10`/`pow10`), çizici; hesap yok. `qp-grammar` ve `qp-render` `[dependencies]`'inde `qp-compute` yok (yalnız dev). |
| 15 | Hesap özeti ve `qp-compute` | **PASS** | Özet `7b1e99d40889f83c` üç hedefte (verify 12) ve `fixtures/compute` değişmedi. `git diff 506bdf0 521d5f4 -- crates/qp-compute/src`: `MAX_PERIOD = 100000` ve `check_period` (beş hesap), `period_param` iletisi (`1e18` bilimsel biçimde), `Study::discard_last` + `Chain` düzeltmesi, `late.rs` sınır yorumları/assert — yalnız denetim kalanları. `period 1e18` → "parameter `period` is 1e18, which exceeds the maximum period 100000"; `slow 100001` reddedildi, `period 100000` kabul. |
| 16 | Gizli bilgi; eski klasörler | **PASS** | İzlenen dosyalarda `ghp_…`, `github_pat_…`, `sk-ant-…`, `sk-…{32}`, `AKIA…`, `BEGIN … PRIVATE KEY`, `xox[bpa]-`, `AIza…` → 0 değer (yalnız R1/RD1 AUDIT'te desen adları). `rounds/R2` (12 MB ham kayıt, 618 dosya) içinde `bearer`/`oauth`/`api_key`/JWT → 0; `.qp-token` dosyası izlenmiyor. Adında `.env`/`secret`/`token`/`.pem`/`.key` olan izlenen dosya yalnız `scripts/qapsule-gh.env` (bilinçli istisna). `506bdf0..521d5f4` farkında depo dışı yol yok. `../Docs` ve `../Services` git deposu değil; mtime'ları 2026-11-14 00:27/00:28 (round öncesi); `find -newer rounds/R2/GOAL.md` iki klasörde boş (yalnız ad listeleme; `.env` okunmadı). |

## Kabul ölçütleri (GOAL.md, 14)

**Toplam: 9 ✅, 5 ⚠️, 0 ❌.**

| # | Ölçüt | Durum | Gerekçe |
|---|-------|:-----:|---------|
| 1 | Tipler `qp-schema`'da; şema ve MCP ondan; `GRAMMAR.md` örnekli, elenenler sonda; kavramlar sözlükte (koddan önce) | ⚠️ | Tipler, 23 şema, MCP araçları şemadan, GRAMMAR §1–16 örnekli + §17 elenen 13 seçenek: doğru. `d1e0ea0` (sözlük) `b462ed2`'nin (tipler) atası. Ama **"seçici" ve "nokta serisi" sözlükte yok** (Bulgu 3); REPORT'un kendi sapma 1'i (taslak sözlükten önce yazıldı) de var. |
| 2 | Kimlikle adresleme; kimlikli işlemler; indeksli alanlar kullanımdan kalkmış; 18 durumda yanlış 0 | ✅ | Şemada `add_layer.index`/`remove_layer` `"deprecated": true`; 18 durum WRONG 0 (VERIFY 9); düşmanca girdilerde kendine göre yerleştirme, yinelenen kimlik, 65 karakterlik kimlik doğru reddedildi. |
| 3 | R0 sahnesi protokolden, altınla bayt-eşit | ✅ | `v0-r0` → `sample.png` sha256 aynı (kendi koşum); tarayıcı adımı PASS; `sample.png`/`stress.png` değişmedi. |
| 4 | Beş yeni görünüm yalnız protokolle, altın görseller | ✅ | VERIFY 4 ve 6. Görsel nitelik notları (Bulgu 6) kabulü etkilemiyor. |
| 5 | Derinlik kuralları, sunucu taşındı, capsule/chart altın, üç yüzey, belirleyici | ✅ | `d1`/`d2` altınla aynı; d1'de olgu noktası RSI 72,84'te, kural 70'te (px hesabımla 0 px); d2'de vurgu şeridi bar 150'de (x=435); döküm 25; 1.000 × 3 tek çıktı (VERIFY 11). |
| 6 | Kökensiz işaret reddi (katman, alan, neden, yakın seçenek); geçersiz kanal; testler | ⚠️ | Ret, katman, alan (`path`), neden doğru; RSI panelinde aday listesi iyi. Ama eşiği/olgusu olmayan panelde (fiyat paneli) **`candidates` hiç yok**, yalnız `hint`; GRAMMAR §10'daki örnek aynı 123.45 durumu için aday gösteriyor (örnek dokümanda; `thyao`'da yok). "Yakın doğru seçeneği öneriyor" her durumda değil. |
| 7 | `screen` her görünümü özetliyor; `hit`; gidiş-dönüş; sahne tarifinden | ⚠️ | `hit` değerleri birebir (VERIFY 10), `qp-session` `qp-render`'a bağlı değil. Ama **V3 yelpazesi ekranlı bağlantıların `screen`/`hit`'inde yok** (Bulgu 1); `reading.rs` testi `headless` soranla koştuğu için bunu yakalamıyor. |
| 8 | ≥2 anlamla başvuru tasarımı ölçüldü; seçilen şartnamede; belirsizlik seçenekli hata | ✅ | T8a/T8b, T9 ile ölçüldü, PROTOCOL §17; `ambiguous` adaylarla (VERIFY 8); günlükte yalnız kimlik (kendim `log` sorgusuyla doğruladım). |
| 9 | Ajan yazabilirlik (gramer) | ✅ | 20 görev, rastgele kimlik, OS düzeyinde okuma reddi, RD1 v2r ile karşılaştırma, düzeltme (`output` katılığı + `split_layer`) ve v2 yeniden ölçümü; sayılar yeniden üretiliyor. Sınırlar REPORT'ta dürüstçe yazılı (n=20, tek koşu, v1 base/A eski profil). |
| 10 | 100k bar < 8 ms; LOD/tam farkı; eski matematikten alınan/alınmayan | ✅ | 2,683 ms; fark %0,316; `crates/qp-grammar/LEGACY.md` var. |
| 11 | ≥8 yeni döküm, üç taşıma; RD1'in 17'si değişmeden | ✅ | 10 yeni; 27 × 3; 01–17 farkı boş. |
| 12 | R1 denetim bulguları | ✅ | VERIFY 15; `Chain` testleri ve `chain_modes_are_bit_identical` test adımında geçiyor. |
| 13 | WASM boyutu ölçüldü, RD1 ile karşılaştırıldı | ⚠️ | Ölçülmüş ve karşılaştırılmış, ama sayılar tutarsız: REPORT 595.475 B (+120.349), `DECISIONS.md` A13 "+116,6 KB (476,6 → 593,2 KB)", benim temiz kopyam 595.584 B. Fark büyük olasılıkla yol uzunluğuna bağlı panik dizgelerinden; tek bir sayı ve ölçüm yeri yazılmalı. |
| 14 | REPORT, AUDIT, STATUS, ROADMAP; önerilen kararlar ayrı bölümde | ⚠️ | `DECISIONS.md` "Önerilen (R2)" Ö10–Ö17, A13–A14 var. Ama `521d5f4`'te `AUDIT.md` yoktu; REPORT "Durum: Tamamlandı" + ölçüt 14 ✅ + boş "Denetim" bölümü; `STATUS.md`/`ROADMAP.md` "14/14" ve `AUDIT.md`'yi kanıt gösteriyor. Bu dosya ve REPORT'un güncellenmesiyle kapanır. |

## CLAUDE.md kuralları

| Kural | Durum | Kanıt |
|-------|:-----:|-------|
| `NORTH_STAR.md` değişmedi | ✅ | Son değişiklik R0/T1 (`5b36964`); `506bdf0..521d5f4` farkı boş. |
| Hesap/gramer/oturum kodunda LLM, ağ, rastgelelik, saat yok | ✅ | `qp-compute`, `qp-grammar`, `qp-session` `src/`'de `rand`, `SystemTime`, `Instant::now`, `std::net`, `HashMap/HashSet`, HTTP istemcisi → 0 (ilk aramadaki `rand::` eşleşmeleri `Operand::`). Bağımlılıklar: compute/grammar yalnız `qp-schema`; session'da `tempfile` yalnız döküm koşucusunda (`conform.rs`, RD1'den beri). |
| Fiyat/hesap değerleri `f64` | ✅ | `f32` yalnız sahne ilkellerinin piksel koordinatlarında (`draw.rs`). |
| Var olan altın görseller değişmedi | ✅ | VERIFY 5. Yeni görünümler için yeni altınlar eklendi. |
| Şemanın tek kaynağı Rust tipleri | ✅ | İzlenen `*.schema.json` yalnız RD1 ölçüm paketlerinin anlık kopyaları; MCP girdi şemaları üretiliyor (yalnız küçük `outputSchema` elle, RD1'den). |
| Kavramlar sözlükte | ⚠️ | Bulgu 3. |
| Alt ajan, ayrı denetçi, dallar | ✅ | REPORT'ta her alt görevin dalı ve denetçisi; dallar yerelde var; `r2/t8b-names` birleştirilmemiş. |

## VERIFY 6 — Görseller (gözle ve sayıyla)

| Görsel | Gördüğüm | Bağımsız noktalar (beklenen y → bulunan) |
|--------|----------|------------------------------------------|
| `sample.png` (V0) | R0: mumlar, turuncu SMA20, başlık `SAMPLE 1d C 124.69`, sağ üstte `SMA20 124.97`, ızgara 90–130. | SMA20 bar 60/120/180 (94.75/102.71/117.80) → 487.4/392.5/212.3: 0 px; mum yüksek/düşük bar 100 ve 199: 0/−1 px; ızgara 100 → y 424.8, 120 → 186.1 (etiketlerle aynı). |
| `v1-three-panels.png` | Üç panel (3:1:1): mumlar; RSI (turuncu) 70/30 kurallarıyla, sol üstte `RSI14 55.10`; MACD histogramı işarete göre yeşil/kırmızı, `0.00` ve `-0.41` etiketleri. | RSI bar 50/150/190 → −2/0/0 px; histogram uçları bar 60/175/199 (0.421/−0.387/−0.413) → 0/−1/−1 px; 70/30 kuralları y 389.2/431.2. Not: MACD panelinde açıklama yok; RSI panelinde 30/70 eksen etiketi yok (yalnız son değer). |
| `v2-bollinger-band.png` | Bollinger alanı (yarı saydam kahve), orta çizgi turuncu, mumlar üstte; sağ üstte açıklama **iki kez** `BOLLINGER(20,2)` (REPORT Öneriler'de biliniyor); sağ eksende `128.51` (üst bant). | Orta bar 80/120/160 → −1/0/0 px (bar 40 ve 199'da mum üstte örttüğü için çizgi pikseli yok); bant üst/alt kenarı bar 40/120/175: 436.1→436, 488.8→488; 344.8→345, 440.2→439; 173.2→173, 246.4→245. |
| `v3-fan.png` | Mumlar; son bardan sağa açılan üç katlı yelpaze (iç band en opak), x ekseni 2025-11'e uzanıyor; açıklama `q05 q20 q35` (seri kimlikleri). | Alan [86.8875, 141.9425] yeniden hesaplandı (aynı); yuva 209: q95 134.34 → 108.8, q05 118.78 → 262.0, dolu piksel 109..261; yuva 219: q95 139.44 → 58.6, q05 117.42 → 275.4, x=1124/1125'te dolu 59/60..275. |
| `v4-log.png` | GROWTH 10 → 394, log eksen 10/20/50/100/200 (eşit aralıklı oranlar), mumlar düz bir doğru üstünde. | Kapanış bar 20/100/199 (14.50/64.40/394.06) → 500.9/301.6/59.5: 0 px; ızgara 10/20/50/100/200 → 550.6/457.9/335.4/242.8/150.2 (etiketlerle aynı); alan [8.2668, 476.909] = log uzayında %5 pay (benim hesabım aynı). |
| `v5-conditional.png` | Kapanış çizgisi (açık gri), SMA50 ile arası koşullu dolgu: üstteyken yeşil, mart başında altındayken kırmızı (SMA50 ısınmadan önce dolgu yok, doğru); alt panelde RSI basamaklı renk: 30 altı yeşil, 70 üstü kırmızı, arası gri. Sağ üst açıklama `SAMPLE 1d 124.69` yön renginde (çizgi gri) — küçük tutarsızlık. | Bar 100 ve 199'da c ≥ SMA50 → dolgu pikseli koyu yeşil; bar 60'ta c 96.38 < SMA50 96.75 → kırmızımsı; RSI bar 150 (72.84) kırmızı, 0 px; bar 180 (61.70) gri, 0 px. |
| `d1-capsule.png` (390×300) | Başlık "THYAO aşırı alımda: RSI 70'in üstünde.", RSI çizgisi son 60 bar, 70 kuralı, bar 150'de olgu noktası, eksen 40/60, `55.10`. | Alan [39.479, 79.955] yeniden hesaplandı (aynı); RSI bar 150/170/199 → 0/0/−2 px; kural 70 → y 93.5; nokta 72.84 → 76.5 (görselde aynı). |
| `d2-chart.png` | Son 80 bar mum (yuva 14 px), RSI + 70 kuralı, bar 150'de iki panelde vurgu şeridi. | Fiyat alanı [102.54, 132.02] aynı; mum yüksek bar 150/199 → 0 px; RSI bar 150/185 → 0/−1 px; vurgu merkezi x=435 (şerit pikseli). |

## VERIFY 7 — Yalnız şartname + şemayla kurulan görünüm

Görev: `thyao`'nun tek görünümüne fiyatın altına hacim çubuklu yeni panel + fiyat panelinde
kapanış alanı (mumların altında); kimlik yazmadan.

```json
{"type":"command","id":"c1","ops":[
 {"op":"insert_panel","view":{},"panel":{"name":"volume","y":{"kind":"linear"}},
  "after":{"mark":"candle"},"layers":[{"mark":"bar","series":{"kind":"bars"},"encoding":{"y":"v"}}]},
 {"op":"insert_layer","view":{},"layer":{"mark":"area","series":{"kind":"bars"},
  "panel":{"mark":"candle"},"encoding":{"y":"c","opacity":0.3}},"before":{"mark":"candle"}}]}
```

Sonuç: ilk denemede `ack applied`, `created ["3","l5","l6"]`, `resolved` her seçici için
(`/ops/0/view: main`, `/ops/0/after: "0"`, `/ops/1/before: l0` …); günlükte istek yalnız
kimliklerle. `qp render --transcript … --view main` iki kez bayt-eşit; `qp explain`: panel 3
"volume" fiyatın hemen altında, son `v=1387978.56` (benim veriden okuduğumla aynı), alan
katmanı `l6` mumlardan önce. Görselde hacim paneli ve kapanış alanı doğru yerde.

Karşılaşılan belirsizlikler / kusurlar:

1. **Seçicili mesaj dökümde `send` ile yazılamıyor** (Bulgu 2): şema doğrulaması `valid`,
   koşucu `invalid type: map, expected a string at line 139 column 4` diyor (hangi alan
   olduğu yok). `send_raw` gerektiği yalnız `t-select.json` örneğinden anlaşılıyor; §15 söylemiyor.
2. `LayerIn.panel` şemada **varsayılan 0**; GRAMMAR §4 ve PROTOCOL §5.1 bunu söylemiyor
   (paneli unutan katman sessizce fiyat paneline gider). `insert_panel.layers[].panel`'in yok
   sayıldığı yalnız şema açıklamasında.
3. `insert_panel`'in günlükteki isteği katmanları `"panel": 0` ile saklıyor (yeni panel 3'e
   gittikleri halde) — yeniden oynatma doğru ama denetim izini okuyan için yanıltıcı.
4. Hacim paneli otomatik alanı tabanı (0) da kapsıyor ve %5 pay **negatife** iniyor
   ([−124455, 2613560]); eksen etiketi `{:.2}` biçimiyle `2000000.00` / `1387978.56` (sağ
   kenara yapışık). Hacim için okunaklı değil.
5. Seri sırası: `{"kind":"bars"}` tek eşleşme olduğu için çözüldü; birden çok bar serili
   dokümanda ne olacağı açık (`ambiguous`), sorun değil.
6. `insert_panel` sonrası tek görünüm için `view: {}` ve `after: {"mark":"candle"}` kolayca
   yazılabildi; bu, T9'daki A sonucuyla uyumlu.

## Bulgular

| # | Bulgu | Önem | Kanıt / öneri |
|---|-------|------|---------------|
| 1 | Nokta serilerinin son bardan sonraki yuvaları (V3 yelpazesi) ekranlı bağlantıların `screen`/`hit`'inde **hiç görünmüyor**. Ekranlı bağlantının bakışı her zaman var (PROTOCOL §4.6: son N bar) ve GRAMMAR §3'teki uzatma "yalnız aralık belirtilmemişse"; `look` `to_bar 219` → `invalid "bar 219 is past the last bar 199"`; `zoom 0.5` → 0..199. Masaüstü `screen`: `b90/b60/b30` **1 öğe**, alan [93.8, 132.4] (q95 = 139.4 dışarıda). `headless` bağlantı (bakışı yok) yelpazeyi görüyor (`hit` bar 216 → `b30`, `q65.v 130.25`). `qp render`/`explain` yelpazeyi gösteriyor. | Orta | Kullanıcının gördüğü ile ajanın okuduğu ayrışıyor (ilke 9 ve ölçüt 7). Ya bakış son bara dayalıyken uzatma, ya şartnamede açık kural + ölçüt 7'ye not. Ayrıca PROTOCOL §4.6 "ekransız yüzeylerde de aynı kural, N = 120" ile `hit`'in `headless`'ta bütün barları kullanması uyumsuz görünüyor. |
| 2 | Seçicili `send.msg` dökümü şemadan geçiyor, koşucu reddediyor; hata alan adı vermiyor. | Düşük–orta | §17.8'deki bilinen uzlaşmanın (Rust tipi kimlik, şema kimlik-ya-da-seçici) döküm biçimine yansıması. §15'e not ya da koşucuda ham metinle besleme. |
| 3 | `no_origin` adaysız: eşiği ve olgusu olmayan panelde `candidates` yok. | Düşük | Ölçüt 6. En azından `{"kind":"series","series":…,"field":…,"at":"last"}` kökeni ya da "olgu ekle" hazır işlemi aday olarak verilebilir. |
| 4 | `CONCEPTS.md`'de "seçici" ve "nokta serisi" yok. | Düşük | CLAUDE.md "yeni kavram önce sözlüğe". |
| 5 | REPORT/STATUS/ROADMAP denetimden önce tamamlandı/14/14 diyor; WASM sayıları tutarsız. | Düşük | Aşağıdaki liste. |
| 6 | Görsel nitelik: v1'de MACD panel açıklaması yok, RSI panelinde 30/70 eksen etiketi yok; v2 yinelenen açıklama; v3 açıklaması seri kimlikleri (`q05 q20 q35`); v5 açıklama rengi çizgiyle uyuşmuyor; hacim etiketi biçimi ve negatif alan (VERIFY 7.4). | Düşük | Altın görseller doğru veriyi doğru yerde gösteriyor; bunlar okunaklılık. Var olan altınlar kendiliğinden güncellenmemeli; ayrı karar. |
| 7 | Sınırlar: `set_emphasis` hedefinde `to_bar: 4294967295` kabul ediliyor (çizimde kırpılıyor, hata yok); `insert_panel` `id: 4294967295` sonrası kimliksiz `insert_panel` "no panel id is left" ile reddediliyor (kendi kendini kilitleme). | Düşük | Aralık doğrulaması ya da üst sınır. |
| 8 | VERIFY 2 `main` CI'ı diyor; R2 henüz `main`'de değil. | Not | Birleştirmeden sonra `main` CI'ı da bakılmalı. |

### Düşmanca girdiler (gramer işlemleri ve seçiciler)

`qp serve`, `thyao`, `arkin`; 22 mesaj, 0,014 s, süreç çökmedi, hepsi tanımlı cevap:

| Girdi | Sonuç |
|-------|-------|
| `set_panel weight 1e308` | uygulandı; çizim ve `explain` normal (ağırlıklar en büyüğe bölünüyor) |
| `weight 0`, `weight -3` | `invalid` "panel weight must be > 0" |
| `set_scale domain [1e308,-1e308]`, `[5,5]` | `invalid` (min < max, sonlu) |
| `log` + `domain [-10,100]` | `invalid` "a log scale needs a positive domain" |
| MACD paneline `log` (negatif değerler) | uygulandı; ≤ 0 değerler çizilmiyor ("131 value(s) not drawn"), çizim normal |
| `opacity 7`, `width 1e308`, `size -1` | `invalid` (ilk hata: opacity) |
| Seçicide bilinmeyen anahtar `{"study":"rsi","foo":1}` | `schema` "unknown field `foo`", `path /ops/0/view/foo` |
| 60 kat iç içe `{"on":{"on":…}}` | `not_found`, tam yol (`/…/on/on/…`) |
| `{"panel":{"panel":{}}}` katman seçicisinde | `schema` "unknown field `panel`" (doğru yol) |
| `first` + `latest` | `invalid` |
| `move_layer after` kendisi, `move_panel before` kendisi | `invalid` "cannot be placed relative to itself" |
| Var olan katman kimliği, 65 karakter kimlik | `duplicate`, `invalid` |
| `set_view_range from 150 to 10` | `invalid` |
| `y: "1e308"` (sabit dizge) | `no_origin` |
| `hit x=-1e9, y=1e300` | `{"view":"main"}` (panel/öğe yok) |
| `hit width 0` / `1e8×1e8` | `invalid` "size … out of range (200x120 ..= 16384x16384)" |
| `study` seçici `params.period 1e308` | `not_found` |
| `1e999` sayısı | `parse` "number out of range" |
| 300 kat iç içe dizi | `limit` "recursion limit exceeded" (serde 128) |

## REPORT'ta abartılı ya da tutarsız

1. "Durum: Tamamlandı (denetim sonucu aşağıda…)" ve ölçüt 14 ✅ — `521d5f4`'te `AUDIT.md` yok,
   "Denetim" bölümü boş. `STATUS.md` ("14/14 … denetim sonucu `AUDIT.md`") ve `ROADMAP.md`
   ("denetim `rounds/R2/AUDIT.md`") da aynı.
2. Ölçüt 7 ✅ — V3 yelpazesi ekranlı bağlantılarda okunmuyor (Bulgu 1).
3. Ölçüt 6 ✅ "yakın doğru seçeneği öneriyor" — adaysız durum var (Bulgu 3).
4. Ölçüt 1 ✅ — sözlükte iki yeni kavram eksik (Bulgu 4).
5. Ölçüt 13 / A13 — 595.475 B (+120.349) ile "593,2 KB (+116,6 KB)" aynı ölçüm için iki sayı;
   temiz kopyada 595.584 B.

Sayıların geri kalanı (verify 14/14, 27 × 3 döküm, 23 şema, 49 MCP aracı — verify ve test
çıktısında; T9 tablosu; 100k kare; hesap özeti; 18 durum) yeniden üretildi.

## Denetimde kullanılan komutlar (özet)

```
QP_VERIFY_BROWSER=1 scripts/verify.sh                       # EXIT=0, 14/14
scripts/preflight.sh; scripts/qgh run list --branch r2/int  # (ana kopyadan) success
qp render --transcript fixtures/views/<v>.json <seçim> --out a|b.png ; sha256 karşılaştırma
cargo test --release -p qp-session --test concurrency -- --nocapture
cargo test --release -p qp-session --test presenter
cargo run --release -p qp-render --example frame_100k
python3 rounds/R2/ergonomics/harness/score.py v1 ; score.py v2 A ; git status
qp serve --init fixtures/protocol/init/thyao.json            # VERIFY 7, 8, 10, 11, düşmanca girdiler
git diff 506bdf0 521d5f4 -- fixtures/golden/sample.png fixtures/golden/stress.png crates/qp-compute NORTH_STAR.md
```
