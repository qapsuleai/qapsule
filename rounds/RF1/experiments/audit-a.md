# RF1 / A denetimi — `rf1/a-tools` (A1 moved.py, A2 arch-check)

Denetçi: bağımsız alt ajan, 2026-11-23. Dal: `rf1/a-tools` @ `81793f9b` (merge-base `7fb01d11`).
Kendi worktree'si (`.claude/worktrees/audit-a`, detached) ve ayrı derleme klasörü
(`target/wt/audit-a`); hile denemeleri ayrı bir karalama worktree'sinde, hiçbir şey commit
edilmedi; ikisi de iş sonunda kaldırıldı.

## Hüküm: **GEÇTİ (koşullu)**

Dalın söz verdiği her şey çalışıyor ve sınanıyor: İ2'nin beş yan istisnası kalktı, yerine
katman başına bildirilmiş sıra (K-2) geldi; sıraya aykırı yan kenar hata veriyor; İ6'ya tavan
kondu; `--config-diff` (K-9) kanıtsız gevşetmeyi (yeni istisna, yükselen değer, yükselen
bütçe, sıra takası) yakalıyor; `moved.py` impl başlığı / makro / cfg değişikliklerini
CHANGED, saf taşımayı 0 CHANGED veriyor. `crates/**` altında değişiklik yok; yapılandırma
farkı yalnız sıkılaşma.

Koşul: aşağıdaki **O1–O4** (orta) birleştirmeden önce ya da RF1 içinde kapatılmalı ya da
Arkin'in kabulüyle DEBT/raporda açık kör nokta olarak yazılmalı. Hiçbiri bu dalın bugünkü
durumunu bozmaz (bugünkü fark temiz); hepsi aracın **ileride** kandırılabileceği yollar.

## Adım sonuçları

| # | Adım | Sonuç |
|---|------|-------|
| 2 | `git diff main...rf1/a-tools --stat` | 11 dosya, yalnız `scripts/` ve `docs/DEBT.md`; `crates/**` değişmedi ✅ |
| 2 | `arch-config.json` farkı | İ2 (5 kalem) silindi; her katmana `order`; `hard_rules` + `layer.order`; İ6 iki kaleme `max` (910, 891 = bugünkü ölçüm). Gevşetme yok ✅ |
| 2 | `arch-baseline.json` | Değişmedi ✅ |
| 2 | `docs/DEBT.md` | `--debt-md` ile yeniden üretilince bayt eşit ✅. Not: `scene.rs::compile 654 → 564` ve `file.soft 37 → 36` satırları bu dalın değil, `main`'deki DEBT.md'nin RD2 C2a'dan beri bayat kalmasının düzeltmesi |
| 3 | `scripts/arch-check` | 0 hata; 6 adlı istisna; sıralar basılıyor ✅ |
| 3 | `--self-test` | PASS (config-diff vakaları dahil) ✅ |
| 3 | `--table` | `main` ile bayt eşit ✅ |
| 3 | `--config-diff` | taban `7fb01d11` (merge-base); 14 değişiklik, 0 kanıtsız gevşetme, çıkış 0 ✅ |
| 3 | `moved.py --self-test` | 13/13 ok, PASS ✅ |
| 3 | `moved.py main HEAD` | 0 öğe, 0 CHANGED (dalda `.rs` değişikliği yok) ✅ |
| 3 | `bash -n scripts/verify.sh` | ✅ |
| 3 | `scripts/verify.sh` (tam) | 21/21 adım PASS (fmt, clippy, test, schema, …, arch, **config**, design); çıkış 0 ✅ |

## Hile denemeleri (karalama kopyada; commit yok)

Taban: dalın merge-base'i (`7fb01d11`, sırası yok) ve — sıra/tavan kurallarını gerçekten
sınamak için — `--config-diff HEAD` (sıralar ve tavanlar var; birleşmeden sonraki durum).

