# R5 — Rapor: Dil ve Tasarım

- **Durum:** Ölçümler, seçimler ve bütünleştirme bitti; S `main`'de (`2965a95`, bağımsız verify
  18/18). Bağımsız son denetim (A) ve round sonu temizliği bekliyor. Kabul ölçütleri (kendi
  değerlendirmem; A'ya bağlı olanlar yer tutuculu): aşağıdaki tablo. Bağımsız denetim: `<A:denetim-hükmü>`.
- **Tarih:** 2026-11-17 / 2026-11-18
- **Alt ajanlar:** Başlatılabildi (ilk iş deneme alt ajanı, haiku, cevap verdi). Bütün alt görevler
  (T0–T3c, 11 tasarım alternatifi, 6 ölçüm, D, G, H, S, belgeler) ayrı alt ajanlarla, ayrı
  worktree ve dallarda yapıldı. Orkestratör görev tariflerini, düzenek eklerinden Ek G3'ü (karar),
  seçim kurallarının yeniden uygulanışını (M18 ve M21 ekleri), birleştirmeleri ve `DECISIONS.md`
  "Önerilen (R5)" bölümünü yazdı. Bu rapor ayrı bir alt ajanın işidir (`r5/report`).
- **Dallar:** `r5/t0-small`, `r5/t1-research`, `r5/t2-glance`, `r5/t2-lang`, `r5/t2-broken`,
  `r5/t2-g4`, `r5/t3-look`, `r5/t3b-look`, `r5/t3c-glyph`, `r5/m17-{selvi,tunca,marla}`,
  `r5/m17-measure`, `r5/m17-int`, `r5/m16-{bertan,kivra,oltan,zemre}`, `r5/m16-measure`,
  `r5/m18-protocol`, `r5/m18-{korda,selen,pavka}`, `r5/m18-measure`, `r5/m18-pavka-perf`,
  `r5/m19-feed`, `r5/m20-{tesir,nobra}`, `r5/m20-measure`, `r5/m21-{surek,nabiz}`,
  `r5/m21-measure`, `r5/d-density`, `r5/g-gallery`, `r5/h-human`, `r5/docs-design`, `r5/s-int`,
  `r5/report`. Elenenlerin kodu dalında kalır (ör. `tunca`: `r5/m17-tunca` 0a37561).
- **Gerçek veri:** `fixtures/local/` yalnız yerelde (31 enstrüman günlük, 2 220 gün); yalnız M19
  benzetiminde kullanıldı, rapora yalnız toplu sayı girdi. Tasarım ölçümleri, galeri ve altınlar
  sentetik.

> **Ajan insan değildir.** Bu round'daki her tasarım seçimi ajan okuyucu ve eleştirmenlerle
> yapılmış bir ön elemedir. Her konuda ilk iki aday gösterimde ve galeride seçilebilir kalır. Son
> söz Arkin'in telefondaki karşılaştırması ve insan testi v3'tür (aşağıda ayrı bölüm).

## Özet

R5 kapsül deneyiminin dilini ve görsel dilini kurdu. Her konu için düzenek ve puanlama kuralı
alternatiflerden önce commit edildi, alternatifler birbirini görmeden ayrı ajanlarca kuruldu ve
kör okuyucu + eleştirmen ajanlarıyla ölçüldü. Bugünkü tasarım (`r4`) her ölçümde başvuru olarak,
gizli bir bozuk tasarım da düzeneğin kendi sınaması olarak yarıştı.

| Konu | Gösterim varsayılanı | İkinci (seçilebilir) | Sonuç | Kaynak |
|---|---|---|---|---|
| M17 kapsül dili | **`marla`** | `selvi` | Üç aday ayırt edilemedi; önceden yazılı bağ çözme (kısa olan) | `experiments/M17/RESULTS.md` |
| M16 görsel dil | **`oltan`** (yüz `oltan`) | `zemre` | Ayırt edildi (kısma kopyası 0,471, öteki adaylardan ayrık) | `experiments/M16/RESULTS.md` |
| M18 hikâye çizelgesi | **`pavka`** | `korda` | Ayırt edildi (0,987 ↔ 0,928); başarım düzeltmesinden sonra K52'yi geçti | `experiments/M18/RESULTS.md` (+ek), `PAVKA-PERF.md` |
| M19 akış | kural **`rate55`** (K42 muaf); sıra `elikov` kaldı | kural `none`; sıra `grouped`, `live_new` | Hiçbir kural "hiç gizleme"den ayırt edilemedi; bağ çözme `rate55`; sırada ölçümde `grouped` önde | `experiments/M19/RESULTS.md` |
| M20 açılmış kapsül | **`tesir`** | `nobra` | Ayırt edildi (0,968 ↔ 0,848), ama tek bir anahtar seçimine duyarlı | `experiments/M20/RESULTS.md` |
| M21 hareket | **`surek`** (geçici) | `nabiz` | **Ölçüm geçersiz**, kazanan ilan edilmedi; geçici varsayılan önceden yazılı bir kapıyla | `experiments/M21/RESULTS.md` (+ek) |
| D yoğunluk | `oltan` R2'nin altı görünümünde | — | `oltan` = `r4` okunurluk (ayırt edilemedi, tavan); karşıtlık ve yazı boyu tutuyor; seri paleti ve açıklama kaybı R7'ye | `experiments/D/RESULTS.md` |

Ölçüm maliyeti toplam ≈ **47,7 $** (aşağıda tablo). Kapsül dili taramaları (13 deterministik
tarama, 531 durum × 2 dil) iki seçilen dilde 0 bulgu (`r4` 9 266). Seçilen görsel dil bütün sert
eşikleri geçiyor: metin karşıtlığı en kötü 6,33; kapanmış işaret dahil işaret karşıtlığı 3,46 (A31
kapandı); en küçük yazı 13 px; 44 px'ten küçük dokunma hedefi 0; anlam renkleri beş görüş koşulunda
ΔE00 ≥ 10. Gösterim varsayılanları bir arada K52: JIT açık tahmininde (bağlayıcı) tik / yüz / sahne p95 1 / 1 / 3 ms, ilk kapsül 212 ms, kaydırmada uzun kare 0; hikâye çizelgesi 200 hikâyede ≤ 1 ms (JIT kapalı sahne boyaması 14,8 ms, bu kipte bağlayıcı değil, uzun kare 0). Paket: WASM gzip 1 463 643 → 1 771 943 B (+%21,1), brotli 967 161 → 1 146 726 B (+%18,6); gömülü yazı tipleri +97 604 B ham.

## Ne yapıldı

| # | Görev | Yürüten | Doğrulayan | Çıktı |
|---|-------|---------|------------|-------|
| T0 | Ana akış belgeleri (`a3a5d27`); `scripts/clean.sh` (`--dry-run`, `--force`, `--node-modules`; kilitli worktree koruması); worktree başına `target/wt/<görev>` (Ö61); eski 19 worktree'nin temizliği; `cpu` yolu kaldırıldı (`?paint=cpu` → `canvas`, bunu söyler); ölçüm ekranında "tik + geçiş" bölme hatası (yalnız veri geçişleri, en az 10 veri çerçevesi, yoksa "yetersiz örnek") | alt ajan + orkestratör | verify 18/18 (ayrı `target/wt`) | `4574c42`, `a9d4b64`, `8587b0d`, `ce3f55c`, `f5a480d`; birleşme `bb71e92` |
| T1 | Araştırma: özet/kart dili, anlam kodlama, tek bakış, yazı tipleri (fontdue tuzakları yerel denemeyle), hareket, dokunsal, saat/kilit ekranı, erişilebilirlik, belirsizlik, somut dil, ajanların okuduğu arayüz, kümeleme, akış önceliği, ajan değerlendirmesinin sınırları; aday fikirler F1–F22, düzenek uyarıları U1–U12, elenen fikirler | alt ajan | denetim (A) | `RESEARCH.md` (72 bağlantı; her kaynakta [A]/[Ö]/[Y] durumu); birleşme `efd557d` |
| T2 | Düzenekler, alternatiflerden önce: dil (M17) ve akış (M19) `fa0487d`; tek bakış (M16/M18/M20/M21) `70d8e11`; bozuk tasarım görünüş kaydı olarak + Ek G3 `0e85cb6`; Ek G4 `9c01848` | 4 alt ajan | kuru koşular, bozuk tasarım sınaması | `experiments/harness/{lang,feed,glance}/` (SPEC §G, Ek G1–G7), `DRYRUN.md`, `BROKEN.md` |
| T3, T3b, T3c | Görünüş kaydı (`Look`): yazı ve renk rolleri, kabuk boyları, kanıt geometrisi, sahne ofsetleri, glif ilerleme tablosu, işaret tabanları (`closed_alpha`, `fresh_alpha_min`, `plate_alpha_min`), `mark_role`, yarı kalın yüz; `?look=`, `qp look list/show/css`; `style.css`'te sayı yok (arch-check R7); sözlüğe "Görünüş" | 3 alt ajan | verify 18/18 her birleşmede; `r4` v1 altınları bayt eşit | `docs/GRAMMAR.md` §9a, `crates/qp-grammar/src/look.rs`; birleşmeler `8650c2a`, `70cb646`, `6808268` |
| M17 | Kapsül dili: `selvi`, `tunca`, `marla` (ayrı ajanlar); ölçüm; bütünleştirme (`?voice=`, sözlük dokununca açılır, yetkisize şablonlu rakamsız yüz A32, ayrıntı yüzün yapı taşlarından A33, S1 düzeltmesi) | 3 + 1 + 1 alt ajan | kör okuyucu (haiku tam, sonnet alt küme) + eleştirmen; 13 tarama | `experiments/M17/`, `docs/LANGUAGE.md`; birleşme `71c2aec` |
| M16 | Görsel dil: `bertan` (sakin editoryal), `kivra` (yoğun, keskin), `oltan` (işaret önde / saat yüzeyi), `zemre` (zaman şeridi; alışılmışın dışı, `r4`'ü görmeden); T3c sonrası yeniden tabanlama; ölçüm (Ek G5 ölçümden önce) | 4 + 4 + 1 alt ajan | kör okuyucu + eleştirmen; sert eşikler; K52; paket | `experiments/M16/`, `docs/DESIGN.md`; birleşme `ba94379` |
| M18 | Protokol eki (hikâye listesi sorgusu ve konusu, PROTOCOL §23, dökümler 60–63) önce ve ortak; `korda` (ömür şeritleri), `selen` (fiyatın üstünde işaret ve iz), `pavka` (hikâye omurgası, fiyat bağlam şeridi); ölçüm; `pavka` başarım düzeltmesi | 1 + 3 + 1 + 1 alt ajan | kör okuyucu + eleştirmen; K52 5/50/200 | `experiments/M18/`; birleşmeler `c414e44`, `717c7f2`, `d28cd2c` |
| M19 | Akış kuralı (`qp_capsule::feedrule`), sıralamalar, boş akış, geriye dönük benzetim (29 kural × 3 sıra × K42 muaf/muafsız; sentetik + gerçek), döküm 64 | alt ajan | önceden yazılı kural; ürün ↔ düzenek denetimi (80 601 karnede 0 ayrışma) | `experiments/M19/`; birleşme `2e06721` |
| M20 | `tesir` (tek sütun: iddia → dayanak → kim doğruladı → karne), `nobra` (soru kartları); ölçüm (Ek G7 önce) | 2 + 1 alt ajan | kör okuyucu + eleştirmen; dokunma sayısı; K52 aynı ekranda | `experiments/M20/`; birleşme `85a7691` |
| M21 | `surek` (süreklilik: aynı nesne yer değiştirir; hareketi azalt'ta kalıcı değişim etiketi), `nabiz` (kısa çapraz geçiş, canlı kapsülde nabız, durum değişiminde tek vuruş ve iz); ölçüm (Ek G6 önce, Ek G6-8) | 2 + 1 alt ajan | kör okuyucu + eleştirmen; bilgi kaybı; K52 | `experiments/M21/`; birleşme `6606914` |
| D | Yoğunluk sınaması: `oltan` R2'nin altı görünümünde + 8 serili sınama görünümü, 4 boy, 2 tema | alt ajan | hesap (sahne tarifinden) + kör okuyucu | `experiments/D/`; `a350af7`, `3447d9b` |
| G | Karşılaştırma galerisi (`scripts/demo.sh --gallery`, `/gallery.html`): adaylar alt alta, aynı içerik, "bunu seçerdim" + not, tek dokunuşla özet, `?pick=` | alt ajan | verify adım 18 `tests/gallery.mjs` | `crates/qp-wasm/src/gallery.rs`, `shells/web/gallery.js`; birleşme `a4bbc50` |
| H | İnsan testi v3 paketi (GUIDE, TASKS, FORM, SCORING, DECISION, README) | alt ajan | — | `human-test/`; birleşme `89bba78` |
| — | `docs/DESIGN.md` | alt ajan | — | `eca289a`, `ded7b65` |
| S | Bütünleştirme: `oltan`+`zemre`, `pavka` (perf) + `korda`, `tesir`+`nobra`, `surek`+`nabiz`; gösterim varsayılanları; görünüş eşlik eden yüzünü getirir; `?feed_exempt=0`; galeri `FINALISTS`; v1 sabitleme + v2 altınlar; K52 önce/sonra; paket | alt ajan | bağımsız verify 18/18 | `r5/s-int` (`rounds/R5/experiments/S/{K52,GOLDENS}.md`, `k52/`); main'e birleşme `2965a95` |
| A | Bağımsız son denetim | `<A:denetim>` | — | `AUDIT.md` (`<A:denetim>`) |

## Kabul ölçütleri

| # | Ölçüt (kısa) | Sonuç | Kanıt |
|---|---|---|---|
| 1 | Araştırma: bağlantılı kaynaklar, aday ve elenen fikirler, ölçüm planı | ✅ | `RESEARCH.md` (72 bağlantı, kaynak durumu [A]/[Ö]/[Y]; §15 aday fikirler, §16 elenenler, §17 düzenek uyarıları) |
| 2 | Düzenek önce (commit sırası); bozuk tasarım sınaması; körlük kayıtlardan | ✅ (notlarla) | Düzenek `fa0487d` 00:07, `70d8e11` 00:20, `0e85cb6` 01:10 (18 Kasım) < ilk alternatif commit'leri (M17 `657a9b2` 00:30, M19 `2fa4931` 01:23, M18 `5db01d8` 01:28, M16 `a816424` 03:26, M20 `9cd323f` 06:09, M21 `a361aee` 06:13); ölçüme özgü ekler ölçümden önce (Ek G5 `85338e2`, G6 `fca8f33`, G7 `655e93e`). Bozuk tasarım M16, M17, M18, M20'de kesin son; M21'de fark edilmede son değil → o koşunun ajan sıralaması kullanılmadı (kural gereği). Körlük: her istemde `check_prompt` (M17 180 istemde yasak ad 0; M16 `prompts.md`'de aday adı 0). Not: bir tasarım ajanı paylaşılan scratchpad'i gördü ("Sınırlar" 2) |
| 3 | M17: taramalar 0; eşik adıyla ve değeriyle; karne beklentinin kendisi; ayrıntı yüzün yapı taşlarından (A33); sözlük iki dilde, dokununca; tablo LANGUAGE.md'de | ✅ | `M17/RESULTS.md` §2 (13 tarama `marla`/`selvi` 0, `contradiction` dahil), §11 (S1 düzeltmesinden sonra yine 0); `docs/LANGUAGE.md` "Yedi dedektör × yaşam döngüsü aşaması × iki dil", "Sözlük"; döküm `langh dump` ile bayt bayt yeniden üretilir |
| 4 | M16: dört yön; kayıtlar çekirdekte; kabukta biçem yok; bütün eşikler (kapanmış işaret dahil); gri + renk körlüğü ayrımı; DESIGN.md | ✅ | `M16/RESULTS.md` (dört yön; `oltan` metin 6,33, işaret 3,46, en küçük 13 px, dokunma 0, ΔE00 en kötü 10,53); gri kopya `dir`+`valid` 1,00; `crates/qp-grammar/src/look.rs`; arch-check R7; `docs/DESIGN.md` |
| 5 | Yoğunluk: altı görünüm, okunabilirlik, R7 soruları | ✅ | `D/RESULTS.md` (+ek: okuyucu kapsamı tam; `oltan` 0,983 = `r4` 0,983), §4 R7 soruları 1–9 |
| 6 | M18: bütün kapsüller, seçili vurgulu, kanıt yalnız seçiliye, odak taşıma, kümeleme, kapanmışlar görünür ve gizlenebilir; liste sunucudan (K48); protokol eki ve dökümler; 200 kapsülde K52 | ✅ | `M18/RESULTS.md` (+ek); PROTOCOL §23, dökümler 60–63 (3 taşıma); `pavka` JIT açık 5/50/200 bütçe içinde, JIT kapalı 50'de uzun kare 0,058/0,073; S: JIT açık 12 / 52 / 207 hikâyede tik, sahne boyama ve kurma p95 ≤ 1 ms; JIT kapalı (ham benzetim) sahne boyama p95 4 / 11 / 13 ms (`r4` 3 / 9 / 10) |
| 7 | M19: canlı ve yeni önce; eşik önceden yazılı, geleceğe bakmıyor; gizlenenler karnede; az örnekli söyleniyor; gizlenenler grafikte ve "Hepsi"nde işaretli; tablo; boş akış | ⚠️ | Kural, geleceğe bakmama, karne, az örnek, işaret, benzetim tablosu, boş akış ✅ (`M19/RESULTS.md` §2–6, `feed_m19.rs`, döküm 64). **Sıra:** `grouped`/`live_new` seçilebilir ama gösterim varsayılanı `elikov` kaldı (değişseydi v1 demo altınları ve `settings.mjs` değişirdi); sıra bir akış tasarım kararı olarak Arkin'e (Ö65) |
| 8 | M20: seçildi; sunucu/cihaz ayrımı anlaşılıyor; yetkisize şablonlu rakamsız yüz (A32); girişimsizlik testleri | ✅ (notla) | `M20/RESULTS.md`: `tesir` 0,968; yetkisizde rakam 0; `tesir` yetkisiz okur 0,993 (`r4` 0,500). Not: sonuç `e_who` anahtarına duyarlı; ölçüm `look=r4` ile |
| 9 | M21: seçildi; hareketi azalt'ta bilgi kaybı yok; ara kare ölçümü | ⚠️ | Ara kare ölçümü yapıldı ama **süreklilik sorusu geçersiz** çıktı ve bozuk tasarım sınaması geçmedi → kazanan ilan edilmedi; geçici varsayılan `surek` (önceden yazılı G6-5 kapısı), ikinci `nabiz`. Bilgi kaybı `surek` 0 (`nabiz` ana tanımda 0, lafzî tanımda 4). `M21/RESULTS.md` (+ek), A43 |
| 10 | Performans: seçilen tasarım `canvas`'ta PHONE ve JIT açık tahmininde K52; önce/sonra tablosu; paket önce/sonra | ✅ (notla) | `rounds/R5/experiments/S/K52.md`: gösterim varsayılanları birlikte, `canvas`, JIT açık tahmini (bağlayıcı) önce → sonra tik 1 → 1, yüz 1 → 1, sahne 4 → 3 ms p95, ilk kapsül 164 → 212 ms, kaydırmada uzun kare 0 → 0; PHONE (JIT kapalı) tik 1,6 → 3,2, yüz 2 → 1, sahne 9,25 → 14,8 ms (bu kipte K52 "kullanılabilir kalır" ister: uzun kare 0); hikâye çizelgesi 12 / 52 / 207 hikâyede JIT açık her ölçü ≤ 1 ms. Not: JIT kapalı sahne boyaması ve ilk kapsül kötüleşti (bütçe içinde); paket WASM gzip 1 463 643 → 1 771 943 B (+%21,1), brotli 967 161 → 1 146 726 B (+%18,6); gömülü yazı tipleri +97 604 B ham |
| 11 | İlk iki aday Ayarlar'dan ve adresle; galeri sentetikte, alt alta, "bunu seçerdim" + not, tek dokunuşla özet | ✅ | `gallery.rs` `FINALISTS` (dil, görünüş, hikâye, açılmış kapsül, hareket: ilk iki + `r4`), akış sırası ve kuralında bütün adaylar; `?voice= ?look= ?story= ?open= ?motion= ?feed_order= ?feed_rule= ?feed_exempt=0`; verify 18 `tests/gallery.mjs`; `main` `2965a95` |
| 12 | İnsan testi v3: görevler, form, puanlama; A21 dahil hangi kararı nasıl kapatacağı | ✅ (notla) | `human-test/` (G0–G9; DECISION.md: A21, A32, A33, K53 + K42 muafiyeti, akış sırası, M16–M21). Aday yer tutucuları (`<GÖRÜNÜŞ-1>` …): aday adları dolduruldu (`c301602`: `oltan`/`zemre`, `tesir`/`nobra`, `surek`/`nabiz`); içerik anları (`<ADIM-…>`, `<SATIR-…>`, `<KİTLE-…>`) yürütücünün provasında doldurulur (`GUIDE.md`) |
| 13 | `cpu` yolu kaldırıldı; bölme hatası; `clean.sh` (`--dry-run`), round sonunda çalıştı | ✅ / `<A:disk>` | T0 (`ce3f55c`, `f5a480d`, `4574c42`, `8587b0d`); round sonu koşusu `<A:disk>` |
| 14 | v1 altınlar yerinde; v2 eklendi; değişenler gerekçeli; hesap özeti aynı; dökümler 3 taşımada | ✅ | v1: `fixtures/golden` R5 boyunca değişmedi; v2: 16 çekim, 28 sahne PNG'si (772 KB): `feed-*` (akışın ilk üç yüzü), `closed-*` (kapanmış iki yüz), `capsule-*` (`tesir`), `chart-*` (`pavka`) × TR/EN × koyu/açık; iki koşuda bayt eşit; dökümler 64 × 3 taşıma (verify `conform` (bağımsız verify 18/18)); hesap özeti `7b1e99d40889f83c` (değişmedi) |
| 15 | REPORT, RESEARCH, AUDIT; DESIGN, LANGUAGE, FACE; STATUS, ROADMAP; Önerilen (R5); Arkin için galeri bölümü; disk | `<A:ölçüt15>` (AUDIT bekliyor) | bu dosya; `RESEARCH.md`; `docs/DESIGN.md`, `docs/LANGUAGE.md`, `docs/FACE.md`; `STATUS.md`, `ROADMAP.md`; `DECISIONS.md` Ö61–Ö70, A41–A44; son bölüm; `AUDIT.md` `<A:denetim>` |

## Konu konu: seçim, ilk iki, ölçüm, duyarlılıklar

### M17 — kapsül dili

- **Seçim `marla`, ikinci `selvi`; `tunca` elendi** (kodu `r5/m17-tunca`).
- Taramalar (13; 531 durum × 2 dil): üç adayda 0; `r4` 9 266, bozuk 34 006.
- Birincil anlama doğruluğu (haiku): `tunca` 0,914 [0,883; 0,937], `selvi` 0,898, `marla` 0,889,
  `r4` 0,869, bozuk 0,500 → **üç aday ayırt edilemedi**. `r4` de birincilde adaylardan ayrılmıyor;
  adayların üstünlüğü taramalarda ve eleştirmende.
- Eleştirmen BT: `selvi` 1,69, `marla` 1,41, `tunca` 1,35 (örtüşüyor; çelişki 0,36–0,39 > 0,30),
  `r4` −0,75, bozuk −3,69 → adaylar ayırt edilemedi, üçü de `r4`'ün ayrık önünde.
- Bağ çözme (§5.4, kısa olan): `marla` 222,4 karakter (başlık + ayrıntı, TR+EN) < `selvi` 234,9 <
  `tunca` 236,0 → `marla`.
- **§5.4 yorumu (duyarlılık):** SPEC "kısa olan"ın hangi adaylar arasından seçileceğini
  adlandırmıyor. Uygulanan okuma: 2.–3. adımda birbirinden ayırt edilemeyen bütün adaylar →
  `marla`. "Birincilin ilk ikisi arasında" okunursa `selvi` (1,1 karakter fark); "BT'nin ilk ikisi
  arasında" okunursa `marla`. `selvi` her okumada ilk ikide; bu yüzden ikisi de gösterimde.
- Sonnet alt kümesi büyük resimde aynı, ince sırada uyuşmuyor (aday çiftlerinde seçim uyumu 0,65).
- Bütünleştirmede S1 düzeltmesi (`continuation.ma_stack` başlığı: olay fiili önce ve düz; 97 durum,
  yalnız başlık; taramalar yine 0). **Okuyucu ölçümü yeniden koşulmadı.**

### M16 — görsel dil

- **Seçim `oltan` (ayırt edildi), ikinci `zemre`.** `bertan` sert eşiklerde (Ek G5 renk kapısı),
  `kivra` okuma aşağı kalmama kapısında elendi.
- Birincil (kısma kopyası, σ 4,5 + 0,5×, F2:H1): `oltan` **0,471 [0,445; 0,497]**, `bertan` 0,345,
  `r4` 0,318, `zemre` 0,279, `kivra` 0,243, bozuk 0,183. `oltan`'ın aralığı `zemre`'ninkiyle
  örtüşmüyor. Üstünlüğün kaynağı `t_first` (işaret ilk göze çarpan öğe) 0,83 ve `t_dir` 0,41.
- Tam boy okuma (kapı): `r4` 0,866, `oltan` 0,865, `zemre` 0,862, `bertan` 0,855, `kivra` **0,775**.
- Eleştirmen BT: `oltan` 2,15, `kivra` 1,79, `bertan` 1,30, `zemre` 1,24, `r4` −1,20, bozuk −5,29;
  `oltan|zemre` çelişki 0,33 (eleştirmen bu ikisini ayırmadı). Uzunluk yanlılığı yok (Spearman −0,03).
- K52: dört aday da geçti (`zemre` JIT kapalı kaydırmada %2,1 uzun kare; sıfır olmayan tek değer).
  Paket (brotli, `r4`'e göre): `oltan` +27,8 KB, `zemre` +25,4, `bertan` +29,0, `kivra` +71,6.
- **Ek G5 duyarlılığı / `bertan`:** `bertan`'ı eleyen tek eşik Ek G5 renk kapısı (açık temada
  `up ↔ invalid` gri tonlamada 7,28 < 10). Ek G5 olmasaydı kazanan yine `oltan`, ama ilk iki
  `oltan` + `bertan` olurdu. Ek G5 iki adayın kapı sonucunu çevirdi (eski sahne tablosuyla `zemre`
  kalır, `bertan` geçerdi). Ek G5 bir aday ajanın raporuyla tetiklendi ve ölçümden önce ayrı
  commit'te yazıldı (`85338e2`).
- **İşaret tabanları ve tazelik:** `oltan`'ın tabanları 255 (`closed_alpha`, `fresh_alpha_min`);
  işaret tazelikle solmuyor, yaş cümlede söyleniyor. Gösterimdeki kapanmış kapsüllerin tazeliği
  0,06–0,29 ölçüldü; taban olmadan koyu temada tutmadı izi 1,5–1,9:1'e düşüyordu. Ölçüme etkisi:
  `oltan` solmayan işaretle, öteki adaylar kendi tabanlarıyla ölçüldü (ör. `zemre` açık işaret
  172/180); karşıtlık kapısını herkes kendi çiziminden geçti.
- Kısma kopyasında her adayda TR < EN (bozuk dahil); dil mi model mi, ayrılamadı.
- Zayıf yan (A44): akışta canlıyı bulma (`f_live`) `oltan` 0,65 [0,45; 0,81] (n 23), bozuk
  dışındaki en düşük; seçime girmiyor, insan testinde bakılacak.

### D — yoğunluk sınaması

- `oltan` R2'nin altı görünümünde okunuyor: kör okuyucu `oltan` ile `r4`'ü **ayırt edemedi**
  (tam kapsamla haiku 0,983 [0,971; 0,990] ↔ 0,983). Okuma tavanda: sorular dil farkını ölçecek
  kadar zor değil.
- Hesapla: bütün metin öğeleri ≥ 4,5:1 (en kötü 5,67), en küçük yazı 13 px (`r4` 11); 112 sahnede
  metin çakışması, taşma ve 2 px'ten yakın metin çifti 0.
- Kırıldığı yerler (R7'ye): seri paleti s1–s8 gri tonlamada ayrılmıyor (28 çiftin 22–23'ü
  ΔE00 < 10); açık `s2` açıklama metninde 3,94; sığmayan açıklama sessizce düşüyor (telefonda 18'den
  10'u kalıyor); `muted` neredeyse metin kadar koyu (eksen yazıları veri çizgisinden belirgin);
  eksen çizgisi 2,88/2,95 (< 3).

### M18 — hikâye çizelgesi

- **Seçim `pavka`, ikinci `korda`** (ek; başarım düzeltmesinden sonra). İlk koşunun kararı `korda`
  + `selen` idi.
- Birincil (`chart`, F2:H1): `pavka` **0,987 [0,977; 0,996]**, `korda` 0,928, `selen` 0,868, `r4`
  0,411, bozuk 0,406. Canlıyı bulma (`c_live`, kapanmışlar görünürken): `pavka` 1,00, `korda` 0,96,
  `selen` **0,52**, `r4` 0,01.
- Eleştirmen BT: `pavka` 3,82, `korda` 1,37; ama **uzunluk yanlılığı şüpheli** (Spearman 0,70;
  daha çok metinli olanın kazanma payı 0,845); BT seçimde kullanılmadı.
- **K52 ve `pavka` başarım eki:** ilk koşuda `pavka` JIT kapalı sürüklemede 50 hikâyede %13,5 uzun
  kare ile (K52: ≤ %10) elendi. Bu kapı yorumu ilk JIT'siz sayılar görüldükten sonra yapıldı (ölçüm
  ajanı açıkça yazdı; SPEC §G7.6 o kipi "kayıt" sayıyordu). Tasarımı değiştirmeyen düzeltme
  (`r5/m18-pavka-perf`: ilk olgu haritası yalnız kümeli sahnede ve bir kez kuruluyor; şerit uçları
  bir kez) sahne tarifini bayt eşit bıraktı (7 488 sahne + cihazda 76 sahne özeti), bu yüzden okumalar
  geçerli kaldı. Yeniden K52: 50 hikâyede 0,058 / 0,073 (10 koşunun 1'inde 0,115), 200'de
  0,064 / 0,068 → kural yeniden uygulandı, `pavka` kazandı. Telefon JIT kapalı sürüklemede kare p99
  `pavka` 27–28 ms, `korda` 21: kullanılabilir ama daha ağır.
- Bozuk tasarım kesin son ama `r4`'ten ayrılmıyor (+0,006 [−0,028; +0,039]): bozuk kayıt hikâye
  çizelgesi olmayan `main` üstüne kurulduğu için adaylar arası duyarlılığı sınamıyor.

### M19 — akış önceliği ve gizleme eşiği

- **Kural `rate55`** (n ≥ 20 ve tutma < 0,55 → gizli), ikinci `none`. Gerçek veride `rate55`:
  kalan 3,18/gün, boş gün 0,138, kalanların sonradan tutma oranı 0,583 (`none` 0,556), gizlenenlerin
  0,509. Birincil (Brier becerisi) `rate55` 0,024 ↔ `delta` 0,022 ↔ `none` 0,019: **hiçbir kural
  "hiç gizleme"den ayırt edilemedi.** Gizleme güveni yazı turayı göstermeyerek korur, öngörüyü
  artırmaz.
- N'ye duyarsız (N 5–30 arasında kalan/gün 3,15–3,22); τ'ya çok duyarlı (0,60'ta günlerin %61'i boş).
- **K42 muafiyeti önerildi:** muafiyetsiz kural elde tutulan hissedeki olumsuz geçişlerin yalnız
  0,422'sini gösteriyor (muaf 1,000). Gösterim muaf çalışır; `?feed_exempt=0` muafiyetsiz biçimi
  açar (S).
- Sıra: `grouped` 0,881, `trust` 0,760, `live_new` 0,727 → `grouped` önde; gösterim varsayılanı
  `elikov` kaldı (altınlar), karar Arkin'in (Ö65).
- Düzeneğin ikincisi `rate τ=0,55 N=10` ürüne alınmadı: N=10 ürünün az örnek eşiğinin (20)
  altında; 10–19 örnekli kapsül "az örnek" derken gizlenirdi.

### M20 — açılmış kapsül ve kanıt ekranı

- **Seçim `tesir`, ikinci `nobra`.** Birincil (kapsül derinliği): `tesir` **0,968 [0,949; 0,987]**,
  `nobra` 0,848, `r4` 0,732, bozuk 0,726. Dokunma (dayanak / kim doğruladı / neden rakam yok): iki
  adayda 1/1/1, `r4`'te 1/1/2. Yetkisizde rakam 0 (hepsi). `r4` "neden rakam göremiyorum"a 0/48.
- **`e_who` anahtarı (duyarlılık):** `nobra`'nın düşüklüğünün tamamı "bunu kim doğruladı"
  sorusundan. Cümlesi "Sunucu doğruladı ve bu cihazda da sınandı"; okuyucular "sunucu" diyor, sıkı
  anahtar (Ek G7d, ölçümden önce) "bu cihazda sınandı" ister. Gevşek anahtarla (sunucu ∪ cihaz)
  `nobra` 0,998, `tesir` 0,979 → kazanan `nobra` olurdu, ilk iki aynı. Kural `tesir` dedi; anahtarın
  sonucu belirlediği açık. `nobra`'nın cümle sırasını çevirmek ucuz bir düzeltme olur.
- Eleştirmen BT `nobra` 2,41 > `tesir` 1,81 (uzunluk yanlılığı şüpheli, 0,971; kararı belirlemedi).
- Ölçüm `look=r4`, `voice=marla` ile yapıldı (görünüş henüz seçilmemişti).

### M21 — hareket

- **Kazanan yok; geçici varsayılan `surek`, ikinci `nabiz`; hareketsiz `?anim=0`.**
- **Geçersiz ölçüm:** süreklilik sorusu ("aynı nesne mi taşınıyor") son kareyle de %93–100 doğru
  cevaplandı; soru geçişi değil, açılan kapsülde aynı metnin görünmesini ölçüyor. `m_where` hiçbir
  tasarımda puanlanamadı. Kural mekanik olarak hareketsiz başvuruyu öne koyuyordu (yapay). Bozuk
  tasarım fark edilmede son değil (0,310; `r4` ve `still` 0,265) → Ek G3'e göre ajan sıralaması
  kullanılmadı.
- Geçerli kapılar: K52 (hepsi geçti; `surek` JIT kapalı yüz değişiminde %1,7) ve hareketi azalt
  bilgi kaybı.
- **Geçici varsayılan nasıl seçildi:** yalnız sonuçlardan önce yazılmış kurallar uygulandı. Ek
  G6-5'in lafzı ("bilgi metin olarak durur") `nabiz`'i doğum değişiminde eler (nabız → metinsiz
  canlı noktası; 4 kayıp); `surek` bütün kapıları geçer. Ek G6-8'in "herhangi bir durağan ilke"
  okuması kayıp sonuçları görüldükten sonra yazıldığı için yalnız duyarlılık: o okumada ve geçersiz
  adım atlanırsa fark edilme (0,515) ve BT (1,72) `nabiz`'i ayrık biçimde öne koyuyor.
- Ölçüm `look=r4` ile yapıldı; `surek` ve `nabiz` süreyi, rengi ve boyu görünüşten okur, `oltan`
  altında sonuçlar değişebilir.

## Ölçüm düzeneği ve R4 dersinin uygulanışı

R4'te persona ajanları görüntüyü büyütüp uzun okuyabildi ve ölçüm metin ağırlıklı tasarımı kayırdı
(K46). R5 düzeneği (`experiments/harness/glance/SPEC.md` §G0) bunu şöyle engelledi:

- **Gerçek boyut ve tek bakış:** 402 × 874 CSS px, DPR 1; tek görüntü, tek `Read`, `Bash` yok;
  her kayıtta `one_look` denetimi (tutmayan çağrı puanlamadan çıkarıldı; M16'da 18 kayıt). Cevap en
  çok bir cümle (ilk cümle ve 200 karakterden kesilir).
- **Kısma kopyası birincil ölçüdür** (σ 4,5 CSS px Gauss + 0,5× Lanczos). σ kuru koşuda ayarlandı:
  2,5'te ajan başlığı harfi harfine okudu (benzerlik 0,99), 4,5'te okuyamadı (0,18/0,42). Tam boy
  okuma yalnız "aşağı kalmama" kapısı ve son bağ çözücü (Ek G1-2): yazıldığı hâliyle tam boy
  birincil bozuk tasarımı sona koyamamıştı (bozuk 0,882 – `vadrel` 0,893).
- **Bozuk tasarım her koşuda** (Ek G3): gizli adlı bir görünüş kaydı (`kestim`: ters hiyerarşi,
  %65 zemine çekilmiş renkler, 12 px işaret), yalnız düzenekte yamayla derlenir, ürüne girmez. Kesin
  son değilse koşunun ajan sıralaması kullanılmaz. M16, M17, M18, M20'de geçti; M21'de geçmedi ve
  kural uygulandı.
- **Sınıf dengeli puan ve 6 tekrar** (Ek G3): `t_valid` ikili sorusunda tek cevaba saplanan okuyucu
  1/2 alır.
- **Hesapla ölçülenler ajanlardan önce kapıdır:** karşıtlık (1.4.3, 1.4.11), en küçük yazı,
  dokunma, ΔE00 beş koşulda, K52, paket.
- **İkili kör karşılaştırma:** iki sıra × tekrar, Bradley–Terry + önyükleme; çelişki > %30 ise o
  çift "ayırt edilemedi"; uzunluk yanlılığı denetimi (M18, M20'de şüpheli çıktı; BT seçimi
  belirlemedi).
- **Ölçüm öncesi ekler** (her biri, o ölçümün ilk okuyucu çağrısından önce commit edildi):

| Ek | Ne | Tetikleyen |
|---|---|---|
| G1 | Kuru koşu düzeltmeleri: σ 2,5 → 4,5, birincil = kısma kopyası, `m_where` p = 0,25, tek bakış dışı çağrı, kesin son | kuru koşu (hiçbir alternatif yokken) |
| G2 | Bozuk tasarım görünüş kaydı olarak | T3'ün `Look` kaydı |
| G3 | Sınıf dengeli `t_valid`, 6 tekrar, bozuk her koşuda | orkestratör kararı (`BROKEN.md` riskleri), M16 alternatiflerinden önce |
| G4 | `voice` aktarımı, renk çiftleri tema başına, işaret rengi = en opak ilkel | **bir M16 aday ajanının raporu** |
| G5 | Anlam rengi kapısı görünüş kaydının rollerinden; K52 JIT'siz ≤ %10 kapı | **bir M16 aday ajanının raporu**; M16 ölçümünden önce (`85338e2`) |
| G6 | M21: süreklilik geçerliliği, fark edilme, bilgi kaybı, seçim kuralı | M21 ölçüm ajanı, ölçümden önce (`fca8f33`) |
| G6-8 | Bilgi kaybı uygulama düzeltmesi ve tanım boşluğu | kayıp sonuçları görüldükten sonra, okuyucu sonuçlarından önce (`69d27cc`); bu yüzden yalnız duyarlılık |
| G7 | M20: iki derinlik, deterministik dokunma sayısı, K52 aynı ekranda, `e_who` anahtarı, rakam taraması | **M20 aday ajanlarının raporları**; görüntü çekilmeden önce (`655e93e`) |

- **Çözünürlük sınırı ≈ 0,02.** Ek G3'ten sonra bozuk tasarım ile `fotolay` arasındaki eşleştirilmiş
  fark +0,015 [−0,010; +0,041] ayrılmadı; M17'de 3 okuyucuyla birincil aralık ≈ ±0,03 ve adaylar
  arası nokta farkları (≤ 0,025) bunun içinde. Bu düzenek ≈ 0,02'den küçük farkları ayıramaz;
  "ayırt edilemedi" sonuçları bu sınırla okunmalı.

## Ajan insan değildir

- Ajan okuyucu küçük metni insandan iyi okur, renk ve açıklık hiyerarşisini zayıf okur (RESEARCH
  §14). Tam boy okumada her tasarım tavana yakın (0,78–0,87); kanıt derinliğinde 0,95–0,98;
  yoğunlukta 0,98. Tasarımlar ancak kısma kopyasında ve grafik görevlerinde ayrılıyor.
- Hareketi bir insan gibi deneyimlemez: M21'de "aynı nesne" sorusunu son kareden cevapladı. Bir
  geçişin hissi, nabzın rahatsız edip etmediği ajanla ölçülemedi.
- "Beş saniyelik bakış" personası (F) ajanda görünmüyor; F ile H çoğu ölçümde yakın.
- Eleştirmen daha çok metni ödüllendirmeye eğilimli (M18 0,845, M20 0,971).
- Tasarımları ve okuyucuları aynı model ailesi üretti; kendini kayırma yazar bilgisi gizlenerek
  azaltıldı, giderilemedi.
- Bu yüzden her konuda ilk iki aday gösterimde ve galeride seçilebilir; son söz Arkin'in
  telefondaki karşılaştırması ve insan testi v3'tür (`human-test/DECISION.md`: insan sonucu ajan
  sonucundan farklıysa insan sonucu geçerli).

## Sınırlar ve sapmalar

1. **Tek model ailesi.** `OPENAI_API_KEY` ortamda yok; ikinci aile kullanılmadı (anahtar
   istenmedi, `.env.local` açılmadı). Aynı aileden iki boy (haiku tam, sonnet alt küme) arası uyum
   raporlandı: M16 0,861, M18 0,875; M17'de aday çiftlerinde seçim uyumu 0,65.
2. **Körlük sızıntısı.** Bir M18 tasarım ajanı (`selen`) paylaşılan scratchpad'de başka bir ajanın
   (`m17-int`) dosyalarını gördü. Okuyucu ve eleştirmen körlüğünü etkilemez (istemler ayrıca
   denetlendi), ama tasarım ajanlarının birbirini görmemesi kuralını çiğner. Kural sonra
   sıkılaştırıldı: her ajan yalnız `scratchpad/agents/<görev>/` altına yazar, başka klasörleri
   listelemez (COMMON.md).
3. **Kesintiler.** Round'un başında bir ağ kesintisi ilk altı ajanı durdurdu (devam ettirildiler,
   worktree'ler yeniden kuruldu). İki kez kullanım sınırı (06:16 ve 09:06) ajanları durdurdu: M21'de
   2 654, D'de 354 okuyucu çağrısı cevapsız döndü; yanlış sayılmadı, silinip aynı tohumla yeniden
   koşuldu. Ders: devam ettirilecek bir ajanın worktree'si kaldırılınca ajan sürdürülemiyor; M16
   yeniden tabanlaması taze ajanlarla yapıldı.
4. **M20 ve M21 `look=r4` ile ölçüldü** (görünüş henüz seçilmemişti); S'de `oltan` altında
   yeniden okunmadı.
5. **Adaylar ayrı derlemelerle çekildi** (dallar birleştirilemediği için); içerik eşitliği her
   koşuda denetlendi (M16: 216 görüntüde hedef kapsül ve adım aynı; M18: 533 satırlık hikâye listesi
   beş derlemede aynı).
6. **Bozuk tasarım M18 ve M20'de `r4`'ten ayrılmıyor:** bozuk kayıt bugünkü tasarımın üstüne
   kuruldu; adaylar arası duyarlılığı sınamıyor. Sonraki ölçümde bir aday tasarımın üstüne kurulmalı.
7. **M18 K52 kapı yorumu** ilk JIT'siz sayılar görüldükten sonra yapıldı (ölçüm ajanı açıkça
   yazdı); M16'da aynı kapı ölçümden önce Ek G5-4 olarak yazıldı.
8. **Yeniden okunmayanlar:** M17 S1 düzeltmesi, `pavka` başarım düzeltmesi (sahne bayt eşit olduğu
   için gerekmedi), bütünleştirmedeki tasarım ayarları.
9. **K52 makinesi paylaşımlıydı** (başka ajanların derlemeleri ve okuyucu çağrıları sürerken; yük
   7–15); JIT kapalı değerler WebKit benzetimi (RP3 kalibrasyonu, `PHONE`).
10. **Grafik derinliği ve `pavka` yazısı (S'de düzeltildi):** cihaz grafik görünümünü zaten
    görünüşten kuruyordu; otoritenin `tap_at`/`hit` görünümü `r4` ile kuruluyordu, artık o da
    görünüşten (`e447571`; v2 altınlar bayt eşit kaldı). `pavka`'nın "kapanmış" kutusu `oltan`
    boylarında kesiliyordu; sözcükler derleyicinin ölçüsüyle ölçülüyor (`55f0884`; `r4` bayt eşit).
11. **S'nin kalan sorunları:** JIT kapalı (PHONE) kipte gösterim varsayılanlarıyla sahne boyaması 9,25 → 14,8 ms ve hikâye çizelgesinde 50/200 hikâyede `r4`'ten 2–3 ms fazla (bu kipte bağlayıcı değil, uzun kare 0); ilk kapsül 164 → 212 ms (bütçe 1 500). K52 ölçümü başka işler sürerken tek koşu dizisi (`S/K52.md`).

## Ölçümler (önce / sonra)

| Ölçü | R4 / RP3 (önce) | R5 (sonra) | Kaynak |
|---|---:|---:|---|
| Kapsül dili taramaları (13; 531 durum × 2 dil) | `r4` 9 266 | `marla` 0, `selvi` 0 | `M17/RESULTS.md` §2 |
| Yüz: metin karşıtlığı en kötü | 2,75 (`karnesi yazı tura`, 11 px) | 6,33 (`oltan`) | `M16/RESULTS.md` |
| Kapanmış işaret karşıtlığı (A31) | 2,06 | 3,46 | `M16/RESULTS.md` |
| En küçük yazı | 10 px | 13 px | `M16/RESULTS.md` |
| 44 px'ten küçük dokunma hedefi | 108 | 0 | `M16/RESULTS.md` |
| Anlam renkleri en kötü çift ΔE00 (beş koşul) | 0,50 | 10,53 | `M16/RESULTS.md` |
| Tek bakış, kısma kopyası | 0,318 (`r4`) | 0,471 (`oltan`) | `M16/RESULTS.md` |
| Grafikte "kaç olay / kaçı tuttu / canlı hangisi / kanıtı" | 0,411 (`r4`) | 0,987 (`pavka`) | `M18/RESULTS.md` |
| Açılmış kapsül: dayanak / kim / neden rakam yok | 0,732 (`r4`) | 0,968 (`tesir`) | `M20/RESULTS.md` |
| K52, akış + yüz + grafik (gösterim varsayılanları) | tik / yüz / sahne p95 1 / 1 / 4 ms (JIT açık); ilk kapsül 164 ms; JIT kapalı sahne 9,25 ms | 1 / 1 / 3 ms; 212 ms; 14,8 ms (uzun kare 0) | `S/K52.md` |
| K52, hikâye çizelgesi 5 / 50 / 200 | — | JIT açık 12 / 52 / 207 hikâyede tik, sahne boyama ve kurma p95 ≤ 1 ms; JIT kapalı (ham benzetim) sahne boyama p95 4 / 11 / 13 ms (`r4` 3 / 9 / 10) | `S/K52.md` |
| WASM gzip / brotli | 1 463 643 / 967 161 B | 1 771 943 / 1 146 726 B (+%21,1 / +%18,6) | `S/K52.md` §3 |
| Uygunluk dökümleri | 59 × 3 taşıma | 64 × 3 taşıma (60–63 hikâye listesi, 64 akış kuralı) | verify `conform` (bağımsız verify 18/18) |

## Değişen dökümler ve görseller

- **Var olan dökümler değişmedi.** Yeni dökümler: 60–63 (`stories-entitled`, `-unentitled`,
  `-200`, `-live`; M18 protokol eki), 64 (`feed-rule`; M19). Yeni şema örnekleri (`query.stories`,
  `client_msg.subscribe_stories`, geçersiz `query.stories_no_instrument`).
- `crates/qp-session/tests/capsule_perf_equality.rs` özetleri: `describe` cevabına hikâye sorgusu
  ve konusu eklendiği için yeniden kaydedildi (M18 protokol).
- **v1 altınlar değişmedi** (`fixtures/golden/**`). Gösterim varsayılanları R5 seçimlerine geçtiği
  için v1 demo çekimleri adreste R5 öncesini açıkça adlandırır:
  `?look=r4&voice=r4&face=vadrel&story=r4&open=r4&motion=r4` (S).
- **v2 altınlar (yeni):** 16 çekim, 28 sahne PNG'si (772 KB): `feed-*` (akışın ilk üç yüzü), `closed-*` (kapanmış iki yüz), `capsule-*` (`tesir`), `chart-*` (`pavka`) × TR/EN × koyu/açık; iki koşuda bayt eşit (`fixtures/golden/demo/v2/`; ayrıntı
  `rounds/R5/experiments/S/GOLDENS.md`).
- Değişen sınama kayıtları: `crates/qp-wasm/tests/data/pavka_scene_digests.txt` (`pavka` başarım eşitliği); `crates/qp-grammar/tests/story_pavka_equal.rs`'e `r4` bayt eşitlik sınaması eklendi (`pavka` ölçü düzeltmesi, `55f0884`). Dört `chart-*` v2 altını o düzeltmeyle üretildi; otoritenin grafik görünümü düzeltmesinden (`e447571`) sonra v2 altınları bayt eşit kaldı.
- Protokol varsayılanları değişmedi (Ö51): çıplak bağlantılar `r3`/`r4`; dökümler bayt eşit. Hesap
  özeti: `7b1e99d40889f83c` (değişmedi).

## Maliyet

| Ölçüm | Çağrı | $ | Kaynak |
|---|---:|---:|---|
| T2 tek bakış kuru koşu (σ ayarı dahil) | 372 | ≈ 1,16 | `harness/glance/DRYRUN.md` |
| T2 bozuk tasarım sınaması (Ek G2/G3) | 1 488 | 2,64 | `harness/glance/BROKEN.md` |
| T2 dil düzeneği kuru koşu (`dry2`) | 54 | 0,16 | `harness/lang/runs/dry2/RESULTS.md` |
| M17 dil (haiku tam + sonnet alt küme) | 240 koşu | 3,47 | `M17/RESULTS.md` §7 |
| M16 görsel dil | 6 720 | 17,02 | `M16/RESULTS.md` |
| M18 hikâye çizelgesi | 930 | 3,84 | `M18/RESULTS.md` |
| M18 `pavka` başarım eki | — (yeni okuma yok) | 0 | `M18/PAVKA-PERF.md` |
| M19 akış | — (deterministik benzetim) | 0 | `M19/RESULTS.md` |
| M20 açılmış kapsül | 1 042 | ≈ 1,93 | `M20/RESULTS.md` |
| M21 hareket | ≈ 6 490 | ≈ 13,4 | `M21/RESULTS.md` |
| D yoğunluk (ilk koşu 2,98 + tamamlama) | 768 | 4,09 | `D/RESULTS.md` (+ek) |
| **Toplam ölçüm** | | **≈ 47,7** | |

Tasarım, uygulama ve bütünleştirme alt ajanlarının oturum maliyeti bu tabloda yok (ölçülmedi).

## Açık sorular

1. **A41** — Yetkisiz okuyucuya lisanslı hikâyenin "rakamsız biçimi" mi gitmeli, yoksa yalnız
   sunucu yorumları mı (bugün Ö37 yolu: satır gitmez). R6 öncesi.
2. **A42** — Yetkisize dedektörün kamuya açık kuralı yazıyla ("beş günlük ortalamanın iki katı")
   verilebilir mi (K33 yorumu); bugün lisans koruması rakamı sözcüğe çeviriyor. İnsan testi v3.
3. **A43** — Hareket seçimi; düzenekte konuma dayalı süreklilik sorusu ve bozuk hareket. İnsan testi
   v3 / R8.
4. **A44** — `oltan`'da akışta canlıyı bulma 0,65 (n 23). İnsan testi v3.
5. **Ek G5 rol kümesi:** `valid` ve işaretin rolleri (`text`) anlam rengi çiftlerine girsin mi?
   İlk ikinin ikincisi (`zemre` ↔ `bertan`) buna bağlı.
6. **M17 §5.4:** "kısa olan" hangi adaylar arasından? SPEC'e tarihli ek önerisi.
7. **M20 `e_who`:** GOAL 8'in "sunucu ↔ cihaz ayrımı" sıkı mı okunmalı (`tesir`), gevşek mi
   (`nobra`)?
8. **Akış:** `grouped` gösterim varsayılanı olsun mu; K42 muafiyeti (Ö65).
9. **M18:** "son üç ayda ne oldu" görevi doğrudan ölçülmedi; grafik içi dokunuşlar (odak taşıma,
   kümeye yakınlaşma, kapanmışları gizle) sayılmadı (isabet kutuları kayda girmiyor).
10. **M16:** `face_visitor/numbers` sorusu her tasarımda şansa yakın (okuyucu yetkisiz yüzde de
    rakam gördüğünü söylüyor): soru metni mi, kabuk metni mi?
11. **M20:** değerleri gizli ama barları açık okur sentetik dünyada yok; A32'nin bu yarısı insan
    testinde ya da lisanslı bir sentetik olguyla ölçülmeli. `tesir`'in yeni bar anındaki uzun karesi
    yeniden ölçülmeli.
12. **D / R7:** seri paleti, açıklama kaybı, `chrome`/`rule` rolü, yoğun boy, mum yoğunluğu, son
    değer etiketi, koyu tema masaüstü (`D/RESULTS.md` §4).
13. Kısma kopyasında her tasarımda TR < EN: dil mi, model mi?

## Öneriler (kapsam dışı; yapılmadı)

- **Düzenek:** M21 için konuma dayalı süreklilik sorusu (ara karede hedefin işareti numaralı
  bölgede; son karede başka yerdeyken) ve bozuk hareket (yanlış yere uçan geçiş); bozuk kaydı bir
  aday tasarımın üstüne kurmak; açık sahnenin `SceneDesc`'ini (katmanlar, isabet kutuları) kayda
  almak (grafik içi dokunuş sayımı ve katmanlı karşıtlık için); fark edilmede canlılık sinyalini
  değişim sinyalinden ayırmak.
- **Dil:** S1 düzeltmesinin (ortalama dizilimi) okuyucuyla yeniden ölçümü; hacim olayında
  "alışılmadık alım"; `nobra` kalırsa cümle sırası "Bu cihazda sınandı; sunucu da doğruladı".
- **Yazı tipleri:** `canvas` açılışta gömülü bütün yazı tiplerini yüklüyor; yalnız seçili
  görünüşün dosyaları yüklensin. `₺` Atkinson Hyperlegible Next'te yok (bugün kullanılmıyor).
- **`surek`:** hareketi azalt etiketinin hareket kipinde de kısa süre kalması; JIT kapalıda değişim
  başı 50 ms (yalnız değişen kutuları çizmek).
- **`pavka`:** aynı girdide şerit yerleşimini önbelleğe almak (telefon JIT kapalı p99 27–28 ms).
- **Sözlük (CONCEPTS):** hikâye listesi; nabız, vuruş, iz, canlı noktası, değişim etiketi (seçimden
  sonra); "doğum" sınıflamasının ortak çekirdek kuralı.
- **M19:** sıralamada `trust` (0,760) ürüne alınmadı; gerekirse eklenebilir. Hikâye yorumlarında
  sayfalama (M18 protokol raporu).
- **`vadrel` + `oltan`:** S'de görünüş eşlik eden yüzünü getiriyor, ama `?look=oltan&face=vadrel`
  hâlâ mümkün ve o bileşimde geçersiz başlık `invalid` renginde (3,75:1).
- Gösterim paketinde sıyrılmamış WASM uyarısı (R4 sapması 9) sürüyor.

## Önerilen kararlar

`DECISIONS.md` "Önerilen (R5) — kesinleşmedi": Ö61–Ö70, açık A41–A44. Kesinleştirme ana akış
oturumunda yapılır.

## Disk (K56)

`<A:disk>` — round sonu `scripts/clean.sh` koşusu (önce/sonra), kalan `target/`,
`.claude/worktrees/`, depo toplamı. Round başında (T0): `target/` 1,9 G → 1,7 G, depo 4,1 G →
2,6 G (19 eski worktree kaldırıldı).

## Arkin için: galeriyi telefonumda nasıl açarım, neye bakarım

Süre: kurulum 2 dakika, galeri 10–15 dakika. `scripts/preflight.sh` gerekmez (gönderim yok).

### 1. Bilgisayarda başlat

```sh
scripts/demo.sh --gallery --lan --synthetic
```

- İlk koşu çekirdeği derler (bir dakika kadar); galeri açılışta adayları boyar (dil ve tema
  başına bir saniye kadar).
- Konsol iki adres yazar: `galeri / gallery: http://localhost:8390/gallery.html` ve `--lan` ile
  `http://192.168.x.y:8390/gallery.html`. Gösterimin kendisi aynı adreste `/` olarak çalışır.
- macOS "gelen bağlantılara izin verilsin mi" diye sorarsa **İzin Ver**.
- Galeri her zaman sentetik, duraklatılmış dünyayı kullanır; gerçek veri galeriye girmez.
  `--synthetic` gösterimin kendisini de sentetik açar (`--lan` açıkken aynı ağdaki cihazlar
  gösterimi görebilir).

### 2. Telefonda aç

1. Telefon **aynı Wi-Fi'de** olsun (misafir ağı değil, VPN kapalı).
2. Konsolun yazdığı `http://192.168.x.y:8390/gallery.html` adresini **IP ile, olduğu gibi** aç
   (başka ad 403 verir).
3. **iOS notu (K51, A40):** iOS düz `http://` sayfada JavaScript ve WASM'ı JIT'siz çalıştırır.
   Galerinin görüntüleri bilgisayarda çekirdek tarafından önceden boyanır, galeri bundan
   etkilenmez. Ama gösterimin kendisinde hareket ve kaydırma `http`'de daha ağır hissedilir. Ürünün
   gerçek hâli için HTTPS:

   ```sh
   scripts/demo.sh --gallery --synthetic --https-host <bilgisayar-adı>.<tailnet>.ts.net
   tailscale serve --bg http://127.0.0.1:8390      # ikinci terminalde
   ```

   Telefonda `https://<bilgisayar-adı>.<tailnet>.ts.net/gallery.html`. İş bitince
   `tailscale serve --https=443 off`. Ayrıntı `rounds/RP3/REPORT.md` §8B.

### 3. Galeride ne var

Konular alt alta; her konuda adaylar alt alta, hepsi **aynı içerikte** (aynı yeniden oynatma anı,
adım 120; aynı açık kapsül ve derinlik; konunun altındaki gri satır bunu yazar). Üstte tema (açık /
koyu) ve dil (TR / EN). Her adayın altında **Bunu seçerdim** (konu başına bir; ikinci dokunuş geri
alır) ve **Kısa not**. Aday sırası: ajan seçimi, ikinci, bugünkü (`r4`).

Her konuda bakılacaklar. Seçtiğinin altına bir cümle not yaz: neden seçtin, neyi beğenmedin.

- **Dil** (`marla`, `selvi`, `r4`; yüz + açılmış kart):
  - Başlığı okuyunca ne olduğunu, hangi eşiği ve kaç lirayı bir bakışta söyleyebiliyor musun?
  - "Ne zaman" ve "hâlâ geçerli mi" ilk okumada belli mi?
  - Karne cümlesi ("Benzer 20 durumun 11'inde …") neye ne kadar güveneceğini söylüyor mu?
  - Hiçbir cümle sana bir şey yapmanı söylüyor mu? (Söylememeli.)
- **Görünüş** (`oltan`, `zemre`, `r4`; yüz + açılmış kapsül + grafik):
  - Ekrana ilk baktığında gözün nereye gidiyor: işarete mi, cümleye mi?
  - Telefonu kol mesafesinde tut: yönü (yukarı/aşağı) ve bitmiş mi sürüyor mu, okumadan
    seçebiliyor musun?
  - Yazı rahat mı? Açık ve koyu temanın ikisini de dene.
  - Akışta **canlı** olan kapsülü hemen buluyor musun? (`oltan`'ın ajan ölçümündeki zayıf yanı, A44.)
  - `zemre`'nin alttaki ömür şeridi "hangisi daha yeni" sorusunu cevaplatıyor mu?
- **Hikâye çizelgesi** (`pavka`, `korda`, `r4`; grafik):
  - Bu hissede kaç olay olmuş, kaçı tutmuş, şu an canlı olan hangisi: kaç saniyede?
  - Fiyat yeterince görünüyor mu, yoksa kapsüller grafiği boğuyor mu?
- **Akış sırası** (`grouped`, `live_new`, bugünkü düzen; akışın ilk satırları):
  - Son baktığından beri ne değiştiğini ilk satırlarda görüyor musun?
  - Elindeki hissede olumsuz bir şey olsa ilk ekranda görür müydün?
- **Yazı tura eşiği** (`rate55`, `none`):
  - Karnesi yazı tura olanların gizlenmesi akışı daha güvenilir mi yapıyor, yoksa bir şey
    saklanıyormuş gibi mi hissettiriyor?
- **Açılmış kapsül** (`tesir`, `nobra`, `r4`; açılmış kapsül + kanıt):
  - Bu iddia neye dayanıyor? Bunu kim doğruladı: sunucu mu, bu cihaz mı?
  - Rakam göremediğin bir durumda nedenini anlar mıydın?
- **Hareket** (`surek`, `nabiz`, `r4`): galeri yalnız son kareyi gösterir; hareketi gösterimde dene
  (aşağıda 5). Notuna hangisinin "ne değişti"yi daha iyi söylediğini, hangisinin yorduğunu yaz.

### 4. Özeti gönder

En altta **Özet** ve **Özeti kopyala**. `https` ya da `localhost`'ta "Kopyalandı" der; düz `http`'de
pano izni yoktur, metin kutusu seçili gelir, paylaş/kopyala ile elle kopyala. Özet şöyle görünür:

```
R5 galeri · voice=… (not: …) · look=… · feed_order=… · feed_rule=…
notlar: …
an: adım 120 / … · tr · koyu
cihaz: iPhone iOS … · Safari · 402×874 @3 · tarih: …
```

Özeti bana (ana akış oturumuna) yapıştır. Özet yalnız seçimleri, notları, anı ve cihaz bilgisini
taşır; piyasa verisi taşımaz. Seçimler ve notlar bu tarayıcıda saklanır (yenileyince kaybolmaz).
Seçimleri paylaşmak için **seçimlerin bağlantısı** (`?pick=voice.marla,look.oltan,…`; notlar
girmez).

### 5. Gösterimde tek tek açmak (hareket ve dokunma için)

Aynı sunucuda gösterimi aç: `http://192.168.x.y:8390/` (ya da HTTPS adresi). Varsayılan R5'in
seçimleridir (`oltan` + yüz `oltan`, `marla`, `pavka`, `tesir`, `surek`, `rate55`). Ayarlar'dan ya
da adresle tek tek değiştir:

| Parametre | Değerler | Ne |
|---|---|---|
| `look` | `oltan`, `zemre`, `r4` | görünüş (eşlik eden yüzünü getirir; `r4` → `vadrel`) |
| `voice` | `marla`, `selvi`, `r4` | kapsül dili |
| `story` | `pavka`, `korda`, `r4` | hikâye çizelgesi (bir kapsüle iki kez dokun) |
| `open` | `tesir`, `nobra`, `r4` | açılmış kapsül (bir kez dokun; sonra **Kanıt**) |
| `motion` | `surek`, `nabiz`, `r4` | hareket; `&reduced_motion=1` hareketi azalt (eski `anim=0` da çalışır; bilgi aynı kalmalı) |
| `feed_order` | `grouped`, `live_new`, `layout` | akış sırası |
| `feed_rule` | `rate55`, `none` | yazı tura eşiği |
| `feed_exempt` | `0` | kuralı K42 muafiyetsiz çalıştırır (elindeki hissenin olumsuz geçişi de gizlenebilir) |

Örnekler:

```
http://192.168.x.y:8390/?look=zemre
http://192.168.x.y:8390/?voice=selvi&open=nobra
http://192.168.x.y:8390/?story=korda&motion=nabiz
http://192.168.x.y:8390/?feed_order=grouped&feed_rule=none
http://192.168.x.y:8390/?feed_exempt=0
http://192.168.x.y:8390/?look=r4&voice=r4&face=vadrel&story=r4&open=r4&motion=r4   # R5 öncesi
```

`&theme=light`, `&lang=en` eklenebilir. Hareket için oynatmayı başlat (▶, hız 8–16×) ve akışta bir
kapsülün doğmasını, güçlenmesini ya da kapanmasını izle; bir kapsülü açıp kapat.

İnsan testi v3 (5–10 kişi, ~20 dakika): `rounds/R5/human-test/GUIDE.md`; oturumlardan önce
`README.md` "Doldurulacak yer tutucular".
