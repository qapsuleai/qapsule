# RF1 — Oturumun ve gramerin bölünmesi: rapor

Durum: **tamamlandı** (2026-11-25). Davranış değişmedi; yeni özellik yok. Bölme planı Arkin'in
onayıyla (`DECISION.md`) uygulandı. Arkin'in karar bekleyenleri: önerilen kararlar Ö119–Ö126 ve
açık sorular A70–A73 (`docs/decisions/rounds/RF1.md`).

## 1. Özet

- **Tutanak bayt eşit.** Başlangıç tutanağı 270 kalem: RG1'in bütün kalemleri, RD2'nin
  `rd2-*` altınları, `design-check`, `design-contract --check` ve galeri çekirdek dökümü.
  Round sonunda `main`'de 270/270 bayt eşit (`experiments/compare-final.txt`). Her birleştirmede
  dalda ve denetçide de bayt eşit çıktı.
- **Tek bileşen yok.** `qp-session` (32,0 bin satır, tek bileşen) dokuz oturum crate'ine
  ayrıldı. Otorite çekirdeği `qp-authority` **12.744** satır (İ3, tavan ≤ 13.000, Arkin'in
  kararı). Çekirdek hiçbir alt sistemi tanımaz. Yeni iş alt sistem sözleşmesiyle (`qp-subsystem`)
  kendi crate'inde doğar ve bileşime (`qp-session`) bir kayıt satırıyla girer; ek noktası
  sınaması bunu otorite crate'ine tek satır eklemeden gösteriyor.
- **Gramer bölündü.** Dört crate: `qp-look`, `qp-glyph`, `qp-grammar`, `qp-designs`.
  `qp-grammar` 20.964'ten **7.710** satıra indi; R7'ye 4.290 satır pay kaldı. Derleyici tasarım
  tarafını tanımıyor; kaydı `Designs` fonksiyon tablosuyla dışarıdan alıyor.
- **`qp-voice` ayrıldı.** `qp-capsule` **8.467** satır.
- **İki dev fonksiyon bölündü.** `expand_op` 903 satırdan 23 kol fonksiyonuna; dağıtıcısı 101
  satır, Arkin'in kararıyla taban çizgisinde. `prepare` 883 satırdan adlandırılmış adımlara;
  en uzunu 76 satır.
