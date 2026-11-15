# RP2 — Rapor: Canlı Yol ve Veri Bağı

- **Durum:** Tamamlandı. Kabul ölçütleri: **11 ✅, 1 ⚠️** (7: seçilen tasarımın kendini
  onarma yolu `qp/0`'da yok; aşağıda). Bağımsız son denetim: **koşullu kabul**
  (`AUDIT.md`); iki koşul denetimden sonra giderildi ("Denetim" bölümü).
- **Tarih:** 2026-11-15
- **Alt ajanlar:** Başlatılabildi (ilk iş olarak deneme alt ajanı koşuldu ve cevap verdi).
  Bütün alt görevler, dört tutarlılık yaklaşımı, düzenek, ölçümler ve her alt görevin
  doğrulaması ayrı alt ajanlarla yapıldı; orkestratör T2 (sözlük, şema, şartname §18), T7
  karşılaştırması, T10 şartname geçişi ve bütünleştirmeyi yaptı.
- **Dallar:** bütünleştirme `rp2/int`; alt görev dalları `rp2/t1-baseline`, `rp2/t2-binding`,
  `rp2/t2b-fixes`, `rp2/t3-source`, `rp2/t4-session`, `rp2/t4fix`, `rp2/t5-live`, `rp2/t5fix`,
  `rp2/t6-research`, `rp2/t7-harness`, `rp2/t7a-approach` … `rp2/t7d-approach`,
  `rp2/t7-compare`, `rp2/t8-consistency`, `rp2/t9-k25`, `rp2/docs`; denetim dalları
  `rp2/audit-t2`, `rp2/audit-b`, `rp2/audit-t4`, `rp2/audit-t5`, `rp2/audit-final`. Hepsi
  `--no-ff` ile `rp2/int`'e.

## Ne yapıldı

| # | Görev | Yürüten | Doğrulayan | Çıktı |
|---|-------|---------|------------|-------|
| — | Bekleyen doküman değişiklikleri tek commit | orkestratör | — | `9ea22ab` |
| — | Deneme alt ajanı | alt ajan | — | başlatıldı, cevap verdi |
| T1 | Ölçüm tabanı | alt ajan | denetim B (yeniden koşum ±%2,5) | `BASELINE.md`, `experiments/tick-bench/` |
| T2 | Sözlük (önce, `fdd755c`), şema (`qp-schema::data`), §18 | orkestratör | denetim T2–T6 (koşullu kabul; koşullar T2b'de) | `docs/CONCEPTS.md`, `crates/qp-schema/src/data.rs`, PROTOCOL §18 |
| T3 | Alan düzeyinde veri olayları, `FeedStore`, pencere, `SourceProvider`, `MemoryProvider`, sentetik olay akışı, Matriks eşlemesi | alt ajan | denetim B (399 tohumlu düşmanca deneme) | `crates/qp-source/`, `MATRIKS_MAPPING.md`, `experiments/T3.md` |
| T4 | Oturumda bağ çözümleme, barsız anlık görüntü, yetki süzgeci, kanıt damgası/doğrulama, barındırıcı veri yolu | alt ajan | denetim T4 (koşullu kabul; düzeltmeler `rp2/t4fix`) | `crates/qp-session/src/plane.rs` …, dökümler 30–32, `experiments/T4.md`, `T4-fixes.md` |
| T5 | Artımlı canlı yol, yalnız etkilenenler, `view` konusu, ölçüm | alt ajan | denetim T5 (koşullu kabul; düzeltmeler `rp2/t5fix`) | `live.rs`, `values.rs`, döküm 33, `experiments/T5.md`, `T5-fixes.md` |
| T6 | Tutarlılık araştırması | alt ajan | denetim T2–T6 | `RESEARCH.md` |
| T7 | Düzenek (yaklaşımlardan önce, bağımsız) + A, B, C, D (ayrı dallar, ayrı alt ajanlar, birbirini görmeden) + aynı tohumla karşılaştırma | 5 alt ajan + orkestratör | son denetim | `experiments/harness/`, `HARNESS.md`, `T7a–d.md`, `T7-COMPARE.md` |
| T8 | Seçilen yaklaşım (B + §18.5): §19, iddia kanıtı, `claims` konusu, yansı (yerel + WASM), verify adımı 15 | alt ajan | son denetim | `claims.rs`, `replica.rs`, `tape.rs`, döküm 35, `experiments/T8.md` |
| T9 | K25 ve R2 kalanları | alt ajan | denetim B | dökümler 28–29, `experiments/T9.md` |
| T10 | Dökümler ve şartnamenin son hali | orkestratör | son denetim | PROTOCOL durum satırı, §15 döküm listesi |
| T11 | Son denetim | bağımsız alt ajan | — | `AUDIT.md` |

## Kabul ölçütleri

| # | Ölçüt | Sonuç | Kanıt |
|---|-------|-------|-------|
| 1 | Veri bağı şemada ve şartnamede; kavramlar önce sözlükte; sağlayıcıyla çözüm; "yetki yok"/"veri yok" tanımlı; döküm | ✅ | `fdd755c` (sözlük) → `0e209be` (şema + §18); `SeriesSource::Bind`, `DataBinding`, `Window`, `BindingState` (`ready`/`pending`/`no_entitlement`/`no_data`/`stale`); hata kodları `no_entitlement`, `no_data`; dökümler 30, 31. |
| 2 | 10.000 barlı bağlı anlık görüntü barsız; boyut ve hazır olma süresi | ✅ | 2.744 B ham / **985 B gzip** (RD1/T6d 749.600 / 210.638 B; T1 750.180 / 210.753 B). Hazır olma (ayrıştır + oturum + bağı çöz + hello): yerel **0,50 ms** (denetçi yüklü makinede 0,61–0,75), WASM release 1,09–1,32 ms (denetçi 0,92); T6d 2,57 / 3,23 ms. `crates/qp-session/examples/bound_snapshot.rs`. Veri cihaza kendi kaynağından gelir (K23); bu süreye girmez. |
| 3 | Alan düzeyinde olay; sentetik kaynak; Matriks akışıyla eşlenebilirlik (uygulanmadan) | ✅ | `DataEvent {feed, seq, change: bar \| patch{t, fields}}`; `SyntheticEvents` yalnız değişen alanları üretir (özellik testi: eski bar akışıyla bit-eşit). `crates/qp-source/MATRIKS_MAPPING.md`: `SymbolMessage` UPDATE → `patch`, REFRESH → yeniden çözüm/fark, `TradeMessage` (f32) v0'da bar kaynağı değil, `seq` yerel sayaç (Matriks QoS 0, belgelenmiş sıra numarası yok), 12 açık soru R6'ya. |
| 4 | Artımlı yol: tikte kopya yok; artımlı = baştan bayt düzeyinde; 5/50/500 hesap, yerel ve WASM; hedef ≤ 3× | ✅ (payda tanımıyla; aşağıda) | Denetçi `Document::clone` sayacıyla doğruladı: tik yollarında 0 kopya. Özellik testi `tests/live.rs` (12 senaryo, geri alma, düzeltme, yeniden canlanma; artımlı ve baştan yol mesaj/doküman/günlük/değer bit-eşit). Ölçüm tablosu aşağıda: **54/54 durumda hedefte** (en kötü 2,66). |
| 5 | Tik yalnız etkilenen görünüm ve abonelikleri uyandırır; test | ✅ | `view` konusu + `view_changed`; test `a_tick_wakes_only_affected_views_and_watchers` (200 olay, öbür bağlantıya 0 mesaj); döküm 33; T5 denetimi: 21.834 olayda yetkisiz seriye ait 0. |
| 6 | Dondurulmuş kanıt: başvuru + özet; doğrulama sorgusu; düzeltmede fark; döküm | ✅ | `Fact.evidence_ref` (bar 0 → olgu barı, FNV-1a 64), `verify_evidence`; döküm 32 (`verified` → bar 100 düzeltmesi → `changed`, değer 48,41675196259 → 48,41674791482106). Denetçi: olgu barından sonraki düzeltme → `verified`; bölünme → `changed`. |
| 7 | En az üç yaklaşım, ayrı dallar/alt ajanlar; senaryolar gecikme, sıra bozulması, düzeltme, yansı gecikmesi; seçilen uygulanmış ve şartnamede; öneri DECISIONS'ta | ⚠️ (denetim O1: B'nin "yansı itiraz eder, otorite geri çeker" yolu `qp/0`'da yok; düzenekteki `bs` gerçek yansıyı ama B'nin düzenek otoritesini koşturdu, gerçek otorite düzenekte ölçülmedi. Denetçinin AUD1 senaryosunda (bölünme + sıra bozulması) otorite kendi beslemesinden yanlış bir kesin iddia ediyor; yansı onu doğru olarak `tartışmalı` gösteriyor — yanlış gösterim 0 — ama otorite geri çekmediği için 7 kesin iddia kaçırılıyor (AUD2'de 49). Açık karar A20.) | Dört yaklaşım (A, B, C, D) `rp2/t7a…d-approach`; düzenek önce (`rp2/t7-harness`); 24 senaryo (S0–S19 + 4 ek); karşılaştırma `T7-COMPARE.md`; seçilen B + §18.5 → PROTOCOL §19; Ö25. |
| 8 | Otorite ↔ yansı (yerel ↔ WASM) özetleri aynı; geride kalan yansı yanlış göstermez | ✅ | `qp replica-tape` + Node: gecikme 0/1/7/25/60 olayda doküman, görünüm ve görüntü özetleri her adımda aynı (son adım doc `832528870cf6c127`, view `8a04abbca67e3bdd`); kâhin ihlali 0; verify adım 15 `replica`. |
| 9 | K25 (a) belirsiz sunum, (b) katman adları, (c) geleceğe bakış; değişen dökümler gerekçeli | ✅ | Dökümler 28, 29; değişen fikstürler aşağıda. |
| 10 | R2 kalanları | ✅ (MACD/RSI etiketi öneri) | Özet Bollinger sırası; seçicili komutta kopya kaldırıldı (41,8 → 31,3 µs; bağlı barlar kopyalanmıyor); sunucu kopyaları (709 → 458 ns); koşucu `send.msg`'i ham iletiyor (üç taşıma). MACD açıklaması ve RSI 30/70 etiketi var olan altın görselleri değiştireceği için **yapılmadı**, öneri. |
| 11 | Dökümler üç taşımada; yeni dökümler | ✅ | 36 döküm × inproc/serve/MCP (RP2'de 28–36: belirsiz sunum, geleceğe bakış, veri bağı, yetki hatası, kanıt doğrulama, etkilenenler, yetki/çapa/boşluk, tutarlılık, sorumlu kişiler). |
| 12 | REPORT, RESEARCH, AUDIT; STATUS, ROADMAP; önerilen kararlar ayrı bölümde | ✅ | Bu dosya; `RESEARCH.md`; `AUDIT.md`; `STATUS.md`, `ROADMAP.md`; `DECISIONS.md` "Önerilen (RP2)" Ö18–Ö28, açık A15–A19. |

## Ölçümler

**Tik maliyeti (ölçüt 4).** 10.000 bar, 600 tik (saniyede 1 yeni bar + 59 canlı düzeltme);
µs/tik, uzun döngü medyanı, iki koşunun ortalaması; Apple M4 Max, rustc 1.92, Node 24.3.
Önce: T1 (`a310186` öncesi kod). Payda (orkestratör kararı, Ö23): çekirdeğin **bağlı bütün
hesapları tam sütunla ilerletmesi + aynı mesajı çözmesi**. Oran = (oturum − çekirdek) /
çekirdek; hedef ≤ 3. Kaynak `experiments/T5-fixes.md` §6 (`24be50a`), ham `raw/t5-fixes/`.

| Yol, N | Önce (T1, `t6d` yerel) | Sonra yerel `t6d` / `all` | Sonra WASM opt-s `all` | En kötü oran (3 hedef × 3 düzen) |
|---|---:|---:|---:|---:|
| belge içi, 5 | 85,1 | 2,04 / 2,16 | 3,08 | 2,34 |
| belge içi, 50 | 90,7 | 2,71 / 3,58 | 5,63 | 2,33 |
| belge içi, 500 | 157,6 | 10,18 / 20,71 | 35,07 | 2,57 |
| bağlı, 5 | 80,0 | 0,76 / 0,92 | 1,60 | 1,40 |
| bağlı, 50 | — | 1,31 / 2,06 | 3,96 | 1,87 |
| bağlı, 500 | 80,3 | 8,71 / 19,27 | 34,72 | **2,66** |

- 54/54 durumda hedefte (medyanla ve en iyi/en iyi ile).
- **Dürüstlük notu:** T1'in yalın çekirdeği paydaysa (çıktı sütunları yalnız akan barlar)
  N=500 `all` düzeni hedefi aşar (yerel 5,06). Çözme paydadan çıkarılırsa N=5'te oran 12–43
  (çekirdek tikte yalnız 0,02–0,08 µs iş yapıyor). İkisi bilgi olarak raporlandı.
- 100 niyet + 100 canlı kapsül ekli düzen (`ic`): N=500'de hedefte (en kötü 1,45); N=5/50'de
  2,2–7,2 (200 koşul değerlendirmesi paydada olmayan gerçek iş).
- Bir gerileme yakalandı ve düzeltildi: T4 düzeltmesinin abone başına yetki denetimi her tikte
  doğrusal yürüyüş yapıyordu (bağlı N=500 `all` 317 µs); `LiveIndex` ile 19,3 µs.

| Diğer ölçüm | Değer |
|---|---|
| `QP_VERIFY_BROWSER=1 scripts/verify.sh` | 15/15 PASS (RP2'de adım 15 `replica`) |
| Uygunluk dökümleri | 36 × inproc/serve/MCP |
| Hesap özeti | `7b1e99d40889f83c` (aarch64, x86_64, wasm32) — değişmedi |
| Altın görseller | değişmedi (8 görünüm × 2 bayt-eşit; tarayıcı 0 px) |
| Bağlı anlık görüntü (10k bar) | 2.744 B / 985 B gzip |
| Veri olayı uygulama (`FeedStore`) | 23,6 ns/olay; özet: ekleme O(1), bar i'de düzeltme O(n−i) |
| WASM (gzip -9) | varsayılan 598.332 B (R2 595.475); `--features replica` 1.088.011 B |
| Tutarlılık (B, 24 senaryo × 100 tohum) | yanlış/erken/çelişkili 0; kaçırma yalnız S4'te 2 (30 s kanal kopukluğu) |
| Tutarlılık (gerçek yansı `bs`) | yanlış/erken/çelişkili 0; S16/S18'de çift ve geçersizlik dedektörleri `qp/0` koşulu olmadığı için iletilmiyor (model dışı kaçırma); denetçinin karma senaryoları AUD1–AUD3'te `b`, `bs`, `a` yanlış/erken 0, `bs` 7/49/7 kaçırma (geri çekme yolu yok, A20) |

## Tutarlılık araştırması (özet; ayrıntı `RESEARCH.md`, `experiments/T7-COMPARE.md`)

Soru: veri protokolün dışından akınca otorite "RSI 70'i kesti" dediğinde yansı aynı veriye
baktığını nasıl bilir? Literatür (filigran, olay zamanı, oturum garantileri, özet zincirleri,
piyasa verisinde sıra numarası + boşluk doldurma) ve dört yaklaşım: A konum damgası, B kanıt
değerleri, C yansı kendi verisiyle, D kanıt zarfı + görüntü durumları (özgün, K18).
Yaklaşımlardan önce ve bağımsız yazılmış belirleyici düzenekle (tohumlu sanal zaman; iki
besleme, kanal, hakem; 24 senaryo) ölçüldü. Dört savunucu ayrı ayrı aynı üç ilkeye vardı:
(1) yansı iddiayı **kendi verisinde yeniden sınar** — erken ve çelişkili gösterimi yapı
gereği sıfırlar; (2) kesinlik sağlayıcının kesin barına dayanır; (3) canlı barda geçici
iddia sıfır hatayla gösterilemez (yansı ≥ ~80 ms geride). Seçim B: sert hata 0, en az kaçırma,
en düşük gecikme ve bayt grubunda, tek otorite modeline uygun; dondurulmuş kanıt için §18.5
özeti korunur.

## Değişen var olan fikstürler (gerekçeli)

| Fikstür | Değişim | Gerekçe |
|---|---|---|
| `fixtures/protocol/conformance/11-capability-depths.json` | yalnız bir adımın `note` metni ("en önemli canlı bulgu" seçimini anlatıyordu) | K25 (a); beklentiler aynı |
| `fixtures/views/d2-chart.explain.txt` | katman kimlikleri `l0/l1/l2` → `candles/line/level` | K25 (b); `d2-chart.png` bayt-eşit |

Başka var olan döküm, açıklama ya da altın görsel değişmedi.

## Tasarım kararları (önerilen; ana akış onayına açık)

`DECISIONS.md` "Önerilen (RP2)" Ö18–Ö28: veri bağı; kalıcı çapa; alan düzeyinde veri olayı
ve `seq`; veri yetkisi modeli (koşul yazmak veri okumaktır, sorumlu kişiler); dondurulmuş
kanıt; artımlı canlı yol ve ölçüt paydası; `view` konusu; tutarlılık kuralı (B + §18.5);
yansı ayrı WASM özelliği; K25 uygulaması; ödünç veren sağlayıcı. Açık: A15–A19.

## Sapmalar

1. **Şema denetimden sonra değişti:** T2 denetimi (Y1–Y3) çapa, `seq` hizası ve yetki modelini
   T4'ten önce düzelttirdi (`rp2/t2b-fixes`); T3 o sırada yazılmıştı ve uyarlandı.
2. **Alan dışı küçük yamalar:** düzenek `sim.rs`'e bir satır (`anchor_t: None`, T8); T3'ün
   `Cargo.lock`'u; orkestratörün `qp-source`'ta çapa sınırı düzeltmeleri (`9d640b4`,
   `e15d8da`, denetim bulgularına göre).
3. **Depo bağ başına** (görev "besleme başına" diyordu; T4 D-T4-1, A18).
4. **Yeni dökümler 34 ve 36** görev listesinde yoktu (denetim düzeltmelerinin kanıtı).
5. **Bir gerileme bütünleştirmede ortaya çıktı** (T4 düzeltmesi × T5 yolu); T8 ajanı fark etti,
   orkestratör ölçümle doğruladı, `rp2/t5fix` giderdi.

## Denetim

Bağımsız denetçi alt ajan `5319cf0`'ı temiz bir kopyada denetledi (`AUDIT.md`). **Sonuç:
koşullu kabul.** VERIFY 16/16 PASS (1: `QP_VERIFY_BROWSER=1 scripts/verify.sh` 15/15; 6: 54
hücre T5-fixes'e göre ±%5,8, en kötü oran 2,54; 12: `qp replica-tape` gecikme 0/3/25/100,
Node'da WASM = yerel her adımda). Denetçinin bağımsız yeniden ürettikleri: tam verify, tik
maliyeti, anlık görüntü (2.744 B, 0,48 ms), yansı kasetleri, a/b/c/d düzenek tabloları
(`raw/t7` ile bayt-eşit), `b`/`bs` sonuçları; kendi senaryoları AUD1–AUD3 (gecikme + sıra
bozulması + yineleme + geride kalan yansı + 8 düzeltme + bölünme; çıkış kesintisi; f32);
yetki sızdırma denemesi (karma panel ölçeği, `hit`, renk/`y2` işlenenleri, bağlı serideki
hesap, koşullar, olgular, `view`/`claims` abonelikleri, `log`, `document`, `list`, `get`,
`present`, `binding`): bar değeri sızan yol yok.

| Koşul | Giderme |
|---|---|
| K1: `REPORT.md` yok; `STATUS.md`, `ROADMAP.md` RP2'yi koşulmamış gösteriyor | Bu rapor (denetlenen commit'ten sonra yazıldı); `STATUS.md`, `ROADMAP.md` güncellendi. |
| K2: O1 (kendini onarma yolu `qp/0`'da yok, gerçek otorite düzenekte ölçülmedi) raporda ve DECISIONS'ta | Ölçüt 7 ⚠️ ve yukarıdaki tablo; Ö25'e not; açık karar A20. |

Düşük önemdekiler (öneri/açık soru olarak): D1 `Fact.evidence_ref.digest` yetkisize gidiyor
(`binding` onu gizlerken) — A15/A19'a eklendi; D2 hazır senaryolar hataları birleştirmiyor
(denetçinin karma senaryolarında C 278–313, D 5 yanlış kesin; B ve A 0) — karma senaryolar
öneri; `bs`'nin yansı işi `b`'den çok yüksek (S12'de ~2.460 / ~7 iş birimi/olay); D3 kapsül
sonucuna ek olarak `maturity` ve `t` de yetkisize gidiyor — A15'e eklendi; D4 özellik testi
`claim` olayı üretilmesini şart koşmuyor, az tohum.

## Açık sorular (ana akış için)

1. Ö18–Ö28 kabul mü? Özellikle Ö25 (tutarlılık kuralı B + §18.5) ve Ö21 (veri yetkisi:
   koşul yazmak veri okumaktır, sorumlu kişiler).
2. Kanıt değerleri ve kapsül durumu veri lisansı kapsamında mı (A15)? Bugün iddia kanıtının
   değerleri yetkisize gitmiyor; `Fact.evidence` ve kapsül durumu gidiyor.
3. Canlı barda geçici iddia: kapalı (varsayılan) mı, açık mı, toleranslı mı (A16)? Kapsül
   geçersizleşmesi kesinleşmeyi beklesin mi?
4. Ölçüt 4'ün paydası Ö23'teki gibi kabul mü (yalın paydayla N=500 `all` 5,06)?
5. Yansılı WASM 1,09 MB gzip: ayrı paket mi (Ö26, A13 ile)?
6. `update_finding` aynı koşulu yeniden yazarsa "son değiştiren" değişsin mi?
7. Otoritenin kendi beslemesinden yanlış kesin iddiasını kim onarır (A20): `qp/1`'de yansıdan
   `dispute`, otoritede kesin bar doğrulaması (B'nin düzenekteki hijyeni) ya da ikisi?

## Öneriler (kapsam dışı, yapılmadı)

- Besleme başına tek paylaşılan depo; bağlı barların üç kopyadan bire inmesi (A18).
- `qp-compute`'ta durum anlık görüntüsü: geçmiş düzeltmede yalnız düzeltilen bardan yeniden
  hesap (bugün N=500'de ~40 ms).
- Sağlayıcıdan "bar kesinleşti" işareti ve toplu düzeltme/bölünme olayı (T3, T4 denetimi;
  bölünme bugün bar başına bir olay).
- Yansıdan otoriteye `dispute`/`check` bildirimi `qp/0`'a (Ö-T8-5).
- Çok bağlı ve geçersizlik dedektörlerinin iddia kanıtı (R3 dedektörleriyle).
- MACD paneli açıklaması ve RSI 30/70 eksen etiketi yeni bir görünüm + yeni altın görselle.
- Düzenekte REST çağrı sayısının M9'a girmesi, kayıplı kanal senaryosu, toplu `apply_settled`.
- `scripts/qapsule-gh.env` izlenmiyor (R1'den beri açık öneri).
