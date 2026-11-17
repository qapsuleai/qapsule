# Web kabuğu — mimari kuralları (R4/T5)

Sınır `docs/ARCHITECTURE.md` "Kabuk sınırı (R4)". Kabuk (`index.html`, `main.js`,
`style.css`) bayt ve piksel taşır; **alan mantığı Rust çekirdektedir** (`crates/qp-wasm/src/shell.rs`,
`ShellCore`): yansı, gönderilecek her protokol mesajı, hareket tanıyıcı (dokunma / sürükleme /
kıstırma), yüz ve açık kapsül sahneleri, pikseller, oynatma durumu ve kanıt metinlerinin
biçimi, geçiş kutuları, ölçüm istatistikleri.

Denetim: `node shells/web/arch-check.mjs` (`scripts/verify.sh` adım `shell`). Yorumlar ve
dizge içerikleri çıkarıldıktan sonra `main.js` ve (RP3/T2) **her `paint/*.js` boyama yolu
dosyası** aynı kurallarla taranır; kendi kendini sınama örnekleri (her kural için bozuk bir
satır; boyama yolu bağlamında ayrıca bir takım) yakalanmazsa adım düşer.

| Kural | Ne yasak | Neden |
|-------|----------|-------|
| R1 | Dış adres: `http(s)://` (JS, HTML, CSS), CSS `@import`/dış `url()`, başka kökene `fetch`/`EventSource`/`WebSocket` | Sayfa dış ağa çıkmaz (GOAL 2); yazı tipi de yerel |
| R2 | `.sort( .filter( .reduce(`, `toFixed toPrecision toLocale* Intl. Math. Number( parseInt parseFloat Date String( .toString(`, `.join( .concat( .padStart .padEnd .repeat(`, `.slice( .splice( .substring( .substr(` | Sıralama, süzme, sayı/tarih biçimleme, metin kurma, liste kesme çekirdeğin işi |
| R3 | Aritmetik ve birleştirme işleçleri: `+ - * / %` (`++ -- += …` dahil; `=>` serbest) | Kabuk hiçbir değer hesaplamaz: px → bar, ölçüm farkları, geçiş kutuları çekirdekte |
| R4 | Şablon dizgesinde `${…}` | Alan verisinden metin kurma |
| R5 | `core.<ad>` yalnız izin listesindeki yöntemler (aşağıda) | WASM yüzeyi dar ve bilinir |
| R6 | `main.js`'te `./pkg/qp_wasm.js` ve (RP3/T2) `./paint/index.js` dışından `import`; bir boyama yolunda (`paint/*.js`) kardeş dosya (`./ad.js`) dışından `import`; her yerde dinamik `import()`; satır içi betik; `main.js` dışında `<script>`; HTML'de `on…=` | Yüklenen kod yalnız çekirdek paketi, `main.js` ve `paint/` klasörü |

**İstisna listesi:** yok. (Ölçüm süreleri ham `performance.now()` olarak çekirdeğe verilir;
`metric_span(ad, t0, t1)` farkı çekirdekte alır. (R4/H-M8910) Çizim yüzeyi boyutu
çekirdekten gelir: yüz tasarımı kendi yüksekliğini bildirir (`face_size`: CSS boyutu, aygıt
ölçeğinde arka tampon boyutu, CSS uzunlukları), açık sahne `open().px/style`; kabuk
`devicePixelRatio`'yu olduğu gibi çekirdeğe verir (`set_dpr`), ölçeği çekirdek seçer ve o
ölçekte boyar — kabukta çarpma yok. Geçiş kareleri (`transition_frame`) ve ilerleme
(`transition_progress`, ham zaman damgasından) çekirdekte.)

