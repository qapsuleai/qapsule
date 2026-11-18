# Yol Haritası

Sıra, "önce boş olan yerde başla" ilkesine göredir: önce kapsül, sonra profesyonel
çalışma alanı. Her round'un çıktısı `qp` komut satırıyla ekransız çalıştırılabilir olmalıdır.

| Round | Hedef | Kabul ölçütü (özet) | Durum |
|-------|-------|---------------------|-------|
| R0 | Temel: repo, şema v0, doğrulama düzeneği, çizim denemesi, eski koddan envanter | Sabit veriden deterministik PNG; aynı kod tarayıcıda da çiziyor; şema gidiş-dönüş testleri yeşil | Tamamlandı (2026-11-14, `rounds/R0/REPORT.md`) |
| R1 | Hesap çekirdeği: akışkan, çok çıktılı, `f64`; seri kaynağı arayüzü; parametre taraması | Eski 5 indikatör yeni çekirdekte; 1.000 parametre varyasyonu hedef sürede | Tamamlandı (2026-11-14, `rounds/R1/REPORT.md`) |
| RD1 | Tasarım round'u: motor–arayüz–ajan alan protokolü. Araştırma, şartname v0, referans oturum, ajan yazabilirlik ölçümü | Protokol şartnamesi ve uygunluk dökümleri; çelişkili kararlar alternatifleriyle ölçülmüş; ajanlar yalnızca şartnameyi okuyarak protokolü kullanabiliyor | Tamamlandı (2026-11-15, `rounds/RD1/REPORT.md`; denetim koşullu kabul, koşullar giderildi) |
| R2 | Görsel gramer v0, protokolün kelime dağarcığı olarak: kimlikli katmanlar, anlamla başvuru, derinliğe göre indirgeme, ekransız çizim, anlamsal okuma | Altı görünüm yalnızca protokol mesajlarıyla kuruluyor; aynı bulgu üç derinlikte çiziliyor; ajanlar kimlik bilmeden gramer görevlerini yapabiliyor | Tamamlandı (2026-11-15, `rounds/R2/REPORT.md`; denetim koşullu kabul, koşullar giderildi, `rounds/R2/AUDIT.md`) |
| RP2 | Ara round: oturumun artımlı canlı yolu, veri bağı (K23), veri ve anlam düzlemlerinin tutarlılığı | Canlı tikte doküman kopyalanmıyor; doküman bar taşımıyor, bağ taşıyor; yansı ile otorite aynı veriye baktığını kanıtlayabiliyor | Tamamlandı (2026-11-15, `rounds/RP2/REPORT.md`; denetim koşullu kabul, koşullar giderildi, `rounds/RP2/AUDIT.md`) |
| RP1 | Ara araştırma round'u: hesap çekirdeği performansı. Bilinen yöntemlerin ötesinde yaklaşımlar denenir | Dizüstü sınıfı hedefleri CI koşucusunda da tutuyor; denenen her yaklaşım ölçümüyle kayıtlı; bit-eşitlik korunuyor | Tamamlandı (2026-11-14, `rounds/RP1/REPORT.md`) |
| R3 | Kapsül: olgu dedektörleri, altı anlam türü ve işaret alfabesi, hikâye olarak yaşam döngüsü, zamana göre dil, karne, sunucu olgusu, hassasiyet | Sentetik ve (varsa) gerçek geçmiş veride kapsüller doğuyor, evriliyor, sonuçlanıyor ve karneyi besliyor; işaret alfabesi ölçümle seçilmiş; insan testi paketi hazır | Tamamlandı (2026-11-16, `rounds/R3/REPORT.md`; denetim koşullu kabul, koşullar giderildi, son hüküm kabul, `rounds/R3/AUDIT.md`) |
| R4 | İlk kullanılabilir gösterim: telefon boyutunda web kabuğu, gerçek verinin yeniden oynatılması, kapsülün yüzünün yeniden tasarımı, anlam zoom'u etkileşimli | Arkin tek komutla açıp kapsül akışını izliyor, dokunarak kanıta iniyor; yüz tasarımı ölçümle seçilmiş; insan testi bu gösterim üzerinden yapılabiliyor | Tamamlandı (14 ✅ / 1 ⚠️: ayrıntı cümlesi R5'e; denetim koşullu kabul, koşullar karşılandı; telefon ölçümü ve insan testi v2 Arkin'de) |
| RP3 | Ara round: telefonda çizim ve açılış performansı. Çizim stratejileri alternatifleriyle, gerçek telefona kalibre edilmiş ölçümle | Üst sınıf telefonda kaydırma ve tik akıcı; ilk kapsül hızlı; karar Arkin'in telefon ölçümüyle doğrulanmış | Tamamlandı ve kabul edildi (2026-11-17, K51–K52: görüntüleyici + `canvas`; telefonda HTTPS ile tik + boyama ≈ 2 ms, uzun kare %0; `rounds/RP3/REPORT.md`) |
| R5 | Dil ve tasarım: kapsül dili (K49), görsel dil (yazı, renk, boşluk, hareket), hikâye çizelgesi (K50), akış önceliği (K53), kanıt ekranı. Yeni nesil arayüz araştırması; alternatifler kör üretilir, ajanlarla puanlanır, telefonda karşılaştırılır | Kapsül deneyimi tek bakışta anlaşılır ve kendini açıklar; her seçim ölçümle; K52 bütçesi tutuyor; Arkin finalistleri telefonda karşılaştırabiliyor; insan testi v3 paketi hazır | Tamamlandı (2026-11-18, `rounds/R5/REPORT.md`; seçimler ajan ön elemesi: `marla`, `oltan`, `pavka`, `rate55`, `tesir`, geçici `surek`; 7 ⚠️ akış sırası, 9 ⚠️ hareket ölçümü geçersiz; denetim `bağımsız denetim koşullu kabul (engelleyici yok; dört koşul karşılandı, `rounds/R5/AUDIT.md`)`; galeri karşılaştırması ve insan testi v3 Arkin'de) |
| R5B | Ajan yüzeyi: komut yolu, anlamsal okuma, MCP, karne (eski R5) | Ajan ekransız kapsül üretiyor; her öğe olguya izlenebiliyor | Planlandı |
| R6 | Canlı veri: Matriks/NATS bağlayıcı; indicator-engine ve anomaly-engine incelemesi | Canlı BIST akışında yaşayan kapsül | R4'ten sonra ya da paralel; girdileri bekleniyor (.proto, belirteç) |
| R7 | Profesyonel mercek: etkileşim, paneller, çizim nesneleri | Alışkın kullanıcı için tanıdık çalışma alanı | Planlandı |
| R8 | Mobil kontrol kulesi ve gömme | Kapsül akışı, onay/ret/ertele/derinleş; uygulama içine gömülü | Planlandı |
| R9 | Niyetler, kişiselleştirme, anlatım (metin/ses) | Kalıcı niyetten kişiye özel kapsül | Planlandı |

## Round kuralları

- Bir round, `rounds/Rn/` içindeki dört dosyayla tanımlanır: `GOAL.md`, `TASKS.md`,
  `VERIFY.md`, `PROMPT.md`. Bittiğinde `REPORT.md` yazılır.
- Sonraki round'un dosyaları, önceki round'un raporu okunmadan yazılmaz.
- Bir round kabul ölçütlerini geçmeden kapanmaz; geçemeyen kısım açıkça rapora yazılır.
