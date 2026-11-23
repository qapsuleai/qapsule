# Ana akış devir notu — yeni oturum buradan başlar

Ana akış (round'ları tasarlayan, raporları okuyan, Arkin ile karar veren oturum) bir sohbettir
ve sohbetler uzar, değişir. **Doğruluk kaynağı sohbet değil, bu depodaki dosyalardır.** Yeni
bir ana akış oturumu (yeni sohbet, başka bir araç) şu sırayla okur:

1. `NORTH_STAR.md` — değişmeyen çekirdek.
2. `STATUS.md` — şu an neredeyiz, sıradaki adım.
3. `ROADMAP.md` — round sırası.
4. `DECISIONS.md` — özellikle K69–K83 (2026-11-22 hedef düzeltmesi) ve "Önerilen" bölümleri.
5. `docs/ENGINEERING.md` — geliştirme kuralları.
6. `docs/research/README.md` — araştırma kanalının dizini; ayrıntı gerektiğinde `../refs`.
7. Son biten round'un `rounds/<ad>/REPORT.md` ve `AUDIT.md` dosyaları.

## Ana akışın işi

- Round'u tasarlar: `rounds/<ad>/` altına `GOAL.md`, `TASKS.md`, `VERIFY.md`, `PROMPT.md`.
  Önceki round'un raporu okunmadan sonraki yazılmaz. Her `GOAL.md` bir **yapı ölçütü** taşır
  (`docs/ENGINEERING.md` §10).
- Arkin round'u ayrı bir Claude Code oturumunda `PROMPT.md` ile koşturur.
- Rapor gelince: okur, Arkin'e kısa ve dürüst özetler, önerileri karara bağlar, kararları
  `DECISIONS.md`'ye, durumu `STATUS.md` ve `ROADMAP.md`'ye yazar.
- Yazdığı belge değişiklikleri commit edilmeden durur; sıradaki round ilk iş olarak
  "ana akış: …" commit'iyle kaydeder (`CLAUDE.md` "Ana akış değişiklikleri").
- `NORTH_STAR.md` yalnız Arkin'in onayıyla ve onu anan bir karar satırıyla değişir.

## Çalışma biçimi (Arkin'in tercihleri, kararlarla kayıtlı)

- Emin olunmayan yerde alternatifleri kur ve ölç; ama **aynı ölçümü tekrar etme** (K82).
- Round'lar yalın: kesme sırası önceden yazılır; bitirmek genişletmekten önce gelir.
- Her round sonunda temizlik zorunlu (K68); yapı kuralları kabul ölçütüdür (K80).
- Gerektiğinde belirteç ve model kullanımı kaydedilir; ürün için hangi iş hangi model boyuyla.
- Dışarıdan uyarlama iki kademeli (K91): MIT/Apache projelerde kod uyarlanabilir (kart + atıf); öbürlerinden yalnız fikir.
- Arkin'e sorulacak sorular önerisiyle birlikte gelir; karar onundur.

## Tek ana akış

2026-11-22'den itibaren tek ana akış vardır (K69). Eski oturumlar round tasarlamaz. Bir oturum
bu dosyaları okumadan round tasarlıyorsa durdurulur.
