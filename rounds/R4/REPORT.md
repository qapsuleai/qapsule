# R4 — Rapor: İlk Kullanılabilir Gösterim

- **Durum:** Tamamlandı. Bağımsız son denetim: **koşullu kabul**, engelleyici yok; dört koşul karşılandı ("Denetim" bölümü; Ö3 düzeltmesi denetçinin ek kontrolünde PASS). Kabul ölçütleri: **14 ✅ / 1 ⚠️** (6: kapsülün yüzü yeni, ayrıntı cümlesi R3 cümlesi kaldı; "Sapmalar" 1).
- **Tarih:** 2026-11-16
- **Alt ajanlar:** Başlatılabildi (ilk iş deneme alt ajanı koşuldu ve cevap verdi). Bütün alt
  görevler, 9 alternatif (M8 3, M9 2, M10 2, M11 2), dört düzenek/değerlendirme, evre 1
  doğrulaması, düzeltmeler, bütünleştirme, cihaz ölçümü, insan testi paketi, ajan yazabilirlik ve
  son denetim ayrı alt ajanlarla yapıldı. Orkestratör M8–M11 şartnamesini (SPEC, ekler E1–E7),
  görev tanımlarını, seçim kurallarının uygulanmasını, birleştirmeleri, `pagecmp` toleransını,
  `docs/FACE.md`, DECISIONS "Önerilen (R4)" bölümünü ve bu raporu yazdı.
- **Dallar:** bütünleştirme `r4/int`; alt görev dalları `r4/t2-replay`, `r4/t3-k42`,
  `r4/t4-input`, `r4/t5-shell`, `r4/t6-startup`, `r4/verify-e1`, `r4/fix-e1`, `r4/h-m11`,
  `r4/h-m8910`, `r4/h-fp`, `r4/m8-a`, `r4/m8-b`, `r4/m8-c`, `r4/m9-a`, `r4/m9-b`, `r4/m10-a`,
  `r4/m10-b`, `r4/m11-x`, `r4/m11-y`, değerlendirme `r4/eval-m8`, `r4/eval-m9`, `r4/eval-m10`,
  `r4/eval-m11`, `r4/k42-feed`, bütünleştirme `r4/s1`, `r4/s2`, `r4/d`, `r4/ht`, `r4/e-ergo`,
  denetim `r4/audit`, denetim düzeltmesi `r4/fix-replay`. Hepsi `--no-ff` ile `r4/int`'e.
- **Gerçek veri:** Var, yalnız yerelde (`fixtures/local/`, depoda yok). 31 sembolde günlük ve
  saatlik barlar (`symbols.txt` 29 satır; yeniden oynatma ilk 30 seriyi kullanır). Raporda
  yalnız toplu sayılar. Gösterim yerel veri yoksa sentetik pazarla açılır ve bunu ekranda söyler.

## Ne yapıldı

| # | Görev | Yürüten | Doğrulayan | Çıktı |
|---|-------|---------|------------|-------|
| — | Bekleyen doküman değişiklikleri; deneme alt ajanı | orkestratör | — | `2ebf51f` (round öncesi); alt ajan cevap verdi |
| T1 | Sözlük (önce), M8–M11 şartnamesi (alternatiflerden önce) | orkestratör | verify-e1, denetim | `docs/CONCEPTS.md` "Gösterim (R4)", `experiments/harness/SPEC.md` (`bad3d67`, E1–E5 `5b327e5`, E6–E7 okumalardan önce) |
| T2 | Yeniden oynatma kaynağı ve denetimi; `qp replay`; PROTOCOL §21 | alt ajan | verify-e1 | `experiments/T2.md` |
| T3 | K42 (karne yönü, olumsuz geçiş, geçerlilik), süre ifadesi, R3 ergonomi önerileri; hassasiyet benzetimi | alt ajan | verify-e1 | `experiments/T3.md`, `T3-sensitivity.md` |
| T4 | Etkileşim: piksel hareketleri, bakış durumu, isabet, açılma, kanıta inme | alt ajan | verify-e1 | `experiments/T4.md` |
| V1 | Evre 1 bağımsız doğrulama | denetçi alt ajan | — | `experiments/VERIFY-E1.md` (B1 takvim FAIL, B2/B3 bulgu) |
| FIX-E1 | B1 XIST takvimi 2018–2027, B2 kapanmış kapsülün cümlesi | alt ajan | verify, denetim | `experiments/FIX-E1.md` |
| T5 | Kabuk: `qp demo`/`qp drive`, `ShellCore` (WASM), web kabuğu, mimari testi, eşitlik/ağ testi, ekran düzeneği, ölçüm sekmesi, `scripts/demo.sh` | alt ajan | verify 17–18, denetim | `experiments/T5.md`, `shells/web/README.md`, `ARCH.md` |
| T6 | Açılış süresi ve paket boyutu | alt ajan | denetim | `experiments/T6.md` |
| H | M11 düzeneği; M8/M9/M10 düzenekleri ve kayıt arayüzü; yüz başlık şablonu arayüzü (H-FP) | 3 alt ajan | yeniden üretim | `experiments/harness/`, `H-M8910.md`, `H-FP.md` |
| M8–M11 | 9 alternatif, ayrı dallar, ayrı alt ajanlar, birbirini görmeden | 9 alt ajan | körlemeli persona + eleştirmen ajanları; değerlendirme ajanları | `experiments/M8..M11/` |
| K42-FEED | Akış sınırının pozisyonun olumsuz geçişini kesmesi (bulundu, düzeltildi) | alt ajan | verify | `experiments/K42-FEED.md` |
| S1, S2 | Seçimlerin bütünleştirilmesi (gösterim varsayılanları), yeni altınlar, Ayarlar testi, son bakış, ergonomi gerilemesi (C11) düzeltmesi | 2 alt ajan | verify, denetim | `experiments/S1.md`, `S2.md`, `docs/FACE.md` |
| D | Cihaz ölçümü (masaüstü, 4×/6× kısıt), ölçüm ekranının tamamlanması, telefon yönergesi, K17/K45 önerileri | alt ajan | denetim | `DEVICE.md`, `experiments/D.md` |
| HT | İnsan testi v2 paketi | alt ajan | denetim | `human-test/` |
| E | Ajan yazabilirlik (C01–C20, iki koşu) | alt ajan | denetim | `ergonomics/RESULTS.md` |
| A | Son denetim | bağımsız alt ajan | — | `AUDIT.md` |

