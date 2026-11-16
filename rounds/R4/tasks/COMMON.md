# R4 — Alt ajanlar için ortak kurallar

Bu dosyayı her alt ajan görevine başlamadan okur.

1. **Önce oku:** `CLAUDE.md`, `NORTH_STAR.md`, `docs/CONCEPTS.md` (özellikle "Gösterim (R4)"),
   `docs/ARCHITECTURE.md` (özellikle "Kabuk sınırı (R4)"), `STATUS.md`, `rounds/R4/GOAL.md`,
   `rounds/R4/TASKS.md`, `rounds/R4/VERIFY.md`. Görevinle ilgili `docs/PROTOCOL.md`,
   `docs/GRAMMAR.md`, `docs/CAPSULE.md` bölümlerini oku.
2. **Yalnız kendi worktree'nde ve dalında çalış.** Görev metninde verilen klasörlere yaz.
   Başka bir worktree'ye, `main`'e ya da `r4/int`'e yazma, birleştirme yapma, `git push` yapma.
   Küçük ve anlamlı commit'ler; ileti biçimi `R4/<görev>: …`, sonunda
   `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
3. **Değişmezler:** hesap kodunda LLM, ağ, rastgelelik yok; `f64`; şemanın tek kaynağı
   `qp-schema` (JSON Schema üretilir); dokümanı değiştiren her şey komut yolundan; yeni kavram
   önce `docs/CONCEPTS.md`'ye (sözlükte yoksa ekle ve raporda belirt).
4. **Var olan altın görseller, dökümler (`fixtures/protocol/transcripts/`), görünüm altınları ve
   hesap özeti değişmez.** Görevin gereği (K42) değişmesi gerekiyorsa: her değişen dosyayı
   gerekçesiyle `rounds/R4/experiments/<görev>.md`'de listele; eski dosyanın üzerine yazmak
   yerine mümkünse yeni dosya ekle. Yeni altın görsel yalnız yeni görünüm için eklenir.
5. **Bitmiş sayılma:** `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
   `cargo test --workspace` yeşil; görevle ilgili `scripts/verify.sh` adımları yeşil
   (tam `scripts/verify.sh` tercihen). Çıktı notu: `rounds/R4/experiments/<görev>.md` (ne
   yapıldı, kararlar, ölçümler, değişen dökümler/görseller ve gerekçesi, açık sorular,
   öneriler). Mimari seçimleri DECISIONS'a yazma; notunda "Önerilen karar" başlığıyla yaz,
   orkestratör derler.
6. **Gerçek veri:** `fixtures/local/` yalnız ana çalışma ağacında
   (`/Users/arkns/Works/Mchart/qapsule/fixtures/local/bars`); worktree'den
   `QAPSULE_LOCAL_BARS=/Users/arkns/Works/Mchart/qapsule/fixtures/local/bars` ile göster.
   Veri, türevleri, ondan alınan görüntüler hiçbir zaman commit edilmez; rapora yalnız toplu
   istatistik. `.env.local` açılmaz.
7. **Derleme klasörü:** worktree'nin kendi `target/`'ı. Disk sınırlı; gereksiz profiller
   derleme. İşin bitince `target/` kalabilir, orkestratör temizler.
8. **Emin olmadığın kütüphane davranışını varsayma;** belgesine bak, küçük denemeyle doğrula.
   Yeni bağımlılık eklemeden önce gerçekten gerekli mi düşün; eklersen notunda gerekçe yaz.
9. **Kapsam dışına çıkma:** fark ettiğin başka işleri notunda "Öneriler" altına yaz.
10. `../Docs`, `../Services` salt okunur. `gh` çağırma.
11. Son mesajında kısa özet ver: dal, son commit, testlerin durumu, notun yolu, açık sorunlar.
