# Yüzey şartnamesi (RD2 M1)

R7, RS1, RA1 ve R8'in kuracağı yüzeylerin **görünüşü ve durumları**. Davranışı bu round kurmaz
(`rounds/RD2/GOAL.md` §4). Kaynak: `../refs/reports/PLAYGROUND.md` §5 (Y01–Y30), GOAL §4 tablosu
(Y31–Y37), seçilen yön `tavek` (`mockups/tavek/DIRECTION.md`, Arkin'in ara kararı
`rounds/RD2/DECISION.md`), sözleşme `CONTRACT.md`, uyarlama kartları `ADOPTED.md`.

Bu dosya dizin ve ortak kurallardır; yüzeyler altı dosyada:

| Dosya | Yüzeyler |
|---|---|
| [`surfaces/chart.md`](surfaces/chart.md) | Y01, Y02, Y03, Y09, Y10, Y11, Y16, Y17, Y18, Y19, Y23 |
| [`surfaces/agent.md`](surfaces/agent.md) | Y04, Y05, Y06, Y14, Y15, Y24 |
| [`surfaces/watch.md`](surfaces/watch.md) | Y07, Y08, Y12, Y13, Y20, Y21, Y31, Y32 |
| [`surfaces/ledger.md`](surfaces/ledger.md) | Y33, Y34, Y35 |
| [`surfaces/phone.md`](surfaces/phone.md) | Y26, Y27 |
| [`surfaces/system.md`](surfaces/system.md) | Y22, Y25, Y28, Y29, Y30, Y36, Y37 |

## Kademe dizini (GOAL §4, K82)

| Kademe | Ne teslim edilir | Yüzeyler |
|---|---|---|
| 1 | Tam maket, bütün durumlar, iki profil | Y01, Y04, Y05, Y06, Y07, Y09, Y11, Y12, Y18, Y25, Y26, Y27, Y31, Y32, Y33 |
| 2 | Ana durumun maketi + durum listesi | Y02, Y03, Y08, Y10, Y13, Y14, Y16, Y17, Y20, Y21, Y23, Y24, Y34, Y35, Y36 |
| 3 | Yalnız şartname | Y15, Y19, Y22, Y28, Y29, Y30, Y37 |

## Okuma kuralları

- **Her yüzeyin biçimi:** Ne · Kademe · Yerleşim · Durumlar (tablo) · Köken · Veri durumu · Akış ·
  Masaüstü / telefon · Uyarlanan kartlar · Roller. Bir yüzey ortak kalıbı aynen kullanıyorsa
  tabloda kalıbın adını (`D-bayat`, `İ-kırıldı` …) yazar, tekrar etmez.
- **Roller adla:** yalnız `css_vars` adları (`qp look css tavek`); renk kodu, piksel değeri yok
  (CONTRACT "Yapılmayacaklar"). Krom: `--bg`, `--card`, `--fg`, `--muted`, `--line`, `--control`,
  `--page`, `--accent` (yalnız seçim ve odak), `--on-accent`, `--note-bg`/`--note-fg`,
  `--alert-bg`/`--alert-fg`. Anlam: `--qp-up`, `--qp-down`, `--qp-warn`, `--qp-valid`,
  `--qp-invalid`, `--qp-neutral`, `--qp-closed`, `--qp-focus`. Köken: `--qp-origin-human`,
  `--qp-origin-agent`, `--qp-origin-engine`, `--qp-agent-dash`. Ölçü: `--qp-space-0..5`,
  `--qp-fs-*`, `--qp-control-h`, `--qp-tab-h`, `--radius`, `--qp-radius-card`, `--pill-radius`,
  `--qp-shadow`, `--qp-font-figure`, `--motion`, `--ease`.
- **Metin çekirdekten:** bu belgedeki tırnaklı metinler **kalıptır**; cümleyi çekirdek kurar
  (sayı, tarih, saat `display_tz` ile). Kabuk metin birleştirmez, aritmetik yapmaz.
- **Kavram adları** `docs/CONCEPTS.md`'dendir. Bu belgede geçen ve sözlükte satırı olmayan üç ad
  kararlardan gelir, yeni kavram değildir: *çizim* (RX3 şema taslağı, K73), *hazırlık görevi*
  (K76), *kurumsal işlem* (K88). "İzleme" = niyetin izleme biçimi (`Intent.watch`); "sicil" =
  karne (`Ledger`; kayıt `ledger entry`, K75). Kuran round sözlüğe ekler ("Arkin'e sorular" S20).
