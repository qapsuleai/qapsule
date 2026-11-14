# RD1 — Rapor

- **Durum:** Tamamlandı. On üç kabul ölçütünün on üçü kanıtlarıyla karşılandı (aşağıda).
  Bağımsız denetim: `AUDIT.md` (sonucu "Denetim" bölümünde).
- **Tarih:** 2026-11-14
- **Alt ajanlar:** Bu round'da alt ajanlar **başlatılabildi** (R1 ve RP1'deki engel yoktu;
  ilk iş olarak bir deneme alt ajanı koşuldu). Bütün alt görevler, deneyler, ajan
  yazabilirlik ölçümü ve denetim ayrı alt ajanlarla yapıldı.
- **Dal:** `rd1/t3-protocol` (bütünleştirme dalı); alt görev dalları `rd1/t4-session`,
  `rd1/t9`, `rd1/t6a-exp`, `rd1/t6b-exp`, `rd1/t6c-exp`, `rd1/t6d`, `rd1/t6e`, hepsi ona
  `--no-ff` ile birleştirildi; `verify.sh` yeşilken `main`'e birleştirildi.

## Ne yapıldı

| # | Görev | Yürüten | Çıktı |
|---|-------|---------|-------|
| — | Bekleyen doküman değişiklikleri tek commit | orkestratör | `eccdc63` |
| — | Deneme alt ajanı | alt ajan (haiku) | başlatıldı, cevap verdi |
| T1 | Araştırma: 12 protokol, 2026–2030 varsayımları, kodlamalar | **3 alt ajan** (paralel, web) | `RESEARCH.md`, `research/*.md` |
| T2 | 20 doğal dil görevi (tasarımdan önce); şartname v0 | orkestratör | `tasks/TASKS.md` (`492f79a`), `docs/PROTOCOL.md` |
| T3 | Mesaj tipleri (tek kaynak), JSON Schema, örnekler, 17 uygunluk dökümü | orkestratör | `crates/qp-schema/src/protocol.rs`, `fixtures/protocol/`, `tools/protocol/` |
| T4–T5 | Referans oturum; süreç içi ve `qp serve` taşıması; döküm koşucusu | **alt ajan** (`rd1/t4-session`) | `crates/qp-session`, `qp serve`, `qp conform` |
| T6a | Eşzamanlılık: sıralı günlük vs CRDT (Automerge, Loro) | **alt ajan** | `experiments/T6a-concurrency.md` |
| T6b | Kodlama: JSON vs CBOR/MessagePack/postcard/sütunlu | **alt ajan** | `experiments/T6b-encoding.md` |
| T6c | Değişim biçimi: JSON Patch/Merge Patch vs anlamsal | **alt ajan** | `experiments/T6c-changes.md` |
| T6d | Devir: 10.000 barlı doküman sunucu → cihaz | **alt ajan** | `experiments/T6d-handover.md` |
| T6e | Sunucu (presenter) vs dil modelinin grameri yazması | **alt ajan** + 10 model örneği (alt ajan) | `experiments/T6e-presenter.md` |
| T7 | Ajan yazabilirlik: 3 tur × 5 taze alt ajan × 4 görev; tasarım değişikliği; yeniden ölçüm | **20 alt ajan** (biri geçersiz tur dahil) | `ergonomics/RESULTS.md`, `ergonomics/raw/` |
| T8 | Yetenek uzlaşması ve anlatım çizelgesi | T4 alt ajanı + şartname | döküm 11, 13; §10, §11 |
| T9 | MCP uyarlayıcısı; A2UI ve MCP Apps analizi | **alt ajan** (`rd1/t9`) + orkestratör | `crates/qp-mcp`, `qp mcp`, `EMBEDDING.md` |
| T10 | Şartnamenin son hali, `R2_INPUT.md`, sözlük, mimari | orkestratör | `docs/`, `rounds/RD1/` |
| T11 | Denetim | **bağımsız denetçi alt ajan** | `AUDIT.md` |

Deneyler (T6a–e) her biri ayrı dalda, ayrı alt ajanla, birbirinin sonucunu görmeden
yürütüldü; her biri hipotezini ölçümden önce ayrı bir commit'te kaydetti.

## Kabul ölçütleri

