# R5 — Bağımsız son denetim (görev A)

- **Denetçi:** bağımsız alt ajan. R5'in hiçbir tasarım, ölçüm, bütünleştirme ya da rapor işini yapmadım.
  Görev tarifi orkestratörün iletisi + `COMMON.md`.
- **Taban:** `main` `c82850b` (R5 raporunun birleşmesi). Dal `r5/audit`, worktree
  `.claude/worktrees/agent-a0a0e58e47c8d480c`, `CARGO_TARGET_DIR=target/wt/audit`. `fixtures/local/`
  bu ağaçta yok; bütün denemeler **sentetik**.
- **Makine:** paylaşımlı; yük ortalaması 8–12 (verify sırasında), 4 (sonda).
- **Yazdığım yerler:** bu dosya ve `rounds/R5/audit/` (betikler ve küçük çıktılar `out/`). Ürün koduna,
  dökümlere, altınlara dokunmadım. Yeni dil modeli çağrısı yapmadım (ölçümler kayıtlı ham veriden).
- **Yapmadıklarım:** `git push`, `qgh`, birleştirme; bu yüzden `scripts/preflight.sh` ve CI (VERIFY 2)
  denetlenmedi. K52 kalibre benzetimi (VERIFY 13) yeniden koşulmadı. `scripts/clean.sh`'ın gerçek koşusu
  round kapanışının işi (yalnız `--dry-run` koştum).

## Hüküm: **KOŞULLU KABUL**

Engelleyici bulgu yok. Kendim koştuğum `QP_VERIFY_BROWSER=1 scripts/verify.sh` 18/18 PASS. Düzeneğin
alternatiflerden önce yazıldığı commit sırasından doğrulandı. M16'nın birincil ölçüsü kayıtlı ham veriden,
düzeneğin kodu kullanılmadan **üç haneye kadar aynı** çıktı. M17 taramaları yeniden üretildi (aday 0, `r4`
9 266, bozuk 34 006). Lisans girişimsizliği kendi denememde de tuttu. Yazı tipi lisansları uygun.

Ama raporun üç beyanı kanıtından geniş, bir seçim de yazılı kuraldan sapıyor. Koşullar (önem sırasıyla):

1. **Round kapanışı yapılmadı:** `REPORT.md`, `STATUS.md` ve `ROADMAP.md`'de `<A:…>` yer tutucuları
   duruyor (denetim hükmü, disk, ölçüt 13 ve 15, CI). `scripts/clean.sh` round sonu koşusu yapılmadı;
   `target/` **30 G** (T0'da 1,7 G idi). VERIFY 2 (preflight, CI) ana akışta koşulmalı.
2. **Karşıtlık beyanı (ölçüt 4, "bütün öğelerde") yalnız yüz ve akış için kanıtlı.** M16 yalnız yüz ve akışı
   ölçtü; M20 açılmış kapsülü `look=r4` ile ölçtü; S de `oltan` altında açılmış kapsül ve grafikte karşıtlık
   hesaplamadı. Kendi ölçümüm (B1): `oltan`'da **kapanmış kapsülün açılmış başlığı** `closed` renginde,
   19 px yarı kalın (600): açık 3,46, koyu 3,75. Düzeneğin kendi tanımıyla (600 = kalın, büyük metin, 3:1)
   geçiyor; 600'ü kalın saymayan yaygın tanımla (ör. ≥ 700) 4,5'in altında. Açılmış kapsül, kanıt ve
   grafikteki **eksen çizgisi** 2,88 / 2,95 (< 3; D bunu R7 için yazdı, ama aynı öğe kapsül deneyiminde de
   var). Ya REPORT ölçüt 4 ve `docs/DESIGN.md` beyanı "yüz ve akış" diye daraltılmalı ve bu iki öğe açık
   soru olarak yazılmalı, ya da düzeltilmeli.
3. **`marla`'nın sapma başlığında kaçamak kip (B2):** `reversal.rsi_divergence` başlıkları "SYN26 yukarı
   **dönebilir**: …" / "SYN26 **could** turn up: …" (531 durumun 60'ında, iki dilde). Düzeneğin `vague`
   listesinde "-ebilir" ve "could" yok; ama SPEC §2.1'in kendi gerekçesi (sözel olasılık sözcükleri geniş
   yorumlanır, yerine sayı ve pencere) bunu kapsar. Ölçüt 3'ün "yasak belirsiz ifade 0" beyanı listeye göre
   doğru, amaca göre tartışmalı. Karar Arkin'in (cümle iddianın beklentisini mi söylüyor, tahmin mi); REPORT
   ve `docs/LANGUAGE.md` bunu bilinen bir sınır olarak yazmalı ya da taramaya tarihli ekle girmeli. `selvi`'de 0.
