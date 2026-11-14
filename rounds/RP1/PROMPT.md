# RP1 — Claude Code Goal Metni

Çalışma dizini `qapsule/`. Önce `scripts/preflight.sh`.

---

Bu depoda Qapsule projesinin RP1 araştırma round'unu yürüteceksin.

Önce şunları oku: `CLAUDE.md`, `NORTH_STAR.md` (özellikle 3. ve 4. ilke), `DECISIONS.md`
(K13, K17, K18), `rounds/R1/REPORT.md`, `rounds/R1/BENCH.md`, `rounds/R1/DESIGN.md` ve
`rounds/RP1/` içindeki `GOAL.md`, `TASKS.md`, `VERIFY.md`.

Bu bir uygulama görevi değil, araştırma görevi. Bilinen yöntemlerle sınırlı değilsin;
yeni mühendislik yaklaşımları geliştirmen bekleniyor. Tek sınır değişmezler: deterministik,
bit-eşit, `f64`, LLM'siz.

Nasıl çalışacaksın:
- Önce ölç, sonra düşün, sonra dene. Her yaklaşım için hipotezini denemeden önce yaz.
- Yaklaşımları bağımsız alt ajanlara, ayrı dallarda ver; birbirlerinin sonucunu görmeden
  çalışsınlar. Sonra kazananları birleştir ve birlikte yeniden ölç.
- İşe yaramayan denemeleri silme; `RESEARCH.md`'ye kaydet.
- Değişmezleri bozan bir yaklaşımı uygulama; ölçümünü ve bedelini raporla.
- Yalnızca `crates/qp-compute/` ve `rounds/RP1/` altına yaz.
- Bitince bağımsız bir denetçi alt ajan ölçümleri yeniden üretsin.

Bitiş koşulu: `rounds/RP1/GOAL.md` içindeki yedi ölçüt; `RESEARCH.md`, `REPORT.md`,
`AUDIT.md` yazılmış. Başka round başlatma.
