# M28 — Araç yüzeyi ölçümü (sonuç)

Düzenek: `rounds/R5B/experiments/harness/` (SPEC §5.4 seçim kuralı ölçümden önce commit edildi).
4 yüzey (`thin`, `task`, `query`, `mixed`) × 3 model (haiku, sonnet, opus; Claude Code alt
ajanı) × 45 görev (g1–g4 dörde bölünmüş, her koşu taze sunucu) = 48 koşu, 2160 görev cevabı.
Sentetik oturum (seed 5); dökümler depoya girmedi (scratchpad, kapanışta silindi).
Toplam alt ajan belirteci: 4 308 829 (48 koşu; koşu başı ~72–116 k).

## Seçim

**`thin` (protokol aynası)** — `score.py tasks … --select`:
birincil ölçüt (model ortalaması, ilk denemede doğru) thin 0,822 · query 0,763 · task 0,763 ·
mixed 0,756. Havuzlanmış Wilson aralıkları örtüştü (thin [0,749, 0,878] — query [0,685, 0,827])
→ "ayırt edilemedi" → (a) küçük model: thin haiku 0,756, diğerleri 0,667–0,689 → **thin**.
Sızıntı 0 (her yüzeyde); hiçbir koşuda ajan öneri kararı denemedi (istemci günlüklerinde
`accept/reject_proposal`, `approve/reject_finding` çağrısı yok) → güvenlik koşulu engel değil.

**Küçük model (haiku) ayrı:** thin 34/45 (0,76) · query 31/45 · mixed 31/45 · task 30/45.

## Tablo (score.py çıktısı)

| yüzey | biçim | model | n | ilk denemede | %95 | doğru | çağrı | belirteç/görev | sızıntı | sayı aktarım hatası | araç tanımı (B) |
|---|---|---|---:|---:|---|---:|---:|---:|---:|---:|---:|
| mixed | None | haiku | 45 | 31 (0.69) | [0.54, 0.80] | 36 | 1.8 | 9536 | 0 | 1/7 | 95547 |
| mixed | None | opus | 45 | 37 (0.82) | [0.69, 0.91] | 45 | 2.7 | 7825 | 0 | 0/7 | 95547 |
| mixed | None | sonnet | 45 | 34 (0.76) | [0.61, 0.86] | 42 | 2.2 | 7620 | 0 | 0/7 | 95547 |
| query | None | haiku | 45 | 31 (0.69) | [0.54, 0.80] | 39 | 1.8 | 8773 | 0 | 0/7 | 16300 |
| query | None | opus | 45 | 37 (0.82) | [0.69, 0.91] | 41 | 3.4 | 7600 | 0 | 0/7 | 16300 |
| query | None | sonnet | 45 | 35 (0.78) | [0.64, 0.87] | 41 | 2.3 | 7197 | 0 | 1/7 | 16300 |
| task | None | haiku | 45 | 30 (0.67) | [0.52, 0.79] | 33 | 2.5 | 7834 | 0 | 2/7 | 19755 |
| task | None | opus | 45 | 38 (0.84) | [0.71, 0.92] | 39 | 3.3 | 8140 | 0 | 1/7 | 19755 |
| task | None | sonnet | 45 | 35 (0.78) | [0.64, 0.87] | 41 | 3.3 | 7028 | 0 | 1/7 | 19755 |
| thin | None | haiku | 45 | 34 (0.76) | [0.61, 0.86] | 44 | 2.1 | 8867 | 0 | 0/7 | 523564 |
| thin | None | opus | 45 | 40 (0.89) | [0.77, 0.95] | 45 | 2.4 | 7651 | 0 | 0/7 | 523564 |
| thin | None | sonnet | 45 | 37 (0.82) | [0.69, 0.91] | 42 | 2.5 | 7679 | 0 | 0/7 | 523564 |

`araç tanımı (B)`: `tools/list` sonucunun sıkıştırılmış JSON boyutu (kurala girmez, §5.4.4).
thin 524 KB ile en ağırıdır (her qp/0 işlemi ayrı araç + tam şema); istemci tanımları
tembel yüklüyorsa (Claude Code araç araması) maliyet görünmez, yüklemiyorsa bağlamı
doldurur. Bu, `docs/AGENTS.md`'de not edildi; görev başına belirteçte fark çıkmadı
(thin 8066, en düşük task 7668).

## Kategori bazında gözlemler

- `propose` en zayıf kategori: task/query yüzeylerinde ilk denemede 0/4 (üç modelde de),
  thin'de 2–4/4. Ajanlar öneri işleminde ilk denemede şema hatası yapıyor; thin'in
  işleme özgü araçları (`op_set_view_range` vb.) şemayı doğrudan veriyor.
- `find` (yapı bulma) haiku'da her yüzeyde zayıf (3–5/8).
- `guest` (yetkisiz okur): bütün koşularda 6/6 — sızıntı yok.
- Sayı aktarım hatası (motor değeri ≠ cevap): en çok 2/7 (task haiku); thin'de 0.

Kategori satırları: `summary.json` (`by_category`).
