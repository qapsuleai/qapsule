# R3 — Rapor: Kapsül

- **Durum:** Tamamlandı (denetim sonucu aşağıda, "Denetim" bölümü). Kabul ölçütleri: tablo.
- **Tarih:** 2026-11-15 / 16
- **Alt ajanlar:** Başlatılabildi (ilk iş deneme alt ajanı koşuldu ve cevap verdi). Bütün alt
  görevler, 16 alternatif, iki düzenek, doğrulama, ölçümler, insan testi paketi, ajan
  yazabilirlik ve son denetim ayrı alt ajanlarla yapıldı. Orkestratör T1'i (sözlük, şema,
  sözleşme iskeletleri, CAPSULE §9, 20 görev), düzenek şartnamesini (SPEC), okuma testlerinin
  koşulmasını, seçimleri ve bütünleştirmeyi yaptı; ayrıca güvenlik incelemesinin iki
  bulgusunu (mühür anahtarları) kendisi düzeltti.
- **Dallar:** bütünleştirme `r3/int`; alt görev dalları `r3/t2-data`, `r3/t3-facts`,
  `r3/t45-capsule`, `r3/verify-e1`, `r3/h-m12`, `r3/h-m3456`, `r3/m1a-siluet`, `r3/m1b-rozet`,
  `r3/m1c-iz`, `r3/m2a-template`, `r3/m2b-llmslot`, `r3/m3a-rarity`, `r3/m3b-ledger`,
  `r3/m3c-confirm`, `r3/m4a-kinddir`, `r3/m4b-arc`, `r3/m5a-barrier`, `r3/m5b-semantic`,
  `r3/m6a-coarse`, `r3/m6b-novel`, `r3/m7-dispute`, değerlendirme `r3/eval-m1`, `r3/eval-m2`,
  `r3/eval-sim`, bütünleştirme `r3/s-engine`, `r3/s-grammar`, `r3/s-session`, `r3/s-perf`,
  `r3/s-views-fix`, `r3/ht`, `r3/e-ergo`, `r3/fix-e-bugs`, `r3/fix-perf`, denetim `r3/audit`.
  Hepsi `--no-ff` ile `r3/int`'e.
- **Gerçek veri:** Var. `fixtures/local/symbols.txt`'teki 6 sembolden 5'i çekildi (MIGRS her
  istekte 503 `EAGAIN`; MGROS aynı istekle çalışıyor, büyük olasılıkla yazım hatası; dosya
  Arkin'in, değiştirilmedi). Günlük düzeltilmiş + düzeltilmemiş 2018-01-02 → 2026-11-13 (sembol
  başına 2.219 bar) ve 60 dakikalık son iki yıl (sembol başına 3.983 bar). Raporda yalnız toplu
  istatistik; veri, türevleri ve görselleri depoda yok. Gerçek **kurumsal akış** (lisanslı)
  yerelde yok: M6 ve akış ailesi yalnız sentetik.

## Ne yapıldı

| # | Görev | Yürüten | Doğrulayan | Çıktı |
|---|-------|---------|------------|-------|
| — | Bekleyen doküman değişiklikleri tek commit | orkestratör | — | `ed57a11` |
| — | Deneme alt ajanı | alt ajan | — | başlatıldı, cevap verdi |
| T1 | Sözlük (önce), şema `qp-schema::capsule`, sözleşme iskeletleri (`qp-facts`, `qp-capsule`, `qp-grammar::glyph`), CAPSULE §9, 20 ajan görevi | orkestratör | verify-e1 | `0ad3a4e` |
| T2 | Rejimli sentetik pazar, sentetik lisanslı akış, `scripts/fetch-bars.sh`, yerel okuyucu, DATA_ACCESS doğrulamaları | alt ajan | verify-e1 | `experiments/T2.md` |
| T3 | Yedi dedektör (altı tür), `Highest`/`Lowest`/`Atr`, ek özet | alt ajan | verify-e1 (bağımsız yeniden hesap) | `experiments/T3.md` |
| T4+T5 | Motor: yaşam döngüsü, aday, karne, ileriye bakma testi, hassasiyet süzgeci | alt ajan | verify-e1 | `experiments/T45.md` |
| V | Evre 1 bağımsız doğrulama | denetçi alt ajan | — | `experiments/VERIFY-E1.md` (9/9) |
| H | Düzenek şartnamesi (alternatiflerden önce) + M1/M2 düzeneği + M3–M6 düzeneği; M7 = RP2 düzeneği | orkestratör + 2 alt ajan (+ M7 ajanı kopyaladı) | yeniden üretim | `harness/SPEC.md` (`a35d726`), `harness/`, `harness-sim/` |
| M1–M7 | 16 alternatif, ayrı dallar, ayrı alt ajanlar, birbirini görmeden | 15 alt ajan | orkestratör yeniden ölçtü/puanladı | `experiments/M*/` |
| S | Ürün motoru, gramer işareti ve görünümler, otorite + protokol §20, hassasiyet benzetimi, performans | 4 alt ajan + 3 düzeltme ajanı + orkestratör | denetim | `experiments/S/` |
| HT | İnsan testi paketi | alt ajan | denetim | `human-test/` |
| E | Ajan yazabilirlik (C01–C20, iki koşu) | alt ajan | denetim | `ergonomics/RESULTS.md` |
| A | Son denetim | bağımsız alt ajan | — | `AUDIT.md` |

