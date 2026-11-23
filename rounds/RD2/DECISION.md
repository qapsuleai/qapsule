# RD2 — Arkin'in kararları

## 1. Ara karar (K1) — 2026-11-22

Sunulan: üç yön (A `sakel`, B `tavek`, C `ravek`), dört çapa ekranı, iki tema, sentetik ve
gerçek veriyle (GARAN, günlük) karşılaştırma sayfası; `oltan` (R5) ve başvuru ekranları
(OpenChart, OpenTerminalUI) yan yana. Oturumun önerisi en sonda ayrı başlık altındaydı
(A tabanı + C sicili + B köken çizgisi).

**Arkin'in sözleri (aynen):**

> En mantıklısı tavek gibi duruyor. ama basvurulardaki openchart'ın inceleme ve akış şemaları
> hala daha iyi gozukuyor. yani orneklendirmelerimizde halen ciddi eksikler var. geliştirmelerinin
> ileriki aşamalarında openchart ve openterminal tarafındaki kodlar üzerinden de inceleme ve
> uyarlama dah aiyi olabilir.

**Oturumun okuması (Arkin'in sözlerinden; karar değil, uygulama):**

- Seçilen yön **B — `tavek`** (`tavek`, `tavek-tel`). Çekirdeğe bu işlenir (C1–C3).
- "İnceleme ve akış şemaları" OpenChart'ın ajan paneli (katlanan çalışma, cevap, soru kartı,
  araç satırları) ve grafikteki açıklama akışı (haplar, öncü çizgiler) olarak okundu. Bu round'da
  maket ve şartname işi (M1–M3) bu yüzeylerin yoğunluğunu ve ayrıntısını başvuru düzeyine
  yaklaştırmayı hedefler; eksik kalan raporda adıyla yazılır.
- "Kodlar üzerinden inceleme ve uyarlama" sonraki round'lar için öneri olarak raporda
  ("Öneriler", "Arkin'e sorular"): K83 bugün kod kopyalamayı yasaklıyor; MIT/Apache projelerde
  kod düzeyinde inceleme ve uyarlamanın sınırı yeni bir karar ister.

## 2. Son kabul (K2) — 2026-11-23

Sunulan: `tavek` / `tavek-tel` çekirdekte (görünüş kaydı, ortak grafik alfabesi, köken çizgileri ve
seviye etiketleri, eşlik eden yüz `zemre`), sözleşme ve `design-check` (P1–P13, H1–H3 geçiyor),
Y01–Y37 şartnamesi, 30 yüzey maketi, proje becerisi, insan testi v4 paketi, E1 ölçümleri (gri
düzeltmesi sonrası bütün öğelerde eşik altı 0), önce/sonra görseli, bilinen eksikler listesi.
Soru: "profesyonel ve güvenilir" bulup kabul ediyor musunuz; kabulde gösterim varsayılanı
`oltan` → `tavek` olur.

**Arkin'in cevabı (aynen):** "Kabul, varsayılanı değiştir"

Uygulama: gösterim varsayılanı `tavek` (telefon profili `tavek-tel`); protokol varsayılanı `r4`
ve var olan altınlar değişmez. İnsan testi v4'ün öbür katılımcılarının sonuçları sonradan
`human-test/RESULTS.md`'ye eklenir.
