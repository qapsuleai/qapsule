# Yüzeyler — izleme ve haber (Y07, Y08, Y12, Y13, Y20, Y21, Y31, Y32)

Ortak kurallar, **izleme yaşam döngüsü** (`İ-*`), **sağlık** tablosu ve veri kalıpları (`D-*`):
[`../SURFACES.md`](../SURFACES.md). "İzleme" = niyetin izleme biçimi (`Intent.watch`). Eşleşme
yalnız kesin barda (K73); açık barda durum değişmez.

### Y07 — İzleme görevleri listesi

**Ne:** Kullanıcının adına izlenen her şey: kaynağı (çizim, gösterge, elle seviye, ajan önerisi,
koruyucu varsayılan), koşul cümlesi, durumu, sağlığı. · **Kademe:** 1 · **Kararlar:** K72, K73, K78.

**Yerleşim:** Masaüstünde sol sütunun alt yarısı ("İzleme görevleri · 5 etkin"); satır iki
satırlıktır: (1) sembol + koşul cümlesi, (2) durum sözcüğü + sağlık + "Grafikte". Sol kenar
kaynağın köken çizgisi. Telefonda İzleme sekmesi.

**Durumlar**

| Durum | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|
| Boş | `O-boş` | `--muted` | "Henüz izleme yok. Bir seviye çiz, notunu yaz; ajan kursun." |
| Yaşam döngüsünün her durumu | Satırın 2. satırında `İ-*` şekli + sözcüğü | `İ-*` rolü | "Kırıldı, tutunma 2/3 bar · Grafikte" |
| Sağlık | 2. satır sonunda (yalnız İzleniyor dışındaysa vurgulu) | sağlık rolü | "İzleniyor, son kontrol 14:35" · "Bilinmiyor, ilk veri bekleniyor" |
| Gecikmeli veri | 2. satırda bilgi (S22) | `--muted` | "Veri 6 dk gecikmeli" |
| Sınıra yaklaşıldı / ulaşıldı | Başlıkta sayaç (S3) | `--qp-warn` | "48 / 50 izleme" · "Sınır doldu" |
| Süzgeç | Başlık altında haplar: sembol, köken, durum | `--accent` | "Ajan · Kırıldı · 2" |
| Toplu işlem | Satır başında seçim kutuları, altta çubuk | `--accent` | "3 seçili · Duraklat · Kaldır" |
| Sıralama | Varsayılan: önce karar bekleyen, sonra durumu değişen, sonra izlenen (çekirdek sıralar) | — | — |

**Köken:** Satırın sol kenarı kaynağın köken çizgisi: insan düz, bekleyen ajan önerisi kesik,
motorun kapsül izlemesi noktalı. Koruyucu varsayılan (K72) insanın pozisyonu için kurulur: düz
çizgi + "varsayılan" sözcüğü.

**Veri durumu:** Satır başına; bayat ve yetki yokta `İ-değerlendirilemiyor`; liste başlığında
toplu sözcük "2 izleme değerlendirilemiyor".

**Akış:** Satıra gel → grafikte seviye vurgulanır; tıkla → ayrıntı (Y08); "Grafikte" → hücre o
sembol ve aralığa gider; durum değişince satır bir kez vurgulanır ve listenin üstüne çıkar.

**Masaüstü / telefon:** Telefonda satırlar tam genişlik, sağa kaydırma "Duraklat", sola "Kaldır"
(geri alınabilir, alt bildirim "Geri al"); sağlık tek simge + sözcük.

**Uyarlanan:** OV-21, AT-B3, AT-C7, VV-B2. **Roller:** `--fg`, `--muted`, `--qp-valid`,
`--qp-invalid`, `--qp-warn`, `--qp-closed`, `--qp-origin-*`, `--accent`.

### Y08 — İzleme görevi ayrıntısı

**Ne:** Tek görev: koşulun tam hali, kim kurdu, kim onayladı, hangi çizimin hangi sürümüne bağlı,
olay geçmişi, kalan süre, sicil kaydı. · **Kademe:** 2 · **Kararlar:** K73, K75.