- **Varsayımlar** satır içinde `**Varsayım (S#)**`; hepsi en sonda "Arkin'e sorular"da.

## Yerleşim iskeleti

**Masaüstü (Ç1, 1440×900, profil `tavek`, OV-13 üç sütun):**

| Bölge | İçerik | Yüzey |
|---|---|---|
| Üst çubuk | Sembol ya da komut ara (⌘K), gelen kutusu zili, ayarlar | Y22, Y13, Y28 |
| Sol sütun | İzleme listesi üstte, izleme görevleri altta (oransal bölme, AT-A3) | Y20, Y07 |
| Orta | Grafik hücresi(leri): periyot ve gösterge çubuğu üstte, çizim araç çubuğu sol içte, köken anahtarı çubuğun sonunda | Y01, Y02, Y18, Y19, Y23 |
| Grafik üstü | Seviye eksen etiketleri, sol uç çipleri, niyet hapı, kanıt bandı, ajan önerisi | Y04 hap, Y09, Y11, Y31, Y32 |
| Sağ sütun | Ajan paneli: katlanan çalışma, cevap, onay ve soru kartları, besteci (≤ %30 genişlik, AT-C1) | Y24, Y05, Y06, Y14 |
| Sağ yuva | Veri görünümleri için ayrılmış sekme yeri | Y29 |
| Alt şerit | Veri ve piyasa durumu, izleme sağlığı özeti, köken çizgi anahtarı | Y25, Y31 |

**Telefon (Ç3/Ç4, 390×844, profil `tavek-tel`, AT-E1):** alt sekme çubuğu
**Kule · Grafik · İzleme · Ajan**; üstte tek satır durum (Y25); karar ve ayrıntı **alt sayfada**,
tam sayfa yalnız bildirimden açılan karar için. Dokunma hedefi `--qp-control-h` (touch profili).

**Yerleşim kuralı (tavek):** seviyenin **sağ ucu değeridir** (fiyat oluğunda eksen etiketi), **sol
ucu kim ve ne durumda** (çip). Son barların üstü boş kalır. Varsayılan olarak yalnız seçili ve
durumu son 24 saatte değişmiş seviyenin çipi görünür; ötekiler yalnız eksen etiketi
(**Varsayım (S21)**, DIRECTION "Bilinen zayıflıklar").

## Köken gösterimi (bütün yüzeyler)

Köken = renk **ve** çizgi biçimi **ve** sözcük; hiçbiri tek başına (VV-D5, H2/H3).

| Köken | Grafikte | Satırda / kartta | Rol | Sözcük |
|---|---|---|---|---|
| İnsan | Düz çizgi, dolu eksen etiketi | Sol kenar düz çizgi | `--qp-origin-human` | "Sen" / kişinin adı |
| Ajan | Kesik çizgi (`--qp-agent-dash`), kesik çerçeveli etiket | Sol kenar kesik | `--qp-origin-agent` | "Ajan" |
| Motor | Noktalı çizgi, noktalı çerçeveli etiket | Sol kenar noktalı | `--qp-origin-engine` | "Motor" |
| Ajan önerdi, sen onayladın | Onayla düz çizgiye döner (sahibi insan) | Ayrıntıda iki satır: "Ajan önerdi · Sen onayladın, 14:35" | `--qp-origin-human` | — |
| Ortak analiz (sistem yayımlar) | Motor gibi | "Sistem · 1.240 üye" | `--qp-origin-engine` | "Sistem" |

- Grafiğe yeni ton ekleyen tek köken ajandır; sahne sakin kalır.
- Ajan işareti kullanıcı stiliyle maskelenmez; köken renksiz bırakılmaz (CONTRACT).
- Onay ve ret yalnız insan kökenlidir (`human_only`); ajan kökenli bağlantıda onay düğmesi hiç çizilmez.

## Veri durumu gösterimi (bütün yüzeyler)

Bağ durumu `BindingState` (`ready`, `pending`, `no_entitlement`, `no_data`, `stale`) + piyasa
durumu (`delayed`, `stale`, açık/kapalı, seans evresi) + oynatma. Kalıp adları yüzey tablolarında kullanılır.

