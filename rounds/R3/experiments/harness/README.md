# R3/H — M1 ve M2 ölçüm düzeneği

Bağlayıcı şartname: `SPEC.md` ("Ortak ilkeler", "M1", "M2"). Bu klasör bağımsız bir Rust
crate'idir (`r3-harness`; ana çalışma alanının üyesi değil, `crates/*`'a yol bağımlılığı,
kendi `Cargo.lock`'u) ve okuyucu ajanları başlatan `run_readers.sh`.

## Bağımsızlık beyanı

Düzenek hiçbir alternatif görülmeden yazıldı; içindeki tek gerçekleme `control`, aday değil.
`m1::control::Control` (daire + türün baş harfi) ve `m2::control::Control` (makine kodu
basan cümleci) yalnız düzeneğin kendini sınaması içindir; bilerek en sade biçimdedir ve
hiçbir aday onlardan başlamamalıdır. Alternatif ajanlar düzeneği değiştirmez; yalnız trait'i
uygular ve kayıt listesine bir satır ekler.

## Komutlar

Hepsi bu klasörden (`rounds/R3/experiments/harness/`). Çıktılar
`rounds/R3/experiments/M1/raw/<alt>/` ve `.../M2/raw/<alt>/` altına yazılır
(`--root <dizin>` ile başka köke).

```sh
cargo build --release
B=./target/release/harness

# M1 — işaret alfabesi
$B m1 list
$B m1 measure --alt <ad>                       # distinct.json + distinct.md
$B m1 sheets  --alt <ad> --seed 1 [--cond nolegend|legend|learn]   # sayfa + gizli anahtar
./run_readers.sh <ad> nolegend 3               # 3 taze okuyucu (tohum 1, 2, 3)
./run_readers.sh <ad> legend 3
./run_readers.sh <ad> learn 3
$B m1 score --alt <ad>                         # scores.json + scores.md

# M2 — cümle üretimi
$B m2 list
$B m2 measure --alt <ad> [--passes 20]         # auto.json + auto.md + cost.json
$B m2 inputs  --out girdiler.json              # dil modeli alternatifi için girdiler (doğru cevapsız)
$B m2 sheets  --alt <ad> --lang tr --seed 1    # 24 cümlelik istem + gizli anahtar
./run_readers.sh m2 <ad> tr 2                  # alternatif × dil başına 2 okuyucu
./run_readers.sh m2 <ad> en 2
$B m2 score --alt <ad>                         # scores.json + scores.md

cargo test                                     # düzeneğin kendi testleri
```

`run_readers.sh` ortam değişkenleri: `SEED_BASE` (ilk tohum, varsayılan 1; okuyucu k,
`SEED_BASE + k − 1` tohumlu sayfayı görür), `MODEL` (varsayılan `sonnet`), `DRY_RUN=1`
(yalnız sayfa ve istem; `claude` çağrılmaz). Cevabı olan okuyucu dizini atlanır (yeniden
harcama yok).

## Alternatif ekleme

**M1 (alfabe).** Alternatif kendi crate'inde, `rounds/R3/experiments/M1/<alt>/`:
`qp_grammar::glyph::Alphabet`'i uygulayan bir tip ve en çok 120 kelimelik
`pub const LEGEND: &str` dışa verir. Düzenekte iki satır:

1. `Cargo.toml`, "M1 alternatives" yorumunun altına:
   `m1-<alt> = { path = "../M1/<alt>" }`
2. `src/m1/registry.rs`, `all()` içine:
   `entry(Box::new(m1_<alt>::Alphabet), m1_<alt>::LEGEND),`

`--alt` adı alfabenin `name()` değeridir (`[a-z0-9_-]+`, `m1`/`m2` olamaz). Açıklama
yalnız `legend` koşulunda okuyucuya gider; 120 kelimeyi aşarsa `sheets` hata verir.
`qp-grammar`'daki trait değişmez; açıklama düzenek tarafında `(alfabe, açıklama)` çifti
olarak tutulur.

