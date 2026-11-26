# RL1 — Rapor (hesap dili)

Durum: tamamlandı (2026-11-26); bağımsız kapanış denetimi `AUDIT.md`.

## 0. Başlangıç

- Round başı: `08bcb4d6` ("ana akış: RL1 paketi …"). T0: ana akıştaki belge değişiklikleri bu
  commit'le kaydedilmişti; çalışma ağacı temizdi. Kökte `Prompt.txt` yok, izlenmiyor.
- Alt ajanlar başlatılabiliyor. İlk oturum harcama sınırında kesildi (2026-11-25); ikinci
  oturum kaldığı yerden sürdü. Arkin'in sözleşme kararı: `rounds/RL1/DECISION.md` (D1).
- Ön işler (Ö1–Ö3) her biri kendi worktree'sinde alt ajanla; şartname (S) ayrı alt ajanla.

## 1. Ön işler (K93)

| # | Sonuç | Dal / denetim |
|---|---|---|
| Ö1 | Kabuk paketinde `wasm-opt` (`-O1` + işlev ayıklama/sıralama). Ham 9.004.332 → 6.598.168 B (−%26,7), gzip 2.485.078 → 1.939.823 B (−%21,9). Açılış 140 → 126 ms; K52 gürültü içinde; özetler üç mimaride eşit (`7b1e99d40889f83c`). Yeni İ8 tavanı 6.763.122 / 1.988.319 (D4). Ayrıntı `experiments/o1-wasm/RESULT.md`. | `rl1/o1-wasm`, KABUL; verify 21/21 |
| Ö2 | İzleyici yöntemleri (8 adet) `qp-replica` `Follower` tipine kapatıldı; `arch-check` `calls.restricted` kuralı (öz-sınamalı). RF1 tutanağı 270/270 bayt eşit. `qp-authority/src`, `qp-session` değişmedi. Derleyici düzeyinde kapatma çekirdekte görünürlük ister (Öneriler). | `rl1/o2-follower`, KABUL |
| Ö3 | `unimplemented`, `unreachable` açık; 16 yer öğe başına izinle; `lint.allow` qp-compute 27→30, qp-ops 20→23, qp-plane 1→2 (D2). `qp-authority/src`: 2 öznitelik değişikliği, yeni satır yok (D2, Arkin). CI `fetch-depth: 0`; sığ klonda `config` açıkça SKIP. `config-diff`: atıflı artış yayılma kuralını bozmaz (düzeltme + öz-sınama). | `rl1/o3-lints`, KABUL |

Not: Ö2 birleşince main'de `config` adımı kırıldı (yeni anahtar `restricted_calls`; Ö2 denetçisi
`--config-diff`'i koşmamıştı). D3 + `rule_changes` girdisiyle aynı gün onarıldı.

## 2. Şartname

