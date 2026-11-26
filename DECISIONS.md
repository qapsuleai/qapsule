# Karar Günlüğü

Karar kaydı RG1'de (2026-11-22) belge bütçesi (`docs/ENGINEERING.md` §3) yüzünden bölündü.
Bu dosya dizindir; satırlar aşağıdaki dosyalarda, **numaraları ve metinleri değişmeden**
durur. Bir başvuru "`DECISIONS.md` K16" ya da "`DECISIONS.md` \"Önerilen (R5)\"" diyorsa
numarayı ya da bölüm adını bu tablodan bulun.

| Bölüm (özgün başlık) | Dosya | Numaralar | Eski adı (round belgelerinde) |
|---|---|---|---|
| Verilmiş kararlar | [`docs/decisions/kararlar.md`](docs/decisions/kararlar.md) | K1–K85 |  |
| Açık kararlar | [`docs/decisions/acik.md`](docs/decisions/acik.md) | A1–A9 |  |
| RD1 kararları — kabul edildi (2026-11-15, K22); Ö9 K23 ile geçersiz | [`docs/decisions/rounds/RD1.md`](docs/decisions/rounds/RD1.md) | Ö1–Ö9, A10–A12 | "Önerilen (RD1)" |
| R2 kararları — kabul edildi (2026-11-15, K24) | [`docs/decisions/rounds/R2.md`](docs/decisions/rounds/R2.md) | Ö10–Ö17, A13–A14 | "Önerilen (R2)" |
| RP2 kararları — kabul edildi (2026-11-15, K35) | [`docs/decisions/rounds/RP2.md`](docs/decisions/rounds/RP2.md) | Ö18–Ö28, A15–A20 | "Önerilen (RP2)" |
| R3 kararları — kabul edildi (2026-11-16, K41) | [`docs/decisions/rounds/R3.md`](docs/decisions/rounds/R3.md) | Ö29–Ö41, A21–A30 | "Önerilen (R3)" |
| R4 kararları — altyapı kabul (K46); tasarım seçimleri Ö45–Ö48 geçici | [`docs/decisions/rounds/R4.md`](docs/decisions/rounds/R4.md) | Ö39–Ö51, A31–A33 | "Önerilen (R4)" |
| RP3 kararları — kabul edildi (2026-11-17, K51); Ö54 ertelendi | [`docs/decisions/rounds/RP3.md`](docs/decisions/rounds/RP3.md) | Ö52–Ö60, A34–A40 | "Önerilen (RP3)" |
| R5 önerileri — K57 ile kısmen kabul (altyapı ve dil kuralları); tasarım seçimleri kabul edilmedi, RD2'ye | [`docs/decisions/rounds/R5.md`](docs/decisions/rounds/R5.md) | Ö61–Ö71, A41–A45 | "Önerilen (R5)" |
| R6 önerileri — kabul edildi (2026-11-20, K64) | [`docs/decisions/rounds/R6.md`](docs/decisions/rounds/R6.md) | Ö72–Ö76, A47–A52 | "Önerilen (R6)" |
| Önerilen (RP4) — Arkin'in onayını bekliyor (2026-11-21) | [`docs/decisions/rounds/RP4.md`](docs/decisions/rounds/RP4.md) | Ö77–Ö93, A53–A56 |  |
| Önerilen (R5B) — Arkin'in onayını bekliyor (2026-11-22) | [`docs/decisions/rounds/R5B.md`](docs/decisions/rounds/R5B.md) | Ö94–Ö102, A57–A62 |  |
| Önerilen (RG1) — Arkin'in onayını bekliyor (2026-11-22) | [`docs/decisions/rounds/RG1.md`](docs/decisions/rounds/RG1.md) | Ö103–Ö108, A63–A64 |  |
| Önerilen (RF1) — Arkin'in onayını bekliyor (2026-11-25) | [`docs/decisions/rounds/RF1.md`](docs/decisions/rounds/RF1.md) | Ö119–Ö126, A70–A73 |  |
| Önerilen (RL1) — Arkin'in onayını bekliyor (2026-11-26) | [`docs/decisions/rounds/RL1.md`](docs/decisions/rounds/RL1.md) | Ö127–Ö137, A74–A77 |  |

## Yeni satır nereye yazılır

- Verilmiş karar (K…): `docs/decisions/kararlar.md` tablosunun sonuna.
- Genel açık karar (A1–A9 türü): `docs/decisions/acik.md`.
- Bir round'un önerdiği satırlar (Ö…, A…): `docs/decisions/rounds/<round>.md` (yeni round için yeni dosya) ve bu dizine bir satır.
- Numaralar depo genelinde tektir ve hiç yeniden kullanılmaz.
- Not (RG1): özgün kayıtta Ö39, Ö40, Ö41 iki kez kullanılmış (R3 ve R4 bölümlerinde ayrı kararlar).
  Bölünmede değiştirilmedi; başvururken bölümü de anın (ör. "R4 Ö39").

Bölünmenin denetimi: `scripts/rg1-decisions-check.py <bölünmeden önceki commit>` (satır kümesi
eşitliği, dizin, depo içi başvurular).