**M2 (kurallı cümleci).** Alternatif kendi crate'inde, `rounds/R3/experiments/M2/<alt>/`,
`qp_capsule::Realizer`'ı uygular. Düzenekte:

1. `Cargo.toml`: `m2-<alt> = { path = "../M2/<alt>" }`
2. `src/m2/registry.rs`, `all()` içine: `Box::new(m2_<alt>::Realizer),`

**M2 (dil modeli).** Crate gerekmez. Alternatif `harness m2 inputs --out …` ile girdileri
(durum kimliği + `SentenceInput`'un tam değerleri; doğru cevap yok) alır, cümleleri
kendisi üretir ve `rounds/R3/experiments/M2/<alt>/sentences.json` dosyasına yazar:

```json
{"sentences": {"s01": {"tr": "…", "en": "…"}, "k26": {"tr": "…", "en": "…"}}}
```

Çağrı başına süre ve belirteçleri `rounds/R3/experiments/M2/<alt>/calls.json` dosyasına
yazar: `[{"id": "s01", "lang": "tr", "seconds": 1.2, "input_tokens": 900, "output_tokens": 40}]`.
Düzenekte tek satır (`src/m2/registry.rs`):
`Box::new(FileRealizer::new("<alt>", alt_file("<alt>"))),`
`FileRealizer` girdiyi parmak iziyle durum kimliğine eşler (küme içinde tekil olduğu test
edilir); eksik cümle boş dize döner ve ölçüde başarısız sayılır. Dil modeli tohumla tekrar
üretilmediği için belirleyicilik ölçüsü yalnız dosyayı okuyan sarmalayıcıyı ölçer (1,00);
asıl belirleyicilik için alternatif aynı girdiyle ikinci üretimini ayrıca vermelidir.

## Yöntem

### M1 (a) hesapla ayırt edilebilirlik — `m1 measure`

- Glif kümesi (`src/m1/glyphset.rs`): çekirdek `K` = tür × yön; aşama `final`, güç 0,6,
  güven 0,6, teyit 1, tazelik 1, sunucu işareti yok. Kanal çiftleri her `K` üyesinden:
  yön (aynı türün `K`'deki yönleri), aşama (5 değer, bütün çiftler), güç (0,2 / 0,9),
  teyit (1/2/3), güven (yok / 0,3 / 0,8), tazelik (1 / 0,3), sunucu işareti (yok/var).
- Çizim: `Alphabet::draw` → `qp_render::render_scene_rgba` (altın görsellerle aynı CPU yolu),
  kutu `size × size`, tema arka planı.
- Koşullar: 16/24/32 px × açık/koyu tema × normal/döteranopi/protanopi/tritanopi/gri = 30.
- Piksel yolu: sRGB → doğrusal RGB → Gauss bulanıklığı σ = 0,5 px (yarıçap 2, normalize,
  kenar kıskaçlı) doğrusal RGB'de → görme benzetimi doğrusal RGB'de (Machado, Oliveira &
  Fernandes 2009, şiddet 1,0 matrisleri; sonra [0, 1]'e kıskaç; gri = doğrusal sRGB
  parlaklığı Y) → XYZ → CIELAB (D65). Bulanıklık ve matris doğrusal olduğundan sıraları
  kıskaç dışında fark etmez.
- Çift uzaklığı: dokuz tamsayı kaydırmanın (±1 px) en küçüğü; her biri bütün piksellerde
  ΔE76'nın ortalaması (dışarıdan kayan pikseller arka plan rengi).
- Rapor: koşul başına `K`'nin en zayıf çifti ve ortalaması, her kanalın en zayıf çifti ve
  ortalaması; kanal başına en kötü koşul; kutu dışına taşan ilkel sayısı (sözleşme: 0).
  **Ana ölçü: en kötü koşulda `K`'nin en zayıf çifti.** Değerler 4 ondalığa yuvarlanır.

### M1 (b) okuma, (c) öğrenme — `m1 sheets`, `run_readers.sh`, `m1 score`

- `nolegend` / `legend`: 4×4 ızgara, A1…D4 (satır harfi, sütun rakamı; etiket işaretin
  üstünde): `K`'nin tamamı (12) + tohumlu 4 kanal çeşitlemesi (aşama, güç, teyit, güven,
  tazelik ya da sunucu işaretinden biri değişmiş bir `K` glifi), tohumlu sıra.
- `learn`: `examples.png` 5 etiketli örnek (en az 4 tür; cevabı yanında yazılı), sonra
  `test.png` 12 yeni glif (A1…C4); testteki hiçbir tür × yön × aşama birleşimi örneklerde
  yoktur. Havuz: `K` × 5 aşama × güç {0,2; 0,9}.
- Her glif 32 px çizilir, 4× en yakın komşuyla büyütülür (128 px hücre), açık tema.
- Cevap alanları: `kind`, `direction` (up/down/neutral), `strength` (low/high; doğru
  sınıf: güç < 0,5 → low), `phase`. İstem altı türü ve aşamaları birer satırlık anlamlarıyla
  verir (CAPSULE §4 sözcükleri); işaretlerin kuralını vermez.
- Okuyucu: `claude -p --model sonnet`, yalnız `Read` aracı, `--restricted` (dosya araçları
  çalışma dizinine kapalı), `--strict-mcp-config`, `--permission-mode dontAsk`,
  `--no-session-persistence`, `--output-format json`; çalışma dizini içinde **yalnız sayfa
  görselleri** olan yeni bir geçici dizin. Gizli anahtar, diğer koşullar ve diğer
  alternatifler oraya hiç kopyalanmaz. Kayıt: `readers/<koşul>_s<tohum>_r<k>/`
  (`prompt.txt`, ham `answer.json`, `stderr.txt`, `meta.json`: okuyucu kimliği, tohum, model).
- Puan: alan başına doğruluk ve tam doğru oranı; şans düzeyi (tür 1/6, yön 1/3, güç 1/2,
  aşama 1/5, tam 1/180) ve çoğunluk tabanı (anahtardaki en sık doğru değeri hep söylemek).
  Cevapsız ya da ayrıştırılamayan madde yanlış sayılır. Puanlama yalnız ham dosyaları okur;
  yeniden puanlama bayt düzeyinde aynıdır (test edilir).

### M2 — `m2 measure`, `m2 sheets`, `m2 score`

- Durum kümesi (`src/m2/situations.rs`, sabit tohum 3): zaman türü (olay/durum/beklenti) ×
  tür (6) = 18 hücre, her hücrede bar-önce 0, 1, 5, 30 (dengeli); 72 durumda her aşama
  (doğdu, teyit edildi, zayıfladı, gerçekleşti, geçersiz, süresi doldu) 12, `redacted` 18,
  karne küçük (n = 8) / büyük (n = 140) 36'şar kez, tohumlu karıştırmayla. `still_holds`:
  olay → yok; durum → geçersiz/süresi dolmuşsa hayır, bar-önce 0 ise evet, değilse tohumlu;
  beklenti → geçersiz/süresi dolmuşsa hayır, gerçekleşmişse yok, değilse evet (2/3) ya da yok.
  Ayrıca **K26**: RSI 3 bar önce 70'i yukarı kesti, şimdi 55 (`still_holds = false`,
  `current = 55`). `Story`/`Fact` değerleri motor biçimindedir (kimlik
  `fx.<enstrüman>.<dedektör>.<kod>.<bar>`, aileler sıralı, geçmiş `born` + aşama geçişi,
  doğumdaki karne). Cümlede görülebilecek her sayı bu değerlerden gelir.
