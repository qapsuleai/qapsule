# Araştırma kanalı — dizin

2026-11-21/22'de Qapsule'ün yanında ayrı bir kanalda (`/Users/arkns/Works/Mchart/refs`, bu
depoya göre `../refs`) beş araştırma round'u (RX1–RX5) ve ana akış karar oturumları yürüdü.
Sonuçları Qapsule'ün hedefini düzeltti (K69–K83). Bu klasör o kanalın **karar taşıyan**
belgelerinin kopyasını ve geri kalanının dizinini tutar. `../refs` salt okunurdur.

## Bu klasördeki kopyalar

| Dosya | İçerik |
|---|---|
| `RX-DECISIONS.md` | Araştırma kanalının kararları X1–X35 (ayrıntılı; K69–K83'ün kaynağı) |
| `RX-PROPOSALS.md` | Qapsule için öneriler: kuzey yıldızı ekleri E1–E16, yol haritası Y1–Y31, aday deneyler |
| `SICIL.md` | Tek sicil tasarımı: iddia + sonuç; yeniden oynatma, tutma ihtimali, zaman aralığı, paylaşım |
| `KURUMSAL-ISLEMLER.md` | Bölünme ve öbür kurumsal işlemlerde izleme, çizim ve sicilin taşınması (K88; RS1 girdisi) |

## `../refs` içindeki kaynaklar (konuya göre)

| Konu | Belge |
|---|---|
| Dış projelerin incelemesi, puan tablosu, boşluk haritası | `reports/MATRIX.md`, `reports/QAPSULE-GAP.md`, `reports/PATTERNS.md`, `reports/TRANSPLANT.md`, `reports/repos/*.md`, `reports/LICENSES.md` |
| 2026+ derlemesi (trading, mühendislik, UI/UX) | `docs/rounds/RX2/OUTLOOK.md` |
| Görsel dil (RD2 girdisi) | `reports/VISUAL.md` |
| Profesyonelin oyun alanı (RD2 ve R7 girdisi): 113 kalem, 30 yüzey, 15 soru | `reports/PLAYGROUND.md` |
| Çizimden niyete deneyi, şema taslağı, eşleştirici | `reports/DRAWING-INTENT.md`, `experiments/rx3/` |
| İnsan testi (Arkin'in 20 cevabı) | `docs/rounds/RX3/HUMAN/RESULTS.md`, `FORM.md`, `answers.json` |
| Ajan sistemi sözleşmesi, çalışma modeli, maliyet | `reports/AGENT-SYSTEM.md`, `experiments/rx3/sim/` |
| Ajan yazımı hesap: ara gösterim, metin dil, şartname taslağı | `reports/COMPUTE-LANGUAGE.md` (§8), `experiments/rx4/SPEC.md` |
| Dil karşılaştırması (Tea, Pine, Qapsule) | `reports/LANGUAGE.md` |
| Sicil prototipi, yeniden oynatma, tutma ihtimali | `reports/ODDS.md`, `experiments/rx5/SPEC.md` |
| Zaman aralığı okuması | `reports/RANGE-READING.md` |
| Ortak analiz nesnesi ve maliyet | `reports/SHARED-ANALYSIS.md` |
| Sinyal paylaşımı düzenlemesi (hukuk görüşü değildir) | `reports/SIGNAL-SHARING-REGULATION.md` |
| Geliştirme kurallarının kaynağı | `docs/design/ENGINEERING-RULES.md` (bu depoda `docs/ENGINEERING.md`) |

## Kurallar

- Deney kodu (`../refs/experiments/`) atılabilir koddur ve bu depoya **kopyalanmaz**; round'lar
  onu şartname olarak alır ve `docs/ENGINEERING.md` kurallarıyla yeniden kurar.
- Dış projelerden (`../refs/src/`) kod alınmaz; lisans hükümleri `reports/LICENSES.md`.
- `../refs` şimdilik yalnız bu makinede; kalıcılığı için ayrı bir özel depo açılması önerilir.
