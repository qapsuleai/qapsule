# Yüzeyler — grafik (Y01, Y02, Y03, Y09, Y10, Y11, Y16, Y17, Y18, Y19, Y23)

Ortak kurallar, kalıp adları (`D-*`, `İ-*`, `O-*`) ve köken tablosu: [`../SURFACES.md`](../SURFACES.md).
Grafik alfabesi (sağ fiyat ekseni, iki düzeyli zaman, OHLC lejantı, son değer etiketi + değer
çizgisi, imleç hapları, hacim) çekirdekte, gramerdedir (OV-5…OV-10, CONTRACT "Grafik alfabesi");
burada yeniden tanımlanmaz, yalnız durumları yazılır.

### Y01 — Grafik hücresi

**Ne:** Tek grafiğin kendisi: mumlar, alışılmış öğeler ve üstünde Qapsule katmanı (seviye eksen
etiketleri, sol uç çipleri, kanıt bandı). · **Kademe:** 1 · **Kararlar:** K70, K73, K14.

**Yerleşim:** Masaüstünde Ç1'in ortası; üstte periyot/gösterge çubuğu (Y23), sol içte araç
çubuğu (Y02), lejant sol üstte (Y18), fiyat oluğu sağda (`chart.axis_gutter_px`), hacim alt
bölmede. Telefonda Grafik sekmesi, tam genişlik (Y27).

**Durumlar**

| Durum | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|
| Boş (sembol yok) | `O-boş`: ızgara ve eksenler yok, ortada tek satır + arama alanına odak | `--muted` | "Bir sembol seç ya da ajana sor (⌘K)" |
| Yükleniyor | `O-yükleniyor`: eksen oluğu ve lejant iskeleti, mum alanı boş | `--line` | — |
| Canlı | `D-canlı`: son mum güncellenir, son değer etiketi seri renginde | `--qp-up`/`--qp-down` mum | "Canlı · son bar 14:35:02" (şeritte) |
| Gecikmeli | `D-gecikmeli`: lejantın sonunda sözcük | `--muted` | "Gecikmeli 15 dk" |
| Veri bayat | `D-bayat`: son değer etiketi soluk, değer çizgisi kalkar, nabız yok | `--qp-warn` ◌ | "Veri bayat · son veri 14:35" |
| Yetki yok | `D-yetkiyok`: mum alanında kilit satırı; çizimler ve Qapsule katmanı görünmez, varlıkları sayılır | `--muted` | "Bu sembolün fiyat verisi için yetkin yok: BIST seviye 1 · 2 çizim, 1 izleme bu veriye bağlı" |
| Veri yok | `D-veriyok` | `--muted` | "Bu periyotta veri yok · günlüğe geç" |
| Hata | `O-hata` hücre içinde; eksenler kalır | `--qp-warn` | "Grafik çizilemedi: … · Yeniden dene" |
| Piyasa kapalı | `D-kapalı`; seans bantları (OV-11) soluk | `--muted` | "Seans kapalı · açılış 10:00" |
| Yeniden oynatma | `D-simüle`: başlıkta bant, zaman ekseninde oynatma imleci hapı | `--note-bg`/`--note-fg` | "Yeniden oynatma · simüle · 22 Ağu 14:35" |
| Kanıt penceresi açık | Kapsül açıldığında: bütün bölmeleri geçen soluk bant + zaman ekseninde "Kanıt" parantezi + tepede ≤ `chart.mark_max_px` motor elması | `--qp-origin-engine` (alfa) | "Kanıt · 14–22 Ağu" |

**Köken:** Mumlar ve göstergeler veridir (seri paleti `--qp-s1..8`), kökensizdir. Üstteki her
seviye köken çizgi biçimini taşır (SURFACES "Köken gösterimi"): insan düz, ajan kesik, motor
noktalı; eksen etiketi aynı biçimde çerçevelenir (dolu / kesik / noktalı).

**Veri durumu:** Yukarıdaki tablo; bağ durumu seri başınadır: karşılaştırma serisi `no_entitlement`
ise yalnız o seri kilitli satıra döner, ana seri çizilir ("çözülemeyen bağ belgenin geri kalanını durdurmaz").