- Doğru cevap: **ne** = tür + yön; **ne zaman** = `past` + bar-önce (olay, bitmiş durum),
  `now` (süren durum), `conditional` (beklenti); **hâlâ geçerli** = geçersiz/süresi dolmuşsa
  `no`, değilse `still_holds`'tan (`yes`/`no`/`unknown`).
- (1) Zaman doğruluğu, belirleyici sınıflayıcı (`src/m2/lexicon.rs`): beklenen sınıf
  `event` (geçmiş eki + zaman başvurusu), `state_holding` (şimdiki zaman, "artık değil"
  yok), `state_ended` (geçmiş + "artık … değil"/"no longer"), `expectation` (koşul eki /
  "if"). İhlaller listelenir; zaman başvurusunun gerçek bar-önceye eşitliği ayrıca raporlanır.
- (2) Belirleyicilik: her cümle iki kez üretilir; bayt eşitlik oranı.
- (3) Sızıntı (`redacted`): herhangi bir rakam ya da sayı sözcüğü (sözlük) = sızıntı;
  tanı için hangi lisanslı değerin yuvarlanmış biçimde geçtiği de yazılır. Uydurma sayı
  (açık durumlar): her sayı belirteci bir motor yuvasına eşit olmalı. Biçim kuralı: `k`
  ondalıklı belirteç (`,` ya da `.` ondalık; üçlü gruplar binlik) bir `v` yuvası için
  `round(v, k)` ya da `round(|v|, k)`'ye eşit. Yuvalar: baş ve son olgunun kanıt değerleri,
  güncel değer, karne n / gerçekleşen / gerçekleşmeyen / yüzde, bar-önce, son geçişten beri
  bar, ufuk, `within_bars`, güç ve güç×100, aile sayısı, kademe, bar indeksleri.
