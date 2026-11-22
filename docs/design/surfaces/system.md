# Yüzeyler — sistem (Y22, Y25, Y28, Y29, Y30, Y36, Y37)

Ortak kurallar ve kalıplar: [`../SURFACES.md`](../SURFACES.md).

### Y22 — Komut paleti ve sembol arama

**Ne:** Tek giriş: sembol, komut, ajana soru. · **Kademe:** 3 · **Kararlar:** K71.

**Yerleşim:** Üst çubukta "Sembol ya da komut ara ⌘K"; açılınca ortada yüzen liste (OV-23),
gruplar: Semboller · Komutlar (kısayolu sağda) · "Ajana sor". Sözdizimi `<SEMBOL> <İŞLEV>` (AT-D1).

**Durumlar**

| Durum | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|
| Boş | Son kullanılanlar | `--muted` | "Son kullanılanlar" |
| Yazıyor | Gruplu sonuçlar, eşleşen harf kalın | `--fg` | — |
| Sonuç yok | Tek satır + ajana yönlendirme | `--muted` | "‘xyz’ bulunamadı · ajana sor" |
| Belirsiz eşleşme | Birden çok sembol, borsa etiketiyle | `--fg` | "SYN20 · BIST" · "SYN20 · XETRA" |
| Ajana yönlendirildi | Son satır seçili, kesik kenar | `--qp-origin-agent` | "Ajana sor: ‘112,40 tutar mı’" |
| Ajan çalışıyor | Palet kapanır, ajan panelinde canlı satır | `--qp-origin-agent` | — |
| Yetki yok | Sembol satırında kilit | `--muted` | "veri yetkin yok" |

**Köken:** Komut insanındır; "Ajana sor" satırı kesik kenar. **Veri durumu:** Satırda yalnız yetki.
**Akış:** ⌘K → yaz → Enter → sembol etkin hücreye / komut çalışır / soru ajan paneline.
**Masaüstü / telefon:** Telefonda Kule başlığında arama alanı; komutlar yalnız gezinme.
**Varsayım (S25):** telefonda sonuçlar alanın altında tam genişlik (yüzen kart değil), komut grubu
"Gezinme" adını alır, kısayol sütunu yoktur; soru gönderilince arama kapanır ve Ajan sekmesi açılır.
Maket: `mockups/surfaces/system/Y22.html`.

**Uyarlanan:** OV-23, AT-D1. **Roller:** `--card`, `--qp-shadow`, `--fg`, `--muted`, `--qp-origin-agent`, `--accent`.

### Y25 — Veri ve piyasa durumu şeridi

**Ne:** Piyasa açık mı, veri canlı mı, gecikmeli mi, bayat mı; izlemeler değerlendiriliyor mu.
· **Kademe:** 1 · **Kararlar:** K72 (geri bildirim kanalı hep açık).

**Yerleşim:** Masaüstünde ekranın altında tek satır (AT-B1): sol "Veri kaynağı · durum", orta
"Seans açık 10:00–18:00 · Son bar 14:35:02", sağ izleme sağlığı ("5 izleme · son kontrol 14:35")
ve köken çizgi anahtarı ("İnsan, düz · Ajan, kesik · Motor, noktalı"). Telefonda üstte tek satır.

**Durumlar**

| Durum | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|
| Açık, canlı | Sakin şerit | `--muted` | "Canlı · seans açık · son bar 14:35:02" |
| Kapalı | Evre sözcüğü | `--muted` | "Seans kapalı · açılış 10:00" |
| Seans arası | Evre sözcüğü | `--muted` | "Seans arası · 13:00–14:00" |
| Gecikmeli | Sözcük | `--muted` | "Gecikmeli 15 dk" |
| Bayat | ◌ + sözcük; izleme sağlığı "gecikiyor" | `--qp-warn` | "Veri bayat · son veri 14:35 · 3 izleme değerlendirilemiyor" |
| Bağlantı koptu | Şerit tam renk (tek engelleyici durum) | `--alert-bg`/`--alert-fg` | "Bağlantı yok — veri bayat · son veri 14:35" |
| Yeniden bağlanıyor | Sayaçlı | `--alert-bg`/`--alert-fg` | "Yeniden bağlanıyor (3) · son veri 14:35" |
| Yetki yok | Kilit + lisans adı | `--muted` | "BIST seviye 1 yetkin yok · gecikmeli gösteriliyor" |
| Yeniden oynatma | Şerit başında bant | `--note-bg`/`--note-fg` | "Sentetik veri, oynatma · 22 Ağu 14:35" |
| İzleme sağlığı sorunlu | Sağ bölüm vurgulu (Y31) | sağlık rolü | "İzlemeler gecikiyor · son kontrol 25 dk önce" |

