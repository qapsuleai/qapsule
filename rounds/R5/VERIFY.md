# R5 — Doğrulama

| # | Komut / kontrol | Beklenen |
|---|-----------------|----------|
| 1 | `QP_VERIFY_BROWSER=1 scripts/verify.sh` | Çıkış kodu 0 |
| 2 | `scripts/preflight.sh`; CI son çalışma | Yeşil |
| 3 | `git log` sırası: düzenek ve puanlama kuralı commit'leri alternatif dallarından önce | Önce |
| 4 | Bozuk tasarım sınaması | Düzenek bozuk tasarımı son sıraya koyuyor |
| 5 | Yasak belirsiz ifade, tekrar ve iç terim taraması; yedi dedektör × aşamalar × iki dil | 0 bulgu; tablo eksiksiz |
| 6 | Karşıtlık hesabı, seçilen dilin bütün öğeleri, açık ve koyu | Metin ≥ 4,5:1 (büyük metin ≥ 3:1), metin dışı ≥ 3:1; kapanmış işaret dahil |
| 7 | Anlam renkleri gri tonlamada ve üç renk körlüğü benzetiminde | Ayrım eşiğin üstünde; biçimle de okunuyor |
| 8 | `?look=r4` ile `v1` altınlar | Bayt eşit |
| 9 | `v2` altınlar iki koşuda | Bayt eşit (deterministik) |
| 10 | Hikâye çizelgesi: 5 / 50 / 200 kapsül; odak değiştirme; kümeleme; kapanmışları gizle | Çalışıyor; `screen` ve nokta sorgusu aynı şeyi söylüyor |
| 11 | Hikâye listesi sorgusu, yetkili ve yetkisiz okuyucu | Yetkisize rakam ve veri özeti gitmiyor; girişimsizlik testleri geçiyor |
| 12 | Akış: gizleme eşiği geleceğe bakmıyor (zamanı ileri alınca geçmiş kararlar değişmiyor); gizlenenler karnede | Doğru |
| 13 | Kalibre benzetim (`PHONE`, JIT açık tahmini), seçilen tasarım, `canvas` | K52 bütçesi içinde; raporla uyumlu (±%20) |
| 14 | Paket boyutu, yazı tipleri dahil | Önce/sonra tablosuyla uyumlu; yazı tipi lisans dosyaları depoda |
| 15 | Sayfa dış ağa istek yapmıyor (galeri dahil) | 0 istek |
| 16 | Hareketi azalt ayarı | Bilgi kaybı yok |
| 17 | Mimari testi; denetçinin kendi taraması | Kabukta biçem ya da alan kararı yok |
| 18 | Galeri sayfası başsız tarayıcıda telefon boyutunda | Adaylar aynı içerikle; seçim özeti kopyalanıyor |
| 19 | `?paint=cpu` | Yol yok; `canvas`'a düşüyor ve bunu söylüyor |
| 20 | `scripts/clean.sh --dry-run`, sonra gerçek koşu | Depo dışına ve `fixtures/local/`, `.env.local`'a dokunmuyor; önce/sonra boyut raporda; `git worktree list` yalnız ana ağaç |
| 21 | Gizli bilgi taraması; `fixtures/local` depoda yok | Temiz |
