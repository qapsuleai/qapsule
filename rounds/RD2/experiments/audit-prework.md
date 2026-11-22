# RD2 ön işler birleştirme adayı denetimi (rd2/prework-merge @ 0e0f53df)

Karar: KABUL (iki küçük not, koşul değil)

1. verify.sh: PASS. 19/19 adım PASS, atlanan yok (browser ve demo koştu). Toplam 1019 s, CARGO_TARGET_DIR=target/wt/audit-prework.
2. arch-check: PASS. 0 hata, 11 adlandırılmış istisna. qp-session 31.999 (tavan 31.999, diff'te qp-session değişikliği yok), qp-grammar 19.330 (tavan 21.000), qp-capsule 13.230 (tavan 13.230). --table çıkış 0. --self-test PASS; mandal durumları: değer==tavan geçer, değer>tavan hata, max'sız crate.lines istisnası hata. arch-baseline.json main'e göre değişmemiş; arch-config yalnızca max alanları ekliyor (İ3/İ5 güncel değere, İ4 K89 21.000). Gevşetme yok.
3. Fikstürler: PASS. 9aee506c..HEAD diff --stat boş.
4. A62: PASS. `qp mcp --help` yalnızca `serve` gösteriyor; `--init` -> "unexpected argument", çıkış 2. human_decision 5/5 geçti. Testler dürüst: sunulan her yüzey (scout ve arkin için) karar aracı göstermiyor; kaynak taraması `McpServer::connect(`/`with_decisions(` çağrısının src/examples altında yalnızca conform.rs/server.rs/thin.rs'de olduğunu doğruluyor. qp-cli/tests/mcp.rs eski bayrakları ve yardım metnini kontrol ediyor.
5. WASM: PASS. Yeniden derlendi (wasm-pack web, profil wasm, --no-opt, replica): ham 8.731.379 B, gzip -9 2.408.953 B <= 2.506.216 B (-97 KB, const->static ile tutarlı).
6. Dallar/silmeler: PASS. recovered/* 0; r5/m16-bertan ve r5/m16-kivra var; R3 harness/target yok.
7. Kod incelemesi: PASS. Kapsam dışı davranış değişikliği yok; ölü CLI yolu kaldırıldı; docs (AGENTS, ARCHITECTURE, DEBT, NOTES, cmd/README, rg1-snapshot) tutarlı.

Notlar:
- rounds/R5B/experiments/harness/driver/test_qmcp.py hâlâ `qp mcp --init` çağırıyor (tarihsel deney, verify dışı). REPORT'ta anılmalı.
- Mandal yalnızca crate.lines istisnalarına uygulanıyor; İ6 file.hard için tavan yok (K89'un "istisna değerleri mandallıdır" ifadesi ileride bunu da kapsayabilir). Kaynak taraması metin eşleşmesine dayanıyor (`use ...connect as x` ile atlatılabilir); yeterli ama katı değil.
- VERIFY satır 3: RG1 tutanağındaki mcp/adapter.jsonl dökümü artık üretilmiyor; REPORT'ta adıyla anılmalı.