`docs/PROGRAMS.md` (dil, `qp-ir/0`), `docs/PROGRAMS-SESSION.md` (oturuma bağlanma, protokol,
dökümler, dokunulan yerler, eksik kancalar, R7/RS1 notları), `docs/CONCEPTS.md` (16 kavram),
`docs/PARAMETERS.md` (RL1 anahtarları), `docs/PROTOCOL.md` §26 işaretçisi (dosya net sıfır satır;
Ek A `docs/LEGACY.md`'ye taşındı). İki bağımsız eleştiri: `experiments/spec/CRITIQUE-A.md`
(tutarlılık, 14 bulgu), `CRITIQUE-B.md` (R7/RS1 yükü, 15 bulgu). Yürüten oturumun kararları:
K13 korunur (olay serisi tam uzunlukta, canlı bar `null`, kapanmış uzunluk ayrı); sinyal ve durum
tanımı dokümana girmez (yalnız denetle/dene/yeniden oynatma); `since_t`, `program_id`,
`instance_id`, `scope`, `known_t` eklendi. Şartname commit'i `ae5ad278`, ilk `qp-lang` kodundan önce.

## 3. Tipler, çekirdekler, denetim, metin dil

| İş | Sonuç | Denetim |
|---|---|---|
| I1 `qp-schema` | `qp_schema::program` (43 düğüm, 4 çıktı türü, parametre tipleri, kanonik/kimlik biçimi); `SeriesSource::Study.program`, `ProtocolError.errors` isteğe bağlı; 3 işlem, 3 sorgu. Şema dışa aktarımı yalnız ekleme (309 ekleme, 0 değişen/silinen). `describe` karakterizasyon kayıtları yalnız eklemeyle yeniden kaydedildi (D5). verify 21/21. | KABUL |
| I2 `qp-compute` | `stdev`, `percentrank` (`highest lowest atr` zaten vardı). Toplu = akışkan = düzeltme bit eşit; yeni özet `digest-rl1.txt` üç mimaride eşit. `add_study` listesi değişmedi. | KABUL (+ CI'da wasm32 karşılaştırması atlanmasın düzeltmesi) |
| LA `qp-lang` denetim | Bütün hatalar birden, konum + öneri; kod başına sınama; statik maliyet; `program_id`/`instance_id` (elle yazılmış SHA-256). Güvenlik incelemesi: `and` zincirinde üstel özyineleme, sınırların geç uygulanması, sınırsız öneri → düzeltildi (`fed10715`). | KABUL |
| L `qp-lang-text` | Elle yazılmış ayrıştırıcı, kanonik yazıcı, kaynak haritası, ret biçimi, iki dilli iletiler; gidiş-dönüş özellik sınaması. İlk denetim RET (yığın taşması: 1000 `(` süreci düşürüyordu), ikinci RET (yazıcı/ayrıştırıcı derinlik farkı); üçüncüde KABUL. | KABUL (3. tur) |

## 4. Çalıştırıcı, veri düzlemi, yüzeyler, uygunluk

| İş | Sonuç | Denetim |
|---|---|---|
| LB `qp-lang` çalıştırıcı | Akışkan, toplu = akışkan = canlı düzeltme bayt eşit; canlı barda kopyasız geçici değer; olay/sinyal/kayıt yalnız kapanmış barda; köken; seansa hizalı üst periyot (XIST günlük/4s, yarım gün, tatil; 7/24 UTC); sinyal sonuç kuralı; iş sayan bütçe; önek kararlılığı özellik testi (240 program); beş `add_study` hesabı bit eşit; program özeti `4e77c249…` arm64 = x86_64 = wasm32. Kayıt defterine 20 (sonra 21) `program.*` satırı. | KABUL |
| PA `qp-plane` | Program serisi hesap serisi olarak (D1): aynı önbellek, bağımlılık, yetki, kanıt. Olay serisi tam uzunluk (K13), canlı bar `null`, kapanmış uzunluk ayrı; `threshold ge 1` olay barı kapanınca bir kez, olgunluk `final`; art arda olay → seride her bar, tek bildirim. Artımlı yol yeniden kurmayla bit eşit. Tik: program serisi eşdeğer `add_study`'nin 2–5 katı (µs düzeyi). | KABUL; verify 21/21 |
| WASM (D1 şart 1) | `qp-lang` bağlanınca kabuk paketi ham 7.154.035 / gzip 2.076.328; main zaten I1 tipleri yüzünden D4'ün üstündeydi. Arkin: çalıştırıcı pakette, İ8 yeniden (D6: 7.332.886 / 2.128.236). Pay %2,4. | — |
| M yüzeyler | `qp program check | run | explain`; MCP ince araçları `program_check/try/explain/add` (ajandan öneri; karar aracı yok); `docs/AGENTS.md` programlar bölümü, `agents/programci.md`. Araç tanımları yalnız ekleme (thin 72 → 76). | KABUL |
| U1 RX4 | 26 görev × 2 veri kümesi: **52/52 eşit**, 4/4 ret görevi eşit. İlk koşuda G8-02/03 farklıydı: denetleyici `align(cat)`'i reddediyordu (uygulama hatası, şartname §4.6) → düzeltildi. G2-01/02'de bit eşitliği kısmi: Qapsule `sma` kayan toplam (PR §15 satır 20). `experiments/rx4/RESULT.md`. | düzeltme KABUL |
| U3 kötü niyet | 115 deneme (CLI, `qp serve`, MCP): 109 OK; çökme/takılma yok (200 bin derin JSON, 50 MB IR dahil); statik sınır, çalışma bütçesi (bar 18914'te kodlu durma), kişi başı sınırlar (32 örnek, 2000 maliyet, günlük baytı, komut başı 8 işlem), geleceğe bakma reddi ve önek kararlılığı tuttu. FAIL (düşük): `try/check_program` ikinci girdiyi sessizce yok sayıyordu → düzeltildi (PB, `E_NOT_BAR_SERIES`); L20 PA sonrası yeniden koşuldu, OK. `experiments/abuse/REPORT.md`. | ayrı denetçi |
| PB `qp-program` | Alt sistem: `add/set/remove_program` açılımı (çıktı başına program serisi, isteğe bağlı katman), politika (ajan yalnız öneri; `remove` ajana kapalı), `Check` fazında korumalar (`E_PROGRAM_SERIES`, `E_EVENT_OPERAND`, `E_NOT_BAR_SERIES`, `E_TRY_ONLY`) ve kişi başı bütçe (kabulde ve geri almada yeniden; komut başı işlem sınırı), sorgular `check/try/explain_program`; bileşime bir satır. İlk denetim **RET** (aynı komutta önce program ekleyip sonra o seriyi koşulda kullanmak korumayı atlatıyordu); otomatik güvenlik incelemesi ayrıca sahip kimliği, geri alma ve günlük baytı sayacında altı bulgu çıkardı; hepsi düzeltildi. | 2. denetim KABUL; verify 21/21 |
| C kapanış işleri | Uygunluk dökümleri 71–77 (üç taşımada), verify adımı `program` (22. adım), CLI `W_LIVE_BUDGET` eşliği, performans (`experiments/perf/RESULT.md`), VERIFY 2 karşılaştırması (`COMPARE.md`), L20 eki. | KABUL; verify 22/22 |

## 5. Dil bir sayfada

Program, tek tipli bir veri akışı ara gösterimidir (`qp-ir/0`, `docs/PROGRAMS.md`): adlı, tipli,
aralıklı **parametreler**; topolojik sıralı **düğümler** (her düğüm yalnız kendinden öncekine
başvurur, döngü yapı gereği yok); **çıktılar** (seri, olay, sinyal, durum tanımı). Günlüğe yalnız
ara gösterim yazılır; metin dil yalnız yazım yüzeyidir, iki yönlü (metin → ara gösterim, ara
gösterim → kanonik metin). Program kimliği kanonik biçimin SHA-256'sı; örnek kimliği = program +
bağlı parametreler.

Gösterge (seri):

```
param fast int = 12 in 2..200
param slow int = 26 in 2..400
diff = ema(close, fast) - ema(close, slow)
series spread = diff
series trigger = ema(diff, 9)
```

Koşul (olay; canlı yolda var olan `threshold` koşulu, kesin barda):

```
param len int = 20 in 2..500
top = highest(high, len)[1]
event kirilim = close > top and defined(top)
```

Sinyal (sonuç kuralı zorunlu; yalnız "dene" ve yeniden oynatmada kayıt, `mode: "replay"`):

```
r = rsi(close, 14)
entry = cooldown(cross_below(r, 30), 12)
signal rsi_dip = entry side al target 3% stop 1.5% max_bars 24 source "rsi_dip" "1"
```

Üst periyot seansa hizalı (`close@1d`, `align(...)`); ret: `reject E_LOOKAHEAD "…"`.

**Düğümler (43):** `input const bar_index time`; çekirdekler `sma ema rsi macd bollinger highest
lowest atr stdev percentrank` (`stdev`, `percentrank` yeni; hepsi `qp-compute`'ta aynı kurallarla);
aritmetik `add sub mul div min max neg abs`; `lag`; `gt ge lt le eq ne and or not`; `cross_above
cross_below`; durum `streak bars_since once rearm cooldown`; `defined`; `align`; `line`; `bucket`.

**Bilerek yok:** döngü, kullanıcı işlevi, atamayla durum (`var`), çoklu sembol, emir, oluşan bar
(`partial`), "tut" sinyali, `exp/ln/pow`, Pine. Kesilen: iç içe gösterge adresi (aşağıda).

## 6. Sonuçlar (kabul ölçütleri)

| # | Ölçüt | Durum | Kanıt |
|---|---|---|---|
| 1 | Ön işler | ✅ | §1; Ö1 RESULT, Ö2/Ö3 denetimleri |
| 2 | Kavramlar, PROGRAMS, PROTOCOL koddan önce | ✅ | `ae5ad278` < ilk `qp-lang` commit'i |
| 3 | Tipler; dışa aktarım yalnız ekleme; dökümler/altınlar/özet bayt eşit | ✅ | I1 denetimi; `experiments/perf/COMPARE.md` (270 kalemden 245 eşit, 25 farkın hepsi ekleme) |
| 4 | Bütün hatalar birden, konum + öneri; kod başına sınama | ✅ | `qp-lang/tests/codes.rs`, `all_at_once.rs` |
| 5 | Belirlenimcilik; üç mimaride özet | ✅ | program özeti `4e77c249…` arm64 = x86_64 = wasm32; rl1 çekirdek özeti `e196f240…` |
| 6 | Önek kararlılığı; olay yalnız kesin barda | ✅ | `qp-lang/tests/exec/prefix.rs` (240 rastgele program); denetçi sondası (XIST/UTC, `line`, `align`) |
| 7 | Beş `add_study` hesabı bit eşit | ✅ | `qp-lang/tests/exec` eşdeğerlik (toplu, akışkan, düzeltme) |
| 8 | Üst periyot seansa hizalı | ✅ | XIST günlük/4s (yarım gün, tatil), 7/24 UTC sınamaları |
| 9 | Parametreli program, iki örnek | ✅ | `qp-lang/tests/params.rs` |
| 10 | Metin ↔ ara gösterim gidiş-dönüş; hata konumu; ret | ✅ | `qp-lang-text` (3. denetimde KABUL) |
| 11 | Maliyet önce; bütçe işi sayar; kişi başı toplam `Check`'te; sınırlar kayıt defterinde | ✅ | 21 `program.*` satırı; U3 |
| 12 | Oturum: ekle/değiştir/kaldır/denetle/dene; seri grafikte ve koşulda; olay kesin barda; yeniden oynatma; ajan önerisi | ✅ | PA, PB denetimleri; dökümler 71–77 |
| 13 | `qp-authority/src` farkı | ✅ (D2) | yalnız iki öznitelik değişikliği (+2/−2, yeni satır yok; Arkin'in kararı D2); yeni istisna yok |
| 14 | MCP araçları, `qp program`; ajan belgeleri | ✅ | M denetimi; `docs/AGENTS.md`, `agents/programci.md` |
| 15 | RX4 uygunluğu; her fark açıklanmış | ✅ | 52/52 eşit, 4/4 ret (`experiments/rx4/RESULT.md`) |
| 16 | Ajan teyidi; kötü niyet | ✅ | §8, §9 |
| 17 | K17, K52, WASM (İ8), tik maliyeti | ✅ | §10: K17 var olan taramada ve program taramasında (`qp_lang::sweep`, beş §14 programı yerel 12–24 ms, WASM 17–30 ms) tutuyor; K52 uyarısız; WASM D6/D7 içinde; tik ölçüldü |
| 18 | Bağımsız denetim, belgeler, temizlik | ✅ | `AUDIT.md`, §14 (Disk); CI 45814608591 yeşil (`d3f02eba`, Linux x86_64, 47 dk) |

## 7. RX4 uygunluğu

26 görev × 2 veri kümesi (seen, heldout), UTC hizalı kapalı pencere: **52/52 eşit**, ret
görevleri 4/4 eşit (`experiments/rx4/RESULT.md`). İlk koşudaki tek fark (G8-02/03) bir uygulama
hatasıydı (`align(cat)`), düzeltildi. Kısmi bit eşitlik (G2-01/02): Qapsule `sma`'sı kayan toplam,
RX4 başvurusu baştan toplam; bilinçli sapma (PR §15 satır 20; `add_study` bit eşitliği bunu
zorunlu kılar).

## 8. Ajan yazabilirlik teyidi

RX4 kümesinden 20 görev (4 ret, 1 tuzak dahil), MCP ince yüzeyi üstünden, Claude Code alt
ajanlarıyla (`experiments/agents/RESULT.md`):

| Ölçüt | haiku | RX4 A-haiku | opus | RX4 A-opus |
|---|---:|---:|---:|---:|
| İlk denemede doğru | 20/20 | 20/20 | 20/20 | 20/20 |
| Hata iletisinden sonra doğru | 2/2 | — | 1/1 | — |
| Ret gereken yerde ret | 4/4 | 4/4 | 4/4 | 4/4 |
| Uydurma işlev adı | 0 | 0 | 0 | 0 |
| Sessiz yanlış | 0 | 0 | 0 | 0 |

RX4 ile aynı bantta. Bütün teslimler metin dilde. Belirteç medyanı (~338 bin) RX4'ten yüksek;
çoğu alt ajanın sabit başlangıç yükü (doğruluk değil maliyet farkı).

## 9. Bütçe ve kötü niyet

Maliyet çalıştırmadan önce: düğüm türü × parametre × bar sayısı (PR §8.1; bar başı sınır
`program.max_cost_per_bar` 666). Çalışma bütçesi işi sayar: bar × ağırlık + doğan kayıt + açık
kayıt + okuma; aşınca kodlu durur (`stopped {bar, t, work, limit}`). Kişi başı (doküman içinde):
örnek sayısı, canlı bar maliyeti, günlük baytı (`AfterCommit`'te güncellenen yuva), komut başı
program işlemi; hepsi `Check` fazında, kabulde ve geri almada yeniden. Sınırlar kayıt
defterinde (21 `program.*` anahtarı).

Kötü niyet (ayrı denetçi, `experiments/abuse/REPORT.md`): 115 deneme, 109 OK + L20 (PA sonrası
yeniden koşuldu, OK). Çökme/takılma yok (200 bin derin JSON, 50 MB IR, 60 bin parantez). Bulunan
iki düşük önemli hata (çok girdili `try/check` sessizce ilkini alıyordu) düzeltildi. Round boyunca
otomatik güvenlik incelemesi ve denetçiler kötü niyet yüzeyinde ayrıca on üç bulgu çıkardı (metin
dilinde yığın taşması, denetleyicide üstel özyineleme ve geç sınır, bütçede sahip kimliği, geri
alma, aynı komutta koruma atlatma); hepsi birleştirmeden önce düzeltildi.

## 10. Performans ve WASM

| Ölçüm | Değer | Bütçe |
|---|---|---|
| Varsayılan modül (adım 8) | ham 2.602.684 / gzip 817.223 B (RF1: 2.354.884 / 773.639; +%10,5 / +%5,6) | ±%2 kuralı aşıldı → Arkin: yeni taban (D7) |
| Kabuk paketi (adım 18) | ham 7.167.360 / gzip 2.078.388 B | D6 İ8 7.332.886 / 2.128.236 (pay %2,3) |
| K17 var olan tarama | yerel 4,1–9,5 ms, WASM 10,9–20 ms | 50 / 150 ms ✅ |
| K17 program taraması (`qp_lang::sweep`, beş §14 programı, 10.000 bar × 1.000 değer) | yerel 12–24 ms, wasm32 17–30 ms; her varyasyon tek tek koşuyla bit eşit (ilk ölçüm, varyasyon başı çalıştırıcıyla: 264 / 392 ms) | 50 / 150 ms ✅ |
| K52 (30 program açıkken) | tik p95 değişmedi (0,4 / 1,3 ms); ilk sahne kurulumu p95 27–30 → 43–45 ms (p50 aynı) | uyarı yok |
| Tik (program serisi) | yerel 0,7–2,7 µs / 32 seri; WASM ≤ 4,1 µs | — |
| Çalıştırıcı | 25–66 ns/bar (§12.5 programları), XIST takvimli 286–304 ns/bar | — |

K17 program yolunda ilk ölçümde tutmuyordu (çalıştırıcı her varyasyonu ayrı koşuyordu). Kapanışta
düzeltildi: `qp_lang::sweep` parametreye bağlı olmayan düğümleri bir kez hesaplar, taranan
çekirdekleri `qp_compute::sweep_each` ile paylaşımlı koşar, gerisinde varyasyon başına çalıştırıcıya
döner (`SweepPath` nedenini söyler); bağımsız denetimde KABUL (`experiments/perf/RESULT.md`).
Sınır: tarama yalnız seri çıktısı döndürür; komut satırı ve MCP'de henüz yok (Öneriler 4).

## 11. Alt sistem sözleşmesi ve çekirdek

- **Yetmedi, iki yerde** (D1): `ConditionKind` (yeni koşul kolu çekirdekteki iki kapalı eşlemeyi
  kırar, canlı yol belleği türsüz koşulu değerlendirmez) ve `ValueSource` (çizim, önbellek,
  bağımlılık, yetki onu görmez). Çözüm: program çıktısı hesap serisi; olay = kapanmış bar serisi +
  var olan `threshold` koşulu. Bilinen sınırlar: art arda olaylarda tek bildirim, toplu bar
  eklemede 0,1,0 deseni bildirimsiz (seri ve kanıt doğru; `check`/"dene" söyler).
- **Eksik kancalar, çözümleriyle:** `experiments/hooks/HOOKS.md` (D1 şart 3). Özet: `ValueSource`
  sözleşmeden çıkarılsın (0 çekirdek satırı; uzantı yolu "qp-plane'de hesaplanan doküman serisi");
  bar başı olay değerlendirmesi `qp-program`'ın `AfterData` fazıyla 0 çekirdek satırıyla
  yapılabilir, çekirdek sicili gerekirse `ConditionKind::edges` ≈ +55–70 satır (bayt eşit ama İ3
  kararı ister); `param_for` + `Ctx` uzantısı ≈ +21 satır, bayt eşit, RS1 kişi başı bütçesi için
  gerekli; `Condition::Ext` R7 için gerekmiyor (+4 satır olurdu). **Not:** `qp-authority` İ3
  tavanında (12.744); her net satır karar ister. R7'den önce karar.
- Ayrıca eksik: `SessionView::param` otoritede `None` (kayıt defteri varsayılanları geçerli);
  kararı verenin ilke kimliği açılım bağlamında yok (sahip, önerenin adına çalıştığı kişi olarak
  çözüldü); kamu dünyası yansısı `Check`'e uğramıyor (PS §6).
- **Çekirdeğe dokunuldu mu:** `qp-authority/src`'de yalnız D2'nin iki öznitelik değişikliği (lint
  izni, yeni satır yok). `qp-session/src`'de yalnız `compose.rs` kayıt satırı. Bileşim dışında
  dokunulan: `qp-plane` (program serisi hesabı), `qp-ops` (`canon.rs` program serisinin alanı;
  yeni kollar "burada sunulmaz"), `qp-read`, `qp-schema`.

## 12. R7 ve RS1 bu dile nereden bağlanacak

- **R7 çizim seviyeleri:** `line {a:{t,price}, b:{t,price}}` düğümü; çapalar parametre olabilir
  (sürükleme = `set_program_params`, aynı seri kimliği, yeni örnek, `since_t` geçmişe bildirim
  doğurmaz). İzleme görevleri: olay çıktısı + `threshold` (D1); dokunma türü koşullar en küçük
  periyodun kapanmış barında. Bar başı değerlendirme ya da çekirdek sicili gerekiyorsa HOOKS.md
  kararı. Program yazma ekranı R7'de.
- **RS1 sicil:** sinyal kaydı biçimi RX5 `LedgerEntry` adlarıyla (`born_t`, `known_t`, kimlik `t`
  ile, `mode`), sonuç kuralı doğumda; bugün yalnız `"replay"`, canlı yol aynı kodla `"live"`
  yazacak. `program_id` kararlılık kuralları (PR §2) sicil kimliğini korur. Kişi başı bütçe için
  `param_for` kancası. Durum tanımı (`state`) X34/1'in taban oranına yeter. Notlar: PS §8.

## 13. Kendi ajanınla beş dakikada deneme

1. `cargo build -p qp-cli` (ya da `scripts/qp-mcp.sh`'nin kullandığı ikili).
2. Komut satırı: aşağıdakini `deneme.qp` diye kaydet ve
   `target/debug/qp program check deneme.qp`,
   `target/debug/qp program run --data fixtures/bars/sample.json deneme.qp`,
   `target/debug/qp program explain deneme.qp`:
   ```
   r = rsi(close, 14)
   series rsi14 = r
   event asiri = cross_above(r, 70)
   ```
3. Ajanla: `.mcp.json`'daki `qapsule` sunucusunu `QP_MCP_SURFACE=thin` ile aç; ajana "RSI 70'i
   yukarı kestiğinde olay kuran bir program yaz, önce `program_check`, sonra `program_try`, sonra
   `program_add` ile öner" de. Öneri `ack.status: "proposed"` döner; kararı sen verirsin
   (`accept_proposal` insan kökenli). Dil kartı: `rounds/RL1/experiments/agents/LANG-CARD.md`;
   ajan rolü: `agents/programci.md`; ayrıntı `docs/AGENTS.md` "Programlar".

## 14. Disk

- **Silinenler:** `target/` 35 GB → 0 (`scripts/clean.sh --deep`: artımlı önbellek 18 GB, eski
  hedef klasörleri, `cargo clean` 20,9 GiB; günlük `experiments/clean.log`); iş başına derleme
  klasörleri (`target/wt/*`, her biri 2–21 GB) iş bitince silindi, son denetiminki 19 GB; bütün
  worktree'ler (Ö1–Ö3, I1, I2, LA, LT, LB, PA, PB, M, C, düzeltme) birleşince kaldırıldı; U2 ham
  koşuları (49 MB) ve oturum geçici dosyaları; `agents/__pycache__`.
- **Bilerek bırakılanlar:** dallar (`rl1/*`, kod geçmişi için); `fixtures/local/` 1,5 GB (saklama
  sınırı içinde, canlı kayıt 2 GB / önbellek 5 GB); kanıt dosyaları `rounds/RL1/experiments/`
  (her biri < 1 MB, klasör < 20 MB; en büyüğü `rx4/data/seen.bars.json` 217 KB, sentetik).
- **Kalan boyutlar:** `target/` 1,8 MB; `.claude/worktrees/` 44 KB (boş); `fixtures/local/` 1,5 GB;
  depo toplamı 2,1 GB; depo dışı oturum klasörü 16 KB. `git worktree list` yalnız ana ağaç;
  `git ls-files Prompt.txt` boş; `git status --porcelain --ignored` özetinde beklenmeyen artık yok
  (yalnız `.DS_Store`, `settings.local.json`).

## Kesilenler

1. **İç içe gösterge adresi** (`ISCTR:5m:EMA[RSI:18]:20` → program; K63): kesme sırasının ilki;
   zaman öbür kesilemez işlere (uygunluk, ajan teyidi, güvenlik düzeltmeleri) verildi. Dil bunu
   ifade ediyor (`ema(rsi(close, 18), 20)`); eksik olan yalnız adres ayrıştırıcısı. R7'ye.

Kesme sırasındaki öbür maddeler (kutulama ve durum tanımı, kanonik metin yönü, `percentrank` /
`stdev`, küçük modelle teyit) **yapıldı**.

## Öneriler

1. **Görünürlükle izleyici kapatma (Ö2):** `qp_authority::Follower` yeni tipi ya da yetenek
   belirteci; bugün `calls.restricted` makine kuralı koruyor (makro ile atlatılabilir).
2. **HOOKS.md kararı R7'den önce** (`ValueSource` çıkarılsın; bar başı olay için `AfterData`
   yolu; `param_for` + `Ctx` uzantısı).
3. **Program serisinin günlük maliyeti:** her `set_program_params` ara gösterimi çıktı başına
   yeniden yazar; sayılan bayt yalnız `changes` (istek, ters değişim ve bekleyen öneriler
   sayılmıyor, bekleyen öneri sayısına sınır yok).
4. **Program taraması yüzeyi:** `qp compute --sweep`'e program kipi ve `sweep_program` sorgusu
   (API `qp_lang::sweep` hazır).
5. **K52 ilk sahne kurulumu** program serisiyle uzuyor (p95 27–30 → 43–45 ms); nedeni ölçülmeli.
6. **`program_try` kapalı pencerede son barı düşürüyor** (`window.to_t` ile bağlı seri;
   `claims::closed` o barı kapalı sayıyor). Ayrıca `E_TRY_ONLY` ipucu araç adını `try_program`
   diyor, MCP aracı `program_try`.
7. **`qp serve` ajan kuralı:** `agent_policy` verilmeden hiçbir bağlantı ajan sayılmıyor (R5B
   tasarımı); varsayılan yapılsın ya da belgelensin (PS §2.2'ye not yazıldı).
8. **Sahip bütçesi:** başka yazarın `set_program_params`'ı sahibin `max_log_bytes`'ını tüketir;
   günlük baytı küçülmez.
9. **XIST takvimli üst periyot** bar başına iki `bar_at` çağrısı ve ayırma (≈ 300 ns/bar);
   önbellekleme.
10. **Paylaşılan `CARGO_TARGET_DIR`** worktree'ler arasında aynı crate'in eserlerini karıştırıp
    yanlış derleme sonucu veriyor (bu round'da ölçüldü); K56 metni "iş başına klasör" diye
    güncellensin (`rounds/RL1/AGENT-RULES.md`).
11. **binaryen indirmesi:** CI'da `~/.cache/.wasm-pack` önbelleğe alınsın (Ö1 denetimi).
12. **`scripts/refactor/snapshot.sh`** rl1 ve program özetlerini kaydetmiyor.
13. **SHA-256 ve en yakın ad** `qp-lang`'de yeniden yazıldı (katman sırası); ileride `qp-schema`'ya
    inebilir.
14. **`live_budget_warning`** CLI'da ve `qp-program`'da ayrı kopya; eşitlik sınaması tutuyor, ama tek
    yere (ör. `qp-lang`) taşınsın (C denetimi).

## Açık sorular (Arkin)

1. HOOKS.md önerileri (R7'den önce).
2. Program taramasının yüzeye açılması (Öneriler 4) R7'de mi RS1'de mi?
3. Öneri 7 (`qp serve`'de ajan kuralı varsayılan mı).
4. Metin dilin varsayılan sınırları kayıt defterinden mi okunsun (bugün `TextLimits` varsayılanı yapısal sabit; CLI ve oturum `program.text.*` anahtarlarından okuyor).