| Kalıp | Kaynak | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|---|
| `D-canlı` | `ready`, `delayed: false` | İşaret yok; alt şeritte "Canlı"; son değer etiketi akar | `--muted` | "Canlı · son bar 14:35:02" |
| `D-gecikmeli` | `delayed: true` | Lejantın sonunda ve şeritte sözcük; değerler normal | `--muted` | "Gecikmeli 15 dk" |
| `D-bekleniyor` | `pending` | Son yerleşimin iskeleti (VV-E2); grileşme yok | `--line` iskelet | "Veri bekleniyor" |
| `D-bayat` | `stale` | Değerler donar, nabız yok, son değer etiketi soluk; izlemeler `İ-değerlendirilemiyor` | `--qp-warn` şekil ◌ + sözcük | "Veri bayat · son veri 14:35" |
| `D-yetkiyok` | `no_entitlement` | Değer yerine kilit şekli; kanıtın **var olduğu**, neye dayandığı ve gereken yetki yazılır (NORTH_STAR "Vazgeçilmezler") | `--muted` | "Bu veri için yetkin yok: BIST seviye 1 · kanıt var, 3 olguya dayanıyor" |
| `D-veriyok` | `no_data` | Boş alan + neden; varsa önerilen periyot (OV-19 geri düşüş) | `--muted` | "Bu periyotta veri yok · günlüğe geç" |
| `D-kapalı` | piyasa kapalı | Değerler son kapanış; şeritte evre | `--muted` | "Seans kapalı · açılış 10:00" |
| `D-koptu` | bağlantı yok | **Tek kalıcı şerit** (engelleyici; VV-B1: yalnız gerçek kopuşta, sayaçlı) | `--alert-bg` / `--alert-fg` | "Bağlantı yok — veri bayat · son veri 14:35 · yeniden deneniyor (3)" |
| `D-simüle` | yeniden oynatma | Grafik başlığında ve şeritte bant; sicil sayılarının yanında "simüle" | `--note-bg` / `--note-fg` | "Yeniden oynatma · simüle · 22 Ağu 14:35" |

Kural: **canlı ve simüle aynı toplamda, aynı listede sırasız karışmaz** (K75, K77); simüle satırı
hep sözcükle etiketlidir. Gecikmeli veri yalnız bilgi; bayat ve kopuk veri "değerlendirilemiyor"
üretir, gecikmeli üretmez (**Varsayım (S22)**).

## İzleme yaşam döngüsü (Y07–Y09, Y12, Y13, Y20, Y31, Y32, Y36 ortak)

PLAYGROUND §5: *önerildi → onaylandı → kuruldu → eşleşti → haber verildi → yeniden kuruldu /
süresi doldu / kaldırıldı*; yan durumlar ve K73 iki aşaması, K88 beklemede. Şekil + sözcük + rol;
sonuç sözcükleri `--qp-up`/`--qp-down` kullanmaz (yön rengi fiyatındır, VV-D1).

| Kalıp | Şekil | Sözcük (çip ve satır) | Rol | Grafikte |
|---|---|---|---|---|
| `İ-önerildi` | ○ kesik çerçeve | "Önerildi · onay bekliyor" | `--qp-origin-agent` | Kesik seviye + kesik eksen etiketi |
| `İ-soru` | ? | "Soru bekliyor · izleme kurulmadı" | `--qp-origin-agent` | Seviye insanınsa düz, çipte "?" |
| `İ-kuruldu` | ○ | "Kuruldu · izleniyor, son kontrol 14:35" | `--fg` | Düz seviye, soluk (`--muted` opaklığı) |
| `İ-kırıldı` | ◐ + pipler ■■□ | "Kırıldı · tutunma 2/3 bar · günlük" | `--fg` | Kırılım barında dikey çentik; çentikten bugüne tam güç |
| `İ-tutundu` | ● | "Tutundu · 3/3 bar" | `--qp-valid` | Çentik + seviye tam güç |
| `İ-geridöndü` | ◇ | "Geri döndü · 2. barda" | `--qp-invalid` | Çentik içi boş, seviye soluk |
| `İ-eşleşti` | ● | "Eşleşti · 14:35 · 112,40" (tek aşamalı: dokunma, varış, giriş) | `--qp-valid` | Tetik barında işaret (≤ `chart.mark_max_px`) |
| `İ-haber` | ek: "· haber verildi" | Eşleşme satırının sonuna eklenir | `--muted` | — |
| `İ-yeniden` | ↻ | "Yeniden kuruldu · sonraki kesişim bekleniyor" | `--fg` | `İ-kuruldu` gibi |
| `İ-durdu` | ‖ | "Duraklatıldı" | `--muted` | Seviye soluk, eksen etiketi çerçevesiz |
| `İ-süredoldu` | – | "Süresi doldu · 22 Ağu" | `--qp-closed` | Etiket kalkar; seviye çizim olarak kalır |
| `İ-kaldırıldı` | ✕ | "Kaldırıldı" (yalnız günlükte ve gelen kutusunda) | `--qp-closed` | Yok |
| `İ-çizimdeğişti` | △ | "Çizim değişti · yeniden onay bekliyor" | `--qp-warn` | Yeni geometri, eski konum soluk hayalet |
| `İ-çizimsilindi` | ✕ | "Çizimi silindi · izleme durdu" | `--qp-warn` | Yok |
| `İ-değerlendirilemiyor` | ◌ | "Değerlendirilemiyor · veri bayat (son veri 14:35)" / "· yetki yok" | `--qp-warn` | Seviye soluk; eksen etiketinin yanında ◌ |
| `İ-beklemede` | ◷ | "Beklemede · kurumsal işlem (bölünme 1:2, 23 Kas)" (K88) | `--qp-warn` | Seviye soluk; boşluk barı işaretsiz |
| `İ-başkaperiyot` | (ek) | "· günlükte kurulu" | `--muted` | Bu periyotta soluk |

