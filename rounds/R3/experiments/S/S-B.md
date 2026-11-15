# S-B — Gramer ve çizim (R3/T7 bütünleştirmesi)

Dal `r3/s-grammar` (`r3/int` `dd62244` üstünde). Şartname: `docs/GRAMMAR.md` §18.

## 1. Ürün alfabesi

- `qp_grammar::glyph::siluet::Siluet`: M1 deney crate'i `rounds/R3/experiments/M1/siluet/`
  çizimi aynen taşındı (yalnız tip adı `Alphabet` → `Siluet`, iki `use` satırı). Deney
  crate'i dokunulmadı.
- Ürün tek alfabe kullanır: `qp_grammar::glyph::alphabet()` — **değişim bu tek satır**
  (insan testi `rozet`'i öne çıkarırsa). `Alphabet` trait'i duruyor.
- Eşdeğerlik kanıtı: `crates/qp-grammar/tests/glyph_alphabet.rs` tüm M1 kümesi (6 tür × 3
  yön × 5 aşama × 4 güç × 4 güven × 5 teyit × 2 tazelik/sunucu = 14 400 tarif) × 16/24/32
  px × açık/koyu = 86 400 çizimin `Prim` özetini (FNV-1a, float bitleriyle) sabitler:
  `0x98b3_5851_ec19_049d`. Aynı özet deney crate'iyle bir kerelik programla hesaplandı ve
  her durumda `Vec<Prim>` eşitliği doğrulandı:

  ```
  experiment m1_siluet digest 0x98b35851ec19049d
  product    siluet    digest 0x98b35851ec19049d
  identical Vec<Prim> for 86400 (spec, size, theme) cases
  ```

  Program (yeniden üretmek için, depo dışında bir klasörde): `Cargo.toml` bağımlılıkları
  `m1-siluet`, `qp-grammar`, `qp-schema` (path) ve boş `[workspace]`; `src/main.rs`
  `#[path = "<depo>/crates/qp-grammar/tests/glyph_alphabet.rs"] mod pinned;` ile
  `pinned::digest(&m1_siluet::Alphabet)` ve `pinned::digest(&Siluet)`'i karşılaştırır ve
  `all_specs()` üzerinde iki alfabenin `draw` çıktısını `assert_eq!` eder.

## 2. Yeni gramer öğeleri ve API

| Öğe | Yer |
|-----|-----|
| `Mark::Glyph` (`"glyph"`) — veri: katmanın `finding`'i | `qp-schema/src/grammar.rs` |
| `Position::Panel(PanelPos { panel, px })` — panele göre düzen konumu (`text`, `glyph`) | aynı |
| `Encoding.x2` — `rule` parçası (yatay/eğik), `band`/`area` bar aralığı | aynı |
| `Origin::Fact` alanı `geometry.<i>.from_bar` / `to_bar` / `points.<j>.bar` | aynı (belge), `depth::geometry_field` |
| `DataProvider::capsule(kapsül_ya_da_bulgu) -> Option<Cow<Capsule>>`, `DataProvider::glyph(…) -> Option<Cow<GlyphSpec>>` (ikisi de varsayılan `None`) | `qp-grammar/src/provider.rs` |
| `MemProvider::with_capsule`, `with_glyph` | aynı |
| `depth::story_glyph`, `GLYPH_MIN_LEDGER_N = 20`, `ledger_text`, `geometry_field`, `evidence_note`, `evidence_layers`, `capsule_card_view` | `qp-grammar/src/depth.rs` |

- `glyph` hataları tiplidir (`invalid`/`not_found`, katman + gösterici); sahne tarifi
  işaretin ilkellerini taşır; `hit` işaret öğesinde `data = {"kind":"finding",…}` döndürür
  (`crates/qp-render/tests/capsule_views.rs` her derinlik görselinde sınar).
- Kapsül görünümü (`capsule` derinliği, kapsülün `story`'si varsa): `card` paneli (işaret,
  cümle, karne satırı) + `price` paneli (son `max_points` barın kapanış çizgisi, seviye,
  olgu noktası). Kart panelinde ızgara/eksen yok.
- Grafik derinliği (ilk olguda `meaning` varsa): mumlar + her olgunun kanıt geometrisi
  (`Level` → parça, `Band` → mumların altında bant, `Points` → noktalar + eğik parça) +
  olgu barı üstünde işaret. R2 bulguları R2 kurallarıyla aynen.

### Güvenlik düzeltmesi (K33)

Güvenlik incelemesi: ilk sürümde geometri katmanları kanıt değerlerini görünüme sabit
olarak yazıyordu (`unwrap_or_default` ile eksik değer 0 çizilebiliyordu). Düzeltme: bütün
geometri konumları (değerler ve barlar) **yalnız kökenle** bağlanır; görünüm sayı taşımaz,
derleyici değeri okuyucunun sağlayıcısındaki olgudan çözer. Değeri olmayan geometri hiç
çizilmez; yerine "Kanıt değerleri veri yetkisi gerektirir: `<entitlement>`" metni çıkar.
Yetkili okuyucu için kurulmuş görünüm yetkisiz sağlayıcıyla derlenirse 0 değil
`not_found` döner. Test: `capsule_depth.rs::licensed_evidence_never_reaches_an_unentitled_reader`
(lisanslı olgu; yetkili görünümde ve yetkisiz görünüm + sahnede hiçbir kanıt değeri yok).

**Kalan risk (oturum bütünleştirmesine):** `show_finding` görünümü dokümana yazarken
otorite kökenli değeri sabit olarak yazar (`canon`, GRAMMAR §10 kural 4). Lisanslı olgunun
saklanan görünümünü yetkisiz okuyucuya göndermeden önce ayıklamak (ya da bu tür kökenlerde
sabit yazmamak) oturumun işidir; T7 kapsamında değiştirilmedi.

## 3. Oturuma dokunulan yerler (en küçük)

Yeni enum değerleri yüzünden derleme zorunlu kollar ve R3 görünümlerinin `show_finding`'den
geçmesi için gerekenler:

- `canon.rs`: `Position::Panel` (sabit değil), `Mark::Glyph` (`finding` ister), `x2`
  köken yuvası ve kanal denetimi, olgu alanı `geometry.…` (`depth::geometry_field`),
  `band`'in `y`/`y2`'si kökenle verilebilir.
- `explain.rs`, `summary.rs`: `glyph` adı, panel konumunun metni.
- `provider.rs` (`DocProvider`): `capsule()` ve `glyph()` (`depth::story_glyph`, son bar).
- `present.rs` değişmedi (derinlik kuralları `qp_grammar::depth` içinde dallanır).
- `qp-schema/src/protocol.rs`: `EncodingIn.x2`, `PositionIn::Panel` (tel biçiminin şeması
  saklanan tiple aynı kalsın diye; `wire_forms_mirror_stored_types` testi).

**Oturum bütünleştirmesine kalanlar:** motor oturumda çalıştığında `DocProvider::glyph`
motorun `glyph_spec`'ini versin (bugün aynı kuralın grameri: `story_glyph`, testle eşit);
`set_encoding` `x2`'yi tanımıyor (`gops.rs` kanal listesi); `summary`'de `x2` sabitleri
yazılmıyor; saklanan görünümlerin lisanslı değerleri (yukarıda); "kart paneli" ve "panele
göre konum" `docs/CONCEPTS.md`'ye eklenmeli (bu ajanın yazma alanı dışında); cümle ve
yetki notu Türkçe sabit (M2 realizer'ı ürüne girince dil seçimi).

## 4. Altın görseller (hepsi yeni; var olanlar değişmedi)

| Dosya | Ne gösteriyor |
|-------|---------------|
| `fixtures/golden/views/g1-glyphs-light.png`, `g1-glyphs-dark.png` | Altı tür × üç yön (satırlar) × beş aşama (sütunlar: aday kesik çizgi + boş ok başı; kesin dolu; gerçekleşti gri + ✓; gerçekleşmedi gri + ✗; geri çekildi gri + üstü çizili), 32 px; güç 0,6, güven 0,6 (soluk zemin), iki teyit (alt çubuklar). Bakıldı: etiketler sütunlarla çakışmıyor, aşamalar ayırt ediliyor; bilinen zayıflık M1'deki gibi teyit çubukları küçük. |
| `c2-capsule.png` (telefon 390×300) | Kapsül kartı: yukarı kırılım işareti (40 px), cümle dört satıra kırılmış ("SYN03 6 bar önce son 20 barın en yükseğini (135.56) yukarı kırdı; ertesi gün hacim 20 günlük ortalamanın 4.8 katına çıktı."), sol altta "Karne 3/7"; altta son 60 barın kapanış çizgisi, 135.56 seviyesi ve olgu noktası. Kart panelinde ızgara/eksen yok. |
| `c3-chart.png` (masaüstü) | Aynı kapsül grafik derinliğinde: mumlar, üç kırılım olgusunun 20 barlık seviye parçaları (135.56, 136.24, 139.54; her biri kendi bar aralığında), olgu barı üstünde işaret, olgu barında vurgu şeridi. |
| `c4-band.png` | Sıkışma: Bollinger bandı (4.48–4.70) yalnız geometrinin barlarında (226..245), mumların altında; nötr sıkışma işareti. |
| `c5-points.png` | RSI uyumsuzluğu (pozitif): iki dip noktası (bar 187, 4.56; bar 193, 4.43) ve aralarındaki eğik parça; dönüş işareti. |

`c1-sentence.json` (saat yüzeyi): yalnız cümle; görseli yok (`qp conform` ve
`capsule_views.rs::sentence_depth_is_the_capsule_sentence` sınar). Her görselin
`fixtures/views/<stem>.explain.txt` dosyası var (köken satırları: `geometry.0.from_bar` vb.).

**Örnek dokümanlar** `fixtures/views/docs/r3-{breakout,squeeze,divergence}.json`, üreteç
`crates/qp-render/tests/support/capsule_gen.rs` (`cargo run -p qp-render --example
make_capsule_views [-- --force]`): `qp_source::synthetic_market(4, 300, seed)`, gerçek
dedektörler (`qp_facts::default_detectors`) ve gerçek motor (`simulate_engine`); tema,
sonuç ve sıra kuralları fikstüre özel (M4 `kinddir` istisnası dahil tür+yön; %4 kapanış
hareketi/20 bar; sabit sıra) çünkü M3–M5 seçimleri ürün crate'lerinde henüz yok; cümle de
fikstüre özel şablon (her sayısı kanıt değeri). `capsule_views.rs::fixtures_are_generated`
dosyaların üreteçle bayt eşit olduğunu sınar (motor/dedektör değişirse bu test uyarır).

**Var olan altınlar ve explain dosyaları değişmedi:** `git diff dd62244 -- fixtures/golden
fixtures/views/*.explain.txt fixtures/views/v* fixtures/views/d*` boş; `cargo test -p qp-cli
--test views` ve `qp-render --test golden` geçiyor; verify `views` adımı (aşağıda).

## 5. verify.sh

`views` döngüsü sabit `VIEWS_LIST` kullanıyor; yeni kökler kendiliğinden girmiyor. Ancak
`qp conform fixtures/views` yeni beş dökümü zaten koşuyor, ve `cargo test` (`test` adımı)
yeni görselleri `capsule_views.rs` ve `glyph_sheet.rs` ile denetliyor. CLI ile de
denetlenmesi için **`scripts/verify.sh` `VIEWS_LIST`'e eklenecek satırlar**:

```
c2-capsule --present fnd.SYN03.breakout-up.144 --surface phone --width 390 --height 300
c3-chart --present fnd.SYN03.breakout-up.144 --surface desktop
c4-band --present fnd.SYN01.squeeze-neutral.245 --surface desktop
c5-points --present fnd.SYN01.reversal-up.195 --surface desktop
```

Bu dört satır CLI ile iki kez çizildi: bayt eşit, altınla eşit, explain eşit (aynı
döngünün betik kopyasıyla denendi). `g1-glyphs-*` görselleri bir döküm değil (yalnız
`cargo test -p qp-render --test glyph_sheet`); `c1-sentence` görselsiz.

## 6. Belirleyicilik ve WASM

- İki çizim bayt eşit: her yeni görsel için testlerde ve CLI'da sınandı.
- Yerli ↔ WASM: tarayıcı adımı (`QP_VERIFY_BROWSER=0`) koşulmadı; tarayıcı karşılaştırması
  yalnız R0 örneğini kapsıyor, yeni görünümleri kapsamıyor. Yeni çizim kodu yalnız temel
  işlemler, `floor`, `sqrt` (alfabe) kullanır.
- WASM (`cargo build -p qp-wasm --target wasm32-unknown-unknown --profile wasm`):
  önce ham 1 889 334 / gzip -9 598 450 bayt → sonra ham 1 943 806 / gzip 616 017
  (**+54 472 ham, +17 567 gzip**; alfabe + yeni işaret/derinlik kodu).

## 7. Doğrulama

`cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace` ve `QP_VERIFY_BROWSER=0 scripts/verify.sh` (`CARGO_INCREMENTAL=0`, 41aedc8 öncesi ağaç = 9425d12):
**verify: OK** (1089 s). Bütün adımlar PASS; `browser` SKIP (`QP_VERIFY_BROWSER=0`).
`views` 8 görünüm ×2 bayt eşit (yeni dörtlü listede değil, bkz. §5); `wasm` ham 1 943 806 /
gzip -9 616 022 B; `digest` arm64 = x86_64 `7b1e99d40889f83c`; `conform` 36 döküm × inproc,
serve, mcp.

## 8. Sonradan düzeltme: kart cümlesi ürün gerçekleştiricisinden

Dal `r3/s-views-fix`. §4'teki R3 örneklerinin cümlesi fikstüre özel elle yazılmış bir
şablondu (ondalık nokta, ürün cümlesi değil). Üreteç (`capsule_gen.rs::headline`) artık
ürün cümlesini kullanır: `qp_capsule::sentence::sentences_at(motor, &realize::Template,
kapsül, barlar, now, &Reader::entitled())` Türkçesi; motor kesilmiş barlarda yeniden
koşulur. Tohumlar, örnekler, barlar, olgular ve bulgular aynı. Yeni cümleler:

- kırılım: "SYN03 6 bar önce 135,56'daki direnci yukarı kırdı (kapanış 136,20): yukarı
  kırılım. 4 bar önce aynı aileden yeni bir kanıtla güçlendi. Artık geçerli değil. Karnesi
  henüz az örnekli: 7 seferin 3'ünde tuttu."
- sıkışma: "SYN01'de 6 bardır sıkışma (yön belirsiz) sürüyor. Henüz karnesi yok."
- uyumsuzluk: "SYN01'de 6 bar önce yukarı dönüş işareti çıktı: fiyat yeni dip yaparken RSI
  daha yüksek dip yaptı (fiyat 4,43, RSI 40,1). Hâlâ geçerli olup olmadığı bilinmiyor.
  Karnesi henüz az örnekli: 3 seferin 1'inde tuttu."

Yeniden üretilenler: `c1-sentence.json`, `docs/r3-*.json`, `c2-capsule.png` (c3–c5
görselleri ve bütün explain dosyaları cümleyi çizmediği için bayt eşit kaldı). Ürün
cümlesi kartta dört satırda kesiliyordu ("Artık geçerli …"); **kart düzeni en küçük
değişiklikle düzeltildi** (`depth::capsule_card_view`): kart/fiyat panel ağırlığı 2:3 →
3:2, cümle 13 → 12 px. Kırılım cümlesi artık altı satırda tam görünür. Bu kural yalnız
`story` taşıyan kapsüllerde çalışır; R2 altınları (`d1`, `d2`) değişmedi.

`QP_VERIFY_BROWSER=0 scripts/verify.sh` (`CARGO_INCREMENTAL=0`): **verify: OK**; `views` 12 görünüm ×2 bayt eşit, `test` ve `golden` PASS.