- (4) Anlama: 24 cümle = K26 + tohumlu 23 (seçim yalnız tohuma bağlı, dile değil); istem
  İngilizce, cümleler kendi dilinde; okuyucu `claude -p --model sonnet`, hiç araç yok, boş
  geçici dizin. Puan: `kind`, `direction`, `what` (ikisi), `when` (kategori; gerçek "0 bar
  önce" için `now` da kabul), `when_exact` (geçmişte bar-önce de eşit), `still_valid`,
  `full`; şans ve çoğunluk tabanı; K26 maddesi ayrıca.
- (5) Maliyet: `cost.json` — işlem içi `sentence()` çağrısının cümle başına ns'si (20 geçiş;
  saat ölçüsü, belirleyici değil, hiçbir bayt eşitlik iddiasına girmez); dil modeli için
  alternatifin `calls.json`'u özetlenir.

## Sapmalar ve yorumlar (SPEC'e göre)

- **`K` 12 elemanlı.** SPEC "devam↑↓, dönüş↑↓, kırılım↑↓, sıkışma·, aşırılık↑↓, akış↑↓·
  (13)" diyor; liste 2+2+2+1+2+3 = 12 birleşim veriyor. Düzenek listeyi kullanır; okuma
  sayfasında "K'den 12" böylece `K`'nin tamamıdır.
- Sözlük dosyası SPEC'teki `harness/m2_lexicon.rs` yerine `src/m2/lexicon.rs` (görev tanımı).
- Okuma sayfaları açık temada; ayırt edilebilirlik iki temada ölçülür.
- İstemler altı türün ve beş aşamanın birer satırlık anlamını verir (sözcük dağarcığı;
  işaret kuralı değil).
- `legend` ve `nolegend` aynı tohumda aynı glifleri gösterir; okuyucular farklı tohum alır.

## Bilinen sınırlar

- **Görme modeli ≠ insan gözü.** Okuyucu bir dil modelidir; küçük pikselleri insan gibi
  görmez. Bu yüzden glifler 4× en yakın komşuyla büyütülür; 16 px'teki gerçek okunurluğu
  okuma testi değil, (a)'daki hesap ölçer. Okuma testi ön elemedir (K39); son söz kullanıcı
  testindedir.
- 4× büyütme pikselleri keskinleştirir; ekranda 32 px'te görülenden daha okunaklıdır.
- ΔE76 algısal olarak düzgün değildir; ortalama ΔE ince ama anlamlı farkları (ör. küçük bir
  ok ucu) seyreltir. Machado matrisleri tam şiddettir (anomali değil, -opi).