**Sağlık** (OV-21, VV-B1; K72 "geri bildirim kanalı hep açık") yaşam döngüsünden ayrı eksendir:

| Sağlık | Sözcük | Rol | Kural |
|---|---|---|---|
| Bilinmiyor | "Bilinmiyor · ilk veri bekleniyor" | `--muted` | Başlangıç; yeşil kazanılır, varsayılmaz |
| İzleniyor | "İzleniyor · son kontrol 14:35" | `--fg` | Son kontrol saati motor saatinden |
| Bozuk | "Gecikiyor · son kontrol 25 dk önce" | `--qp-warn` | Süreli rapor yenilenmedi |
| Başarısız | "Çalışmıyor · neden: …" | `--alert-fg` | Tek kalıcı şerit (Y25) |
| Duraklatıldı | "Duraklatıldı" | `--muted` | İnsan kararı |

Sorun yokken sessiz; **şerit yoksa güvence geçerli**, ama alt şerit sağlığı bir satırla hep söyler
("5 izleme · son kontrol 14:35").

## Ortak durum kalıpları

Bütün yüzeyler bu kalıplara başvurur; yüzey yalnız farkını yazar.

| Kalıp | Ne gösterilir | Rol | Metin kalıbı | Kart |
|---|---|---|---|---|
| `O-boş` | Panel içinde tek satır: neden boş + bir sonraki adım; tek birincil eylem; çizim yok, simge yok | `--muted`, eylem `--accent` | "Henüz izleme yok. Bir seviye çiz ya da ajana sor." | AT-F1, VV-E1, OV-19 |
| `O-yükleniyor` | Son yerleşimin iskeleti; dönen simge yalnız 1 sn'den uzunsa; ilerleme yüzdesi uydurulmaz | `--line` | "Yükleniyor" (ekran okuyucu) | VV-E2, AT-F2 |
| `O-hata` | Satır içi, sınırlı, nedenli; okuma hatası "Yeniden dene", yazma hatası taslağı korur; aynı hata tek bildirim | `--qp-warn` şekil, metin `--fg` | "Kaydedilemedi: öneri yazıldıktan sonra doküman değişti. Taslak duruyor." | OV-20, VV-E3 |
| `O-bayat` | `D-bayat` | `--qp-warn` | "Veri bayat · son veri HH:MM" | AT-F2 |
| `O-yetkiyok` | `D-yetkiyok`; eylem varsa "Yetkiyi gör" (Y37) | `--muted` | "Bu veri için yetkin yok: …" | AT-B2 |
| `O-koptu` | `D-koptu`; yüzeyler son durumu gösterir, yazma denetimleri devre dışı ve nedenli | `--alert-bg`/`--alert-fg` | "Bağlantı yok — değişiklik gönderilmedi, taslak duruyor" (kendiliğinden yeniden gönderilmez, OV-20) | VV-B1 |
| `O-simüle` | `D-simüle`; yazma denetimleri oynatma kaydına yazar, canlıya değil | `--note-bg`/`--note-fg` | "Yeniden oynatma · simüle" | — |
| `O-ajankapalı` | Basamak "izle" ya da ajan devre dışı: ajan yüzeyleri tek satır | `--muted` | "Ajan yalnız okuyor (basamak: izle)" | — |
| `O-sınır` | Hak sınırına yaklaşınca (≥ %80) sayaç, ulaşınca eylem devre dışı + neden + Y37 bağı | `--qp-warn` | "Bu ay 48 / 50 soru" · "İzleme sınırı doldu (50 / 50)" | — |

