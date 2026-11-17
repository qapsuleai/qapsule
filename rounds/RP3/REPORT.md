# RP3 — Rapor: Telefonda Çizim ve Açılış

Durum: kabul ölçütleri **10 ✅ / 2 ⚠️** (1: kalibrasyon yüzde tutuyor, sahne ve tikte tutmuyor;
3: grafik açıkken JIT'siz telefon benzetiminde tik p95 11 ms > 8 ms) — ayrıntı aşağıda. Bütün
sayılar benzetimdir; seçim Arkin'in telefon karşılaştırmasına kadar **öneridir** (`DECISIONS.md`
"Önerilen (RP3)", Ö52–Ö60). Bağımsız son denetim: `AUDIT.md`.

## 1. Özet

1. **Telefonun yavaşlığının asıl nedeni bulundu: iPhone ölçümü JIT'siz yapılmıştı.** iOS 27,
   güvenli olmayan (`http://`) sayfalarda JS ve WASM'ı yorumlayıcıyla çalıştırıyor (WebKit hata
   kaydı 324968, WebKit mühendisleri: "kasıtlı", "Enhanced Security"; Safari ve iOS için Chrome
   aynı). Arkin'in ölçümü `scripts/demo.sh --lan` ile `http://192.168…` üzerinden alınmıştı.
   Masaüstü → telefon farkının (≈30–100 kat) payları: JIT'in kapalı olması %55–72, piksel sayısı
   (DPR 1→3) %32–34, açıklanamayan kalıntı sahnede %14 (`CALIBRATION.md`). R4'ün 4×/6× CPU
   kısıtı yanlış mekanizmayı taklit ediyordu.
2. **Cihaz artık görüntüleyici (K48, Ö52).** Akış kapsülleri sunucuda derlenmiş yüzlerle,
   "Sunucu doğruladı" güveniyle gelir; cihaz yalnız açık enstrümanın verisine bağlanır ve
   hikâyeyi kendi barlarında sınar ("Bu cihazda sınandı"). Cihazın tik işi ve belleği 30, 300 ve
   3.000 enstrümanda aynı (±%20); tam yansı 3.000'de 1,77 GB'a çıkıyordu.
