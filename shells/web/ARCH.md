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
`open_capsule`, `set_dpr`, `face_size`, `choices`, `transition_progress`, `transition_frame`; (R4/D) `metric_device`, `metric_memory`, `metric_reset` (ölçüm ekranı: cihaz bilgisi, bellek örneği, seriyi sıfırlama).

(RP3/T2) Eklenenler ve gerekçeleri:

| Yöntem | Neden |
|--------|-------|
| `metric_inherit(prev)` | Sonraki bir başlangıç çerçevesi (yeniden bağlanma, dil, geri sarma) yeni çekirdek kurar; ölçümler (açılış zaman çizelgesi, seriler, sürmekte olan karşılaştırma) eskisinden devralınır. R4'te açılış işaretleri dakikalar sonra yeniden alınıyordu (`open_ms.start_frame` 836.034 ms). Kabuk hiçbir değeri taşımaz, yalnız iki nesneyi verir. |
| `set_paint(istenen, kullanılan, neden)` | Hangi boyama yolunun kullanıldığı ve neden yedeğe düşüldüğü özetin içinde (özet kendini anlatır). |
| `face_scene(id, force)`, `scene_desc(force)` | Piksel almayan boyama yolları için sahnenin aygıt ölçeğinde **boyama listesi** (`paint/README.md`): bütün sayılar, renk ve yazı tipi dizgeleri çekirdekte hazırlanır; yol yalnız çizim çağrısı yapar (R2–R4 yollarda da geçerli). `face_rgba`/`scene_rgba` ile aynı "değişmediyse boş" kısa devresi. |
| `font()` | Başvuru çizicinin gömülü yazı tipi dosyası (IBM Plex Mono, OFL): tuvalle metin çizen yol aynı dosyayı `FontFace` ile kaydeder (yazı tipi farkı görsel farkın ana kaynağı olmasın; ağ isteği yok, R1). |
| `bench_start(cfg, t)`, `bench_tick(t, scrollHeight, clientHeight)`, `bench_rows()`, `bench_summary()` | Otomatik karşılaştırma (GOAL 8): senaryo sürücüsü (zamanlama, kaydırma konumu, yapay sürükleme, oynatma denetimi, istatistik, tablo metni) çekirdekte; kabuk dönen eylemleri olduğu gibi uygular. Kaydırma sınırı için `scrollHeight`/`clientHeight` ham verilir; çıkarma çekirdekte. |
| `cpu_options(seçenekler)`, `cpu_face(id, force)`, `cpu_scene(force)`, `cpu_moving()`, `cpu_memory()`, `cpu_pixels()` | (RP3/M12a) `cpu` boyama yolu (`paint/cpu.js`): çekirdek her yüzün ve açık sahnenin son karesini tutar, yalnız değişen dikdörtgeni başvuru çizicinin kendi koduyla yeniden boyar ve cevapta piksel göstergesini, kare boyutunu ve değişen dikdörtgeni **hazır** verir (`[ptr, len, w, h, dx, dy, dw, dh, settle, reuse, view]`); kabuk hiçbir sayı hesaplamaz, yalnız `putImageData(img, 0, 0, dx, dy, dw, dh)` çağırır. `cpu_memory` WASM belleğinin kendisidir: kare sınırdan kopyalanmadan bir görünümle okunur (kopya ölçümü için `cpu_pixels`). `cpu_moving`: grafikte sürükleme/kıstırma sürüyor mu (çekirdeğin kendi hareket tanıyıcısı); bitince yol `env.repaint()` ile aygıt ölçeğinde yeniden boyatır. `cpu_options`: ölçüm için teknikleri kapatma (`?cpu=`). |

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
