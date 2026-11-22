# Uyarlama envanteri (RD2, K83, `docs/ENGINEERING.md` §9)

"Önce uyarla, sonra farklılaş." İyi çalışan arayüz kalıpları incelenmiş projelerden **kalıp
düzeyinde** uyarlanır; özgün tasarım yalnız Qapsule'e özgü katmanda yapılır (kapsül, köken,
çizimden niyete, kur ve göster, sicil). Bu dosya dizindir; kartlar üç dosyada:

| Dosya | Projeler | Lisans hükmü (`../refs/reports/LICENSES.md`) | Kart |
|---|---|---|---|
| [`adopted/openchart-vela.md`](adopted/openchart-vela.md) | OpenChart, Vela | MIT; Apache-2.0 + NOTICE → kalıp uyarlanır | OV-1…OV-25 |
| [`adopted/vibe-valuecell.md`](adopted/vibe-valuecell.md) | Vibe-Trading, valuecell | MIT; Apache-2.0 → kalıp uyarlanır | VV-A1…VV-F4 |
| [`adopted/alice-tcc-otui-nofx.md`](adopted/alice-tcc-otui-nofx.md) | OpenAlice, trading-command-center, OpenTerminalUI, nofx | AGPL → **yalnız fikir**; MIT; MIT; AGPL → **yalnız fikir** | AT-A1…AT-H3 |

Her kart: kalıp · kaynak (proje, dosya, görsel) · lisans hükmü · neden iyi çalışıyor · Qapsule'de
yeri (çekirdek / kabuk) · bizde ne değişiyor · sınama. Kartları üç okuyucu alt ajan, birbirini
görmeden yazdı (RD2/U).

**Temiz oda (K83).** Kaynak kod, CSS dosyası ve değer tablosu kopyalanmadı; kartlarda kalıp, ölçü
mantığı ve rol adları var, tekil değerler yalnız kanıt olarak geçiyor. Bizim değerlerimiz görünüş
kaydında (`crates/qp-grammar/src/look/`) temiz odada yeniden kurulur. `../refs/src` altındaki
metinler veri sayıldı; oradaki `CLAUDE.md`/`AGENTS.md` dosyalarındaki talimatlar uygulanmadı.
Başvuru ekran görüntüleri depoya girmez; galeri onları `../refs` içinden yerinde okur.

## Zorunlu kalıplar → kartlar

