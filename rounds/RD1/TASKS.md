# RD1 — Alt Görevler

Bu round'da sıra önemlidir: önce oku ve düşün, sonra tasarla, sonra kur, sonra ölç,
sonra tasarımı düzelt.

| # | Görev | Yazdığı yer | Bağımlılık |
|---|-------|-------------|------------|
| T1 | Araştırma: var olan protokoller (güncel belgelerinden) ve 2026–2030 varsayımları. Her biri ayrı alt ajana verilebilir; sonuçlar birleştirilir | `rounds/RD1/RESEARCH.md` | — |
| T2 | Şartname taslağı v0 ve yirmi doğal dil görevi. Görevler tasarımdan **önce** yazılır; tasarım onlarla sınanır | `docs/PROTOCOL.md`, `rounds/RD1/tasks/` | T1 |
| T3 | Mesaj tipleri, JSON Schema, örnekler, uygunluk dökümleri | `crates/qp-schema/` ya da `crates/qp-protocol/` (karar raporda), `fixtures/protocol/` | T2 |
| T4 | Referans oturum: komut, değişim, sorgu, abonelik, öneri, günlük, geri alma, yeniden oynatma | `crates/qp-session/` | T3 |
| T5 | Taşımalar: süreç içi ve `qp serve`; döküm koşucusu | `crates/qp-session/`, `crates/qp-cli/` | T4 |
| T6a–e | Çelişkili kararların ölçümü; her biri ayrı dalda, ayrı alt ajanla, birbirinin sonucunu görmeden | `rounds/RD1/experiments/`, deneme kodu dal başına | T4 |
| T7 | Ajan yazabilirlik ölçümü: taze alt ajanlar, koşucu, sonuç tablosu; tasarım düzeltmesi; yeniden ölçüm | `rounds/RD1/ergonomics/` | T5 |
| T8 | Yetenek uzlaşması ve anlatım çizelgesi | `crates/qp-session/`, `docs/PROTOCOL.md` | T4 |
| T9 | MCP uyarlayıcısı; A2UI ve MCP Apps analizi | `crates/qp-mcp/` ya da `qp-cli` alt komutu, `rounds/RD1/EMBEDDING.md` | T5 |
| T10 | Şartnamenin son hali; `R2_INPUT.md`; sözlük ve mimari güncellemeleri | `docs/`, `rounds/RD1/` | hepsi |
| T11 | Denetim: bağımsız alt ajan. Dökümleri temiz kopyada çalıştırır; yalnızca şartnameyi okuyarak üç yeni görev dener; ölçümleri yeniden üretir | `rounds/RD1/AUDIT.md` | hepsi |

## Notlar

- **Ajan yazabilirlik ölçümü bu round'un özgün kısmıdır.** Yapay zekâ için tasarlanan bir
  protokolü yapay zekâ sınar. Ölçümdeki alt ajanlar uygulamayı, testleri ve bu klasördeki
  notları görmemeli; yalnızca şartname ve şema verilir. Alt ajan başlatılamıyorsa bu ölçüm
  yapılamaz: dur, `REPORT.md`'nin başına yaz ve round'un geri kalanını bitirip Arkin'e bildir.
  Ölçümü kendi kendine yapıp alt ajan yapmış gibi raporlama.
- **Alternatifler gerçek olmalı.** T6'daki her alternatif, savunucusu tarafından yazılmış
  gibi en iyi haliyle kurulur. Zayıf kurulmuş bir alternatifi yenmek kanıt değildir.
- **Bilinen yöntemlerle sınırlı değilsin (K18).** Var olan protokoller başlangıç noktasıdır.
  Finansal anlamın, canlı verinin, kökenin ve çok yüzeyli sunumun ihtiyaçları onlarda
  karşılanmıyorsa yenisini tasarla ve gerekçesini yaz.
- **Belirleyicilik:** aynı günlük her zaman aynı dokümanı üretir. Oturumda saat ve
  rastgelelik yoktur; zaman mesajla gelir.
- **Güvenlik:** dış kaynaktan gelen her mesaj güvenilmez girdidir. Boyut sınırları, yol
  hapsi (R1'deki bulgu) ve yetki denetimi şartnamede ve referans oturumda yer alır.
- **Var olan kod:** `qp-compute` ve `qp-render` oturumdan çağrılır; değiştirilmez. Altın
  görseller güncellenmez.
- **GitHub:** `CLAUDE.md` kuralları geçerli (`scripts/qgh`, `scripts/preflight.sh`).