| # | Hile | arch-check | config-diff | Sonuç |
|---|------|-----------|-------------|-------|
| C1 | `qp-live → qp-mcp` bildirilmemiş yan kenar (Cargo.toml) | **hata** (`layer.lateral … against its declared order`) | — | yakalandı ✅ |
| C2 | `uyarlayıcılar` sırasında `qp-matriks`/`qp-mcp` takası | geçer | taban HEAD: **FAIL** (`swaps`); taban merge-base: geçer (eski tabanda sıra yok — beklenen) | yakalandı ✅ |
| C3 | Katmanı yeniden adlandır (`çekirdek` → `çekirdek2`), sırasını ters çevir, `qp-calendar → qp-render` kenarı ekle | **geçer** | **geçer** (yeni ad "declared" sayılır) | **kaçtı — O1** |
| C4a | Kararsız yeni istisna (`layer.lateral qp-live -> qp-mcp`) | geçer | **FAIL** | yakalandı ✅ |
| C4b | Aynı istisna, `status` İ1'den kopya (`onaylandı (K85, …)`) | geçer | **geçer** (`cites K85`) | **kaçtı — O2** |
| C4c | Aynı istisna, olmayan `K999` | geçer | **geçer** | **kaçtı — O2** |
| C5a | Taban `scene.rs::compile 564 → 600` | hata (bayat taban) | **FAIL** | yakalandı ✅ |
| C5b | Aynı, `{"value":600,"note":"K85"}` | hata (bayat) | geçer | arch-check yakalıyor; config-diff O2 |
| C6 | **Gerçek kodla:** `livehost.rs::new` (84) kısaltıldı, ilgisiz `qp-calendar/src/brand_new.rs` içine 83 satırlık yeni `fn new`; `--write-baseline --allow-new` | **0 hata** | **0 kanıtsız** (`moved … livehost.rs::new -> brand_new.rs::new`) | **kaçtı — O3** |
| C6b | `file.soft`: `qp-live/src/config.rs` (660) çıkar, ilgisiz `qp-calendar/src/config.rs` (650) girer (taban düzeyinde) | — | geçer (`moved`) | **kaçtı — O3** |
| C7a | `compile` → `compile2` yeniden adlandır + 564 → 570 büyüt | hata | **FAIL** | yakalandı ✅ |
| C7b | `compile`'ı başka dosyaya taşı + 570'e büyüt | hata | **FAIL** | yakalandı ✅ |
| C7c | Aynı boyda taşı (564) | — | geçer (`moved`) | tasarım gereği ✅ |
| C8 | `lint.allow`: `qp-session` 71 → 69, temiz ve **eski** `qp-calendar`'a 2 (yeni crate değil) | — | geçer (`spread`) | **kaçtı — D1** |
| C9 | Sona boş katman `boş` ekle, `shell_layer: "boş"` → `qp-cli`/`qp-wasm` kabuk bütçesi 4000'den 12000'e | **geçer** | **geçer** (`shell_layer` hiç karşılaştırılmıyor) | **kaçtı — O4** |
| C10 | `kabuklar`'ın `order`'ını sil | **hata** (`layer.order … missing`) | geçer | yakalandı ✅ |
| C11 | İ6 `op.rs` `max 910 → 950`, `max_note: "K85"` (K85 zaten status'ta) | geçer | taban HEAD: **FAIL** (yeni atıf değil) | yakalandı ✅ |
| C12 | Aynı, `max_note: "K999"` | geçer | **geçer** (`cites K999`) | **kaçtı — O2** |
| C13 | `budgets.fn_lines 80 → 120` + `rule_changes` `reason: "K1"` | hata (bayat taban) | geçer | arch-check yakalıyor; config-diff O2 |
| C14 | Yeni `layer_overrides` `qp-live → alan`, status kopya (K85) | hata (`layer.order`) | geçer | arch-check yakalıyor; config-diff O2 |

Yazarın bilerek koyduğu iki gevşetme için sonuç: **"aynı adla taşınan ve büyümeyen kalem"**
yeniden adlandırmayla ya da taşıyıp büyütmeyle aşılamıyor (C7a/b), ama **ilgisiz yeni bir
fonksiyon eski bir kalemin adını ve payını devralabiliyor** (C6, uçtan uca yeşil).
**"`lint.allow` yeni crate'lere yayılabilir"** kuralı yeni crate ile sınırlı değil: borç temiz,
var olan bir crate'e taşınabiliyor (C8).

## `moved.py` sentetik sınamalar (geçici git deposu)

| # | Değişiklik | Sonuç |
|---|-----------|-------|
| M0 | Saf taşıma: `a.rs` → `b.rs` + `c.rs`, impl bölündü, `pub` → `pub(crate)`, `#[cfg]` `use` ve `thread_local!` taşındı | 5 unchanged, **0 CHANGED**, 0 FLAG ✅ |
| M1 | Taşırken impl başlığı `impl<T: Clone>` → `impl<T: Clone + Send>` | **2 CHANGED** (impl ±) ✅ |
| M2 | Taşırken fn'nin `#[cfg(not(target_arch = "wasm32"))]`'i düştü | **CHANGED** ✅ |
| M3 | Taşırken `thread_local!(… = 1)` → `= 2` | **CHANGED** ✅ |
| M4 | `a` yöntemi `#[cfg(feature="x")] impl Foo`'dan kapısız `impl Foo`'ya taşındı (dosyada başka bir `#[cfg(feature="x")] impl Foo` kalıyor) | **0 CHANGED** — **kaçtı, O5** |
| M9 | Aynısı sınırla: `s` yöntemi `impl<T: Send> G<T>`'den `impl<T> G<T>`'ye (iki başlık da var kalıyor) | **0 CHANGED** — **kaçtı, O5** |
| M5 | Gövdede `i32::MAX` → `i64::MAX`, `f32::EPSILON` → `f64::EPSILON` | **PATH** (yol değişikliği sayıldı) — **kaçtı, D2** |
| M6 | Dosya başındaki `#![cfg(feature = "x")]` düştü | 0 CHANGED — **kaçtı, D3** |
| M7 | `#[cfg(test)] mod tests;` → `mod tests;` | 0 CHANGED — **kaçtı, D3** |
| M8 | `use crate::exact::round;` → `use crate::fast::round;` (aynı ad, başka kaynak) | yalnız `USE +1 -1 (path-only)`, çıkış 0 — D4 |

## Bulgular

| No | Ağırlık | Bulgu | Önerilen düzeltme |
|----|---------|-------|-------------------|
| O1 | orta | `--config-diff` sıra takasını katman **adıyla** eşliyor (`oo.get(layer["name"])`); katmanı yeniden adlandırınca eski sıra yok sayılıyor ve yeni sıra "declared" olarak geçiyor (C3: ters sıra + yeni ters kenar, iki araç da yeşil). Katman adı değişikliği de hiçbir yerde gevşetme sayılmıyor. | Sırayı katman **indeksine** (crate yerleşimiyle aynı anahtar) göre karşılaştırın ya da her crate çifti için eski göreli sırayı (adı ne olursa olsun) koruyun; katman adı değişikliğini `rule_changes["layers.rename"]` isteyen gevşetme sayın. Öz-sınamaya C3'ü ekleyin. |
| O2 | orta | "Karara atıf" yalnız `\bK\d+\b` düzenli ifadesi: olmayan karar (`K999`, `K1`) ve başka bir şey için verilmiş eski karar (`K85`'i başka bir istisnanın status'undan kopyalamak) geçiyor (C4b, C4c, C12, C13). Yeni istisna/override için `_new_cite` değil `cites` kullanılıyor, yani taban yapılandırmasında zaten geçen bir numara da yetiyor. ENGINEERING §12: "Yeni istisna … karar satırı ister"; VERIFY 8: "yeni istisna yok ya da `DECISION.md`'de". | Atıfı doğrulayın: `K<n>` `DECISIONS.md`'de satır olarak var olmalı **ve** tabanda (`<base>:DECISIONS.md`) olmamalı (ya da tabandaki yapılandırmada hiç geçmemiş olmalı); `DECISION.md` atfında dosyanın tabana göre değişmiş olması ve kalemin adının (crate/yol) geçmesi aranmalı. Öz-sınamaya "kopya K85" ve "K999" vakaları. |
| O3 | orta | Bilerek konan "aynı adla taşınan kalem" gevşetmesi yalnız **son ada** bakıyor (`fn.lines` için fonksiyon adı, `file.soft` vb. için dosya adı) ve crate/yol ya da içerik sınamıyor. Taban çizgisinde 4 `new`, 4 `view`, 2'şer `scene`/`draw`/`handle`… var; bir borç kalemi kapanırken (bölündü/kısaldı) ilgisiz yeni uzun bir fonksiyon onun payını devralıyor: C6'da gerçek kodla arch-check 0 hata, config-diff 0 kanıtsız. Mandalın inmesi gereken yerde borç yer değiştiriyor ("borç büyümez" sayısal olarak doğru, ama yeni borç kanıtsız girdi). | Taşıma eşini içerikle doğrulayın: eski kalemin öğe metni (rustitems parmak izi) tabanda, yeni kalemin metni HEAD'de eşit ya da `moved.py` ile PATH olmalı; olmuyorsa yeni kalem gevşetmedir. En azından aynı crate içinde kalma şartı ve `moved` satırlarının denetçiye ayrı listelenmesi. |
| O4 | orta | `shell_layer` (ve katman ekleme) `--config-diff`'te hiç karşılaştırılmıyor: sona boş bir katman ekleyip `shell_layer`'ı ona çevirmek `qp-cli`/`qp-wasm` bütçesini 4000'den 12000'e çıkarıyor, iki araç da yeşil (C9). | `shell_layer` değişikliğini (ve yeni katman eklemeyi) `rule_changes` isteyen gevşetme sayın; ya da kabuk kümesini (etkin crate listesi) karşılaştırın: eski kabuk crate'i kabuk olmaktan çıkıyorsa gevşetme. Genel çözüm: bilinmeyen/yargılanmayan her üst düzey anahtar değişikliğini FAIL yapın (beyaz liste). |
| O5 | orta | `moved.py` impl başlıklarını **küme** olarak karşılaştırıyor ve yöntemleri başlıklarına bağlamıyor: aynı başlıktan başka bir blok kaldığı sürece, bir yöntem `#[cfg]`'li ya da sınırlı impl'den kapısız/sınırsız impl'e geçebiliyor (M4, M9: 0 CHANGED). `qp-session`'da `impl Session` 18 dosyaya dağılmış olduğundan RF1'in asıl taşımalarında gerçek risk. RG1 B8'in tam kapanması değil. | Her `fn`'i içinde durduğu impl başlığıyla birlikte parmak izi alın (`(başlık, öğe metni)`), başlık eşleştirmesini PATH kuralıyla yapın; ya da başlıkları küme değil çoklu küme + "başlık başına yöntem kümesi" olarak karşılaştırın. Öz-sınamaya M4/M9. |
| D1 | düşük | `lint.allow` yayılma gevşetmesi "yeni crate" ile sınırlı değil: tabanda kalemi olmayan **var olan** temiz bir crate'e (`qp-calendar`) izin taşınabiliyor (C8). Toplam büyümüyor, ama temiz crate kirleniyor. | `spread` yalnız tabanda **crate olarak hiç bulunmayan** (`crates/<ad>/Cargo.toml` tabanda yok) kalemlere uygulansın. |
| D2 | düşük | `unpath` küçük harfli her `x::` önekini siliyor; ilkel türler de küçük harf: `i32::MAX` → `i64::MAX`, `f32::EPSILON` → `f64::EPSILON`, `u8::from` → `u16::from` PATH sayılıyor (M5). Hesap kodunda `f64` kuralı varken sessiz anlam değişikliği. | İlkel tür adlarını (`i8…i128`, `u8…u128`, `isize`, `usize`, `f32`, `f64`, `bool`, `char`, `str`) yol öneki saymayın; ya da `tails` karşılaştırmasına önekin son bölümünü de katın. |
| D3 | düşük | İç öznitelikler (`#![cfg(…)]`, `#![allow(…)]`) ve `#[cfg(test)] mod x;` bildiriminden `cfg(test)`'in düşmesi izlenmiyor (M6, M7). İlki bütün dosyanın derlenme koşulunu değiştirir; ikincisi test kodunu ürün koduna sokar (ve arch-check test muafiyetiyle birleşince RG1 B3 benzeri kaçak). | Dosya başı `#![…]` özniteliklerini dosya düzeyinde ayrı parmak izi olarak toplayın (taşınan dosya için eşleşmeli); `mod` bildiriminde bare `cfg(test)`'in eklenmesi/düşmesini de CHANGED sayın (yalnız eşleşmeyenleri). |
| D4 | düşük | Düz `use` yeniden yönlendirmesi (`exact::round` → `fast::round`) yalnız sayım olarak raporlanıyor, başarısız olmuyor (M8). Belge "A `use` cannot change behaviour unseen" diyor; bu iddia yanlış: aynı imzalı başka bir öğeye bağlanmak derlenir. RG1 B8'in önerisi ("use hedeflerini son bölüm + tam yol çoklu kümesi") yarım uygulandı. | Aynı bağlı ad için hedefin **crate'i ya da son bölüm öncesi modül adı** değiştiyse (yalnız `crate::a::` → `crate::a::b::` gibi derinleşme değil) FLAG yapın; belge cümlesini düzeltin. |
| D5 | bilgi | `--config-diff` dalda merge-base ile karşılaştırdığı için bu dalın kendisi sıra/tavan kurallarını yalnız "declared/tighter" olarak görüyor; sıra takası ve tavan yükseltme denetimi ancak birleşmeden sonraki dallarda etkili (C2, C11 taban HEAD ile doğrulandı). `QP_CONFIG_BASE` ortam değişkeni tabanı keyfî bir commit'e (ör. HEAD) çevirerek denetimi boşa düşürebilir; `main` üstündeki "round tutanağı" dalı bu denetimde sınanmadı. | `verify.sh` `QP_CONFIG_BASE` kullanıldığında tabanı belirgin basıyor; ek olarak tabanın HEAD'in atası ve HEAD'den farklı olduğunu doğrulayın. |

## Disk

Silinen: `.claude/worktrees/audit-a` (worktree), karalama worktree'si ve geçici git deposu
(scratchpad), `target/wt/audit-a` (12 GB). Bırakılan: yalnız bu dosya.

## Yeniden denetim

Dal `rf1/a-tools` @ `cb03bdc1` (`main` `c0b9a45b` üstüne rebase; düzeltmeler `b97c304b`
config-diff O1–O4, D1, D5 ve `96e00bd9` moved.py O5, D2–D4). Kendi worktree'm
(`audit-a2`, detached), derleme `target/wt/audit-a2`; hileler ayrı karalama worktree'sinde,
commit yok. Sıra/tavan kurallarını birleşme sonrası haliyle sınamak için taban `HEAD~1`
(`96e00bd9`; yapılandırma dosyaları HEAD ile aynı) — D5 düzeltmesi tabanı HEAD yapmayı artık
reddediyor (`--config-diff HEAD` ve `QP_CONFIG_BASE=HEAD` çıkış 1).

### Hüküm: **GEÇTİ (koşullu)**

O1, O3, O4, O5, D1, D2, D3, D5 kapandı; D4 büyük ölçüde kapandı. **O2 yarım kapandı:** atıf
artık "var olan ve tabanda kullanılmamış" olmalı, ama bu "bu kalem için verilmiş yeni karar"
demek değil — 90 küsur eski karar satırından tabanda geçmeyen herhangi biri (K1 gibi) ya da
eski bir round'un `DECISION.md`'si her gevşetmeyi geçiriyor (aşağıda R1). Bu, ENGINEERING §12
"yeni istisna karar satırı ister" ve VERIFY 8 "yeni istisna `DECISION.md`'de" için hâlâ açık
kapı; birleştirmeden önce kapatılması ya da Arkin'in açıkça kabul etmesi gerekir. Kalanlar
düşük.

### Adımlar

| Adım | Sonuç |
|------|-------|
| `git diff c0b9a45b...rf1/a-tools --stat -- crates` | boş ✅ |
| `arch-check` / `--self-test` / `--table` | 0 hata / PASS / `main` (`c0b9a45b`) ile bayt eşit ✅ |
| `--config-diff` (merge-base) | 14 değişiklik, 0 kanıtsız ✅ |
| `moved.py --self-test` / `moved.py main HEAD` | PASS / 0 CHANGED ✅ |
| `scripts/verify.sh` (tam) | 21/21 adım PASS (`arch`, `config`, `design` dahil), çıkış 0 ✅ |

### Önceki vakalar

| Vaka | Önce | Şimdi |
|------|------|-------|
| C3 katman adı + ters sıra + ters kenar | kaçtı | **FAIL** (`layers.names` + üç `order.x<y`) ✅ |
| C3b aynı ad, ters sıra | — | **FAIL** ✅ |
| C4a / C4b (K85 kopya) / C4c (K999) | C4b, C4c kaçtı | üçü de **FAIL** ✅ |
| C6 ilgisiz crate'e `new` devri | kaçtı | **FAIL** ✅ |
| C6b ilgisiz crate'e `config.rs` devri | kaçtı | **FAIL** ✅ |
| C8 `lint.allow` eski temiz crate'e | kaçtı | **FAIL** (kalem + sayı) ✅ |
| C9 boş katman + `shell_layer` | kaçtı | **FAIL** (`shell_layer`, `layers.names`) ✅ |
| C12 tavan 950, `K999` | kaçtı | **FAIL** ✅ |
| C13 bütçe yükseltme, `rule_changes` `K1` | kaçtı | **hâlâ geçiyor** (K1 gerçek satır, tabanda geçmiyor) — R1 |
| M4 / M9 yöntem cfg'li / sınırlı impl'den çıkıyor | kaçtı | **CHANGED** ✅ |
| D2 `i32::MAX→i64::MAX`, `f32::EPSILON→f64::EPSILON` | PATH | **CHANGED** ✅ (gövdede `std::f32::consts::PI→f64` de CHANGED) |
| D3 `#![cfg]` düştü / `mod tests;`'ten `cfg(test)` düştü | kaçtı | **CHANGED** ✅ |
| D4 `exact::round → fast::round` | sayım | **FLAG** (`--strict-use` ile çıkış 1) ✅ |
| D5 taban = HEAD | boşa düşer | **reddedilir** (çıkış 1) ✅ |
| M0 saf taşıma; M0b crate arası yol değişikliği | 0 CHANGED | 0 CHANGED / PATH ✅ |
| M1–M3 impl başlığı, fn cfg, makro | CHANGED | CHANGED ✅ |

### Yeni denemeler

| # | Hile | Sonuç |
|---|------|-------|
| N1 | Yeni istisna, `status` "onaylandı (K1)" (gerçek ama ilgisiz eski karar) | **geçti** (`cites K1`) — R1 |
| N2 | Yeni istisna, `rounds/RD2/DECISION.md` (eski round'un dosyası) | **geçti** — R1 |
| N6 | İ6 tavanı 910 → 950, `max_note: "K1"` | **geçti** — R1 |
| N10 | `generated_files` += `crates/qp-session/src/lib.rs`, gerekçe `K2` | **geçti** (config-diff; arch-check ayrıca başlıkta "üretilmiştir" ister) — R1 |
| N7 | Bütçe yükseltme, `rule_changes` `K999` | FAIL ✅ |
| N8 | `docs/decisions/kararlar.md`'ye aynı değişiklikte uydurma `K9999` satırı ekleyip ona atıf | **geçti** — R2 |
| N9 | Bilinmeyen üst düzey anahtar (`test_globs`) | FAIL ✅ |
| N5 | Yalnız sona boş katman | FAIL ✅ |
| N3 | Crate içi: `qp-host/livehost.rs::new` (84) çıkar, ilgisiz `qp-host/brand_new.rs::new` (84) girer | geçti (`moved`) — tasarım gereği, R3 |
| N4 | Yapılandırmada yeni crate `qp-dump` bildir; ilgisiz `new` borcu ve 5 `lint.allow` oraya | config-diff geçti (`moved`, `spread`) — R3 |
| N12 | `use std::f32::consts::PI` → `use std::f64::consts::PI` | **0 FLAG, 0 CHANGED** (son modül `consts` aynı sayılıyor) — R4 |
| N15 | `use crate::round` → `use crate::fast::round` | 0 FLAG (boş ebeveyn "aynı kaynak") — R5 |
| N16 | `use crate::m::round` → `use crate::m::fast::round` | 0 FLAG (derinleşme aynı sayılıyor; tasarım) — R5 |
| N17 | `use qp_a::util::round` → `use qp_b::util::round` | 0 FLAG (crate değişti, modül adı aynı) — R5 |
| N13 | Fonksiyon `#[cfg(feature="x")] mod gated { … }` satır içi modülünden kapısız `mod plain { … }`'a (cfg'li mod kalıyor) | **0 CHANGED** — R6 (O5'in modül düzeyi eşi) |
| N14 | Yardımcı fonksiyon satır içi `#[cfg(test)] mod tests { … }`'ten ürün koduna | 0 CHANGED — R6 |
| D3c | `#[cfg(test)] mod checks;` bir dosyadan silinip başka dosyada `cfg`'siz `mod checks;` | 0 CHANGED — R6 |
| N18 | `use super::*;` eklendi | FLAG ✅ (`--strict-use` 1) |
| N19 | `#![allow(clippy::unwrap_used)]` eklendi | FLAG ✅ (etiketi "use (inner attribute)"; kozmetik) |

### Kalan açıklar

| No | Ağırlık | Açık | Öneri |
|----|---------|------|-------|
| R1 | orta | Atıf "gerçek ve tabanda kullanılmamış" olunca geçiyor; **yeni** karar olması gerekmiyor. Tabanda geçmeyen ~90 eski `K<n>` (K1 "kuzey yıldızı" gibi) ve var olan her `rounds/*/DECISION.md` (bugün `rounds/RD2/DECISION.md`) istisnaya, tavana, bütçeye ve üretilmiş dosyaya izin veriyor (N1, N2, N6, N10, C13). Ayrıca tek bir atıf aynı değişiklikte sınırsız kaleme yetiyor. | `K<n>` satırı tabanda (`git show <base>:docs/decisions/…`) **olmamalı** (bu değişiklikte ya da tabandan sonra eklenmiş); `DECISION.md` tabana göre değişmiş olmalı ve tercihen kalemin adını (crate/yol/anahtar) içermeli. Öz-sınamaya "eski K1" ve "eski round DECISION.md". |
| R2 | düşük | Karar satırı çalışma ağacından okunuyor: aynı değişiklikte `docs/decisions/`'a uydurma satır eklemek yetiyor (N8). Fark görünür olduğu için denetçi yakalayabilir. | config-diff `docs/decisions/` farkını da bassın ("bu değişiklikte eklenen karar satırları: …"); karar satırlarını yalnız ana akışın yazdığı kuralı denetim listesine. |
| R3 | düşük | Taşıma eşi hâlâ yalnız ad + crate ile kuruluyor; crate içinde ilgisiz aynı adlı fonksiyon borcu devralıyor (N3); yapılandırmada "yeni crate" bildirmek borç ve `lint.allow` için serbest alan açıyor (N4). Yazarın bilinçli sınırı. | `moved` eşini `moved.py`/rustitems parmak iziyle doğrulayın ya da `moved` satırlarını denetçi listesine "elle bakılacak" diye ayrı basın. |
| R4 | düşük–orta | İçe aktarmada ilkel tür modülü yok sayılıyor: `use std::f32::consts::PI` → `std::f64::consts::PI` ne FLAG ne CHANGED (N12). D2 düzeltmesi yalnız gövde içi yollara uygulandı. `f64` kuralı olan hesap kodu için anlamlı. | `_same_source`'ta ilkel tür bölümü (`f32`/`f64`/`i32`…) farklıysa yönlendirme sayın; ya da tam ebeveyn yolunu karşılaştırıp yalnız crate/klasör yeniden adlandırmasını muaf tutun. |
| R5 | düşük | Yönlendirme sezgisi geniş: boş ebeveyn (`crate::round` → `crate::fast::round`), derinleşme ve farklı crate'te aynı adlı modül (`qp_a::util` → `qp_b::util`) FLAG değil (N15–N17). FLAG zaten varsayılan olarak başarısız değil. | Boş ebeveyni "aynı" saymayın; crate kökü (ilk bölüm) değişip modül adı aynıysa FLAG verin (crate bölme round'unda gürültü olursa `--list`'te ayrı başlık). |
| R6 | düşük | O5 düzeltmesi impl bloklarına özgü; satır içi `mod` blokları için aynı bağ yok: `#[cfg(feature)] mod x { }` ya da `#[cfg(test)] mod tests { }` içinden çıkan fonksiyon 0 CHANGED (N13, N14); `cfg(test)`'li `mod` bildirimi dosya değiştirince `_test_mods` görmüyor (D3c). | Öğeleri içinde bulundukları satır içi `mod` bloğunun öznitelikleriyle de önekleyin (impl önekiyle aynı yol); `_test_mods`'u dosya çifti yerine değişikliğin bütününde ad üstünden eşleyin. |

### Disk (yeniden denetim)

Silinen: `.claude/worktrees/audit-a2`, karalama worktree'si ve geçici git deposu, günlük, `target/wt/audit-a2` (12 GB). Bırakılan: yalnız bu dosya.
