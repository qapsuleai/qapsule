# Web kabuğu — mimari kuralları (R4/T5)

Sınır `docs/ARCHITECTURE.md` "Kabuk sınırı (R4)". Kabuk (`index.html`, `main.js`,
`style.css`) bayt ve piksel taşır; **alan mantığı Rust çekirdektedir** (`crates/qp-wasm/src/shell.rs`,
`ShellCore`): yansı, gönderilecek her protokol mesajı, hareket tanıyıcı (dokunma / sürükleme /
kıstırma), yüz ve açık kapsül sahneleri, pikseller, oynatma durumu ve kanıt metinlerinin
biçimi, geçiş kutuları, ölçüm istatistikleri.

Denetim: `node shells/web/arch-check.mjs` (`scripts/verify.sh` adım `shell`). Yorumlar ve
dizge içerikleri çıkarıldıktan sonra `main.js` taranır; kendi kendini sınama örnekleri (her
kural için bozuk bir satır) yakalanmazsa adım düşer.

| Kural | Ne yasak | Neden |
|-------|----------|-------|
| R1 | Dış adres: `http(s)://` (JS, HTML, CSS), CSS `@import`/dış `url()`, başka kökene `fetch`/`EventSource`/`WebSocket` | Sayfa dış ağa çıkmaz (GOAL 2); yazı tipi de yerel |
| R2 | `.sort( .filter( .reduce(`, `toFixed toPrecision toLocale* Intl. Math. Number( parseInt parseFloat Date String( .toString(`, `.join( .concat( .padStart .padEnd .repeat(`, `.slice( .splice( .substring( .substr(` | Sıralama, süzme, sayı/tarih biçimleme, metin kurma, liste kesme çekirdeğin işi |
| R3 | Aritmetik ve birleştirme işleçleri: `+ - * / %` (`++ -- += …` dahil; `=>` serbest) | Kabuk hiçbir değer hesaplamaz: px → bar, ölçüm farkları, geçiş kutuları çekirdekte |
| R4 | Şablon dizgesinde `${…}` | Alan verisinden metin kurma |
| R5 | `core.<ad>` yalnız izin listesindeki yöntemler (aşağıda) | WASM yüzeyi dar ve bilinir |
| R6 | `./pkg/qp_wasm.js` dışından `import`; satır içi betik; `main.js` dışında betik; HTML'de `on…=` | Yüklenen kod yalnız bu iki dosya |

**İstisna listesi:** yok. (Ölçüm süreleri ham `performance.now()` olarak çekirdeğe verilir;
`metric_span(ad, t0, t1)` farkı çekirdekte alır. Çizim yüzeyi boyutu `FACE_W`/`FACE_H`
sabitleri ve çekirdeğin bildirdiği `frame` ile verilir; çarpma yok — HiDPI ölçeği bu yüzden
yok, bkz. `rounds/R4/experiments/T5.md` açık sorular.)

**İzinli çekirdek yöntemleri** (`Shell`, `crates/qp-wasm/src/lib.rs`): `on_frame`,
`take_outbox`, `idle`, `feed`, `face_rgba`, `scene_rgba`, `open`, `evidence`, `status`,
`take_error`, `look`, `instruments`, `audience`, `set_frame`, `replay`, `seek`,
`set_sensitivity`, `set_audience`, `set_style`, `back`, `open_to`, `pointer`, `wheel`,
`transition`, `transition_from`, `metric_frame`, `metric_span`, `metric_mark`, `metric_env`,
`metrics`, `metrics_rows`, `digests`, `png`, `sent`.

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