**Akış:** (1) Sembol seçilir → iskelet → mumlar. (2) İmleç gezer → kesik imleç, oluktaki fiyat
hapı, zaman eksenindeki tarih hapı, lejant imleç barının OHLC'sine geçer. (3) Seviyeye gelinir →
seviye kalınlaşır, sol uçta çip açılır (S21). (4) Çipe tıklanır → yan panelde ilgili satır
(Y07/Y08) vurgulanır. (5) Kapsül "Grafikte göster" → hücre kanıt penceresine kayar, bant + parantez.

**Masaüstü / telefon:** Masaüstünde imleç ve OHLC lejantı sürekli; telefonda lejant yalnız C ve
değişim, imleç dokunup tutunca; telefonda oluk `tavek-tel` değerinde, çip yer darlığında eksen
etiketinin içine "112,40 │ ◐ 2/3" biçiminde girer.

**Uyarlanan:** OV-5…OV-12, OV-11, AT-H1. **Roller:** `--qp-s1..8`, `--qp-up`, `--qp-down`,
`--qp-grid`, `--qp-axis`, `--qp-scene-bg`, `--qp-scene-text`, `--qp-origin-*`, `--qp-agent-dash`,
`--qp-font-figure`, `--qp-fs-figure`.

### Y02 — Çizim araç çubuğu

**Ne:** Çizim türleri, ölçüm, mıknatıs, geri al / yinele. · **Kademe:** 2 · **Kararlar:** K70.

**Yerleşim:** Masaüstünde grafik hücresinin sol iç kenarında dikey, yüzen değil yapışık şerit
(OV-15); grup ayırıcıları saç çizgisi. Telefonda yok; yerine Grafik sekmesinde tek "Seviye" düğmesi (S11).

**Durumlar**

| Durum | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|
| Araç seçili değil | İmleç aracı etkin | `--muted` simge | İpucu: "Seç (V)" |
| Araç seçili | Simge zemini dolu | `--accent` / `--on-accent` | "Trend çizgisi (T) · iki nokta" |
| Çizim sürüyor | İlk çapa kondu; imleçte geçici çizgi insan mürekkebinde | `--qp-origin-human` | "İkinci noktayı koy · Esc iptal" |
| Sürekli kip | Simgede kilit noktası | `--accent` | "Sürekli kip açık" |
| Mıknatıs açık/kapalı | Mıknatıs simgesi dolu / boş | `--accent` / `--muted` | "Mıknatıs: açık (OHLC)" |
| Salt okunur | Bütün araçlar soluk, neden ipucunda | `--muted` | "Bu belgede yalnız okuma yetkin var" / `D-yetkiyok` |
| Yeniden oynatma | Araçlar etkin; çizim oynatma kaydına | `--note-fg` rozet | "Çizim simüle kayda yazılır" |

**Köken:** Bu yüzeyden çıkan her çizim insanındır (düz mürekkep). Çubuğun sonunda köken anahtarı:
"İnsan, düz · Ajan, kesik · Motor, noktalı" (DIRECTION).

**Veri durumu:** Bayatta ve kopukta çizim yerelde yapılabilir ama "gönderilmedi" taslağı olur
(`O-koptu`); yetki yokta araçlar soluk.

**Akış:** Araç seç → çapalar → çizim biter → **niyet hapı kendiliğinden** (Y04, S2) → kullanıcı
yok sayarsa çizim yalnız çizim kalır, sessiz zil (Y31).

**Masaüstü / telefon:** Masaüstü tam araç kümesi (S1); telefon yalnız hızlı yatay seviye.

**Uyarlanan:** OV-15, OV-14. **Roller:** `--muted`, `--accent`, `--on-accent`, `--line`,
`--qp-control-h`, `--radius`.

### Y03 — Seçili çizim denetimleri ve bağlam menüsü

**Ne:** Seçili çizimin yüzen çubuğu ve sağ tık menüsü: stil, not, kilitle, gizle, sil, "Bunu izle".
· **Kademe:** 2 · **Kararlar:** K70, K72, K73.

**Yerleşim:** Masaüstünde çizimin üstünde yüzen tek satır (yalnız yüzen kartta `--qp-shadow`);
sağ tık menüsü aynı eylemler. Telefonda alt sayfa.

**Durumlar**

