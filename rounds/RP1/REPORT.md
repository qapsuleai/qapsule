# RP1 — Rapor

- **Durum:** Tamamlandı. Yedi kabul ölçütünün yedisi karşılandı (kanıt aşağıda). Bağımsız
  denetim: **kabul**, yüksek önemde bulgu yok (`AUDIT.md`).
- **Tarih:** 2026-11-14
- **Kod:** `rp1/integrate` dalı, `main`'e `--no-ff` ile birleştirildi. Ölçülen kod commit'i
  `0cd37ad` (sonraki commit'ler yalnızca belge).
- **Ana belge:** `rounds/RP1/RESEARCH.md` (literatür, 21 yaklaşım, önce/sonra tabloları, K17,
  değişmezleri bozanlar, bir sonraki sınır). Ölçüm tabanı `BASELINE.md`, deney notları
  `experiments/E1…E8`, ham çıktılar `raw/`, denetim `AUDIT.md`.

## Ne yapıldı

| # | Görev | Dal | Yürüten | Çıktı |
|---|-------|-----|---------|-------|
| T1 | Ölçüm tabanı, dört yük profili, başlangıç ölçümleri | `rp1/t1-baseline` | orkestratör | `profiles.rs` (A–D, profil özetleri), `benches/driver`, `benches/profiles.rs`, `benches/wasi-run.mjs` (WASM'ı `qp-wasm`'a dokunmadan ölçmek için wasip1 + Node), CI'da CPU modeli; `BASELINE.md` |
| T2 | Literatür ve var olan motorlar | — | orkestratör | `RESEARCH.md` §1 |
| E1 | Izgarada ortak alt hesap paylaşımı | `rp1/e1-share` | **alt ajan** | `sweep/share.rs` |
| E2 | SoA şerit çekirdekleri, seri şeritleri, hedefe göre şerit; AVX2 ve simd128 denemeleri | `rp1/e2-simd` | orkestratör | `lanes.rs`, `sweep_series_each` |
| E3 | İş parçacıkları | `rp1/e3-threads` | orkestratör | `par::sweep_reduce_par` |
| E4 | Hesap grafiği (zincir önekleri), içerik adresli önbellek | `rp1/e4-graph` | orkestratör | `graph::sweep_chains`, `graph::ResultCache` |
| E5 | Canlı güncelleme; commit-only attach | `rp1/e5-live` | orkestratör | `Study::extend` |
| E6 | Bellek / çıktı trafiği | `rp1/e6-output` (E2 üstünde) | orkestratör | sıfır doldurma kaldırıldı |
| E7 | Değişmezleri bozanlar: ölçüm ve bedel | `rp1/e7-beyond` | orkestratör | `examples/rp1_beyond.rs` |
| E8 | Kanıtlı geç başlangıç (yeni yöntem) | `rp1/e8-sandwich` | orkestratör | `late::{ema,rsi}_tail` |
| Tn+1 | Kazananların birleştirilmesi, birlikte ölçüm | `rp1/integrate` | orkestratör | E1 yolları + E2 çekirdekleri (`soa_plain` köprüsü); WASM'da bir kod yerleşimi gerilemesi bulunup giderildi |
| Tn+2 | Denetim | — | **bağımsız denetçi alt ajan** | `AUDIT.md` |

## Kabul ölçütleri

| # | Ölçüt | Sonuç | Kanıt |
|---|-------|-------|-------|
| 1 | Her yaklaşım hipotez + ölçüm + kararla; işe yaramayanlar dahil; ≥ 6 yön | ✅ | `RESEARCH.md` §3: 8 yön, 21 yaklaşım (9 al, 10 alma/sonra, 1 gözlem, 1 analiz); her biri `experiments/` notunda; hipotez bölümü denemeden önce yazıldı (E2–E8'de not dosyası kodla aynı commit'te kaydedildi, yani commit sırası bunu ayrıca kanıtlamıyor; E1 notu da aynı biçimde). |
| 2 | K17 dizüstü hedefleri beş indikatörde GitHub Linux koşucusunda | ✅ | `RESEARCH.md` §5: final commit'in 5 CI çalışmasında en kötü: SMA 15,0 · EMA 5,3 · RSI 42,2 · MACD 13,2 · Bollinger 14,3 ms (< 50); WASM en kötü RSI 57,3 ms (< 150). Üç farklı koşucu CPU'sunda (AMD EPYC 9V74, AMD EPYC 7763, Intel Xeon 8573C) tutuyor. Taban: RSI 62,9, MACD 69,1, Bollinger 72,0; WASM MACD 159,9. |
| 3 | Dört profil için önce/sonra: referans makine, CI, WASM | ✅ | `RESEARCH.md` §4 (A, B, C, D × M4 Max, WASM, CI 9V74, CI 7763; A için CI WASM da). |
| 4 | Bit-eşitlik testleri değişmeden geçiyor | ✅ | R1 testleri (`hand`, `legacy`, `modes`, `chain`, `sweep`) dokunulmadan geçiyor; `cross_target_digest` = `7b1e99d40889f83c` (aarch64, x86_64 Rosetta, Linux x86_64 ×5, wasm32); her profil özeti tabanla aynı; `verify.sh` 11/11. Yeni testler: `lanes`, `share`, `graph`, `late`, `par`, `extend`, `profiles`. |
| 5 | Değişmezleri bozan umut vadeden yaklaşımlar ayrı bölümde, bedeliyle | ✅ | `RESEARCH.md` §6, ayrıntı `experiments/E7-beyond.md`. |
| 6 | Bir sonraki sınır | ✅ | `RESEARCH.md` §7. |
| 7 | `REPORT.md`, `AUDIT.md`; `STATUS.md`, `ROADMAP.md` güncel | ✅ | Bu dosya, `AUDIT.md`, `STATUS.md`, `ROADMAP.md`. |

## Ölçümler (özet)

| Ölçüm | Önce | Sonra |
|-------|------|-------|
| A, M4 Max (SMA/EMA/RSI/MACD/Bollinger, ms) | 15,1 / 9,6 / 24,7 / 41,4 / 46,3 | 4,3 / 3,8 / 9,0 / 7,5 / 8,6 |
| A, CI AMD EPYC 9V74 | 17,4 / 21,6 / 62,9 / 69,1 / 72,0 | 12,6 / 4,7 / 38,6 / 11,0 / 11,8 |
| A, WASM CI (Node 22) | 26,4 / 37,6 / 118,5 / 159,9 / 120,7 | 18,8 / 22,4 / 49,2 / 25,2 / 21,6 |
| B, M4 Max (2.500 seri) | 14,4 / 25,9 / 62,0 / 44,2 / 42,8 | 6,3 / 5,4 / 18,5 / 12,4 / 32,4 |
| C, M4 Max (ema>rsi / rsi>bollinger / sma>ema>rsi) | 100,0 / 175,8 / 161,7 | 11,6 / 14,4 / 16,0 |
| D attach (500 hesap + 10k bar), M4 / WASM / CI | 26,3 / 51,1 / 51,8 ms | 13,8 / 18,1 / 17,3 ms (`extend`) |
| D tik (500 hesap) | 1,8 µs | aynı (sınır değil) |
| Kanıtlı geç başlangıç, 1M bar, son 500 bar | tam tarama EMA 118 / RSI 233 ms | 2,7 / 10,8 ms (100/100 kanıtlı) |
| Paralel tarama, 4 iş parçacığı (M4) | — | 3,5–3,9× |
| Önbellek isabeti, 1.000 varyasyon tekrar | 23,3 ms | 0,44 ms |
| WASM modülü (`verify.sh`, gzip -9) | 445.302 B (R1) | 475.519 B (+30 KB) |
| `verify.sh` | 11/11 | 11/11 |

## Sapmalar

1. **Alt ajanlar.** Yedi deney alt ajanı aynı anda başlatıldı; auto mode hepsini denetleyemedi
   çünkü context-mode hook'u Agent çağrısının girdisini değiştiriyor (R1'deki sorunun
   aynısı). Hook'un ürettiği girdiyle yeniden gönderildiğinde E1 başladı; E2–E7 ikinci kez de
   reddedildi ve kural gereği tekrar denenmedi. E2–E8'i orkestratör kendisi, her birini
   tabandan ayrı dalda, diğerlerinin sonucunu kullanmadan yürüttü (E6 bilinçli olarak E2
   üstünde, çünkü ölçtüğü şey E2'nin çıktı yolu). Bu, "deneyler bağımsız alt ajanlarla"
   kuralının tam karşılanmadığı anlamına gelir; E1 bağımsız bir alt ajanın işidir ve denetim
   bağımsız bir alt ajanla yapıldı (o çağrı da aynı yolla, ikinci gönderimde başladı).
2. **CI koşucu donanımı değişken.** Aynı iş tanımı dört çalışmada üç farklı CPU'ya düştü (AMD
   EPYC 9V74, AMD EPYC 7763, Intel Xeon Platinum 8573C). Önce/sonra karşılaştırması aynı CPU
   (9V74) üzerinden yapıldı; diğerleri ayrıca verildi. CI sayıları bilgi amaçlı (K17).