**Köken:** Şeridin kendisi motorun/barındırıcının sesi; köken anahtarı burada sürekli durur.

**Veri durumu:** Bu yüzey veri durumunun kaynağıdır (`MarketStatus`, `BindingState`); sözcükler CONCEPTS "Piyasa durumu" metinleriyle aynı.

**Akış:** Durum değişir → şerit sözcüğü değişir (hareketsiz, yalnız renk geçişi) → koptuysa tek
kalıcı şerit → iyileşince tek bildirim ("Bağlantı geri geldi · 14:35–14:41 arası yeniden çözüldü").

**Masaüstü / telefon:** Telefonda tek satır: en önemli tek durum (öncelik: koptu > bayat > yetki yok > gecikmeli > canlı) + saat.

**Uyarlanan:** AT-B1, AT-B2, VV-B1, OV-21. **Roller:** `--muted`, `--qp-warn`, `--alert-bg`, `--alert-fg`, `--note-bg`, `--note-fg`, `--qp-origin-*`.

### Y28 — Bildirim ve ajan tercihleri

**Ne:** Hassasiyet, sessiz saatler, kanallar, kişi başı izleme sınırı. · **Kademe:** 3 · **Kararlar:** K72, K78.

**Yerleşim:** Ayarlar > Bildirimler (Y15 ile aynı sayfa, ayrı bölüm); satır = ayar + değer + kaynak sözcüğü.

**Durumlar**

