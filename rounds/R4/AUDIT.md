# R4 — Bağımsız son denetim

- **Denetçi:** bağımsız alt ajan (R4'ün hiçbir işini yapmadı). Görev tanımı `rounds/R4/tasks/A.md`.
- **Taban:** `r4/int` `39fcaf0`; çalışma ağacı `.claude/wt/audit` (dal `r4/audit`), temiz kopya,
  `fixtures/local/` yok, `QAPSULE_LOCAL_BARS` tanımsız. Bütün gösterim denemeleri **sentetik pazarla**.
- **Makine:** Apple M4 Max, 16 çekirdek. Yük ortalaması denetim boyunca 4,3–25 (başka işler koşuyordu;
  zaman ölçümlerinde ayrıca yazıldı).
- **Denetçinin yazdığı yerler:** bu dosya ve `rounds/R4/audit/` (betikler, sentetik ekran görüntüleri,
  ham çıktılar, `m8-repeat/`). Ürün koduna, dökümlere, altınlara dokunulmadı. Hassasiyet benzetimi
  `rounds/R4/experiments/T3/sensitivity.{md,json}`'u yeniden yazar; çıktı commit'lenmiş dosyayla bayt
  eşit çıktığı için ağaçta değişiklik kalmadı (`git status` yalnız `rounds/R4/audit/`).
- **Yapılmayan (talimat gereği):** `git push`, `gh`/`qgh`, birleştirme. Bu yüzden `scripts/preflight.sh`
  (içinde `qgh api` ve `gh api` çağırır) ve CI son çalışması denetlenmedi.

## Genel hüküm: **KOŞULLU KABUL**

Belirleyicilik, kabuğun inceliği, ağ, hijyen, K42 kuralları, açılış süresi ve cihaz ölçümü denetçinin
kendi denemelerinde tuttu. Engelleyici bulgu yok. Kabul koşulları (önem sırasıyla):

1. **GOAL 6 kısmen karşılanıyor:** kapsül derinliğindeki **ayrıntı cümlesi** hâlâ R3 cümlesi: bar sayısıyla
   süre ("85 bar önce", "Bu barda"), iç terimler ("aynı aileden", "Sinyal", "aşağı kırılım"). Aynı kapsülün
   yüzüyle de çelişebiliyor (aşağıda Ö1, Ö2). Ya düzeltilmeli ya da REPORT'ta "ayrıntı cümlesi R5'e" diye açıkça
   sapma olarak yazılmalı (bugün REPORT ölçüt 6'yı ✅ gösteriyor ve bundan söz etmiyor).
2. **GOAL 15 tamamlanmamış:** `REPORT.md` `r4/int`'te yok (r4-int ağacında commit edilmemiş taslak; `<!--DURUM-->`,
   `<!--C2-->`…`<!--C4-->`, `<!--SAPMALAR-->`, `<!--DENETIM-->` yer tutucuları duruyor); `STATUS.md` ve `ROADMAP.md`
   `main`'e göre değişmemiş (`git diff main -- STATUS.md ROADMAP.md` boş). Round kapanışında yapılmalı.
3. **`qp replay` geri sarmadan sonra susuyor (Ö3):** VERIFY 16'nın "iki koşu bayt eşit" kanıtı geri sarmadan
   sonraki 180 adım için boş akışı karşılaştırıyor. Gösterim (tarayıcı) geri sarmadan toparlanıyor; sorun
   kayıt aracında ve testin kapsamında. REPORT'a yazılmalı, düzeltme R5'e bırakılabilir.
4. VERIFY 2 (preflight, CI) ana akışta koşulmalı (denetçiye `gh` yasaktı).

> **Persona ajanı insan değildir.** M8 tekrarı da dahil, persona ve eleştirmen ajanlarının sonuçları
> ön elemedir; küçük pikseli ve beş saniyelik bakışı insan gibi deneyimlemezler. Bu denetimdeki
> "kullanıcı gibi kullanım" da bir ajanın başsız tarayıcıda yaptığı denemedir; insan testi v2'nin
> yerini tutmaz.

## VERIFY — 20 adım

| # | Kontrol | Sonuç | Kanıt |
|---|---------|-------|-------|
| 1 | `QP_VERIFY_BROWSER=1 scripts/verify.sh` | **PASS** | Çıkış 0; 18/18 adım PASS (fmt, clippy, test 163 s, schema 28, fixtures, golden, views 12×2, wasm, hygiene, browser, compute, digest `7b1e99d40889f83c` arm64=x86_64=wasm32, protocol 66, conform 58×3, replica, replay 693 satır, shell, demo). Yük 9–25. |
| 2 | `scripts/preflight.sh`; CI | **KOŞULMADI** | Betik `qgh api`/`gh api` çağırıyor; denetçiye `gh` yasaktı. Ana akışta koşulmalı. |
| 3 | `qp conform` üç taşımada; değişen dökümler raporla aynı | **PASS** | verify `conform`: 58 döküm × inproc/serve/MCP. `git diff --name-status main` ile değişen dökümler 38–44 (+ `views/docs/r3-*.json`, `c1-sentence.json`); hepsi `experiments/T3.md` §2 tablosunda ve REPORT taslağında; 01–37 değişmemiş. |
| 4 | Var olan altınlar ve hesap özeti | **PASS** | `main`'e göre değişen altın yalnız `views/c2-capsule.png`, `c3-chart.png` (T3.md'de K42 gerekçesiyle listeli); `fixtures/compute` değişmemiş; digest adımı PASS. Ek değişen veri: `fixtures/calendar/xist.json` (FIX-E1'de gerekçeli). |
| 5 | `scripts/demo.sh` temiz kopyada, yerel veri olmadan | **PASS** | `scripts/demo.sh --port 8641`: konsol "veri: sentetik pazar … (no local data on this machine …)"; yalnız `127.0.0.1:8641` dinleniyor (`lsof`); sayfada "Sentetik veri" bandı (`audit/shots/01-acilis.png`). `Host: evil.example` → 403, belirteçsiz POST → 403. |
| 6 | Gösterimi kullanıcı gibi kullanma | **PASS (bulgularla)** | `audit/scripts/explore.mjs`, 390×844, DPR 2; 17 ekran görüntüsü `audit/shots/`, günlük `audit/explore.log`. Her işlev çalıştı; sorunlar aşağıda "Gösterim bulguları". |
| 7 | Yeniden oynatma iki kez | **PASS (bulguyla)** | Kendi denetim dizim (`audit/scripts/replay-controls.json`: hız, adım 7, geri seek, duraklat/sürdür, tarihe git, ileri seek; tohum 7, 12 enstrüman, 400 adım): iki koşu `cmp` eşit, SHA-256 `1c9036…5c358`; OHLC yolu (`--fine none`) iki koşu eşit. Tarayıcıda aynı girdiler → aynı 716 çerçeve (VERIFY 8). **Ama** geri seek'ten sonra `qp replay`'in bağlantısı kapanıyor, sonraki denetimler `handshake` hatası, akış boş (Ö3). |
| 8 | Kabuksuz sürüş | **PASS** | Kendi yöntemim, `qp drive` kullanmadan (`audit/scripts/rawparity.mjs`): tarayıcıda belirleyici bir oturum (adım/tarih, 3. kapsüle dokunma, grafik, sürükleme, tekerlek, kanıt, geri, hassasiyet, kitle, tema, alfabe, yüz `fotolay`); sayfanın 33 POST'u WASM'sız düz bir HTTP istemcisiyle taze `qp demo`'ya gönderildi: **716/716 çerçeve, 4.389.307 bayt, belirteç dışında bayt bayt aynı** (`audit/rawparity/result.json`). Ayrıca verify 18 (`qp drive` = tarayıcı özetleri) PASS. |
| 9 | Mimari testi + kendi taramam | **PASS** | `arch-check.mjs` PASS. `main.js` (704 satır) satır satır okundu: aritmetik, sıralama, süzme, biçimleme, metin kurma yok; metinler `textContent`'e olduğu gibi; konum/isabet kararı yok (`core.pointer` ham `offsetX/Y`). Tek karşılaştırma `firstFaces`'ta görünürlük (`r.bottom > box.top`) — liste kaydırma düzen öğesi, alan bilgisi değil. `index.html`/`style.css`'te dış adres, `@import`, `url()` yok. |
| 10 | Ağ istekleri | **PASS** | İki yoldan kayıt (`page.on('request')` ve `context.route('**')`): 71/71 istek, başka kökene 0; yollar `/`, `/style.css`, `/main.js`, `/pkg/qp_wasm.js`, `/pkg/qp_wasm_bg.wasm`, `/pkg/sizes.json`, `/events`, `/send`. Sayfa hatası/uyarısı 0. |
| 11 | Geçerli olmayan kapsül: başlık ve işaret | **PASS (yüz)** | `audit/k42-sheet.png` (+ gri): `SYN26` "Kapandı: 10 Mart'ta tutmadı." / "Closed: did not play out on 10 March." + soluk iz ✗; `SYN02` "Kapandı: geçen hafta gerçekleşti." / "played out last week." + ✓; `SYN03` "13 işlem günü önce zayıfladı. Artık geçerli değil." + ⊖ rozeti (`shots/10`); TR/EN × açık/koyu aynı. **Ayrıntı derinliğinde** aynı kapandı-tutmadı kapsülü "artık geçerli değil" diyor (Ö2). |
| 12 | Süre ifadesi, günlük ve 60 dk, hafta sonu/tatil | **PASS (yüz)** | `audit/probe` (`elapsed_probe`), XIST takvimi, 21 kendi örneğim: Cuma→Pazartesi, Kurban 2025 (6 ve 9 Haz kapalı), 1 Ocak, 29 Ekim (28 yarım gün), Ramazan 2025, yıl aşan tarih; 60 dk'da "cuma 15:00'te", "perşembe 11:00'de", "dün 17:00'de", "4 saat önce": **21/21** (`audit/elapsed_probe.out`; ilk koşuda tek fark benim beklentimdi: Salı→Pazartesi 4 işlem günü, kural 2–5 önce gelir). Ürünün ayrıntı cümlesi bu işlevi kullanmıyor (Ö1). |
| 13 | M8'den üç persona koşusu | **PASS** | `audit/m8-repeat/`: aynı düzenek ve yöntem (`run_readers.sh m8 <yüz> F 1 1`, `SEED_BASE=1`, `claude -p --model sonnet --restricted …`, model `claude-sonnet-5-5`), istem şablonları özgün r1'lerle **aynı** (`diff`), görüntüler aynı dosyaların kopyası. Tam doğru (6 kart): `vadrel` 3, `fotolay` 2, `tavrik` 2 → sıra `vadrel` > `fotolay` = `tavrik`; özgün F/s1 toplamı 10/18, 9/18, 8/18. Seçim (vadrel birinci; ilk iki vadrel, fotolay) ile tutarlı. Farklar aşağıda. |
| 14 | M8–M11 kayıtları | **PASS (notla)** | `git log`/`merge-base`: SPEC `bad3d67` (12:04) her alternatifin atası; M11 düzeneği `2db6d25` M11 alternatiflerinin (`10fabcd`, `187edc2`) atası; M8910 düzeneği `6b4b581` M8/M9/M10 alternatiflerinin atası; okuyucu kayıtları alternatiflerden sonra (M11 13:27, M9 16:02, M10 16:11, M8 20:36). İstemlerde tasarım adı taraması (M8 102, M9 36, M10 36, M11 36 istem; `meta.json`, `images.json` 492 görüntü adı): **0 isabet**. Not: SPEC E6 (`fa07648`, 16:39) iki M8 alternatifi teslim edildikten sonra ama okumalardan önce yazıldı (Ö7). |
| 15 | Renk karşıtlığı ve gri kopya | **PASS (bilinen açıkla)** | `audit/scripts/contrast.py` altın yüzlerde: ek bilgi 15,8 (açık) / 11,7 (koyu); başlık 5,78 / 5,27 (geçerli olmayanın soluk rengi dahil) ≥ 4,5. İşaretin anlam taşıyan köşesi (✓/✗) 3,57–4,47 ≥ 3. Gövde: gerçekleşti-kapandı **2,06** (açık) / 2,36 (koyu) — bilinen A31; **tutmadı-kapandı soluk izin gövdesi 1,06–1,37** (raporda yok, Ö6). Gri kopyada (`shots/*.gray.png`, `k42-sheet.gray.png`) yön ok/şekille, geçerlilik sözcük + rozetle (⊖/✓/✗) okunuyor. |
| 16 | Ölçüm ekranı, masaüstü ve 4× | **PASS** | `node shells/web/tests/device.mjs … --rates 1,4 --dprs 1,3 --looks default --reps 3` (demo.sh ile aynı adsız paket, 4.377.216 B; yük 4,3–9). DEVICE.md §2.1 ile: ilk kapsül 114,6/117/397/410 ms (DEVICE 120/121/401/460; en çok −11 %); tik p50/p95, yüz ve sahne boyama p50/p95, kare p95 ve kare/sn hepsi ±%20 içinde (örn. 4× DPR3 tik 3,3/32,6 ↔ 3,3/35,9; sahne 10,1/11,9 ↔ 10,5/12,2). Tek aşım: 4× DPR1 durağan **p99** 23,1 ↔ 17,1 (+%35; kuyruk ölçüsü, uzun kare payı %1,2 ↔ %1). `audit/device-synthetic.log`. |
| 17 | Bağlanma, 30 enstrüman | **PASS** | `capsule_tick 30 1000 120 3 caps,flow` (T6 düzeneği): `caps` **82,6 ms** (min 81,9, max 91,6) < 100; `flow` 121,4 ms (> 100, T6.md'de nedeniyle raporlanmış). Yük 4–6. `audit/capsule_tick.log`. |
| 18 | Hassasiyet benzetimi | **PASS (sentetik)** | `sensitivity -- synthetic` iki kez: iki koşu ve commit'lenmiş `T3/sensitivity.{json,md}` SHA-256 eşit (`0c681e…`, `522d0f…`); `important`: doğum 906, yalnız pozisyon/niyetle geçen doğum **0** (K42 öncesi 1229/2143); seviyeyi aşan olaylar yalnız `invalidated` 617 + `weakened` 379. Gerçek veri sürümü yeniden koşulmadı (yerel veri temiz ağaçta yok, bilinçli). |
| 19 | `fixtures/local` ve ekran görüntüleri | **PASS** | `git ls-files \| grep fixtures/local` boş. R4'te eklenen 117 PNG (golden/demo 16, views 2, M8 24, M9 18, M10 48, M11 9) örneklenerek bakıldı: hepsi `SYN..` ve "Sentetik veri". Gerçek sembol adları yalnız toplu istatistik dosyalarında (`T3/sensitivity-real*.json`: evren listesi + sayımlar; R3'te de aynı uygulama). |
| 20 | Gizli bilgi taraması, eski klasörler | **PASS** | `git grep` ve `git log -p main..HEAD` anahtar/belirteç kalıpları (Anthropic, GitHub PAT, AWS, özel anahtar, Slack, JWT, `token/secret = "…"`): 0. `.env.local` hiçbir commit'te yok (`git log --all -- .env.local`); izlenen tek `*.env` `scripts/qapsule-gh.env` (gizli bilgi içermez). `git diff main -- rounds/R0 R1 R2 R3 RD1 RP1 RP2`: boş. |

## GOAL — 15 kabul ölçütü (denetçinin hükmü)

| # | Hüküm | Gerekçe |
|---|-------|---------|
| 1 | ✅ (bulguyla) | Hız/duraklat/adım/tarihe git, belirleyicilik, yansı yolu (verify 15) tuttu. `qp replay` geri seek sonrası boş (Ö3). Yerel gerçek veri bu denetimde koşulmadı. |
| 2 | ✅ | Tek komut, sentetiği ekranda ve konsolda söylüyor; ağ testi + benim kaydım: yalnız kendi kökeni. `--lan` ve telefondan açma denenmedi (yönerge ekranda ve DEVICE.md §4'te var). |
| 3 | ✅ (kullanılabilirlik sorunlarıyla) | Bütün işlevler çalıştı (aşağıda). Hassasiyet seçimi varsayılan gösterimde görünür bir şey değiştirmiyor (Ö4). |
| 4 | ✅ | Kendi taramam + WASM'sız istemciyle 716 çerçeve bayt eşit. |
| 5 | ✅ | Gönderilenler `input` (`tap_at`, `drag`, `pinch`, `open`, `close`, `style`); belge özeti dokunma/sürükleme/tekerlek/kanıt boyunca aynı (`7fe48824b040a567`, `rawparity/A-digests.json`); hareket kare süreleri ölçüm ekranında. |
| 6 | **KISMİ** | Yüz M8 ile seçildi, başlık şablonlu, iki dilli, takvime göre (21/21), FACE.md ve altınlar var. **Ayrıntı cümlesi** şablon dilbilgisinin R4 kurallarına uymuyor (bar sayısı, iç terim) ve yüzle çelişebiliyor (Ö1, Ö2). |
| 7 | ✅ (notla) | Karne yönlü, olumsuz geçiş seviyeden bağımsız, işaret geçerliliği gösteriyor, koşul/çözüm ayrı, değişenler listeli, benzetim belirleyici. Ayrıntı derinliğinde kapanmış kapsül için "artık geçerli değil" (Ö2). |
| 8 | ✅ (notla) | Sıra ve körleme kayıtlardan doğrulandı; E6 zamanlaması (Ö7). |
| 9 | ✅ | Ekran, özet, 1×/4×, yönerge, K17/K45 önerisi; sayılar ±%20. Telefon ölçümü Arkin'de. |
| 10 | ✅ | 82,6 ms; ilk kapsül 115–410 ms; WASM ölçülmüş; ayrı yükleme değerlendirilmiş. |
| 11 | ✅ | verify 18; depodaki görüntüler sentetik. |
| 12 | ✅ | Paket var; alfabe/yüz ilk ikisi Ayarlar'da ve URL'de; bağlantılar ayarları URL ile verdiği için dil değişiminde sıfırlanma (Ö5) testi etkilemez. Paketin içeriğini derin denetlemedim. |
| 13 | ✅ | Dökümler 44, 48, 57, 58; PROTOCOL §20.3; ergonomi 14/20, 17/20 (R3 12/20). Ajan koşusunu tekrarlamadım. |
| 14 | ✅ | 58 × 3 taşıma; yeni 45–58. |
| 15 | **EKSİK** | AUDIT (bu dosya), FACE.md, DECISIONS "Önerilen (R4)" var; REPORT commit edilmemiş ve yer tutuculu; STATUS/ROADMAP güncellenmemiş. |

## Bulgular (önem sırasıyla)

### Engelleyici

Yok.

### Önemli

**Ö1 — Ayrıntı cümlesi R4 dil kurallarına uymuyor (GOAL 6).** Kapsüle dokununca üstte (HTML) ve sahnenin
içinde (iki kez) R3 cümlesi çıkıyor: "SYN26 85 bar önce 110,95'teki desteği aşağı kırdı (kapanış 108,38): aşağı
kırılım. Sinyal 83 bar önce geçersiz oldu; artık geçerli değil." / "SYN12 3 bar önce … Bu barda aynı aileden
yeni bir kanıtla güçlendi." EN: "85 bars ago, … It was invalidated 83 bars ago". GOAL 6 "başlık ve ayrıntı
cümleleri … süre ifadesi takvime göre" diyor; K43 iç terimleri ("aile", "bar") başlık için yasaklıyor.
Kanıt: `audit/k42/k42faces.log` (6 kapsül × TR/EN × iki tema), `audit/shots/05-kapsul.png`, `06-grafik.png`.
REPORT taslağı ölçüt 6'yı ✅ gösteriyor ve bundan söz etmiyor (Öneriler'de yalnız "; artık geçerli değil" eki var).

**Ö2 — Yüz ile ayrıntı aynı kapsül için farklı şey söylüyor.** (a) `cap.SYN26.breakout.down.47`: yüz
"Kapandı: 10 Mart'ta tutmadı." (✗), ayrıntı "Sinyal 83 bar önce geçersiz oldu; artık geçerli değil." — yüzün
sözlüğünde "artık geçerli değil" açık ama tutmayan kapsülün sözü. (b) `cap.SYN12.breakout.down.129`: yüz "dün
güçlendi", ayrıntı "Bu barda … güçlendi" (adım 120, 2023-07-06 12:00). İkisi de K42/K43'ün "işaret ve cümle aynı
şeyi söyler" ilkesinin derinlikler arasında da beklenen hali. Kanıt: `audit/k42-sheet.png`, `audit/k42/k42faces.log`.

**Ö3 — `qp replay` geri sarmadan sonra hiçbir şey yazmıyor; VERIFY 16 bunu görmüyor.** Geri `seek` otoriteyi
yeniden kurup bağlantıları kapatıyor (Ö39); `qp replay`'in izleyen bağlantısı yeniden `hello` demiyor:
sonraki her denetim `{"code":"handshake","message":"the first message on a connection must be hello"}` ile
dönüyor ve veri satırı gelmiyor. Benim dizimde 400 adımın 146 satırı; 4 `handshake` hatası (satır 143–146).
verify adım 16'nın denetim dizisinde geri seek **sonuncu** (`at 120 → step 30`), yani 300 adımlık koşunun son
180 adımında karşılaştırılan akış boş. Tarayıcı gösterimi toparlanıyor (120 → 40 → +1: epoch 1, 40 kapsül,
adım 41; `audit/k42/k42faces.log`, `k42/backward-seek.png`). Komut:
`qp replay --source synthetic --seed 7 --instruments 12 --days 300 --steps 400 --controls rounds/R4/audit/scripts/replay-controls.json`.

**Ö4 — Hassasiyet seçimi varsayılan gösterimde etkisiz görünüyor.** Kitle verilmemiş açılışta `important`,
`balanced`, `all` üçü de 40 kapsül (akışın sayfa sınırı 40). Kitle verilince çalışıyor: `future` grubu + pozisyon
ile 27 / 38 / 40, varsayılan 27 (`audit/scripts/feedorder.mjs`). Kullanıcı (ve insan testi katılımcısı) seviyeyi
değiştirince hiçbir şey olmadığını görür; `all`'ın 40'ta kesilmesi de seviyeler arasındaki farkı gizler.

### Küçük

- **Ö5 — Dil ya da "veri yetkisi yok" değişince Ayarlar'dan seçilen yüz, alfabe ve son bakış kayboluyor.**
  Önce `{"face":"fotolay","alphabet":"rozet",…,"seen":…}`, EN'e geçince `{"face":"vadrel",…}` (alfabe, son bakış
  yok); tema ve kitle kalıyor (`audit/explore.log`). Nedeni: kabuk yeniden bağlanıyor, yalnız URL ayarlarını
  yeniden uyguluyor. İnsan testi bağlantıları ayarları URL ile verdiği için testi etkilemez.
