# RP3/T4 — bağımsız denetim (bellek)

Denetçi alt ajan. Dal `rp3/t4-mem` (15fc9c5), tümleştirme `rp3/s-integrate` (c769553). Kendi
karalama worktree'lerimde derlendi (c769553 ve 6877bda), portlar 8500–8502. Dallara yazılmadı.

**Karar: T4 için PASS. c769553'teki `free()` yerleşimi için koşullu: rapordaki "sonra" sayısı
bu yerleşimle tutmuyor (aşağıda 3b).**

## 1. Kapsam ve geri dönüş yolunun doğruluğu: PASS
- `6877bda..rp3/t4-mem`: `qp-wasm` (Cargo.toml'a serde, lib.rs, shell.rs), Cargo.lock'ta tek satır
  (`serde`), `tests/mem_t4.rs`, `rounds/RP3/experiments/T4*`. Özet, döküm ya da altın görsel değişmedi.
- `core_from_start`: tipli ayrıştırma başarısız olursa eski JSON ağacı yoluna düşüyor. principal/lang
  yoksa, null ise ya da metin değilse `arkin`/`tr` kullanılıyor; bozuk `events` boş liste oluyor;
  `data` zorunlu kalıyor. Sonuç iki yolda da aynı.
- `on_frame` veri çerçevesi: `data` dizi değilse, bir olay bozuksa ya da replay bozuksa tipli
  ayrıştırma başarısız oluyor ve frame eski yola düşüyor (bozuk parçayı atlıyor). `msg` varsa yine
  eski yol kullanılıyor. `data:null`/`replay:null` iki yolda da hiçbir şey yapmıyor. Tek fark,
  gerçekte gönderilmeyen bir uç durum: `{"msg":null,"data":[…]}`. Eski yol bunda hata veriyordu;
  yeni yol `Option<IgnoredAny>` alanını None okuyor ve veriyi uyguluyor. Pratikte önemsiz.
- `face_shown` budaması en kötü durumda tek bir zorunlu yeniden boyama yapar. `sent` sınırı da
  1024–2048 arasında kalıyor; kısa oturumlarda (parite) kayıt tam kalıyor.
- `mem_t4.rs`: tipli ve ağaç yolundan geçen başlangıç ve veri çerçeveleri için `digests`,
  `status_json`, `version` ve değişiklik nesnesinin eşit olduğunu doğruluyor. Ayrıca `sent`
  sınırının kısa ve uzun oturumdaki davranışını sınıyor. Anlamlı bir test.

## 2. Testler: PASS
c769553 üzerinde:
- `cargo test -p qp-wasm --features replica`: bütün test ikilileri ok (toplam 25 test, 0 başarısız).
- `cargo clippy -p qp-wasm --features replica --all-targets -- -D warnings`: temiz.
- `node shells/web/arch-check.mjs`: 0 ihlal, öz-sınama 22/22.

## 3. Bellek yeniden üretimi (Chromium, başsız, 64 adım/sn, WASM MB)
a) **Sürekli akış, c769553: PASS.** Ölçülen değerler 0: 16,8 · 1261: 36,0 · 2053: 45,4 ·
   2562: 62,4. Ham değer 2534: 62,4; beklenen 17,5 + 0,0176 × 2550 ≈ 62. JS yığını 2–3 MB ve DOM
   düz kaldı.

b) **Her 250 adımda yeniden başlama:**

| | ~1265. adım | ~2530. adım |
|---|---|---|
| önce (6877bda) | 148,4 | — |
| T4 ham, önce | 148,4–148,6 | 331 |
| T4 yamasının yerleşimi (`free` yeni çekirdekten **önce**) | 37,8 | 61,3 |
| T4 ham, sonra | 37,7 | 61,5 |
| **c769553 yerleşimi** (`free`, `new Shell` + `metric_inherit` **sonrası**) | **54,9** | **88,2** |

   T4'ün kendi sayıları, kendi tanımladığı koşulda (yama) birebir üretiliyor. Tümleştirilmiş kodda
   ise eski çekirdek yenisi kurulurken hâlâ canlı. İki yansı aynı anda bellekte duruyor ve WASM
   belleğinin tepesi (hiç küçülmüyor) yükseliyor. Düşüş yine büyük (331 → 88 MB, −%73) ve sınırlı:
   eğri basamaklı düzlükler yapıyor, her yeniden başlamada birikmiyor. Ama yeniden başlamasız
   eğrinin (62 MB) yaklaşık 26 MB üstünde. **rp3/s-integrate için "347 → 64 MB" yazılmamalı;
   ölçülen değer 88 MB.**

   Not: T4'ün `mem.mjs` dosyasındaki reconnect değişkesi T2 sonrası `main.js` ile takılıyor.
   `metric_inherit` yüzünden `open_ms.start_frame` değişmiyor. Bu yüzden `open_repeats.start_frame`
   sayacına bakan bir kopya kullandım.

