# R3 — Doğrulama

| # | Komut / kontrol | Beklenen |
|---|-----------------|----------|
| 1 | `QP_VERIFY_BROWSER=1 scripts/verify.sh` | Çıkış kodu 0 |
| 2 | `scripts/preflight.sh`; CI son çalışma | Yeşil |
| 3 | `qp conform` üç taşımada | Var olan 36 döküm ve yeniler geçer; değişen döküm varsa raporla aynı |
| 4 | Var olan altın görseller ve hesap özeti | Değişmedi (özet yalnızca yeni hesaplarla genişledi) |
| 5 | Dedektörler: toplu = akışkan; günlükten yeniden oynatma aynı kapsül geçmişi | Bit düzeyinde aynı |
| 6 | Denetçi her türden bir olguyu kendi betiğiyle veriden bağımsız hesaplar | Aynı bar, aynı değerler |
| 7 | K26: geçmişte olmuş olgu, güncel değer farklıyken | Cümle geçmiş zamanda ve "artık geçerli değil" bilgisiyle; iki dilde |
| 8 | İleriye bakma: denetçi gelecekteki veriyi karıştırır | Geçmiş karne değişmez |
| 9 | Aday: bar içinde oluşup kaybolan durum | Otorite günlüğünde ve karnede iz yok |
| 10 | Yetkisiz kullanıcı, lisanslı olgu | Yorum ve karne gelir; sayısal değer ve veri özeti gelmez |
| 11 | Denetçi kendi geri çıkarma saldırısını yazar (nitel yorum dizisinden değer aralığı) | Sonuç AUDIT'te; raporun iddiasıyla tutarlı |
| 12 | RP2 denetçi senaryoları AUD1–AUD3 | Yanlış gösterim 0; kaçırılan kesin iddia RP2'den az |
| 13 | Hassasiyet benzetimi yeniden çalıştırılır | Dağılımlar raporla aynı (belirleyici) |
| 14 | M1–M7 ham kayıtları | Düzenek commit'i alternatiflerden önce; alt ajanların birbirini görmediği kayıtlardan anlaşılır; yeniden puanlama aynı |
| 15 | Denetçi işaret altın görsellerini ve üç derinliği gözle inceler; aynı kapsülün üç derinliğinde bilginin tutarlı olduğunu kontrol eder | Bulgular AUDIT'te |
| 16 | Denetçi insan testi paketini yönergeyi izleyerek kendi üzerinde dener | Anlaşılır; eksikler AUDIT'te |
| 17 | `git ls-files \| grep fixtures/local` ve depoda gerçek veri izi | Yok |
| 18 | Gizli bilgi taraması ve eski klasörlerin değişmediği | Temiz |
