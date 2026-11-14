# T6b — Protokol kodlaması: JSON kanonik mi kalmalı?

Tarih: 2026-11-14. Dal: `rd1/t6b-exp`. Kod: `experiments/t6b-encoding/` (workspace üyesi değil).
Ham çıktılar: `rounds/RD1/experiments/raw/t6b/`.

## 1. Hipotez (ölçümden önce yazıldı)

Test edilen çalışma kararı: **JSON (serde_json, `float_roundtrip` açık) kanonik kodlamadır.**

Beklentiler (ölçümden önce, literatür ve sezgiye dayanarak):

- H1 — Canlı tick (`notify` + `set_last_bar` / 1 barlık `append_bars`): JSON mesajı ~200–300 bayt,
  ikili biçimler (CBOR/MessagePack) ~%40–60'ı. 60 mesaj/s'de JSON ~15–20 KB/s; bu, herhangi bir
  ağ veya cihaz için önemsiz. Kodlama/çözme mesaj başına birkaç µs; 16 ms çerçeve bütçesinin
  binde birinin altında. **Sonuç beklentisi: tick yolunda kodlama seçimi önemsiz.**
- H2 — Seri parçası (10 000 bar, RSI/MACD `values`, 10 000 barlık `append_bars`): JSON ham boyutu
  ikili biçimlerin ~2–3 katı; `float_roundtrip` açık JSON çözme, postcard/sütunlu bloğa göre
  5–20 kat yavaş (ms mertebesi, 10 000 bar için 1–5 ms). gzip/brotli sonrası boyut farkı
  daralır (~1,3–1,8x). Sütunlu f64 LE blok, boyutta (8 B/değer) ve hızda (memcpy'ye yakın)
  açık ara en iyisi olur.
- H3 — Anlık görüntü (10 000 barlık `Document`): H2'ye benzer; tek seferlik olduğu için
  birkaç ms'lik fark kullanıcıya görünmez.
- H4 — f64 birebirlik: `float_roundtrip` açık JSON ve tüm ikili biçimler bit düzeyinde birebir;
  `float_roundtrip` kapalı JSON rastgele bit desenlerinde bazı ULP sapmaları gösterir.
- H5 — postcard, `#[serde(tag = "type")]` (iç etiketli enum) ve `skip_serializing_if` ile aynı
  tipleri **çözemez** (self-describing değil); ancak ayna tiple ölçülebilir. CBOR (ciborium /
  minicbor-serde) ve MessagePack (rmp-serde, struct-map modunda) aynı tiplerle çalışır.
- H6 — WASM'de oranlar benzer, mutlak süreler yerel ölçümün ~1,5–3 katı.

Beklenen karar: JSON kanonik kalır; seri verisi (bar ve `values` blokları) için müzakere
edilen isteğe bağlı bir ikili/sütunlu özellik (`features` ile) değerli olabilir, ama v0 için
zorunlu değil.

---

## 2. Yöntem

**Ortam.** Apple M4 Max, macOS 27.0.1, rustc 1.92.0 (repo toolchain), Node v24.3.0.
Yerel: `--release` (lto thin, codegen-units 1). WASM: `wasm32-wasip1`, Node `node:wasi`
(preview1) altında (`run-wasi.mjs`); V8 = Chrome ile aynı motor, ama tarayıcı değil.

**Kod** (`experiments/t6b-encoding/`, kendi `[workspace]` tablosu, workspace üyesi değil):

- `src/common.rs` — deterministik korpus ve ölçüm. Fikstür `fixtures/protocol/docs/thyao.json`
  (200 bar) sabit tohumlu xorshift64* rastgele yürüyüşle 10 000 bara uzatılır (fiyat ve hacim
  fikstürdeki gibi 2 ondalık). RSI(14) ve MACD(12,26,9) bu barlardan gerçekten hesaplanır
  (göstergeler tam hassasiyetli f64, ısınma `null`). 60 canlı tick (`set_last_bar`) ve 60 tek
  barlık `append_bars`. Süre: her örnek ≥0,5 ms olacak kadar tekrar, 31 örneğin medyanı
  (çağrı >20 ms ise 11), önce ısınma turu.
- `src/main.rs` — kodlayıcılar **aynı `qp-schema` tipleri** üzerinde (`ServerMsg`, `Document`).
  Her kodlayıcı `&ServerMsg -> bytes -> ServerMsg`; ayna/sütun tiplerine dönüşüm maliyeti
  süreye **dahildir**. Her ölçümden önce gidiş-dönüş `PartialEq` ve bit eşitliğiyle doğrulanır.
- `src/qcol.rs` — sütunlu sıcak yol denemesi ("qcol"): küçük başlık + `t` i64 ve `o,h,l,c,v`
  f64 little-endian bloklar, her blok 8 bayta hizalı (JS tarafında kopyasız `Float64Array`
  görünümü için); `values` için `null` = geçerlilik bitmap'i (Arrow tarzı, NaN değil).
  Anlık görüntüde: barları çıkarılmış `Document` JSON + bar blokları.
- `plainjson/` — `float_roundtrip` **kapalı** serde_json karşılaştırması. `qp-schema` bu özelliği
  koşulsuz açtığı için (Cargo özellik birleşmesi) şema kaynak dosyaları `#[path]` ile ayrı bir
  workspace'te yeniden derlendi; tipler birebir aynı.
- `sizeprobe/` — `wasm32-unknown-unknown` cdylib, ana workspace'in `wasm` profili ayarları
  (opt-level s, lto fat) + strip; özelliğe göre `ServerMsg` kodla+çöz kodu eklenir, `.wasm`
  boyutu ölçülür. `wasm-opt` kurulu değil (ölçüm wasm-opt'suz).
- `js-parse.mjs` — aynı yükler için V8 `JSON.parse`/`JSON.stringify` (ince JS kabuğu senaryosu).

**Kodlamalar ve sürümler** (crates.io, 2026-11-14): serde_json 1.0.154 (`float_roundtrip`),
ciborium 0.2.2 (CBOR), minicbor-serde 0.7.2 (CBOR; minicbor 2.3.1), rmp-serde 1.3.1
(MessagePack; `to_vec_named` = struct-map ve `to_vec` = struct-array), postcard 1.1.3,
flate2 1.1.11 (gzip-6 ve WebSocket permessage-deflate benzetimi), brotli 9.1.1 (q6, pencere 22).
Arrow IPC denenmedi: qcol, Arrow'un tampon yerleşiminin (hizalı LE blok + bitmap) metadata'sız
hâlidir; Arrow'un ek maliyeti FlatBuffers şema/mesaj başlığıdır (bkz. Öneriler).

**Sıkıştırma.** Mesaj başına gzip/brotli (yalnız yerel; boyutlar platformdan bağımsız).
Canlı akış için ayrıca "ws-deflate": tek raw-deflate akışı, her mesajdan sonra sync flush
(RFC 7692 permessage-deflate, context takeover açık; sondaki 4 bayt düşülür). Sıkıştırmanın
CPU maliyeti ölçülmedi.

**Çalıştırma:** `cargo run --release -- <raw dizini> [yük dizini]`; WASM:
`cargo build --release --target wasm32-wasip1 && node run-wasi.mjs target/wasm32-wasip1/release/t6b-encoding.wasm`.
Ham çıktılar: `raw/t6b/native.md`, `native_rerun.md`, `wasm.md`, `wasm_rerun.md`,
`native_plainjson.md`, `wasm_plainjson.md`, `wasm_size.md`, `js_parse.md`, `cpu.txt`, örnek
tick mesajı (`sample_tick_set_last.*`). İki koşu arasında süreler ±%5 içinde; tablolar `native.md`
ve `wasm.md`'den.

## 3. Uyumluluk: aynı tiplerle hangi biçim çalışıyor?

| Biçim | Aynı `qp-schema` tipleri (`tag="type"`, `skip_serializing_if`, `deny_unknown_fields`) |
|---|---|
| JSON (serde_json) | Çalışır. |
| CBOR ciborium / minicbor-serde | Çalışır (self-describing; `deserialize_any` var). |
| MessagePack rmp, struct-map (`to_vec_named`) | Çalışır. |
| MessagePack rmp, struct-array (`to_vec`) | **Çözemez**: "invalid type: sequence, expected a string" (iç etiketli enum + atlanan alanlar konumsal diziyle bozulur). Yalnızca `values` (enum'suz iç yapı) geçti. |
| postcard | **Çözemez**: "This is a feature that PostCard will never implement" (iç etiketli enum `deserialize_any` ister). `skip_serializing_if` de konumsal biçimi bozar. Yalnızca ayna tiplerle (dış etiketli enum, atlama yok) ölçüldü; `Document` için ayna yazılmadı (tüm şemanın ikinci kopyası olurdu). |
| qcol | Elle yazılmış; yalnızca `set_last_bar`, `append_bars`, `values` ve anlık görüntüdeki barlar. |

Şema evrimi denemesi (`Bar`'a gönderen tarafta fazladan `x` alanı, alıcıda `deny_unknown_fields`):
JSON, ciborium, minicbor-serde, rmp(struct-map) hepsi **açık hata** verir ("unknown field `x`");
`deny_unknown_fields` olmayan tipte yok sayarlar; eksik alan hepsinde hata. **postcard fazladan
alanı sessizce kabul eder** (sondaki baytları okumaz) — `deny_unknown_fields` postcard'da etkisiz;
eksik alan "Hit the end of buffer". Yani konumsal biçimde sürüm uyumsuzluğu fark edilmeden
yanlış veri üretebilir.

## 4. Sonuçlar

Tablolarda seçilmiş kodlayıcılar var; tamamı (ciborium, rmp struct-array, postcard same-types
dahil) `raw/t6b/native.md` ve `wasm.md`'de. "enc/dec" = mesaj başına medyan.

### 4.1 Canlı tick — tek `set_last_bar` / tek barlık `append_bars`

Örnek mesaj (234 B): `{"type":"notify","sub":"sub-px","event":{"kind":"change","seq":100030,"at":2606601600000,"author":"feed","changes":[{"op":"set_last_bar","series":"px","bar":{"t":2606601600000,"o":57.38,"h":57.79,"l":57.38,"c":57.46,"v":604460.41}}]}}`

| `set_last_bar` | B/mesaj | enc µs yerel / wasm | dec µs yerel / wasm | gzip-6 tek mesaj | ws-deflate B/mesaj | B/s @60 ham | B/s @60 ws-deflate | CPU µs/s (enc+dec, 60 mesaj) yerel / wasm |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| JSON (float_roundtrip) | 234 | 0,22 / 0,39 | 0,86 / 1,11 | 174 | 35,1 | 14 030 | 2 107 | 67 / 92 |
| CBOR minicbor-serde | 183 | 0,23 / 0,24 | 0,79 / 1,18 | 190 | 33,4 | 10 980 | 2 003 | 64 / 87 |
| CBOR ciborium | 183 | 0,27 / 0,43 | 1,60 / 1,98 | 190 | 33,4 | 10 970 | 2 005 | 125 / 148 |
| MessagePack rmp (map) | 183 | 0,25 / 0,32 | 0,69 / 0,84 | 196 | 33,5 | 10 980 | 2 008 | 55 / 70 |
| postcard (ayna) | 78 | 0,19 / 0,16 | 0,08 / 0,11 | 101 | 33,9 | 4 680 | 2 036 | 16 / 17 |
| qcol | 83 | 0,10 / 0,08 | 0,08 / 0,09 | 106 | 34,1 | 4 980 | 2 048 | 11 / 10 |

`append_bars(1)`: aynı tablo; JSON 236 B, ws-deflate 52,8 B/mesaj (3 166 B/s), CBOR/MsgPack
~60 B, postcard 55 B, qcol 55,7 B. CPU JSON 70 / 94 µs/s.

Okuma: 60 mesaj/s'de JSON **14 KB/s ham, 2,1 KB/s sıkıştırılmış akışta**; ikili biçimlerle
sıkıştırılmış akışta fark **%0–5** (bazı durumlarda JSON daha küçük). CPU: saniyede ~70–90 µs =
bir çekirdeğin **%0,01'i**; 16 ms çerçeve başına ~1,5 µs. Mesaj başına (context takeover'sız)
sıkıştırma küçük ikili mesajları **büyütür** (183 → 190–196 B); tick yolunda yalnızca akış
sıkıştırması anlamlı.

### 4.2 Seri parçası — 10 000 bar

| Yük | Kodlama | Bayt | gzip-6 | brotli-6 | enc µs yerel / wasm | dec µs yerel / wasm |
|---|---|---:|---:|---:|---:|---:|
| `values` RSI (1 çıktı) | JSON | 321 991 | 122 124 | 102 907 | 202 / 333 | 397 / 550 |
| | JSON, `float_roundtrip` kapalı | 321 991 | | | 205 / 342 | 373 / 483 (**448 değer bozuk**) |
| | CBOR minicbor-serde | 179 986 | 123 468 | 101 001 | 43 / 81 | 187 / 361 |
| | MessagePack rmp (map) | 179 986 | 123 471 | 100 998 | 70 / 123 | 169 / 254 |
| | postcard (ayna) | 154 631 | 115 148 | 109 993 | 60 / 89 | 49 / 46 |
| | qcol | 161 296 | 110 946 | 105 348 | 37 / 22 | **7,8 / 9,8** |
| `values` MACD (3 çıktı) | JSON | 733 728 | 308 099 | 290 159 | 870 / 1 181 | 1 030 / 1 392 |
| | JSON, `float_roundtrip` kapalı | 733 728 | | | 867 / 1 192 | 864 / 1 098 (**3 337 değer bozuk**) |
| | CBOR minicbor-serde | 359 388 | 280 789 | 248 834 | 68 / 124 | 371 / 744 |
| | MessagePack rmp (map) | 359 388 | 280 782 | 248 811 | 121 / 241 | 355 / 528 |
| | postcard (ayna) | 334 031 | 270 699 | 257 637 | 84 / 121 | 69 / 63 |
| | qcol | 323 816 | 261 321 | 254 522 | 82 / 51 | **23 / 25** |
| `append_bars` 10 000 | JSON (bar dizisi, AoS) | 767 220 | 214 460 | 209 681 | 859 / 1 682 | 2 355 / 3 037 |
| | **JSON SoA** (barlar sütun dizisi) | 507 256 | **174 735** | **150 849** | 593 / 1 187 | 895 / 1 275 |
| | CBOR minicbor-serde | 670 119 | 235 588 | 214 648 | 326 / 271 | 1 719 / 2 973 |
| | CBOR ciborium | 659 189 | 237 900 | 218 279 | 506 / 1 017 | 4 532 / 5 605 |
| | MessagePack rmp (map) | 670 119 | 235 589 | 214 769 | 319 / 671 | 1 494 / 1 785 |
| | postcard (ayna) | 464 751 | 213 695 | 211 059 | 88 / 127 | 60 / 67 |
| | qcol | 480 040 | 189 167 | 177 532 | 81 / 48 | **27 / 33** |

### 4.3 Anlık görüntü — 10 000 barlık `Document`

| Kodlama | Bayt | gzip-6 | brotli-6 | enc µs yerel / wasm | dec µs yerel / wasm |
|---|---:|---:|---:|---:|---:|
| JSON | 769 516 | 215 744 | 210 717 | 859 / 1 674 | 2 381 / 3 054 |
| JSON SoA (barlar sütun) | 509 577 | **175 730** | **151 512** | 614 / 1 192 | 638 / 1 054 |
| CBOR minicbor-serde | 671 948 | 236 712 | 215 634 | 339 / 266 | 1 722 / 2 994 |
| CBOR ciborium | 660 958 | 238 989 | 219 264 | 514 / 1 025 | 4 701 / 5 664 |
| MessagePack rmp (map) | 671 948 | 236 730 | 215 771 | 329 / 681 | 1 529 / 1 810 |
| qcol (JSON gövde + bar blokları) | 482 480 | 190 549 | 178 424 | 80 / 54 | **32 / 44** |

### 4.4 JS kabuğu tarafı (V8, aynı yükler)

| Yük | Bayt | `JSON.parse` µs | `JSON.stringify` µs |
|---|---:|---:|---:|
| tick | 234 | 0,8 | 0,6 |
| RSI 10k | 321 991 | 487 | 998 |
| MACD 10k | 733 728 | 1 137 | 2 229 |
| append 10k | 767 220 | 2 274 | 3 170 |
| Document 10k | 769 516 | 2 302 | 3 538 |

V8 `JSON.parse` serde_json'dan hızlı değil (aynı mertebe). V8 sayı gidiş-dönüşü MACD'nin 29 909
değerinde 0 fark (JS kabuğu JSON'u birebir okur). qcol bloğu için `Float64Array` görünümü
oluşturmak ~0,02 µs (kopyasız).

### 4.5 f64 birebirlik

100 001 değer: 45 zor değer (alt-normal 5e-324, -0.0, 0.1, 0.1+0.2, max/min normal, 2^53±1,
1e21–1e23, 1−ε/2 …) + rastgele sonlu bit desenleri + tam hassasiyetli fiyat benzeri değerler;
hem `append_bars` (5 değer/bar) hem `values` içinde.

| Kodlama | Bit farkı (yerel ve wasm aynı) |
|---|---:|
| JSON `float_roundtrip` | 0 |
| CBOR ciborium, minicbor-serde; MessagePack rmp (map); postcard (ayna); JSON SoA; qcol | 0 |
| **JSON `float_roundtrip` kapalı** | **23 132 / 100 001**; zor değerlerden 5'i (ör. `123456789.12345679`, `9007199254740991`, `0.9999999999999999 -> 1`); gerçek göstergelerde RSI'nin 448, MACD'nin 3 337 değeri |

`float_roundtrip`'in maliyeti: çözmede MACD +%19, RSI +%6, 2 ondalıklı barlarda ~%0 (yerel).

### 4.6 WASM ikili boyutu (`ServerMsg` kodla+çöz; opt-level s, lto fat, strip; wasm-opt yok)

| Özellikler | .wasm bayt | gzip -9 |
|---|---:|---:|
| hiçbiri (boş iskelet) | 13 331 | 5 134 |
| **JSON (taban)** | **822 239** | **206 689** |
| JSON + CBOR ciborium | 1 684 550 (+862 KB) | 394 134 (+187 KB) |
| JSON + CBOR minicbor-serde | 1 718 544 (+896 KB) | 405 246 (+199 KB) |
| JSON + MessagePack rmp | 1 699 210 (+877 KB) | 397 590 (+191 KB) |
| JSON + postcard (yalnız `append_bars` aynası) | 827 084 (+4,8 KB) | 207 156 (+0,5 KB) |
| JSON + qcol | 839 920 (+17,7 KB) | 214 244 (+7,6 KB) |
| yalnız qcol | 64 420 | 25 209 |

Serde her biçim için tüm şema ağacını yeniden monomorfize eder: tüm `ServerMsg` için ikinci bir
serde biçimi WASM'i **ikiye katlıyor** (+~860–900 KB, gzip +~190 KB). Dar, elle yazılmış bir
sütun çerçevesi ~18 KB.

## 5. Hipotez ile karşılaştırma

| # | Beklenti | Sonuç |
|---|---|---|
| H1 | Tick'te kodlama önemsiz; ikili ~%40–60 | **Doğrulandı**, oran farklı: CBOR/MsgPack JSON'un %78'i, postcard/qcol %33–35'i; ama akış sıkıştırmasıyla hepsi ~34–35 B (fark ≤%5). CPU %0,01 çekirdek. |
| H2 | JSON 2–3x büyük, çözme 5–20x yavaş, gzip sonrası 1,3–1,8x | Kısmen. Ham: MACD 2,3x, bar 1,6x. gzip sonrası **1,0–1,2x** (beklenenden dar); JSON SoA gzip/brotli'de **tüm ikili biçimlerden küçük**. Çözme: CBOR/MsgPack yalnızca 1,4–3x hızlı (wasm'da bar çözmede minicbor ≈ JSON, ciborium JSON'dan **yavaş**); qcol/postcard **40–90x** hızlı. |
| H3 | Anlık görüntü tek seferlik, fark görünmez | Doğrulandı: 10k bar JSON çözme 2,4 ms yerel / 3,1 ms wasm (bir çerçeveden az). |
| H4 | `float_roundtrip`'siz JSON bazı ULP sapmaları | Doğrulandı ve beklenenden ağır: rastgele desenlerin %23'ü, gerçek MACD değerlerinin %11'i bozuk. |
| H5 | postcard aynı tiplerle çözemez; CBOR/MsgPack(map) çalışır | Doğrulandı. Ek bulgu: rmp struct-array da çözemez; postcard `deny_unknown_fields`'ı sessizce deler. |
| H6 | WASM oranları benzer, 1,5–3x yavaş | Çoğunlukla 1,2–2x yavaş; qcol/postcard wasm'da yerele yakın (qcol kodlama wasm'da daha hızlı, çözme ~1,2x yavaş). |

## 6. Değerlendirme (ölçülmeyen eksenler)

- **Hata ayıklama:** JSON doğrudan okunur (`raw/t6b/sample_tick_set_last.json`); CBOR/MsgPack
  araçla okunur ama self-describing; postcard ve qcol şema olmadan okunamaz.
- **Ajan yazılabilirliği:** LLM'ler ve MCP araçları JSON yazar/okur; komut yolu (ajan → çekirdek)
  JSON'dan başka bir şey olamaz. İkili biçim yalnızca makineden makineye veri düzleminde anlamlı.
- **Şema evrimi:** Tek kaynak `qp-schema` + JSON Schema üretimi zaten JSON'a göre kurulu.
  CBOR/MsgPack(map) JSON veri modelini taşıdığı için aynı evrim kurallarına uyar; postcard ve
  qcol konumsaldır, sürüm alanı ve sürüm başına düzen gerekir (postcard fazlalığı fark etmez).
- **Bağımlılık:** ciborium'un son sürümü 2024-01; minicbor-serde/rmp aktif; postcard kararlı;
  qcol sıfır bağımlılık ama elle bakım (ve "şemanın tek kaynağı Rust tipleri" kuralı için
  düzenin Rust tiplerinden türetilmesi gerekir).

## 7. Sonuç

1. **JSON kanonik kalmalı.** Tick yolunda kodlama seçimi ölçülebilir bir fark yaratmıyor
   (akış sıkıştırmasıyla ~35 B/mesaj, 2,1 KB/s; CPU %0,01). 10k barlık seri/anlık görüntüde
   JSON çözme 1–3 ms (wasm), bir çerçevenin altında. `float_roundtrip` **zorunlu** kalmalı
   (kapalıyken gerçek MACD değerlerinin %11'i bozuluyor; maliyeti çözmede ≤%19).
2. **Genel amaçlı ikili biçim (CBOR/MessagePack) isteğe bağlı özellik olarak bile önerilmez:**
   sıkıştırılmış ağda kazanç ≤%10 (bazen kayıp), wasm çözmede bar yükünde ≤1,7x (minicbor ~1x,
   ciborium JSON'dan yavaş), `values`'da 2–2,6x (ms'nin altında mutlak kazanç), WASM boyutunu ikiye katlıyor (+~860 KB / +~190 KB gzip).
   postcard aynı tiplerle çalışmıyor ve evrimde sessiz hata riski taşıyor.
3. **Hacim seri bloklarında ve kazanç orada.** İki adım, sırayla:
   a. *JSON içinde sütunlu bar* (`bars: {t:[], o:[], …}`): ham −%34, gzip −%19, brotli −%28,
      çözme 2,6x (yerel) / 2,4x (wasm) hızlı; JSON ve ajan-okunurluğu korunur. `values` zaten
      bu biçimde. Bu bir protokol/şema kararı (Öneriler).
   b. *Müzakere edilen isteğe bağlı sütun blokları* (`features: ["columnar-f64"]` gibi;
      zarf JSON, `values`/`append_bars`/anlık görüntü barları 8 bayt hizalı LE f64 blok +
      bitmap): çözme **40–90x** hızlı (10k bar: 3 ms → 33 µs wasm), ham boyut MACD'de 2,3x
      küçük, JS'de kopyasız `Float64Array`, WASM +18 KB. v0 için gerekli değil; 100k+ bar,
      çok panel veya saat/telefon gibi zayıf cihazlarda ilk değerli adım olur. Seçilecekse
      özel çerçeve ile Arrow IPC arasında ayrı bir karar gerekir.
4. Tick yolu için ikili özellik **gerekmez**; bunun yerine WebSocket permessage-deflate
   (context takeover) önerilir.

## 8. Yapılamayanlar / sınırlar

- Tarayıcı içinde (Chrome/Safari) ölçülmedi; Node WASI = V8, Safari/JSC farklı olabilir.
- Arrow IPC denenmedi (bağımlılık ve wasm boyutu ayrıca ölçülmeli).
- Sıkıştırma CPU süresi ölçülmedi; brotli q6 / gzip-6 sabit.
- Tek makine (M4 Max); zayıf cihaz (telefon/saat) süreleri ölçülmedi, doğrusal ölçek varsayımı
  çıkarımdır.
- Fiyatlar 2 ondalıklı (fikstür gibi); bu, JSON'u bar yükünde kayırır. Gösterge değerleri tam
  hassasiyetli.
- Ayna/sütun dönüşümleri süreye dahil; elle yazılmış kodlayıcılar desteklenmeyen mesajlarda hata
  döner.

## 9. Öneriler (kapsam dışı, yapılmadı)

- `append_bars`, `put_series` ve `Document.series[].data.bars` için JSON içinde sütunlu bar
  biçimini protokol tasarımında değerlendirmek (`values` ile tutarlı olur).
- `features` müzakeresine ileride "sütunlu f64 blok" veri düzlemi özelliği için yer ayırmak;
  zarf ve komut yolu daima JSON.
- `docs/PROTOCOL.md`'ye taşıma notu: WebSocket'te permessage-deflate (context takeover) önerilir;
  mesaj başına sıkıştırma küçük mesajlarda zararlı.
- `float_roundtrip`'in `qp-schema`'da koşulsuz açık kalması bir test ile korunmalı (kapalıyken
  gerçek veride bozulma ölçüldü).