## Kabul ölçütleri

| # | Ölçüt | Sonuç | Kanıt |
|---|-------|-------|-------|
| 1 | Altı türün dedektörleri; tipli olgu, kanıt değerleri, dondurulmuş kanıt; toplu = akışkan; kavramlar önce | ✅ | 7 dedektör (`T3.md`); `Fact.meaning`; otorite `evidence_ref` damgalıyor (S-C); bit-eşitlik testleri + denetçinin 16.723 tespitlik ek denemesi (0 fark); sözlük `0ad3a4e` kodlardan önce. Denetçinin bağımsız Python yeniden hesabı: 7 dedektörde karar farkı 0 (`VERIFY-E1.md`). |
| 2 | Aday yalnız yansıda, otorite günlüğüne/karneye girmiyor; kapanınca kesinleşir/kaybolur | ✅ | `Engine::candidates` saf (`&self`); `Replica::candidates`; test `a_candidate_that_disappears_leaves_no_trace`, döküm 42. |
| 3 | Kanıt aileleri; aynı aile tek teyit; teyit sayısı bulguda ve işarette | ✅ | `EvidenceFamily`, CAPSULE §9.3 (`same_family` → güçlendi, `new_family` → teyit); `Story.families`; işarette teyit blokları (`g1-glyphs`). |
| 4 | Tek hikâye evrilir; geçişler dökümde; eskime bar sayısıyla; günlükten yeniden oynatma aynı | ✅ | Aşamalar `Stage`; dökümler 38, 40, 43; `horizon_bars`; test `replaying_the_log_reproduces_the_capsule_history`. |
| 5 | Her kapsül sonuçla kapanır; tür+dedektör karnesi kapsülde; ileriye bakma testi; geri çekildi karneye yazılmaz | ✅ | Karne anahtarı `kind/detector@v`; `ledger_at_birth`; kartta "Karne 3/7"; ileriye bakma: sahte dedektör, gerçek dedektör, seçilen yapılandırma (S-A) ve M5 düzeneği (0 fark); geri çekme testi. |
| 6 | Üç zaman türü; TR ve EN; K26; lisanslıda rakam yok | ✅ | Şablon dilbilgisi (M2 seçimi); K26 uçtan uca: "THYAO'da RSI 19 bar önce 70'in üstüne çıktı; şu an 52,6, artık 70'in üstünde değil" / "…19 bars ago … no longer above 70"; döküm 40; 35.640 birleşimlik rakamsızlık taraması. |
| 7 | Alfabe M1 ile seçildi; gramere yeni işaret türü; altı tür × yön × aşama altın görsel; kapsül görünümü | ✅ (ön eleme; son söz insan testinde) | `M1/RESULTS.md`; `glyph` işareti (GRAMMAR §18); `g1-glyphs-light/dark.png` (6 tür × 3 yön × 5 aşama); `c2-capsule.png` (işaret + cümle + küçük grafik + karne). |
| 8 | Anlam zoom'u üç derinlikte; `chart`'ta geometri olgu kanıtından | ✅ | `c1-sentence` (döküm), `c2-capsule`, `c3-chart` (kırılan seviye), `c4-band` (bant), `c5-points` (tepe/dip noktaları); geometri yalnız kökenle bağlı, değer okuyucunun sağlayıcısından. |
| 9 | Yetkili değerleri görür; yetkisize yorum + karne + "şu yetkiyi gerektirir"; sayısal değer/özet gitmez (testle) | ✅ | `capsule` sorgusu `requires`/`rests_on`; tarama testi `entitlement_sweep_no_licensed_value_reaches_an_unentitled_reader` (her sorgu/konu/sunum yolu). |
| 10 | Sunucu olgusu uçtan uca; güven yolu şartnamede; geri çıkarma saldırısı raporda | ✅ | Sentetik lisanslı akış → `flow.institutional` → mühür → `server_comment` (döküm 39); PROTOCOL §20; `M6/RESULTS.md`: kâhin saldırısı bit p90 1,22, yapı gereği sınır ≤ 2,81 bit/hedef bar. |
| 11 | İtiraz ve geri çekme: M7; uygulandı; "geri çekildi" dökümde | ⚠️ | §19.9, döküm 37 ve 43; yanlış/erken/çelişkili gösterim her senaryoda 0; kaçırılan kesin iddia AUD1 7 → 0, AUD3 7 → 0, M7X 51 → 1; **AUD2 49 → 53 (iyileşmedi**: yansının REFRESH'siz 240 s kesintisi). |
| 12 | Üç seviye, iki grup varsayılanı, pozisyon/niyet bağımsız, sistem değiştirmez; ≥250 gün × ≥30 enstrüman benzetimi | ✅ | `S/sensitivity.md` (30 × 500 × 3 tohum; belirleyici, iki koşu bayt eşit): `important` 1,59/gün, `balanced` 2,06, `all` 3,23; gerçek veri `S/sensitivity-real.md`. Bulgu: pozisyon/niyet geçişi `important`'ı dolduruyor (A22). |
| 13 | M1–M7 düzenekler önce, alternatifler, ham, seçim; DECISIONS'ta öneri | ✅ (notlarla) | SPEC `a35d726` < bütün alternatifler; ham `experiments/M*/raw/`; DECISIONS Ö29–Ö40. Notlar aşağıda ("Sapmalar"). |
| 14 | Rejimli sentetik; gerçek veride ayrı ölçüm; sentetik karnenin anlamsızlığı yazılı; gerçek veri depoda yok | ✅ | `qp_source::synthetic_market` (dört rejim); M3/M4/M5 gerçek veri tabloları; `git ls-files | grep fixtures/local` boş. |
| 15 | İnsan testi paketi | ✅ | `human-test/` (iki kitapçık sırası, yönerge ~16 dk, form, puanlama, örnek). |
| 16 | Protokol eki; yeni dökümler üç taşımada; var olan 36 değişmeden; ≥20 kapsül görevi | ✅ | PROTOCOL §19.9, §20; dökümler 37–43 (+ düzeltmelerden gelenler) × inproc/serve/MCP; 36 eski döküm bayt eşit; E: C01–C20 iki koşu, son 19/20 ve 20/20, ilk deneme 12/20 ve 11/20. |
| 17 | 30 enstrüman × altı dedektör canlı tik; RP2 tabanıyla karşılaştırma; WASM | ⚠️ → bkz. "Performans" | Ölçüldü; ilk ölçüm otoritenin kapsül yolunda ciddi bir maliyet buldu (bar kapanışı 3,65 s, bağlanma 161 s, 1.000 bar geçmişte); düzeltme ve yeniden ölçüm aşağıda. WASM varsayılan 616.021 B gzip (+%3,0), yansılı 1.310.330 B (+%20,4). |
| 18 | REPORT, AUDIT; STATUS, ROADMAP; CAPSULE uygulamayla tutarlı | ✅ | Bu dosya; `AUDIT.md`; CAPSULE §9 (çerçeve değişmedi, §9.9 seçimler). |

## Ölçüm ve seçim özeti (M1–M7)

| # | Alternatifler | Seçim (öneri) | Belirleyici ölçü |
|---|---|---|---|
| M1 | siluet, rozet, iz (özgün) | **siluet** (ön eleme; insan testi siluet ↔ rozet) | Ayırt edilebilirlik en kötü koşul 4,60 / 2,55 / 7,55 ΔE; okuma açıklamasız tür 0,88 / 0,85 / 0,67, açıklamalı tam doğru 0,83 / 0,92 / 0,75, öğrenme tür 0,75 / 0,94 / 0,75. Birleştirme kuralı sonuç görüldükten sonra kondu (açıkça yazıldı). |
| M2 | şablon, dil modeli + yuva | **şablon** | Anlama tam doğru TR 0,98 / EN 0,94 vs 0,88 / 0,71; belirleyicilik 1,00 vs 0,08 / 0,14; ~µs vs ~8 s + 0,006 $. |
| M3 | nadirlik, karne, teyit birleşimi | **teyit birleşimi** | Bütünleşik koşum: en düşük değişim katsayısı (0,74), en az boş gün (0,15); gerçek günlükte tek pozitif K1−K3 (+0,088, 200 kapanış). Karne kuralı soğuk başlangıçta çöküyor. |
| M4 | tür + yön, yön yayı | **tür + yön** | Yanlış birleştirme ikisinde 0; kaybolan bilgi ~%4 vs ~%22–30; yay %11–13 daha az kapsül. |
| M5 | oynaklık bariyeri, türün anlamı | **bariyer** | Gerçek − yalancı eş sentetikte ≈ 0 (doğru), gerçekte +0,088 / +0,043; "türün anlamı"nın farkı yapısal (sentetikte de 0,16). |
| M6 | kaba ilişki, mühür (özgün) | **mühür** | Yapı gereği sınır ≤ log2 7 bit/hedef bar; sınanabilir makbuz; gecikme 0; iletiler 9.503 vs 21.901. Kaba ilişki deneysel kuyrukta daha iyi (3,98 vs 5,42 bit). |
| M7 | itiraz + kesin bar hijyeni | uygulandı | M1–M3 = 0; AUD1/AUD3 7 → 0; AUD2 iyileşmedi. İyileşmeyi hijyen taşıyor (ablasyon). |

Sentetik veride karne bir başarı oranı kanıtı değildir (veri öngörülemez; M3'te bütün
sıralayıcılar sentetikte K1−K3 ≈ 0). Görsel ve metin okuyan alt ajanlar insan gözü/okuru
değildir; M1 ve M2'nin okuma ölçüleri ön elemedir.

## Performans (ölçüt 17)

Kaynak `experiments/S/S-D.md`. Apple M4 Max; yük ortalaması ölçüm anında 5–14.

| | Değer |
|---|---|
| RP2 tabanı (aynı gün yeniden koşum) | bağlı N=5 0,78 µs, N=500 8,78 µs; belge içi N=5 2,04 µs (RP2 raporuyla ±%4,5) |
| Motor yalnız (30 enstrüman × 7 dedektör) | pazar tiki 215 µs (enstrüman başı 7,2 µs; adaylar 204 µs), bar kapanışı 218 µs; `ledger_stat` 13 ns (S-C dizini: 5.784 → 14 ns) |
| Oturum, ilk ölçüm (1.000 bar geçmiş, 30 enstrüman) | düzeltme tiki 11,3 µs/olay; bar kapanışı **3,65 s** (lisanslı akışla 8,0 s); bağlanma **161 s** — maliyet motorda değil, oturumun kapsül yolunda (cümle tazeleme %50, okuyucu projeksiyonu %45, `hello` özeti) |
| Oturum, düzeltmeden sonra | (aşağıda doldurulur) |
| WASM gzip -9 | varsayılan 616.021 B (+17.697, +%3,0; RP2 598.324); yansılı 1.310.330 B (+222.319, +%20,4) |

## Sapmalar

1. **M1 birleştirme kuralı önceden yazılmamıştı.** SPEC yalnız ölçüleri tanımlıyordu; üç alfabe
   ölçülerde farklı sıralandı; kural (hiçbir ölçüde sonuncu olmayan) sonuç görüldükten sonra
   kondu ve `M1/RESULTS.md`'de açıkça yazıldı. İnsan testi iki finalisti karşılaştırır.
2. **M1 okuma sayfalarında aşama sabit kaldı** (düzenek kusuru; açıklamasız/açıklamalı koşulda
   aşama ve güç doğruluğu bilgi taşımaz). Düzenek alternatifler görüldükten sonra
   değiştirilmedi; insan testi sayfalarında düzeltildi.
3. **M3–M5 karşılıklı bağımlı:** düzenek ölçülmeyen stratejiler için başvuru kurallarını
   kullandı; karar, SPEC'in öngördüğü bütünleşik yeniden koşumla verildi (`QP_R3_THEME`,
   `QP_R3_OUTCOME`, `QP_R3_RANKER` → `raw-chosen/`; ölçü kodu değişmedi, ayrı commit).
