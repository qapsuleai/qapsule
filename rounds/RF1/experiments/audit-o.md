# RF1/Ö denetimi — `rf1/o-design` (RD2 denetim koşulu K1: P8 çizilen işaret, P9 tarayıcısız)

- Denetlenen: `rf1/o-design` @ `27cfc883` (2 commit: `7e08fc7a`, `27cfc883`); dal değiştirilmedi, birleştirilmedi.
- Ortam: ayrı worktree (`.claude/worktrees/audit-o`, ayrık HEAD), ayrı derleme klasörü
  `target/wt/audit-o` (K87). Denetim sonunda ikisi de silindi.

## Hüküm: **GEÇTİ**

Engelleyici bulgu yok. Üç düşük önemli bulgu ve iki bilgi notu aşağıda.

## Adımlar

### 1. Worktree
`git worktree add .claude/worktrees/audit-o rf1/o-design --detach` — tamam; denetim sonunda
`git status --porcelain` boş (koşular ağaçta artık bırakmadı).

### 2. Değişiklik kapsamı (`git diff main...rf1/o-design --name-status`)

| Durum | Dosya |
|---|---|
| M | crates/qp-grammar/tests/design_check/alphabet.rs |
| A | crates/qp-grammar/tests/design_check/layout.rs |
| M | crates/qp-grammar/tests/design_check/main.rs |
| A | crates/qp-grammar/tests/design_check/marks.rs |
| M | crates/qp-grammar/tests/design_check/record.rs |
| M | crates/qp-grammar/tests/design_check/scene.rs |
| M | docs/design/CONTRACT.md |
| A | fixtures/design/desk-marks.json |
| A | fixtures/design/docs/syn03-breakout.json |
| M | scripts/design-check (yalnız yorum ve `--help` satır aralığı `2,15p` → `2,19p`) |

- `crates/*/src` altında değişiklik **yok**.
- `fixtures/golden`, `fixtures/views`, `fixtures/protocol`, `fixtures/compute`, `fixtures/schema`,
  `fixtures/design/desk.json` **değişmedi**. Yeni dosyalar yalnız `fixtures/design/` altında.
- `fixtures/design/docs/syn03-breakout.json`, `fixtures/views/docs/r3-breakout.json` ile bayt
  bayt aynı (`cmp`: IDENTICAL).
- Kapsam: **uygun**.

### 3. Komutlar

| Komut | Sonuç |
|---|---|
| `cargo fmt --all -- --check` | rc 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | rc 0 |
| `cargo test -p qp-grammar` | rc 0 (182 geçti, 0 kaldı) |
| `qp conform fixtures/design` | rc 0 — `PASS desk-marks`, `PASS desk`, 2/2 |
| `scripts/design-check` (iki kez) | rc 0 / rc 0 |
| `scripts/design-contract --check` | rc 0 (fark 0) |
| `scripts/arch-check` | rc 0 — 0 hata, 11 adlı istisna, taban çizgisi içinde |

`tavek`: P8 dark/light `20 / 20 (1 glyph)` PASS (çizilen > 0), P9 `5 / 3 / 7`
(`Ç1 mockup + desk`) PASS. `tavek-tel`: P8 `24 / 24 (1 glyph)`, P9 `n.a.` (dokunmatik).

### 4. Ölçümün değerlendirmesi

**P8 çizilen değer gerçekten ölçülüyor mu?** Evet.
- `marks.rs::drawn`, derlenmiş sahnede `Mark::Glyph` katmanlarının öğe sayısını ve
  her öğenin vuruş kutusunun (`hit`) ve katmanın çizilen ilkellerinin (Rect/Polyline/Fill/
  Circle) dikey açıklığının en büyüğünü alır. Sabit değer yok.
- Değer kayıttan bağımsız: eski görünüşlerde kayıt 32 iken çizilen `oltan` 28, `r4` 24 —
  yani sayı kayıttan kopyalanmıyor, sahneden geliyor. Yeni görünüşlerde çizilen = kayıt
  (20, 24), çünkü derleyici `Look::mark_px` ile boyu `mark_max_px`'e kırpar
  (`compile/draw/marks.rs:291`); bu beklenen davranış.
- `n >= 1` koşulu eklendi: işaret çizilmezse satır KALIR; çizilen değerin sessizce 0'a
  düşmesi (RD2 B1 sorunu) artık geçemez.
- Görünüm `marks`, fikstürün kendi `put_view` adımından okunuyor; bulgu ve kapsül
  belgeden (`syn03-breakout.json`) sağlayıcıya yükleniyor.

**P9 galerinin `layoutDump` kuralını izliyor mu?** Evet, belgelenmiş bir farkla.
- Bölge: galeri `[data-region]` öğelerini DOM'da sayar; `layout.rs` aynı dosyayı seçer
  (`mockups/<görünüş>/c1.html`, yoksa `_base/c1.html`; galeri `design-gallery:194-195` ile
  aynı kural) ve kaynakta `data-region=` sayar. `tavek/c1.html` 5 öğe; `_base/frame.js`
  bölge eklemiyor (yalnız `link` öğesi oluşturuyor). Sayım eşit.
- Bölme ve lejant sayıları: galeri `qp scene --look <l> --roles` (tema yok, imleç yok)
  çağırır; `Fixture::layout_scene` da `roles = true`, tema yok, imleç yok. Metin süzgeci
  (`x < right`, `baseline <= bottom`, `under` + `over`) ve sayı belirteci
  (`^[-+−]?\d[\d.,]*%?$`) birebir aktarılmış; birim testi (`numeric_tokens_match_the_gallery`) var.
