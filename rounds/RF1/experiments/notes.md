# RF1 yürütme notları (rapora girdi)

- T0: ana akış belge değişiklikleri `05430e06` ("ana akış: …"; SKILL.md dahil). Alt ajan
  başlatılabiliyor (eleştirmenler, Ö, A1/A2 alt ajanlarla; worktree yalıtımı çalışıyor).
- B: tutanak `7fb01d11` (270 kalem; iki koşu bayt eşit). `scripts/rg1-*` → `scripts/refactor/`.
- Ö (Playwright): `$TMPDIR` altında yalnız üç klasör vardı, tarihleri doğrulandı ve silindi:
  `playwright_chromiumdev_profile-nYVsik` (14 Kasım, 6,7 MB), `playwright_chromiumdev_profile-FpDzx7`
  (16 Kasım, 144 KB), `playwright-artifacts-Kmn72H` (16 Kasım, boş). Başka depo dışı klasöre dokunulmadı.
- S1: d2 kolu (project_changes SetCapsuleState gizli) bugün ulaşılamaz savunma kolu (audit-s1 yeniden denetim); S3.2'de as_reader taşınırken yanına gerekçe yorumu.
- Sıra notu: S4 (`expand_op` bölmesi) S2'den sonraya alındı. `qp-session` İ3 tavanı 31.999, değer 31.997; fonksiyon başlıkları tavanı aşardı. `expand_op` önce `qp-ops`'a iner, orada bölünür.
- Kapanan istisnalar için karar satırı kapanışta önerilen kararlara (docs/decisions/rounds/RF1.md): İ2 (A2), İ4 (G2), İ5 (V), İ6 prepare (G3), İ6 op.rs (S4). audit-g3 B1: birden çok hatayı aynı anda kıran vakalar yok (sıra değişimi yakalanmaz) — Fark edilenler.
- audit-s33 Y1 (Öneriler): komut yolu dışından yazan gizli izleyici yöntemleri (follow_start, mirror_apply, mirror_commit, follow_data, follow_resync, take_provider) her Session'da; DerefMut yüzünden her barındırıcı çağırabilir. Öneri: Follower(Session) tipi. N1: kabuk WASM İ8 tavanına 4,8 KB; wasm-opt önerisi öncelikli.
