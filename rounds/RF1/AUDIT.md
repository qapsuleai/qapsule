# RF1 — Bağımsız kapanış denetimi

**Denetçi:** bağımsız kapanış denetçisi alt ajanı (kodu yazmadı). **Tarih:** 2026-11-25.
**Denetlenen:** `main` @ `e2f69afc` (RF1/X: STATUS, ROADMAP; REPORT taslağı).
**Ortam:** `git worktree add .claude/worktrees/audit-final main --detach`; ayrı derleme klasörü
`CARGO_TARGET_DIR=target/wt/audit-final`; `QP_CONFIG_BASE=7fb01d11`. İş bitince ikisi de silindi.

## Hüküm: **KOŞULLU KABUL**

Davranış değişmedi; bunu kendim ölçtüm. Tutanak **270/270 bayt eşit** çıktı, `verify.sh` temiz
derleme klasöründe **21/21 PASS** verdi. Yapı hedefleri tuttu: 12 bin üstünde yalnız İ3 var
(`qp-authority` 12.744 ≤ 13.000), 800 satırı aşan tek dosya üretilmiş tablo, `--config-diff` 0
kanıtsız gevşeme gösteriyor. Ek noktasını kendi örneğimle denedim ve otorite crate'ine
dokunmadan çalıştı.

Koşullar kapanış belgeleri ve diskle ilgili. Kod tarafında koşul yok. Kapanmadan önce K1–K3
yapılmalı.

| # | Kapanmadan önce yapılacak |
|---|---|
| K1 | `REPORT.md`'deki "(kapanışta)" yerleri doldurulsun: §6a (K17 A/B; veri `experiments/perf-k17-ab.txt`'te var), §13 adım 3, CI, §17 Disk. `scripts/clean.sh` koşsun. `target/` şu an **137 GB** (bütçe 6 GB); `target/wt/final` hâlâ duruyor. CI'ın yeşil olduğu gösterilsin. |
| K2 | `experiments/rewritten.md`'ye **G2b** ve **G3** bölümleri eklensin; bugün ikisi de yok (B2). |
| K3 | REPORT §3'teki "toplam test dışı satır ≈ 1,5 bin arttı" cümlesi yanlış; ölçülen **+4.097** (B3). |

## Kabul ölçütleri (GOAL 1–15)