Durum hiçbir zaman yalnız renkle söylenmez; her kalıpta sözcük vardır. Hareket yalnız durum
değişiminde (`--motion`, `--ease`); azaltılmış hareket tercihinde kapanır (AT-D2).

## Akış kuralları (OpenChart düzeyi; Arkin K1)

Arkin'in "inceleme ve akış şemaları" eleştirisi bu kurallarla karşılanır; her ajan yüzeyi uyar:

1. **Çalışma tek satıra katlanır, cevap açık durur** (OV-17, AT-C1, VV-A1): "Çalışma: 4 adım,
   6 sn ›". Koşarken adımlar canlı satırlar (araç adı + argüman özeti + ✓/✗ + süre); bitince
   katlanır; **başarısız ya da bitmemiş iş açık kalır**; elle açma bitişte sıfırlanır.
2. **Cevap önce, kanıt sonra:** cevabın ilk cümlesi sonucu söyler; sayılar motor cümlesinden;
   her sayı tıklanınca kanıta açılır (Y16); "Grafikte" bağı grafikteki öğeyi seçer.
3. **Onay kartı eylemi adlandırır ve durum izini tutar** (AT-C3, OV-18): tür etiketi · tek
   cümle özet · yük · "neye dayanıyor" satırı · birincil düğme fiil + nesne ("Grafiğe ekle",
   "İzlemeyi kur"), yanında nötr "Reddet". Karar sonrası düğmeler kalkar, yerine durum satırı:
   "Onaylandı · Sen, 14:35 · Grafikte".
4. **Soru kartı:** tek soru, ≤ 3 seçenek, her seçenek **tam koşul cümlesi**, biri hep
   **"İzleme, yalnız not kalsın"**; serbest metin alanı; "Soru 1 / 2"; cevapsız soru izleme kurmaz.
5. **Kur ve göster** (K72): ajan en olası okumayı kurar, grafikte hemen gösterir, tek dokunuşla
   değiştirilir; kazanılan güvenle aynı kalıpta sormaz.
6. **Grafiğe dönüş:** her panel öğesi "Grafikte" ile grafiğe iner; her grafik öğesi seçilince
   panelde kendi satırını vurgular (`--accent` yalnız seçim).
7. **Ajan boş dönebilir:** "Bulunacak yapı yok" bir sonuçtur, uydurma değil; satır olarak görünür.

## Yapılmayacaklar

- Kabukta renk kodu, piksel, metin birleştirme ya da aritmetik; değer yalnız `css_vars`.
- Durumu yalnız renkle söylemek; tutundu/geri döndü için `--qp-up`/`--qp-down` kullanmak.
- Krom rolüne renk: dolu not kartı, renkli başlık, turuncu büyük harf başlık (AT-C1 zayıf yanı).
- Kutu içinde kutu (AT-A1 karşı örneği); kart içinde kart yalnız onay kartının yükü.
- Son barların üstüne işaret, yüz, plaka, büyük simge; anlam işareti `chart.mark_max_px`'i aşmaz.
- Ajan önerisini onaysız uygulamak; ajan kökenli bağlantıya onay düğmesi çizmek.
- Canlı ve simüle kaydı tek toplamda, tek sıralamada göstermek; n < 20'de yüzde göstermek.
- "Al / sat / tut"u ajanın sesi olarak yazmak; kişiye uyarlanmış al/sat dili (K77).
- Uydurulmuş ilerleme yüzdesi, geri sayan saat, süs animasyonu, avatar ya da ajan kişiliği (VV-A6).
- Sessiz varsayılan: varsayılan değer her yerde "varsayılan" sözcüğüyle işaretlenir (PLAYGROUND §4).
- Bildirimde ve kilit ekranında lisanslı rakam (**Varsayım (S16)**).
- Yeni grafik öğesi icat etmek: seviye, alarm etiketi, imleç hapı biçimleri alfabedir.

## Arkin'e sorular

PLAYGROUND §6'daki tasarımı etkileyen sorular ve bu şartnamenin kendi soruları; her birinde
önerilen cevap şartnamede uygulandı.