4. **M21'in "ikinci" adayı yazılı kuraldan sapıyor (B3):** geçici varsayılan `surek`, "yalnız sonuçlardan önce
   yazılmış kurallar" (G6-5'in lafzı) uygulanarak seçildi. Aynı lafız `nabiz`'i sert eşikte eler (bilgi kaybı
   4); o okumada G6-7'ye göre ilk iki `surek`, `still`'dir (`M21/RESULTS.md` "Bağ çözme" tablosu, okuma B).
   `nabiz`'in ikinci olması geçersiz sayılan ajan sıralamasına (fark edilme, BT) dayanıyor; Ek G3 bu koşunun
   ajan sıralamasının kullanılmamasını söyler. Ayrıca SPEC Ek G6-8'in metni hâlâ "ana karar `any_prim`"
   diyor; orkestratörün eki tersini uyguladı. REPORT ve `DECISIONS.md` `nabiz`'in seçilebilirliğini "kuralın
   dışında, insan testi için" diye açıkça yazmalı (ya da ikinci `still` olmalı) ve SPEC/RESULTS çelişkisi
   bir notla giderilmeli.

> **Ajan insan değildir.** Bu denetimdeki tarayıcı denemeleri de başsız Chromium'da bir ajanın yaptığı
> denemelerdir. Seçimlerin hiçbiri insanla doğrulanmadı; son söz galeri ve insan testi v3'tür.

## VERIFY — 21 adım