**Yerleşim:** Masaüstünde listeden açılan sağ panel ayrıntısı (liste yerinde kalır); telefonda tam sayfa.

**Bölümler:** Başlık (sembol, koşul cümlesi, durum hapı) · Koşul parçaları (Y04 tablosu, kaynak
sözcükleriyle) · Köken izi ("Sen çizdin 12 Kas → Ajan okudu → Sen onayladın") · Bağ ("Trend
çizgisi, sürüm 3") · Olay geçmişi (zaman sıralı, her olay kanıta açılır) · Sicil ("Bu izlemenin
sonucu sicile yazılır: tutundu / geri döndü", Y33) · Eylemler (Düzenle, Duraklat, Kaldır).

**Durumlar**

| Durum | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|
| Kuruldu | `İ-kuruldu` + sağlık | `--fg` | "İzleniyor, son kontrol 14:35" |
| Eşleşti | Olay satırı + kanıt bağı | `--qp-valid` | "14:35 · günlük kapanış 113,05 > 112,40 · Kanıt" |
| Yeniden kuruldu | `İ-yeniden`, olay sayısı | `--fg` | "2 kez eşleşti · yeniden kuruldu" |
| Durduruldu | `İ-durdu` | `--muted` | "Duraklatıldı · Sen, 14:40" |
| Süresi doldu | `İ-süredoldu` | `--qp-closed` | "Süresi doldu 22 Ağu" |
| Çizimi değişti | `İ-çizimdeğişti`; eski/yeni geometri karşılaştırması, onay eylemi | `--qp-warn` | "Çizim taşındı: 112,40 → 111,80 · izlemeyi yeni çizgiye bağla" |
| Çizimi silindi | `İ-çizimsilindi` | `--qp-warn` | "Çizimi silindi · izleme durdu" |
| Değerlendirilemiyor | `İ-değerlendirilemiyor` + neden | `--qp-warn` | "Veri bayat (son 14:35)" |
| Beklemede | `İ-beklemede` (Y36) | `--qp-warn` | "Kurumsal işlem: taşınıyor" |
| Düzenleniyor | Parçalar düzenlenebilir; kaydet/vazgeç; değişen parça "Sen" | `--qp-origin-human` | "Kaydet" · "Vazgeç" |

**Köken:** Köken izi bölümü üç kökeni sırayla, çizgi biçimleriyle gösterir.

**Veri durumu:** Başlıkta `D-*`; olay geçmişinde simüle olaylar ayrı alt başlıkta ("Yeniden oynatma · simüle").

**Akış:** Liste → ayrıntı → olay → kanıt (Y16) → geri; "Düzenle" → parçalar → kaydet → durum `İ-yeniden`.

**Masaüstü / telefon:** Telefonda bölümler katlanabilir başlıklar; olay geçmişi son 5 + "Tümü".

**Uyarlanan:** AT-C3 (durum izi), AT-C7, VV-B2. **Roller:** `İ-*` rolleri, `--qp-origin-*`, `--card`, `--line`.

### Y12 — Bildirim

**Ne:** Olay olduğunda gelen kısa haber: ne oldu, hangi çizim ya da koşul, hangi değer; kanıta
açılır. · **Kademe:** 1 · **Kararlar:** K72, K73, A57.

**Yerleşim:** Masaüstünde sağ üstte, başlığın altında yığılan uygulama içi kart (OV-20 konumu),
ortak bildirim yüzeyi; zil sayacı (Y13). Telefonda sistem bildirimi; uygulama açıkken üstte şerit.

**Metin kalıbı (çekirdek kurar):** "SYN20 · 112,40 kırılımı tutundu (3/3 bar, günlük) · 113,05".
Birinci kısım sembol, ikinci olay sözcüğü (`İ-*`), üçüncü değer. Eylemler: "Kanıt" · "Grafikte" · türe göre karar.

**Durumlar**

| Durum | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|
| Tetiklendi | Kart, sol kenar kaynağın köken çizgisi | `İ-*` rolü | "SYN20 · Kırıldı · tutunma 1/3 bar" |
| Soru soruyor | Kart + en çok üç seçenek düğmesi (Y05) | `--qp-origin-agent` | "SYN20 · Ajan soruyor: kırılımı neye göre sayayım?" |
| Öneri bekliyor | Kart + "Reddet" / fiil + nesne | `--qp-origin-agent` | "Ajan önerisi · 117,50'ye destek ekle" |
| Süresi dolmak üzere | Sakin kart, geri sayım yok | `--muted` | "SYN20 izlemesinin süresi yarın doluyor · uzat" |
| Süresi doldu | Yalnız gelen kutusunda | `--qp-closed` | — |
| Değerlendirilemiyor | Kart, tek kez | `--qp-warn` | "3 izleme değerlendirilemiyor: veri bayat (son 14:35)" |
| Düşük güven (A57) | Elindeki hissede yazı-tura düzeyindeki olumsuz tespit | `--qp-warn` sözcük | "SYN20 · zayıfladı · düşük güven (taban orandan ayrışmıyor)" |
| Bastırıldı | Gösterilmez; gelen kutusunda süzgeçle | `--muted` | "Süzgeçle bastırıldı" |
| Gruplanmış | Tek kart, sayı | `--fg` | "SYN20 · 3 olay · aç" |
| Simüle | Bildirim üretilmez; oynatma panelinde satır | `--note-fg` | "simüle olay" |

**Köken:** Sol kenar çizgisi: insan izlemesi düz, ajan sorusu/önerisi kesik, motor kapsülü noktalı.

**Veri durumu:** Telefonda kilit ekranında rakam yok (**Varsayım (S16)**): "SYN20 · kırılım tutundu";
rakam uygulama açılınca. Gecikmeli veride metne "· 15 dk gecikmeli" eklenir.

**Akış:** Olay → bildirim → dokun → kanıt (Y16) ya da karar kartı (Y05/Y06) → karar bildirimden
verilirse uygulama açılmadan durum satırı ("Eklendi") gelir.

**Masaüstü / telefon:** Masaüstünde kart birkaç saniye sonra zile iner (kapatma düğmesi yok,
OV-20); karar bekleyen kart inmez, gelen kutusunda kalır. Telefonda günlük üst sınır parametre,
aşan gruplanır (S11).

**Uyarlanan:** OV-20, AT-C6, VV-F2. **Roller:** `İ-*` rolleri, `--qp-origin-*`, `--card`, `--qp-shadow`, `--qp-warn`.

### Y13 — Gelen kutusu

**Ne:** Bütün bildirimlerin, soruların ve bekleyen önerilerin zaman sıralı listesi; kaçırılanlar
burada. · **Kademe:** 2 · **Kararlar:** K72.

**Yerleşim:** Masaüstünde üst çubuktaki zilden açılan liste (panel genişliği); başlıkta iki sekme
"Karar bekleyen · Tümü". Telefonda kontrol kulesinin üst bölümü (Y26).

**Durumlar**

| Durum | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|
| Boş | `O-boş` | `--muted` | "Yeni bir şey yok · 5 izleme sağlıklı" |
| Okunmamış var | Zilde sayı; satır başında nokta | `--fg` | "3 yeni" |
| Karar bekleyen var | Sekmede sayı; satırda karar eylemleri yerinde (AT-C6) | `--qp-origin-agent` | "2 karar bekliyor" |
| Süzülmüş | Haplar: sembol, tür, köken | `--accent` | "SYN20 · soru" |
| Eski (arşiv) | 7 günden eski katlı | `--muted` | "Daha eski · 24" |
| Ajan çalışıyor | Listenin tepesinde canlı satır | `--qp-origin-agent` | "Ajan SYN20'yi okuyor…" |

**Köken:** Satır sol kenarı köken çizgisi; insanın kendi işlemleri listeye girmez.

**Veri durumu:** Satır o anki veri durumunu taşır; simüle olaylar ayrı süzgeçte, varsayılan gizli.

**Akış:** Zil → liste → satırda karar (soru/öneri yerinde cevaplanır) → satır durum satırına iner.

**Masaüstü / telefon:** Telefonda kulede "Karar bekleyen" kartları üstte, "Bugün" akışı altta.

**Uyarlanan:** AT-C6, OV-21 (zil), VV-F2. **Roller:** `--qp-origin-*`, `--fg`, `--muted`, `--accent`.

### Y20 — İzleme listesi paneli

**Ne:** Sembol listeleri, sütunlar, satırda izleme ve kapsül işareti. · **Kademe:** 2 · **Kararlar:** K74.

**Yerleşim:** Masaüstünde sol sütunun üst yarısı: Sembol · Son · Değişim (tablo rakamı), satır
sonunda küçük durum işareti (izleme ◐ / kapsül ◆). Telefonda İzleme sekmesinde ikinci bölüm.

**Durumlar**

| Durum | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|
| Boş liste | `O-boş` | `--muted` | "Liste boş · sembol ekle (⌘K)" |
| Yükleniyor | Satır iskeleti | `--line` | — |
| Canlı | Değişim yön renginde, son değer akar | `--qp-up`/`--qp-down` | "+1,42%" |
| Gecikmeli | Başlıkta sözcük | `--muted` | "Gecikmeli 15 dk" |
| Satırda yetki yok | Değer yerine kilit | `--muted` | "yetki yok" |
| Satırda izleme kurulu / tetiklendi / soru | Satır sonu işaret + `İ-*` şekli | `İ-*` rolü | "◐" · "●" · "?" |
| Satırda kapsül | Motor elması (≤ `chart.mark_max_px`) | `--qp-origin-engine` | "◆" |
| Sıralanmış | Sütun başında ok | `--muted` | — |
| Düzenleme kipi | Sürükleme tutamacı, sil | `--muted` | "Bitti" |

**Köken:** Satır işaretleri kökenlidir: izleme işareti kaynağın çizgisini taşımaz (yer yok), ama
üstüne gelince "Sen kurdun" / "Ajan önerdi" der.

**Veri durumu:** Satır başına `D-*`; simülede başlıkta "Sentetik veri, oynatma".

**Akış:** Satıra tıkla → etkin hücre o sembole geçer; işarete tıkla → izleme ayrıntısı (Y08) ya da kapsül.

**Masaüstü / telefon:** Telefonda Sembol · Son · Değişim; ek sütun yok.

**Uyarlanan:** OV-22, VV-F1, AT-B4. **Roller:** `--qp-up`, `--qp-down`, `--qp-font-figure`, `--muted`, `--qp-origin-engine`.

### Y21 — İzleme kurma formu (çizimsiz)

**Ne:** Fiyat seviyesi ya da gösterge koşulu için elle kurma: olay, kaynak, periyot, tekrar, süre.
· **Kademe:** 2 · **Kararlar:** K73, K78.

**Yerleşim:** Masaüstünde izleme görevleri başlığındaki "+" ile açılan yüzen kart; alanlar tek
sütun; altta motorun kurduğu önizleme cümlesi. Telefonda en yalın hali: seviye + yön.

**Durumlar**

| Durum | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|
| Boş | Alanlar varsayılanlarıyla, her varsayılan "varsayılan" | `--muted` | "Kapanış · varsayılan" |
| Geçerli | Önizleme cümlesi + "İzlemeyi kur" | `--fg`, `--accent` | "SYN20 günlük kapanış 112,40'ın altına inerse haber ver, bir kez" |
| Geçersiz birleşim | Alan altında neden | `--qp-warn` | "Yatay seviyede ‘bölgeye giriş’ yok" |
| Zaten doğru | Uyarı + seçenek (K72 uyar ve öner) | `--qp-warn` | "Fiyat şu an 112,40'ın altında; bu koşul olay üretmez. Yukarı kesişimi mi izleyeyim?" |
| Sınıra ulaşıldı | `O-sınır`, kur düğmesi devre dışı | `--qp-warn` | "İzleme sınırı doldu (50 / 50)" |
| Kuruldu | Kart kapanır, listede yeni satır vurgulu | `--fg` | "Kuruldu" |

**Köken:** Form insanındır (düz); ajan alanı doldurduysa o alan kesik çerçeveli + "Ajan".

**Veri durumu:** Bayatta kurulabilir, `İ-değerlendirilemiyor` ile başlar; yetki yokta sembol seçilemez.

**Akış:** "+" → sembol (etkin hücreden dolu) → olay → değer → önizleme → kur.

**Masaüstü / telefon:** Telefonda seviye + yukarı/aşağı iki düğme; geri kalan varsayılan, işaretli.

**Uyarlanan:** VV-E3, OV-21. **Roller:** `--card`, `--line`, `--muted`, `--qp-warn`, `--accent`.

### Y31 — Sessiz zil ve izlemenin sağlık göstergesi

**Ne:** (a) Notsuz, cümlesiz, pozisyonsuz çizimde hiçbir şey kurulmaz: çizimin sol ucunda sessiz
zil, üstüne gelince açıklama. (b) İzlemelerin sağlığı: tek tek ve toplu. · **Kademe:** 1 ·
**Kararlar:** K72 (kur ve göster, geri bildirim kanalı, koruyucu varsayılan), K73 (sağlık göstergesi).

**Yerleşim:** (a) Grafikte, çipin yerinde ≤ `chart.mark_max_px` soluk zil, üstü çizili değil, içi boş.
(b) Satırda (Y07) ve alt şeritte (Y25) "5 izleme · son kontrol 14:35"; sağlıksızsa tek kalıcı şerit.

**Durumlar**

| Durum | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|
| Sessiz zil | İçi boş zil, soluk | `--muted` | — |
| Sessiz zil · üstüne gelince | Küçük açıklama kartı + "Bunu izle" | `--card`, `--muted` | "Bu çizim için izleme kurulmadı: not, cümle ya da pozisyon yok. Not yaz ya da ‘Bunu izle’." |
| Koruyucu varsayılan kuruldu | Zil yerine `İ-kuruldu` çipi + sözcük | `--fg` + `--muted` | "Koruyucu izleme · varsayılan · pozisyonunun altında" |
| Kur ve göster | Not yazılınca zil çipe döner | `--fg` | "Kuruldu: 112,40 altında kapanış · değiştir" |
| Sağlık: bilinmiyor | Satırda ve şeritte | `--muted` | "Bilinmiyor · ilk veri bekleniyor" |
| Sağlık: izleniyor | Yalnız saat | `--fg` | "İzleniyor · son kontrol 14:35" |
| Sağlık: gecikiyor | Satır vurgulu | `--qp-warn` | "Gecikiyor · son kontrol 25 dk önce" |
| Sağlık: çalışmıyor | Tek kalıcı şerit, başlangıç saati, ayrıntı açılır | `--alert-bg`/`--alert-fg` | "İzlemeler çalışmıyor: veri bağlantısı yok · 14:35'ten beri" |
| İyileşti | Tek bildirim, kaçırılan süre | `--fg` | "İzlemeler yeniden çalışıyor · 14:35–15:02 arası değerlendirildi" |
| Duraklatıldı | `İ-durdu` | `--muted` | "Duraklatıldı" |

**Köken:** Zilin kendisi kökensizdir (sistemin söylediği); koruyucu izleme insan izlemesidir ve
"varsayılan" sözcüğünü taşır; sağlık şeridi motordan.

**Veri durumu:** Bayat ve kopuk veri sağlığı "gecikiyor/çalışmıyor"a çeker; gecikmeli veri sağlığı bozmaz (S22).

**Akış:** Çizim → (not yok) sessiz zil → üstüne gel → açıklama → "Bunu izle" → niyet kartı (Y04).
Sağlık: başlangıç "bilinmiyor" → ilk kontrol "izleniyor" → rapor gecikirse "gecikiyor" → kopuşta şerit → iyileşince tek bildirim.

**Masaüstü / telefon:** Telefonda zil yok (yer yok); çizime dokununca alt sayfada aynı cümle.
Sağlık telefonda üst durum satırında tek simge + sözcük, şerit tam genişlik.

**Uyarlanan:** OV-21, VV-B1, VV-B2, AT-B1. **Roller:** `--muted`, `--fg`, `--qp-warn`, `--alert-bg`, `--alert-fg`, `--card`.

### Y32 — İki aşamalı kırılım

**Ne:** Kırılım iki aşamalıdır: **kırıldı → tutundu / geri döndü**; yön açık, periyot yazılı,
sonuç sicile. · **Kademe:** 1 · **Kararlar:** K73 (iki aşama; periyot birinci sınıf; günlük
çizimde ara periyot, günlükte teyit), K75.

**Yerleşim:** Grafikte seviyenin sol ucu çipi + kırılım barında dikey çentik; çentikten bugüne
seviye tam güçte. Satırda (Y07) aynı sözcük. Pipler (■■□) tutunma barlarını sayar.

**Durumlar**

| Durum | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|
| Kuruldu | `İ-kuruldu` | `--fg` | "İzleme ○ Kuruldu · aşağı kırılım · günlük kapanış" |
| Ara periyotta kırıldı | Çip + periyot yazısı; çentik ara periyot barında, içi boş | `--fg` | "◐ 4 saatlikte kırıldı · günlükte teyit bekleniyor" |
| Kırıldı | `İ-kırıldı`: çentik, pipler ■□□ | `--fg` | "◐ Kırıldı · tutunma 1/3 bar · günlük" |
| Tutunma sürüyor | Pipler doluyor | `--fg` | "◐ Kırıldı · tutunma 2/3 bar" |
| Tutundu | `İ-tutundu`: pipler ■■■, çentik dolu | `--qp-valid` | "● Tutundu · 3/3 bar · günlük · sicile yazıldı" |
| Geri döndü | `İ-geridöndü`: çentik içi boş, seviye soluk | `--qp-invalid` | "◇ Geri döndü · 2. barda · sicile yazıldı" |
| Açık bar | Açık barda kırılım görünse de durum değişmez | `--muted` | "Açık bar seviyenin altında · bar kapanınca kesinleşir" |
| Değerlendirilemiyor | `İ-değerlendirilemiyor`; pipler donar | `--qp-warn` | "◌ Değerlendirilemiyor · veri bayat" |
| Simüle | Aynı çip, başında "simüle" | `--note-fg` | "simüle · ◐ Kırıldı 2/3" |

**Köken:** Seviye çizgisi çizimin kökeni; çentik ve pipler motorun tespiti (noktalı ince çentik
çerçevesi); sonuç sözcüğü motor cümlesi.

**Veri durumu:** Kırılım yalnız kesin barda; bayatta pipler donar; gecikmelide çipe "· 15 dk gecikmeli".

**Akış:** Kuruldu → kesin bar seviyeyi geçer → "Kırıldı 1/3" + bildirim (Y12) → her kesin bar pip
doldurur → 3/3 "Tutundu" ya da geri geçişte "Geri döndü" → ikinci bildirim → sicil kaydı (Y33) →
izleme `rearm`'a göre yeniden kurulur ya da biter.

**Masaüstü / telefon:** Telefonda eksen etiketi biçimi "112,40 │ ◐ Kırıldı 2/3"; pipler yalnız
alt sayfada; periyot yazısı alt sayfada tam cümle.

**Uyarlanan:** OV-12, OV-21. **Roller:** `--fg`, `--qp-valid`, `--qp-invalid`, `--qp-warn`,
`--muted`, `--note-fg`, `--qp-origin-engine`.
