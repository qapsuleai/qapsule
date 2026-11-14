# Yol Haritası

Sıra, "önce boş olan yerde başla" ilkesine göredir: önce kapsül, sonra profesyonel
çalışma alanı. Her round'un çıktısı `qp` komut satırıyla ekransız çalıştırılabilir olmalıdır.

| Round | Hedef | Kabul ölçütü (özet) | Durum |
|-------|-------|---------------------|-------|
| R0 | Temel: repo, şema v0, doğrulama düzeneği, çizim denemesi, eski koddan envanter | Sabit veriden deterministik PNG; aynı kod tarayıcıda da çiziyor; şema gidiş-dönüş testleri yeşil | Hazır, koşulacak |
| R1 | Hesap çekirdeği: akışkan, çok çıktılı, `f64`; seri kaynağı arayüzü; parametre taraması | Eski 5 indikatör yeni çekirdekte; 1.000 parametre varyasyonu hedef sürede | Planlandı |
| R2 | Görsel gramer v0 ve headless çizim | Mum grafiği gramerle ifade ediliyor; altın görseller | Planlandı |
| R3 | Kapsül v0: şema, yaşam döngüsü, ilk olgu dedektörleri, anlam işaretleri v0 | Sahte akışta kapsül doğuyor, güncelleniyor, kendini geçersiz kılıyor | Planlandı |
| R4 | Anlam zoom'u ve web görüntüleyici | Kapsülden tam grafiğe kesintisiz açılma | Planlandı |
| R5 | Ajan yüzeyi: komut yolu, anlamsal okuma, MCP, karne | Ajan ekransız kapsül üretiyor; her öğe olguya izlenebiliyor | Planlandı |
| R6 | Canlı veri: Matriks/NATS bağlayıcı; indicator-engine ve anomaly-engine incelemesi | Canlı BIST akışında yaşayan kapsül | Planlandı |
| R7 | Profesyonel mercek: etkileşim, paneller, çizim nesneleri | Alışkın kullanıcı için tanıdık çalışma alanı | Planlandı |
| R8 | Mobil kontrol kulesi ve gömme | Kapsül akışı, onay/ret/ertele/derinleş; uygulama içine gömülü | Planlandı |
| R9 | Niyetler, kişiselleştirme, anlatım (metin/ses) | Kalıcı niyetten kişiye özel kapsül | Planlandı |

## Round kuralları

- Bir round, `rounds/Rn/` içindeki dört dosyayla tanımlanır: `GOAL.md`, `TASKS.md`,
  `VERIFY.md`, `PROMPT.md`. Bittiğinde `REPORT.md` yazılır.
- Sonraki round'un dosyaları, önceki round'un raporu okunmadan yazılmaz.
- Bir round kabul ölçütlerini geçmeden kapanmaz; geçemeyen kısım açıkça rapora yazılır.