- **İstisnalar 11'den 2'ye.**
  - Kapananlar: İ2 (katman içi sıra kuralı), İ4, İ5, İ6 (ikisi de).
  - Kalanlar: İ1 (`qp-source`, kesildi) ve İ3 (çekirdek, 12.744).
  - Yeni: İ8, kabuk WASM paketi (Arkin'in kararı, §6).
- **Araçlar kalıcı.**
  - `scripts/refactor/`: `snapshot`, `compare`, `perf`, `moved` (`use`, `impl` başlığı, öğe
    makrosu, `cfg` denetimleri), `graph --map`.
  - `arch-check`: katman içi sıra, istisna tavanları, `--config-diff` (yeni `verify.sh` adımı
    `config`).
  - Bağımsız denetçi araçları iki turda kandırmayı denedi; bulunan açıklar kapatıldı.

## 2. Kabul ölçütleri

| # | Ölçüt | Sonuç | Kanıt |
|---|---|---|---|
| 1 | Başlangıç tutanağı ilk iş, commit; RD2 kalemleri dahil | ✅ | `7fb01d11` (ilk kod değişikliğinden önce); v2 `c0b9a45b` (ön iş Ö'den sonra, kod değişmeden, yalnız `design/check.txt` ve fikstür özeti) |
| 2 | Tutanak bayt eşit; verify ve CI yeşil; altın, döküm, özet, şema ya da araç tanımı değişmedi | ✅ | `compare-final.txt` 270/270; `verify-final.log` (§13); CI §13 |
| 3 | K17, K52 gürültü içinde; WASM ±%2 | ✅ İ8 | §6. Varsayılan WASM +%0,39; kabuk paketi +%2,45 / +%2,17 → İ8 (Arkin) |
| 4 | P8 çizilen, P9 `design-check`'te; eski Playwright klasörleri silindi | ✅ | `audit-o.md`; §11 |
| 5 | `scripts/refactor/` kalıcı; sıralı katman kuralı; config-diff adımı; öz-sınama | ✅ | `audit-a.md` (iki tur) |
| 6 | `PLAN.md` iki eleştiriyle; onay `DECISION.md`'de; kod onaydan sonra | ✅ | `critique-1/2.md`; plan v2 `aaa207a5`; onay `dbd8efb9`; ilk gramer kodu `8253ca23` sonrası |
| 7 | `qp-grammar` ≤ 12 bin; kayıt enjeksiyonla; tasarım ayrı crate'te; R7'ye ≥ 3 bin | ✅ | 7.710; `cargo tree -p qp-grammar` tasarım crate'i göstermiyor; pay 4.290 |
| 8 | `qp-capsule` ≤ 12 bin; `qp-voice` ayrı | ✅ | 8.467 / 4.763 |
| 9 | 12 bin üstü crate yok ya da tek adlı istisna (tavan < 32 bin) | ✅ İ3 | `qp-authority` 12.744 (≤ 13.000, Arkin) |
| 10 | `expand_op` ve `prepare` bölündü; 800 üstü kaynak dosya 0 (üretilmiş hariç) | ✅ | En büyük: `qp-look/advances.rs` 878 (üretilmiş), sonra 762 |
| 11 | İ2–İ6 kapandı ya da küçüldü; yeni istisna yok ya da Arkin'in kararıyla | ✅ | İ2, İ4, İ5, İ6 kapandı; İ3 31.999 → 12.744; İ8 Arkin'in kararı |
| 12 | Ek noktası belgesi ve sınaması | ✅ | `docs/ARCHITECTURE.md` iki bölüm; `qp-session/tests/extension` (`audit-e.md`) |
| 13 | Borç taban çizgisi büyümedi; kalem sayısı ve en uzun fonksiyonlar önce/sonra | ✅ | §8 |
| 14 | README'ler; ARCHITECTURE ve ENGINEERING §2 güncel; dışa açık Rust yolu değişiklikleri listeli | ✅ | 29/29 README; `experiments/paths.md` |
| 15 | Bağımsız denetçi `AUDIT.md`; REPORT, STATUS, ROADMAP; kapanış temizliği | ✅ | `AUDIT.md`; §16 |

## 3. Önce ve sonra

**Crate'ler** (test dışı satır; `scripts/arch-check --table`; tutanak `7fb01d11` → kapanış):

| Crate | Katman | Önce | Sonra | Not |
|---|---|---|---|---|
| `qp-session` | oturum | 31.999 | **979** | bileşim (`Session` = otorite + kayıt) |
| `qp-authority` (yeni) | oturum | — | **12.744** | çekirdek; İ3 |
| `qp-ops` (yeni) | oturum | — | 9.372 | işlemler, seçiciler, kanonik biçim, değişim uygulama; pay 2.628 |
| `qp-read` (yeni) | oturum | — | 3.817 | okuma ve sunum alt sistemi |
| `qp-plane` (yeni) | oturum | — | 2.729 | veri düzlemi, değer önbelleği, hata, kapsül dizinleri |
| `qp-viewer` (yeni) | oturum | — | 1.634 | görüntüleyici alt sistemi |
| `qp-replica` (yeni) | oturum | — | 1.390 | istemci yansısı |
| `qp-replayhost` (yeni) | oturum | — | 1.285 | yeniden oynatma barındırıcısı |
| `qp-subsystem` (yeni) | oturum | — | 1.194 | alt sistem sözleşmesi |
| `qp-watch` (yeni) | oturum | — | 482 | izleme niyeti alt sistemi |
| `qp-grammar` | alan | 20.964 | **7.710** | derleyici |
| `qp-designs` (yeni) | alan | — | 8.926 | adlı tasarımlar |
| `qp-look` (yeni) | alan | — | 2.815 | görünüş, tema |
| `qp-glyph` (yeni) | alan | — | 1.989 | anlam işareti |
| `qp-capsule` | alan | 13.230 | **8.467** | |
| `qp-voice` (yeni) | alan | — | 4.763 | |
| öbürleri | | | | değişmedi (`qp-host` 10.433, `qp-shellcore` 9.520 …) |

Toplam test dışı satır ≈ 1,5 bin arttı. Nedenleri: sözleşme, kayıt, fazlar, yeni `README`/`//!`
belgeleri, crate sınırında açılan arayüzler.

**Oturum nasıl bölündü** (sıra alttan üste):

| Crate | Ne |
|---|---|
| `qp-plane` | aşağı: veri düzlemi |
| `qp-ops` | aşağı: saf işlemler |
| `qp-subsystem` | sözleşme |
| `qp-watch`, `qp-read` | alt sistemler; otoritenin altında, onu tanımazlar |
| `qp-authority` | çekirdek: alım, komut yolu, kabul, günlük, saat, bağlantılar, iki dünya, `as_reader`, kapsül yaşamı, iddia/itiraz/yorum, canlı yol; kayıt ve fazlar |
| `qp-replica`, `qp-viewer`, `qp-replayhost` | otoritenin istemcileri; görüntüleyici aynı zamanda alt sistem |
| `qp-session` | bileşim: iki kayıt listesi (`STANDARD`, `FOLLOWER`) |

**Otorite ne kadar kaldı:** 31.999 → 12.744 satır (−%60).

**Çekirdekte kalanlar:** kapsül yaşamı, iddia/itiraz/yorum ve canlı yol. Nedeni: aralarında
döngü, `as_reader` doküman takası, iki dünya takası, yeniden giriş zinciri ve tek yerden
korunabilen olay sırası var (`PLAN.md` §9 B1–B5). Arkin'in kararı: çekirdek donuktur ve kalan
ayrım borçtur (`docs/DEBT.md` İ3; aday RS1'den sonra).

## 4. Gramer

- **`qp-look`, `qp-glyph` aşağı (saf taşıma).** `advances.rs` üretilmiş; yeni yeri
  `DECISION.md`'de.
- **`Designs` tablosu.** Derleyicinin tasarıma uzandığı 5 nokta (başlık şablonu, iki açılmış
  kapsül metni, iki katman terimi) bir fonksiyon göstergesi tablosundan geçer. Tablo
  `CompileOptions`'ta taşınır; varsayılanı `qp_designs::options` (yaklaşık 70 çağrı yeri yalnız
  yol düzeltmesiyle).
- **`qp-designs` yukarı.** Yüzler, açılmış kapsül, hikâye çizelgeleri, akış düzenleri, geçişler,
  hareket ve `depth` sarmalayıcıları taşındı.
- **`prepare` bölündü:** `layer_source`, `capsule_texts`, `check_encoding`, `start_state`,
  `mark_rows` (işaret başına kol), `finish`; ara durum `PrepState`'te.
- **R7 payı.** `qp-grammar` 7.710 → pay 4.290. İşaretler derleyici alfabesine (bugün 926 satır)
  girecek. Çizim geometrisinin gramerin altında bir alan crate'inde doğması öneriliyor (Ö123,
  eleştiri 2).

## 5. Ek noktası ve sonraki round'lar

- **Belge:** `docs/ARCHITECTURE.md`'de iki bölüm:
  - "Yeni alt sistem nasıl eklenir": crate yeri, yuva, `impl Subsystem`, fazlar ve çağrıldıkları
    dosyalar, iki liste, kayıt satırı, `sub::<T>()`, bilinen sınırlar.
  - "Yeni doküman nesnesi nasıl eklenir": D2 ölçümü; otoritede **0 zorunlu + 1 yumuşak** dosya.
- **Sınama** (`qp-session/tests/extension`; `audit-e.md`):
  - (a) Yalnız testte yaşayan `Tally` bir işlem türünü kendi `OpHandler`'ıyla açar. Sayısını
    `AfterCommit` fazında motor girdisiyle günlüğe yazar, bir sorguyu cevaplar, kendi konusunda
    bildirim gönderir. `replay` aynı dokümanı verir; aynı listeyle kurulan yansı da aynısını.
  - (b) İzleme satırı listeden çıkınca `add_watch` "desteklenmiyor" der; geri gelince bugünkü
    çıktı gelir.
  - (c) Otorite hiçbir alt sistem crate'ine bağımlı değil (`cargo metadata`).
  - Otoritenin `src/`'si değişmedi. Denetçi kayıt dağıtımını bozdu; sınamalar kırmızıya döndü.
- **Sonraki round'lar nereden başlar:**

| Round | Nerede doğar |
|---|---|
| RL1 | `qp-lang` (alan) + `qp-program` (oturum alt sistemi). Dayanaklar: `OpHandler` (açılım + politika); `Check` fazında maliyet bütçesi (`param`/`entitled`); canlı değerlendirme için `ConditionKind`; program çıktısı için `ValueSource` (D3). |
| R7 | `qp-watch` büyür (izleme görevleri, yaşam döngüsü); çizim nesneleri yeni alt sistemde, geometrisi alan katmanında; "kur ve göster" `qp-read`'de; işaretler `qp-grammar` alfabesinde (4,3 bin pay). |
| RS1 | `qp-ledger`. Dayanaklar: `Settled` fazı ve iddia türü kaydı (D1); K88 için `Adjust` fazı. |
| RA1 | `qp-prep` ve üyelik alt sistemi; ortak analizin üretimi barındırma katmanında; zaman aralığı okuması `qp-read`'de. |

- **Bilinen sınır.** Kapalı enum'lar (`Query`, `Topic`, `Event`, `ChangeOp`) yeni tür için şema
  kolu ister; bu, şemayı değiştiren round'un işidir (A73).

## 6. Performans ve boyut

| Ölçü | Tutanak (`7fb01d11`) | Son (`bd5bf705`) | Not |
|---|---|---|---|
| K17 `sweep_bench` medyan ms (sma / ema / rsi / macd / bollinger) | 4,39 / 4,05 / 9,59 / 8,63 / 9,71 | 4,49 / 4,05 / 9,57 / 8,26 / 8,78 | özetler aynı; aynı saatte A/B: §6a |
| K52 `metrics.mjs` CPU ×4: tik p95 / sahne p95 / veri geçişi p95 (ms) | 0,5 / 31 / 4,8 | 0,5 / 28,4 / 4,8 | gürültü içinde; uyarı yok |
| WASM varsayılan (adım 8) ham / gzip -9 | 2.345.821 / 769.836 | 2.355.036 / 773.715 | +%0,39 / +%0,50 |
| Kabuk paketi (`replica`) ham / gzip -9 | 8.789.414 / 2.432.388 | 9.004.524 / 2.485.151 | **+%2,45 / +%2,17 → İ8** |

**İ8 (Arkin'in kararı, 2026-11-25).** Kabuk paketi `--no-opt` kuruluyor ve paylaşımlı jenerik
yok. Bir tipi kullanan her yeni crate jenerik örneklerin kendi kopyasını gömüyor (`BTreeMap`
arama ve ekleme, `drop_in_place`, `Document`/`ChangeOp` `clone`, serde ziyaretçileri; isim
bölümü +17,5 KB). Büyümenin kaynağı taşınan kod değil.

- **Seçenekler:** (1) istisna; (2) yansı ve görüntüleyici çekirdekte kalsın (çekirdek 15.449,
  tavanı aşar); (3) bir tur daha küçültme.
- **Seçim:** (1). Tavan ham ve gzip en çok +%2,5.
- **Kalan pay** ≈ 4,8 KB: R7'den önce kabuk paketinde `wasm-opt` önerisi (A70).

**§6a K17 aynı saat A/B.** (kapanışta eklenecek)

## 7. Yeniden yazılan öğeler

Taşınabilen taşındı (`moved.py` "değişmedi"); gövdesi değişen her öğe `experiments/rewritten.md`'de
adıyla ve nedeniyle listeli.

| Dalga | Yeniden yazılan / yeni (sayı) | Ne |
|---|---|---|
| G2b | 8 + yeni `designs.rs`, `registry.rs` | `CompileOptions` (`designs` alanı), `Frame`, `compile` (başlık şablonu), `prepare`'in 2 çağrı noktası, `terms::mark` + `underline_layer` |
| G3 | 31 | `prepare` adımları (29 fonksiyon/tip + `PrepState`, `Encoding`) |
| S2 | 4 + 2 yeni | `DocProvider` (`&dyn HeadlineSource`), `with_parts`, `headline_parts`, `voice`; `Computed::is_empty` |
| S4 | 1 + 23 yeni + `Arm` | `expand_op` dağıtıcısı ve kol fonksiyonları (gövdeler aynen); `put_series_source` |
| S3.0 | 37 | sözleşme, faz çağrı noktaları, kayda önce bakan dağıtıcılar, `note_audiences`/`initial_audiences` bölmesi |
| S3.1 | 71 | izleme alt sistemi, kurucular (liste parametresi), `run_capsules`, `allowed`, `proposable`, `agent_op`, `project_entry`, `eval_memo`, `advance_clock` |
| S3.2 | `impl Session` → `impl Rd<'_>` (gövdeler aynı) + listede 23 öğe + 7 dosyada `needless_borrow` | okuma bağlamı, `bindings`, `capsule_query_with`, `ledger_rows_of`, `ReadView` |
| S3.3 | 21 öğe (`moved.py` 154 satır) | gizli izleyici yöntemleri, görüntüleyici itme kancaları, `face_scene` bölmesi, yansı adımları |

Denetçiler her dalgada yeniden yazılanları eski gövdeyle karşılaştırdı ve karakterizasyon
sınamalarını bozarak duyarlılığı denedi.

## 8. Borç taban çizgisi (önce → sonra)

| Kural | Önce | Sonra |
|---|---|---|
| `fn.lines` | 177 | 173 |
| `file.soft` | 36 | 32 |
| `lint.allow` | 252 izin (13 crate) | 247 izin (22 crate) |
| `dir.files` / `doc.size` / `test.file` | 3 / 2 / 6 | 3 / 2 / 6 |

| En uzun fonksiyonlar (önce) | Satır | En uzun fonksiyonlar (sonra) | Satır |
|---|---|---|---|
| `qp-session` `expand_op` | 903 | `qp-designs` `stories/pavka/extend.rs::extend` | 592 |
| `qp-grammar` `prepare` | 883 | `qp-ops` `gops.rs::expand_grammar` | 571 |
| `qp-grammar` `stories/pavka/extend.rs::extend` | 592 | `qp-grammar` `scene.rs::compile` | 562 |
| `qp-session` `gops.rs::expand_grammar` | 571 | `qp-read` `query.rs::query` | 365 |
| `qp-grammar` `scene.rs::compile` | 564 | `qp-authority` `command.rs::command` | 300 |

## 9. Kapanan ve kalan istisnalar

| İstisna | Durum |
|---|---|
| İ1 `qp-source` alan katmanında | **kalıyor** (dosya okuma ayrımı kesildi, §12) |
| İ2 yan bağımlılıklar (5) | **kapandı**: katman içi bildirilmiş sıra (`order`) |
| İ3 `qp-session` 31.999 | **küçüldü**: `qp-authority` 12.744 (tavan mandallı; Arkin: ≤ 13.000) |
| İ4 `qp-grammar` 21.000 | **kapandı** (7.710) |
| İ5 `qp-capsule` 13.230 | **kapandı** (8.467) |
| İ6 `op.rs`, `prepare.rs` | **kapandı** (bölündü) |
| İ8 kabuk WASM paketi | **yeni**, Arkin'in kararı (§6) |

## 10. Dışa açık Rust yolu değişiklikleri

- **Liste:** tam hali `experiments/paths.md` (V, G2, S2, S3.0–S3.3).
- **Değişmeyenler:** protokol (`qp/0`), şema, komut satırı ve MCP yüzeyi değişmedi.
  `qp_session::Session`, `qp_session::{ReplayHost, Replica, tape, viewer, conform, …}`
  bileşimden aynı adlarla kullanılıyor.
- **Öne çıkan değişiklikler:**
  - `qp_capsule::voice` → `qp_voice`
  - `qp_grammar::{look, theme, error, glyph, faces, open, stories, feeds, transitions, motion}`
    → `qp_look`, `qp_glyph`, `qp_designs`
  - `CompileOptions::new` → `qp_designs::options`
  - `qp_session::{replay, apply_all, canon_document, DocProvider, …}` → `qp_ops`
  - `qp_session::{DataError, did_you_mean, headparts, …}` → `qp_plane`
  - `qp_session::Session` artık yeni tip `Session(Authority)` (`Deref`); otoritenin kendisi
    `qp_session::Authority`

## 11. Ön işler

- **RD2 denetim koşulu K1.**
  - P8'in çizilen değeri ayrı bir tasarım fikstüründen ölçülüyor (`fixtures/design/desk-marks.json`).
  - P9, `design-check`'te tarayıcısız ölçülüyor (`tavek` 5/3/7, RD2 galeri dökümüyle aynı).
  - Var olan fikstür ve altınlar değişmedi. Tutanak v2 bu yüzden alındı (yalnız
    `design/check.txt` ve fikstür özeti).
- **Playwright klasörleri.** `$TMPDIR` altında yalnız üç klasör vardı, tarihleri doğrulandı ve
  silindi: 14 Kasım (6,7 MB), 16 Kasım (144 KB), 16 Kasım (boş). Başka depo dışı klasöre
  dokunulmadı.

## 12. Kesilenler (GOAL kesme sırası)

1. **`arch-check`'in sözdizimi ağacına geçmesi (K-10).** Sözcük tarayıcı kaldı. Yerine kapatılan
   kandırma yolları: O1–O5, R1–R6 (`audit-a.md`).
2. **`qp-source` dosya okumasının uyarlayıcıya ayrılması (İ1).** İ1 kalıyor.
3. **Geçici yeniden dışa aktarmalar.** RG1'in `qp-session`'daki iç yeniden bildirimleri bölmeyle
   kendiliğinden kalktı. `qp-wasm`, `qp-cli`, `qp-mcp`, `qp-source`/`qp-compute`/`qp-facts`
   kökleri dokunulmadı.
4. **`unimplemented`/`unreachable` lint'leri (K-11).** Açılmadı. Bu round yeni `unreachable!`
   eklemedi: S4'te 22 kol `unreachable!`'sız kalıba çevrildi; `qp-ops`'ta 4 eski kullanım duruyor.

Gerekçe: dört kalem de GOAL'de kesilebilir; kesilemeyenler (tutanak, plan, gramer, ses, oturum,
iki dev fonksiyon, sıralı katman, ek noktası) tamamlandı. Round iki gün sürdü (K82). Öneri:
A72.

## 13. Doğrulama (VERIFY 1–14)

| # | Sonuç |
|---|---|
| 1 | Tutanak `7fb01d11` ilk kod değişikliğinden eski (`git log`; ilk kod `793a0975` Ö testleri, ilk crate kodu `8253ca23` sonrası) ✅ |
| 2 | `compare.sh` 270/270 bayt eşit (`experiments/compare-final.txt`) ✅ |
| 3 | `verify.sh` (temiz derleme klasörü): (kapanışta) |
| 4 | `git diff --stat 7fb01d11 HEAD -- fixtures/golden fixtures/compute fixtures/protocol fixtures/views fixtures/schema` boş; eklenen yalnız `fixtures/design/desk-marks.json` ve `fixtures/design/docs/syn03-breakout.json` (Ö) ✅ |
| 5 | `arch-check` 0 hata; 12 bin üstü yalnız İ3; İ2 yok; sıralar bildirilmiş; `--self-test` PASS ✅ |
| 6 | `qp-grammar` 7.710, `qp-capsule` 8.467, R7 payı 4.290 ✅ |
| 7 | 800 üstü kaynak dosya yalnız `qp-look/src/advances.rs` (üretilmiş) ✅ |
| 8 | `arch-config`/`arch-baseline` farkı: yalnız sıkılaşma, taşıma ve kararlı kalemler (`--config-diff` 0 kanıtsız; İ3 yer değiştirme ve S4 dağıtıcısı Arkin'in kararıyla `DECISION.md`'de) ✅ |
| 9 | Ek noktası sınaması geçiyor; `qp-authority/src` farkı o dalda boş ✅ |
| 10 | `cargo tree -p qp-grammar` tasarım crate'i göstermiyor ✅ |
| 11 | Performans ve WASM §6 ✅ İ8 |
| 12 | Onay `dbd8efb9`, gramer kodu `8253ca23`'ten (G1 testleri) ve `083d7878`'den önce; oturum kodu sonra ✅ |
| 13 | Rapor §7, §9, §10 ✅ |
| 14 | `git worktree list` yalnız ana ağaç; `target/` ve belgeler §16 |

**CI:** (kapanışta)

## 14. Yöntem, alt ajanlar, dallar

- **Alt ajanlar.** Her iş kendi worktree'si ve dalında bir alt ajanla yapıldı. Her dal ayrı bir
  denetçi alt ajandan geçmeden birleştirilmedi. Kesin hüküm iş başına ayrı derleme klasöründe
  verildi.
- **Denetim raporları:** `experiments/audit-*.md` (o, a, g1, v, g2, g3, q, s1, s2, s30, s31, e,
  s32, s33). KALDI ya da koşullu çıkanlar düzeltmeden sonra yeniden denetlendi: a, s1, e, s32 ve
  s2 (koşul).
- **Dallar** (kod dalda da duruyor): `rf1/o-design`, `rf1/a-tools`, `rf1/g1-charac`,
  `rf1/v-voice`, `rf1/s1-charac`, `rf1/g2-grammar`, `rf1/g3-prepare`, `rf1/s2-ops` (S2+S4),
  `rf1/q-grammar-docs`, `rf1/s30-registry`, `rf1/s31-watch`, `rf1/e-extension`, `rf1/s32-read`,
  `rf1/s33-replica`.
- **Karakterizasyon.** Gramer 11 test, 12 kitap; oturum 25 test, 27 kitap (`expand_op`'un 38
  işlem türü, bildirim sırası, iki dünya, ajan, izleme, yansı). Kitaplar hiçbir dalgada yeniden
  yazılmadı.
- **Hatalarım ve düzeltmeler:**
  - Tutanak betiği galeri font yollarını mutlak yazıyordu; worktree'de hep farklı çıkıyordu
    (`62d487d7`).
  - Plan v1'in oturum sınırları döngülüydü. Eleştiri ve ölçümle v2'de düzeltildi.
  - İlk E ajanı 100 dakika hiçbir şey yazmadı; durdurup yeniden başlattım.
  - S3.1'den sonra kabuk WASM payı darlaştı; S3.3'te sınır aşıldı. Durdum ve sordum (İ8).
  - Taban çizgisi yazımları otomatik izin denetiminde iki kez engellendi; ikisinde de Arkin'e
    sordum.

## 15. Fark edilenler (dokunulmadı)

- **Şema belge yorumları eskimiş yolları anıyor.** `qp-schema/src/protocol/input.rs:128-173`
  `qp_grammar::faces::FACES`, `qp_capsule::voice` vb. diyor. Şema dışa aktarımına girdiği için
  dokunulmadı.
- **Bugünkü davranış olarak kilitlenenler** (hata mı istenen mi, karar gerekiyor):
  - `set_audience` bilinmeyen kimliği kabul ediyor.
  - `add_watch` bilinmeyen enstrümanı kabul ediyor.
  - `line` işaretinde yalnız kökenle verilen `x` hata vermiyor.
- **Kapsama boşluğu:** gramer karakterizasyonunda birden çok hatayı aynı anda kıran vaka yok;
  bağımsız iki denetimin yer değiştirmesi yakalanmaz (audit-g3 B1).
- **Ulaşılamayan kol:** `project_changes`'in gizli `SetCapsuleState` kolu bugün ulaşılamaz bir
  savunma kolu; yanında not var.
- **Ortak `target/` kirleniyor.** Eşzamanlı worktree'ler ortak `target/`'ı kirletiyor: derleme
  ürünleri üstüne yazılıyor, silinmiş worktree'nin yolunu gömmüş test ikilileri kalıyor. Bu
  round'da da görüldü; kesin hüküm her seferinde ayrı klasörde verildi.
- **`docs/DEBT.md` bayattı.** RD2'den beri yeniden üretilmemişti (A2'de düzeldi).
- **CI sığ klonla koşuyor.** `fetch-depth` 1 olduğu için `config` adımı CI'da SKIP olur.

## 16. Öneriler (kapsam dışı; yapılmadı)

1. **Kabuk WASM paketinde `wasm-opt`** ya da `codegen-units`/`lto` ölçümü (A70). İ8'e 4,8 KB
   kaldı.
2. **`Follower(Session)` tipi** (A71). Komut yolu dışından yazan gizli izleyici yöntemleri yalnız
   orada olsun.
3. **Kesilen dört kalem** (A72). Önce lint'ler.
4. **CI'da `fetch-depth: 0`**, ki `config` adımı CI'da da koşsun.
5. **`arch-check`'e `cargo doc` uyarı adımı:** kırık rustdoc bağlantıları (Q bunları sıfırladı).
6. **`qp-replica` → `qp-authority` gizli yöntemleri** için `arch-check`'e bir çağıran kuralı
   (Follower tipi gelene kadar).

## 17. Disk (K56, K68)

(kapanışta)
