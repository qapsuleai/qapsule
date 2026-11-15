# R2 — Rapor: Görsel Gramer v0

- **Durum:** Tamamlandı (denetim sonucu aşağıda, "Denetim" bölümünde). Kabul ölçütleri tablosu
  kanıtlarıyla aşağıda.
- **Tarih:** 2026-11-15
- **Alt ajanlar:** Başlatılabildi (ilk iş olarak deneme alt ajanı koşuldu ve cevap verdi).
  Bütün uygulama alt görevleri, iki anlamla başvuru tasarımı, ölçüm ve her alt görevin
  doğrulaması ayrı alt ajanlarla yapıldı; her dal ayrı bir denetçi alt ajan geçtikten sonra
  birleştirildi.
- **Dallar:** bütünleştirme dalı `r2/int`; alt görev dalları `r2/t1-grammar` (T1+T2,
  orkestratör), `r2/t10-compute`, `r2/t5-session`, `r2/t3-grammar`, `r2/t4-render`,
  `r2/t6-reading`, `r2/t4b-views`, `r2/t10b-conform`, `r2/t8a-selectors`, `r2/t9-ergo`; hepsi
  `--no-ff` ile `r2/int`'e. `r2/t8b-names` (elenen tasarım) **birleştirilmedi**, dalda duruyor.

## Ne yapıldı