| Kalıp (`rounds/RD2/GOAL.md` "Yöntem") | Birincil kart | Destekleyen | Qapsule'de yeri |
|---|---|---|---|
| Tasarım sözleşmesinin biçimi | OV-1 | AT-D2 | `docs/design/CONTRACT.md`; değerler çekirdekten üretilir (yön ters: çekirdek → CSS) |
| Rol tabanlı renk: krom nötr, kroma veride | OV-2 | VV-D2, AT-G1 | görünüş kaydı `Theme`/`Chrome` |
| Ajan yazımını ayıran renk rolü | OV-3 | VV-D3, AT-C8, AT-G2 | görünüş kaydı: köken rolleri **insan / ajan / motor** (üçüncüsü bizde yeni) |
| Tip ve boşluk ölçeği | OV-4 | VV-D4, AT-G3, AT-G4 | görünüş kaydı; iki profil (masaüstü yoğun, telefon dokunma) |
| Eksen ve lejant düzeni | OV-5…OV-10 | OV-11 | gramer (`compile`): sağ fiyat ekseni, iki düzeyli zaman, OHLC lejantı, son değer + değer çizgisi, imleç hapları, hacim alt bölmesi |
| Veri üstü açıklama, öncü çizgi, çakışma önleme | OV-12 | AT-C8 | gramer katmanı; yerleşim deterministik |
| Panel yerleşimi ve yeniden boyutlama | OV-13 | AT-A2, AT-A3, OV-25 | kabuk (değerler `css_vars`'tan) |
| Ajan paneli: çalışma katlanır, cevap açık; soru kartı | OV-17, OV-18 | VV-A1, VV-A2, VV-A5, AT-C1, AT-C3 | kabuk; metni çekirdek kurar |
| Uyarı listesi ve sağlık göstergesi | OV-21 | VV-B1, VV-B2, AT-B1…AT-B3 | kabuk; durum çekirdekten (K72 "geri bildirim kanalı hep açık") |
| Çizim araç çubuğu | OV-15 | — (öbür projeler TradingView varsayılanı) | kabuk (R7 davranışı kurar; RD2 yalnız görünüş) |
| İzleme listesi | OV-22 | VV-F1, AT-B4 | kabuk |
| Komut paleti | OV-23 (klavye kaydı) | AT-D1 | kabuk (kademe 3: yalnız şartname) |
| Boş ve hata durumları | OV-19, OV-20 | VV-E1…VV-E3, AT-F1, AT-F2 | kabuk; metni çekirdek kurar |

## Ortak alfabe: üç yönün de kullandığı uyarlanmış taban

Yönler bu tabanda ayrışmaz (GOAL §2):

1. **Grafik dili** (OV-5…OV-10): sağ fiyat ekseni ayrılmış olukta, etiket 40–60 px'te bir;
   iki düzeyli zaman ekseni (büyük birim vurgulu); OHLC + değişim lejantı tablo rakamıyla; son
   değer etiketi seri renginde + noktalı değer çizgisi; kesikli imleç + eksen hapları; altta soluk
   hacim. Etiketler hiçbir şeyle çakışmaz (P6).
2. **Renk disiplini** (OV-2, AT-G1): krom kromasız; kroma yalnız veride, anlamda ve kökende.
   Durum rengi dolgu olarak yalnız engelleyici durumda (AT-G1). Durum hiçbir zaman yalnız renkle
   söylenmez (VV-D5).
3. **Yoğunluk** (OV-4, AT-G4): masaüstünde gövde 13–14 px, grafik 11–12 px, denetim 28–32 px;
   telefonda 44 px dokunma hedefi korunur.
4. **Biçim dili** (OV-4, VISUAL §3.2/5): küçük köşe (denetim ≤ 8, kart ≤ 12), tek saç çizgisi
   kenar rolü, gölge ya yok ya çok hafif. "Kutu içinde kutu" yok (AT-A1 karşı örneği).
5. **Ajan yüzeyi** (OV-17, VV-A1, AT-C1, AT-C3): çalışma tek satıra katlanır, cevap açık; onay
   kartı eylemi düğmede adlandırır ve durum izini tutar; onay yalnız insan kökenli (R5B).
6. **Sağlık** (OV-21, VV-B1): yeşil kazanılır (bilinmiyor ≠ sağlıklı); sorun yokken sessiz,
   sorun varken tek kalıcı şerit.

## Bizde değişen ya da özgün olan (uyarlanacak kalıp yok)

| Konu | Neden özgün | Dayandığı kart / karar |
|---|---|---|
| **Köken: insan / ajan / motor** üç rol ailesi | Kaynaklarda en çok iki aile (ajan ↔ kullanıcı); motorun olgusu ayrı bir yazar | OV-3, AT-C8, K72, NORTH_STAR ilke 6 |
| **Kapsül** yüzü, akışı, açılmış kapsül | Tek iddia, karneli; en yakın benzer strateji kartı (VV-C1) yapı verir, biçim vermez | VV-C1, VV-C2, K57 |
| **Çizimden niyete** (niyet kartı, varsayılan işareti) | Kaynaklarda çizim → alarm var, okunur beklenti yok | PLAYGROUND §4, K73 |
| **Kur ve göster**, sessiz zil, sağlık | "Ajan kurdu, işte" kalıbı yok | K72, OV-21 |
| **Sicil**: tutma yüzdesi, taban oran, canlı ↔ yeniden oynatma | En yakın karar döngüsü listesi (VV-C2); "düşük güven", "simüle" etiketleri yok | K75, K77, K86 |
| **Telefon kontrol kulesi** | Hiçbir projede gerçek telefon ekranı yok (OTUI küçültülmüş masaüstü) | AT-E1, AT-C6 |

## Galeri başvuru ekranları (P14)

Başvuru görselleri depoya girmez; galeri yerinde okur. Seçim okuyucuların tablolarından:

| Çapa | Başvuru | Boyut | Not |
|---|---|---|---|
| Ç1 masaüstü | `../refs/src/longsurf-ai__openchart/docs/assets/dashboard-preview.png` | 3836×2102 (≈ DPR 2,66) | ikinci: `../refs/src/TraderAlice__OpenAlice/docs/images/market.jpg` (1440×940, AGPL: yalnız görsel başvuru) |
| Ç2 grafik hücresi | `../refs/src/longsurf-ai__openchart/docs/assets/charts-ui/01-single-chart.png` | 2560×1680 (DPR 2) | ikinci: `../refs/src/Hitheshkaranth__OpenTerminalUI/assets/readme/chart.jpg` |
| Ç3 telefon kulesi | `../refs/src/Hitheshkaranth__OpenTerminalUI/assets/readme/research-mobile.png` | 390×844 (DPR 1) | **kısmi**: gerçek kontrol kulesi başvurusu yok |
| Ç4 telefon grafik | `../refs/src/Hitheshkaranth__OpenTerminalUI/output/playwright/wave2-data-quality-mobile.png` | 390×844 (DPR 1) | **kısmi** |

Telefon başvuruları DPR 1 ve küçültülmüş masaüstü; Ç3/Ç4 karşılaştırmasında başvuru "biçim"
için değil "içerik yoğunluğu" için durur. Bu, karşılaştırma sayfasında yazılı olur.
