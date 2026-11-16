# R4/H-M8910 — M8, M9, M10 ölçüm düzenekleri

Bağlayıcı şartname: `rounds/R4/experiments/harness/SPEC.md` §0–§4, §6, §7. Bu klasör bağımsız
bir Rust crate'idir (`r4-m8910-harness`; ana çalışma alanının üyesi değil, `crates/*`'a yol
bağımlılığı, kendi `Cargo.lock`'u; derleme kökteki yok sayılan `target/harness-m8910/`'a,
`.cargo/config.toml`), iki Playwright betiği (`shoot.mjs`: durağan sayfa, `demo_shots.mjs`:
canlı gösterim; ikisi de `shells/web/node_modules`'tan Playwright kullanır) ve okuyucu
ajanlarını başlatan `run_readers.sh`. Örnek kalıp M11 düzeneğidir (`../m11/`); `rng.rs`,
`io.rs` oradan kopyalandı, M11 düzeneği değişmedi.

## Bağımsızlık beyanı

Düzenek hiçbir alternatif görülmeden yazıldı. İçindeki tek tasarım her deneyin başvurusudur:
`r3` (M8: R3 kapsül kartı, 240 px; M9: `capsules` sırası; M10: T4/T5 geçişi). Alternatif
ajanlar düzeneği değiştirmez; yalnız ürünün gramerinde bir kayda bir tasarım ekler
(aşağıda). Seçim kuralları `src/score.rs`'te sonuç görülmeden yazılıdır.

## Alternatif ekleme (arayüz) — her M için tek dosya

Ortak kurallar: ad `[a-z][a-z0-9_-]*`, sıradan bir sözcük değil (istemlerde yasak sözcük
listesine kendiliğinden girer: `src/blind.rs` `design_names()`); belirleyici (aynı girdi →
aynı çıktı; hesapta yalnız IEEE temel işlemleri, gerisi `qp_grammar::detmath`); renkler
yalnız `Theme`'den; kabukta (`shells/web`) alternatife özel kod yok — kabuk adları
çekirdekten alır (`choices`), ayarlar sayfası seçenekleri kendiliğinden listeler. Bir
alternatif başka dosyaya dokunmak zorunda kalırsa dur ve orkestratöre yaz.

### M8 — kapsülün kapalı yüzü: `crates/qp-grammar/src/faces.rs`

1. `// --- M8 alternatives: view functions …` işaretinin altına yüzün görünüm kuralını yaz
   (ya da orada `mod <ad>;` bildirip `crates/qp-grammar/src/faces/<ad>.rs`'e koy):

   ```rust
   pub type FaceViewFn = fn(finding: &str, max_points: u32, p: &dyn DataProvider, id: &str)
       -> Result<View, GrammarError>;
   ```

   Gramerin bütün katmanları kullanılabilir (bkz. `depth::capsule_view`,
   `depth::capsule_card_view`): bulgu metin alanları (`headline` = okuyucunun cümlesi,
   `ledger` = karne satırı), `glyph` katmanı (işaret; alfabe ve sonuç stili bakış ayarından),
   olgu/seri katmanları. Başlık katmanı `DataSource::Finding` + `Mark::Text` + alan
   `headline` olmalı (düzenek başlık uzunluğunu buradan ölçer); işaret `Mark::Glyph`.
2. `pub static FACES` listesine, `// --- M8 alternatives: one line each` altına tek satır:
   `FaceDesign { name: "<ad>", height: <px>, view: <işlev>, headline: Some(<şablon>) },`
   **Yükseklik kuralı:** yüz kendi yüksekliğini bildirir (CSS px, telefon genişliği 390'da,
   120..=844). Kabuk sayıyı çekirdekten alır (`ShellCore::face_size_json` →
   `faces::height`), otorite de aynı adla derler; kabukta hesap yok.

3. **Başlık şablonu (SPEC E5; H-FP).** Alternatif başlığını kapsülün uzun cümlesinden değil,
   kendi şablonuyla kurar; şablon da `faces.rs`'te (ya da `faces/<ad>.rs`'te) yazılır:

   ```rust
   pub type HeadlineFn = fn(&HeadlineParts) -> String;   // qp_grammar::faces
   fn <ad>_headline(hp: &qp_grammar::HeadlineParts) -> String { … }
   ```

   `HeadlineParts` (`qp_grammar::provider`, kökten `qp_grammar::HeadlineParts`) okuyucunun
   dilinde düz metinler ve birkaç bayraktır; otorite ve kabuğun yansısı okuyucunun cümlesini
   yazdığı kuralla doldurur (`qp_capsule::realize::parts`; iç terim yok):

   | Alan | Anlamı | Örnek TR / EN |
   |---|---|---|
   | `lang` | metinlerin dili (`Language::Tr`/`En`); şablon dili buradan seçer | |
   | `instrument` | enstrüman etiketi (sembol) | `SYN03` |
   | `event` | ne oldu, kısa ad öbeği | yukarı kırılım / upside breakout |
   | `direction_word` | yön, tek sözcük | yukarı / up |
   | `when` | doğduğu an, periyoda ve takvime göre süre ifadesi | bugün, dün, cuma günü, 3 işlem günü önce, 2 saat önce / today, 3 trading days ago |
   | `moved` | son geçişin anı, aynı tür ifade | dün / yesterday |
   | `validity_phrase` | hâlâ geçerli mi; kapandıysa nasıl | hâlâ geçerli, artık geçerli değil, gerçekleşti, tutmadı, süresi doldu, geri çekildi / still holds, no longer holds, played out, did not play out, ran out of time, withdrawn |
   | `ledger_short` | doğumdaki karne, kısaltılmış ama dürüst | 7'de 3 tuttu (az örnek), karnesi yok / 3 of 7 played out (few cases), no record yet |
   | `stage` | aşama (`Stage`) | |
   | `validity` | geçerlilik (`Validity`; kapsül öğesi ve işaretle aynı değer) | |
   | `redacted` | okuyucu rakam görmez (veri yetkisi yok) | |
   | `ledger_n` | karne `n`; yetkisizde `None` | |
   | `few_cases` | `n < GLYPH_MIN_LEDGER_N` (20; `n = 0` dahil) — ürünün "az örnek" eşiği | |

   **"Şimdi" ve takvim:** süre ifadelerinin "şimdi"si serinin son barıdır (bitmiş olmasa
   da; günlük seride bugünün barı sürerken dünkü olay "dün"); periyot serinin `interval`'ı
   (gösterim `1d`, saatlik dünya `1h`), takvim enstrümanın takvimi, yoksa oynatmanınki
   (sentetik pazarın donmuş takvimi; yerel veride `XIST`). Anahtarın S2'si (ne zaman)
   **son geçişin** günüdür: ona karşılık gelen parça `moved`'dır, `when` doğuşun anıdır
   (çoğu açık olayda aynı; #3, #11'de farklı). Anahtarın "birkaç gün önce" kovası takvim
   günüdür; parça hafta sonunu aşan bir günlük farkı "cuma günü" der (#3).

   **Yetkisiz okuyucu:** bütün metinler rakamsızdır (`when` "birkaç gün önce" türü, karne
   nitel: "az örnek", "çoğunlukla tuttu"); şablon rakam eklememeli (enstrüman etiketindeki
   rakamlar dışında). Bugün yetkisiz okuyucunun yüzü bütün tasarımlarda düz metindir
   (Açık sorular 2), yani şablon orada çizilmez; yine de rakamsız yazılmalıdır.

   **Nerede geçerli:** yalnız o yüzün sahnesinde (`CompileOptions.face`, otoritede açılma
   derinliği `sentence`, kabukta `face_scene`). Orada bulgunun `headline` metin alanı
   şablonun çıktısıdır — görünüm kuralı başlık katmanını yine `DataSource::Finding` +
   `Mark::Text` + alan `headline` ile kurar, düzenek başlığı buradan ölçer. Parça yoksa
   (öykü ya da bar yok) cümle kalır. Protokoldeki kapsül `headline`'ı, açık kapsül, grafik ve
   dökümler değişmez. `r3`: `headline: None` (bugünkü cümle). **Sert eşik** başlık ≤ 80
   karakter (TR ve EN; `computed.json`).

   **Kayıt satırı yüz yüksekliğiyle birlikte:** `height` yüzün kendi yüksekliği (CSS px,
   390 genişlikte, 120..=844; E4: hesap ölçüsü olarak kaydedilir), `headline` şablon:

   ```rust
   FaceDesign { name: "<ad>", height: 180, view: <ad>_view, headline: Some(<ad>_headline) },
   ```

Seçim: bakış ayarı `face` (`style` hareketi, `ViewState.face`; gösterimde URL `?face=<ad>`
ya da Ayarlar → Yüz tasarımı). Bilinmeyen ad `invalid` (`/gesture/face`). `card` = `r3`'ün
eski adı (kabul edilir).

### M9 — akış düzeni: `crates/qp-grammar/src/feeds.rs`

1. `// --- M9 alternatives: layout functions …` altına düzen işlevini yaz
   (`fn(&FeedCtx) -> Vec<FeedRow>`). `FeedCtx`: `items` (okuyucunun `capsules` cevabı,
   otoritenin sırası ve izdüşümü), `lang` (başlık/işaret metinleri bu dilde), `seen`
   (son bakış, ms), `fresh` (öğe başına: son geçişi son bakıştan **sonra** bilinir oldu mu —
   çekirdeğin kuralı `ShellCore::known_at`; `CapsuleItem.t` geçişin **barının** zamanıdır,
   bilinme anı değil), `now`. Çıktı satırları: `FeedRow::Capsule { id, mark }` (yüz; `mark`
   kısa işaret metni, ör. "yeni") ve `FeedRow::Header { text }` (grup başlığı, metin
   çekirdekten). Var olmayan ya da yinelenen kapsül satırları düşer.
2. `pub static LAYOUTS` listesine tek satır: `FeedLayout { name: "<ad>", rules: <n>, layout: <işlev> },`
   `rules`: düzenin kendi kural/parametre sayısı (M9 son eşitlik bozucusu: az olan; `r3` 0).

Seçim: bakış ayarı `feed` (`?feed=<ad>`, Ayarlar → Akış düzeni). "Son bakış" `style`
hareketinin `seen` alanıdır (ms; `ShellCore::mark_seen` oynatma saatini yazar); bakış
durumundadır, günlüğe girmez. Kabuk satırları alt alta dizer (başlık `h4.fhead`, işaret
`.face .mark`); grup açma/kapama yok (düzenin gerektirdiği dokunma 0).

### M10 — açılma geçişi: `crates/qp-grammar/src/transitions.rs`

1. `// --- M10 alternatives: shared / frame functions …` altına iki işlev (ya da biri;
   öteki için `r3_shared` / `r3_frame` kullanılabilir):

   ```rust
   pub type SharedFn = fn(before: &Side, after: &Side) -> Vec<SharedElement>;
   // Side { depth, scene: Option<&SceneDesc>, price_series, marked_bar }
   pub type FrameFn = fn(shared: &[SharedElement], from: (f64, f64), to: (f64, f64), p: f64) -> Frame;
   // Frame { elements: [FrameElement { id, source: Before|After, src: PixelRect,
   //         rect: [left, top, w, h] (sayfa CSS px), opacity }], scene_opacity }
   ```

   `shared` otoritenin `Opening.shared`'ını verir (her geçişte: cümle → kapsül → grafik →
   kanıt ve geri). `anchors(side)` = `depth::transition_anchors` (`glyph`, `headline`,
   `ledger`, `price`, `bar:<i>`, `fact:<id>`, `evidence:<fact>`); bir alternatif sahneden
   kendi öğelerini de çıkarabilir (bir çapa adını kullanıyorsa aynı anlamda). `frame` ara
   kareyi verir: öğe başına kaynak sahne (eski/yeni), o sahnedeki kutu, sayfadaki kutu,
   opaklık; yeni sahnenin opaklığı. Kabuk yalnız bu değerleri uygular (pikselleri kopyalar);
   ilerleme `p`'yi de çekirdek verir (`transition_progress`, ham zaman damgasından).
   **Kare kuralı:** öğe listesi (sayı, sıra, kaynak, `src`) her `p` için aynı olmalı; `p` ile
   yalnız `rect` ve `opacity` değişir (kabuk kopyaları bir kez kurar, sonra yalnız konum ve
   opaklık uygular). Görünmeyecek öğe için opaklık 0.
2. `pub static TRANSITIONS` listesine tek satır:
   `TransitionDesign { name: "<ad>", duration_ms: <ms>, shared: <işlev>, frame: <işlev> },`

Seçim: bakış ayarı `transition` (`?transition=<ad>`, Ayarlar → Geçiş). Grafiğin kendisi
(`depth::chart_view`) bütün geçişlerde aynıdır: grafiği farklı işaretlemek bugün bu arayüzde
yok (Açık sorular 3).

### M11 — sonuç işareti (bağlantı)

M11 stili (`qp_grammar::glyph::outcome::STYLES`, M11 düzeneği) artık bakış ayarıdır:
`style.outcome` (`ViewState.outcome`; `?outcome=<ad>`, Ayarlar → Sonuç işareti); otoritenin
isabet derlemesi ve yansı `CompileOptions.outcome`'u bundan doldurur. Varsayılan `r3`.

## Komutlar

Hepsi depo kökünden (ya da bu klasörden `cargo run --release --`). Çıktılar
`rounds/R4/experiments/{M8,M9,M10}/raw/` altına (`--root <dizin>` başka köke; `--s8 <dosya>`;
`--salt <n>`). Ekran görüntüleri için önce: `scripts/demo.sh`'in yaptığı gibi
`wasm-pack build crates/qp-wasm --target web --profile wasm --no-opt --out-dir ../../shells/web/pkg -- --features replica`,
`cargo build --release -p qp-cli`, `cd shells/web && npm ci`.
Elle wasm-pack derlemesinden sonra `shells/web/pkg/sizes.json` de yazılmalı (sayfa onu ister; yoksa 404):
`printf '{"wasm_raw":%s,"wasm_gzip":%s}\n' "$(wc -c <shells/web/pkg/qp_wasm_bg.wasm)" "$(gzip -9 -c shells/web/pkg/qp_wasm_bg.wasm | wc -c)" > shells/web/pkg/sizes.json`
(`scripts/verify.sh` ve `scripts/demo.sh` aynısını yapar).

```sh
(cd rounds/R4/experiments/harness/m8910 && cargo build --release)
B=target/harness-m8910/release/m8910

$B list                          # kayıtlı tasarımlar (faces, feeds, transitions)
$B s8                            # S8 → rounds/R4/experiments/harness/s8.json (bir kez; ~20 sn)

# M8
$B m8 --face <ad>                # raw/screens/<kod>{,.dark,.gray}.png (2 sayfa), raw/keys/<kod>.json,
                                 # raw/key.json, raw/prompts/{F,H}_<kod>.txt, raw/<ad>/computed.{json,md}
./run_readers.sh m8 <ad> F 1 3   # persona × sayfa × 3 okuyucu (F/H × 1/2)
./run_readers.sh critic - - - 3  # 3 eleştirmen; her biri bütün tasarımların sayfalarını görür
$B score --exp m8                # raw/scores.{json,md} + seçim kuralı

# M9
$B m9-scenario                   # raw/scenario.json (bir kez)
$B m9 --feed <ad>                # raw/screens/<kod>-<k>.png (≤ 6 ekran), raw/keys/<kod>{,.rows,.positions}.json
./run_readers.sh m9 <ad> F - 3
$B score --exp m9

# M10
$B m10 --transition <ad>         # raw/<ad>/computed.json, raw/screens/<kod>-<i>-<j>.png, raw/keys/<kod>.json
./run_readers.sh m10 <ad> F - 3
$B score --exp m10

$B check-prompt <dosya>          # yasak sözcük denetimi (run_readers her istemde çağırır)
cargo test                       # düzeneğin kendi testleri (uçtan uca, tarayıcısız)
```

`run_readers.sh` ortamı: `SEED_BASE` (okuyucu k tohum `SEED_BASE + k − 1`; M10 dizilerin sırası,
eleştirmende harfler ve sıra), `MODEL` (varsayılan `sonnet`), `EXP_ROOT`, `DRY_RUN=1` (istem,
görüntü listesi ve meta yazılır; `claude` çağrılmaz). Cevabı olan okuyucu atlanır. Okuyucu:
`claude -p --model sonnet --tools Read --allowedTools Read --restricted --strict-mcp-config
--permission-mode dontAsk --no-session-persistence --output-format json`, çalışma dizini
yalnız `images.json`'da listelenen görüntüleri içeren yeni geçici dizin.

SPEC planı: M8 persona 2 × 2 sayfa × 3 = alternatif başına 12 okuyucu + 3 eleştirmen; M9 ve
M10 persona 2 × 3 = 6 okuyucu. Başvuru `r3` da aynı yolla okutulabilir (karşılaştırma için
sıralanır, seçilmez).

## Yöntem

### S8 (`src/s8.rs`, `harness/s8.json`)

Sentetik yeniden oynatma adım adım ilerletilir; her adımda otoriteye çıplak protokol
istemcisiyle (kitle süzgeci yok) bütün kapsüller sorulur; her durumun kuralına uyan **ilk**
kapsül adım + kapsül kimliğiyle kaydedilir. Tercih kuralları (sonuçtan önce): #10 dışında
karne `n ≥ 5`; #10 `n < 5`; bir kapsül tek duruma. **Anahtar motordan** (`CapsuleItem` +
serinin barları): yön, tür (sade karşılık tablosu `kind_plain`), ne zaman (takvim: geçiş
barının yerel günü ile oynatma saatinin yerel günü — 0 bugün, 1 dün, 2–6 birkaç gün, ≥ 7
bir haftadan önce; gün içi seride aynı gün bar sonundan ≤ 3 saat "birkaç saat önce", sonrası
"bugün"), geçerlilik + aşama, güven kovası (`ledger_at_birth`: ≥ %60, %40–60, < %40, n < 20),
tavsiye: yok. **E1:** güven kovasının "az örnek"i ürünün eşiğidir, `n < GLYPH_MIN_LEDGER_N`
(20); #10 (`n < 5`) bunun içinde kalır. Durum seçimindeki `n ≥ 5` tercihi değişmedi (yalnız
anahtarın S4 kovası).

