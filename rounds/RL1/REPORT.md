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

## Kesilenler

(henüz yok)

## Öneriler

(round sırasında doldurulur)