- Zaman sınıflayıcısı yüzey işaretlerine bakar, sözdizimi çözümlemez: TR'de eksiz ad
  yüklemi ("RSI 70'in üstünde.") şimdiki zaman işareti sayılmaz; `-dı/-di` ile biten bilinmeyen
  sözcükler geçmiş sayılır; EN'de `-ed` ile biten her sözcük (istisna listesi dışında)
  geçmiştir. Makine kodu gibi İngilizce sözcük taşıyan metin de işaret verebilir (bkz.
  `control`'ün EN sonucu). Kısa TR sayı sözcükleri (`iki`, `üç`, `beş`, `on`, `bin`, `yüz`,
  `altı`) yalnız tam sözcük olarak sayılır; `bir`/`one`/`a` hiç sayılmaz.
- Uydurma sayı denetimi tarih, kısaltma ("1,2M") ve yazıyla sayıyı ayrıştırmaz; tarih
  uydurma sayılır.
- Anlama testi cevabı açıkça yazan metni ödüllendirir: makine kodu basan `control` bile
  yüksek puan alır. Anlama puanı tek başına doğallığı ölçmez; zaman doğruluğu ve sızıntı
  ölçüleriyle birlikte okunmalıdır.
- Okuyucu süreci kullanıcının genel `~/.claude` belleğini yükleyebilir (`--restricted`
  ayar dosyalarını yok sayar; çalışma dizini boş geçici dizin olduğundan proje belgeleri
  yüklenmez).
- Kayan nokta: Lab dönüşümü `powf`/`cbrt` kullanır; aynı makinede bayt eşit, platformlar
  arası son basamaklar farklı olabilir (değerler 4 ondalığa yuvarlanarak yazılır).

## Kontrol sonuçları (öz-sınama, aday değil)

M1 `control` (`M1/raw/control/`):

- Ana ölçü: **0,00** (`continuation.up` ↔ `continuation.down`, 16/açık/normal) —
  beklenen: kontrol yönü kodlamaz. Her kanalın en zayıf çifti 0,00 (yalnız tür kodlanır).
  `K` ortalamasının en düşüğü 0,69 (16 px, gri); 16 px açık normalde 1,95.
  Kutu dışı ilkel: 0.
- Bir okuyucu, `nolegend`, tohum 1 (16 madde): tür 1,00; yön 0,44 (şans 0,33); güç 0,56
  (şans 0,50, çoğunluk 1,00); aşama 0,38 (şans 0,20, çoğunluk 1,00); tam 0,12. Yeniden
  puanlama bayt eşit (sha1 aynı).

M2 `control` (`M2/raw/control/`):

| ölçü | tr | en |
|---|---:|---:|
| zaman doğru (73) | 0,00 | 0,45 (yüzey eserleri: `stage=confirmed`, `holds=`, `bars_ago`) |
| K26 zaman doğru | hayır | hayır |
| belirleyicilik | 1,00 | 1,00 |
| sızıntı / redacted | 18/18 | 18/18 |
| uydurma sayılı cümle / açık | 0/55 | 0/55 |
| ns/cümle (saat ölçüsü) | ~100 | ~100 |

- Bir okuyucu, TR, tohum 1 (24 madde): tür 1,00, yön 1,00, ne zaman 0,92, hâlâ geçerli
  1,00, tam 0,92; K26 tam 0/1 (okuyucu bitmiş durumu `now` saydı).

Bir okuyucu çağrısının maliyeti (`answer.json` içindeki `total_cost_usd`, model
`claude-sonnet-5-5`): M1 sayfa okuması 0,018 $, ~7 s; M2 24 cümle 0,028 $, ~14 s.
SPEC'in tam okuyucu planı (M1: 3 koşul × 3 ajan = 9; M2: 2 dil × 2 ajan = 4) alternatif
başına yaklaşık 0,16 + 0,11 ≈ 0,3 $.