**İzinli çekirdek yöntemleri** (`Shell`, `crates/qp-wasm/src/lib.rs`): `on_frame`,
`take_outbox`, `idle`, `feed`, `face_rgba`, `scene_rgba`, `open`, `evidence`, `status`,
`take_error`, `look`, `instruments`, `audience`, `set_frame`, `replay`, `seek`,
`set_sensitivity`, `set_audience`, `set_style`, `back`, `open_to`, `pointer`, `wheel`,
`transition`, `transition_from`, `metric_frame`, `metric_span`, `metric_mark`, `metric_env`,
`metrics`, `metrics_rows`, `digests`, `png`, `sent`; (R4/H-M8910) `set_look`, `mark_seen`, (R4/S1) `seen_at`,
`open_capsule`, `set_dpr`, `face_size`, `choices`, `transition_progress`, `transition_frame`; (R4/D) `metric_device`, `metric_memory`, `metric_reset` (ölçüm ekranı: cihaz bilgisi, bellek örneği, seriyi sıfırlama); (RP3/M15) `visible` (görüntüleyici: hangi yüzün ekranda olduğunu çekirdeğe bildirir — yüzler otoriteden yalnız ekrandakiler için hazır gelir; karar çekirdekte), `viewer`, `check` (ölçüm düzeneği için: cihazın veri abonelikleri ve aldığı bayt, açık kapsülün kendi verisinde yeniden sınanması).

(RP3/T2) Eklenenler ve gerekçeleri:

| Yöntem | Neden |
|--------|-------|
| `metric_inherit(prev)` | Sonraki bir başlangıç çerçevesi (yeniden bağlanma, dil, geri sarma) yeni çekirdek kurar; ölçümler (açılış zaman çizelgesi, seriler, sürmekte olan karşılaştırma) eskisinden devralınır. R4'te açılış işaretleri dakikalar sonra yeniden alınıyordu (`open_ms.start_frame` 836.034 ms). Kabuk hiçbir değeri taşımaz, yalnız iki nesneyi verir. |
| `free()` | (RP3/T4) wasm-bindgen'in nesne bırakma yöntemi: yeni başlangıç çerçevesinde eski çekirdek ölçümleri devredildikten sonra bırakılır; yoksa bütün yansısı bir JS çöp toplaması onu sonlandırana kadar WASM belleğinde kalır (ölçüm: yeniden başlatma başına ≈30 MB, `rounds/RP3/experiments/T4.md`). Değer taşımaz. |
| `metric_detach()`, `metric_attach(carry)` | (RP3/S) Ölçümler eski çekirdekten çıkarılır, eski çekirdek bırakılır, sonra yeni çekirdek kurulup ölçümleri alır: iki yansı WASM belleğinde aynı anda yaşamaz (bellek tepe değeri geri verilmez; T4-verify: 88 → 62 MB). Kabuk ölçüm değerine dokunmaz; yalnız opak nesneyi taşır. `metric_inherit` testler için kalır. |
| `set_paint(istenen, kullanılan, neden)` | Hangi boyama yolunun kullanıldığı ve neden yedeğe düşüldüğü özetin içinde (özet kendini anlatır). |
| `face_scene(id, force)`, `scene_desc(force)` | Piksel almayan boyama yolları için sahnenin aygıt ölçeğinde **boyama listesi** (`paint/README.md`): bütün sayılar, renk ve yazı tipi dizgeleri çekirdekte hazırlanır; yol yalnız çizim çağrısı yapar (R2–R4 yollarda da geçerli). `face_rgba`/`scene_rgba` ile aynı "değişmediyse boş" kısa devresi. |
| `font()` | Başvuru çizicinin gömülü yazı tipi dosyası (IBM Plex Mono, OFL): tuvalle metin çizen yol aynı dosyayı `FontFace` ile kaydeder (yazı tipi farkı görsel farkın ana kaynağı olmasın; ağ isteği yok, R1). |
| `bench_start(cfg, t)`, `bench_tick(t, scrollHeight, clientHeight)`, `bench_rows()`, `bench_summary()` | Otomatik karşılaştırma (GOAL 8): senaryo sürücüsü (zamanlama, kaydırma konumu, yapay sürükleme, oynatma denetimi, istatistik, tablo metni) çekirdekte; kabuk dönen eylemleri olduğu gibi uygular. Kaydırma sınırı için `scrollHeight`/`clientHeight` ham verilir; çıkarma çekirdekte. |
| `cpu_options(seçenekler)`, `cpu_face(id, force)`, `cpu_scene(force)`, `cpu_moving()`, `cpu_memory()`, `cpu_pixels()` | (RP3/M12a) `cpu` boyama yolu (`paint/cpu.js`): çekirdek her yüzün ve açık sahnenin son karesini tutar, yalnız değişen dikdörtgeni başvuru çizicinin kendi koduyla yeniden boyar ve cevapta piksel göstergesini, kare boyutunu ve değişen dikdörtgeni **hazır** verir (`[ptr, len, w, h, dx, dy, dw, dh, settle, reuse, view]`); kabuk hiçbir sayı hesaplamaz, yalnız `putImageData(img, 0, 0, dx, dy, dw, dh)` çağırır. `cpu_memory` WASM belleğinin kendisidir: kare sınırdan kopyalanmadan bir görünümle okunur (kopya ölçümü için `cpu_pixels`). `cpu_moving`: grafikte sürükleme/kıstırma sürüyor mu (çekirdeğin kendi hareket tanıyıcısı); bitince yol `env.repaint()` ile aygıt ölçeğinde yeniden boyatır. `cpu_options`: ölçüm için teknikleri kapatma (`?cpu=`). |