- **Ö6 — Tutmadı-kapandı işaretinin gövdesi ≈ 1,1–1,4:1** (soluk iz, `tg5`); raporlanan "bilinen açık 2,06"
  yalnız gerçekleşti-kapandı gövdesi. Anlamı taşıyan ✗ köşesi 3,8 (açık) / 4,5 (koyu) ile eşiği geçiyor; yön
  kapanmış kapsülde işaretten neredeyse okunmuyor (cümlede var). A31'e eklenmeli.
- **Ö7 — SPEC E6 zamanlaması.** E6 (`fa07648`, 16:39; "hiçbir alternatifin geçemediği ve en kötü değeri aynı olan
  eşik uygulanmaz") `tavrik` (16:30) ve `vadrel` (16:35) teslim edildikten sonra, `fotolay`'dan (16:46) ve bütün
  okumalardan önce yazıldı. E6 tasarımlar arasında ayrım yapmıyor (hepsinde 2,06), sonucu değiştirmiyor; "ölçüt
  alternatiflerden önce" ilkesine harfiyen uymadığı yine de kayda geçmeli.
- **Yetkisiz okuyucu akışı:** 40 satırın hepsi aynı cümle, EN'de büyük harf hatası: "…downside breakout signal for
  **This stock**; the details require a data license." Kanıt ekranında ise sembol açık ("This evidence requires the
  entitlement: SYN26"); akış sembolü saklarken kanıt söylüyor (`shots/16`, `17`). Zaman/geçerlilik yokluğu bilinen A32.
- **Kanıt ekranı ham:** TR arayüzde İngilizce iç anahtarlar ve biçimsiz sayılar: "price / atr 1.39 / vol_avg
  1475274787.72 / volume 9219577515.37 / Dayandığı: bars SYN26 1d (feed default)" (`shots/08`). Cümlede ondalık virgül
  ("110,95"), grafikte ve kanıtta nokta ("108.38", "95.67").
- **Akışın başı eski kapanmış kapsüller:** adım 120'de (2023-07-06) ilk ekran Mart'ta kapanmış iki `SYN26` kapsülü ve
  19 işlem günü önce kapanmış bir kapsül ("geçmişte en çok tutan önce"); pozisyon verilince "Elindeki hisseler"
  bölümü 13 ardışık "tutmadı" işaretli geçmiş kapsülle başlıyor (`feedorder.mjs`). REPORT açık soru 10 ile aynı yön.
- **Kapsül derinliğinin grafiği olayı göstermeyebiliyor:** `SYN26` kapsülünün küçük grafiği 2023-05'ten başlıyor, olay
  Mart'ta; seviye (110,95) ve olay barı görünmüyor (`shots/05`); grafik derinliğinde görünüyor (`shots/06`).
- **verify 18 gösterim paketini adlı (soyulmamış) derliyor ve `shells/web/pkg`'a yazıyor:** 5.645.727 B (demo.sh'ın
  4.377.216 B'ı yerine), `sizes.json`'da `js_*` yok. `demo.sh` paketi yalnız `.rs` daha yeniyse yeniden derlediği için
  verify'dan sonra açılan gösterim 5,6 MB WASM sunar ve Ölçüm ekranındaki paket boyutu/açılış süresi DEVICE.md ile
  karşılaştırılamaz (Arkin'in telefon ölçümünden önce `QP_DEMO_REBUILD=1` gerekir).
- Son bakış işaretlendikten sonra 2,5 sn oynatmada "son bakıştan beri" bölümü görünmedi; o aralıkta yeni kapsül doğup
  doğmadığını ayrıca doğrulamadım (doğrulanmamış gözlem).
- Yüzlerin içinde metnin altında geniş boşluk (188 px yükseklik, ≈120 px metin), ekran başına ≈3,5 kapsül
  (`shots/03`).

## Gösterim — kullanıcı gibi kullanım (VERIFY 6)

`audit/scripts/explore.mjs` (sentetik, duraklatılmış açılış, `?anim=0`); ekranlar `audit/shots/01…17`.

İşe yarayanlar: açılış 301 ms'de oturdu, "Sentetik veri" bandı; ▶ 4 sn → adım 16, ❚❚; +1; tarihe git alanına adım
(120) yazınca gidiyor; akış tekerlekle kaydı (scrollTop 1800); yüze dokunma → Kapsül derinliği, sahneye dokunma →
Grafik; sürükleme ve tekerlek sahneyi değiştirdi; **Kanıt** → değerler (yetkili); ‹ Geri üç kez → cümle; Ayarlar'da
seçenekler çekirdekten (yüz `r3, vadrel, fotolay, tavrik`; alfabe `siluet, rozet`; akış `r3, elikov, vakiro`; geçiş
`r3, tutamir, zumlak`; sonuç `r3, muhrak, tg5`); izleme listesi + pozisyon uygulanınca akış 40 → 27, "Elindeki
hisseler" bölümü; son bakış bildirimi; açık tema + `rozet` + `fotolay` (`shots/12`, `13`); EN arayüz ve cümleler
(`shots/14`); Ölçüm sekmesi (`shots/15`, `audit/metrics-desktop.json`); yetkisiz kipte akış düz metin, kanıtta
"This evidence requires the entitlement: SYN26" (`shots/16`, `17`); geri sarma toparlanıyor. Sayfa hatası 0.

Bozuk ya da kafa karıştırıcı olanlar: Ö1, Ö2, Ö4, Ö5 ve "Küçük" başlığındaki gösterim maddeleri.

## M8 tekrarı (VERIFY 13) — farklar

| Yüz (kod) | Özgün r1 | Tekrar | Değişen cevaplar |
|---|---:|---:|---|
| `vadrel` (`262bbace`) | 3/6 | 3/6 | C S1 yukarı → belirsiz; F S4 "hiçbir bilgi göremedim" → "az örnek var" |
| `fotolay` (`37da4c4c`) | 3/6 | 2/6 | C S1 yukarı → belirsiz; D S1b "hareket sürüyor" → "anlamadım" |
| `tavrik` (`be730d1e`) | 3/6 | 2/6 | C S1 yukarı → belirsiz; D S1b ters yönde; D S5 "hiçbir şey" → "bilmiyorum" |

Üç tekrarda da C kartının (durum 7, "büyük alım ya da satım var") yönü "belirsiz" okundu: akış olayının yönü üç
tasarımda da başlıktan çıkmıyor (tasarımları ayırmıyor). D kartı (yetkisiz) hep tahmin (bilinen #9). Sıralama ve ilk
iki raporla tutarlı; fark tek okuyuculuk gürültü düzeyinde.

## Denetçinin dosyaları

- `rounds/R4/audit/scripts/` — `explore.mjs` (kullanım + ağ), `rawparity.mjs` (kabuksuz eşitlik), `k42faces.mjs`
  (yüzler TR/EN × tema, ayrıntı cümlesi, geri sarma), `feedorder.mjs` (akış düzeni, hassasiyet), `contrast.py`,
  `replay-controls.json`.
- `rounds/R4/audit/probe/` — `elapsed_probe` (çalışma alanı dışı crate; `CARGO_TARGET_DIR=<geçici> cargo run --release
  --manifest-path rounds/R4/audit/probe/Cargo.toml --bin elapsed_probe`), çıktı `elapsed_probe.out`.
- `rounds/R4/audit/shots/`, `k42/`, `k42-sheet*.png` — yalnız sentetik.
- `rounds/R4/audit/rawparity/` — sonuç, gönderilen POST'lar, özetler; çerçeve dökümleri (2 × 4,8 MB) silindi, SHA-256'ları
  `frames.sha256`'da (start çerçevesindeki belirteç farklı olduğundan iki dosyanın özeti farklı; karşılaştırma
  belirteç normalize edilerek yapıldı).
- `rounds/R4/audit/m8-repeat/` — üç okuyucunun istemi, cevabı, meta, puanlar.
- `rounds/R4/audit/device-synthetic.log`, `capsule_tick.log`, `metrics-desktop.json`, `explore.log`.
