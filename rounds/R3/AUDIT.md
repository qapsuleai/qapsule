# R3 — Son denetim

Denetçi: bağımsız alt ajan (bu round'un hiçbir işini yazmadı). Denetlenen: `r3/int` @
`92b0d84` (`main..92b0d84`: 121 commit, 1.706 dosya). Dal `r3/audit` (`92b0d84`'ten), ayrı
worktree. Bu dosya ve `rounds/R3/audit/` (denetçinin betikleri) dışında hiçbir şey
commit'lenmedi; yeniden puanlama ve benzetimin yeniden yazdığı dosyalar bayt eşit çıktı, derleme
sırasında değişen üç düzenek `Cargo.lock`'u geri alındı (`git checkout`). `.env.local`
açılmadı; gerçek veriden depoya hiçbir şey kopyalanmadı (yalnız toplu sayı). Ortam: Apple
silicon, Darwin 27.0.0, rustc 1.92.0, Node 24.3.0, `CARGO_INCREMENTAL=0`, denetçinin derleme
klasörleri scratchpad'de (iş bitince silindi).

## Hüküm: **koşullu kabul**

Makine istenen işi yapıyor ve iddiaların büyük çoğunluğunu bağımsız olarak yeniden ürettim:
`QP_VERIFY_BROWSER=1 scripts/verify.sh` **15/15 PASS**; 44 döküm × 3 taşıma, 01–36 `main`
ile bayt eşit; hesap özetleri `7b1e99d40889f83c` (arm64, x86_64, wasm32) ve
`914fcfdb9f653a6d`; yedi dedektörü kendi verimle ve kendi formül uygulamamla yeniden
hesapladım (11.592 tespit, karar farkı 0); kendi dört ileriye bakma bozmam 12/12 farksız;
aday izsiz; K26 iki dilde ve "şu an" değeri benim RSI hesabımla aynı; AUD1–AUD3 (100 tohum)
ve hassasiyet benzetimi bayt eşit; M2/M5 hamdan yeniden puanlama bayt eşit; düzenekler
alternatiflerden önce; performans ±%3 içinde.

**Ama** VERIFY 10'un "yetkisize sayısal değer gitmez" iddiasını kırdım: kendi
**girişimsizlik** denememde (aynı açık barlar, yalnız lisanslı akış farklı) akış yetkisi
olmayan Bora'ya ve hiç veri yetkisi olmayan Can'a giden **açık** bulguların `confidence`
alanı, gizlenen lisanslı akış olgusunun gücünü tam `f64` olarak taşıyor
(`(|net|/avg5 − 2)/2`); mühürün ≤ log2 7 bit sınırını tamamen atlıyor (bulgu **Y1**).
Değere dayalı taramalar (ekibinki ve benimki) bunu göremez; yakalayan girişimsizlik
karşılaştırması oldu.

Koşullar (`main`'e birleştirmeden önce):

1. **K1 — Lisanslı olgunun açık hikâyeden sızması (Y1, O1).** Yetkisiz okuyucuya giden
   belge/günlük/`list`/`changes` izdüşümünde açık bulgunun `confidence`'ı, kapsülün
   `importance`/`tier`'ı (ve sıralaması) gizlenen lisanslı olgulardan bağımsız olmalı
   (bugün `project_story` yalnız hikâyenin içini düzeltiyor; `Finding.confidence =
   story.strength` ve kapsül alanları düzeltilmiyor). Aynı açık veri + farklı lisanslı akış
   tohumlarıyla Bora ve Can'ın akışının (mühür iletileri dışında) bayt eşit olduğunu sınayan
   bir girişimsizlik testi eklenmeli; K33 gereği bilerek bırakılan kanal varsa (karne
   istatistikleri, sıralayıcının `ledger_all`/`stories_all`'u) testte açıkça istisna olarak
   adlandırılmalı ve `DECISIONS.md`'ye (Ö37'nin "olgunun ve kapsülün varlığı ayrıca gitmez"
   cümlesiyle çelişmeyecek biçimde) yazılmalı.
2. **K2 — Kapanış belgeleri (ölçüt 18).** `STATUS.md` ("R3 koşulmayı bekliyor") ve
   `ROADMAP.md` ("Hazır, koşulacak") güncellenmemiş; `REPORT.md` "Denetim" bölümü boş. REPORT
   ölçüt 9 ve 10 K1 giderilene kadar ⚠️ olmalı; Y1 ve düzeltmesi REPORT'a yazılmalı.

## VERIFY tablosu

| # | Kontrol | Sonuç | Kanıt |
|---|---------|-------|-------|
| 1 | `QP_VERIFY_BROWSER=1 scripts/verify.sh` | **PASS** | Çıkış 0, 15/15, 322 s (özet aşağıda); testler 95 ikili, 456 geçti, 0 kaldı, 2 yok sayıldı |
| 2 | preflight; CI | **PASS / yok** | `scripts/preflight.sh` → "preflight: TAMAM" (origin, ssh, qgh hesabı `qapsuleai`, yazma izni, e-posta). `r3/int` itilmediği için CI koşusu yok; `scripts/qgh run list --limit 3`: `main` son koşu `completed success` "Merge RP2…" (43732887160, 2026-11-15T15:59Z). İtme yapılmadı |
| 3 | `qp conform` üç taşıma | **PASS** | `qp conform fixtures/protocol/conformance/*.json --transport inproc|serve|mcp` → 44/44 her birinde. `git diff main -- …/0* …/1* …/2* …/3[0-6]*` boş (36 eski döküm bayt eşit). Yeni: 37 `dispute-retract`, 38 `capsule-lifecycle`, 39 `capsule-entitlement`, 40 `capsule-k26-sentence`, 41 `capsule-sensitivity`, 42 `capsule-candidate`, 43 `capsule-retract`, 44 `capsule-selectors-bound` |
| 4 | Altın görseller, hesap özeti | **PASS** | `git diff --diff-filter=MD main -- fixtures` boş (yalnız eklemeler: 6 yeni altın, `digest-r3.txt`, c1–c5 görünümleri ve açıklamaları, üç belge); verify 6/7/10 altınlarla bayt eşit; `qp-compute` `src` farkında tek silinen satır `pub use` genişletmesi; `cargo run -p qp-compute --example digest` → `7b1e99d40889f83c`, `-- r3` → `914fcfdb9f653a6d` (`fixtures/compute/digest-r3.txt` ile aynı); verify 12: arm64, x86_64, wasm32 `7b1e99d40889f83c` |
| 5 | Toplu = akışkan; günlükten yeniden oynatma | **PASS** | verify 3 içinde `qp-facts` bit-eşitlik testleri, `replaying_the_log_reproduces_the_capsule_history`, `history_replay_reproduces_capsule_history` geçti. Kendi ek kontrolüm (`session_probe`, kendi barlarım, 200 bar, lisanslı akışlı): `qp_session::replay(initial_document, log)` = canlı belge (bayt); aynı girdinin ikinci koşusunda günlük (144 kayıt, 68 kapsül) ve Ana/Bora'ya giden bütün satırlar bayt eşit. (`run_batch` tanım gereği `push` döngüsü; yeniden hesabım onu besliyor) |
| 6 | Bağımsız yeniden hesap | **PASS** | `recompute.py` (Python stdlib, `qp-facts` çağrılmaz, verify-e1 betiğine bakılmadı) + kendi bar üretecim `gen_bars.py`: 2 veri kümesi (6×1.500 + 4×800 bar), yedi dedektörde **11.592 tespit, yalnız-denetçi 0, yalnız-motor 0**; kanıt/güç değerleri en çok 7,6e-11 göreli fark (benim kayan olmayan toplamlarım). Ayrıntı aşağıda |
| 7 | K26 | **PASS** | Ürün API'siyle (`Session`, kendi barlarım, döküm 39 kurgusu), `cap.THYAO.extreme.up.163`: TR "THYAO'da RSI 35 bar önce 70'in üstüne çıktı, 32 bar önce yeniden altına indi; şu an 47,4, artık 70'in üstünde değil…"; EN "35 bars ago, THYAO's RSI crossed above 70; it fell back below 32 bars ago and is now 47.4, no longer above 70…". Bağımsız RSI'm: son kapanmış bar 198'de 47,4278; giriş 163, çıkış 166 (198−163 = 35, 198−166 = 32) |
| 8 | Kendi ileriye bakma testim | **PASS** | `lookahead` (`EngineConfig::r3()`, 8 enstrüman × 400 bar, kendi barlarım): `truncate`, `crash_one` (yalnız bir enstrümanın geleceği çöker), `new_inst` (kesimden sonra yeni enstrüman), `flow_flip` (yalnız gelecekteki lisanslı akış) × 3 kesim = 12 durum; kesimden önceki olaylar (629/1.417/2.162), kayıtlı satırlar, doğum karnesi ve kesimden önceki 6 zamanda her anahtarın `ledger_stat`'ı **12/12 eşit**; gelecek her durumda gerçekten değişti |
| 9 | Aday izsiz | **PASS** | `session_probe`: açık bara yamayla ×50 hacim, ×1,2 kapanış/tepe → 3 aday (`breakout.up.198`, `continuation.up.198`, `extreme.up.198`), sonra yama geri alınıp bar kapandı. Sıçramasız koşuyla kapsüller, karne, olgular ve günlükteki motor kayıtları eşit; günlükte aday kimliği ve `"candidate"` yok |
| 10 | Yetkisiz + lisanslı | **FAIL (Y1)** | Değer taraması geçiyor: Bora 357, Can 356 satır; ham lisanslı akışın 821 değeri (`net`, `buy`, `sell`, `brokers_net_top5`, her bar), 31 lisanslı olgu ve 14 lisanslı hikâye kimliği, 85 özet → 0 sızıntılı satır (pozitif kontrol: Ana'da 61 değer görünüyor); varlık kâhini yok (`capsule`, `get`, `verify_evidence`, `approve_finding` gerçek = sahte kimlik cevabı). **Ama girişimsizlik kırık:** akış tohumu 1 ↔ 2/7/42, Bora ve Can'a giden son 16 sorgu cevabının 9–12'si farklı; açık `fnd.THYAO.breakout.up.30`'ın `confidence`'ı 0,3298 ↔ **0,7782682859251837** = gizli `fx.THYAO.flow.institutional.net_buy.30`'ın gücü (Y1) |
| 11 | Kendi geri çıkarma saldırım | **yapıldı; raporla tutarlı (sınır), K1 ile atlanıyor** | Tam Bayesçi çok-bar saldırgan (aşağıda): ortalama 0,51 bit/hedef bar, p90 1,39, p99 3,90, en çok 5,71 (iletili bar); bilinen-diğerleri kontrolünde en çok 5 farklı döküm (≤ 7). Yapı gereği sınır ve "çok bar" uyarısı doğrulandı; ama Y1 mühürü tamamen atlıyor |
| 12 | AUD1–AUD3 | **PASS (AUD2 hariç iyileşme)** | `harness run --approach bs|bs2 --spec aud1/2/3 --seeds 100` (100 tohum, hepsi): M1 = M2 = M3f = M3p = 0 her ikisinde; M5 kaçırılan kesin iddia `bs` 7/49/7 → `bs2` **0/53/0**. Satırlar `raw/bs2/table-*-aud.txt` ile aynı. AUD2 iyileşmedi (REPORT'ta açık) |
| 13 | Hassasiyet benzetimi | **PASS** | `cargo run --release -p qp-capsule --example sensitivity -- synthetic` → `S/sensitivity.{md,json}` yeniden yazıldı, `git diff` boş (bayt eşit) |
| 14 | M1–M7 ham kayıtlar | **PASS (notla)** | SPEC `a35d726` 20:01, M1/M2 düzeneği `71fd241` 20:42, M3–M6 düzeneği `176f77e` 20:59; 14 alternatif dalının ilk commit'i 21:00:22 (m2b) … 22:34 (m6a), hepsi SPEC'i ve ilgili düzeneği ata olarak taşıyor; M7: düzenek+taban `0f0a49e` (20:33) uygulamadan (`ab36ad0`) önce. Her alternatif dalı yalnız kendi `M<k>/<ad>`, `M<k>/raw/…` ve düzenekte kayıt satırı (+1–3 satır `registry.rs`, `Cargo.toml`/`.lock`) değiştiriyor. Yeniden puanlama: `harness-sim m5 score` (barrier, semantic, reference, control) ve `harness m2 score` (template, llmslot, control) → yazılan özetler bayt eşit. Birbirini görmeme kayıttan kanıtlanamıyor (REPORT Sapma 4 bunu söylüyor) |
| 15 | Görseller ve üç derinlik | **PASS (bulgularla)** | Aşağıda "Görseller"; O2, D4 |
| 16 | İnsan testi paketi | **PASS (eksiklerle)** | Kitapçık A'yı katılımcı gibi doldurdum, `score.py` ile puanladım; D3 |
| 17 | Gerçek veri izi | **PASS** | `git ls-files \| grep fixtures/local` boş. `realtrace.py` (5 sembol, 18 seri; 3.179 izlenen metin dosyası): ≥ 3 ayırt edici gerçek hacim içeren dosya **0**; ardışık gerçek kapanış dizisi 1 dosya (`rounds/RP2/experiments/raw/t5-fixes/after-wasm-release-run2.txt`, RP2'den, `main`'de aynı; ölçüm tablosu, eşleşmeler 5 anlamlı haneli süre sayılarıyla tesadüf). Pozitif kontrol: 6 gerçek bar içeren geçici dosya iki ölçüyle de yakalandı (2 anlamlı haneli düzeltilmemiş fiyatlar ayırt edici değil; sınır) |
| 18 | Gizli bilgi; eski klasörler | **PASS** | `git grep -nIE "jwt [A-Za-z0-9]|eyJ[A-Za-z0-9_-]{10,}|BEGIN (RSA|EC|OPENSSH) PRIVATE"` boş; `git diff main -- rounds/R0 rounds/R1 rounds/RD1 rounds/RP1 rounds/R2 rounds/RP2` boş; `NORTH_STAR.md` farksız; `../Docs`, `../Services`'te 2026-11-15 12:00'den sonra değişen dosya yok |

`verify.sh` özeti:

```
1 fmt PASS · 2 clippy PASS · 3 test PASS · 4 schema PASS (28 schemas) · 5 fixtures PASS (4)
6 golden PASS · 7 views PASS (12 views x2) · 8 wasm PASS (raw 1943806 B, gzip -9 616022 B)
9 hygiene PASS · 10 browser PASS (headless Chromium) · 11 compute PASS
12 digest PASS (arm64, x86_64, wasm32: 7b1e99d40889f83c) · 13 protocol PASS (61 examples)
14 conform PASS (44 x inproc, serve, mcp) · 15 replica PASS (lag 0, 25; native = wasm32)
total: 322s · verify: OK
```

## GOAL kabul ölçütleri

| # | REPORT | Denetim | Kanıt / not |
|---|---|---|---|
| 1 | ✅ | **PASS** | 7 dedektör, 6 tür; bağımsız yeniden hesap 0 karar farkı (VERIFY 6); bit-eşitlik testleri; sözlük önce (VERIFY-E1) |
| 2 | ✅ | **PASS** | VERIFY 9 (kendi denemem) |
| 3 | ✅ | **PASS** | Aileler ve teyit sayısı kodda ve işarette (`confirmations` = aile sayısı); ama Y1: gizli aileden gelen güç açık bulguya yansıyor |
| 4 | ✅ | **PASS** | Dökümler 38, 40, 43; yeniden oynatma testleri; kendi koşumlarım belirleyici |
| 5 | ✅ | **PASS** | VERIFY 8 (kendi testim) + geri çekme testleri |
| 6 | ✅ | **PASS** | VERIFY 7; lisanslı cümle rakamsız (taramam 0). Not O2: açık hikâyede "Artık geçerli değil" |
| 7 | ✅ (ön eleme) | **PASS** | `g1` 6 tür × 3 yön × 5 aşama, açık/koyu; `glyph` işaret türü, eski görünümler bayt eşit |
| 8 | ✅ | **PASS** | c1/c2/c3 aynı kapsül; c3/c4/c5 geometrisi olgu kanıtından (`explain`: `/encoding/y = 135.56 from fx…level`) |
| 9 | ✅ | **FAIL → K1** | Yetkisiz okuyucuya lisanslı olgudan türeyen sayı gidiyor (Y1); REPORT'un "her sorgu/konu/sunum yolu" taraması türetilmiş değeri kapsamıyor |
| 10 | ✅ | **PARTIAL → K1** | Uçtan uca akış, §20, saldırı sonucu raporda (doğruladım); ama seçilen güven yolu ürün izdüşümünde atlanıyor |
| 11 | ⚠️ | **PARTIAL (dürüst)** | AUD1/AUD3 7 → 0 ve yanlış gösterim 0 yeniden üretildi; AUD2 49 → 53 |
| 12 | ✅ | **PASS** | Benzetim bayt eşit; seviye kodda sabit (`the_engine_never_changes_the_audience`) |
| 13 | ✅ (notlarla) | **PASS** | VERIFY 14; sapmalar REPORT'ta açık |
| 14 | ✅ | **PASS** | Dört rejimli üreteç; gerçek veri ayrı tablolarda; VERIFY 17 |
| 15 | ✅ | **PASS** | VERIFY 16; eksikler D3 |
| 16 | ✅ | **PASS** | §19.9, §20; 44 döküm × 3; C01–C20 (ilk deneme 12/20 ve 11/20, son 19/20 ve 20/20, REPORT'ta açık) |
| 17 | ✅ (notla) | **PASS** | `capsule_tick -- 30 1000 600 3 none,caps,flow bound`: düzeltme tiki 2,311 µs/olay (REPORT 2,27), kapanış 49,3 ms (48), bağlanma 136 ms (133), akışla kapanış 90,8 ms (90), bağlanma 247 ms (247). WASM 616.022 B (REPORT 616.021; 1 B). Bilgi B1 |
| 18 | ✅ | **FAIL → K2** | `STATUS.md`, `ROADMAP.md` güncel değil; REPORT "Denetim" boş; `docs/CAPSULE.md` yalnız eklemeyle (112+/0−) güncel |

## Bağımsız yeniden üretimler

### Dedektörler (VERIFY 6)

`gen_bars.py` (denetçinin LCG'si; rejimler: yükseliş, düşüş, dar bant + sert çıkış, oynak,
testere; %3 hacim sıçraması; ayrı "net" serisi: %4 nokta yok, %2 `net` yok, %5 ×6 sıçrama) →
`facts_export` (yalnız `run_batch` çağırır) ↔ `recompute.py` (her pencere sıfırdan, iki geçişli
std; Wilder RSI/ATR; tepe/dip teyidi `j+2`'de; kurumsal pencere = önceki 5 gözlem).

| Dedektör | Tohum 7 (6×1.500) | Tohum 11 (4×800) | En büyük göreli fark |
|---|---:|---:|---:|
| `continuation.ma_stack` | 899 = 899 | 310 = 310 | 7,6e-11 |
| `reversal.rsi_divergence` | 220 = 220 | 71 = 71 | 0 |
| `breakout.range` | 1.914 = 1.914 | 662 = 662 | 0 |
| `squeeze.bandwidth` | 118 = 118 | 36 = 36 | 6,4e-10 |
| `extreme.rsi` | 755 = 755 | 238 = 238 | 0 |
| `flow.volume_surge` | 225 = 225 | 84 = 84 | 4,2e-13 |
| `flow.institutional` | 1.499 = 1.499 | 527 = 527 | 2,3e-13 |

Tür başına birer örnek (tohum 7, AUD0): kırılım yukarı bar 788 (seviye 18,00633, kapanış
18,120777, ATR 0,27028, güç 0,423436); ma_stack ayı bar 746; aşırı alım bitişi bar 884 (önceki
73,505171 → 64,939529); kurumsal net alış bar 747 (net 45.411,98, avg5 10.919,068); hacim
sıçraması bar 615 (oran 6,703369); ayı uyumsuzluğu bar 908 (p1 896, p2 906, RSI 82,769 → 79,571);
sıkışma girişi bar 784 (bw 0,068917 = bw_min). Hepsi motorla aynı bar ve değer.

### Girişimsizlik (VERIFY 10'un kırıldığı yer)

`session_probe`: döküm 39 kurgusu (Ana `*`, Bora `bist/THYAO`, Can hiçbir şey), barlar
benim; 200 bar; Bora ve Can `capsules`, `capsules {all}`, `capsules {instrument}`,
`changes`, `claims` konularına abone ve sonunda 16 sorgu (`capsules` beş biçim, `ledger` iki,
`describe`, `log`, `document`, `list` facts/capsules/findings, `present` iki, `verify_evidence`).
İkinci koşu yalnız `init.capsules.flow[0].seed` farklı. Mühür iletileri (`trust: server`,
`comments`, `server_comment`) çıkarıldıktan sonra:

| Tohum | Bora: farklı son cevap / 16 | Can: farklı / 16 | Farklı alanlar | `fnd.THYAO.breakout.*` `confidence` farkı |
|---|---:|---:|---|---:|
| 1 ↔ 2 | 10 | 9 | `confidence`, `importance`, `tier`, sıra, karne `n`/`realized`, Bora'da `claim` | 8 bulgu |
| 1 ↔ 7 | 11 | 10 | aynı + `capsules {meaning: flow}` `importance` | 5 |
| 1 ↔ 42 | 12 | 11 | aynı | 4 |

Örnek (tohum 1 ↔ 2, Bora ve Can'ın `list findings` cevabı): `fnd.THYAO.breakout.up.30`
`confidence` 0,32982528358072916 ↔ 0,7782682859251837. Ana'nın belgesinde hikâyenin olguları:
`breakout.range.up.30` (güç 0,3298) + **`flow.institutional.net_buy.30`** (teyit, güç 0,0641 ↔
**0,7783**) + `breakout.range.up.31`. Yani `confidence = max(güç)` gizli akış olgusundan geliyor:
`|net|/avg5 = 2 + 2·0,7782682859251837` tam olarak geri çıkıyor; 1,0 ise oranın ≥ 4 olduğunu
söylüyor. `fnd.THYAO.breakout.up.38`: 1,0 ↔ 0,5622 (tohum 2'de akış olgusu yok → yalnız
kırılımın gücü): olgunun **varlığı** da okunuyor. Kod: `crates/qp-session/src/capsule.rs`
`confidence: story.strength.clamp(0.0, 1.0)` (PutFinding) ve `project_doc` bulgu/kapsül
alanlarını izdüşürmüyor (`project_story` yalnız `Story`'yi).

### Geri çıkarma saldırısı (VERIFY 11)

`inversion` — düzeneğin kâhin saldırısından farklı: saldırgan diğer barların değerini
**bilmez**; bütün döküm dizisini (sessiz / net alış / net satış) birlikte kullanan tam Bayesçi
çok-bar saldırgan. Mühürün iç durumu (son 5 kırpılmış ikili düzey) gizli Markov durumu; durum
modeli gerçek dökümü bire bir üretiyor (doğrulandı). Önsel, saldırganın da gördüğü açık veriye
koşullu (`floor(log2|net|) − floor(log2 hacim)` ve getiri işareti; 6 ayrı eğitim tohumu). Hedef
bar için sonsal: ileri süzgeç + `L = 8` bar etiketli ileri süzgeç. Veri `synthetic_market(6,
300, 1)` + `synthetic_flow`; 300 hedef (74 iletili, 226 sessiz).

| Ölçü | Değer |
|---|---|
| KL(sonsal ‖ önsel), bit/hedef bar | ort **0,51**, medyan 0,23, p90 1,39, p99 3,90, en çok **5,71** |
| iletili barlar / sessiz barlar | ort 1,39 (p90 2,01, en çok 5,71) / 0,22 (p90 0,40, en çok 1,72) |
| H(önsel) − H(sonsal) | ort 0,63, p90 1,63, en çok 1,85 |
| %90 güven kümesinin daralması (|net| aralığı, log2) | ort 0,37, p90 0,59, en çok 1,0 |
| En olası düzey = gerçek düzey | sonsalla 100/300; yalnız açık veriyle (önsel) 84/300 |
| Kontrol: diğer barlar bilinirken hedef taranır (−60…79 düzey × işaret) | en çok **5** farklı döküm (≤ 7) |

Sonuç: REPORT/M6 ile **tutarlı**. Ortalama bilgi (0,51 bit) yapı gereği sınırın (log2 7 ≈ 2,81
bit) çok altında; bilinen-diğerleri kontrolünde döküm sayısı ≤ 7. Noktasal kazanç tek tek
iletili barlarda 2,81'i aşabiliyor (en çok 5,71; M6'nın "çok bar / dar kova 5,42 bit" uyarısıyla
aynı sınıf). Saldırgan |net|'in ikili büyüklüğünü en fazla bir kova daraltabiliyor. **Ancak** Y1
yüzünden ürün yolunda saldırıya gerek yok: açık bulgunun `confidence`'ı oranı tam veriyor.

### Görseller (VERIFY 15)

Okunan: `g1-glyphs-light/dark.png`, `c2-capsule.png`, `c3-chart.png`, `c4-band.png`,
`c5-points.png`, `c1-sentence.json`, `c2/c3.explain.txt`.

- **g1:** 18 satır × 5 aşama, açık ve koyu temada okunaklı; tür şekilleri birbirinden ayrık;
  aday kesik çizgi, kesin dolu renk, kapanmışlar gri (✓, ×, üstü çizili). Teyit blokları 24 px'te
  ~2 px; koyu temada gri zemin üzerinde zayıf. Sıkışmanın yukarı/aşağı biçimleri var ama
  dedektör yalnız yönsüz üretiyor (B3).
- **c1 ↔ c2 ↔ c3 aynı kapsül** (`fnd.SYN03.breakout-up.144`): cümle her iki derinlikte aynı
  ("6 bar önce 135,56'daki direnci … (kapanış 136,20) … 4 bar önce aynı aileden yeni bir kanıtla
  güçlendi. Artık geçerli değil. … 7 seferin 3'ünde"); karne "Karne 3/7" ile tutarlı; c3'te
  vurgulanan bar 144'ün kapanışı 136,2 ve son bara 6 bar var; seviye çizgileri 135,56 / 136,24 /
  139,54 olgu kanıtından (`explain`). Tutarsızlıklar: (a) kapsül açık ve işaret "kesin" (renkli
  yeşil) ama cümle "Artık geçerli değil" (O2); (b) c1 açıklaması "confirmed by volume" diyor,
  işaret 2 teyit bloğu gösteriyor, ama cümle hacim teyidini hiç söylemiyor; c2 kartı yalnız ilk
  seviyeyi, c3 üçünü çiziyor (D4).
- **c4:** bant 20 bar geriye sabit dikdörtgen (girişteki üst/alt) olarak çiziliyor; o dönemin
  gerçek Bollinger bandı değil — okur bunu tarihsel bant sanabilir (D4).
- **c5:** p1 (dip ~4,56) ve p2 (dip ~4,43) mumların fitil uçlarında; olgu barı p2+2'de; işaret
  "dönüş yukarı". Doğru.

### İnsan testi (VERIFY 16)

`GUIDE.md`'ye göre Kitapçık A (M = siluet önce), ekran, kendim; formu `FORM.md` biçiminde
CSV'ye geçirip `python3 -I score.py` ile puanladım (`key.json`'a bakmadan). Kirlenme: g1
görselini ve iki alfabenin açıklamasını önceden görmüştüm; puanım üst sınırdır.

| | tür | yön | güç | aşama | tam |
|---|---:|---:|---:|---:|---:|
| siluet (20) | 1,00 | 1,00 | 0,95 | 1,00 | 0,95 |
| rozet (20) | 1,00 | 1,00 | 0,90 | 0,85 | 0,80 |
| cümleler (8) | ne 1,00 | ne zaman 1,00 | — | hâlâ geçerli 0,88 | tam 0,88 |

Anlaşılır; puanlayıcı hatasız çalıştı. Eksikler D3'te.

## Bulgular (önem sırasıyla)

### Yüksek

**Y1 — Lisanslı akış olgusunun gücü açık bulgunun `confidence`'ı ile yetkisize gidiyor.**
`kinddir` teması akış olgusunu açık hikâyeye "teyit" olarak ekliyor; otorite
`Finding.confidence = story.strength` (gizli olgu dahil en büyük güç) yazıyor; izdüşüm
(`project_doc`) bulgunun `confidence`'ını ve kapsülün `importance`/`tier`'ını yeniden
hesaplamıyor. Sonuç: akış yetkisi olmayan (Bora) ve hiçbir veri yetkisi olmayan (Can) okuyucu
`document`, `log`, `list findings`, `changes` yollarıyla `(|net|/avg5 − 2)/2`'yi tam `f64`
olarak alıyor; 1,0 ↔ başka değer farkı olgunun varlığını ve zamanını söylüyor. K33/K36
("yetkisize sayısal kanıt değeri gitmez"), Ö37 ("olgunun ve kapsülün varlığı ayrıca gitmez")
ve ölçüt 9 ihlal; mühürün kanıtlanmış sınırı ürün yolunda anlamsızlaşıyor. Mevcut tarama testi
yalnız kanıt değerlerini ve kimlikleri arıyor, türetilmiş sayıyı göremiyor. → K1.

### Orta

**O1 — Açık hikâyelerin önemi, kademesi, sırası ve karne sayıları lisanslı veriye bağlı.**
Girişimsizlik denemesinde Bora/Can'ın gördüğü `importance` (ör. doğum anında 1,5555 ↔ 1,5581),
`tier`, liste sırası, `ledger`/`by_kind` `n`/`realized` ve `capsules {meaning: flow}` cevabı
lisanslı tohumla değişiyor: sıralayıcının `RankCtx`'i (`stories_all`, `ledger_all`) lisanslı
hikâyeleri sayıyor; karne lisanslı hikâyelerin sonuçlarını içeriyor. Karnenin lisanstan
bağımsız gitmesi K33 kararı; ama bunun lisanslı hikâyelerin sayısını ve kapanış zamanını kaba
bir kanal olarak taşıdığı ve kademe → hassasiyet süzgeci → teslim zinciriyle `capsules` konusunda
hangi açık olayın kime gittiğini değiştirdiği hiçbir yerde yazılı değil. → K1'in parçası
(ya düzelt ya da karar olarak yaz).

**O2 — Açık hikâyede "Artık geçerli değil" ve "kesin" işaret (c2).** Kırılımın koşulu
(3 bar içinde seviyenin üstünde) tutmadığında cümle "Artık geçerli değil" diyor, hikâye açık
kalıyor ve işaret renkli "kesin" aşamasında (zayıflama ya da geçersizlik göstermiyor). REPORT açık
soru 4 cümleyi anıyor, işaretteki çelişkiyi anmıyor. İnsan okuru için aynı kartta iki farklı
"durum". `MeaningCondition` ile beklenen çözümün ayrılması (S-A önerisi) bu round dışı; ama
REPORT'ta işaret tarafı da yazılmalı.

### Düşük

**D1 — Düzeneklerin `Cargo.lock`'ları güncel değil.** `rounds/R3/experiments/harness`,
`harness-sim` ve `M7/harness` derlenince lock değişiyor (M7: `qp-calendar`, `qp-capsule`,
`qp-facts` eklenir). Yeniden üretim sonuçları etkilenmiyor, ama temiz ağaçta derleme ağacı kirletiyor.

**D2 — Mühür anahtarı verilmezse sessizce demo anahtarı.** `Session::new` ve
`Session::with_provider` (gerçek barındırıcı yolu) anahtarsız kurulursa `Muhur::demo` kullanılır;
iletide ya da makbuzda bunun izi yok. R3'te tek akış kaynağı sentetik (`init.capsules.flow`),
bu yüzden bugün etkisi yok; geç `set_trust_keys` doğru biçimde reddediliyor, anahtarlar
belgeye/günlüğe girmiyor. Öneri: lisanslı akış yapılandırılmış ve anahtar yokken
`with_provider`'ın reddetmesi ya da makbuzun "demo" diye işaretlenmesi (A25'e not).

**D3 — İnsan testi paketinde eksikler.** (a) S7 bir fiyat/RSI dönüşü (açık fiyat ailesi) ama
"Sunucu doğruladı; rakamlar kurumsal akış lisansı gerektiriyor" diyor — üründe böyle bir cümle
çıkmaz, katılımcıyı yanıltır. (b) S3'te "Koşul hâlâ açık" için anahtar "evet"; "bilinmiyor" da
savunulabilir (benim tek yanlışım). (c) Ekran gösteriminde kitapçık tek HTML: 1. adımda bir
sonraki ekranda alfabenin açıklaması duruyor; yalnız görevlinin denetimine bağlı (aşağı kaydırma
riski) — 1. adımı ayrı dosya yapmak daha sağlam. (d) Rozet'te çerçeve kalınlığı (kesin ↔
gerçekleşti) ve gri dolgu (aday) 24 px'te zor; benim üç rozet hatam buradan.

**D4 — Derinlikler arası küçük bilgi farkları.** c1/c2 cümlesi hacim teyidini söylemiyor
(işaret 2 aile gösteriyor; c1 açıklaması "confirmed by volume"); c2 tek seviye, c3 üç seviye;
c4'te bant tarihsel değil girişteki genişlik olarak geriye uzatılmış ama bu görselde belli değil.

### Bilgi

- **B1 — Tik maliyeti:** REPORT'un "2,9×" oranı yalnız düzeltme tiklerinde. Kapanışlar dahil
  ortalama olay başına maliyet `caps` 29,7 µs, `flow` 52,8 µs (RP2 yolu `none` 2,14 µs); bar
  kapanışı enstrüman başına 1,6 ms (akışla 3,0 ms). REPORT kapanışı ayrıca veriyor; sadece
  "≤ 3× hedefi" ifadesi bu ayrımı açıkça söylemeli.
- **B2 — WASM:** gzip -9 616.022 B (REPORT 616.021 B), 1 B fark.
- **B3 — Sıkışma yukarı/aşağı işaretleri** alfabe ve altın görselde var, dedektör yalnız yönsüz.
- **B4 — Ajan yazabilirlik:** ilk deneme 12/20 ve 11/20 (son 19/20, 20/20); REPORT açık.
- **B5 — Gerçek veri taraması** düzeltilmemiş 2 haneli fiyatları ayırt edemez (tesadüf
  çok); düzeltilmiş 5–6 haneli kapanışlar ve hacimler için pozitif kontrol geçti.
- **B6 — M3c/M1c bağımsızlık notu** (paylaşılan geçici klasör) kayıtla doğrulanamıyor; dallar
  yalnız kendi klasörlerine yazıyor.

## Güvenlik incelemesi düzeltmeleri (REPORT'taki liste)

| Düzeltme | Denetim |
|---|---|
| İtiraz cevabı varlık sızdırmıyor | Benim varlık kâhini denemem (`capsule`, `get`, `verify_evidence`, `approve_finding`): gerçek = sahte. M7 testleri geçiyor |
| Grafik derinliği kanıt değerini sabit yazmıyor | c3/c4/c5 `explain`'de değerler olgu kökeniyle bağlı; tarama testi lisanslı görünümde değer bulmuyor |
| Kitle yalnız sahibine | Ayrıca kırmayı denemedim; ilgili testler verify 3'te geçti (`project_doc` `audiences`'ı okuyucuya göre süzüyor, kod okundu) |
| Mühür anahtarları kurulumda | `new_with_trust_keys`, `with_provider_and_trust_keys` var; geç `set_trust_keys` reddi testli; D2 |

## Kullanılan komutlar (özet)

```sh
git checkout -b r3/audit 92b0d84
CARGO_INCREMENTAL=0 QP_VERIFY_BROWSER=1 scripts/verify.sh
scripts/preflight.sh; scripts/qgh run list --limit 3
target/release/qp conform fixtures/protocol/conformance/*.json --transport inproc|serve|mcp
git diff main -- fixtures/protocol/conformance/0* …/1* …/2* …/3[0-6]*; git diff --diff-filter=MD main -- fixtures
cargo run --release -p qp-compute --example digest [-- r3]
# denetçi düzeneği (rounds/R3/audit/, ayrı Cargo paketi; CARGO_TARGET_DIR scratchpad)
python3 -I rounds/R3/audit/gen_bars.py bars.json 7 6 1500      # ve 11 4 800, 23 8 400, 5 1 400
cargo run --release --manifest-path rounds/R3/audit/Cargo.toml --bin facts_export -- bars.json eng.json
python3 -I rounds/R3/audit/recompute.py bars.json eng.json
cargo run … --bin lookahead -- la_bars.json
cargo run … --bin session_probe -- <depo> sp_bars.json 200
cargo run … --bin inversion -- 300 8
python3 -I rounds/R3/audit/realtrace.py <depo> <fixtures/local>
bash rounds/R3/audit/history.sh
# yeniden üretimler
harness run --approach bs|bs2 --spec ../specs/aud1.json --spec ../specs/aud2.json --spec ../specs/aud3.json --seeds 100
cargo run --release -p qp-capsule --example sensitivity -- synthetic; git diff rounds/R3/experiments/S
harness-sim m5 score --alt barrier|semantic|reference|control --data synthetic; harness m2 score --alt template|llmslot|control
cargo run --release -p qp-session --example capsule_tick -- 30 1000 600 3 none,caps,flow bound
python3 -I rounds/R3/human-test/score.py answers.csv
git grep -nIE "jwt [A-Za-z0-9]|eyJ[A-Za-z0-9_-]{10,}|BEGIN (RSA|EC|OPENSSH) PRIVATE"; git ls-files | grep fixtures/local
git diff main -- rounds/R0 rounds/R1 rounds/RD1 rounds/RP1 rounds/R2 rounds/RP2
```

## Koşulların giderilmesi (yeniden denetim, 9ae6b50)

Denetlenen: `r3/int` @ `9ae6b50` (K1 düzeltmesi `r3/fix-k1`, tasarım `experiments/S/S-E.md`,
PROTOCOL §20.2/§20.5, testler `crates/qp-session/tests/non_interference.rs`). Dal `r3/audit2`.
Ek denetçi düzeneği: `rounds/R3/audit/src/bin/k1_probe.rs` (yeni).

**K1: kısmen giderildi.** Asıl açık (Y1 ve O1) kapandı: yetkili bir yazarın lisanslı öğeye
dokunan komutu yokken yetkisiz okuyucuların (okuyucu, yazar, öneren, kısmi akış yetkili) bütün
akışı lisanslı veriden bağımsız. **Ama** iki kanal kaldı: (a) açık görünümlü bir yazarın komutu
otoritede yalnız lisanslı olan bir kapsülü değiştiriyor (bütünlük); (b) **yetkili** bir yazarın
yalnız lisanslı öğeye dokunan komutu, açık günlüğe boş bir girdi (`changes: []`, `summary: "…"`,
`cause`, `seq`) olarak geçiyor; girdinin varlığı lisanslı veriye bağlı. (a) koordinatörün
bildirdiği YÜKSEK bulguyla aynı; (b) bildirilen üç bulgunun hiçbiri değil, yeni.

### Kanıt

| # | Deneme | Sonuç |
|---|---|---|
| 1 | Kendi `session_probe`'um, yeni kod (döküm 39 kurgusu, kendi barlarım, 200 bar; akış tohumu 1 ↔ 2/7/42; Bora ve Can; mühür iletileri çıkarıldı) | **0 fark**: Bora 276/276, Can 275/275 satır; son 16 sorgu cevabının 0'ı farklı; kırılım bulgularında `confidence` farkı 0 (eski kodda 8/5/4). Otoritenin dokümanında `fnd.THYAO.breakout.up.30` güveni hâlâ tohuma göre 0,3298 ↔ 0,7783: değer otoritede duruyor ama artık açık görünüm okuyucusuna gitmiyor. Yeniden oynatma, K26, aday ve varlık kâhini sonuçları değişmedi |
| 1b | Aynı koşunun değer taraması | 15 satırda `cap.THYAO.flow.up.124` kimliği var: bu açık dünyadaki **açık** bir hacim sıçraması hikâyesi (olgu `fx.THYAO.flow.volume_surge.surge.124`), otoritedeki lisanslı doğumlu hikâyeyle aynı kimliği taşıyor (S-E §2 kimlik çakışması, belgelendi). Girişimsizlik eşitliği bunun lisanslı veriden bağımsız olduğunu gösteriyor; sızıntı değil |
| 2 | `k1_probe`, **iki enstrüman**, iki lisanslı akış, 5 yapılandırma: (1,1001), (2,1002), yalnız THYAO akışı farklı (7,1001), yalnız ASELS akışı farklı (1,1007), akışsız. Kişiler: Bora (barlar), Can (hiçbir şey), **Dora (barlar + yalnız `flow/THYAO`)**, **Eda (sahip, yalnız barlar)**, **Fatih (öneren, yalnız barlar)**, Ana (`*`, pozitif kontrol). 588 kimlik evreni (bütün koşuların otoritelerindeki olgu/bulgu/kapsül; 153 lisanslı olgu). Okuyucular her kimliğe `capsule`, `get`, `present`, `verify_evidence`, `mode: propose` ile `reject_finding` ve uydurma `dispute` gönderir; `log` (`after_seq` 0/50/100), `ledger` iki biçim, `capsules` dört biçim, `document`, `describe`. Eda her kimliğe doğrudan `approve`/`reject_finding` ve `show_finding`; Fatih her kimliğe öneri; Eda bütün önerileri ve 40 uydurma öneri kimliğini `accept_proposal`. Arkın'ın komutları **kapalı** | Bora, Can, Dora, Eda, Fatih: dört karşılaştırmanın hepsinde **0 farklı satır** (4.416 / 4.415 / 4.414 / 4.829 / 4.128 satır); Ana'nınki 4.490–4.517 satır farklı (pozitif kontrol). Kısmi yetkili Dora'nın akışı THYAO akışı değişince de değişmiyor: S-E'deki "kısmi yetki → açık görünüm" davranışı doğrulandı (yetkili olduğu THYAO lisanslı hikâyelerini de görmüyor) |
| 3 | Aynı, ama bar 120'de **Arkın (sahip, `*`)** evrendeki her bulguya `approve`/`reject_finding` (aynı komut listesi her yapılandırmada) ve bir `show_finding` | **Kırık (b):** Bora/Can/Dora 47 (tohum 2) – 4.266 (akışsız) satır farklı, Eda/Fatih 3–971. İlk fark: Bora'nın `changes` konusunda `{"author":"arkin","cause":"o37","changes":[],"seq":153}` ↔ aynı `seq`'te `cause":"o52"`; Eda'nın `log` cevabında `{"cause":"o37","changes":[],"summary":"…"}` girdileri. Arkın'ın komutunun otoritede uygulanıp uygulanmadığı (= hedeflediği kimliğin otoritede lisanslı doğumlu olarak bulunup bulunmadığı) açık günlükte girdinin varlığı, `cause` ve sonraki `seq` kayması olarak görünüyor. Arkın'ın seçimi aynı; değişen yalnız lisanslı veri. Örn. her yeni bulguyu otomatik onaylayan bir ajan, lisanslı doğumların zamanını açık okuyuculara bar bar duyurur |
| 4 | Açık görünümlü yazarların otoriteye etkisi (otoritenin dokümanında lisanslı doğumlu kapsüllerin `state`/`view`'ı, Eda/Fatih'ten önce ve sonra) | **Kırık (a):** her akışlı yapılandırmada 35–41 lisanslı doğumlu kapsülün **1**'i değişti (akışsızda 0). Açık görünümlü bir yazar, açık dünyada var olan bir kimlik üzerinden otoritede yalnız lisanslı olan kapsülün durumunu ya da görünümünü değiştirebiliyor. Koordinatörün bildirdiği YÜKSEK bulgu (ters çevirme / kimlik çakışması) ile aynı; bağımsız olarak buldum |
| 5 | `AcceptProposal` kâhini (bildirilen ORTA) | Benim kurgumda **görünmedi**: Eda'nın bütün `accept_proposal` cevapları ve sonrası tohumdan bağımsız. Bulunmadı demek; kâhin doğrulamanın otoritede yapılmasından, benim önerilerim (`approve_finding` her kimliğe) bu farkı tetiklemedi |
| 6 | `apply_each` atomikliği (bildirilen ORTA) | Ayrıca denemedim |

### Davranış değişiklikleri

| Değişiklik | Belgede | Denetim |
|---|---|---|
| Çalışan motorun kapsülü §8.1 veri koşuluyla geçersizleşmez (yalnız sonuç kuralıyla kapanır) | PROTOCOL §20.5, S-E §2 ve §2b-1 | Kabul edilebilir: motor kapsülünün yaşamı zaten sonuç kuralına bağlı; motor olmayan hikâyeli kapsül (döküm 37) eskisi gibi §8.1 ile geçersizleşiyor |
| Kısmi akış yetkisi → açık görünüm | PROTOCOL §20.5, S-E §6 | Kabul edilebilir (güvenli taraf), deneme 2 doğruladı; kayıp (yetkili olunan akışın lisanslı hikâyeleri görünmez) S-E §6'da açık, `DECISIONS` açık kararına da yazılmalı (öneri) |
| Her bar kapanışında bir motor girdisi (boş bile olsa), düzeltilmiş cevapta her zaman girdi | PROTOCOL §20.5 (`seq` paylaşımı), S-E §2 | Kabul edilebilir ve gerekli; ama aynı ilke kişinin komutları için tutmuyor (kanal b) |
| Döküm 39 yeniden üretildi | S-E §3 | `git diff 92b0d84 9ae6b50 -- fixtures`: yalnız `39-capsule-entitlement.json` (12+/9−); 01–38 ve 40–44 bayt eşit. Değişen alanlar yalnız Bora/Can: önem 1,490984 → 1,485308, `e2` önerisi, `ledger by_kind` `flow` 7/5 → 2/1 — hepsi açık-yalnız motorun beklenen sonucu |
| Karne açık görünüm okuyucusuna açık-yalnız motordan (K33 anlamı daraldı) | PROTOCOL §20.2, DECISIONS Ö37 notu | Kabul edilebilir; O1'i kapatıyor |

### verify

`CARGO_INCREMENTAL=0 QP_VERIFY_BROWSER=1 scripts/verify.sh` @ `9ae6b50`: **çıkış 0, 15/15 PASS**
(340 s); testler 96 ikili, 459 geçti, 0 kaldı, 2 yok sayıldı; browser PASS; digest arm64, x86_64,
wasm32 `7b1e99d40889f83c`; conform 44 × inproc, serve, mcp; wasm gzip -9 616.022 B.

### K2

Bekliyor: `STATUS.md` ("R3 koşulmayı bekliyor") ve `ROADMAP.md` ("Hazır, koşulacak") `9ae6b50`'de
hâlâ güncel değil. `REPORT.md`'de "Denetim" bölümü yazılmış; ölçüt 9 ve 10 "(K1 sonrası)",
K1 satırı "(aşağıda doldurulur)".

### Güncel hüküm

**Koşullu kabul, K1 açık.** Kapanması için: (a) açık görünümlü yazarın komutu otoritede yalnız
lisanslı olan öğeye hiçbir yoldan dokunmamalı (`r3/fix-k1b`); (b) yetkili kişinin komutundan
doğan, açık dünyada karşılığı boş olan girdi açık okuyucunun gözleyebileceği bir iz bırakmamalı
(ya açık dünyada girdinin varlığı lisanslı veriden bağımsız olmalı — ör. her komut girdisi her
iki dünyada da yazılır —, ya da bu kanal bilinçli bir karar olarak `DECISIONS.md`'ye yazılmalı;
S-E'nin "kişinin seçimi" gerekçesi burada geçmiyor, çünkü seçim aynı, değişen veri);
(c) girişimsizlik testine yetkili yazarın lisanslı kimliklere komutu ve açık görünümlü yazarın
otorite bütünlüğü denetimi eklenmeli (`k1_probe` deneme 3 ve 4). K2 kapanış belgeleriyle.

Komutlar:

```sh
git checkout -B r3/audit2 9ae6b50
CARGO_INCREMENTAL=0 QP_VERIFY_BROWSER=1 scripts/verify.sh
python3 -I rounds/R3/audit/gen_bars.py sp_bars.json 5 1 400
cargo run --release --manifest-path rounds/R3/audit/Cargo.toml --bin session_probe -- <depo> sp_bars.json 200
python3 -I rounds/R3/audit/gen_bars.py k1_bars.json 13 2 200
cargo run --release --manifest-path rounds/R3/audit/Cargo.toml --bin k1_probe -- <depo> k1_bars.json 200            # deneme 3, 4
K1_NO_ARKIN=1 cargo run … --bin k1_probe -- <depo> k1_bars.json 200                                                  # deneme 2
K1_SHOW=b|e cargo run … --bin k1_probe -- …                                                                          # ilk farkı göster
git diff 92b0d84 9ae6b50 -- fixtures
```

## Son kontrol (a808f04)

Denetlenen: `r3/int` @ `a808f04` (`r3/fix-k1b`: açık dünyanın kendi `seq`'i, kişi komutları her
dünyada ayrı, atomik ters çeviri; S-E §7, PROTOCOL §20.5). Dal `r3/audit3`. `k1_probe` genişletildi
(aşağıda); denetçinin girdileri artık yapılandırmadan bağımsız (önceki sürüm Eda'nın kabul listesini
otoritenin dokümanından okuyordu — bir saldırganın göremeyeceği bilgi; düzeltildi, sabit
`prop-1…prop-599` aralığı).

### K1

| # | Deneme | Sonuç |
|---|---|---|
| 1 | `session_probe` (tohum 1 ↔ 2/7/42, Bora ve Can, mühür çıkarıldı) | **0 fark** (276/275 satır; 16 son cevabın 0'ı; `confidence` farkı 0). Yeniden oynatma, aday, varlık kâhini aynı. `cap.THYAO.flow.up.124` satırları kimlik çakışması (açık hacim hikâyesi), sızıntı değil |
| 2 | `k1_probe`, `K1_NO_ARKIN=1` (5 yapılandırma, Bora, Can, Dora kısmi, Eda sahip, Fatih öneren) | Dört akışlı karşılaştırmanın hepsinde açık görünümlü beş okuyucu **0 farklı satır**; Ana değişiyor (pozitif kontrol) |
| 3 | `k1_probe`, Arkın açık: bar 120'de 588 kimlikli evrendeki her bulguya onay/ret ve `show_finding`; bar 125'te 25 `undo` ve Fatih'in her bulguya önerisi + Arkın'ın `prop-1…prop-499` için `accept_proposal`; bar 130'da **Gül (öneren, `*`)** lisanslı her akış bulgusuna `reject_finding` önerisi (yalnız otoritede öneri); sonda Eda kendi olgu+bulgusunu **lisanslı kimliklerle** yazar ve geri alır, `prop-1…prop-599` için `accept`/`reject_proposal` | Açık görünümlü beş okuyucu (Bora, Can, Dora, Eda, Fatih) dört akışlı karşılaştırmanın hepsinde **0 farklı satır**. Önceki kanal (b) (`cause`/`seq` kayması) kapandı |
| 4 | Otorite bütünlüğü: lisanslı doğumlu kapsüller (`state`/`view`), lisanslı akış olgularını taşıyan bulgular ve lisanslı olgular (bayt), açık görünümlü yazarlardan önce ve sonra | Her yapılandırmada **0** değişen (159–179 öğe). Kanal (a) kapsül/bulgu/olgu için kapandı |
| 5 | **Öneri kimliği çakışması** (koordinatörün bildirdiği YÜKSEK, `reverse_changes`'te "aynı koleksiyon" = eşit sayılması) | **Yeniden üretildi.** Arkın kapalıyken, akışlı her yapılandırmada Gül'ün yalnız otoritede olan 25–30 önerisinin **2**'si Eda'nın kararıyla değişti. Örnek: otoritede `prop-150` = Gül'ün `reject_finding fnd.ASELS.flow.down.108` önerisi, `pending` → `rejected`, `decided_by: Eda`, `decided_seq: 545`; Eda açık dünyada başka bir `prop-150`'yi reddetmişti (her dünyanın kendi `seq`'i → kimlikler çakışıyor). Gizlilik bozulmuyor (Eda'nın satırları da tohumdan bağımsız), ama açık görünümlü bir yazar otoritede lisanslı öğeye dayanan, göremediği bir öneriyi karara bağlayabiliyor. `r3/fix-k1c` bekleniyor |
| 6 | Akışsız ↔ akışlı | Bora/Can/Dora 3.888–3.908 satır farklı (Eda/Fatih 0). Tasarım gereği: akış yokken açık dünya yok, okuyucu otoriteyi okur (S-E §7 "Testler"). Lisanslı akışın **var olup olmadığı** yapılandırmadır, lisanslı veri değil; kanal sayılmadı (Bilgi) |
| 7 | Dora'nın ham satırlarındaki `flow.institutional` | 672 satır: **612** Dora'nın kendi gönderdiği lisanslı olgu kimliğini yineleyen `not_found` hataları (153 kimlik × 4 sorgu; `did_you_mean`'de lisanslı kimlik **yok**), **60** mühür eki (`evidence: flow.institutional/net`, `trust: server`; tasarım gereği, değer içermez). Sızıntı değil |

**K1: gizlilik yönü giderildi; bütünlük yönünde bir YÜKSEK bulgu açık (deneme 5, `r3/fix-k1c`).**

### K2

`STATUS.md` (aktif round yok, R3 tamamlandı), `ROADMAP.md` (R3 "Tamamlandı … koşullar giderildi")
ve `REPORT.md` "Denetim" bölümü güncel. **Giderildi**, bir tutarsızlıkla: `REPORT.md` başı ve
`ROADMAP.md` "koşullar giderildi" diyor, ama REPORT ölçüt 9 ve 10 hâlâ "⚠️ (`r3/fix-k1b`
bekleniyor)" ve K1 deneme 5 açık; k1c'den sonra ikisi birlikte güncellenmeli.

### verify

`CARGO_INCREMENTAL=0 QP_VERIFY_BROWSER=1 scripts/verify.sh` @ `a808f04`: **çıkış 0, 15/15 PASS**
(344 s); 459 test geçti, 0 kaldı, 2 yok sayıldı; browser PASS; digest arm64/x86_64/wasm32
`7b1e99d40889f83c`; conform 44 × 3; wasm gzip -9 616.022 B.

### Son hüküm (a808f04)

**Koşullu kabul.** Kalan tek koşul: K1-c — açık görünümlü yazarın komutu otoritede yalnız
otoritede olan **hiçbir** öğeye (öneriler dahil; eşitlik koleksiyon değil öğe düzeyinde)
dokunmamalı; `k1_probe` deneme 5 (Gül'ün önerileri) 0 vermeli ve girişimsizlik testine eklenmeli.
Ardından REPORT ölçüt 9/10 ve "koşullar giderildi" ifadesi güncellenir. Gizlilik (Y1, O1, kanal b)
bu commit'te giderilmiş durumda.

```sh
git checkout -B r3/audit3 a808f04
CARGO_INCREMENTAL=0 QP_VERIFY_BROWSER=1 scripts/verify.sh
cargo run --release --manifest-path rounds/R3/audit/Cargo.toml --bin session_probe -- <depo> sp_bars.json 200
cargo run --release --manifest-path rounds/R3/audit/Cargo.toml --bin k1_probe -- <depo> k1_bars.json 200     # Arkın, Gül açık
K1_NO_ARKIN=1 cargo run … --bin k1_probe -- <depo> k1_bars.json 200                                           # deneme 2 ve 5 (GULDIFF stderr'de)
# ortam anahtarları: K1_NO_ARKIN, K1_NO_EXTRA (undo/kabul), K1_NO_GUL, K1_SHOW=<bağlantı>
```

## K1-c kontrolü (471af9f)

Denetlenen: `r3/int` @ `471af9f` (`r3/fix-k1c`: ters çeviride öğe düzeyinde eşitlik, öneriler
yalnız açık dünyada, niyetler ve itiraz açık dünyada). Dal `r3/audit4`. Denetçi düzeneği
değişmeden (`k1_probe`, `session_probe`; aynı barlar ve girdiler).

| # | Deneme | Sonuç |
|---|---|---|
| 1 | `session_probe` (tohum 1 ↔ 2/7/42, Bora, Can) | **0 fark** (276/275 satır; son 16 cevabın 0'ı); yeniden oynatma, aday, varlık kâhini aynı |
| 2 | `k1_probe`, `K1_NO_ARKIN=1` | Dört akışlı karşılaştırmanın hepsinde Bora, Can, Dora, Eda, Fatih **0 farklı satır**; Ana 4.249–4.256 farklı (pozitif kontrol) |
| 3 | `k1_probe`, Arkın açık (onay/ret, `show_finding`, 25 `undo`, `accept_proposal prop-1…499`), Gül'ün yalnız otoritedeki önerileri, Eda'nın lisanslı kimliklerle kendi öğeleri ve `prop-1…599` kararları | Dört akışlı karşılaştırmanın hepsinde beş açık görünümlü okuyucu **0 farklı satır**; Ana 4.323–4.330 farklı |
| 4 | Otorite bütünlüğü | Lisanslı doğumlu öğeler: her akışlı yapılandırmada **0** değişen (159–179). Gül'ün yalnız otoritedeki önerileri: akışlı her yapılandırmada `changed_by_public_writers` **0** (25–30 öneri; a808f04'te 2). **K1-c giderildi** |
| 5 | Dora'nın `flow.institutional` satırları | Değişmedi: 612 kendi gönderdiği kimliğin `not_found` yankısı (`did_you_mean`'de lisanslı kimlik yok), 60 mühür eki. Sızıntı değil |
| 6 | Akışsız ↔ akışlı | Bora/Can/Dora 3.908 (Arkın kapalı) / 4.390 (açık) satır farklı; Eda/Fatih 0 / 1.552 ve 869. Akışsızda Eda Gül'ün 3 önerisine karar veriyor (`GULDIFF`, 3 değişen) |

**Akışsız farkın değerlendirmesi: kabul edilebilir (yapılandırma; Bilgi).** Akış yokken lisanslı
veri yok, açık dünya kurulmaz ve herkes tek dünyayı (otoriteyi) okur; o kurguda Eda açık
görünümlü değil, Gül'ün önerileri lisanslı öğeye dayanmıyor (akışsızda "flow" bulguları açık hacim
sıçramaları) ve sahip olarak karar vermesi meşru. Ayırt edilen şey yalnız "lisanslı akış
yapılandırılmış mı" — bu zaten mühür iletilerinin varlığından ve PROTOCOL §20.5'ten bellidir,
lisanslı **veri** değildir. Girişimsizlik iddiası (aynı açık veri, farklı lisanslı veri → aynı
satırlar) yapılandırma içinde tanımlıdır ve dört akışlı yapılandırmada 0 farkla tutuyor. Fark
eden içerik (açık görünüm okuyucusu belge kapsüllerinin iddialarını ve açık dünyada aynı olmayan
niyet tetiklerini almıyor) S-E §7'de işlev eksiği olarak yazılı; öneri olarak kalır.

**verify** @ `471af9f`: `CARGO_INCREMENTAL=0 QP_VERIFY_BROWSER=1 scripts/verify.sh` → **çıkış 0,
15/15 PASS** (339 s); 460 test geçti, 0 kaldı, 2 yok sayıldı; browser PASS; digest arm64/x86_64/
wasm32 `7b1e99d40889f83c`; conform 44 × 3; wasm gzip -9 616.022 B.

**K1: giderildi.** Y1, O1, kanal (a), kanal (b) ve öneri kimliği çakışması kapandı; kalan tek
lisanslı veri kanalı tasarım gereği mühür iletileri.

**K2: giderildi** (`STATUS.md`, `ROADMAP.md`, REPORT "Denetim"). Bu kontrolden sonra yalnız
ifade güncellemesi gerekiyor: REPORT başı ve `ROADMAP.md` "K1-c bekleniyor" diyor, REPORT ölçüt 9
ve 10 hâlâ "⚠️ (`r3/fix-k1b` bekleniyor)" — ✅ olarak ve "koşullar giderildi" diye yazılmalı.

### Nihai hüküm: **kabul**

Koşul kalmadı. Öneriler (kapsam dışı, bağlayıcı değil): O2 (açık hikâyede "Artık geçerli değil" ile
"kesin" işaret), D2 (demo anahtarına sessiz düşüş), D4, kısmi akış yetkisinin açık görünüme
düşmesi ve açık görünüm okuyucusunun belge kapsülü iddialarını almaması `DECISIONS.md` açık
kararlarına; girişimsizlik testine akışsız ↔ akışlı karşılaştırmanın neden dışarıda kaldığı notu.
