# R4 — Doğrulama

| # | Komut / kontrol | Beklenen |
|---|-----------------|----------|
| 1 | `QP_VERIFY_BROWSER=1 scripts/verify.sh` | Çıkış kodu 0 |
| 2 | `scripts/preflight.sh`; CI son çalışma | Yeşil |
| 3 | `qp conform` üç taşımada | Hepsi geçer; değişen dökümler raporla aynı |
| 4 | Var olan altın görseller ve hesap özeti | Değişmedi (K42 gereği değişenler listeli) |
| 5 | `scripts/demo.sh` temiz kopyada, yerel veri olmadan | Sentetikle açılır; ekranda belirtilir |
| 6 | Denetçi gösterimi başsız tarayıcıda bir kullanıcı gibi kullanır: akış, kapsüle dokunma, grafiğe inme, kaydırma, kanıt, seviye, dil, tema | Hepsi çalışır; ekran görüntüleri ve bulgular AUDIT'te |
| 7 | Yeniden oynatma iki kez, aynı hız ve girdilerle | Mesaj akışı ve sahne özetleri bayt düzeyinde aynı |
| 8 | Kabuksuz sürüş: aynı girdiler yalnızca protokol mesajlarıyla | Doküman ve sahne özetleri kabuklu koşuyla aynı |
| 9 | Mimari testi; ayrıca denetçinin kendi taraması (kabuk kodunda hesap, cümle, sıralama, yerleşim) | Alan mantığı yok |
| 10 | Sayfanın ağ istekleri (başsız tarayıcıda kayıt) | Yalnızca yerel adres |
| 11 | K26/K42: geçerli olmayan kapsül | Başlık ve işaret aynı şeyi söylüyor; iki dilde |
| 12 | Süre ifadesi: günlük ve 60 dakikalık periyotta, hafta sonu ve tatil aşan örnek | Takvime göre doğru |
| 13 | Denetçi M8'den üç persona koşusunu kendi taze ajanlarıyla tekrarlar | Sıralama raporla tutarlı; farklar AUDIT'te |
| 14 | M8–M11 kayıtları | Düzenek ve ölçüt commit'i alternatiflerden önce; ajanlar tasarım adını görmemiş |
| 15 | Renk karşıtlığı ve "renksiz okunabilirlik" (gri tonlamalı ekran görüntüsünde yön ve geçerlilik) | Eşikler karşılanıyor |
| 16 | Ölçüm ekranı: masaüstü ve kısıtlanmış işlemci | Sayılar raporla uyumlu (±%20) |
| 17 | Bağlanma süresi, 30 enstrüman | < 100 ms ya da nedeniyle raporlanmış |
| 18 | Hassasiyet benzetimi (K42 sonrası) | Belirleyici; `important` seviyesinde doğumların pozisyon/niyet payı düşmüş |
| 19 | `git ls-files \| grep fixtures/local`; depodaki ekran görüntülerinde gerçek sembol/veri | Yok |
| 20 | Gizli bilgi taraması ve eski klasörlerin değişmediği | Temiz |