Dünyalar: **gösterim** (`qp demo --synthetic --paused` ile aynı barındırıcı,
`qp_wasm::drive::parity_host`) ve **saatlik dünya** (aynı sentetik üreticinin
`synthetic_market_hourly` barları yerel dosya biçiminde `target/m8910/hourly/`'e yazılır,
aynı barındırıcı `interval: "1h"` ile oynatır; 12 enstrüman, 90 gün). Bulunanlar
(2026-11-16 koşusu):

| # | dünya | adım | kapsül | not |
|---|---|---:|---|---|
| 1 | saatlik | 90 | `cap.SYN03.breakout.up.137` | gösterimde günlük kapsül bir sonraki seansta doğar, "bugün" olamaz |
| 2 | gösterim | 19 | `cap.SYN11.breakout.down.119` | |
| 3 | gösterim | 46 | `cap.SYN06.extreme.up.123` | |
| 4 | gösterim | 442 | `cap.SYN03.squeeze.neutral.168` | |
| 5 | gösterim | 1 | `cap.SYN25.breakout.down.93` | |
| 6 | gösterim | 1 | `cap.SYN25.breakout.down.26` | |
| 7 | gösterim | 1 | `cap.SYN13.flow.up.60` | |
| 8 | — | — | **bulunamadı** | geri çekme yalnız itiraz/veri düzeltmesiyle (§20.7); oynatma üretmez |
| 9 | gösterim (`visitor`) | 1 | `cap.SYN28.continuation.down.118` | güven yolu `replica` (yetkisiz okuyucuya `server` kapsülü yok) |
| 10 | gösterim | 28 | `cap.SYN09.flow.neutral.122` | |
| 11 | saatlik | 9 | `cap.SYN12.breakout.down.119` | |
| 12 | saatlik | 417 | `cap.SYN05.reversal.down.222` | gösterimin motoru aşağı dönüşü teyit etmiyor |