## Kabul ölçütleri

| # | Ölçüt | Sonuç | Kanıt |
|---|-------|-------|-------|
| 1 | Yeniden oynatma: yerel ya da sentetik, alan düzeyinde veri olayları; hız, duraklat, adım, tarihe git; saat barındırıcıdan; iki koşu bayt eşit; otorite + yansı | ✅ | `qp-source::replay`, `qp-session::replayhost`, PROTOCOL §21, dökümler 49–50; verify adım 16 `replay` (iki koşu bayt eşit, 1816 satır; geri seek sonrası 1120 bildirim — denetim Ö3 düzeltmesinden sonra); VERIFY-E1 1a–1g (kendi dizisi 257 satır eşit; hız 1/37/900 aynı içerik; geri seek = baştan kurma; 4 adım geride yansı, fark 0; yerel gerçek veride iki koşu eşit) |
| 2 | `scripts/demo.sh` tek komut; sentetikte ekranda söyler; dış ağa istek yok (testle); telefondan açılır, ekranda ve belgede | ✅ | `scripts/demo.sh`; "Sentetik veri" bandı; `parity.mjs` bütün istekleri kaydeder (yalnız sayfanın kökeni; verify 18); `--lan` + konsol/Ayarlar adresi; `shells/web/README.md`, `DEVICE.md` §4; güvenlik: bağlantı başına gizli değer, `Origin`/`Host` denetimi (`demo_security.rs`) |
| 3 | Telefon boyunda akış, açılma, grafikte kaydırma/yakınlaştırma, kanıt (yetkili/yetkisiz), karne, seviye, izleme/pozisyon, TR/EN, açık/koyu | ✅ (notlarla: Ö4, Ö5) | `shells/web/` (390×844); `parity.mjs` ve `settings.mjs` (20/20) kullanıcı gibi tıklıyor; denetimde elle kullanım (`AUDIT.md` VERIFY 6) |
| 4 | Kabuk ince (mimari testi); kabuksuz sürüşte aynı doküman ve sahne özetleri | ✅ | `arch-check.mjs` R1–R6, istisna listesi boş, kendini sınama 14/14; verify 17 `shell`; `qp drive` = tarayıcı 7 anda özet eşit (verify 18 "browser = shell-less digests"); S2 takvim düzeltmesinden sonra 40/40 yüz eşit |
| 5 | Dokunma/kaydırma/yakınlaştırma `input` mesajı; bakış durumu günlüğü şişirmiyor; hareket sırasında kare süresi | ✅ | PROTOCOL §4.6.1 (`tap_at`, `drag`, `pinch`), dökümler 51–56; 1000–1500 girdi sonrası günlük ve doküman özeti aynı (`interaction.rs`, VERIFY-E1 3b); ölçüm ekranı kaydırma/sürükleme/yakınlaştırma kare süreleri (`DEVICE.md`) |
| 6 | Yüz M8 ile seçildi; sınırlara uyuyor; şablon dilbilgisi, iki dil, takvime göre süre; FACE.md; yeni altınlar | ⚠️ **kısmi** — yüz ✅ (ön eleme; son söz insan testinde); **ayrıntı cümlesi R3 cümlesi kaldı** (bar sayısı, iç terim; "Sapmalar" 1) | `docs/FACE.md`; `M8/RESULTS.md`; `vadrel` başlık TR ≤ 75 / EN ≤ 77, ek bilgi 1, en küçük yazı 16 px, metin karşıtlığı 5,78/5,27; birim testi bütün olay × aşama × en uzun zaman ifadesi ≤ 80; XIST takvimi 2018–2027 (FIX-E1 B1, 107 tatil + 22 yarım gün, gerçek veriyle birebir); altınlar `feed-*-r4.*`, `feed-tr-dark-fotolay.*`. Açık: kapanmış işaret karşıtlığı 2,06 (A31) |
| 7 | K42: karne yönü; yalnız olumsuz geçişler seviyeden bağımsız; geçerli olmayan kapsül işarette de; durum koşulu/beklenen çözüm ayrı; değişenler listeli; hassasiyet yeniden | ✅ | `T3.md` §1–2; "Değişen dökümler ve görseller" (bu rapor); `T3-sensitivity.md` (belirleyici, `important` doğumlarında pozisyon/niyet payı %57,3 → %0 sentetik, %71,0 → %0 gerçek); VERIFY-E1 2a–2d; K42-FEED (akış sınırı pozisyonun olumsuz geçişini kesmiyor) |
| 8 | M8–M11: düzenek ve ölçüt önce, alternatifler, ham kayıtlar, seçim ve gerekçe; ajanların körlüğü kayıtlardan anlaşılır | ✅ | SPEC `bad3d67` (12:04) ve M11 düzeneği `2db6d25` < alternatif birleştirmeleri (15:47–); `experiments/M*/RESULTS.md`, `raw/`; okuyucular karma kodlu görüntülerle, `check-prompt` istemde tasarım adı olmadığını denetler; SPEC E6/E7 okumalardan önce ve gerekçeli |
| 9 | Ölçüm ekranı (kare durağan/kaydırma, tik, açılış, paket; tek dokunuşla özet); masaüstü ve kısıtlı; telefon yönergesi; K17/K45 önerisi | ✅ (telefon ölçümü Arkin'de) | Ölçüm sekmesi (T5 + D: yakınlaştırma, p99, uzun kare, bellek, cihaz, Sıfırla); `device.mjs` 72 koşu (1×/4×/6×, DPR 1/3, iki görünüm, sentetik + gerçek); `DEVICE.md` §4 yönerge, §5 K17, §6 K45 |
| 10 | Bağlanma 30 enstrüman < 100 ms; ilk kapsüle kadar süre; WASM boyutu; yansılı paketin ayrı yüklenmesi | ✅ | `caps` 140,5 → **85,7 ms** (`T6.md`; `flow` 118,8 ms, nedeni yazılı); ilk kapsül 121 ms masaüstü, 460 ms 4×; WASM 4,38 MB / gzip 1,29 MB; ayrı yükleme değerlendirmesi `T6.md` §3, `DEVICE.md` §7 |
| 11 | Ekran görüntüsü testleri belirleyici, sentetik, depoda; gerçek veriyle alınanlar depoda değil | ✅ | `screens.mjs` (sabit adım 120, animasyon kapalı); verify 18: iki koşu, sahneler bayt eşit ve altınlarla eşit, sayfalar `pagecmp` toleransında; `--data` ile çıktı yalnız `fixtures/local/` (betik reddeder); `git ls-files | grep fixtures/local` boş |
| 12 | İnsan testi v2: görevler (~20 dk, 5–10 kişi), gözlem formu, puanlama; alfabe ve yüz ilk ikisi ayardan; A21 ve yüzün nasıl kapanacağı | ✅ | `human-test/` (GUIDE, FORM, plan.json Y1 `vadrel` / Y2 `fotolay`, key, score.py, DECISION.md önceden yazılı kural); Ayarlar ve `?face=`/`?alphabet=` (`settings.mjs`) |
| 13 | R3 ergonomi önerileri; ajan yazabilirlik yeniden ölçüldü | ✅ | `capsule` seçici, `capsules` `latest`/`born_since_*`, seçicide `stage`/`open`, PROTOCOL §20.3 kitle (dökümler 44, 48, 57, 58); E: ilk deneme 12/20 → **14/20, 17/20**; ölçümde bulunan C11 gerilemesi (yetkisiz seçici sessizce eski hikâyeye) S1'de `withheld` ipucuyla düzeltildi (döküm 57; yeniden ölçülmedi) |
| 14 | Dökümler üç taşımada; yeni dökümler (yeniden oynatma, girdi, açılma) | ✅ | 58 × inproc/serve/MCP (verify `conform`); yeni 45–58 |
| 15 | REPORT, AUDIT, STATUS, ROADMAP, FACE.md, önerilen kararlar ayrı bölümde | ✅ | bu dosya; `AUDIT.md`; `STATUS.md`, `ROADMAP.md`; `docs/FACE.md`; `DECISIONS.md` "Önerilen (R4) — kesinleşmedi" (Ö39–Ö51, A31–A33) |

## Ölçümler (özet)

| Ölçü | R3 | R4 | Kaynak |
|---|---:|---:|---|
| Bağlanma, 30 enstrüman (4 hello + 6 abone), `caps` | 133 ms (aynı makinede yeniden: 140,5) | **85,7 ms** | `T6.md` §1 |
| Bağlanma, `flow` (lisanslı akışlı) | 247 ms | 118,8 ms (> 100; neden `T6.md`) | `T6.md` |
| Gösterim: ilk kapsül, masaüstü / 4× / 6× (DPR 3) | — | 121 / 460 / 639 ms (gerçek veri 98 / 384 / 573) | `DEVICE.md` |
| Akış kaydırma kare p95, 1× / 4× / 6× | — | 9,9 / 10,2 / 16,7 ms (uzun kare %0 / %2 / %5) | `DEVICE.md` |
| Tik p95 (16 adım/sn), 1× / 4× / 6× | — | 11,3 / 35,9 / 54,5 ms | `DEVICE.md` |
| WASM (gösterim paketi) | 595 KB gzip (R2) | ham 4,38 MB, gzip 1,29 MB; gösterim sunucusu sıkıştırmıyor | `DEVICE.md`, `T6.md` §3 |
| Bellek | — | JS yığını 3 MB, WASM 26 MB | `DEVICE.md` |
| Ajan yazabilirlik, ilk deneme | 12/20, 11/20 | **14/20, 17/20** | `ergonomics/RESULTS.md` |
| `important` doğumlarında pozisyon/niyet payı | sentetik %57,3; gerçek %71,0 | **%0 / %0** | `T3-sensitivity.md` |
| Uygunluk dökümleri | 44 × 3 taşıma | **58 × 3 taşıma** | verify `conform` |
| M8 persona tam doğru (ağırlıklı), seçilen yüz | R3 kartı 0,141 | `vadrel` 0,768 | `M8/RESULTS.md` |
| Başlık uzunluğu TR en çok | R3 kartı 205 | `vadrel` 75 (≤ 80) | `docs/FACE.md` |

## Seçimler (ön eleme; son söz insan testi v2'de)

> **Persona ajanı insan değildir.** Persona ve eleştirmen ajanları küçük pikseli, kısa bakışı ve
> hareketi bir insan gibi deneyimlemez; görüntüleri büyüterek ve istediği kadar uzun okur. Bu
> seçimler bir ön elemedir. Son söz insan testi v2'dedir (`human-test/DECISION.md`: insan sonucu
> ajan sonucundan farklıysa insan sonucu geçerlidir).

