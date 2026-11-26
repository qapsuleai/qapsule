# RL1 — Rapor (hesap dili)

Durum: sürüyor.

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
| U3 kötü niyet | 115 deneme (CLI, `qp serve`, MCP): 109 OK; çökme/takılma yok (200 bin derin JSON, 50 MB IR dahil); statik sınır, çalışma bütçesi (bar 18914'te kodlu durma), kişi başı sınırlar (32 örnek, 2000 maliyet, günlük baytı, komut başı 8 işlem), geleceğe bakma reddi ve önek kararlılığı tuttu. FAIL (düşük): `try/check_program` ikinci girdiyi sessizce yok sayıyor → düzeltiliyor. `experiments/abuse/REPORT.md`. | ayrı denetçi |

## Kesilenler

(henüz yok)

## Öneriler

(round sırasında doldurulur)