| # | Ölçüt | Hüküm | Kanıt (kendi ölçümüm) |
|---|---|---|---|
| 1 | Tutanak ilk iş, commit; RD2 kalemleri dahil | ✅ | `7fb01d11` 270 kalem ve `design/*`, `gallery/*` içinde. Tutanak ilk kaynak değişikliğinden (`717ad036`) eski. Bütünlük kontrolü aşağıda (VERIFY 1–2). |
| 2 | Tutanak bayt eşit; verify yeşil; CI yeşil; altın/döküm/şema/araç değişmedi | ✅ (CI: K1) | `compare.sh` 270/270, çıkış 0. `verify.sh` 21/21 (`total 710s`). `fixtures/{golden,compute,protocol,views,schema}` farkı boş. CI'ı göremedim. |
| 3 | K17, K52 gürültü içinde; WASM ±%2 | ✅ İ8 ile | Varsayılan WASM 2.354.884 B: tutanağa göre +%0,39. Kabuk paketi (adım 18'in derlemesi) 9.004.364 B (+%2,45), gzip 2.484.935 B (+%2,16); İ8 tavanı %2,5. K17 A/B (`perf-k17-ab.txt`): özetler aynı, medyanlar gürültü içinde. K52'yi `perf-final.txt`'ten okudum, kendim koşmadım. |
| 4 | P8, P9; eski Playwright klasörleri | ✅ | `design/check.txt` v1→v2 farkı yalnız P8/P9 satırları ve `desk-marks`. `$TMPDIR`'de 14 ve 16 Kasım tarihli Playwright klasörü yok; kalanlar 17 ve 23 Kasım tarihli WebKit klasörleri, kapsam dışı. |
| 5 | `scripts/refactor/` kalıcı; sıralı katman; config adımı; öz-sınama | ✅ | `arch-check` altı katman sırasını basıyor. `--self-test` PASS (config-diff B7 ve D5 dahil). verify adım 19 ve 20 PASS. |
| 6 | PLAN iki eleştiriyle; onay DECISION'da; kod onaydan sonra | ✅ | `critique-1/2.md` var. Onay `dbd8efb9` (21:52). Kaynak değişiklikleri sırasıyla V `717ad036` (21:57; zaten izinli), G2a `72018802`, S2 `0265a76b`. Onaydan önce yalnız test (`qp-grammar/tests/design_check`) ve betik değişti. |
| 7 | `qp-grammar` ≤ 12 bin; enjeksiyon; tasarım ayrı; R7 ≥ 3 bin | ✅ | 7.710 → pay 4.290. `cargo tree -p qp-grammar`: yalnız `qp-calendar`, `qp-glyph`, `qp-look`, `qp-schema`; `qp-designs` yok. |
| 8 | `qp-capsule` ≤ 12 bin; `qp-voice` ayrı | ✅ | 8.467 / 4.763 |
| 9 | 12 bin üstü crate yok ya da tek adlı istisna | ✅ | Yalnız `qp-authority` 12.744 (İ3, tavan 12.744, DECISION ≤ 13.000). |
| 10 | `expand_op`, `prepare` bölündü; 800 üstü dosya 0 | ✅ | 800'ü aşan tek dosya `qp-look/src/advances.rs` 878 (üretilmiş). Sonraki en büyük 762. `prepare` 229 satırlık dosyaya ve adımlara bölündü. `expand_op` 101 satır (Arkin'in kararı, taban çizgisinde). |
| 11 | İ2–İ6 kapandı/küçüldü; yeni istisna yok ya da Arkin'in | ✅ | config-diff: İ2 (5 kalem), İ4, İ5, İ6 (2 kalem) "removed". İ3 için "moved … (new crate; max 12744)". İ8 DECISION'da (Arkin, 2026-11-25). |
| 12 | Ek noktası belgesi ve sınaması | ✅ | `extension` sınaması verify'da geçiyor. Kendi örneğim de geçti (aşağıda). E birleşmesinde `crates/qp-authority/src` farkı boş. |
| 13 | Borç büyümedi; önce/sonra | ✅ | `fn.lines` 177→173, `file.soft` 36→32, `lint.allow` 252→247 (13→22 crate'e dağıldı, toplam düştü). `dir.files`/`doc.size`/`test.file` 3/2/6 aynı. En uzun fonksiyonlar REPORT §8 ile aynı. |
| 14 | README; ARCHITECTURE, ENGINEERING §2; yol listesi | ✅ | 29 crate'in 29'unda README var. ENGINEERING §2 sırası `arch-config` ile aynı. ARCHITECTURE'da iki yeni bölüm var. `paths.md` 224 satır. |
| 15 | AUDIT, REPORT, STATUS, ROADMAP; kapanış temizliği | ⚠️ K1 | AUDIT bu belge. STATUS ve ROADMAP güncel. REPORT'ta yer tutucular duruyor; temizlik yapılmamış (`target/` 137 GB). |

**Kesilenler (REPORT §12):** dört kalemin dördü de GOAL'de kesilebilir olarak sayılıyor
(sözdizimi ağacı, `qp-source` dosya okuması, geçici yeniden dışa aktarmalar, yeni lint'ler).
Kesilemeyen kalemlerin hepsi tamamlandı. Kabul edilebilir. Dördünün birden kesilmesi "sıkışırsan"
koşulunu biraz zorluyor; A72 ve önerilerde adıyla duruyor. Kesilmeyen §6 kalemine (eskimiş belge
yorumları) Q dalgası dokunulan crate'lerde baktı. `qp-schema` yorumları kasten bırakıldı
(REPORT §15).

## VERIFY 1–14

| # | Komut / bakış | Sonuç |
|---|---|---|
| 1 | `git log -- rounds/RF1/experiments/baseline` | ✅ İlk commit `7fb01d11` (16:54). Sonra `0a2d2a71`, `c0b9a45b`, `62d487d7` geliyor. **Bütünlük:** `7fb01d11→c0b9a45b` manifest farkı yalnız `design/check.txt` ve `fixtures/committed.sha256` (2 satır). Aradaki commit'ler yalnız `qp-grammar/tests/design_check`, `scripts/design-check` ve iki yeni tasarım fikstürünü değiştiriyor; `src/` değişmedi. `62d487d7` farkı yalnız 4 `gallery/vars/fonts-*.css` satırı; içerik yalnız `file:///Users/…` → `file://<root>/…`. |
| 2 | `scripts/refactor/compare.sh` | ✅ `compare: 270 items byte-identical to the baseline`, çıkış 0 (5 dk 33 sn) |
| 3 | `scripts/verify.sh` (ayrı derleme klasörü) | ✅ 21/21 PASS: fmt, clippy, test (480 sn), …, arch, config, design. `verify: OK`. |
| 4 | `git diff --stat 7fb01d11 HEAD -- fixtures/{golden,compute,protocol,views,schema}` | ✅ boş |
| 5 | `arch-check`, `--table`, `--self-test` | ✅ 0 hata. 2 adlı istisna: İ1 katman, İ3 `crate.lines`. İ2 yok; sıralar bildirilmiş. Öz-sınama PASS. |
| 6 | `--table` | ✅ `qp-grammar` 7.710, `qp-capsule` 8.467. R7 payı 4.290; `qp-designs` 8.926 olduğundan orada da 3.074 pay var. |
| 7 | 800 üstü kaynak | ✅ yalnız `qp-look/src/advances.rs` 878 (üretilmiş) |
| 8 | `--config-diff` (taban `7fb01d11`) | ✅ 160 değişiklik, **0 kanıtsız gevşeme**. Tek `LOOSER` satırı üretilmiş dosyanın yer değiştirmesi (`advances.rs`); kanıtı DECISION G2a notunda. İ3 taşındı, tavan yükselmedi. `expand_op` 101 kalemi ve lint dağılımı DECISION S4 / S3.0 notlarında. |
| 9 | Ek noktası; otorite `src` farkı | ✅ `extension` sınamaları geçiyor. `cd8d8e7c^1..cd8d8e7c` farkında `qp-authority/src` yok. Kendi örneğim de `src`'ye dokunmadı (`git status` yalnız sınama dosyası). |
| 10 | `cargo tree -p qp-grammar` | ✅ tasarım crate'i yok. `qp-authority` ağacında alt sistem crate'i yok (`qp-watch`, `qp-read`, `qp-viewer`, `qp-replica`, `qp-replayhost`). |
| 11 | Perf, WASM | ✅ İ8 ile. WASM'ı kendim ölçtüm: varsayılan 2.354.884 / 773.646 gzip (+%0,39); kabuk paketi 9.004.364 / 2.484.935 (+%2,45 / +%2,16 ≤ İ8 %2,5). K17 A/B ve K52 dosyadan okundu. |
| 12 | PLAN, DECISION, `git log` sırası | ✅ Onay `dbd8efb9`, gramer kodu (`72018802`) ve oturum kodu (`0265a76b`…) ondan sonra. D4 sırası tuttu: S3.1 `58001d6d` → E `cd8d8e7c` → S3.2 `bcdb2fa0` → S3.3 `468e0119`. |
| 13 | Rapor listeleri | ⚠️ K2. §7, §9, §10 var. G2b ve G3'te yeniden yazılan öğeler `rewritten.md`'de adıyla yok. |
| 14 | worktree, `du -sh target`, belgeler | ⚠️ K1. Bu denetimden sonra worktree listesinde yalnız ana ağaç kalıyor. `target/` 137 GB (> 6 GB); `target/wt/final` duruyor. |

## Yeniden yazılan öğelerden örneklem (12 öğe; eski gövde `git show` ile)

| Dalga | Öğe | Hüküm |
|---|---|---|
| G2b | `compile`: başlık şablonu | ✅ Eskisi `match face { Some(n) => faces::headline_template(Some(n))?, None => None }`. Yenisi `(designs.headline_template)(face)?`. `qp_designs::registry::headline_template` aynı `match` (yüz yoksa `Ok(None)`); bileşim varsayılanı `DESIGNS`. |
| G2b | `terms::mark` + `underline_layer` | ✅ Gövde aynen ayrıldı; `open::layer_term` / `voice_terms_in` tablodan çağrılıyor. |
| G3 | `prepare` → `layer_source`, `check_encoding`, `start_state`, `mark_rows`, `finish` | ✅ Hata sırası aynı: önce `n_data`, sonra sabitler, kaynak, opaklık, genişlik, `x`, `x2`, kapsam. Etiket hesabı eskisi gibi denetimlerden sonra; işaret kolu sırası aynı; `Prepared` alanları aynı. |
| G3 | `rule` adımı (`segment`, `horizontal`, `vertical`) | ✅ Normalleştirilmiş farkta yalnız çıkarma ve `st.` öneki var; ifadeler ve iletiler aynı. |
| S2 | `DocProvider::headline_parts` | ✅ Eski gövde (`find` + `reader_parts(&s.doc, c, s.proj, …)`) aynen `impl HeadlineSource for Session`'a taşındı. |
| S2 | `DocProvider::voice` | ✅ Yukarıdakiyle aynı biçimde. |
| S4 | `put_series` + `put_series_source` | ✅ Eski kol gövdesiyle satır satır aynı. Yalnız `&sp` → `sp` (`&str`), `s` `&mut` üstünden. |
| S3.0 | `committed` | ✅ Gövde aynı. Kitle döngüsü `note_audiences`'e aynen gitti; sona `AfterCommit` fazı eklendi. |
| S3.1 | `expand_watch` → `add_watch` / `remove_watch` | ✅ Denetimler, sıra ve iletiler aynı. `person_of` = `delegates.get(p).unwrap_or(p)` (eski `person_of_speaker`). `unreachable!` yerine `invalid` geldi; o kol erişilemez. |
| S3.1 | `advance_clock` | ✅ Eski gövde aynen duruyor. Ek: uyandırma silme, `Proposals` fazı, sonda `emit`. Bileşimin fazsız yolunda boş. |
| S3.2 | `bindings` | ✅ değer aynı. `FeedStore::position` (`&mut`, önek belleği) yerine `data_digest(bars)` kullanılıyor. `digest_u64` = `prefix_digest_u64(len)` = `data_digest_u64(bars)`, biçim `{:016x}`; ikisi aynı. Fark yalnız başarım (B6). |
| S3.3 | `OpenSide::{new, update}` `start_from` | ✅ `start_from(&d)` = `start(d.clone())`; eskisi `start((*open.document).clone())`. |

Davranışı değiştiren bir öğe bulamadım. Tutanak, karakterizasyon kitapları ve verify de bunu
destekliyor.

**"Değişmeden taşındı" örneklemi.** 10'dan fazla öğeyi rastgele seçtim (tohum 2511); eski
(`7fb01d11`) ve yeni gövdeleri boşluk ve görünürlük normalleştirmesiyle karşılaştırdım:

- **Birebir aynı:** `assemble` (claims), `phrasings` (`qp-voice`), `seq_to`, `evidence_ref`
  (`qp-plane`), `css_vars` (`qp-look`), `walk_branches`, `need_proposal`, `need_panel`
  (`qp-ops`), `marks_in_both_languages` (`qp-designs`), `about_instrument`.
- **Yalnız yol farkı:** `signs_keep_3_to_1`, `ledger_answer`, `pin_box`.
- **Bilinçli yeniden yazım:** `capsule_topic_lines_in` (`c.look` → `self.looks[ci]`).
  `rewritten.md` S3.2'de listeli, tutarlı.

`moved.py 0265a76b~1 0265a76b` (S2) sonucu: 1.294 değişmeden, 97 yalnız yol, 16 `CHANGED`. Hepsi
`rewritten.md` S2'deki öğeler ve `deliver` (yalnız yol).

## Ek noktası: kendi örneğim (`Quota`)

Tally'den farklı bir alt sistem yazdım. Atılacak sınama dosyası
`crates/qp-session/tests/audit_quota.rs` idi; commit edilmedi, silindi. Alt sistem yalnız açık
API'yle kuruldu: `qp_session::{Authority, Subsystems, STANDARD}` + `qp_subsystem`. Kullandığı
noktalar:

- **`Phase::Check`:** bağlantı başına en çok iki komut. Üçüncüsü `limit` hatasıyla reddedildi,
  günlüğe yazılmadı.
- **`Phase::Connect`:** `["a","b"]`.
- **`Phase::Subscribe`:** `["a/t"]`; `changes` konusu yerleşik yoldan onaylandı, sonra faz çağrıldı.
- **`Phase::AfterCommit`:** her insan girdisinden sonra motor girdisi (`ctx.out.entry`, yazar
  `qp`). Sonuç: `quota 1..3`, seq 2/4/6.
- **Kayıtlı sorgu:** `qp-read`'in `ANY` ile cevapladığı **var olan** bir tür, `log`, devralındı ve
  alt sistemin cevabı döndü. Devralınmayan `describe` hâlâ `qp-read`'e gidiyor.
- `Phase::Settled` listede kayıtlı ama tetiklenmedi.

Sonuçlar:

- **Yeniden oynatma:** `qp_ops::apply::replay(initial, log)` aynı doküman özetini verdi.
- **Determinizm:** aynı iletilerle kurulan ikinci otoritenin günlüğü bayt eşit çıktı.
- **Yansı:** aynı listeyle kurulan `Replica` otoriteyi izledi (`seq` ve doküman özeti eşit).
- **Otorite:** `crates/qp-authority/src` değişmedi; dosya yalnız `qp-session/tests` altındaydı.
  Sınama tek seferde yeşil oldu.

**Belgedeki sürtünmeler** (ARCHITECTURE "Yeni alt sistem nasıl eklenir"; Düşük, öneri):

- (a) `AfterCommit` alt sistemin kendi motor girdisi için de çağrılıyor. Kendi girdisini ayırt
  etmezse tavan `MAX_EMIT_DEPTH` olana kadar yineler. Belge derinlik sınırını anıyor ama "kendi
  girdinizi süzün" demiyor (Tally de süzüyor).
- (b) `check` içinde yuvaya yazılan durum geri alınmıyor; komut sonra açılımda ya da başka bir alt
  sistemin `check`'inde reddedilse bile yazılmış kalıyor. Belgede yok.
- (c) Reddetmek için `ProtocolError` gerekiyor. Kurucusu `qp_plane::error::E`, ama belge bunu
  söylemiyor (ben serde ile kurdum).
- (d) Konu ancak kapalı `Topic` enum'unda varsa kullanılabiliyor. Bu belgede yazıyor; `log`
  konusunu denedim, `schema` hatası verdi.

## Bulgular

| # | Önem | Bulgu | Kapanıştan önce? |
|---|---|---|---|
| B1 | Yüksek (kapanış adımı) | REPORT'ta yer tutucular duruyor: §6a, §13/3, CI, §17. Kapanış temizliği yapılmamış: `du -sh target` = **137 GB** (bütçe 6 GB), `target/wt/final` duruyor. CI sonucunu göremedim; REPORT da göstermiyor. | **Evet (K1)** |
| B2 | Orta | `experiments/rewritten.md`'de G2b ve G3 bölümleri yok. REPORT §7 "gövdesi değişen her öğe rewritten.md'de adıyla" diyor. G2b'nin 8 öğesi yalnız commit iletisinde ve REPORT tablosunda var; G3'ün "29 fonksiyon/tip" adları hiçbir yerde tek tek sayılmıyor. BRIEF: "yeniden yazdığın her öğeyi say ve raporda listele". | **Evet (K2)** |
| B3 | Düşük | REPORT §3: "Toplam test dışı satır ≈ 1,5 bin arttı". `arch-check --table` toplamı 126.597 → 130.694, yani **+4.097**. Değişen crate grubunda 66.193 → 70.296; diğer crate'lerde −6 (`qp-shellcore` −2, `qp-cli` −4). Test satırları 63.815 → 70.647. | **Evet (K3)**, metin düzeltmesi |
| B4 | Bilgi | Varsayılan WASM boyutu derleme klasörüne göre birkaç yüz bayt oynuyor: REPORT 2.355.036, `verify-final.log` 2.354.796, benim ölçümüm 2.354.884. Hepsi +%0,39. | Hayır |
| B5 | Düşük | DECISION'daki S3.2 notu `FOLLOWER` = `STANDARD` eksi okuma diyor. S3.3'ten beri görüntüleyici de dışarıda (kod, ARCHITECTURE ve `rewritten.md` doğru). DECISION'daki S4 notu `lint.allow qp-ops 21` diyor; taban çizgisi 20 (daha sıkı). | Hayır (not) |
| B6 | Düşük | `bindings` sorgusu artık her çağrıda bütün çubukların özetini O(n) hesaplıyor; eskiden önek belleği kullanılıyordu. Değer aynı; tik yolunda değil. | Hayır |
| B7 | Düşük | Ek noktası belgesinde (a)–(c) sürtünmeleri (yukarıda). | Hayır (öneri) |
| B8 | Düşük | İ8'in mekanik bir kapısı yok: `verify.sh`, `arch-check` ya da `DEBT.md` kabuk paketi tavanını (+%2,5) denetlemiyor; yalnız DECISION ve `decisions/rounds/RF1.md`'de yazılı. Tavana ≈ 4,8 KB kaldığı için R7'de sessizce aşılabilir. A70 ile birlikte bir boyut adımı önerilir. | Hayır (öneri) |
| B9 | Bilgi | `arch-config`'te İ3'ün `approved` alanı hâlâ "K85, 2026-11-22" diyor; 13.000 tavanının kaynağı RF1 DECISION. Gerekçe metni DECISION'ı anıyor, yeterli. | Hayır |

## Doğrulayamadıklarım

- **CI:** uzak çalışma sonucunu görmedim. `qgh` ve `preflight` kullanmadım; denetçinin işi değil.
- **K52 `metrics.mjs`** ve **K17 A/B:** kendim koşmadım; `perf-final.txt` ve `perf-k17-ab.txt`
  dosyalarından okudum, akla yatkınlarını kontrol ettim.
- **Playwright silmesi:** yalnız sonucu gördüm (14 ve 16 Kasım klasörleri yok); silme anını
  göremedim.
- **Önceki dalga denetimleri:** yeniden koşmadım; dalga başına denetim raporlarına (`audit-*.md`)
  güvendim. Örneklem 12 yeniden yazılan ve 10'dan fazla taşınan öğeyle sınırlı.
- **Kabuk paketi ölçümü:** verify adım 18'in kendi derlemesi (`shells/web/pkg/qp_wasm_bg.wasm`),
  `perf.sh` ile aynı bayraklarla mı kurulduğunu ayrıca denetlemedim. Sonuç REPORT'la ±160 B
  içinde.

## Temizlik

- `.claude/worktrees/audit-final` ve `target/wt/audit-final` silindi.
- Taban tablosu için açtığım geçici `7fb01d11` worktree'si (scratchpad) silindi.
- Scratchpad'deki denetim dosyaları silindi.
- Ürün kodu değişmedi, birleştirme yapılmadı.

## Yeniden denetim (kapanış)

**Denetlenen:** `main` @ `5f26cbd4` (`636788a6` + `5f26cbd4`, denetim cevapları). **Ortam:** ayrı
detached worktree `.claude/worktrees/audit-close`; derleme gerekmedi. Worktree iş bitince
silindi.

### Hüküm: **KABUL**

Üç koşul da karşılandı ve kanıtları tutuyor.

- **Kod değişmedi.** `git diff accd72ab 5f26cbd4 --stat -- crates scripts fixtures shells
  Cargo.toml Cargo.lock` boş. Değişen 7 dosya yalnız belge: `STATUS.md`,
  `docs/ARCHITECTURE.md`, `AUDIT.md`, `DECISION.md`, `REPORT.md`, `experiments/clean.log`,
  `experiments/rewritten.md`.
- **Bu yüzden önceki ölçümler geçerli:** temiz klasörde `verify.sh` 21/21 ve tutanak 270/270.
- **CI yeşil:** iki koşu da başarılı.

### Koşulların durumu

| Koşul | Hüküm | Kanıt |
|---|---|---|
| K1 | ✅ | **§6a:** tablodaki 20 medyan değer `experiments/perf-k17-ab.txt`'in orta sütunuyla bire bir aynı (A1, B1, A2, B2; sma 4,60/4,48/4,41/4,45 … bollinger 8,92/9,23/9,79/9,53). Dört koşuda özetler aynı. **§13 adım 3:** `verify-final.log` `total: 1003s`, `verify: OK`; ortak `target/`'taki ilk denemenin düşmesi de açıkça yazılmış. **CI satırı:** doğru. **§17 Disk:** dolu; silinenler, bilerek bırakılanlar ve kalan boyutlar listesi var. `ls target`: yok. `git worktree list`: yalnız ana ağaç (`5f26cbd4 [main]`). `experiments/clean.log`: önce 124G, `cargo clean` 384.311 dosya / 94,7 GiB sildi, sonra `target/ -`, depo toplamı 2,1G. **CI** (`scripts/preflight.sh` TAMAM, ardından `scripts/qgh run list --limit 3`): `45579438275` (`accd72ab`) completed/success, 27m56s. `45583858544` (`5f26cbd4`, kapanış) completed/success, 36m30s; bu koşuyu bitene kadar bekledim. |
| K2 | ✅ | `rewritten.md`'ye "G2b — tasarım kaydı" (8 satır: `CompileOptions`, `Frame`, `compile`, `prepare` çağrı noktaları, `terms::mark`, `underline_layer`, `Designs`/`NONE`, `DESIGNS`/`options`/uyarlayıcı) ve "G3 — `prepare` adlandırılmış adımlara" bölümleri eklendi. G3 listesini koddaki öğelerle (`grep fn/struct/type`) karşılaştırdım; altı dosyanın her birinde liste kodla aynı: `prepare.rs` (`PrepState`, `Encoding`, `prepare`, `check_encoding`, `start_state`, `mark_rows`, `main_or_err`, `next_palette`, `finish`), `source.rs` (3), `series.rs` (3), `rule.rs` (4), `panel.rs` (5), `point.rs` (5 + `type SizeChan`). Kodda olup listede olmayan öğe yok. `without_designs` ve `with_designs` `compile.rs`'te var. |
| K3 | ✅ | REPORT §3: "**+4.097** (126.597 → 130.694)". `arch-check --table` toplamı @`5f26cbd4` = 130.694. |
| Notlar | ✅ | **DECISION:** "Güncelleme (S3.3, kapanış denetimi notu)" satırı eklendi: `FOLLOWER` = `STANDARD` eksi okuma eksi görüntüleyici; `qp-ops` `lint.allow` 21 → 20. **ARCHITECTURE:** "Yazarken dikkat" bölümünde üç madde var: kendi `AfterCommit` girdisini süz; `check` içindeki durum geri alınmaz, durumu `AfterCommit`'te yaz; hata kurucusu `qp_plane::error::E` (`new`/`at`/`build`). `E::at` ve `E::build`'in var olduğunu `qp-plane/src/error.rs`'te gördüm. |

### Bilgi notları (koşul değil)

- **G3 sayımı.** `rewritten.md` "29 fonksiyon/tip ve 2 yapı" diyor; benim kodda saydığım
  27 fonksiyon + 1 tip (`SizeChan`) + 2 yapı. Adların hepsi tam ve doğru; fark yalnız sayıda
  (±1). İsterseniz düzeltin.
- **§17'deki "137 GB → 0".** Bu sayı denetim anındaki ölçüm. `clean.log` "önce 124G" diyor;
  aradaki fark denetçinin bu arada sildiği `target/wt/audit-final` (≈ 13 GB). Tutarlı.
- **İ8 kapısı.** Önceki B8 notu duruyor: kabuk WASM tavanını mekanik olarak denetleyen bir adım
  yok. REPORT §18 bunu A70'e bağlıyor. Kabul edilebilir.

### VERIFY 1–14 (son durum, `5f26cbd4`)

| # | Komut | Sonuç |
|---|---|---|
| 1 | `git log --oneline -- rounds/RF1/experiments/baseline \| tail -1` | ✅ `7fb01d11` (16:54). İlk `src` değişikliği `717ad036` (21:57) bundan sonra. v2 ve normalleştirme farkları yukarıdaki ilk denetimde doğrulandı. |
| 2 | `scripts/refactor/compare.sh` | ✅ 270/270 bayt eşit (ilk denetim, `e2f69afc`). Sonrasında kod değişmedi: `git diff accd72ab 5f26cbd4 --stat -- crates scripts fixtures shells Cargo.*` boş. |
| 3 | `scripts/verify.sh` | ✅ İlk denetimde ayrı klasörde 21/21 PASS (710 sn). Yazarın `verify-final.log`'u da 21/21 (1003 sn). Bu iki koşudan sonra kod değişmedi (2. satırdaki boş fark). CI `45583858544` (`5f26cbd4`) success. |
| 4 | `git diff --stat 7fb01d11 HEAD -- fixtures/{golden,compute,protocol,views,schema}` | ✅ boş |
| 5 | `scripts/arch-check`, `--table`, `--self-test` | ✅ `0 error(s); 2 named exception(s), 0 awaiting approval`. Taban içinde: `fn.lines` 173, `file.soft` 32, `lint.allow` 22. 12 bin üstü yalnız `qp-authority` 12.744 (İ3). İ2 yok; altı katman sırası bildirilmiş. `self-test: PASS`. |
| 6 | `arch-check --table` | ✅ `qp-grammar` 7.710 (R7 payı 4.290); `qp-capsule` 8.467 |
| 7 | `find crates -name '*.rs' -path '*/src/*' \| xargs wc -l \| sort -rn \| sed -n 2,3p` | ✅ `878 qp-look/src/advances.rs` (üretilmiş); sonraki 762 |
| 8 | `QP_CONFIG_BASE=7fb01d11 scripts/arch-check --config-diff` | ✅ `160 change(s), 0 uncited loosening(s)`. Tek `LOOSER` satırı `advances.rs`'in yer değiştirmesi (DECISION G2a). İ3 taşındı, tavan 12.744. Kapanan istisnalar silindi. |
| 9 | Ek noktası sınaması; otorite `src` farkı | ✅ `extension` sınaması verify'da yeşil. `git diff --stat cd8d8e7c^1 cd8d8e7c -- crates/qp-authority/src` boş. Denetçinin `Quota` örneği de otorite crate'ine dokunmadan geçti. |
| 10 | `cargo tree -p qp-grammar -e normal` | ✅ `qp-calendar qp-glyph qp-grammar qp-look qp-schema`; tasarım crate'i yok |
| 11 | Perf, WASM | ✅ İ8 ile. Varsayılan paket +%0,39. Kabuk paketi +%2,45 / +%2,16 (≤ %2,5, İ8). K17 A/B gürültü içinde (§6a ile `perf-k17-ab.txt` aynı). K52 için `perf-final.txt`. |
| 12 | `git log` sırası | ✅ Onay `dbd8efb9` → gramer `72018802` → oturum `0265a76b`…. D4 sırası tuttu: S3.1 → E (`cd8d8e7c`) → S3.2 → S3.3. |
| 13 | Rapor listeleri | ✅ §7 ve `rewritten.md` (artık G2b ve G3 dahil, bütün dalgalar), §9 istisnalar, §10 ve `paths.md` |
| 14 | `git worktree list`; `du -sh target`; belgeler | ✅ Yalnız ana ağaç; `target/` yok (0 ≤ 6 GB). AUDIT, REPORT (§17, §18), STATUS ve ROADMAP güncel. |

**Temizlik:** `.claude/worktrees/audit-close` silindi; derleme klasörü açılmadı. Ürün koduna
dokunulmadı.