- Fark: galeri yuvanın tarayıcıdaki boyutunu, ölçüm `th::DESK` boyutunu kullanır. CONTRACT.md
  ve modül yorumu bunu söylüyor; `tavek` sonucu RD2 galeri dökümüyle aynı (5/3/7).
  `--layout` hâlâ ölçümü geçersiz kılıyor (testte kanıtlı).

**Yeni fikstür galerinin seçimini sessizce değiştirebilir mi (`findDesignFixture`)?**
Bugün hayır. `readdirSync().sort()` sırasında `desk-marks.json` (`-` = 0x2D) `desk.json`'dan
(`.` = 0x2E) **önce** gelir; ancak `desk-marks.json` içinde `"id": "desk"` geçmez (id'ler:
`marks`, `arkin`, `c1`, `candles`, `glyph`, `0`), bu yüzden düzenli ifade eşleşmez ve
`desk.json` seçilmeye devam eder. `docs/` alt klasörü `.json` ile bitmediği için taranmaz.
Bkz. bulgu B1 (kırılganlık).

**Belirlenimci mi?** Evet: `scripts/design-check` iki koşu çıktısı `diff` ile aynı.

### 5. Taban çizgisi karşılaştırması (`rounds/RF1/experiments/baseline/text/design/check.txt`)

Değişen satırlar:

| Satır | Değişiklik | Beklenen mi |
|---|---|---|
| yeni 1 | `PASS desk-marks` | evet |
| 2→3 | `1 transcript(s)…` → `2 transcript(s), 2 passed…` | evet (conform özeti) |
| 10, 15 | `c2a-trial` P8 dark/light: `28 / 0` → `28 / 24 (1 glyph)` (kural ve eşik metni de değişti) | evet (P8) |
| 33–35 | `tavek` P8 `20/0` → `20/20 (1 glyph)`; P9 `n.a.` → `5 / 3 / 7` PASS | evet |
| 64–66 | `tavek-tel` P8 `24/0` → `24/24 (1 glyph)`; P9 `n.a.` (kural metnine "touch: no desktop layout" eklendi) | evet |
| 95–97 | `oltan` P8 `32/0` → `32/28 (1 glyph)` FAIL; P9 `n.a.` → `5 / 3 / 3` FAIL | evet (P8/P9) |
| 126–128 | `r4` P8 `32/0` → `32/24 (1 glyph)` FAIL; P9 `n.a.` → `5 / 3 / 3` FAIL | evet (P8/P9) |
| **78** | `oltan … 19 FAIL` → **`20 FAIL`** | **hayır — bulgu B2** |
| **109** | `r4 … 22 FAIL` → **`23 FAIL`** | **hayır — bulgu B2** |

Bunların dışında fark yok.

## Bulgular

- **B1 (düşük) — galeri seçimi sıraya bağlı ve kırılgan.** `findDesignFixture` sıralı
  listede `desk-marks.json`'ı `desk.json`'dan önce okur. Bugün güvenli (`"id": "desk"` yok,
  fikstür açıklaması da bunu not ediyor), ama `desk-marks.json`'a ileride `desk` adlı bir
  görünüm ya da öğe kimliği eklenirse galeri sessizce bu fikstüre geçer. Öneri (kapsam
  dışı): galeri `desk.json`'ı adıyla seçsin ya da design-check bu varsayımı sınayan bir
  test taşısın.
- **B2 (düşük) — eski görünüşlerin başlık satırları değişti.** `oltan` 19→20 FAIL, `r4`
  22→23 FAIL. Neden: P9 artık bütün masaüstü görünüşlerde ölçülüyor; eski görünüşler lejantta
  3 sayı basıyor (eşik ≥ 5) ve KALIYOR. Eski görünüşler "yalnız karşılaştırma, asla
  düşürmez" olduğundan çıkış kodu etkilenmez; değişiklik P9 ölçümünün doğrudan sonucudur
  ve tutarlıdır. Yine de yönergenin izin verdiği satırların dışında olduğu için kayda
  geçirildi; tutanakta (taban çizgisi yenilenirken) adıyla anılmalı.
- **B3 (düşük) — P9 boyutu galeriden farklı.** Ölçüm 960 × 600 (`th::DESK`), galeri
  yuvanın gerçek boyutunu kullanıyor. Bölme sayısı boyuttan bağımsız; lejant sayısı
  masaüstü genişliklerinde aynı olduğu iddia ediliyor ve `tavek` için RD2 dökümüyle eşleşiyor,
  ama dar bir masaüstü yuvasında lejant kısalırsa iki ölçüm ayrışabilir. Belgelenmiş;
  `--layout` kaçış yolu duruyor.
- **N1 (bilgi) — kopya belge.** `fixtures/design/docs/syn03-breakout.json` (1552 satır)
  `fixtures/views/docs/r3-breakout.json`'un kopyası; ikisi zamanla ayrışabilir. Ayrı
  tasarım fikstürü kuralı (desk.json ve altınlara dokunmamak) için bilinçli seçim.
- **N2 (bilgi) — yazım.** `docs/design/CONTRACT.md:112` "…düzeni yoktur: `n.a.`
  `--layout <dosya>` …" iki cümle arasında nokta eksik.

## Temizlik

- `git worktree remove --force .claude/worktrees/audit-o` ve `rm -rf target/wt/audit-o`
  yapıldı; scratchpad'deki koşu günlükleri silindi.