| Durum | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|
| Varsayılan | Değer + "varsayılan" | `--muted` | "Sessiz saatler 22:00–08:00 · varsayılan" |
| Değiştirildi | Değer + "senin seçimin" | `--fg` | "Hassasiyet: yalnız elindekiler · senin seçimin" |
| Kanal bağlı değil | Satır soluk + "Bağla" (S12) | `--muted` | "Telegram · bağlı değil" |
| Sistem izni yok | Uyarı + sistem ayarına bağ | `--qp-warn` | "Telefon bildirimleri kapalı · aç" |
| Sınır | Sayaç (Y37'ye bağ) | `--muted` | "İzleme 34 / 50" |

**Köken:** Ayarlar insanındır; kazanılmış varsayılan "önceki seçimlerinden" (Y37). **Veri durumu:** Uygulanmaz.
**Akış:** Satır → değer → anında yazılır. **Masaüstü / telefon:** Aynı içerik; telefonda alt sayfa seçiciler.

**Uyarlanan:** AT-F1. **Roller:** `--fg`, `--muted`, `--qp-warn`, `--control`.

### Y29 — Yan panel yuvası (veri görünümleri için yer)

**Ne:** Derinlik, işlem akışı, takas, haber için ayrılmış yer; ilk sürümde boş (**Varsayım (S7)**).
· **Kademe:** 3 · **Kararlar:** —.

**Yerleşim:** Masaüstünde sağ sütunda ajan panelinin yanında sekme ("Veri"); ajan paneliyle aynı genişlik, aynı anda biri açık.

**Durumlar**

| Durum | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|
| Yok | Sekme gizli | — | — |
| Yer ayrılmış | `O-boş` | `--muted` | "Derinlik ve takas burada görünecek" |
| Yetki yok | `D-yetkiyok` | `--muted` | "Derinlik için seviye 2 yetkisi gerekir" |

**Köken:** Veri görünümleri kökensiz (veri). **Veri durumu:** `D-*` başlıkta. **Akış:** Sekme → panel.
**Masaüstü / telefon:** Yalnız masaüstü.

**Uyarlanan:** AT-A2, OV-13. **Roller:** `--muted`, `--line`.

### Y30 — İlk kullanım ve boş çalışma alanı

**Ne:** İlk açılış: çizim, izleme, liste yokken ne görünür; ajan basamağı seçimi; (varsa) taşıma.
· **Kademe:** 3 · **Kararlar:** K72.

**Yerleşim:** Ç1 iskeleti boş halde: sol sütun `O-boş` satırları, orta tek soru ve arama alanı
(OV-19: "Nereden başlayalım?"), sağ panelde basamak seçimi kartı (Y15'in kısa hali).

**Durumlar**

| Durum | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|
| Hiç veri yok | Ortada soru + arama + üç önerilen başlangıç | `--fg`, `--muted` | "Nereden başlayalım? Bir sembol yaz ya da ajana sor." |
| Basamak seçimi | Dört seçenek, "Öner" önceden seçili (S14) | `--accent` | "Ajan ne kadar yapsın? · Öner (varsayılan)" |
| Yetki yok | `D-yetkiyok`; gecikmeli ya da sentetik veriyle devam | `--muted` | "Canlı veri yetkin yok · gecikmeli veriyle başla" |
| Örnek çalışma alanı | `D-simüle` bandı | `--note-bg`/`--note-fg` | "Örnek çalışma alanı · sentetik veri" |
| Taşıma sürüyor | İlerleme olay sayısıyla, yüzde uydurmadan (S15: ilk sürümde yok) | `--muted` | "12 / 40 liste taşındı" |

**Köken:** Örnek çalışma alanındaki çizimler "Örnek" etiketli insan çizimi biçiminde; ajan önerisi örneği kesik.
**Veri durumu:** Yukarıda. **Akış:** Aç → soru → sembol → grafik (Y01) → basamak → ilk çizim → niyet hapı (Y04).
**Masaüstü / telefon:** Telefonda kısa: tek cümle + arama + basamak (Y26 "boş").

**Uyarlanan:** OV-19, VV-E1, AT-F1. **Roller:** `--fg`, `--muted`, `--accent`, `--note-bg`, `--note-fg`.

### Y36 — Kurumsal işlemde "beklemede" izleme ve taşıma mesajı

**Ne:** Kurumsal işlem (bölünme, bedelsiz, temettü düzeltmesi) olayında: geçerlilik anından
yenileme bitene kadar o sembolün izlemeleri **beklemede**; fiyat boşluğu kırılım ya da stop diye
okunmaz; yenilenince tek mesaj; taşınamazsa sorulur. · **Kademe:** 2 · **Kararlar:** K88, K74.

**Yerleşim:** İzleme satırlarında `İ-beklemede`; grafikte seviyeler soluk, boşluk barında yalnız
küçük motor işareti ("Bölünme 1:2"); taşıma mesajı bildirim (Y12) + gelen kutusu; taşınamayan
izleme için soru kartı (Y05 biçimi).

**Durumlar**

| Durum | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|
| Duyuruldu (henüz geçerli değil) | Satırda bilgi | `--muted` | "SYN20 · bölünme 1:2, 23 Kas · izlemeler o gün bekletilecek" |
| Beklemede | `İ-beklemede`; değerlendirme durur | `--qp-warn` | "◷ Beklemede · kurumsal işlem (bölünme 1:2, 23 Kas)" |
| Yenileniyor | Satırda süreç sözcüğü | `--qp-warn` | "◷ Düzeltilmiş barlar alınıyor" |
| Taşındı | Tek mesaj: ne oldu, kaç izleme, eski → yeni düzeyler; izlemeler önceki durumlarına döner | `--fg` | "SYN20'de 1:2 bölünme: 3 izleme taşındı · 112,40 → 56,20 · 117,50 → 58,75" |
| Taşınamadı | Beklemede kalır + soru kartı | `--qp-warn` + `--qp-origin-agent` | "Bu izleme taşınamadı (bölge çizimi tarihi bölünmeden önce bitiyor). Ne yapayım?" — seçenekler: "Düzeyi 1:2 ile taşı ve izle" · "İzlemeyi kaldır" · "İzleme, yalnız not kalsın" |
| Sicil notu | Açık kayıtlar taşındı, katsayı yazıldı | `--muted` | "Düzeyler 1:2 ile taşındı" (Y33) |
| Grafik | Düzeltilmiş seri; çizim aynı düzeltmeyi alır (S10) | `--qp-origin-engine` işaret | "Bölünme 1:2" |

**Köken:** Olay ve taşıma motorun (noktalı işaret, "Motor" sözcüğü); taşınamayan izlemenin sorusunu
ajan sorar (kesik); karar insanın. Taşıma insanın çizimini değiştirir ama **deterministik ve
katsayıyla** (K88), çizim kökeni insan kalır, ayrıntıda "Motor taşıdı · 1:2" satırı.

**Veri durumu:** Beklemede durumu bayat/yetki durumundan ayrıdır; ikisi birden varsa önce beklemede yazılır.

**Akış:** Duyuru → geçerlilik anı → bütün izlemeler beklemede → düzeltilmiş barlar → çizimler ve
koşullar taşınır → hesaplar yeniden kurulur → tek mesaj → (taşınamayan için) soru → cevap.

**Masaüstü / telefon:** Telefonda mesaj kulede kart; eski → yeni düzeyler liste; soru kartı aynı.

**Uyarlanan:** AT-C3 (durum izi), OV-20. **Roller:** `--qp-warn`, `--fg`, `--muted`, `--qp-origin-engine`, `--qp-origin-agent`.

### Y37 — Kullanım hakları ve parametre kaynağı

**Ne:** Satılabilir haklar (soru adedi, yapay zekâ kullanımı, kontör, hesap bütçesi, izleme sınırı,
veri lisansları) ve her davranış değerinin kaynağı: "neden 3? → sistem varsayılanı".
· **Kademe:** 3 · **Kararlar:** K78, K72 (kazanılmış varsayılan).

**Yerleşim:** (a) Ayarlar > Haklar: satır = hak + kullanım + sınır + yenileme ("Bu ay 34 / 50
soru · 1 Ara'da yenilenir"); (b) **parametre kaynağı açılır kartı**: her yerde "varsayılan" sözcüğü
ya da "?" ona açılır; kapsam zinciri sistem → plan → kullanıcı → enstrüman → nesne, kazanan kapsam
vurgulu, değiştirme eylemleri ("Bu nesne için" / "Benim için").

**Durumlar**

| Durum | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|
| Sistem varsayılanı | Zincirde "Sistem" vurgulu | `--accent` (ince) | "Neden 3? → sistem varsayılanı (tutunma barı)" |
| Plan değeri | "Plan" vurgulu | `--accent` | "Neden 50? → planın: Profesyonel" |
| Kullanıcı / enstrüman / nesne | İlgili kapsam vurgulu + kim, ne zaman | `--fg` | "Neden 2? → bu izleme için senin seçimin, 12 Kas" |
| Kazanılmış varsayılan | Kaynak sözcüğü | `--fg` | "Neden kapanış? → önceki seçimlerinden (5 kez)" |
| Hak sınıra yakın | `O-sınır` | `--qp-warn` | "Bu ay 48 / 50 soru" |
| Hak doldu | Eylem devre dışı + neden + yükseltme bağı | `--qp-warn` | "Soru hakkın doldu · 1 Ara'da yenilenir" |
| Veri lisansı yok | Kilit + lisans adı | `--muted` | "BIST seviye 1 · yok" |
| Kilitli (kurum) | Değiştirme eylemi yok + neden | `--muted` | "Kurum kuralı: değiştirilemez" |

**Köken:** Parametre değerinin kökeni kapsamdır (sistem / plan / kullanıcı / enstrüman / nesne);
ajan bir değeri önerdiyse kesik "Ajan önerdi" satırı zincirin yanında.

**Veri durumu:** Lisans satırları `D-yetkiyok` kalıbını kullanır. **Akış:** "varsayılan" sözcüğüne
dokun → kaynak kartı → "Bu nesne için değiştir" → değer yazılır, kaynak "senin seçimin" olur.

**Masaüstü / telefon:** Aynı içerik; telefonda kaynak kartı alt sayfa.

**Uyarlanan:** VV-B3, AT-H2. **Roller:** `--accent`, `--fg`, `--muted`, `--qp-warn`, `--card`.