| # | Ölçüt | Sonuç | Kanıt |
|---|-------|-------|-------|
| 1 | ≥ 8 protokol incelenmiş, güncel belgelerden | ✅ | `RESEARCH.md` §1: 12 sistem (MCP 2026-07-28, MCP Apps, A2UI v1.0, AG-UI, A2A v1.0, LSP 3.17/3.18, Automerge/Yjs/Loro, ShareDB/ProseMirror, Figma/Replicache/Zero, Vega/Vega-Lite, TradingView/Perspective, RFC 6902/7386); her biri için neyi çözüyor / karşılığı / alındı / alınmadı ve kaynak bağlantısı. Okunan URL listeleri ve "DOĞRULANMADI" işaretleri `research/*.md`. |
| 2 | 2026–2030 varsayımları ve "gerçekleşmezse ne değişir" | ✅ | `RESEARCH.md` §2: 9 varsayım (V1–V9), her biri için kanıt ve protokolün değişecek kısmı. |
| 3 | `docs/PROTOCOL.md` v0: 8 tür, durum modeli, otorite, köken, yetki, yetenek, sürümleme, hata; her tür örnekli | ✅ | §4.1–4.8 (Yetenek, Komut, Değişim, Sorgu, Abonelik, Girdi, Öneri, Dikkat; her biri JSON örnekli), §2 durum, §8 otorite/eşzamanlılık/geri alma/devir, §9 kimlik/yetki/köken/denetim, §10 yetenek, §11 zaman/anlatım, §12 sürümleme, §13 hata, §14 güvenlik, §15 dökümler, §16 gömme. |
| 4 | Tipler tek kaynaktan; JSON Schema üretiliyor; her tür için geçerli/geçersiz örnek | ✅ | `qp-schema::protocol` (`ClientMsg`, `ServerMsg`, `Op`, `ChangeOp`, `Query`, `LogEntry`, `InboundLine`, `OutboundLine`, `SessionInit`, `Transcript`); `qp schema export` 22 şema. `fixtures/protocol/valid/` 27, `invalid/` 20 (neden tablosu `invalid/README.md`); `verify.sh` adım 12. |
| 5 | ≥ 12 dilden bağımsız döküm | ✅ | `fixtures/protocol/conformance/` **17** döküm (`Transcript` şemasıyla doğrulanıyor), şartnameden yazıldı, uygulamadan değil. |
| 6 | Referans oturum dökümlerin hepsini geçiyor; yeniden oynatma bayt-eşit; geri alma; öneri karneye ve kökene işleniyor | ✅ | `qp conform … --transport inproc` 17/17; koşucu her dökümden sonra günlüğü baştan oynatıp dokümanın baytlarını karşılaştırıyor. Geri alma: döküm 07 (kendi son girdisi, `undo_conflict`, yeniden yapma), sil+geri al bayt-eşit testi. Öneri: döküm 05 (kabul: yazar `scout`, `approved_by` `arkin`), 06 (ret, süre aşımı), 04 (ret → karne). |
| 7 | Aynı dökümler süreç içi ve `qp serve` üzerinden | ✅ | `verify.sh` adım 13: inproc 17/17, serve 17/17 (ayrıca mcp 17/17). |
| 8 | Çelişkili kararlar alternatifleriyle ölçülmüş (a–e) | ✅ | Aşağıda "Çelişkili kararlar"; beş deney raporu. |
| 9 | ≥ 20 görev, taze alt ajanlar, yalnız şartname+şema; ilk deneme, düzeltme, mesaj/belirteç; tasarım değişikliği ve yeniden ölçüm; önce/sonra tablosu | ✅ | `ergonomics/RESULTS.md`: v1 ilk 15/20, sonunda 17/20 → tasarım değişikliği (5 madde) → v2 ilk 14/20, sonunda 18/20; sızıntı denetimi v2r ilk 6/20, sonunda 19/20, düzeltme 13/14. Ham kayıtlar `ergonomics/raw/`. |
| 10 | Yetenek uzlaşması: saat, telefon, masaüstü üç derinlik; anlatım çizelgesi şartnamede ve dökümde | ✅ | Döküm 11: aynı bulgu saat → `sentence`, telefon → `capsule` (işaretler, `spark`), masaüstü → `chart` (görünüm, olgular), ses → `sentence`. Anlatım: §11 ve döküm 13. |
| 11 | MCP uyarlayıcısı: araçlar şemadan; ≥ 3 döküm MCP üzerinden; A2UI ve MCP Apps analizi | ✅ | `crates/qp-mcp`: 37 araç şemadan üretiliyor; `qp conform --transport mcp` **17/17**; `qp mcp` stdio duman testi. `EMBEDDING.md` §2 (MCP Apps), §3 (A2UI). |
| 12 | `R2_INPUT.md` | ✅ | Gramerin protokole karşı 10 gereksinimi, önerilen R2 kapsamı, ana akış kararları. |
| 13 | `REPORT.md`, `AUDIT.md`; `STATUS.md`, `ROADMAP.md`, `CONCEPTS.md`, `ARCHITECTURE.md`; önerilen karar satırları `DECISIONS.md`'de | ✅ | Bu dosya, `AUDIT.md`; `docs/CONCEPTS.md` (+21 kavram), `docs/ARCHITECTURE.md` (protokol katmanı, `qp-session`, `qp-mcp`); `DECISIONS.md` "Önerilen (RD1)" bölümü; `STATUS.md`, `ROADMAP.md`. |