| Deney | Seçim | Ölçü | İkinci (insan testinde) | Not |
|---|---|---|---|---|
| M8 yüz | **`vadrel`** (cümle önde, 188 px) | persona ağırlıklı 0,768 vs `fotolay` 0,727 (fark 0,040 < 0,05) → eleştirmen sırası 1,67 vs 2,67 | `fotolay` | `tavrik` 0,596 elendi; `r3` 0,141 başvuru. `docs/FACE.md` |
| M9 akış | **`elikov`** | ağırlıklı 0,667 vs `vakiro` 0,481 (`r3` 0) | `vakiro` | G3 ("en güvenilir") ayırdı |
| M10 geçiş | **`zumlak`** (zayıf) | G1 eşit 1,00; G2+G3 0,694 vs 0,667 (tek duruma dayanıyor) | `tutamir` | Test geçişi gerçekten ölçmüyor (tavan); ikisi de insan testine |
| M11 sonuç işareti | **`tg5`** | açıklamasız doğruluk 1,00 vs `muhrak` 0,10; ΔE ×14 (`r3`'e göre) | `r3` | `r3` okumada da 1,00 (ajan küçük pikseli büyütüp okuyor) |

Seçim kuralları ve eşikler alternatiflerden önce commit edildi (SPEC `bad3d67`, 12:04; ilk
alternatif dalları 15:00'ten sonra birleşti). Okumalar başlamadan iki ek yapıldı ve gerekçesiyle
yazıldı: **E6** (sert eşik eşik başına; hiçbir alternatifin tasarımdan bağımsız bir nedenle
geçemediği eşik uygulanmaz — kapanmış işaret karşıtlığı alfabe düzeyinde 2,06) ve **E7** (#9
yetkisiz okuyucu başlık uzunluğundan hariç; düz metin kabuk ürün kuralı). Eksiz uygulansaydı
bütün yüzler elenirdi.

Protokol varsayılanları (`r3`) değişmedi: ayar göndermeyen bağlantılar, dökümler ve var olan
altınlar bayt eşit. Gösterim açılınca kabuk çekirdeği `hello`'dan sonra
`style {face: vadrel, feed: elikov, transition: zumlak, outcome: tg5}` ister (Ö51).

## Değişen dökümler ve görseller (gerekçeli)

Bütün değişiklikler K42 gereği; yaprak düzeyinde farklar `experiments/T3/diffs/`.

| Dosya | Değiştiren | Neden |
|---|---|---|
| dökümler 38, 39, 40, 41, 42, 43, 44 | T3 (`ed1f6f9`) | A23 karne yönü (anahtar `<tür>/<yön>/…` → karne, önem, sıra, karne yan cümlesi), A28 `validity`/`condition`/`resolution`; 41'de pozisyonlu okuyucuya giden kademe 3 doğum bildirimi kalktı (A22) |
| dökümler 38, 41, 42, 44 (ve R4'te eklenen 48, 51–55) | FIX-E1 (`8036d98`) | Kapanmış kapsülün cümlesinden "Hâlâ geçerli." / "…bilinmiyor" / "Artık geçerli değil" yan cümlesi silindi (işaret `closed` derken cümle "geçerli" diyordu); yalnız silme + başlık kutusunun bir satır kısalması |
| `fixtures/views/docs/r3-breakout.json`, `r3-divergence.json`, `r3-squeeze.json`, `fixtures/views/c1-sentence.json` | T3 | Yönlü karne anahtarı ve karne (yukarı kırılımın önceki örneği yok → "henüz karnesi yok"), `condition`/`resolution` |
| `fixtures/golden/views/c2-capsule.png`, `c3-chart.png` | T3 | İşaret gri + "artık geçerli değil" rozeti (R3'te cümle "artık geçerli değil" derken işaret geçerli görünüyordu — K42'nin düzelttiği çelişki); karne satırı |
| `crates/qp-session/tests/capsule_perf_equality.rs` özetleri | T3, FIX-E1 | Yukarıdaki çıktı değişikliklerinin özetleri (eski değerler yorumda) |
| `crates/qp-capsule/tests/fixtures/m2_sentences.json` `product_changes` | FIX-E1 | R3 şablonuyla belgelenmiş farklar 20 → 22 |

Değişmeyenler: dökümler 01–37; `fixtures/golden` altındaki R3 ve öncesi bütün altınlar (`c4`,
`c5`, `d1`, `d2`, `v0`–`v5`, `g1-glyphs-*`); `explain` dosyaları; hesap özeti
(`7b1e99d40889f83c`, arm64 = x86_64 = wasm32).

Yeni altınlar (yeni görünümler): `views/g2-glyphs-validity-{light,dark}.png` (T3);
`fixtures/golden/demo/` 18 sahne görüntüsü (T5'in 7 R3 tasarımlı çekimi, S1'in `-r4` ve
`-closed` çekimleri, S2'nin `feed-en-light-r4` ve `feed-tr-dark-fotolay` çekimleri). R4 içinde
yeniden üretilen tek gösterim altını `feed-tr-dark.face-2.png` (FIX-E1 B2 cümlesi; R4'te
eklenmişti). S2'de `-r4` çekimleri yeni varsayılan yüzle yeniden üretildi (aynı ad: "gösterim
bugün nasıl açılıyorsa"; `main`'de hiç yoktu).

Yeni dökümler 45–58: K42 (45–47), seçici (48, 57), yeniden oynatma (49, 50), girdi ve açılma
(51–56, 58).

**Doğrulama toleransı (yeni):** verify adım 18'de sayfa ekran görüntüleri (`*.page.png`) artık
`shells/web/tests/pagecmp.mjs` ile karşılaştırılıyor: en çok 64 piksel, kanal başına en çok 2
seviye fark kabul edilir. Neden: başsız Chromium makine yükü altında DOM denetimlerinin kenarını
arada bir farklı yumuşatıyor (ölçülen: "Git" düğmesinin çerçevesinde 14 piksel, ±1–2 seviye;
release ikilisiyle de yeniden üretildi). Çekirdeğin boyadığı sahne görüntüleri hâlâ bayt eşit
ve altınlarla karşılaştırılıyor; tolerans yalnız tarayıcının kendi çizimine uygulanıyor.

## Ne yapılamadı / sapmalar

1. **Ayrıntı cümlesi R4 dil kurallarına uymuyor (GOAL 6, denetim Ö1/Ö2).** Kapsülün yüzündeki
   başlık cümlesi yeni şablondan (`vadrel`, takvime göre süre, iç terimsiz); ama kapsül
   derinliğindeki ayrıntı cümlesi R3'ün ürün cümlesi kaldı: süre bar sayısıyla ("85 bar önce",
   "Bu barda"), iç terimler ("aynı aileden", "Sinyal"), aynı cümle üstte ve sahnede iki kez.
   Bu yüzden aynı kapsül için yüz ile ayrıntı farklı söyleyebiliyor (`cap.SYN26.breakout.down.47`:
   yüz "Kapandı: 10 Mart'ta tutmadı.", ayrıntı "artık geçerli değil"; `cap.SYN12.breakout.down.129`:
   yüz "dün güçlendi", ayrıntı "Bu barda güçlendi"). **Neden yapılmadı:** ayrıntı cümlesi
   (`qp_capsule::sentence`) protokolün kapsül cümlesidir; dökümler 38–58'de ve var olan
   altınlarda (`c2`, `c3`, gösterimin R3 çekimleri) yer alır. Değiştirmek K42 dışı nedenle var olan
   dökümleri ve altınları değiştirirdi (GOAL değişmezi). Görev planlaması bunu yüze odaklayıp
   ayrıntıyı ayırmadı; denetimde fark edildi. **Öneri (R5):** ayrıntı cümlesini aynı yapı
   taşlarından (`HeadlineParts`, süre ifadesi, geçerlilik sözü) yeni bir sürümle kurmak; eski cümle
   protokol varsayılanı olarak kalır, gösterim yeni sürümü ister (yüz için yapılan `style`
   yolu). Ölçüt 6 bu yüzden kısmi.
2. **Hassasiyet seçimi kitlesiz gösterimde görünmüyor (denetim Ö4).** Kitle (izleme listesi,
   pozisyon) verilmeden üç seviye de 40 kapsül gösteriyor: akışın sayfa sınırı 40 ve sentetik
   pazarın 30 enstrümanında her seviyede 40'tan çok kapsül var. Kitle verilince 27 / 38 / 40.
   Hesap doğru (benzetim belirleyici, `important` 906 doğum / 500 gün); gösterimde görünür kılmak
   (ör. sayfa sınırını seviyeye göre ya da "son bakıştan beri" ile kesmek) açık soru.
3. **Ayarlar'dan seçilen yüz/alfabe/son bakış dil ya da yetki kipi değişince kayboluyor (Ö5).**
   Kabuk yeniden bağlanırken yalnız URL ayarlarını yeniden uyguluyor. İnsan testi bağlantıları
   ayarları URL'le verdiği için testi etkilemez. R5'e.
4. **`qp replay` geri sarmadan sonra susuyordu (Ö3).** Geri seek bağlantıları kapatıyor (§21, yeni otorite); kayıt aracı yeniden `hello` demiyordu, sonraki denetimler `handshake` hatası aldı ve verify adım 16'nın "iki koşu bayt eşit" karşılaştırması geri sarmadan sonraki 180 adımda boş akışı karşılaştırıyordu. Gösterim (tarayıcı) etkilenmiyordu. **Düzeltildi** (`r4/fix-replay`, `experiments/FIX-REPLAY.md`): araç `reset: true` görünce aynı `hello` ve abonelikleri yeniden gönderir; test `crates/qp-cli/tests/replay.rs` (eski kodla düşüyor); adım 16 artık hata satırı olmamasını ve son geri seek'ten sonra `welcome` + bildirim gelmesini de denetliyor: 693 satır (geri seek sonrası 0) → 1816 satır (geri seek sonrası 1120 bildirim), iki koşu bayt eşit. Dökümler, altınlar, hesap özeti değişmedi.
5. **Ajan yazabilirlik C11 düzeltmesinden sonra yeniden ölçülmedi.** E'nin bulduğu gerileme
   (yetkisiz okuyucu seçicisi sessizce eski hikâyeye) S1'de `withheld` ipucuyla düzeltildi
   (döküm 57); C01–C20 yeniden koşulmadı.
6. **Telefonda ölçülmedi.** Cihaz ölçümü masaüstü ve kısıtlanmış işlemci benzetimiyle; telefon
   ölçümü Arkin'de (yönerge aşağıda). K17/K45 önerileri o ölçüme kadar geçici.
7. **İnsan testi v2 koşulmadı** (paket hazır; katılımcı gerektirir). A21 ve yüz seçimi açık.
8. **SPEC E6 zamanlaması:** E6 eki iki M8 alternatifi teslim edildikten sonra ama bütün
   okumalardan önce yazıldı (denetim VERIFY 14 notu). Eşik alfabe düzeyinde (bütün tasarımlarda
   aynı 2,06) olduğu için hiçbir alternatifi kayırmıyor; gerekçe SPEC §8'de.
9. **Gösterim paketi:** verify adım 18 adları soyulmamış WASM paketini (5,6 MB) `shells/web/pkg`'a
   yazar; ardından `scripts/demo.sh` güncel sanıp yeniden derlemez. Ölçüm ekranının sayıları
   `DEVICE.md` ile karşılaştırılacaksa `QP_DEMO_REBUILD=1 scripts/demo.sh` ile açılmalı.

## Denetim

Bağımsız son denetim (`AUDIT.md`, dal `r4/audit` `ab07c85`): **koşullu kabul**, engelleyici bulgu
yok. Denetçi gösterimi temiz bir ağaçta sentetikle açıp 390×844'te kullanıcı gibi kullandı
(`audit/shots/01…17`); kabuksuz sürüşü `qp drive` yerine kendi WASM'sız HTTP istemcisiyle yaptı
(tarayıcının 33 POST'u → 716/716 çerçeve, belirteç dışında bayt eşit); ağ 71 istek, başka kökene 0;
`main.js`'i satır satır okudu (hesap, sıralama, metin kurma yok); süre ifadesinde kendi 21
örneğinin 21'i doğru; bağlanma `caps` 82,6 ms; `device.mjs` sayıları `DEVICE.md` ile ±%20 içinde
(tek aşım 4× DPR1 durağan p99 +%35); M8'den üç persona okumasını taze ajanlarla tekrarladı
(`vadrel` 3/6, `fotolay` 2/6, `tavrik` 2/6: sıralama tutarlı); hijyen temiz.

VERIFY: 1, 3–20 PASS (6, 7, 11, 12, 14, 15 bulgu ya da notla); 2 (preflight, CI) denetçiye `gh`
yasak olduğu için ana akışta koşuldu (aşağıda). GOAL hükmü: 1–5, 7–14 ✅; 6 kısmi; 15 o anda
eksikti (rapor taslaktı).

Koşullar ve karşılıkları:

| Koşul | Karşılık |
|---|---|
| 1. Ayrıntı cümlesi düzeltilsin ya da sapma olarak yazılsın | Sapma olarak yazıldı ("Sapmalar" 1); ölçüt 6 ⚠️ kısmi; R5 önerisi |
| 2. REPORT commit edilsin, STATUS/ROADMAP güncellensin | Yapıldı (bu commit) |
| 3. Ö3 REPORT'a girsin | Düzeltildi (`r4/fix-replay`); denetçi ek kontrolde doğruladı: kendi dizisinde iki koşu bayt eşit, 1628 satır, hata 0; adım 16 eski çıktıyı reddediyor (`AUDIT.md` "Ek: Ö3 düzeltmesinin kontrolü", `f45e363`) |
| 4. Preflight ve CI ana akışta | `scripts/preflight.sh` TAMAM (origin, ssh, `qgh` hesabı, yazma izni, commit e-postası); `main`'e birleştirilip gönderildi, CI sonucu `STATUS.md`'de |

Öteki bulgular: Ö4, Ö5 ("Sapmalar" 2, 3); Ö6 tutmadan kapanan işaretin gövdesi 1,1–1,4:1 (✗
köşesi 3,8 / 4,5 eşiği geçiyor) → A31'e eklendi; küçükler "Öneriler"de.

## Açık sorular

1. **Kapanmış işaretin karşıtlığı 2,06 (< 3, WCAG 1.4.11).** Alfabe kuralı (gri + tazeliğe göre
   saydamlık), bütün yüzlerde aynı; düzeltmek `g1` altınlarını değiştirir, R4'te yapılmadı (A31).
2. **Yetkisiz okuyucunun yüzü (#9):** düz metin zaman, geçerlilik ve karne vermiyor (her
   tasarımda 0/6). Şablonlu, rakamsız bir yüz önerilir (A32).
3. **"Bugün" mü "4 saat önce" mi:** M8 anahtarı 3 saatten eskiyi "bugün" sayıyor, ürün "4 saat
   önce" yazıyor (okuyucuların 23/24'ü "birkaç saat önce" dedi). Süre ifadesi kuralı insan
   testinde gözlenmeli.
4. **Takvim günü mü işlem günü mü:** "ne zaman" anahtarı ve ürün cümlesi ("3 işlem günü önce")
   insan için hangisi doğal.
5. **`important` seviyesinde boş gün payı** sentetikte 0,50–0,63: geleceğin yatırımcısı için
   akış fazla sessiz olabilir (T3).
6. **Geri sarmada kitle:** geri `seek` yeni otorite kurar; kitle (izleme listesi, pozisyon)
   kabuk tarafından URL'den yeniden uygulanıyor. Kitlenin oturum durumu olarak devri açık (T2).
7. **M10 ölçüsü geçişi ölçmüyor:** grafik ekranı her tasarımda aynı, okuyucu son ekrandan
   cevaplıyor. Öneri: soru geçişin ortasındaki karede sorulsun.
8. **K17 eşlemesi:** 4×/6× kısıt ile orta sınıf/taban telefon eşlemesi varsayım; Arkin'in
   telefon ölçümü doğrulamalı (`DEVICE.md`).
9. **Sentetik pazar tekdüze:** gösterimdeki kapsüllerin çoğu aşağı kırılım; insan testi ve M9 G1
   yön/tür ayrımını zayıf ölçüyor.
10. **Akışın zaman ufku:** öneme göre düzen 5 aya yayılan kapsülleri yan yana koyuyor; "son
    bakıştan beri" ile sınırlamak açık (K42-FEED).
11. **2027 XIST takvimi geçici;** Borsa İstanbul ilanından sonra (aralık) kontrol edilmeli.

## Öneriler (kapsam dışı; yapılmadı)

- **Ayrıntı cümlesinin yeni sürümü** (Sapmalar 1): yüzün yapı taşlarıyla, takvime göre süre, iç
  terimsiz; kapsül derinliğinde cümle bir kez.
- Kanıt ekranı ham: TR arayüzde İngilizce iç anahtarlar ve biçimsiz sayılar (`vol_avg
  1475274787.72`); cümlede ondalık virgül, grafikte ve kanıtta nokta (denetim).
- Yetkisiz okuyucunun akışında 40 satırın hepsi aynı cümle; EN'de "for This stock" büyük harf
  hatası (denetim).
- Kapsül derinliğindeki küçük grafik olayın anını göstermeyebiliyor (olay pencerenin dışında).
- Akışın başı aylar önce kapanmış kapsüllerle doluyor (zaman ufku, açık soru 10).

- **Sunucu sıkıştırması:** gösterim barındırıcısı önceden sıkıştırılmış `.wasm.gz` verirse
  telefona giden paket 4,4 MB → 1,29 MB (`D.md`).
- **Değişmeyen yüzü yeniden boyamamak** ve hareket sırasında 2× ölçekte boyamak: 4× kısıtta tik
  p95 35,9 ms (orta sınıf öneri sınırı 33 ms); uzun karelerin kaynağı tikte yüzlerin yeniden
  boyanması (`DEVICE.md`). GPU'dan (K45) önce denenmeli.
- `HeadlineParts`'a olay türü alanı (yüz şablonu bugün ürünün ad öbeği metnine bağlı, kırılgan).
- Grameri "çıplak panel" bayrağı (eksensiz küçük resim; `fotolay` tam kurulamadı).
- Gösterimde yüz yüksekliği açılışta 240 → 188 px atlıyor (varsayılan `style` `hello`'dan sonra
  geliyor); ilk çerçeveyi sunulan yüzle istemek.
- `flow` bağlanması 118,8 ms: kamusal okuyucunun rakamsız cümleleri, belge kopyası, izdüşüm
  (`T6.md`).
- `?seen=<step>` geri sarmada yeni otoriteyle birlikte yeniden uygulanıyor; adımla verilen son
  bakışın anlamı belgelenmeli.
- M9 düzeneğinde sabit port (8471); boş port seçimi. M9 G3 eşiği n ≥ 5, ürünün eşiği 20.
- Kapanmış `invalidated`/`expired`/`retracted` notlarında "; artık geçerli değil" eki
  (`no_longer` işaretinin sözü; çelişik değil ama sade değil).
- `XNYS` takvimi hâlâ R1 örnek tatilleri; NYSE verisi gelirse aynı yöntemle.
- İnsan testi README'sindeki "denenen ayarlar" tablosu S1 öncesi; v2 koşulmadan önce yeniden
  denensin.

## Önerilen kararlar

`DECISIONS.md` "Önerilen (R4) — kesinleşmedi": Ö39–Ö51, A31–A33. Kesinleştirme ana akış
oturumunda yapılır.

## Arkin için: gösterimi nasıl açarım ve telefonumda nasıl ölçerim

**Bir kez (ilk koşudan önce):**

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
```

**Bilgisayarda açmak:**

```sh
scripts/demo.sh                 # fixtures/local/bars varsa gerçek veriyle, yoksa sentetik
scripts/demo.sh --synthetic     # her zaman sentetik
scripts/demo.sh --from 2025-06-02 --speed 8   # gerçek veride bir tarihten, 8 adım/sn
```

Tarayıcıda **http://localhost:8390/**. İlk koşu çekirdeği derler (yaklaşık bir dakika).
`scripts/verify.sh`'tan sonra ilk açılışta `QP_DEMO_REBUILD=1 scripts/demo.sh` kullan (verify
paketi ölçüm için uygun olmayan biçimde derler; özellikle telefonda ölçmeden önce).
Sentetikse sayfanın üstünde "Sentetik veri" yazar. Belirteç gerekmez (gösterim
`fixtures/local/`'daki barları okur; yeni veri çekmek için `scripts/fetch-bars.sh`,
`docs/DATA_ACCESS.md`).

**Ne yapabilirsin:** akışta bir kapsüle dokun (açılır), bir daha dokun (grafik), işarete ya da
seviyeye dokun ya da **Kanıt** (kanıt değerleri). Grafikte sürükle, iki parmakla kıstır
(masaüstünde tekerlek). **‹ Geri** bir derinlik geri. Alttaki çubuk: ▶/❚❚, +1 adım, hız,
tarihe git. **Ayarlar:** dil, tema, hassasiyet, işaret alfabesi (`siluet`/`rozet`), yüz
(`vadrel`/`fotolay`, ayrıca `tavrik` ve `r3`), akış düzeni, geçiş, sonuç işareti, "veri yetkisi
yok" kipi, **Son bakışımı şimdi işaretle**, izleme listesi ve pozisyonlar. Aynı ayarlar URL'den:
`?face=fotolay&alphabet=rozet&lang=en&theme=light`. Geri gitmek otoriteyi baştan kurar.

**Telefondan açmak:**

```sh
scripts/demo.sh --lan --synthetic
```

1. Konsol `telefondan aç … http://192.168.x.y:8390/` yazar (Ayarlar sekmesinin altında da).
   Telefon **aynı Wi-Fi'de** olsun (misafir ağı olmasın, VPN kapalı).
2. Adresi **konsolun yazdığı gibi, IP ile** aç; başka ad 403 verir (güvenlik denetimi).
3. macOS "gelen bağlantılara izin verilsin mi" diye sorarsa **İzin Ver**. Sormazsa: Sistem
   Ayarları → Ağ → Güvenlik Duvarı → Seçenekler → `target/release/qp` için izin.
4. `--lan` açıkken aynı ağdaki cihazlar gösterimi görebilir; gerçek veriyle açıyorsan aklında
   olsun.

**Telefonda ölçmek (telefon başına ~2 dakika):**

1. Pil %50'nin üstünde, düşük güç kipi kapalı; sayfayı açıp 10 sn bekle.
2. Oynatma hızını **16×** yap, **▶**.
3. **Ölçüm** sekmesi → **Sıfırla**.
4. **Akış** sekmesi: 10 sn dokunma; sonra 10 sn parmakla aşağı yukarı kaydır.
5. Bir kapsüle iki kez dokun (grafik): 10 sn sağa sola sürükle, 10 sn iki parmakla kıstır.
   **‹ Geri**.
6. **Ölçüm** → **Özeti kopyala** (pano izni yoksa metin kutusundan elle kopyala).
7. Aynısını adresin sonuna `?face=r3&feed=r3&transition=r3&outcome=r3` ekleyerek bir kez daha
   yap (R3 görünümüyle karşılaştırma).
8. İlk açılış süresi için sekmeyi kapatıp yeniden aç; özetteki `open_ms.first_capsule`.

**Özeti nereye:** `rounds/R4/experiments/D/phone-<model>-<görünüm>.json` (ör.
`phone-pixel4a-default.json`) ya da ana akış oturumuna yapıştır. Özet yalnız süre, bayt ve cihaz
bilgisi taşır. **Hangi telefonlar:** kendi telefonun ve mutlaka 4–6 yaşında bir orta/alt sınıf
Android (60 Hz, 3–4 GB); varsa bir iPhone. Model, ekran tazeleme hızı ve pil durumunu not et.
Ölçümler K17 (telefon sınıfı hedefi) ve K45 (GPU) önerilerini doğrulayacak ya da
düzeltecek. Ayrıntı: `rounds/R4/DEVICE.md` §4.

**İnsan testi v2:** `rounds/R4/human-test/GUIDE.md` (yürütücü yönergesi, ~20 dakika, 5–10 kişi),
`links.html` (telefonda açılacak kodlu bağlantılar), `score.py` (CSV → karar).