### M8 (`src/m8.rs`)

- **Yüzler:** her bulunan durumun yüz sahnesi, web kabuğunun kendi işleviyle
  (`qp_wasm::shell::face_scene`), kabuk çekirdeğinin (`ShellCore`) yansısı o duruma
  ilerletilmiş hâlde; TR ve EN cümle, açık ve koyu tema. Yetkisiz okuyucunun (#9) yansısında
  bar yoktur: kabuk cümleyi metin olarak gösterir (ürün kuralı, bütün tasarımlarda aynı).
- **Sayfa:** sabit sırayla (tohum 1) karıştırılan durumlar → sayfa 1 ilk 6, sayfa 2 kalanı.
  Sayfa, gösterimin akış kabuğudur (`shells/web/style.css`, aynı işaretleme) — yüzler
  çekirdeğin DPR 2'de boyadığı sahneler, etiketler (A–F, kartın sağ üstünde) düzen öğesi.
  Başsız Chromium, 390 CSS px genişlik, DPR 2; açık, koyu ve gri (CSS `grayscale`) kopya.
  Adlar tohumlu karma kod (`blind::code`); eşleme `raw/key.json`, gizli anahtar
  `raw/keys/<kod>.json`.
- **(b) Hesap** (`computed.json`, sahne tarifinden): başlık (bulgunun `headline` metin
  katmanının metin öğeleri, birleştirilmiş) karakter/sözcük TR/EN en çok ve ortalama; metin
  öğesi sayısı ve metin kutusu alanı / yüz alanı (gömülü yazı tipinin ilerleme genişliği ×
  yazı boyu); **ek bilgi birimi** = başlık dışındaki her metin katmanı + işaret dışı grafik
  öğesi bulunan her panel; işaret kutusu alanı (px², pay); WCAG karşıtlığı (her metin
  öğesi, merkezinin altındaki zemine göre; işaretin **tanıtıcı parçası**: mürekkebi işaret
  kutusunun ≥ %2'si olan en yüksek karşıtlıklı ilkel, altındaki işaret plakasına ya da
  zemine göre; en düşük ilkel bilgi için); en küçük yazı. Sert eşikler SPEC §2(b).
- **(c) Eleştirmen:** `m8-critic --k <n>` her eleştirmene tasarımları farklı rastgele
  harflerle (`K L M N P R S T V Z`'den, onaltılık harf yok) ve tohumlu sırayla verir; dizine
  yalnız görüntüler (renkli + `-gri`) kopyalanır, harf → tasarım `critics/c<k>/key.json`'da.

### M9 (`src/m9.rs`)

Senaryo (`m9-scenario`): `t0` = adım 600'den itibaren ilk pencere, `t1 = t0 + 9` adım (bir
işlem seansı); pozisyon 2 (biri `t0`–`t1` arasında olumsuz geçişli: `weakened`,
`invalidated`, `retracted`), izleme listesi 8 (pencerede hareket edenler önce, sonra
alfabetik), `balanced`. Olumsuz geçişin okuyucunun akışına düştüğü kabuk çekirdeğiyle
denetlenir. Anahtar `t1`'deki akıştan: G1 = son bakıştan sonra bilinir olan geçişler
(`"<enstrüman> <yön>"`, yön kapsülün yönü), G2 = olumsuz geçişli pozisyon ve ne, G3 = en
yüksek güven kovasındaki (n ≥ 5) bütün kapsüllerin etiketleri. Ekranlar canlı gösterimden:
URL ile düzen, Ayarlar sayfasında kitle (kullanıcı gibi), `t0`'da `window.qp.seen()`,
kontrol çubuğuyla `t1`, akış ekran ekran kaydırılır (en çok 6); etiketler (iki ünsüz,
tohumlu) düzenekçe basılır; hangi etiketin hangi ekranda (≥ %50 görünür) olduğu
`keys/<kod>.positions.json`'a yazılır → anahtardan en az kaydırma.

### M10 (`src/m10.rs`)

S8 #1, #3, #6, #12. #1 ve #12 saatlik dünyadadır, canlı gösterim onları gösteremez: M10
onların **gösterim yedeklerini** kullanır (`s8.json` `substitutes`: `101` = yukarı kırılım,
açık, dün, adım 10, `cap.SYN06.breakout.up.118`; `112` = aşağı dönüş, açık, dün, teyitsiz,
adım 10, `cap.SYN28.reversal.down.120`) — bkz. Açık sorular 1. Her durumda kabuk çekirdeğiyle: `open` (yüze dokunmanın gönderdiği) →
kapsül; sahneye dokunma (`tap_at`) → grafik. Hesap: kapsül → grafik geçişinde ortak öğe
sayısı, alan payları (önce/sonra), merkez kayması (ort./en çok), akıştan grafiğe `input`
mesajı sayısı. Bölge etiketleri: grafiğin çapalarından — doğru olan olgunun barı (sütun
ortası) × olgunun seviyesi (`evidence:` / `fact:` kutusu, yoksa panel ortası); üç çeldirici
fiyat panelinde başka barlarda (genişliğin ≥ %18'i aralıklı, tohumlu); A–D sırası tohumlu.
Ekranlar: kapsül ekranı, `p = 1/3` ve `2/3`'te dondurulmuş ara kare (çekirdeğin karesi,
`window.qp.freeze`), etiketli grafik ekranı.

### Puanlama ve seçim (`src/score.rs`)

Cevap normalleştirilir (Türkçe küçük harf, tırnak/nokta, boşluk) ve anahtarla birebir
karşılaştırılır; serbest metin puanlanmaz. Ağırlık `(2·F + 1·H) / 3`. Başvuru sıralanır,
seçilmez.
- **M8:** (1) sert eşiklerden birini geçemeyen elenir (hepsi geçemezse uygulanmaz,
  yazılır); (2) tam doğru oranı (yüz başına S1–S5 hepsi); (3) fark < 0,05 → eleştirmen
  ortalama sırası, o da eşitse kısa ortalama başlık (TR+EN); ilk iki `top2`. Şans düzeyi
  soru başına `1/seçenek sayısı`, tam doğru için çarpımı.
- **M9:** ortalama doğruluk (G1 F1, G2, G3); fark < 0,05 → ortalama kaydırma + dokunma (doğru
  cevaplarda kaydırma = dayanılan en son ekran − 1; dokunma = "açmam gerekirdi" sayısı + 0);
  sonra `rules` az olan.
- **M10:** G1; fark < 0,05 → G2+G3; sonra ortak öğe alan payı (yüksek); sonra dokunma (az).
- Yeniden puanlama ham dosyalardan bayt eşit (test).

## Ekran görüntüleri ve belirleyicilik

Her tasarımın sahneleri çekirdekte, DPR 2'de boyanır (`qp_render::render_scene_rgba_scaled`;
kabuk `devicePixelRatio`'yu çekirdeğe verir, ölçeği çekirdek seçer). İki koşu bayt eşittir
(aynı makine; DOM metni sistem yazı tipiyle, bu yüzden platformlar arası değil — T5 ÖK-T5-6).
Sayfa görüntülerinin içindeki yüz/sahne pikselleri her platformda bayt eşit.

## Sapmalar ve yorumlar (SPEC'e göre; orkestratöre)

1. **M8 sayfası 844 px'e sığmaz:** 6 yüz × 240 px (`r3`) + aralar ≈ 1.340 CSS px. Sayfa
   telefon genişliğinde **tam boy** (kaydırmalı uzun görüntü) alınır; kısa yüzlü alternatifin
   sayfası 844'e yaklaşır. SPEC "tek ekran görüntüsü" ile "390×844" birlikte uygulanamıyor.
2. **M8 sayfası canlı gösterim anı değildir:** S8 durumları farklı anlarda ve #9 başka okuyucu
   (yetkisiz) ister; tek bir gösterim anı hepsini tutamaz. Sayfa gösterimin akış kabuğu
   (aynı CSS ve işaretleme) + çekirdeğin o anlardaki yüz sahneleridir.
3. **S8 #8 (geri çekildi) bulunamadı**, kurulamadı (düzeltilmiş bar sağlayıcısı gerekir).
   Sayfa 2'de 5 yüz var; S3'ün "kapandı: süresi doldu ya da geri çekildi" seçeneği #7 ile
   sınanıyor.
4. **S4 kovası ile ürünün eşiği:** SPEC E1 ile kapandı (H-FP): anahtarın "az örnek" kovası
   `n < GLYPH_MIN_LEDGER_N` (20). `s8.json`'da S4'ü değişenler #4 (n 7), #6 (n 5), #7 (n 5),
   #12 (n 17): hepsi "az örnek var, bilinmiyor". M9 G3'ün "n ≥ 5" kuralı (SPEC §3) değişmedi.
5. **`alt` yasak sözcüğü:** M11 düzeneğindeki gibi çıplak Türkçe "alt" serbest, kimlik
   biçimleri (`alt-`, `alt_`, `/alt`) yasak. Ayrıca `face`, `feed`, `transition`, `outcome`,
   `card`, `key`, `s8` yasak.
6. **M9 G1 "yön":** kapsülün yönü (olumsuz geçişte bile); "değişti" = son bakıştan sonra
   bilinir oldu (`known_at`).
7. **M9 "düzenin gerektirdiği dokunma"** 0: kabukta grup açma yok; bir alternatif açılır
   grup isterse arayüz genişlemeli.

## Açık sorular

1. **M10 durumları:** SPEC #1 ve #12'yi ister; ikisi yalnız saatlik dünyada var (gösterim
   günlük seride "bugün" olayı ve aşağı dönüş teyidi üretmiyor). Gösterimi saatlik dünyayla
   açmak `qp demo`'ya bir bayrak ister (ürün değişikliği); şimdilik M10 yedeklerle (#101,
   #112; aynı tür ve yön, "bugün" yerine "dün", #112'de teyit yok) koşar.
2. **M8 #9:** yetkisiz okuyucunun yüzü bütün tasarımlarda düz metin (kabuk kuralı
   `feed_json` `scene: false`); M8 bu durumda tasarımları ayırmaz.
3. **M10 grafik işaretlemesi** (SPEC §4 "grafiğin nasıl işaretleneceği"): grafik görünümü
   `depth::chart_view` bütün geçişlerde ortak; geçiş tasarımına bir grafik kuralı eklemek
   otoritenin `present` yolunu değiştirir (paralel T6 işiyle çakışır) — bu turda yok.