## Çelişkili kararlar (K21) — bulgular ve öneriler

Kararlar bu round'da **değiştirilmedi**; öneriler ana akışa bırakıldı.

| Karar | Ölçüm | Bulgu | Öneri |
|-------|-------|-------|-------|
| **(a) K21-2** tek otorite + sıralı günlük | T6a | Üç model (otorite+günlük üç `base` varyantıyla; Automerge 0.12; Loro 1.17), 1 insan + 2 ajan, biri çevrimdışı; 100/1.000/10.000 işlem, 10/100/1.000 işlemlik çevrimdışı pencere. **30 koşuda hepsi yakınsadı.** Anlamsal bozulma: otorite **0** (her koşuda); CRDT'ler pencereyle artan, 10k işlemde 3–13 (10–100 pencere), **33–50** (1.000 pencere) — sessiz: değişmiş iddiaya bağlanan ret, silinmiş seriye işaret eden katman/niyet, aynı kimlikle iki oluşturmanın birleşmesi. Loro'nun yerleşik geri alması başka yazarın sonraki değerini 10k işlemde 7–8 kez ezdi; otoritede 0. CRDT üstünlükleri: liste konumu niyeti (C2), yeniden bağlanma 2 mesaj (Automerge 137 KB/94 KB) vs otorite 1.740 istek ve ~2,7 MB, geçmiş 158 KB (Automerge) / 700 KB (Loro) vs 5,6 MB (gzip 391 KB), kod 669/555 vs 1.398 satır. Otoritenin çakışmalarının %80–96'sı katman listesinin tek öğe sayılmasından (çoğu gereksiz; yazıların %9–11'i gereksiz yeniden deneme). İşlem başına süre: otorite 8–43 µs, Automerge 110–270 µs, Loro 50 µs → ~1 ms. Üçü de deterministik (15/15 bayt-eşit; CRDT'lerde aktör kimlikleri sabitlenerek). | K21-2 **korunsun**, dört inceltmeyle: katman kimliği ve katman başına çakışma; alan düzeyinde `base`; gönderenin kendi girdileri çakışma sayılmasın; yeniden bağlanmada tam `log` yerine değişim akışı/anlık görüntü. Hibrit (yalnız serbest metin için CRDT) ölçülmedi. |
| **(b) Kodlama** | T6b | JSON `float_roundtrip` ile 100.001 değerde bit-eşit (kapalıyken gerçek MACD değerlerinin %11'i bozuluyor). Tik (60/s): JSON 234 B, CBOR/MsgPack 183 B, postcard 78 B; WebSocket permessage-deflate ile hepsi ~33–35 B (%5 içinde); JSON kodla+çöz ~0,01 % çekirdek. 10k bar MACD değerleri: JSON 734 KB / gzip 308 KB, sütunlu 324 / 261 KB; çözme JSON 1,0 ms (WASM 1,4) vs sütunlu 23 µs. İkinci bir serde biçimi WASM'ı ~ikiye katlıyor (+190 KB gzip); postcard iç etiketli enum'ları çözemiyor ve fazla alanı sessizce kabul ediyor. **JSON içinde sütunlu bar** gzip'te bütün ikili biçimlerden küçük. | JSON kanonik kalsın; ikili biçim eklenmesin. Gerekirse JSON içinde sütunlu bar (veri biçimi kararı, D4) ve ileride pazarlıklı 8-bayt hizalı f64 blokları. Tik yolu için akış sıkıştırması. |
| **(c) Değişim biçimi** | T6c | Tel boyutu başa baş (anlamsal 5.171 B, otoritenin ürettiği JSON Patch 5.250 B). Farkı **hesaplamak** 10k barda değişim başına ~4,4 ms vs anlamsal ~1,8 µs (~2.400×). Eşzamanlılıkta yanlış sonuç: körlemesine JSON Patch 10/18, değer `test`'li 3/18, anlamsal + `base` 0/18. Doğrulama: anlamsal 13/13, en iyi korumalı JSON Patch 11/13 (geçiş kuralları kaçıyor). Denetim okunabilirliği: `LogEntry` 49/56, JSON Patch 9/56. Protokolün kendi `remove_layer {index}`'i indeks tabanlı: `base`'siz yanlış sonuçların hepsi ondan. | Anlamsal istek + ilkel değişim (v0 tasarımı) korunsun; JSON Patch yalnız dışa aktarım/hata ayıklama. Katmanlara kararlı kimlik (R2, G1). |
| **(d) Devir (K21-3)** | T6d | Motor iki uçta bayt/bit eşit (yerel = WASM özetleri aynı). 10k bar anlık görüntü 750 KB / gzip 211 KB (%99,7 bar); cihazda hazır 2,6 ms yerel / 3,2 ms WASM (%90 JSON ayrıştırma). Hesap çıktısını göndermek kötü takas (gzip 3,65× büyük, çözmesi hesaplamaktan 8× yavaş). Canlı: yalnızca `change` akıp cihazın hesaplaması (B+) 15 KB/s ve cihaz başına 0,24 µs sunucu işi; sunucudan değer çekmek 55 KB/s + %1,8 çekirdek. Geri devirde ham günlük 1,76 MB → sıkıştırılmış 5,7 KB gzip. Epoch kuralı bölünmeyi tespit ediyor, birleştirmiyor. **Referans oturum her tikte dokümanı kopyalayıp hesapları baştan yapıyor** (84 µs, 50k barda 397 µs); artımlı yol 0,18 µs. | K21-3'ü **incelt**: "motor iki uçta" kalsın; varsayılan "otorite sunucuda, cihaz hesaplayan yansı"; tam devir yalnız başka yazar yokken, kiralı. v1 işleri: artımlı canlı yol, öğe başına `base`, kira/fencing, `(epoch, seq)`, parçalı anlık görüntü. |
| **(e) K21-1** sunucu vs modelin grameri yazması | T6e | Sunucu: işlev 125 ns medyan, `show_finding` uçtan uca 6,6 µs; 1.000 koşuda tek çıktı, süreçler arası aynı özet. Model (5 Haiku 5.5 + 5 Sonnet 5.5, gerçek örnek): 10/10 şemaya uygun ve kabul edildi, var olan serileri kullandı; ama **10/10 birbirinden farklı**, 7/10 olguya dayanmayan 30 kuralı, Sonnet 5/5 modelin yazdığı sayı/metin (ilke 3 ve 6 ile çelişir), 9/10 tanımsız kodlama kanalı. Gecikme ~9 s (üst sınır; doğrudan API 1–3 s tahmini). Ölçek (günde 1,5 M sunum): sunucu ~10 CPU-sn/gün; Haiku ~0,4–1,2 bin $/gün, Sonnet ~10–25 bin $/gün. Yalnız sunucu çıktısı günlükten bayt bayt yeniden üretilebiliyor. | K21-1 **korunsun**. Ajanın grameri doğrudan yazması yeni görünüm türleri için değerli (ilke 8): bulgu başına bir kez `put_view` ile, belgede saklanıp deterministik yeniden kullanılarak. R2: kodlama kanallarını kapalı, tipli küme yapmak; sabit işaretleri olguya bağlayan alan. |

## Ajan yazabilirlik (özet)

Ayrıntı ve tablolar `ergonomics/RESULTS.md`.

| Ölçü | v1 | v2 (değişiklik sonrası) | v2r (kimlikler kopyalanamaz) |
|------|---:|---:|---:|
| İlk denemede başarı | 15/20 | 14/20 | 6/20 |
| Sonunda (≤ 3 deneme) | 17/20 | 18/20 | 19/20 |
| Hatadan sonra düzeltme | 2/5 | 4/6 | 13/14 |

- **Tasarım değişikliği (v1 başarısızlıklarından):** `now` (`welcome`, `result`), `log.within_ms`,
  `list` verisiz, `Target.output`, `ack.undoes`, durum/kesişim kılavuzu. Hedeflenen görevler
  (G02 "son yarım saat", G19 koşul türü) düzeldi; ilk deneme oranı değişmedi.
- **En önemli bulgu (v2r):** v1/v2'deki yüksek ilk deneme başarısının büyük kısmı, şartname
  örneklerinin test dokümanıyla aynı kimlikleri kullanmasından geliyordu. Kimlikler
  kopyalanamayınca ajanlar ilk denemeyi keşfe harcadı ama hata modeliyle 13/14 düzeltti.
  Sonraki tasarım adımı **anlamsal referans** (kimlik yerine anlamla başvuru) ve `welcome`'da
  doküman özeti (v3 önerisi, uygulanmadı).
- Ölçüm sonrası şartnameye "örnekler örnek dokümanın kimliklerini kullanır" notu eklendi;
  etkisi **ölçülmedi**.

## Ölçümler

| Ölçüm | Değer |
|-------|-------|
| `QP_VERIFY_BROWSER=1 scripts/verify.sh` | 13/13 PASS (adım 12 `protocol`, 13 `conform` eklendi) |
| Uygunluk dökümleri | 17; inproc 17/17, serve 17/17, mcp 17/17 |
| `cargo test --workspace` | yeşil (qp-session ve qp-mcp testleri dahil) |
| Hesap özeti | `7b1e99d40889f83c` (aarch64, x86_64 Rosetta, wasm32) — değişmedi |
| Altın görsel, tarayıcı | bayt-eşit, 0 px — değişmedi |
| WASM modülü | 475.126 B gzip (RP1: 475.519) — protokol `qp-wasm`'a girmedi |
| `qp-compute`, `qp-render`, `fixtures/golden` | `git diff main` boş |
| Protokol şemaları | 22 kök şema (`qp schema export`), iki dışa aktarım aynı |
| MCP araçları | 37, şemadan üretilir |
| Sunucu (presenter) | 125 ns işlev / 6,6 µs `show_finding` uçtan uca |
| Devir 10k bar | 211 KB gzip; cihazda hazır 2,6 / 3,2 ms (yerel / WASM) |
| Ajan yazabilirlik alt ajan belirteci | tur başına ~370–380 k (5 ajan) |

## Tasarım kararları (bu round'da verilen, ana akış onayına açık)

1. **Mesaj tiplerinin yeri `qp-schema::protocol`.** Ayrı `qp-protocol` crate'i yerine:
   `CLAUDE.md` "şemanın tek kaynağı `qp-schema`" diyor; doküman ve protokol tipleri birbirine
   başvuruyor (`Document.proposals`, `Narrative`).
2. **Kendi zarfımız + JSON-RPC eşlemesi.** Ajan yazımı için kısa; MCP/LSP kenarında 1:1
   eşlenir.
3. **İstek/değişim ayrımı.** `Op` anlam düzeyinde, `ChangeOp` ilkel ve tam öğeli; günlük
   ikisini de tutar; yeniden oynatma yalnız değişimlerle (motor sürümünden bağımsız).
4. **Öneri dokümanın içinde**, kimlikler öneri anında atanır.
5. **Varlık durumu günlük dışında.** Girdi kalıcı günlüğü şişirmez; ajan sorgu ve abonelikle
   okur.
6. **Saat bir girdidir** (`clock`, yalnız `host`).
7. **Otorite katı, istemci hoşgörülü** (bilinmeyen alan → `schema` + `did_you_mean`).
   Bu yüzden iç etiketli enum'lardaki birim varyantlar boş yapı varyantına çevrildi (serde
   onlarda fazla alanı sessizce kabul ediyordu; T6c buldu; telde JSON aynı).
8. **Ham girdi MCP'den taşınmaz.**

## Sapmalar

1. **Tasarım taslağı araştırma sürerken başladı.** Araştırma alt ajanları ilk iş olarak
   başlatıldı; mesaj tiplerinin ilk taslağı onlar çalışırken yazıldı; `PROTOCOL.md` iki
   araştırma raporu geldikten sonra yazıldı ve üçüncüsüyle düzeltildi. Git sırası
   (`RESEARCH.md` commit'i şartnameden sonra) bunu göstermez. Görevler tasarımdan önce
   commit'lendi (`492f79a`).
2. **Sözlüğe sonradan ekleme.** Protokol kavramları kodda kullanıldıktan sonra
   `docs/CONCEPTS.md`'ye eklendi (kural "önce sözlük").
3. **Ölçüm düzeneğinde iki hata bulundu ve düzeltildi:** (i) denetleyici Türkçe ondalık
   virgülü tanımıyordu (`117,66`); düzeltildi, ölçüt değişmedi (v1 15/20 bu düzeltmeden
   sonraki değer; önce 14/20); (ii) yeniden adlandırılmış doküman hesap adını da
   değiştirmişti; o tur geçersiz sayıldı ve yeniden koşuldu (`ergonomics/raw/v2r-void/`).
4. **G06 katı kaldı.** Ajanlar protokolden kesin değeri aldı ama kullanıcıya yuvarlanmış
   söyledi (78,12); ölçüt "bit düzeyinde eşit" olduğu için başarısız sayıldı.
5. **Ajan görev metinlerine küçük ek:** G11, G12, G17'de "kullanıcı (arkin)".
6. **T6e model örnekleri** doğrudan API ile değil alt ajanla alındı; süre ve belirteç
   üst sınırdır (ayrıntı `experiments/raw/t6e/llm-samples/README.md`).
7. **`verify.sh` hijyen adımı:** `scripts/qapsule-gh.env` (Arkin'in `.gitignore` istisnası)
   yalnızca bilinen dört gizli olmayan anahtarı taşıdığı sürece izinli.
8. **Kapsam:** `qp-session` her komutta dokümanı kopyalıyor (basit ve doğru; canlı yolda
   yavaş, T6d). Performans bu round'un ölçütü değildi; öneri olarak yazıldı.

## Açık sorular (ana akış için)

1. K21-1, -2, -3 ve kodlama için önerilen karar satırları (aşağıda) kabul ediliyor mu?
2. **Bar verisi biçimi:** `Bar` dizisi mi, JSON içinde sütunlu mu (T6b: gzip'te en küçük,
   çözme 2,6× hızlı)? Doküman şemasını ve `qp-render`'ı etkiler.
3. **Anlamsal referans** (kimlik yerine anlamla başvuru) protokole girsin mi? (v2r bulgusu.)
4. **Devir** için kira/fencing/`(epoch, seq)` tasarımı hangi round'da? Telefon sınıfı K17
   hedefi R4'te.
5. **MCP olay kanalı:** `notifications/message` (2026-07-28'de kullanımdan kalkmış) yerine
   `subscriptions/listen` + `log` sorgusuna geçiş.
6. **Öğe başına `base`** v1'e mi?

## Öneriler (kapsam dışı, yapılmadı)

- `qp-session`'a artımlı canlı yol (T6d: 460–700× hız; bayt-eşit sonuç gösterildi).
- Katman kimlikleri ve kimlikle katman işlemleri (T6c, R2 G1).
- Sunucuya olgu barı işareti; kodlama kanalı sözlüğünü kapalı/tipli yapmak (T6e).
- Ajanın yazdığı `put_view` varsayılan olarak öneri olsun (T6e).
- Dokunulan anahtarların daha ince izlenmesi (`main.x` / `main.layers`; T6c: gereksiz bir
  `undo_conflict`).
- Ölçüm düzeneği her zaman yeniden adlandırılmış dokümanla koşsun; şartname örnekleri ayrı
  bir örnek dokümana.
- MCP uyarlayıcısı çok iş parçacıklı barındırıcı için `Arc<Mutex<…>>`.

## Önerilen karar satırları

`DECISIONS.md`'de "Önerilen (RD1)" bölümüne yazıldı; ana akış onayı bekliyor.

## Denetim

(Denetçi alt ajan tamamlandığında dolduruldu.)