| # | Kontrol | Sonuç | Kanıt |
|---|---------|-------|-------|
| 1 | `QP_VERIFY_BROWSER=1 scripts/verify.sh` | **PASS** | Çıkış 0; 18/18 (test 464 s; schema 28; views 12×2; wasm; digest `7b1e99d40889f83c` arm64 = x86_64 = wasm32; protocol 69; conform 64 × 3; replica; replay 1 816 satır; shell; demo: galeri, ayarlar, boyama yolları, 70 görüntü × 2, sahneler altınlara eşit). `QP_DEMO_PORT=8820`. `audit/out/verify-summary.txt` |
| 2 | preflight; CI | **KOŞULMADI** | Denetçiye gönderim/`qgh` yok. Ana akışta. |
| 3 | Düzenek commit'leri alternatiflerden önce | **PASS** | Aşağıda "Düzenek önce". |
| 4 | Bozuk tasarım sınaması | **PASS (M21 hariç, raporla aynı)** | `M16/run/broken-check.json` ve RESULTS'lar; M16'da kendi yeniden puanlamamda bozuk (`kestim`) 0,183 [0,171; 0,197], herkesin altında, ayrık. M21'de geçmediği raporda doğru yazılı. |
| 5 | Dil taramaları; tablo | **PASS (bulguyla)** | `langh dump` dört dil yeniden: `marla`/`selvi` 13 tarama 0; `r4` 9 266, bozuk 34 006 (SPEC'le aynı); `TABLE.md`'ler commit edilmişlerle bayt eşit. Kendi taramam: B2. |
| 6 | Karşıtlık, bütün öğeler, açık + koyu | **KISMEN** | Yüz ve akış: 374 metin ilkeli, eşik altı 0 (en kötü 9,77); kapanmış işaret 3,46 / 3,75. DOM metni 224 düğüm, eşik altı 0. Açılmış kapsül / kanıt / grafik: koşul 2 (B1). |
| 7 | Anlam renkleri gri + renk körlüğü | **PASS** | Kendi CIEDE2000 + Viénot/Machado uygulamam: `oltan` en kötü sayılan çift **10,53** (koyu `up ↔ text`, gri), raporla aynı; 17 çiftte eşik altı 0 (iki tema). Biçimle okunma: M16 gri kopya `dir`+`valid` 1,000 (kayıttan; yeniden okunmadı). |
| 8 | `?look=r4` ile v1 altınlar | **PASS** | verify 18 (v1 R5 öncesi tasarımlara sabit, sahneler altınlara eşit); `git diff 4ccd4d6 HEAD -- fixtures/golden`: yalnız 28 eklenen `v2/` dosyası, değişen 0. |
| 9 | v2 iki koşu | **PASS** | verify 18: iki koşuda sahneler bayt eşit, altınlara eşit (sayfa ekran görüntüleri tolerans içinde; sahneler bayt eşit). |
| 10 | Hikâye çizelgesi 5/50/200 | **PASS (yalnız testlerle)** | verify `test` (`stories*.rs`, `story_pavka*.rs`, `pavka_scene_digests`). Yeniden ölçmedim. |
| 11 | Hikâye listesi, yetkili/yetkisiz | **PASS** | Var olan testler geçti; kendi denemem: aşağıda "Lisans girişimsizliği". |
| 12 | Akış kuralı geleceğe bakmıyor | **PASS (testlerle)** | verify `test` (`feed_m19.rs`). |
| 13 | K52 kalibre benzetim | **KOŞULMADI** | Bütçe ve yük nedeniyle yeniden koşmadım; `S/K52.md` okundu. |
| 14 | Paket boyutu; yazı tipi lisansları | **PASS** | `QP_DEMO_PACK_ONLY=1 QP_DEMO_REBUILD=1 scripts/demo.sh`: WASM gzip **1 772 140** (rapor 1 771 943, +%0,01), brotli 1 147 121 (rapor 1 146 726), JS gzip 27 101 (aynı). Gömülü yazı tipleri +24 032 + 36 768 + 36 804 = **97 604 B** (raporla aynı). Lisans: aşağıda. |
| 15 | Dış ağa istek yok (galeri dahil) | **PASS** | Kendi Playwright denemem (`audit/probe.mjs`): 9 sayfa (iki tema × iki dil, yetkisiz, `?paint=cpu`, hareketi azalt × 2, `/gallery.html`), **312 istek, başka kökene 0**, sayfa hatası 0. |
| 16 | Hareketi azalt, bilgi kaybı | **PASS (gerekli koşul)** | Aynı adımda (216 → 217, 6 s sonra) `reducedMotion: reduce` ↔ `no-preference`: yüz kimlikleri ve yüz metinleri aynı, DOM metninden eksik 0. Geçiş anındaki kayıp M21'in ölçüsü (`surek` 0); onu yeniden üretmedim. |
| 17 | Mimari testi + kendi taramam | **PASS** | `arch-check` PASS. Kendi taramam: `main.js`, `gallery.js`, `paint/*.js`'te `sort`/`filter`/`toFixed`/`Intl`/`Math`/`parse*`/şablon dizgisi 0; `gallery.js` satır satır: metinler çekirdekten `textContent`'e; CSS'te renk sabiti 0, sayılar yalnız yapısal (`100%`, `100vh`, iki kırılma noktası 499/500 px, başlıkta yazılı). |
| 18 | Galeri telefon boyutunda | **PASS** | verify 18 `tests/gallery.mjs`; kendi denememde 55 tuval/görüntü, 61 istek, hepsi aynı köken. |
| 19 | `?paint=cpu` | **PASS** | Ölçüm özeti `{"requested":"cpu","used":"canvas","fallback":"removed"}`. |
| 20 | `clean.sh --dry-run`, sonra gerçek koşu | **YARIM** | `--dry-run`: yalnız depo içi (`target/…`, kilitli worktree'mi atladı), `fixtures/local`/`.env.local` yok. Gerçek koşu ve "yalnız ana ağaç" round kapanışında (koşul 1). |
| 21 | Gizli bilgi; `fixtures/local` yok | **PASS** | `audit/hygiene.sh`: `main` farkında (4ccd4d6..HEAD) ve her `r5/` dalının kendi farkında anahtar kalıbı 0; izlenen `.env`/anahtar dosyası yok; `fixtures/local` ve `.env.local` hiçbir dalın hiçbir commit'inde yok; R5'te eklenen ikili dosyalar (yazı tipi 3, altın 28, örnek görsel 17, `run/` sıkıştırılmış ham kayıt 10) — örnek görseller sentetik (`SYN..`, "Sentetik veri"); deney dosyalarında gerçek BIST simgesi taraması 0. 300 KB üstü 5 dosya (ham okuyucu kayıtları ≤ 1,15 MB, dil tabloları). |

## Düzenek önce (madde 2)

`audit/out/firsts.txt` (her dalın "R5/<dal>" önekli ilk ve son commit'i, ilk commit'in atası) ve
`git merge-base --is-ancestor`:

| Düzenek | Commit (yazar zamanı, 18 Kasım) | Alternatiflerin ilk commit'i (yazar zamanı) | Ata mı |
|---|---|---|---|
| M17 dil + M19 akış | `fa0487d` 00:07 | `tunca` 657a9b2 00:30, `marla` 07ec32f 00:41, `selvi` e544d28 00:46; M19 2fa4931 01:23 | evet (hepsi `fa0487d`'den çıkıyor) |
| Tek bakış (M16/18/20/21) | `70d8e11` 00:20 | M18: `pavka` **7520a1c 01:17**, `korda` 5db01d8 01:28, `selen` b4f435d 01:40 (taban `c414e44` 00:44) | evet |
| Bozuk + Ek G3 | `0e85cb6` 01:10 | M16: `zemre` a816424 03:26, `kivra` 2a628b2 03:30, `oltan` d04b64a 03:52, `bertan` b1660b2 04:05 (taban `6808268`); M20 `nobra` 9cd323f 06:09, `tesir` c7555af 06:54; M21 `nabiz` a361aee 06:13, `surek` 1659a87 06:59 (taban `d28cd2c`) | M16/M20/M21: evet; M18 alternatifleri G3'ten önce dallandı (G3 yalnız puanlama; M18 ölçümü G3'ü içeren dalda) |

Not: raporun saatleri **yazar** zamanıdır; M16 dalları T3c sonrası yeniden tabanlandığı için commit zamanları
04:48'dir. Raporun ölçüt 2'sinde M18'in ilki `5db01d8 01:28` yazıyor; ilki `pavka` `7520a1c` 01:17 (Ö1).

**Ölçüm sonrası ekler.** SPEC'te Ek G1–G7 ve G6-8 (8 ek); dil ve akış SPEC'lerinde ek yok.

| Ek | Commit | Alternatifler o anda var mıydı | Okuma sonuçlarından önce mi (kayıttan) |
|---|---|---|---|
| G1 | kuru koşu (`70d8e11` içinde) | hayır | evet |
| G2 | `16956b7` 01:06 | M18 hariç hayır | evet |
| G3 | `ed1141c` 01:09 | M18 dalları açılmıştı (01:17'den önce commit yok) | evet |
| G4 | `2ea1ab6` 04:34 | evet (M16 dördü, aday raporuyla tetiklendi) | evet (M16 okumaları `m16-measure`'da, 06:00 sonrası) |
| G5 | `85338e2` 06:00 | evet | evet: `m16-measure`'ın **ilk** commit'i; sonuç commit'i `41c948f` 08:45 |
| G6 | `fca8f33` 07:54 | evet | evet: `m21-measure`'ın ilk commit'i |
| G7 | `655e93e` 08:06 | evet | evet: `m20-measure`'ın ilk commit'i; sonuç `49c9e6e` 08:44 |
| G6-8 | `69d27cc` 08:10 | evet | **kayıp sonuçlarından sonra**, okuyucu sonuçlarından önce (metninde yazılı); 09:06 kullanım sınırı okumaların bu saatten sonra sürdüğüyle tutarlı |

Sınır: okuyucu kayıtlarında (`readers.jsonl.gz`) zaman damgası yok; "önce" yalnız commit sırasından ve
eklerin kendi beyanından anlaşılıyor. Kayıtlar bunu çürütmüyor, kanıtlamıyor da. **Karar belirleyen iki ek**
(G5 `bertan`'ı eledi; G7(d) `e_who` anahtarı `tesir`'i öne koydu) adayları gören ölçüm ajanlarınca yazıldı;
okumalardan önce oldukları doğru, adaylardan önce değil. REPORT bunu tetikleyenleriyle açıkça yazıyor.
REPORT'un ek tablosunda yer almayanlar: M18'in `chart` soru kümesi ve anahtarı (`17dce7b`, alternatiflerden
sonra; §G9 bunu öngörüyordu) ve M18'in K52 JIT'siz kapı yorumu (sayılar görüldükten sonra; REPORT "Sınırlar
7"de yazılı) (Ö2).

## Ölçümlerin yeniden üretimi (madde 3)

- **M16 birincil, kendi kodumla** (`audit/rescore_m16.py`; anahtar SPEC metninden türetildi, `keys.py`/
  `select.py` okunmadı; yalnız haiku, `one_look`; `t_valid` sınıf dengeli; F2:H1; 2 000 önyükleme):
  `oltan` **0,471** [0,445; 0,497], `bertan` 0,345, `r4` 0,318, `zemre` 0,279, `kivra` 0,243, bozuk 0,183 —
  RESULTS'la üç haneye kadar aynı, aralıklar ±0,002.
- **M17 taramaları** (`langh dump`, 4 dil): 13 tarama `marla` 0, `selvi` 0, `r4` 756/0/76/2344/1557/1294/
  1002/1581/636/20/0/0/0, bozuk 7681/…/14 — SPEC §2 ve RESULTS §2 ile aynı; `TABLE.md`'ler bayt eşit.
- **Ek G5 / `bertan`** (kendi ΔE uygulamam, G5'in yazılı rol kümesi): açık `up ↔ invalid` gri **7,28** < 10
  (raporla aynı); ayrıca koyu `down ↔ invalid` 7,51 de eşik altı. `zemre` 10,43, `kivra` 11,45, `oltan` 10,53
  geçiyor. Kural metne göre uygulanmış; `bertan` iki temada da kalıyor.
- **M17 §5.4 bağ çözme:** `marla` 222,4 < `selvi` 234,9 < `tunca` 236,0 karakter. "Kısa olan"ın kimin
  arasından seçileceği SPEC'te yazılı değil; raporun üç okuması doğru aktarılmış (`selvi` her okumada ilk ikide).
- **M18 `pavka` eki:** SPEC §G7.6'nın lafzıyla (JIT'siz kayıt, kapı değil) `pavka` ilk koşuda da kazanıyordu;
  ölçüm ajanının sonradan sıkılaştırdığı kapı onu eledi; başarım düzeltmesi (sahne bayt eşit, `pavka_scene_digests`
  testi geçti) onu geri aldı. Sonuç önceden yazılı lafızla aynı (`pavka`, `korda`). Eşitsizlik: düzeltme fırsatı
  yalnız `pavka`'ya verildi (`selen` sınırda zaten geçiyordu); sonucu değiştirmez.
- **M20 `e_who`:** G7(d) sıkı anahtarı yazıldığı gibi uygulanmış (`open().trust` "cihaz" → `device`);
  gevşek okuma yalnız bilgi. Rapordaki duyarlılık doğru.
- **M21:** "geçersiz, kazanan yok" doğru uygulanmış; ikinci aday için koşul 4.
- **M19:** §4 adımları RESULTS'ta doğru; sıralama kuralı `Grouped`'ı seçiyor, gösterim `elikov` kaldı (REPORT ⚠️
  doğru); kuralın ikinci sıralaması `Trust`, ürüne `live_new` girdi (RESULTS'ta yazılı).

## Kendi taramalarım (madde 4)

**Dil** (`audit/langscan.py`; girdi `langh dump`'ın metni, tarama ve listeler benim; düzenekten bilerek geniş:
olasılık/kaçamak, yoğunluk sıfatları sayısız cümlede, al-sat/emir/"gerekir/should/consider", iç terimler
— olgu, bulgu, kapsül, kademe, aile, teyit, sinyal, skor, replika, otorite, mühür, bar, ledger, confidence…,
aynı ekranda tekrar). Duyarlılık sınaması: `r4`'te iç terim 2 367 (düzenek 2 344), tekrar 76 (düzenek 76).

| | kaçamak | yoğunluk (sayısız) | tavsiye | iç terim | tekrar |
|---|---:|---:|---:|---:|---:|
| `marla` | 61 → **60 gerçek** ("dönebilir"/"could"; 1'i "9 May" ayı) | 509 → 0 gerçek (hepsi "en yüksek/en düşük fiyat", "20-day high/low": tanımlı eşik adı) | 402 → 0 gerçek ("net buying/selling" betimleme) | **0** | **0** |
| `selvi` | 1 → 0 ("9 May") | 22 → 0 ("higher high", "large enough move") | 130 → 0 (betimleme) | **0** | **0** |

**Karşıtlık ve ΔE** (`audit/colour.py`: WCAG 2.2, CIEDE2000 — Sharma 2005 test çiftleriyle sınandı —, gri Y'den,
Viénot protan/deutan + Machado protan/deutan/tritan; tritan'da Brettel yerine Machado, bilerek farklı):

- Görünüş kaydı rolleri (`qp look show oltan`; `audit/out/look_oltan.txt`): metin `text` 17,13 / 18,72,
  `muted` 9,77 / 14,35, kabuk rolleri en kötü açık `chrome.accent` üstünde `chrome.bg` 5,52; `closed`/`invalid`/
  `neutral` 3,75 / 3,46 (metin dışı ≥ 3; küçük metin olarak kullanılırsa < 4,5); `axis` 2,95 / 2,88, `line`/
  `grid` 1,2–1,5 (süs).
- Çizilmiş sahneler (`audit/probe.mjs` → `probe_analyze.py`; 4 adım × 2 tema × 2 dil, duyarlılık `all`, 48 yüz,
  36 açılmış sahne): yüz metni 374 ilkel, eşik altı 0, en küçük 15 px; kapanmış işaret gövdesi 3,46 / 3,75;
  işaret plakası (`#…3d`, %24) 1,28 süs. Açılmış kapsül: kapanmış başlık 3,46 / 3,75 (19 px yarı kalın) — B1.
  Eksen 2,88 / 2,95. DOM metni 224, eşik altı 0. `qp scene --roles` R2 görünümlerini D zaten ölçtü; yeniden
  koşmadım.

## Lisans girişimsizliği (madde 5)

- Var olan testler (`non_interference.rs` 5 test, `stories.rs` 3 test, ses testleri) verify'da geçti.
- **Kendi denemem** (`audit/ni/`, ayrı Cargo projesi, ürün crate'lerine yol bağımlılığı): aynı açık barlar,
  lisanslı akış tohumu 7 ↔ 1 / 2 / 42 / yok; okurlar tam yetkili, yalnız bar yetkili, hiç yetkisiz (TR ve EN);
  sesler `marla` ve `selvi`; duyarlılık `all`; `capsules`/`capsules all`/`changes`/`claims`/`stories` konuları
  + `ledger`, `describe`, hikâye sorguları; **akış kuralı kapalı, `rate55`+muaf, `rate55`, ve çok gizleyen sınama
  kuralı (N 4, τ 0,6)**. Var olan testler akış kuralını kapalı koşuyordu; `hidden`/`coin_flip`/`in_feed` dolaylı
  kanal olabilir miydi diye baktım. Sonuç: 2 ses × 4 kural × 4 tohum × 3 yetkisiz bağlantıda sunucu yorumları
  çıkarılınca **bayt eşit, ayrışma 0**; sınama kuralında yetkisiz okurlar 162 `coin_flip` işareti gördü ve yine
  bayt eşit; olumlu denetim (tam yetkili okur tohumla değişiyor) her bileşimde tuttu. `rate55` bu kısa tarihte
  hiç gizlemedi (N ≥ 20 dolmadı) — o bileşim boş sınamadır, sınama kuralı onu kapatıyor.
- Hiç yetkisi olmayan okurun ses, hikâye satırı, yüz ve açılmış kapsül metinlerinde rakam 0. Tarayıcıda yetkisiz
  okur (`principal=visitor`): yüzler cümle derinliğinde DOM'da, rakamlar sözcükle ("yirmi günlük"), rakam 0.
- Bilgi (R5'e özgü değil): aynı okurun `welcome.summary.facts[].text` ve `changes` olgu metinleri bar sırası
  içeriyor ("… at bar 114"); değer değil, makine alanı (Ö5).

## Yazı tipleri (madde 6)

`crates/qp-render/assets/fonts/` (`audit/fonts.py`, kendi TTF ayrıştırıcım):

| Dosya | Boy | Ad tablosu | Lisans | Değiştirilmiş mi | RFN |
|---|---:|---|---|---|---|
| `IBMPlexMono-Regular.ttf` | 135 580 | IBM Plex Mono 2.3, DSIG var | `OFL.txt` ("Reserved Font Name Plex") | hayır (R0'dan beri, `ffbe777`) | var; ad değişmemiş, dosya özgün → uygun |
| `AtkinsonHyperlegibleNext-{Regular,SemiBold,Bold}.ttf` | 36 768 / 36 804 / 24 032 | Atkinson Hyperlegible Next 2.001; ad 0 telif, ad 13–14 OFL | `OFL-AtkinsonHyperlegibleNext.txt` | evet (statik örnek + alt küme, rakamlar `tnum`'a eşlendi) | **yok** → aynı adla dağıtılabilir; telif ve lisans metni dosyada ve depoda |

Türkçe (ğ ş ı İ ç ö ü â î û, büyük-küçük) dördünde tam; rakam ilerlemeleri tek değer (632 / 600 birim:
tablo rakamı). `₺` Atkinson'da yok (REPORT'ta yazılı). Kozmetik: Bold'un ad 3'ü "…-Regular" kimliğini taşıyor (Ö6).
Görünüş kaydındaki `spdx: OFL-1.1` ve `license` alanları dosyalarla tutarlı.

## Kabul ölçütleri — denetçinin hükmü

| # | REPORT | Denetçi | Gerekçe |
|---|---|---|---|
| 1 | ✅ | ✅ | `RESEARCH.md` 73 tekil bağlantı (rapor 72), aday ve elenen fikirler var. İçeriği derinlemesine sınamadım. |
| 2 | ✅ (notlarla) | ✅ (notlarla) | Commit sırası doğrulandı. Notlar: Ö1 (M18 ilk commit), Ö2 (ek tablosunda eksik iki kalem), zaman damgası yok. |
| 3 | ✅ | ⚠️ | Taramalar yeniden üretildi (0). Kendi taramam `marla`'da 60 × 2 kaçamak kip buldu (koşul 3). Tekrar ve iç terim 0 bağımsız olarak doğrulandı. |
| 4 | ✅ | ⚠️ | Dört yön, kayıtlar çekirdekte, kabukta biçem yok (doğrulandı); yüz/akış karşıtlığı ve ΔE kendi kodumla aynı. "Bütün öğeler" açılmış kapsül ve grafiği kapsamıyor (koşul 2). |
| 5 | ✅ | ✅ (okundu) | D'yi yeniden üretmedim; ölçütün istediği belgeler ve R7 soruları var. |
| 6 | ✅ | ✅ | Testler geçti; PROTOCOL §23, dökümler 60–63 üç taşımada. K52'yi yeniden ölçmedim. |
| 7 | ⚠️ | ⚠️ | Raporla aynı. |
| 8 | ✅ (notla) | ✅ (notla) | Girişimsizlik kendi denememde de tuttu. `e_who` duyarlılığı doğru aktarılmış. |
| 9 | ⚠️ | ⚠️ | Raporla aynı + koşul 4 (`nabiz`'in yeri). |
| 10 | ✅ (notla) | ✅ (notla) | Paket boyu yeniden üretildi (±%0,01). K52 yeniden koşulmadı. |
| 11 | ✅ | ✅ | verify 18 + kendi ağ denemem. |
| 12 | ✅ (notla) | ✅ | Paket yerinde; içerik akışını sınamadım. |
| 13 | ✅ / `<A:disk>` | ⚠️ | `cpu` yolu yok (doğrulandı), `clean.sh --dry-run` var; round sonu koşusu yok, `target/` 30 G (koşul 1). |
| 14 | ✅ | ✅ | v1 değişmedi, v2 28 dosya iki koşuda eşit, hesap özeti aynı, 64 × 3. |
| 15 | `<A:ölçüt15>` | ⚠️ | `AUDIT.md` bununla var; DESIGN/LANGUAGE/FACE, Önerilen (R5) Ö61–Ö70 ve A41–A44 var; yer tutucular ve disk bekliyor (koşul 1). |

## Bulgular

### Engelleyici
- Yok.

### Önemli (koşullara bağlı)
- **B1** Açılmış kapsülde kapanmış başlık 3,46 / 3,75 (19 px, 600) ve eksen 2,88 / 2,95 — ölçülmemiş öğeler;
  ölçüt 4'ün kapsamı (koşul 2). Kanıt `audit/out/probe_capsule.txt`.
- **B2** `marla` sapma başlığında "dönebilir / could" (koşul 3). Kanıt `audit/out/langscan.txt`.
- **B3** M21 ikinci aday ve Ek G6-8 metni (koşul 4).

### Küçük
- **Ö1** REPORT ölçüt 2: M18'in ilk alternatif commit'i `pavka` `7520a1c` 01:17 (rapor `5db01d8` 01:28); saatler
  yazar zamanı olduğu bir notla belirtilmeli.
- **Ö2** REPORT'un ek tablosu M18 `chart` soru kümesini (alternatiflerden sonra yazıldı, §G9'un öngördüğü) ve M18
  K52 kapı yorumunu anmıyor.
- **Ö3** Okuyucu kayıtlarında zaman damgası yok; "ek okumalardan önce" yalnız commit sırasıyla kanıtlanabiliyor.
- **Ö4** M19'un yazılı sıralama kuralının ikincisi `Trust`; ürüne `live_new` girdi (RESULTS'ta yazılı, REPORT'ta
  "ikinci (seçilebilir)" sütunu bunu belirtmiyor).
- **Ö5** Hiç yetkisi olmayan okura `welcome` özeti ve `changes` olgu metinleri bar sırası (rakam) taşıyor; değer
  değil, R5 öncesinden geliyor olabilir (doğrulamadım). K33 yorumu için A41'in yanına yazılabilir.
- **Ö6** Atkinson Bold'un ad 3'ü (benzersiz kimlik) "AtkinsonHyperlegibleNext-Regular"; kozmetik.
- **Ö7** verify adım 8'in "wasm" boyu (2 280 587 / 745 443) ile gösterim paketi (5 883 320 / 1 772 140) farklı
  derlemeler; REPORT doğru olanı (gösterim paketini) kullanıyor, karışmasın diye not.
- **Ö8** Gösterimdeki yetkisiz okurun ilk ekranında sıra cümlesi "Diğerleri, geçmişte en çok tutan önce" — K53
  etiketinin yetkisiz okurda anlamı (karne görmüyor) insan testinde bakılabilir.

## Öneriler (kapsam dışı)
- Düzenek: okuyucu kayıtlarına çağrı zamanı; renk/karşıtlık kapısını yüzle sınırlı bırakmayıp açılmış kapsül,
  kanıt ve hikâye çizelgesi sahnelerine de uygulamak (boyama listesi zaten var; bu denetimdeki `probe_analyze.py`
  yöntemi yeter); "kalın" tanımını (600 mü 700 mü) SPEC'e yazmak.
- Dil: `vague` listesine kip ekleri ("-ebilir", "could/may/might") için tarihli ek ve iddianın beklentisini
  söyleyen kalıp için açık bir istisna kuralı.
- Girişimsizlik testine akış kuralı açık bir bileşim (bu denetimdeki `audit/ni/` gibi) eklemek.

## Yeniden üretim

Depo kökünden (`CARGO_TARGET_DIR` kendi alt klasörünüz; çıktılar scratchpad'e):

```sh
QP_VERIFY_BROWSER=1 QP_DEMO_PORT=8820 scripts/verify.sh
bash rounds/R5/audit/firsts.sh; bash rounds/R5/audit/hygiene.sh
python3 rounds/R5/audit/rescore_m16.py rounds/R5/experiments/M16/run
(cd rounds/R5/experiments/harness/lang/tool && cargo build --release --offline)
for l in marla selvi r4 bozuk; do $CARGO_TARGET_DIR/release/langh dump --lang $l --out $OUT/$l; done
python3 rounds/R5/audit/langscan.py $OUT/marla/dump.json $OUT/selvi/dump.json
qp look show oltan > $OUT/oltan.json; python3 rounds/R5/audit/look_check.py $OUT/oltan.json
node rounds/R5/audit/probe.mjs $CARGO_TARGET_DIR/release/qp 8821 $OUT/probe.json
python3 rounds/R5/audit/probe_analyze.py $OUT/probe.json $OUT/oltan.json
(cd rounds/R5/audit/ni && cargo run --release --offline)      # girişimsizlik denemesi
python3 rounds/R5/audit/fonts.py crates/qp-render/assets/fonts/*.ttf
QP_DEMO_PACK_ONLY=1 QP_DEMO_REBUILD=1 scripts/demo.sh          # paket boyu
```

Denetimin çıktıları `rounds/R5/audit/out/` (204 KB'tan küçük; ekran görüntüsü yok). NI denemesinin özet satırındaki
"FAIL", yetkisiz okurun makine olgu metinlerindeki bar sırası rakamlarından geliyor (Ö5); bayt ayrışması 0.