4. **Bağımsızlık:** iki alternatif ajanı (M1c, M3c) paylaşılan geçici klasörde başka bir ajanın
   betiğinin kendi betiklerinin üstüne yazıldığını, öbür ajanın dosyalarına bakmadıklarını
   bildirdi. Alternatiflerin kodları ayrı dallarda; ALTERNATIVES.md öncülleri `r3/int`'teydi,
   ajanlara açmamaları söylendi (açtıklarına dair iz yok; doğrulanabilir değil).
5. **Ürün cümlesi ölçülen çıktıdan sapıyor:** S-A'da iki düzeltme (akış ifadesi; bariyer altında
   aşırılık) 14 durumu, S-C'de Türkçe ünlü uyumu düzeltmesi 7 durumu değiştirdi; liste
   `S/crosscheck` `CHANGED` (20 durum), kalan 53 bayt eşit.
6. **Gerçek kurumsal akış yok:** M6 ve akış ailesinin bütün ölçüleri sentetik.
7. **Disk doldu** (bir ara 118 MiB); bitmiş ajanların derleme klasörleri silindi (~200 GB).

## Güvenlik incelemesi bulguları ve giderilmesi

| Bulgu | Önem | Giderme |
|---|---|---|
| İtiraz cevabı iddianın varlığını yetkisize sızdırıyordu; `seen` kümesi sınırsızdı | ORTA ×2 | M7 `75666b7`: aynı `unknown` cevabı; özet biçimi ve bağlantı/iddia başına sınırlar; testler |
| Grafik derinliği kanıt değerlerini görünüme sabit olarak yazıyordu; eksik değer 0 çiziliyordu | ORTA | S-B `9425d12`: yalnız kökenle bağ, değer okuyucunun sağlayıcısından; eksik değer tipli hata; test |
| Kitle (izleme listesi, pozisyonlar) başka okuyuculara gidiyordu; öneri yolu da | ORTA ×2 | S-C: kitle yalnız sahibine ve barındırıcıya; kitle işlemleri önerilemez; bütün yolları kapsayan test |
| Mühür demo anahtarları kodda; otorite hep onları kullanıyordu | ORTA → YÜKSEK (geç ayar etkisizdi) | `96772bc`, `7d58b5e`: anahtarlar kurulumda (`new_with_trust_keys`, `with_provider_and_trust_keys`); ilk yorumdan sonra `set_trust_keys` reddedilir; demo anahtarları yalnız sentetik test beslemesi ve dökümler; testler |