| Durum | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|
| Seçili | Çizim tutamaçları + yüzen çubuk | `--accent` tutamaç | — |
| Kilitli | Kilit simgesi dolu; taşıma tutamaçları yok | `--muted` | "Kilitli · aç" |
| Gizli | Çizim yok, nesne listesinde göz kapalı (Y10) | `--muted` | "Gizli · izlemesi sürüyor" |
| İzleme kurulu | Çubukta "İzleniyor" + durum çipi (`İ-*`) | `İ-*` rolü | "İzleniyor · Kırıldı 2/3" |
| İzleme soru bekliyor | `İ-soru`; "Cevapla" eylemi Y05'i açar | `--qp-origin-agent` | "Soru bekliyor" |
| Ajan çizimi seçili | Kesik çizgi; çubukta adlı iki eylem | `--qp-origin-agent` | "Reddet" · "Grafiğe ekle" |
| Başka periyotta çizilmiş | Çubukta bilgi | `--muted` | "Günlükte çizildi · izleme günlükte" |

**Köken:** Çubuğun ilk öğesi kökendir: "Sen" / "Ajan önerisi" / "Motor"; motor işareti (olgu,
kapsül) seçildiğinde stil ve sil eylemleri yoktur, yalnız "Kanıta aç".

**Veri durumu:** `D-bayat`'ta "Bunu izle" etkin ama sonuç `İ-değerlendirilemiyor` ile başlar;
`D-yetkiyok`'ta çizim zaten görünmez.

**Akış:** Seç → çubuk → "Bunu izle" → niyet kartı (Y04) tam açılır, grafikten ayrılmadan.

**Masaüstü / telefon:** Telefonda alt sayfa: üstte köken + durum satırı, altta eylemler
dokunma hedefi boyunda.

**Uyarlanan:** OV-14, OV-15, AT-C8. **Roller:** `--card`, `--line`, `--qp-shadow`, `--accent`,
`--qp-origin-*`.

### Y09 — Grafik üstünde izleme işareti

**Ne:** İzlenen çizimin ya da seviyenin grafikte durumunu söylemesi; tetik barının işareti.
· **Kademe:** 1 · **Kararlar:** K72, K73.