## 4. `free()` yerleşimi (main.js, c769553): PASS (doğruluk açısından)
- Yalnız `core` bırakılıyor, `next` hiç bırakılmıyor. `metric_inherit(&mut Shell)` eski nesneyi
  ödünç alıyor, tüketmiyor; bu yüzden `free()` geçerli.
- Bırakma ile `core = next` arasında hata atabilecek bir adım yok. `new Shell` hata atarsa eski
  çekirdek bırakılmamış olarak kalıyor.
- Bekleyen geri çağrıların hepsi modül düzeyindeki `core` değişkenini çağrı anında okuyor: flush
  zinciri (yalnız `out` dizgesini tutuyor), `sizes.json` → `core.metric_env`, `setInterval`
  `sampleMemory`, rAF `frame`/bench, geçişteki `step`, `paintEnv.repaint`. `paint/ref.js` ve
  `fixture.js` kendi modül değişkenlerini aynı eşzamanlı `start()` içinde `paint.init(core)` ile
  güncelliyor; `fixture` yazı tipi `then`'i de modül değişkenini okuyor. Eski nesneyi tutan bir
  kapanış bulmadım, dolayısıyla serbest bırakılmış çekirdeğin kullanılması riski görmedim.

## Öneri
Tepenin T4 seviyesine inmesi için ölçümler yeni çekirdek kurulmadan önce eski çekirdekten alınmalı
(ör. eski çekirdekten ölçümleri dışarı alan bir yöntem, sonra `free()`, sonra `new Shell`, sonra
devralma). Alternatif: rapora c769553 için ölçülen 88 MB (≈2530. adım) yazılır.

## Ek: 9170fba yeniden denetimi (`metric_detach` → `free` → `new Shell` → `metric_attach`)

Karalama worktree'si 9170fba'da kuruldu, sonra kaldırıldı. Yöntem öncekiyle aynı: Chromium,
64 adım/sn, `memv.mjs`, portlar 8500–8502. Sunucu ikilisi c769553'tekiyle aynı; c769553..9170fba
arasında yalnız `qp-wasm` ve `shells/web` değişti.

**Her 250 adımda yeniden başlama, WASM MB, 4 koşu:**

| koşu | ~1265. adım | ~2530. adım |
|---|---|---|
| 1 | 37,8 | 61,4 |
| 2 | 40,1 | 73,6 |
| 3 | 37,9 | 61,4 |
| 4 | 40,1 | 61,4 |
| **medyan** | **≈39** | **61,4** |

Karşılaştırma:

| | ~1265. adım | ~2530. adım |
|---|---|---|
| önce | 148,4 | 331 |
| c769553 | 54,9 | 88,2 |
| T4 yama ölçümü | 37,7 | 61,5 |
| yeniden başlamasız akış, aynı koşu (9170fba) | 36,0 | 62,4 |

Rapordaki eğriye döndü: yeniden başlama artık yeniden başlamasız oturumdan ayırt edilmiyor. Koşu
2'deki 73,6 MB tek bir yüksek su işareti; WASM belleği basamak basamak büyüdüğünden bir geçici
tepe bir basamak atlatmış. Sonraki örneklerde birikme yok. JS yığını 2–4 MB, DOM düz.

**`start()` doğruluğu: kabul edilebilir.**
- Sıra şöyle: eski çekirdekten ölçümler alınıyor (`metric_detach`), eski çekirdek bırakılıyor,
  `core = null`, sonra `new Shell`, sonra `metric_attach(carry)`. `metric_attach` taşıyıcıyı
  değer olarak alıyor; wasm-bindgen JS nesnesini tüketiyor, yani taşıyıcıyı iki kez kullanma
  riski yok.
- `new Shell` hata atarsa: `core` null kalıyor ve taşıyıcı JS çöp toplayıcısına kalıyor
  (FinalizationRegistry ile bırakılır, küçük). `onFrame`, `flush`, `sampleMemory`, rAF `frame`
  ve `paintEnv.repaint` null'a karşı korumalı. Sayfa bir sonraki başlangıç çerçevesine kadar
  ölü kalıyor.
- Aynı durumda korumasız kalan yollar: eski yüz tuvallerindeki işaretçi işleyicileri,
  IntersectionObserver → `paint.paintFace` (paint modülü hâlâ bırakılmış eski nesneyi tutuyor)
  ve `window.qp.*`. Bunlar JS hatası atar (wasm-bindgen'de bırakılmış nesnede çağrı istisnadır,
  bellek bozulması değildir).
- Eski davranış daha iyi değildi: `conn` ve `token` önceden yeni değerlere geçtiği için eski
  çekirdek yeni bağlantıyla karışık durumda kalıyordu.
- `new Shell` yalnız bozuk bir sunucu çerçevesinde hata atar. **Öneri (zorunlu değil):** hata
  durumunda `paint.init` çağrısını atlamak yerine toast göstermek ve yüz listesini temizlemek.

**Karar: PASS.**