(RP3/M14) Eklenenler ve gerekçeleri (ölçüt 6: kare başına en çok bir boyama, görünmeyeni boyamamak):

| Yöntem | Neden |
|--------|-------|
| `on_frame(metin, t)` (ikinci değişken eklendi) | Çerçeve hemen çekirdeğe uygulanır; değişiklik bayrakları **çekirdekte** birikir (kabukta birleştirme yok). `t` ham varış zamanı: `connected` işareti ve "çerçeveden boyamaya" gecikmesi çekirdekte hesaplanır. |
| `take_changes()`, `has_changes()` | Animasyon karesi başına tek boyama geçişi: kabuk birikmiş değişiklikleri alır ve bir kez boyar (boşsa hiç). `has_changes` sınama düzeneğinin `ready()`'si içindir: boyama bekliyorken sayfa hazır sayılmaz. |
| `metric_pass(t0, t1)` | Boyama geçişinin süresi (`paint_pass`), sayacı ve gecikme (`frame_to_paint`); kabuk yalnız ham zaman damgası verir. |
| `faces_to_paint()` | Hangi yüzün yeniden boyanacağı çekirdeğin kararıdır: ekrandakiler (`visible`), açık katman akışı örterken hiçbiri; atlananlar çekirdekte sayılır. |
| `metric_flag(ad, açık)` | Sayfanın kipi özetle birlikte: `isSecureContext`, `navigator.gpu` var mı (iOS'ta güvenli olmayan bağlamda JIT ve WebGPU yok). Değer olduğu gibi verilir. |
| `jit_probe()` | Sabit, belirlenimci bir WASM döngüsü; kabuk bir çağrının iki yanını `metric_span('jit_probe', …)` ile verir, sınıflama (JIT açık/kapalı) ve uyarı çekirdekte. |

(RP3/M12b) `canvas` boyama yolu için eklenenler (`paint/canvas.js`; sözleşme `paint/README.md`
"canvas komut akışı"). Hiçbir kural gevşetilmedi: yol akışı bir yineleyiciyle (`ops.values()`)
okur, çünkü dizinle okumak aritmetik ister (R3).

| Yöntem | Neden |
|--------|-------|
| `canvas_face(id, force)`, `canvas_scene(force)` | Sahnenin aygıt ölçeğinde **Canvas 2D komut akışı** (`Float32Array`): her koordinat, kalınlık, renk ve yazı tipi dizinini, glif konumunu, değişen bölgenin kırpma dikdörtgenini çekirdek hazırlar; yol her komutu tek bir tuval çağrısına çevirir. `face_rgba`/`scene_rgba` ile aynı "değişmediyse boş" kısa devresi ve aynı kayıt. JSON v1'e göre ayrıştırma yok, biçem değişimi yalnız değişince, aynı biçemli yollar tek `stroke()`/`fill()`. |
| `canvas_strings()` | Son akışın dizge tablosu (renkler, yazı tipi dizgeleri, metinler/glifler, hizalama adları; JSON dizisi). Akıştaki dizge bağımsız değişkenleri bu tablonun sırasıdır (dizinleme aritmetik değildir). |
| `canvas_config(değer)` | Adresteki `?canvas=` değeri **olduğu gibi** çekirdeğe verilir (deney çeşitleri: `glyphs`, `run`, `nobatch`, `full`, `notext`); ayrıştırma çekirdekte. |
| `canvas_atlas_px()` | Glif atlası kipinde (varsayılan) akıştaki `ATLAS` komutunun yeni atlas satırları (düz RGBA; fontdue kapsaması metnin renginde, başvuru çizicinin kendi glif pikselleri). Yol bunları `ref.js`'in yaptığı gibi `putImageData` ile atlas tuvaline koyar. |

(RP3/M12c) `gpu` boyama yolu için eklenenler:

| Yöntem | Neden |
|--------|-------|
| `gpu_face(id, force)`, `gpu_scene(force)` | Sahneyi çekirdek **üçgenlere** çevirir (`crates/qp-wasm/src/shell/paint_gpu.rs`): kırpma uzayında köşeler, önçarpımlı renk, glif atlası koordinatı; çizgi kalınlığı, birleşim, daire bölümleri, metin yerleşimi (başvuru çizicinin kuralıyla) çekirdekte. `face_rgba` ile aynı "değişmediyse yanlış" kısa devresi ve kaydı. |
| `gpu_vertices()`, `gpu_capacity()`, `gpu_draws(k)`, `gpu_clear()` | Son kurulan karenin köşe baytları, onu tutan tampon boyutu (yol tamponu yalnız sığmazsa büyütür; büyüklük karşılaştırması aritmetik değildir), çizim listesi sütunları (tür, ilk köşe, köşe sayısı; yol `forEach` ile olduğu gibi uygular) ve temizleme rengi. Yol hiçbir sayıyı hesaplamaz, yalnız GPU'ya yükler. |
| `gpu_atlas_size()`, `gpu_atlas_dirty(full)`, `gpu_atlas_rows()` | Glif atlası (fontdue kapsama bitmapleri, başvuru çiziciyle aynı): boyutu, değişen satırların `[y, genişlik, yükseklik]`'i ve baytları. Yeni doku yaratıldığında `full` bütün satırları ister. |
| (WGSL gölgelendirici metni, `paint/gpu.js`) | GPU üzerinde çalışan genel köşe/parça gölgelendiricisi: doku × renk karışımı ve atlas örneklemesi (`floor`, çarpım). Alan mantığı değildir ve JS'te çalışmaz; mimari testi dizge içeriğini taramaz. Gölgelendiriciye alan değeri girmez: köşeler ve renkler çekirdekten hazır gelir. (RP3 denetimi S-verify notu.) |

Boyama yolları (`paint/*.js`) çekirdeğe `core` adıyla ve yalnız bu listeden erişir; boyama
listesindeki ilkelleri verilen sırada, verilen sayılarla boyar; yerleşim, metin kurma,
sıralama yapmaz (sözleşme: `paint/README.md`). `paint/fixture.js` bir sınama demirbaşıdır
(yalnız `?fixture=1` ile desteklenir).

**Kabuğun yaptığı:** akışı açar (`EventSource`), çekirdeğin kuyruğunu sırayla gönderir
(`POST /send`), çerçeveyi çekirdeğe verir; çekirdeğin döndürdüğü pikselleri `putImageData`
ile boyar; protokolden gelen metni (kapsül cümlesi, hata iletisi) ve çekirdeğin biçimlediği
metni **olduğu gibi** `textContent`'e yazar; liste kaydırma, sekme, ayar sayfası, düğmeler
(düzen öğeleri); TR/EN arayüz etiketleri; geçişte çekirdeğin verdiği iki kutu arasında tek
genel CSS geçişi; süre ölçümü (ham zaman damgası).

**Kabuğun yapmadığı ve test edilenler:** alan öğesi sıralama/süzme (akış sırası `capsules`
cevabındaki gibi), metin kurma, sayı/tarih biçimleme, konum hesabı, isabet testi, açılma
kararı (çekirdek `tap_at` gönderir, otorite karar verir), dış istek. Kabuksuz eşitlik:
`tests/parity.mjs` (tarayıcı) = `qp drive` (yalnız protokol mesajları).