**Yerleşim:** tavek kuralı: **sağ uç eksen etiketi** (değer), **sol uç çip** ("İzleme ◐ Kırıldı
■■□ 2/3 bar"). Kırılım barında küçük dikey çentik; çentikten bugüne seviye tam güçte, öncesi soluk.

**Durumlar**

| Durum | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|
| Kuruldu | `İ-kuruldu`: seviye soluk, eksen etiketi | `--fg` | "İzleme ○ Kuruldu" |
| Soru bekliyor | `İ-soru` | `--qp-origin-agent` | "İzleme ? Soru bekliyor" |
| Kırıldı | `İ-kırıldı` + çentik + pipler | `--fg` | "İzleme ◐ Kırıldı 2/3 bar" |
| Tutundu / geri döndü | `İ-tutundu` / `İ-geridöndü` | `--qp-valid` / `--qp-invalid` | "● Tutundu" / "◇ Geri döndü" |
| Eşleşti | `İ-eşleşti`: tetik barında işaret (≤ `chart.mark_max_px`) | `--qp-valid` | "● Eşleşti 14:35" |
| Durduruldu | `İ-durdu` | `--muted` | "‖ Duraklatıldı" |
| Süresi doldu | `İ-süredoldu`: çip kalkar, eksen etiketi kalkar | `--qp-closed` | — |
| Başka periyotta kurulu | `İ-başkaperiyot`: soluk, çipte periyot | `--muted` | "· günlükte kurulu" |
| Gizli çizimin izlemesi | Yalnız eksen etiketi (çizgi yok), noktasız ince çerçeve | `--muted` | "İzleme · çizim gizli" |
| Değerlendirilemiyor | `İ-değerlendirilemiyor` | `--qp-warn` | "◌ Değerlendirilemiyor" |
| Beklemede | `İ-beklemede` (Y36) | `--qp-warn` | "◷ Beklemede" |
| Geçici bar | Açık barda olay görülse de durum değişmez; çipte not | `--muted` | "bar kapanınca kesinleşir" |

**Köken:** Seviyenin çizgisi çizimin kökenidir (insan düz / ajan kesik); izleme durumu çipte
sözcükle, kökenden bağımsız. Koruyucu izleme (K72) çipte "koruyucu · varsayılan".

**Veri durumu:** Bayat ve yetki yok → `İ-değerlendirilemiyor`; gecikmeli → çipte "· 15 dk gecikmeli" (S22).

**Akış:** Durum değişir → çip bir kez vurgulanır (`--motion`) → bildirim (Y12) → çipe tıklama
görev ayrıntısını (Y08) açar.

**Masaüstü / telefon:** Telefonda yalnız eksen etiketi biçiminde "112,40 │ ◐ 2/3"; çip yok,
dokununca alt sayfa.

**Uyarlanan:** OV-12, OV-21 (çan hapı), AT-C8. **Roller:** `--fg`, `--muted`, `--qp-valid`,
`--qp-invalid`, `--qp-warn`, `--qp-closed`, `--qp-origin-*`.

### Y10 — Köken ayrımı ve nesne listesi

**Ne:** Grafikteki her öğenin kimin olduğu; süzülebilir liste. · **Kademe:** 2 · **Kararlar:** K72.

**Yerleşim:** Masaüstünde sağ sütunda "Nesneler" sekmesi (OV-16 nesne ağacı); grafik araç
çubuğunda köken süzgeci (üç çizgi biçimli düğme). Telefonda liste yok.

**Durumlar**

| Durum | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|
| İnsan çizimi | Sol kenar düz | `--qp-origin-human` | "Trend çizgisi · Sen" |
| Ajan çizimi (onaylı) | Sol kenar düz, alt satır iki köken | `--qp-origin-human` | "Ajan önerdi · Sen onayladın, 14:35" |
| Ajan önerisi (bekliyor) | Sol kenar kesik, "Bekliyor" | `--qp-origin-agent` | "Destek 117,50 · Ajan · onay bekliyor" |
| Motorun işareti | Sol kenar noktalı; silinemez | `--qp-origin-engine` | "Kapsül · Kırılım tutundu · Motor" |
| Kilitli / gizli / izleniyor | Satır sonunda simge + sözcük | `--muted` | "Kilitli" · "Gizli" · "İzleniyor ◐" |
| Boş | `O-boş` | `--muted` | "Bu grafikte nesne yok" |
| Süzülmüş | Başlıkta süzgeç hapı | `--accent` | "Yalnız ajan · 3" |

**Köken:** Yüzeyin kendisi kökendir; her satır üç kanal (renk + çizgi + sözcük).

**Veri durumu:** Yetkisiz veriye bağlı nesne satırı kilitli ama sayılır ("veriye yetkin yok").

**Akış:** Satıra gel → grafikte öğe vurgulanır; tıkla → seçilir (Y03).

**Masaüstü / telefon:** Telefonda yalnız grafikteki çizgi biçimi ayrımı.

**Uyarlanan:** OV-16, OV-3, AT-G2. **Roller:** `--qp-origin-*`, `--qp-agent-dash`, `--muted`, `--accent`.

### Y11 — Ajan katmanı (grafikte bekleyen öneri ve ajan çizimi)

**Ne:** Ajanın grafiğe eklediği ya da önerdiği yapıların insan çizimlerinden ayrı görünmesi; tek
hareketle gizleme. · **Kademe:** 1 · **Kararlar:** K72, K77; `human_only`.

**Yerleşim:** Grafik üstü. Önerilen seviye kesik mor çizgi + eksende kesik çerçeveli etiket; seçiliyken
sol uç çipinde adı konmuş iki eylem. Katman anahtarı araç çubuğunda ("Ajan" çizgi simgesi).

**Durumlar**

| Durum | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|
| Ajan çalışıyor | Lejant satırının sonunda tek satır, grafikte henüz bir şey yok | `--qp-origin-agent` | "Ajan grafiği okuyor…" |
| Öneri bekliyor | Kesik seviye + kesik eksen etiketi + çip | `--qp-origin-agent`, `--qp-agent-dash` | "Ajan önerisi · Destek" · "Reddet" · "Grafiğe ekle" |
| Toplu öneri | Birden çok kesik öğe, çipte sayı | `--qp-origin-agent` | "Ajan önerisi · 3 değişiklik" |
| Kabul edildi | Düz insan çizgisine döner; çip bir kez "Eklendi" der, sonra kalkar | `--qp-origin-human` | "Eklendi · Sen onayladın" |
| Reddedildi | Grafikten kalkar, iz bırakmaz (S18) | — | Ajan panelinde durum satırı |
| Süresi doldu / çakışma | Kesik öğe soluk; çipte neden | `--qp-warn` | "Öneri bayatladı: doküman değişti · yeniden iste" |
| Katman gizli | Ajan öğeleri yok; anahtarda sayı | `--muted` | "Ajan katmanı gizli · 2 öneri bekliyor" |
| Bulunacak yapı yok | Grafikte bir şey yok; panelde sonuç satırı | `--muted` | "Ajan bu aralıkta seviye bulmadı" |

**Köken:** Bu yüzey ajan kökeninin kendisidir; grafiğe yeni ton ekleyen tek köken. Onay düğmesi
yalnız insan kökenli bağlantıda çizilir.

**Veri durumu:** Bayatta öneri yapılmaz ("Veri bayat, öneri bekletiliyor"); simülede öneri "simüle" etiketli.

**Akış:** Kullanıcı ajana sorar (Y24) → çalışma satırı → grafikte kesik öneri + panelde onay
kartı (Y06), ikisi birbirine "Grafikte" ile bağlı → "Grafiğe ekle" → çizgi düz mürekkebe döner,
kart durum satırına iner.

**Masaüstü / telefon:** Telefonda grafikte yalnız kabul edilmiş ajan öğeleri ve bekleyen öneri sayısı
("2 öneri"); öneri kontrol kulesinde kendi küçük grafiğinde (Y26).

**Uyarlanan:** AT-C8, OV-3, OV-18, AT-C3. **Roller:** `--qp-origin-agent`, `--qp-agent-dash`,
`--qp-origin-human`, `--qp-warn`, `--muted`.

### Y16 — Kanıta açılma

**Ne:** Bildirimden, kapsülden ya da görevden grafiğe iniş: tetik barı, o andaki çizim, değerler,
köken. · **Kademe:** 2 · **Kararlar:** K75; NORTH_STAR "Vazgeçilmezler".

**Yerleşim:** Masaüstünde grafik hücresi o ana kayar (kanıt bandı + parantez), sağ panelde kanıt
listesi (olgular, değerler, köken). Telefonda anlam zoom'unun son adımı: grafik + alt sayfa (Ç4).

**Durumlar**

| Durum | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|
| Açılıyor | `O-yükleniyor`, hücre o aralığa kayar | `--line` | — |
| Kanıt gösteriliyor | Bant + parantez + tetik barı işareti; olgu satırları noktalı kenar | `--qp-origin-engine` | "Kapanış seviyenin üstünde 113,05 / 112,40" |
| Çizim o günden beri değişti | O günkü sürüm hayalet çizgi, bugünkü düz | `--qp-warn` sözcük | "O günkü çizim gösteriliyor · sonra 2 kez değişti" |
| Yetki yok | `D-yetkiyok`: kanıt var, neye dayandığı, gereken yetki | `--muted` | "Kanıt var: 3 olgu, BIST seviye 1 gerekiyor" |
| Veri bayat | `D-bayat` | `--qp-warn` | "Veri bayat · kanıt o ana kadar" |
| Geçici | Bar kesinleşmedi; işaret içi boş | `--muted` | "Bar kapanınca kesinleşir" |
| Yeniden oynatma kaydı | `D-simüle` | `--note-fg` | "Simüle kayıttan" |

**Köken:** Olgular motor (noktalı), yorum ajan (kesik), çizim insanın (düz); kanıt listesi üçünü ayrı satırlarda tutar.

**Veri durumu:** Yukarıda; kanıt hiçbir durumda saklanmaz, yalnız değerleri.

**Akış:** Bildirim / kapsül / görev → "Kanıta aç" → hücre o aralık → olgu satırına gel → grafikte o
bar vurgulanır → "Bugüne dön".

**Masaüstü / telefon:** Telefonda alt sayfa yarım yükseklikte, grafik üstte görünür kalır.

**Uyarlanan:** VV-A4, VV-C4, AT-C2. **Roller:** `--qp-origin-*`, `--qp-warn`, `--muted`, `--note-fg`.

### Y17 — Gösterge seçici ve parametre formu

**Ne:** Gösterge arama, ekleme, parametre değiştirme. · **Kademe:** 2 · **Kararlar:** K78.

**Yerleşim:** Masaüstünde periyot çubuğundaki "Göstergeler" düğmesinden açılan yüzen arama kartı;
parametre formu lejant satırının ayarından açılan küçük kart. Telefonda yalnız hazır kümeler.

**Durumlar**

| Durum | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|
| Boş arama | Son kullanılanlar ve kümeler | `--muted` | "Son kullanılanlar" |
| Sonuç yok | `O-boş` | `--muted` | "‘xyz’ için gösterge yok · ajana sor" |
| Eklendi | Lejantta yeni satır, kısa vurgu | `--accent` | "RSI 14 eklendi" |
| Hesaplanıyor | Lejant değerinde iskelet | `--line` | — |
| Isınma | Isınma barları boş, lejantta not | `--muted` | "Isınma: ilk 14 bar boş" |
| Geçersiz parametre | Alan altında neden, alan çerçevesi | `--qp-warn` | "Uzunluk 1–500 arası" |
| Ajan önerdi | Satır kesik kenar, onay eylemleri | `--qp-origin-agent` | "Ajan önerisi · MACD 12 26 9 · Reddet · Ekle" |
| Parametre kaynağı | Her varsayılanın yanında "varsayılan" sözcüğü; "?" Y37'yi açar | `--muted` | "14 · varsayılan" |

**Köken:** Kullanıcının eklediği gösterge insan; ajanın önerdiği kesik satır; motorun kapsül için kurduğu noktalı.

**Veri durumu:** Yetki yokta hesap yapılmaz, lejant satırı kilitli (`D-yetkiyok`).

**Akış:** Ara → seç → eklenir → lejant ayarından parametre → değer anında yeniden hesaplanır.

**Masaüstü / telefon:** Telefonda arama yok; "Trend", "Momentum", "Hacim" kümeleri ve görüntüleme.

**Uyarlanan:** OV-23 (arama biçimi), VV-A5. **Roller:** `--card`, `--line`, `--muted`, `--qp-warn`, `--qp-origin-agent`.

### Y18 — Lejant ve panel başlığı

**Ne:** Seri ve gösterge satırları: ad, parametre, değer, gizle, ayar, kaldır; panel taşıma.
· **Kademe:** 1 · **Kararlar:** K14.

**Yerleşim:** Bölmenin sol üst köşesi; ilk satır OHLC + değişim lejantı (çekirdek kurar),
altında gösterge satırları. Denetimler satıra gelince görünür (OV-14).

**Durumlar**

| Durum | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|
| Görünür | Ad · parametre · değer (tablo rakamı) | `--fg`, değer `--qp-font-figure` | "RSI 14  55,10" |
| Gizli | Ad soluk, göz kapalı | `--muted` | "RSI 14 · gizli" |
| Hata | Değer yerine şekil + kısa neden | `--qp-warn` | "RSI 14 · hesaplanamadı" |
| Isınma | Değer yerine "—" | `--muted` | "RSI 14 —" |
| Ajan ekledi | Ad önünde kesik kısa çizgi + "Ajan" | `--qp-origin-agent` | "Ajan · MACD 12 26 9" |
| Motor ekledi | Noktalı kısa çizgi | `--qp-origin-engine` | "Motor · Hacim ort. 20" |
| Seçili | Satır zemini | `--accent` (ince kenar) | — |
| Daraltılmış | Yalnız ilk satır + "+3" | `--muted` | "+3 gösterge" |
| Gecikmeli / bayat / yetki yok | İlk satırın sonunda `D-*` sözcüğü | `D-*` rolü | "Gecikmeli 15 dk" · "Veri bayat" |
| Simüle | İlk satır sonunda bant sözcüğü | `--note-fg` | "simüle" |

**Köken:** Satır önündeki kısa çizgi biçimi (düz / kesik / noktalı) kökendir; insan eklediğinde çizgi yok (varsayılan).

**Veri durumu:** İlk satır `D-*` kalıbını taşır; gösterge satırları ana seriden miras alır.

**Akış:** Satıra gel → gizle / ayar / kaldır → ayar Y17 formunu açar; sürükle → bölme taşınır.

**Masaüstü / telefon:** Telefonda yalnız C + değişim ve gösterge adları; değerler dokununca.
Bilinen kusur: telefonda "RSI14" lejantı çizgiye binebiliyor (DIRECTION); lejant zemini `--qp-scene-bg` alfa ile ayrılır.

**Uyarlanan:** OV-7, OV-14, VV-D4. **Roller:** `--fg`, `--muted`, `--qp-font-figure`, `--qp-fs-figure`, `--qp-origin-*`, `--qp-warn`.

### Y19 — Çoklu grafik yerleşimi ve bağlama grubu

**Ne:** 1, 2, 4 hücre; hücre büyütme; sembol/zaman bağlama grubu. · **Kademe:** 3 · **Kararlar:** —.

**Yerleşim:** Masaüstü orta bölge ızgarası (OV-25, AT-A2); etkin hücre ince `--accent` kenar; bağlama
grubu hücre başlığında renk değil **harf** rozeti (A, B) (**Varsayım (S8)**).

**Durumlar**

| Durum | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|
| Tek / çok hücre | Izgara; hücreler arası saç çizgisi | `--line` | — |
| Büyütülmüş | Hücre tam alan, ötekiler sekme | `--muted` | "2 hücre gizli" |
| Bağlı / bağımsız | Başlıkta "A" rozeti / rozet yok | `--muted` | "Bağlı: A" |
| Etkin hücre | İnce kenar | `--accent` | — |
| Bir hücre hata / yetki yok | Yalnız o hücre `O-hata` / `D-yetkiyok` | ilgili | — |

**Köken:** Hücre içi kurallar Y01. **Veri durumu:** Hücre başına `D-*`. **Akış:** Yerleşim düğmesi → 1/2/4 → hücreye tıkla → etkin.

**Masaüstü / telefon:** Telefonda ızgara yok; hücreler yatay kaydırılan sayfalar.

**Uyarlanan:** OV-25, AT-A2, AT-A3. **Roller:** `--line`, `--accent`, `--muted`.

### Y23 — Zaman dilimi, karşılaştırma ve ölçek denetimleri

**Ne:** Periyot seçimi, karşılaştırma sembolü, log / otomatik ölçek, düzeltme seçimi.
· **Kademe:** 2 · **Kararlar:** K73 (periyot birinci sınıf), K88.

**Yerleşim:** Grafik hücresinin üst çubuğu: periyot hapları (15dk · 1sa · Günlük), gösterge
düğmesi, karşılaştırma "+"; ölçek ve düzeltme sağ altta (zaman ekseninin sonunda). Telefonda kısa periyot şeridi.

**Durumlar**

| Durum | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|
| Seçili periyot | Hap dolu | `--accent`/`--on-accent` | "Günlük" |
| Özel periyot | Hap yazılı | `--accent` | "2 sa" |
| Karşılaştırma açık | Lejantta ikinci seri satırı, yüzde ölçeği | `--qp-s2` | "+ XU100 %" |
| Bu periyotta veri yok | `D-veriyok`; hap soluk | `--muted` | "15dk veri yok · günlüğe geç" |
| Çizimler başka periyottan | Çubukta bilgi satırı | `--muted` | "3 çizim günlükte çizildi" |
| Düzeltilmiş / ham | Düğme metni (S10) | `--muted` | "Düzeltilmiş" |
| Log / otomatik | Düğme dolu/boş | `--accent` | "Log" · "Oto" |

**Köken:** Periyot değişimi insanın hareketi; ajan periyot önerirse kesik hap: "Ajan: 4 sa önerir".

**Veri durumu:** Periyot hapı veri durumunu taşımaz; `D-*` lejantta.

**Akış:** Periyot seç → hücre yeniden derlenir → başka periyotta kurulu izlemeler `İ-başkaperiyot` olur.

**Masaüstü / telefon:** Telefonda periyot şeridi tek satır, karşılaştırma ve ölçek "Daha fazla" alt sayfasında.

**Uyarlanan:** OV-6, OV-11, AT-H1. **Roller:** `--accent`, `--on-accent`, `--muted`, `--qp-s2`.