3. **Yazma alanı.** Yalnızca `crates/qp-compute/`, `rounds/RP1/`, ve kapanışta `STATUS.md`,
   `ROADMAP.md`. `qp-compute/Cargo.toml`'da `proptest` geliştirme bağımlılığının `fork`
   özelliği kapatıldı (wasip1 derlemesi için; testler kullanmıyordu). `Cargo.lock`
   değişmedi (E2.2'de eklenen `pulp` geri alındı). `qp-wasm`, CI ve betikler değişmedi.
4. **Kamu API eklemeleri** (hepsi ek; mevcut imzalar aynı): `sweep::sweep_series_each`,
   `SERIES_WINDOW`; `graph::{Stage, sweep_chains, ResultCache, content_hash}`;
   `par::sweep_reduce_par`; `late::{ema_tail, rsi_tail, *_with, Bounds, Certificate}`;
   `Study::extend` (varsayılan gövdeli). `profiles` modülü `#[doc(hidden)]`.

## Önerilen karar satırları (`DECISIONS.md` için; ana akışa bırakıldı)

`CLAUDE.md` mimari seçimlerin `DECISIONS.md`'ye yazılmasını istiyor; RP1'in yazma alanı ise
yalnızca `crates/qp-compute/` ve `rounds/RP1/` (PROMPT, VERIFY 6) ve `DECISIONS.md`'de ana
akışın commit'lenmemiş düzenlemeleri var. Bu yüzden satırlar burada öneri olarak duruyor
(denetim M1):

| Karar | Gerekçe / kanıt |
|-------|-----------------|
| Tarama çekirdekleri yapı-dizisi (SoA) şeritlerle; ısınma skaler, kararlı döngü dallanmasız, aynı işlemler aynı sırada; şerit sayısı hedef mimariye göre (`cfg(target_arch)`), ölçümle. | E2; bit-eşitlik testleri ve 5 CI çalışması. |
| Izgarada ardışık eş anahtarlı varyasyonlar ortak alt hesabı paylaşır (MACD çizgisi, Bollinger orta/std, RSI kazanç/kayıp); ziyaret sırası korunur. | E1. |
| Zincir taramaları önek paylaşan hesap grafiğiyle (`graph::sweep_chains`). | E4. |
| `Study::extend`: geçmişe bağlama yalnızca `commit` ile (varsayılan gövde `push`). | E5. |
| Çok iş parçacığı yalnızca ek API (`par::sweep_reduce_par`); K17 hedefleri tek çekirdek kalır. | E3. |
| Kanıtlı geç başlangıç (`late`) deneysel API; tam uzunluklu çıktı (K13) değişmez. | E8. |
| İçerik adresli önbellek deneysel, bellek politikası açık. | E4.2. |
| Değişmezleri bozan yaklaşımlar (FMA, karşılıklı çarpma, önek toplamı, f32, GPU) alınmadı; tamsayı kuruş toplamı karar bekliyor. | E7. |

## Açık sorular (ana akış için)

1. **`unsafe_code = "forbid"`** x86'da geniş SIMD'yi (AVX2/AVX-512) engelliyor; RSI'de
   kalan pay bunun arkasında. Sınırlı bir istisna mı, `pulp` gibi bir bağımlılıkla yeniden
   yazım mı, yoksa hiçbiri mi?
2. **Tamsayı kuruş toplamı** (E7.6): SMA/Bollinger'i daha doğru ve 1,4–2,4× hızlı yapan, ama
   R1 tanımını değiştiren tek umut verici tanım değişikliği. Ayrıca SMA/Bollinger için kanıtlı
   geç başlangıcın önünü açar. Enstrümanın tick boyu şemaya girmeli.
3. **Sorgu biçimli hesap API'si** (istenen çıktı, bar aralığı, indirgeme): bir sonraki büyük
   kazanç çıktı yazımında; K13 ("tam uzunlukta çıktı") ile ilişkisi netleşmeli — K13 belgenin
   saklanan çıktısı için mi, her hesap çağrısı için mi?
4. **Önbellek bellek politikası** (E4.2): bütçe, tahliye, canlı seride geçersizleşme.
5. **CI ölçüm adımı** profil tablosunu ve şerit tablosunu zaten basıyor (`sweep_bench`); CI
   donanımı değiştiği için gerileme izlemede CPU modeline göre ayrım gerekir.

## Bilinen sınırlar (denetim L5–L9)

- `late::*_tail_with` başka bir seriden kurulmuş `Bounds` kabul eder; o zaman doğruluk
  güvencesi geçmez (belgelendi; API deneysel). Genişletme payı periyot ≲ 1e8 (tohum) için
  hesaplandı; izin verilen periyot üst sınırı 1e9, pratikte erişilmez (belgelendi).
- `late`, `par`, `graph`, `extend` wasm32'de test edilmiyor ve hedefler arası özete girmiyor
  (dolaylı: `extend` D özetini, `graph` C özetini wasm'da da veriyor).
- Hipotezlerin denemeden önce yazıldığı git geçmişinden kanıtlanamıyor (notlar sonuçlarla
  aynı commit'te).

## Öneriler (kapsam dışı, yapılmadı)

- `docs/CONCEPTS.md`: "Hesap grafiği", "Seri şeridi", "Kanıtlı geç başlangıç", "Hesap kimliği"
  (önbellek anahtarı) kavramları.
- `qp-schema`: zincir tanımı (`Stage`) ve "hesap sorgusu" (istenen çıktı/aralık/indirgeme).
- `qp-wasm`: `sweep_series_each`, `sweep_chains`, `late::*_tail` bağlamaları; tarayıcıda iş
  parçacığı (SharedArrayBuffer, COOP/COEP) bir kabuk kararı.
- CI: `scripts/wasm-sweep-bench.sh` yalnızca A'yı ölçüyor; `wasi-run.mjs` ile B–D de
  eklenebilir (CI dosyası bu round'un kapsamında değildi).
- E8'in genişletilmesi: MACD (üç EMA; sinyal girdisi `fL−sU … fU−sL` ile sınırlanır),
  `Streaming` durumunun geç başlangıçla kurulması (attach), sandviç yörüngelerinin şeritli
  koşması.
- `sweep_series_each` pencere sonuçlarını `clone_from` ile tutuyor; tampon takasıyla bir yazma
  geçişi daha kalkar.
- Kod yerleşimine duyarlılık (E1 ve birleşimdeki WASM Bollinger bulgusu): CI şerit ve profil
  tablosu gerilemeleri yakalar; ayrıca kritik çekirdekler için `#[inline(never)]` sınırları
  bilinçli konmalı.

## Denetim

Bağımsız denetçi alt ajan, `4628343`'ü temiz klonda denetledi (`AUDIT.md`).

- VERIFY 1–6 geçti: `verify.sh` 11/11; özet üç hedefte `7b1e99d40889f83c`, `fixtures/`
  değişmedi; dört profil yeniden ölçüldü, hepsi `RESEARCH.md` "sonra" sütunlarının %20'si
  içinde (+%1…+%14, yük ≈ 8); CI 43625622117'nin 5 denemesi ve taban 43618730149 günlüklerle
  doğrulandı; ≥ 6 yaklaşım; değişiklik yalnızca izinli klasörlerde, R1 testleri aynı.
- Kurallar: `unsafe`, `mul_add`, saat, rastgelelik yok; `NORTH_STAR.md` aynı; gizli bilgi yok;
  `Cargo.lock` aynı.
- Bit-eşitlik: testler aarch64 ve x86_64 (Rosetta, release) geçiyor; `lanes.rs`, `share.rs`,
  `late.rs` okundu; −0, alt-normal, ±1e300, f64::MAX yakını, NaN/inf, tüm şerit sayıları,
  sırasız ızgaralar, seri yolu ve 840 geç başlangıç durumu içeren geçici bir saldırgan test
  0 fark buldu. Sandviç kanıtı sağlam bulundu.
- Bulgular: yüksek yok; orta 1 (M1 `DECISIONS.md`) → yukarıda önerilen satırlar; düşük 9 →
  L1–L3 `RESEARCH.md`'de düzeltildi, L4–L9 "Bilinen sınırlar" ve "Sapmalar"da.
