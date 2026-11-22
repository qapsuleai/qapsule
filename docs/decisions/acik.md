# Karar Günlüğü — Açık kararlar

<!-- dizin: DECISIONS.md; bu dosyadaki satırlar RG1 bölünmesinde aynen taşındı -->

## Açık kararlar

| # | Konu | Ne zaman | Not |
|---|------|----------|-----|
| A1 | Çizim yığını — **KAPANDI** | R0 | K12'ye taşındı; kanıt `crates/qp-render/SPIKE.md`. |
| A2 | Qapsule adı için marka ön taraması (TÜRKPATENT, EUIPO, USPTO) ve `qapsule.com` alımı | R1 öncesi | Alan adı 2026-11-14 itibarıyla Afternic'te satılık, hemen al fiyatı 4.995 USD. Önce tarama, sonra alım. |
| A3 | Ajanın yazacağı hesap/görünüm dili (kendi küçük dilimiz mi, sandbox'ta WASM mı) | R5 öncesi | Pine uyumluluğu hedef değil. |
| A4 | Yatırım tavsiyesi sınırı — **KAPANDI** | — | K32. |
| A5 | Ajan çalışma ortamı mevcut QODI/MCP altyapısı üzerine mi kurulacak | R5 öncesi | Qapsule kapsamı dışında ama arayüzü etkiler. |
| A6 | Mobil kabuk yaklaşımı (yerel bağlama mı, gömülü web görünümü mü) | R4 sonrası | |
| A7 | x86'da geniş SIMD için `unsafe` yasağına istisna | Gerçek yük görülünce (R3 sonrası) | Öneri: şimdilik hayır. Hedefler istisnasız tutuyor; istisna denetim yükü getirir. |
| A8 | Tamsayı fiyat adımı (kuruş) toplamı: SMA/Bollinger'de daha doğru ve 1,4–2,4× hızlı, ama R1 tanımını değiştirir | RD1 / R3 öncesi | Enstrümana fiyat adımı alanı gerekir. Düzeltilmiş (bölünme/temettü) fiyatlar adımın katı değildir; bu yüzden seçmeli bir yol olmalı. |
| A9 | Önbellek bellek politikası (bütçe, tahliye, canlı seride geçersizleşme) | R3 | Öneri: deneysel kalsın; çalışma ortamı kararı. |