## Denetim

(Son denetim sonucu `AUDIT.md`'den buraya özetlenir.)

## Açık sorular (ana akış için)

1. Ö29–Ö40 kabul mü? Özellikle Ö32 (alfabe, insan testine kadar), Ö37 (mühür ve "yetkisize
   lisanslı hikâye yalnız mühür iletileriyle" kuralı — K34 ile gerilim: kanıtın varlığı ve
   dayanağı mühür ekleriyle söyleniyor, olgunun kendisi değil), Ö38 (AUD2 iyileşmedi).
2. A21–A26 (`DECISIONS.md`): insan testi sonucu; pozisyon/niyet geçişinin `important`'ı
   doldurması; karne anahtarında yön; `CapsuleState::retracted`; mühür makbuzunun kullanıcıca
   doğrulanması; hijyenin REST kotası.
3. Kapanmış hikâyelerin belgeden arşivlenmesi (belge geçmişle büyüyor; S-D).
4. Açık bir kırılım hikâyesi koşulu artık tutmuyorsa cümle "Artık geçerli değil" diyor, ama sonuç
   kuralı hikâyeyi henüz kapatmadı (`c2-capsule`): insan okuru için kafa karıştırıcı olabilir;
   durum koşulu ile beklenen çözüm `MeaningCondition`'da ayrılmalı (S-A önerisi).
5. İnsan testi cevaplarının (anonim) depoya girip girmeyeceği.

## Öneriler (kapsam dışı, yapılmadı)

- `capsule` sorgusu seçici kabul etsin; `capsules` sorgusunda "en son doğan" ve doğum barı
  süzgeci; seçicide `stage`/`open` anahtarı; §17.1'deki `{"kind":"breakout"}` örneği tam olgu
  türüyle (E: kimlik gerektiren 9 görevde ilk deneme 4/18).
- `Audience` biçimi §20.3'te açık yazılsın (E: C18).
- `RankCtx`'e yakın geçmiş (son aynı tür hikâyeden beri bar), aynı bardaki pazar genişliği ve
  periyot (M3 ajanları).
- Karne anahtarına yön (A23); nötr ve yönlü için ayrı taban.
- `iz` alfabesi öğrenme süresi uzun özgün dil olarak R4/R8 insan testinde.
- Mühür makbuzu için belirleyici Ed25519 (A25).
- `CapsuleState::retracted` (`qp/1`, A24).
- M1 düzeneğinde kanal çeşitlemeleri aşamayı da dolaşsın.
- Ligatürsüz Türkçe sayı biçimi ve birim (akış değerleri birimsiz).
- Yansılı WASM paketi 1,31 MB gzip: ayrı paket kararı (Ö26, A13) güçlendi.
- `scripts/qapsule-gh.env` izlenmiyor (R1'den beri açık öneri).