| No | Soru | Önerilen cevap (şartnamede uygulanan) | Etkilediği yüzey |
|---|---|---|---|
| S1 | İlk çizim türleri hangileri? | Yatay seviye, trend çizgisi, ışın, kanal, bölge, zaman işareti, beklenti kutusu, metin notu, ölçüm; Fibonacci "sonra" (araç çubuğunda yer ayrılmaz) | Y02, Y04 |
| S2 | Niyet kartı kendiliğinden mi açılır? | Çizim bitince **tek satırlık niyet hapı** kendiliğinden görünür (kur ve göster); tam kart hapa dokununca ya da "Bunu izle" ile | Y04, Y03 |
| S3 | Kişi başı izleme sınırı ve "yaklaşınca haber ver"? | Sınır parametredir (K78); %80'de sayaç, dolunca kurma devre dışı + neden; "yaklaşınca" ilk sürümde yok | Y07, Y21, Y37 |
| S4 | "Kırıldı" neye göre? | Çizimin periyodunda **bar kapanışı**, "varsayılan" işaretli; fitil seçeneği soru kartında ikinci seçenek | Y04, Y05, Y32 |
| S7 | Derinlik/takas grafiğin yanında mı? | Yanında, sağ yuvada sekme; ilk sürümde yer ayrılmış ve boş | Y29 |
| S8 | Kaç grafik, klavye mi? | 1 / 2 / 4 hücre; klavye kısayolları komut paletinde görünür | Y19, Y22 |
| S9 | Günlük çizim gün içi grafikte görünsün mü? | Görünür, tam güçte; izleme çizimin periyodunda kalır, başka periyotta çipte "· günlükte kurulu" | Y09, Y23 |
| S10 | Düzeltilmiş fiyat varsayılan mı? | Evet; çizim grafiğin gösterdiği seriyle aynı düzeltmeyi alır (K88) | Y23, Y36 |
| S11 | Telefonda çizim? Günde kaç bildirim? | Telefonda yalnız **hızlı yatay seviye**; bildirim üst sınırı parametre, aşan gruplanır | Y02, Y27, Y12 |
| S12 | Hangi kanallar? | İlk sürüm uygulama içi + telefon bildirimi; e-posta ve Telegram "bağlı değil" durumuyla listede | Y28 |
| S13 | "Hedef/durdurma" dili tavsiye sayılır mı? | Ajan "plan" der, yön önermez; hedef/durdurma yalnız kullanıcının kendi beklenti kutusunda; sicil yüzlerinde "Sinyal, tavsiye değil" | Y04, Y33, Y35 |
| S14 | İlk gün varsayılan basamak? | **"Öner"**: ajan yazar, onay bekler; ajan çizimi yalnız öneri | Y15, Y30, Y11 |
| S15 | Taşıma (Matriks/TradingView) beklenir mi? | İlk sürümde yok; "taşıma sürüyor" durumu yalnız şartnamede | Y30 |
| S16 | Kilit ekranında rakam? | Yok: sembol + olay sözcüğü ("SYN20 · kırılım tutundu"); rakam uygulama açılınca | Y12 |
| S17 | Ardışık en çok kaç soru? | 2 ("Soru 1 / 2"); üçüncü belirsizlik işaretli varsayılanla kurulur | Y05 |
| S18 | Reddedilen öneri grafikte iz bırakır mı? | Bırakmaz; iz ajan günlüğünde ve öneri kartının durum satırında | Y11, Y06, Y14 |
| S19 | Önerinin süresi? | Parametre (K78); varsayılan seans sonu; süre geri sayım değil durum metni | Y06 |
| S20 | *Çizim*, *hazırlık görevi*, *kurumsal işlem*, *sağlık* sözlüğe ne zaman? | Kuran round (R7 / RS1) davranıştan önce `docs/CONCEPTS.md`'ye ekler | hepsi |
| S21 | Grafikte kaç çip görünür? | Yalnız seçili ve son 24 saatte durumu değişmiş seviyelerin çipi; ötekiler yalnız eksen etiketi | Y09, Y11, Y31, Y32 |
| S22 | Gecikmeli veride izleme değerlendirilir mi? | Evet, gecikmeyi satırda söyleyerek ("Veri 15 dk gecikmeli"); yalnız bayat ve kopuk veri değerlendirmeyi durdurur | Y07, Y25 |
| S23 | Zaman gezintisinde sonuçlar görünsün mü? | Gezintide görünür (geçmiş biliniyor); yeniden oynatmada imleçten sonrası gizli | Y34 |
| S24 | Kod düzeyinde uyarlama sınırı (K83)? | Ayrı karar satırı; bu şartname yalnız kalıp düzeyinde | — |
