# RD2 — Tasarım turu: rapor (yazılıyor)

Round başı: `9aee506c` (ana akış commit'i; RG1 sonu `ec1c7c89`).

## 0. Round başı notları

- **Ana akış belgeleri** `9aee506c` ile kaydedildi (CLAUDE.md, ROADMAP, STATUS, ENGINEERING,
  kararlar K88–K89, research, `rounds/RD2/`). `NORTH_STAR.md` değişikliği yoktu.
- **Alt ajanlar başlatılabiliyor.** İlk deneme `isolation: worktree` ile düştü (oturumun çalışma
  dizini `../refs`'e kaymıştı, git deposu değil); worktree'ler elle `.claude/worktrees/rd2-*`
  altında açıldı, alt ajanlar oraya yönlendirildi. Ortak derleme klasörü `target/` (K56).
- **Oturumdaki tasarım becerileri** (round başı listesi):
  `frontend-design:frontend-design` (maket ve sözleşme işçiliği), `data:data-visualization` ve
  `dataviz` (grafik ve veri gösterimi, renk), `anthropic-skills:skill-creator` ve
  `superpowers:writing-skills` (proje becerisi), `artifact-design` (yalnız karşılaştırma sayfası
  biçimi için), `data:create-viz`, `data:build-dashboard` (kullanılmadı). Öncelik: depo kuralları >
  uyarlama envanteri ve sözleşme > beceri önerileri (BRIEF).

## 1. Ön işler (Ö1–Ö3) — main `0e0f53df`, denetim KABUL (`experiments/audit-prework.md`)

- **A62 (Ö1, `911a957c`):** eski `qp mcp --init/--principal/--surface/--root` bağdaştırıcısı
  kaldırıldı (`McpArgs`, `cmd/serve.rs::mcp()`, test `mcp_stdio_smoke` → `the_rd1_adapter_is_removed`).
  `qp mcp` artık yalnız `serve`. `human_decision.rs` iki yeni test: hiçbir sunulan yüzey (thin, task,
  query, mixed) ne ajana ne insana karar aracı vermiyor; `McpServer::connect`/`with_decisions`
  yalnız uygunluk koşucusundan çağrılıyor. MCP üzerinden uygunluk 70/70 süreç içinde sürüyor.
  **Kaldırılan döküm:** RG1 tutanağındaki `mcp/adapter.jsonl` artık üretilmiyor (`rg1-snapshot.sh`).
  Kalan eski çağrı: `rounds/R5B/experiments/harness/driver/test_qmcp.py` (deney, verify koşmuyor).
- **İ7 (Ö2, `4bab81b9`):** `fonts::FILES`, `cpu::FONT_BYTES` → `static`. Kabuk paketi
  (`--features replica`) gzip 2.570.017 → **2.408.899 B** (RG1 tabanı 2.506.216'nın %3,9 altında;
  denetçi yeniden derledi: 2.408.953); ham 9.101.210 → 8.731.379; brotli ≈ aynı. Varsayılan
  `qp_wasm.wasm` değişmedi.
- **Silmeler (Ö3):** 24 `recovered/worktree-agent/*` dalı (silmeden önce `recovered.md` ile
  karşılaştırıldı; uçlar raporun ekinde) silindi, `r5/m16-bertan`, `r5/m16-kivra` kaldı;
  `rounds/R3/experiments/harness/target/` (140 MB, izlenmiyordu) silindi.
- **İstisna mandalı (Ö3, `01b63fe1`):** `crate.lines` istisnası `max` taşır; aşılırsa hata. Tavanlar
  qp-session 31.999, qp-grammar 21.000 (K89), qp-capsule 13.230. Öz-sınamada üç yeni durum.
  Not: İ6 (fonksiyon boyu) istisnalarının tavanı yok; RF1'e öneri.

## 2. Sıra notu: ortak alfabe yönlerden önce

TASKS C2'yi (alışılmış grafik öğeleri) ara karardan sonraya koyuyor. Uygulamada C2'nin **ortak**
kısmı (P4–P8: fiyat ekseni yoğunluğu, iki düzeyli zaman, çakışma önleme, son değer + değer çizgisi,
imleç hapları, OHLC lejantı, hacim, işaret boyu) yönlerden önce, `rd2/c2a-alphabet` dalında kuruldu.
Gerekçe: üç yön aynı uyarlanmış alfabeyi paylaşır (GOAL §2) ve karşılaştırma başvuru ekranıyla
yan yana yapılır (P14); R5 düzeyindeki grafikle (tek eksen etiketi, hacim yok) yapılan bir
karşılaştırma VISUAL §3.2/6'daki karıştırıcı etkeni tekrarlardı. Kural korunur: alfabe görünüş
kaydının `ChartLook` alanlarıyla açılır, var olan görünüşler (r4, ornek, oltan, zemre) bayt eşit,
gösterim varsayılanı değişmez; **seçilen yön** ancak Arkin'in kararından sonra çekirdeğe işlenir.

## 3. Üç yön ve ara teslim (K1)

| Yön | Görünüş | Dal / commit | Odak | design-check (2 profil × 2 tema) | qp-grammar |
|---|---|---|---|---|---|
| A | `sakel`, `sakel-tel` | `rd2/y-a` `6d4f17f5` | sözleşmeye sadık, Qapsule katmanı en sessiz | P1–P13, H1–H3 PASS (P9 6/3/7) | +289 |
| B | `tavek`, `tavek-tel` | `rd2/y-b` `193147c0` | anlam grafiğin üstünde, grafiğin dilbilgisiyle | PASS (P9 5/3/7) | +285 |
| C | `ravek`, `ravek-tel` | `rd2/y-c` `7b6d05ca` | anlam akışta; sicil tek bakışta | PASS (P9 5/3/7) | +264 |

Üç yön aynı tabandan (`rd2/c2a-alphabet`), birbirini görmeden üç alt ajanla kuruldu. Galeri dalı
`rd2/gallery-all` (yalnız karşılaştırma; main'e girmez). Galeri: sentetik veri
`rounds/RD2/experiments/gallery/out/` (ignored), gerçek veri (GARAN, günlük) depo dışında
oturum klasöründe; aynı veri, aynı boyut, masaüstü DPR 2, telefon DPR 3, başvuru ekranı yanda,
`oltan` (R5) aynı düzenekte. Ajan eleştirmeni kullanılmadı; kusur taraması gözle yapıldı.
