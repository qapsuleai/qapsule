# Round Protokolü

Her round bir klasördür:

| Dosya | Kim yazar | İçerik |
|-------|-----------|--------|
| `GOAL.md` | Ana akış | Hedef, kapsam, kapsam dışı, kabul ölçütleri |
| `TASKS.md` | Ana akış | Alt görevler, bağımlılıklar, hangi alt ajan nereye yazar |
| `VERIFY.md` | Ana akış | Çalıştırılacak komutlar ve beklenen sonuçlar |
| `PROMPT.md` | Ana akış | Claude Code'a verilecek goal metni |
| `REPORT.md` | Claude Code | Sonuç, ölçümler, sapmalar, açık sorular, öneriler |

## Döngü

1. Ana akışta round tasarlanır, dört dosya yazılır.
2. Arkin `PROMPT.md` içeriğini Claude Code'a goal olarak verir.
3. Claude Code alt ajanlarla işi yapar, denetçi alt ajan doğrular, `REPORT.md` yazar.
4. Arkin ana akışa döner; rapor okunur, kararlar güncellenir, sonraki round tasarlanır.