3. **Boyama yolu: `canvas` önerildi (Ö53).** Üç yol ayrı ajanlarla, birbirini görmeden kuruldu
   (yazılım `cpu`, tarayıcı tuvali `canvas`, GPU `gpu`); seçim kalibre benzetimde ölçümle.
   JIT'siz telefon benzetiminde (`PHONE`) yüz 32 → 2 ms, sahne 24 → 7 ms, grafik açıkken
   kare başına iş 19,6 → 7,3 ms (p95 11 ms). JIT açıkken (HTTPS'teki iPhone tahmini) her şey
   1–2 ms. `gpu` en az onun kadar hızlı ama güvenli bağlam ister — Arkin'in bugünkü `http`
   kurulumunda hiç çalışmaz.
4. **Açılış:** sıkıştırılmış paket ve WebKit'in WASM'ı iki kez indirmesinin giderilmesi;
   bayt 8,85 MB (WebKit) → 1,33 MB; ilk kapsül tahmini ≈0,57 s (hedef 1,5 s).
5. **Ölçüm ekranı düzeltildi ve tek dokunuşla karşılaştırma eklendi** (her yol, dört evre,
   < 3 dk); özet artık sayfanın güvenli bağlamda olup olmadığını ve JIT'in açık görünüp
   görünmediğini de söyler.

## 2. Kabul ölçütleri

| # | Ölçüt | Durum | Kanıt |
|---|-------|-------|-------|
| 0 | İş bölümü (K48) | ✅ | `experiments/M15.md`, `M15-verify.md`. Görüntüleyici varsayılan, tam yansı `?mode=replica`. 30 / 300 / 3.000 enstrüman: tik işi (akış) 1,66 / 1,72 / 1,81 ms/adım, (açık) 5,26 / 5,58 / 5,77; WASM 13,8 / 16,4 / 16,6 MB (+%20, tek bellek büyüme adımı; denetçi yeniden üretti). Akış ekranında bar aboneliği yok; açıkken tek besleme (VERIFY 17). Güven işaretleri metinle ayırt ediliyor. `qp replica-tape --viewer` gecikme 0/25: ihlal 0, son iddialar 4/4; RP2/R3 şeritleri ve itiraz testleri geçiyor; viewer şeritleri `verify.sh` `replica` adımında (M14). Protokol eki PROTOCOL §22; yeni döküm `59-viewer.json`; var olan 58 döküm bayt aynı. İki test sabiti `describe`'ın yeni konuyu listelemesi yüzünden değişti (denetçi `viewer` çıkarılınca eski değerlerin döndüğünü gösterdi). |
| 1 | Kalibrasyon | ⚠️ | `CALIBRATION.md`, `T1-verify.md`. Seçilen ayar `PHONE` (WebKit 26.5, JIT kapalı, DPR 3): yüz 31 / iPhone 32 ms (×0,97 ✓), sahne 38 / 71 (×0,54 ✗), tik 12 / 74 (×0,16 ✗; tik, sahnenin kaç tikte yeniden boyandığına bağlı — telefonda hangi kapsülün açık olduğu bilinmiyor; yalnız sahne boyanan tiklerde 45 / 74 = ×0,61). Hiçbir tek masaüstü ayarı üçünü birden ±%30'a sokmadı (CDP 16× yüz ve sahneyi tutturuyor ama yanlış mekanizmayla). Nedenler ayrıştırıldı ve payları ölçüldü (yukarıda). R4 `DEVICE.md` eşlemesi düzeltildi ("RP3 düzeltmesi"). |
| 2 | Üç boyama yolu | ✅ | `experiments/M12a.md`, `M12b.md`, `M12c.md` (dallar `rp3/m12a-cpu`, `rp3/m12b-canvas`, `rp3/m12c-gpu`; ayrı ajanlar, ortak yönerge diğer yolları anlatmıyordu). Düzenek (T2: arayüz, karşılaştırma) yollardan önce `main`'e girdi. Görsel fark: `cpu` 0 piksel (49/49 her motorda); `canvas` > 2 seviye en çok %0,30 (DPR 3) / %1,06 (DPR 1); `gpu` en çok %0,31, düz alanlarda 0. Hepsi aynı `SceneDesc`'i boyuyor. |
| 3 | Hedefler, kalibre benzetimde | ⚠️ | `experiments/S.md` §6. Görüntüleyici + `canvas`, `PHONE`: tik+boyama p95 grafik açıkken **11 ms (hedef 8 ✗**, ortalama 7,3 ✓), akış ekranında 6 ms ✓; kaydırma karesi p95 18 ms (60 Hz + WebKit'in 1 ms zamanlayıcısı; kayıp kare yok, uzun pay %0,2–0,4); ilk kapsül ≈0,57 s ✓ (tahmin: döngü adresinde 177 ms + 1,55 MB aktarım + 4 gidiş-dönüş). `PHONE_BIAS` ile grafik açık p95 ≈18 ms. `PHONE_JIT_EST`'te tik p95 2 ms ✓. Kalan maliyetin dökümü ölçüldü: sahne kurma 3,2 + değişim sınaması 0,7 + boyama 0,3 + kabuk tarafı ≈2,4 ms/kare; gereken: değişen bar sahnenin aralığı dışındaysa sahneyi kurmamak (A36, ≈3 ms'ye), değişmeyen katman metnini yeniden kurmamak, ya da HTTPS (JIT). |
| 4 | Varsayılan ve adres parametresi | ✅ | `canvas` varsayılan; `?paint=ref|canvas|cpu|gpu`; desteklenmeyen/bilinmeyen yol `ref`'e düşer ve özet nedenini yazar. `tests/paint-paths.mjs` (verify `demo` adımında): her yol açılıyor ve boyuyor; WebGPU yokken `gpu` → `ref` (`unsupported`). |
| 5 | Açılış | ✅ | `experiments/M13.md`, `M13-verify.md`. Önce/sonra (DPR 3, 5 koşu medyanı): Chromium 4× + 40 Mbit/20 ms soğuk ilk kapsül 1.907 → 1.230 ms (br), aktarılan 4.472.496 → 935.707 B; sıcak 1.775 → 892 ms; WebKit 8.850.801 B (WASM iki kez) → 1.334.452 B (gzip). Barındırıcı `Accept-Encoding`'e göre br/gzip veriyor (test `crates/qp-cli/tests/demo_static.rs`, tarayıcı testi `tests/compress.mjs`). |
| 6 | Güncelleme sıklığı | ✅ | `experiments/M14.md`. Kare başına en çok bir boyama geçişi (veri çerçevesi başına 1,00–1,13), çerçeveden boyamaya ≤ 1 kare; görünmeyen ve katman altındaki yüzler boyanmıyor (karar çekirdekte, sayaçlar özette). |
| 7 | Bellek | ✅ | `experiments/T4.md`, `T4-verify.md`. 5.000 adım: sürekli oturumda WASM 17,5 → 102 MB (≈17,6 KB/adım; takipçi günlüğü %60, doküman %15, okuma kopyası ≈%15) — tam yansıya özgü, görüntüleyicide cihazdan kalkar; karar A34. Masaüstündeki 152 MB'ın kaynağı yeniden başlatmalardı (başlangıç çerçevesi JSON ağacı ≈35× metin, eski çekirdek bırakılmıyordu): tipli okuma + önce bırakma ile 2.500. adımda 331 → 61 MB. |
| 8 | Cihazda otomatik karşılaştırma | ✅ | Ölçüm sekmesinde **Karşılaştır**: desteklenen her yol aynı senaryoda (durağan + tik, otomatik kaydırma, grafik açık + tik, otomatik sürükleme), sürücü çekirdekte, elle hareket yok; tek özet (**Karşılaştırmayı kopyala**). Süre yol başına ≈34 s (4 yol ≈136 s, `http`'de 3 yol ≈102 s), 175 s'de zorunlu durma. `tests/compare.mjs` Chromium + WebKit başsız: 139 s (denetçi 140 s). |
| 9 | Ölçüm ekranı hataları | ✅ | `experiments/T2.md`: tazeleme hızı algılanıyor, uzun kare > 1,5 × aralık (60 Hz tamsayı dizisi → 0); açılış işaretleri ilk değer, yeni çekirdeğe taşınıyor (836.034 ms'nin nedeni: her başlangıç çerçevesinde yeni çekirdek); DPR < 1 / kesirli / ölçek farkı uyarısı. |
| 10 | Değişmeyenler | ✅ | `verify.sh` 18/18 (altınlar bayt eşit, 59 döküm × 3 taşıma, hesap özeti `7b1e99d40889f83c`, mimari testi 0 ihlal). Anlamsal okuma yoldan bağımsız: `paint-paths.mjs` özetleri, akışı, açık durumu ve sunucunun `tap_at`/`screen` cevaplarını `ref` ile bayt karşılaştırıyor. Toleranslı ekran testi seçilen yol için `tests/paint-canvas.mjs` (verify `demo` adımında). |
| 11 | Arkin için yönerge | ✅ | §8. Seçim `DECISIONS.md` "Önerilen (RP3)". |
| 12 | Belgeler | ✅ | `REPORT.md`, `RESEARCH.md`, `CALIBRATION.md`, `AUDIT.md`, `STATUS.md`, `ROADMAP.md`; görev notları `experiments/`. |

## 3. Ne yapıldı (görevler, dallar)

| Görev | Dal | Not | Denetim |
|---|---|---|---|
| T1 kalibrasyon | `rp3/t1-calib` | `CALIBRATION.md`, `shells/web/tests/calibrate.mjs` | `T1-verify.md` (A koşullu: resmi Apple belgesi yok, aynı telefonda HTTPS ölçümü yok) |
| T2 ölçüm ekranı, boyama arayüzü, karşılaştırma | `rp3/t2-harness` | `experiments/T2.md` | `T2-verify.md` PASS |
| T3 araştırma | — | `experiments/T3-research.md` → `RESEARCH.md` | — |
| M15 görüntüleyici | `rp3/m15b-viewer` | `experiments/M15.md` | `M15-verify.md` PASS |
| M13 açılış | `rp3/m13-open` | `experiments/M13.md` | `M13-verify.md` PASS |
| T4 bellek | `rp3/t4-mem` (+ `rp3/s-integrate`) | `experiments/T4.md` | `T4-verify.md` PASS (ilk yerleşimde koşullu; düzeltildi, yeniden ölçüldü) |
| M14 güncelleme sıklığı | `rp3/m14-freq` | `experiments/M14.md` | `S-verify.md` |
| M12a/b/c boyama yolları | `rp3/m12a-cpu`, `rp3/m12b-canvas`, `rp3/m12c-gpu` | `M12a.md`, `M12b.md`, `M12c.md` | `S-verify.md` |
| S bütünleştirme, birlikte ölçüm | `rp3/s-paint` | `experiments/S.md` | `S-verify.md` |
| HTTPS vekili | `rp3/https-host` | `qp demo --https-host` | `S-verify.md` |
| A denetim | — | `AUDIT.md` | bağımsız |

## 4. Ölçümler (seçilen yol, birlikte)

`experiments/S.md`, `S/tables.md`. Görüntüleyici kipi, sentetik piyasa, DPR 3, 402×874, evre 8 s,
ileri ve ters yol sırası × 3 koşu (sıra etkisi yok), yük ortalaması 3,6–13,5.

| Ayar | Yol | yüz p50 | sahne p50 | tik+boyama/kare (akış / grafik) | sürükleme karesi p95 (uzun pay) |
|---|---|---:|---:|---|---|
| PHONE | ref | 32 | 24 | 5,6 / 9,4 ¹ | 29 (0,263) |
| PHONE | **canvas** | 2 | 7 | 1,3 / **7,3** ¹ (p95 11) | 19 (0,002) |
| PHONE | cpu | 26 | 7 | 3,2 / 8,8 | 18 (0,008) |
| PHONE | gpu | 3 | 8 | 2,3 / 14,9 | 18 (0) |
| PHONE_JIT_EST | canvas | 1 | 1 | 0,48 / 0,8 ¹ | 18 (0) |
| Chromium 4× | canvas | 2,1 | 1,2 | 0,49 / 2,07 | 9,2 (0) |

¹ Değişim sınaması parmak iziyle (son ölçüm); diğer satırlar JSON özetiyle alınmıştı (`S.md`
§6 önce/sonra). Bellek: `cpu` +33 MB WASM; `canvas` ve `gpu` `ref` üstüne bir şey eklemiyor.
Paket: birleşik WASM gzip 1.463.230 B (+29.961), JS toplam gzip 30.513 B (+10.321).

## 5. Sapmalar ve yapılamayanlar

1. **Kalibrasyon yalnız yüz süresinde tutuyor (ölçüt 1 ⚠️).** Sahne telefonda masaüstü
   motorlarının hepsinden orantısız pahalı (sahne/yüz telefonda 2,2, benzetimde 1,2–1,4); p95
   dağılımı telefonda yaklaşık iki kat geniş. Nedeni telefonsuz ayrıştırılamadı (bellek, tuval
   yüklemesi, ısı). Kararlar ham `PHONE` sayılarıyla verildi; yanlılık `PHONE_BIAS`.
2. **Grafik açıkken tik p95 hedefi JIT'siz benzetimde tutmuyor (ölçüt 3 ⚠️):** 11 ms > 8 ms.
   Gereken ölçümle gösterildi (§2 satır 3); en büyük kalem her veri çerçevesinde sahnenin yeniden
   kurulması (3,2 ms) — oluşan barın her adımda değişmesi gerçek, ama sahnenin onu çizip çizmediği
   derleyicinin bilgisi (A36). HTTPS'te (JIT) 2 ms.
3. **M15c denenmedi** (yüz için veri dilimi, cihazda derleme; A38).
4. **Gerçek telefonda hiçbir şey ölçülmedi.** `gpu` yolunun telefondaki GPU ve kopya maliyeti,
   `canvas`'ın tarayıcı içi tarama süresi (GPU sürecinde; benzetim süresine girmiyor) ve WebKit
   atlas renk hatasının iPhone'da olup olmadığı bilinmiyor.
5. `gpu` yolu verify'ın başsız Chromium'unda çalışmıyor (bağdaştırıcı yok); verify yalnız
   yedeğini sınıyor. WebKit'te ve tam Chromium'da elle çalıştırıldı.
6. Paylaşımlı makine: yük ortalaması 4–55 arasında oynadı; bir kez disk doldu (ENOSPC;
   yalnız birleştirilmiş ve eski worktree'lerin `target/` klasörleri silindi). Ölçüm tablolarında
   yük yazılı; karar ölçümleri sıralı alındı.

## 6. Açık sorular (Arkin'e)

1. **Telefon hedefleri hangi kipte?** HTTPS (JIT açık; Safari'nin normal hâli, ürünün gerçek
   dağıtımı) mı, JIT'siz en kötü durum (http, Kilit Kipi) mı, ikisi ayrı mı? Ö55 ikisini ayrı
   yazmayı öneriyor.
2. Yerel gösterim için HTTPS yolu: `--https-host` ile bir vekil (ör. Tailscale `serve`) yeterli
   mi, yoksa barındırıcıya yerel sertifikalı TLS mi eklensin (yeni bağımlılık)?
3. `cpu` yolunun "hareket sırasında ölçek 1" bulanıklığı ürün olarak kabul edilebilir mi
   (yalnız `?paint=cpu`'da)?
4. K12'nin vello kısmı yerine Ö54 (yalın WebGPU) ve WebGL2 arka ucu (A37) değerlendirilsin mi?

## 7. Öneriler (kapsam dışı, yapılmadı)

- A36: derleyici sahnenin okuduğu bar aralığını bildirsin; değişmeyen sahne kurulmasın.
- Değişmeyen açık katmanın metin ve kanıt DOM'unu her geçişte yeniden kurmamak (≈2,4 ms/kare).
- A34: tam yansıda günlük ve kapanmış hikâye politikası; A35: açık dilimin artımlı gönderimi.
- Gösterim paketini bölmek (görüntüleyici paketi kapsül motorunu taşımasın; ilk paket ≈0,54 MB
  gzip tahmini, M13 (d)).
- `tests/paint-canvas.mjs`'i WebKit'te de verify'a almak; `compress.mjs`, `gpu.mjs` için port
  seçeneği ve verify bağlantısı.
- `qp-session/src/conform.rs:313` wasm32 hedefinde clippy uyarısı (verify o hedefi denetlemiyor).
- Sözlüğe (`docs/CONCEPTS.md`) "boyama yolu" ve "karşılaştırma sınaması".
- Bir orta sınıf Android ölçümü (`MID_ANDROID_EST` bugün yalnız tahmin).

## 8. Telefonumda karşılaştırmayı nasıl çalıştırırım (Arkin için)

Süre: kurulum 2 dk, ölçüm ≈2–3 dk. Telefon şarjda değil ama pil %50'nin üstünde; Düşük Güç
Kipi kapalı.

### A. Bugünkü gibi (http, aynı Wi-Fi) — JIT'siz en kötü durum

1. Bilgisayarda: `scripts/demo.sh --lan --synthetic`. Konsol bir adres yazar:
   `telefondan aç / open on your phone (same network): http://192.168.x.y:8390/`.
   (macOS "gelen bağlantılara izin verilsin mi?" diye sorarsa **İzin Ver**; ayrıntı
   `rounds/R4/DEVICE.md` §4.)
2. Telefonda o adresi **olduğu gibi** açın (Safari ya da Chrome), 10 sn bekleyin.
3. Alt sekmelerden **Ölçüm**'e geçin, en alttaki **Karşılaştır** düğmesine **bir kez** dokunun.
   Ekrana dokunmayın: sayfa kendisi akışı oynatır, kaydırır, kapsülü açar ve sürükler; her
   boyama yolunu sırayla dener (http'de `gpu` desteklenmez, bu yüzden 3 yol, ≈1 dk 45 sn).
4. Bitince tablo görünür ve **Karşılaştırmayı kopyala** düğmesi çıkar; dokunun ("Kopyalandı";
   pano izni yoksa metin seçili gelir, elle kopyalayın).
5. Özetin `mode` satırı `secure_context:false` ve büyük olasılıkla `jit: likely_off` der — bu
   beklenen durumdur.

### B. HTTPS ile (JIT açık, WebGPU var) — ürünün gerçek hâli

Bir HTTPS vekili gerekir. Öneri: Tailscale (bilgisayara ve telefona kurulu, aynı hesapta).

1. Bilgisayarda gösterimi yerelde başlatın, vekil adını vererek:
   `scripts/demo.sh --synthetic --https-host <bilgisayar-adı>.<tailnet>.ts.net`
2. İkinci terminalde: `tailscale serve --bg http://127.0.0.1:8390`
   (Tailscale'in verdiği `https://<bilgisayar-adı>.<tailnet>.ts.net/` adresini yazar.)
3. Telefonda o `https://` adresini açın; A'nın 3–4. adımları. Bu kez 4 yol (`gpu` dahil),
   ≈2 dk 20 sn. `mode` satırı `secure_context:true` demeli.
4. Sayfa 403 verirse vekil `Host` başlığını korumuyordur (Tailscale'in koruduğunu tek bir
   kaynaktan doğrulayabildik); `--https-host`'a tam olarak tarayıcıdaki adı yazdığınızdan emin
   olun. İşiniz bitince `tailscale serve --https=443 off`.
   **Uyarı:** vekile erişebilen herkes (tailnet'inizdeki cihazlar) gösterimi görebilir.

### Sonucu nereye

Her özeti bir dosyaya kaydedin: `rounds/RP3/experiments/phone/<model>-<http|https>.json`
(ör. `iphone-http.json`, `iphone-https.json`) ya da ana akış oturumuna yapıştırın. Özet yalnız
süre, bayt, sayaç, cihaz bilgisi ve tarayıcı kimliği taşır; piyasa verisi taşımaz (sentetik
piyasa). Mümkünse bir eski Android'de de A'yı yapın.

Bu iki özet gelince seçim (`canvas` varsayılan, Ö53) ve hedeflerin telefon sayıları
kesinleşir; `gpu`'nun HTTPS'te `canvas`'ı geçip geçmediği B'den okunur.