| # | Görev | Yürüten | Doğrulayan | Çıktı |
|---|-------|---------|------------|-------|
| — | Bekleyen doküman değişiklikleri tek commit | orkestratör | — | `ecbb497` |
| — | Deneme alt ajanı | alt ajan | — | başlatıldı, cevap verdi |
| T1 | Önce altı görünüm ve yirmi görev (`17d8695`), sonra sözlük (`d1e0ea0`), sonra tipler ve `docs/GRAMMAR.md` (`b462ed2`) | orkestratör | T3/T5 denetçileri (dolaylı), son denetim | `rounds/R2/tasks/`, `crates/qp-schema/src/{grammar,scene}.rs`, `docs/GRAMMAR.md`, `docs/CONCEPTS.md` |
| T2 | Protokol eki: kimlikli gramer işlemleri, kullanımdan kalkan indeksli alanlar, `hit`, `welcome.summary`, `no_origin`/`ambiguous`, `ack.resolved`; örnekler | orkestratör | verify adım `protocol` | `protocol.rs`, `docs/PROTOCOL.md`, `fixtures/protocol/{valid,invalid}/` |
| T3+T7 | `qp-grammar`: derleyici, belirleyici log, derinlik kuralları, görüş alanı, LOD | alt ajan | ayrı denetçi | `crates/qp-grammar/`, `LEGACY.md` |
| T4a | Çizici yalnız sahne tarifini boyar; `qp-render`'dan hesap çıkarıldı; wasm; 100k kare | alt ajan | ayrı denetçi | `crates/qp-render/`, `crates/qp-wasm/` |
| T4b | Görünümler yalnız protokol dökümlerinden; yeni altın görseller; satır içi nokta serisi; çıktı başına isabet öğesi; yerleşim düzeltmeleri; verify `views` adımı | alt ajan | ayrı denetçi (görselleri gözle, noktaları veriyle) | `fixtures/views/`, `fixtures/golden/views/` |
| T5 | Otoritede gramer: kanonik biçim, köken, doğrulama, 10 kimlikli işlem, katman düzeyinde çakışma, özet, kanonik sunucu; 18 eşzamanlı durum | alt ajan | ayrı denetçi | `crates/qp-session/` |
| T6 (+T5b) | Veri sağlayıcı, `screen`/`hit` sahne tarifinden, sunucu derinlik kurallarıyla, `qp explain`/`qp scene` | alt ajan | ayrı denetçi | `crates/qp-session/`, `crates/qp-cli/` |
| T8a | Anlamla başvuru — yapılandırılmış seçici nesneler | alt ajan (T8b'yi görmeden) | ayrı denetçi | `select.rs`, PROTOCOL §17, `experiments/T8a.md` |
| T8b | Anlamla başvuru — `@ad` metin dili | alt ajan (T8a'yı görmeden) | T9 ölçümü | dal `r2/t8b-names`, `experiments/T8b.md` |
| T9 | Ajan yazabilirlik: korumalı alanda, rastgele kimlikle; base/A/B; düzeltme; A v2 | alt ajan + 20 ölçülen ajan | ayrı denetçi | `rounds/R2/ergonomics/` |
| T10a | R1/RD1 denetim kalanları | alt ajan | ayrı denetçi | `crates/qp-compute/` |
| T10b | 10 yeni uygunluk dökümü; MCP araç testi | alt ajan | ayrı denetçi (mutasyon dahil) | `fixtures/protocol/conformance/18…27` |
| T11 | Denetim | bağımsız denetçi alt ajan | — | `AUDIT.md` |

## Kabul ölçütleri

| # | Ölçüt | Sonuç | Kanıt |
|---|-------|-------|-------|
| 1 | Gramer tipleri `qp-schema`'da; JSON Schema ve MCP araçları ondan; `GRAMMAR.md` örnekli, elenenler sonda; kavramlar sözlükte (koddan önce) | ✅ (sapma: aşağıda 1) | `qp-schema::grammar` (`View`, `Panel`, `Layer`, `Mark` 8 işaret, `Scale`, `Encoding` kapalı kanal kümesi, `Origin`, `Emphasis`) ve `::scene` (`SceneDesc`); `qp schema export` 23 şema (`View`, `SceneDesc` dahil), iki dışa aktarım aynı (verify `schema`); MCP 49 araç şemadan (RD1: 37; `crates/qp-mcp/tests/tools.rs`); `docs/GRAMMAR.md` §1–16 her kavram örnekli, §17 elenen 13 seçenek; `docs/CONCEPTS.md` "Görsel gramer (R2)" 20 kavram, commit `d1e0ea0` tiplerin commit'inden (`b462ed2`) önce. |
| 2 | Katman, panel, ölçek kimlikle; gramer işlemleri kimlikle; indeksli alanlar kullanımdan kalkmış işaretli; T6c'nin 18 durumunda yanlış 0 | ✅ | Katman `id`, panel numara kimliği (konum değil), ölçek `x`/`y<panel>`; 10 işlem (`insert_layer`, `delete_layer`, `move_layer`, `set_encoding`, `insert/delete/move_panel`, `set_panel`, `set_scale`, `set_emphasis`) + `split_layer`; şemada `add_layer.index` ve `remove_layer` `"deprecated": true`; `crates/qp-session/tests/concurrency.rs`: `base`'siz 18 doğru, `base` ile 16 doğru + 2 güvenli ret, **yanlış 0** (RD1/T6c: körlemesine JSON Patch 10/18, indeksli `remove_layer` 4/18 yanlış). Döküm 23 katman düzeyinde çakışmayı sınar. |
| 3 | R0 mum+SMA, yalnız protokol mesajlarıyla kurulmuş görünümden, altınla bayt düzeyinde aynı | ✅ | `fixtures/views/v0-r0.json` (`add_study` + `put_view`) → `fixtures/golden/sample.png` ile bayt-eşit (verify `views`: "V0 = sample.png"; `crates/qp-cli/tests/views.rs`); ilkel düzeyinde de eşit (`crates/qp-grammar/tests/r0_equivalence.rs`, f32 bit eşitliği); tarayıcı 0 px. `sample.png` ve `stress.png` değişmedi. |
| 4 | Beş yeni görünüm yalnız protokol mesajlarıyla, altın görselleri var: (a) üç panel, (b) Bollinger alan, (c) yelpaze, (d) log, (e) koşullu renk | ✅ | `fixtures/views/v1…v5-*.json` → `fixtures/golden/views/v1-three-panels.png`, `v2-bollinger-band.png`, `v3-fan.png`, `v4-log.png`, `v5-conditional.png`; başlangıç dokümanlarında görünüm yok; her biri iki kez çizilip bayt-eşit (verify `views` 8×2). Denetçi görselleri gözle açtı ve 30 noktayı veriden bağımsız hesapladı (RSI Wilder, MACD, Bollinger, SMA, log eşleme); hepsi 1–2 px içinde. |
| 5 | Derinlik: `sentence`/`capsule`/`chart` gramer kurallarından; sunucu taşındı; `capsule` ve `chart` altın görseli; üç yüzey indirgemesi dökümde; sunucu belirleyici | ✅ | `qp-grammar::depth` (`sentence`, `capsule_view`, `chart_view`, `present`); `show_finding` ve `present` bunu kullanıyor; `present` telefonda `capsule_view` da veriyor. Altın: `d1-capsule.png` (390×300), `d2-chart.png`. Döküm 25: saat → sentence, telefon → capsule (60 bar, kapsül görünümü), masaüstü → chart (+ RD1 döküm 11). Belirleyicilik: 1.000 × 3 derinlik → derinlik başına tek çıktı (`tests/presenter.rs`). Süre (M4 Max): `chart_view` 709 ns medyan, `show_finding` uçtan uca 10,1 µs (RD1: 125 ns / 6,6 µs; artış sağlayıcının olgu/bulguyu kopyalaması ve bar zamanlarını toplaması, aşağıda Öneriler). |
| 6 | Kökeni olmayan işaret reddediliyor (katman, alan, neden, yakın seçenek); geçersiz kanal reddediliyor; testler | ✅ | `no_origin`: `"layer l5 of view main: constant 123.45 at /encoding/y has no origin"`, `path`, `candidates` (panelin eşikleri ve sayısal kanıtlı olgular, yakın değer önce), `hint` (add_fact ya da eşik); RSI panelinde `30` → `rsi.oversold` otomatik bağ. `colour` → `schema` + `did_you_mean: ["color"]` + `expected` kanal listesi. Testler `crates/qp-session/tests/grammar.rs`, dökümler 21 ve 22, `fixtures/protocol/invalid/op.layer_unknown_channel.json`. |
| 7 | `screen` her görünümü özetliyor; nokta sorgusu panel, katman, bar, zaman, değerler, veri başvurusu; veri→piksel→veri özellik testi; ikisi de sahne tarifinden, çizicisiz | ✅ | `qp-session` `qp-render`'a bağlı değil (`cargo tree`); `crates/qp-session/tests/reading.rs`: 6 görünüm × 2 soran × 250 piksel = 3.000 isabet, 781 değer `study`/`get` verisiyle **birebir** aynı; veriden uzak 120 boş alan "öğe yok"; 329 gidiş-dönüş aynı bar ve panel. `qp-grammar` gidiş-dönüş özellik testi (sabit tohum). Döküm 24. |
| 8 | Anlamla başvuru: en az iki tasarım kurulup ölçülmüş; seçilen şartnamede; belirsiz başvuru seçenekli hata | ✅ | T8a (seçici nesneler) ve T8b (`@ad` dili) ayrı dallarda, birbirini görmeden; T9 ile ölçüldü; **A seçildi**, `docs/PROTOCOL.md` §17. `ambiguous`: `"3 layers match {\"mark\":\"line\"}; the authority does not pick one"`, adaylar türün sırasıyla. Günlükte yalnız kimlik (denetçi doğruladı), `ack.resolved`/`result.resolved`. |
| 9 | Ajan yazabilirlik (gramer): ≥20 görev; kimlikler kopyalanamaz; yalnız şartname+şema dosya erişimiyle; ilk deneme, son, mesaj, belirteç; RD1 v2r karşılaştırması; düzeltme ve yeniden ölçüm | ✅ (sınır aşağıda) | `rounds/R2/ergonomics/RESULTS.md`. Ölçülen ajanlar `sandbox-exec` içinde `claude -p` (sonnet): depo okuması `EPERM`; kimlikler ajan başına rastgele; daemon başka ajanın dosyasını reddediyor. Tablo aşağıda ("Ajan yazabilirlik"). Yeniden puanlama bayt-eşit (denetçi). |
| 10 | 100.000 bar kararlı kare süresi < 8 ms; LOD'lu/tam fark; eski viewport/LOD'dan alınan/alınmayan | ✅ | `crates/qp-render/examples/frame_100k.rs` (1200×600, 31 koşu medyanı, M4 Max): **LOD'lu 2,745 ms** (derleme 1,550 + tarama 1,197), LOD'suz 61,9 ms; denetçi yeniden ölçümü 2,78–2,90 ms (±%6). LOD–tam piksel farkı 2.275 / 720.000 (%0,32). `crates/qp-grammar/LEGACY.md`: alınan 7, alınmayan 9 madde. |
| 11 | En az sekiz yeni döküm, üç taşımada; RD1'in 17'si değişmeden | ✅ | 10 yeni (18–27); inproc/serve/mcp 27/27; 01–17 `git diff` boş. Ayrıca oturumun kendi 4 dökümü (`t-select` dahil) üç taşımada. |
| 12 | R1 denetim bulguları: periyot üst sınırı, `Chain` düzeltme hatası ve özellik testi, göreli tolerans; `period: 1e18` iletisi | ✅ | `MAX_PERIOD = 100000`; `period 1e18` → "parameter `period` is 1e18, which exceeds the maximum period 100000"; `Chain::update_last` hatası düzeltmeden önce başarısız olan testle yakalandı, `chain_modes_are_bit_identical` özellik testi (sabit tohum); göreli tolerans + yüksek fiyat testi. Hesap özeti `7b1e99d40889f83c` (aarch64, x86_64, wasm32) değişmedi. |
| 13 | WASM boyutu ölçülmüş, RD1 ile karşılaştırılmış | ✅ | gzip -9: **595.475 B** (RD1 475.126 B; R2 başı `r2/int` 476.603 B): **+120.349 B (+%25)**. Neden: tarayıcı bağlaması R0 sahnesini `qp-grammar` ile kuruyor (derleyici wasm'a girdi). `wasm-opt -Oz` ile 526 KB. Açık karar A13. |
| 14 | `REPORT.md`, `AUDIT.md`; `STATUS.md`, `ROADMAP.md`; önerilen karar satırları ayrı bölümde | ✅ | Bu dosya; `AUDIT.md`; `STATUS.md`, `ROADMAP.md`; `DECISIONS.md` "Önerilen (R2)" Ö10–Ö17, açık A13–A14. |

## Ajan yazabilirlik (T9)

Görevler `rounds/R2/tasks/TASKS.md` (tasarımdan önce yazıldı). 5 ajan × 4 görev, her
değişkende aynı bölüm; her ajan taze süreç, kendi rastgele kimlikleriyle.

| Değişke | İlk deneme | İlk deneme, kimlik bakmadan | Sonunda | Deneme | Mesaj (ilk/hepsi) | Belirteç |
|---------|-----------:|----------------------------:|--------:|-------:|------------------:|---------:|
| base v1 (yalnız kimlik + özet) | 7/20 | 2/20 | 20/20 | 34 | 66/107 | 3.189.676 |
| B v1 (`@ad`) | 17/20 | 10/20 | 20/20 | 25 | 53/67 | 2.642.064 |
| A v1 (seçici) | 19/20 | 19/20 | 19/20 | 20 | 55/55 | 2.506.378 |
| **A v2 (düzeltme sonrası)** | **19/20** | **16/20** | **20/20** | 21 | 50/52 | 2.384.921 |
| RD1 v2r (başka görevler) | 6/20 | — | 19/20 | — | — | — |

- **Seçim:** A. İlk denemede kimliğe bakmadan yazılan doğru mesaj A'da 19/20, B'de 10/20 (beş
  B ajanından ikisi adları bırakıp kimlik keşfine döndü); A en az deneme ve belirteçle.
- **Tasarım düzeltmesi:** A'nın tek başarısızlığı R02 ("üst bandı kırmızı yap"): `{output:
  "upper"}` üç bandı çizen tek katmanı seçti, `set_encoding` üçünü de boyadı ve "uygulandı"
  dedi (B'de de aynı kök). Düzeltme: `output` yalnız o çıktıyı tek başına çizen katmana uyar;
  çok çıktılı katman `invalid` + hazır sıra; yeni işlem `split_layer`; ayrıca `screen` panel
  sırası hatası (aşağıda). v2'de R02 ilk denemede geçti.
- **Kimliksiz ilk deneme 19 → 16 (v2):** tek bir ajan (D) ilk denemesini keşfe harcadı ve
  sonraki üç görevi öğrendiği kimliklerle yazdı (geçtiler ama kimliksiz sayılmadı); notunda
  §17'deki `{"kind":"breakout","latest":true}` seçicisini görmediğini yazıyor.
- **Sınır:** n = 20, değişke başına tek koşu, görev başına tek ajan. A ve B'nin şartname örnekleri
  görev listesi bilinerek yazıldı (ikisine eşit yardım). Korumalı alan okumayı depo ve oturum
  ağaçlarında zorluyor; yazma yalnız bu ağaçlarda yasak (A v1 ve base v1 eski profille koştu;
  dökümler denetlendi: dışarı yazma 0, başka ajana gönderim 0).

## Ölçümler

| Ölçüm | Değer |
|-------|-------|
| `QP_VERIFY_BROWSER=1 scripts/verify.sh` | 14/14 PASS (R2'de `views` adımı eklendi) |
| Uygunluk dökümleri | 27 (17 RD1 + 10 R2); inproc/serve/mcp 27/27; oturumun kendi 4 dökümü de üç taşımada |
| Hesap özeti | `7b1e99d40889f83c` (aarch64, x86_64 Rosetta, wasm32) — değişmedi |
| Altın görseller | `sample.png`, `stress.png` değişmedi; 7 yeni (`fixtures/golden/views/`) |
| Tarayıcı | 0 px (canvas, ekran görüntüsü, wasm, stres) |
| Şemalar | 23 kök şema, iki dışa aktarım aynı |
| MCP araçları | 49 (RD1: 37) |
| `compile` 100k bar | LOD'lu 1,56 ms, LOD'suz 21,6 ms |
| 100k bar kare | LOD'lu 2,745 ms; LOD'suz 61,9 ms; fark %0,32 piksel |
| Sunucu | `chart_view` 709 ns; `show_finding` 10,1 µs; `present` masaüstü 2,5 µs |
| WASM | 595.475 B gzip (+%25 RD1'e göre) |
| Ölçülen ajan belirteci | 10,72 M (v1 + v2, 20 ajan); ~7,6 USD |

## Tasarım kararları (bu round'da verilen, ana akış onayına açık)

1. **Gramer `qp/0`'ın üstüne ek olarak kuruldu**, kırmadan: eski `y` dizisi yüklemede
   `panels`'a çevrilir; panel kimliği **numara** kaldı (eski `panel: 1` anlamını korur);
   katman düz listede (çizim sırası tek liste). RD1'in 17 dökümü bu yüzden değişmeden geçiyor.
2. **Köken katman düzeyinde, göstericiyle** (`origin: {"/encoding/y": …}`); sabit yerinde
   kalıyor (`"y":"70"`). Otomatik bağ yalnız adlandırılmış eşiklere; olgulara asla.
3. **Sahne tarifi `qp-schema`'da** (çizici, okuma ve ileride GPU kabuğu aynı biçim).
4. **Yerleşim gramerde, tarama çizicide;** R0 yerleşimi derleyicide aynen yeniden üretildi.
5. **Belirleyici `log10`** (üs ayrıştırma + sabit adımlı seri); `f64::log10`'a 4–8 ulp.
6. **LOD piksele göre** (`slot < 1`), işaret değiştirilmez (eski prototipin bar sayısına
   göre seviye ve mumu çizgiye çevirmesi alınmadı).
7. **Anlamla başvuru seçici nesnelerle**; günlükte kimlik, cevapta `resolved`.
8. **Ölçümde dosya erişimi işletim sistemiyle zorlandı** (`sandbox-exec` + Unix soketli daemon).

## Sapmalar

1. **Tip taslağı sözlük commit'inden önce yazıldı.** Görünümler ve görevler tiplerden önce
   commit'lendi (`17d8695`); sözlük commit'i (`d1e0ea0`) tip commit'inden (`b462ed2`) önce, ama
   `grammar.rs`'in taslağı aynı oturumda sözlükten önce yazılmıştı. Git sırası bunu göstermez.
2. **Bir alt ajanın worktree'si yanlış tabandan açıldı** (T10a `main`'den; R2 dokümanlarının
   eski sürümünü gördü). İşi yalnız `qp-compute`'taydı ve doğru bulgulara dayanıyordu;
   denetçi geçti. T3 ve T5 de ilk anda eski tabanı gördü ve kendileri `r2/t1-grammar`'a
   taşıdı.
3. **Alan dışı yamalar (onaylı):** T4a, `qp-render`'dan hesabı çıkarırken `qp-cli`'nin render
   yolunu ve `qp-grammar`'ın R0 kâhinini ayrı, etiketli commit'lerde yamadı (kâhin test
   içinde dondurulmuş kopya; asıl uçtan uca kontrol bayt-eşit altın görsel).
4. **Oturumun iç test beklentileri güncellendi** (RD1 dökümleri değil): `describe` işlem sayısı
   22 → 33; `lst` için `did_you_mean` artık `hit`'i de içeriyor; MCP araç sayısı testi 48 → 49
   (`split_layer`).
5. **Yeni altın görseller birleştirmeden önce bir kez yeniden üretildi:** görselleri gözle
   inceleyince başlık çakışması, yinelenen açıklama ve etiket üstüne binen değer etiketi
   görüldü; düzeltildi, yalnız **yeni** görseller (d1, v1, v5) yeniden üretildi. Var olan
   altın görsellere dokunulmadı.
6. **Görev metinlerine küçük ek** (T9): R01'de "RSI(7)", R04/R19'da "son kırılım bulgusu"
   açıkça yazıldı (tablodaki parantez içi bağlam ajana başka yoldan ulaşamazdı).

## Açık sorular (ana akış için)

1. Önerilen karar satırları Ö10–Ö17 kabul mü? Özellikle Ö13 (seçici nesneler) ve Ö11 (köken
   zorunluluğu, otomatik eşik bağı).
2. **WASM +%25** (A13): tarayıcıda gramer derleyicisi mi, yoksa sahne tarifini sunucuda üretip
   wasm'ı yalnız tarayıcı yapmak mı?
3. `qp/1`'de `add_layer.index` ve `remove_layer` kalksın mı?
4. Sunucunun grafik görünümünde katman kimlikleri `l0…l2` (RD1 uyumu) iken kapsül görünümünde
   gramerin kendi adları kullanılıyor; tek kural hangisi olsun?
5. Bulgu adı verilmeyen `present` en yüksek güvenli bulguyu seçiyor; ölçümde iki ajanı yanılttı.
   "En son" mu olmalı?

## Öneriler (kapsam dışı, yapılmadı)

- `welcome.summary`'de Bollinger alanlarının sırası `middle, upper, lower, bandwidth`
  (belgelerde `upper, middle, lower`); sahne tarafı belgelenen sırada. Düzeltilmeli.
- Açıklama yinelenmesi metne göre ayıklanıyor; v2'de bant ve orta çizgi aynı etiketi (değersiz /
  değerli) iki kez gösteriyor. Etikete göre ayıklama ayrı karar.
- Sunucu süresi 125 ns → 709 ns: sağlayıcı olgu/bulguyu kopyalıyor, derleyici her çağrıda bar
  zamanlarını topluyor. Bütçenin çok içinde; ödünç vermek ve önbellek ucuz kazanç.
- Seçicili komut işlem başına dokümanı kopyalıyor (10k barlı dokümanda ölçülmedi; RP2 ile).
- `Op` hem istek hem günlükte saklanan istek; şema saklananın da seçici kabul ettiğini
  gösteriyor. İstek ve saklanan biçimi ayırmak.
- PROTOCOL §17'nin başına "kimlik aramadan ilk mesajda seçici yazılabilir" cümlesi ve
  `summary.select` örneği (ölçümdeki tek ajanın kaçırdığı).
- Sabit fiyatlı seride değer alanı çok dar (±5e-11); okunaklı bir taban açıklık.
- Ölçüm düzeneği: ajan başına ayrı HOME ve varsayılan-yasak yazma profili.
- Bollinger bantlarında uzun seride M2 sürüklenmesi (A14); R1 `BENCH.md` "~1e-13" ifadesi göreli
  olmalı.
- Dondurulmuş R0 kâhin kopyası (test) eski korumasız `nice_step`'i içeriyor (sabit girdiyle).
- R0 sahnesini kuran yardımcı üç yerde (qp-render testleri, qp-wasm, qp-cli); hesaba izinli
  ortak bir kabuk yardımcısı.
- Saat profili 198 px genişlik derleyicinin 200 px alt sınırının altında; 200'e genişletildi.

## Denetim

(T11 sonucu aşağıya yazılır.)
