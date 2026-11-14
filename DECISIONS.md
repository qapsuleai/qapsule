# Karar Günlüğü

## Verilmiş kararlar

| # | Tarih | Karar | Gerekçe |
|---|-------|-------|---------|
| K1 | 2026-11-14 | Kuzey yıldızı `NORTH_STAR.md` içindeki iki cümledir. | Arkin onayı. |
| K2 | 2026-11-14 | AI-ready araç değil, AI-native sistem. Varsayılan yazar ajan. | TradingView benzeri "ben söyleyeyim AI çizsin" modeli farklılaştırmıyor. |
| K3 | 2026-11-14 | Tüm hesaplar LLM'siz ve deterministik. | Model sayı konusunda güvenilmez; ispat gerekir. |
| K4 | 2026-11-14 | Çekirdekte hızlı, istek anında hesap kabiliyeti vardır. | Sunucu tüm sembol × periyot × indikatör × parametre uzayını önceden hesaplayamaz. |
| K5 | 2026-11-14 | Grafik ve ortak hesap/yayın katmanları ayrıdır; mevcut C++ indicator-engine ve anomaly-engine ana yapı kurulduktan sonra incelenir (R6). | Hedef dağılmasın. |
| K6 | 2026-11-14 | En küçük birim kapsüldür; profesyonel grafik onun tam açılmış halidir. | İki kullanıcı kitlesi tek motorla karşılanır. |
| K7 | 2026-11-14 | Anlam işaretleri için özgün bir görsel alfabe tasarlanır. | Küçültülmüş mum grafiği anlam taşımaz. |
| K8 | 2026-11-14 | Mimari sıfırdan kurulur; eski prototipten parçalar taşınır. | Eski yapıda çizim, etkileşim ve veri TS'te; doküman modeli yok. |
| K9 | 2026-11-14 | Önce kapsül, sonra profesyonel çalışma alanı. | Çekirdeği en az arayüzle sınar; boş alanda başlar. |
| K10 | 2026-11-14 | Geliştirme Claude Code'da round'lar halinde yürür; ana akış ve round tasarımı ayrı oturumda takip edilir. Süreklilik bu repodaki dosyalardadır. | Oturumlar arası hafıza yok. |
| K11 | 2026-11-14 | Proje adı **Qapsule**; paket öneki `qp-`, komut satırı `qp`. Birim adı dokümanda "kapsül", kodda `Capsule`. | Q ailesine uyumlu, uluslararası kullanıma uygun, icadın kendisini adlandırıyor. Marka ön taraması ve alan adı alımı bekliyor; sonuç olumsuzsa ad değişir. |

## Açık kararlar

| # | Konu | Ne zaman | Not |
|---|------|----------|-----|
| A1 | Çizim yığını (GPU kütüphanesi, referans yazılım çizici, WebGPU + yedek) | R0 denemesi | Karar kanıtla verilir, varsayımla değil. |
| A2 | Qapsule adı için marka ön taraması (TÜRKPATENT, EUIPO, USPTO) ve `qapsule.com` alımı | R1 öncesi | Alan adı 2026-11-14 itibarıyla Afternic'te satılık, hemen al fiyatı 4.995 USD. Önce tarama, sonra alım. |
| A3 | Ajanın yazacağı hesap/görünüm dili (kendi küçük dilimiz mi, sandbox'ta WASM mı) | R5 öncesi | Pine uyumluluğu hedef değil. |
| A4 | Yatırım tavsiyesi sınırı ve denetim izi gereksinimleri | R3 öncesi | Hukuk ekibiyle netleştirilecek. |
| A5 | Ajan çalışma ortamı mevcut QODI/MCP altyapısı üzerine mi kurulacak | R5 öncesi | Qapsule kapsamı dışında ama arayüzü etkiler. |
| A6 | Mobil kabuk yaklaşımı (yerel bağlama mı, gömülü web görünümü mü) | R4 sonrası | |
