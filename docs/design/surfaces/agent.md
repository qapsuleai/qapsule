# Yüzeyler — ajan (Y04, Y05, Y06, Y14, Y15, Y24)

Ortak kurallar, kalıp adları ve **akış kuralları** (çalışma katlanır, onay kartı eylemi
adlandırır, soru kartı ≤ 3 tam cümle): [`../SURFACES.md`](../SURFACES.md). Arkin'in ara kararı
(K1) bu yüzeylerin yoğunluğunu OpenChart'ın ajan paneli ve açıklama akışı düzeyine çeker;
başvuru: `dashboard-preview.png` sağ sütun (OV-17, OV-18), OpenTerminalUI `agent.jpg` (AT-C1).

### Y04 — Çizim niyet kartı

**Ne:** Ajanın çizimden ne anladığı: tek cümle beklenti, koşulun parçaları (olay, kaynak, tekrar,
süre, periyot), **varsayılanlar işaretli**, dayandığı not ve bağlam. · **Kademe:** 1 ·
**Kararlar:** K72, K73; PLAYGROUND §4.

**Yerleşim:** İki katman. (a) **Niyet hapı** grafikte, çizginin başında, boş tarafta tek satır:
"Altında kapanış → uyar · 3 bar · günlük" (DIRECTION). (b) **Tam kart** sağ panelde ajan akışının
içinde (masaüstü) ya da alt sayfada (telefon); grafik kapanmaz.

**Durumlar**

| Durum | Hap (grafikte) | Kart | Rol | Metin kalıbı |
|---|---|---|---|---|
| Ajan okuyor | "…" kesik çerçeve | Çalışma satırı canlı | `--qp-origin-agent` | "Çizimi okuyor · 2 sn" |
| Anladı, öneri bekliyor | Tam cümle, kesik çerçeve | Cümle + parçalar + "İzlemeyi kur" / "Değiştir" / "Yalnız çizim" | `--qp-origin-agent` | "112,40 altında günlük kapanış olursa haber ver" |
| Kur ve göster (güven kazanılmış) | Düz çerçeve + "kuruldu" | Kart katlı, "Değiştir" tek dokunuş | `--fg` | "Kuruldu: 112,40 altında kapanış · değiştir" |
| Koruyucu varsayılan (K72) | "koruyucu · varsayılan" | Pozisyon satırıyla kurulmuş | `--fg` + `--muted` sözcük | "Pozisyonunu koruyan taraf: kuruldu" |
| Emin değil, soruyor | "?" | Y05 soru kartına geçer | `--qp-origin-agent` | "Yön belirsiz · cevapla" |
| Okunamadı | Hap yok | Tek satır | `--muted` | "Ölçüm cetveli niyet taşımaz" |
| Bayat çizim | "△ bayat" | Uyarı + seçenek | `--qp-warn` | "Fiyat bu seviyenin 4 bar önce üstüne geçti; geri dönüşü mü bekliyorsun?" |
| Çelişki bulundu | "△ çelişki" | İki çizim yan yana, ikisi de kurulmaz | `--qp-warn` | "Bu bölgede zıt beklentili iki çizim var; hangisi geçerli?" |
| Kullanıcı düzeltti | Düz çerçeve | Değişen parça vurgulu, "varsayılan" kalkar | `--qp-origin-human` | "3 bar → 2 bar · Sen" |
| Onaylandı | Hap kalkar, çip `İ-kuruldu` | Durum satırı | `--fg` | "İzleme kuruldu · Sen, 14:35" |
| Reddedildi | Hap kalkar | Durum satırı | `--muted` | "Yalnız çizim kaldı" |
| Ajan kapalı (basamak "izle") | Hap yok | `O-ajankapalı` | `--muted` | "Ajan yalnız okuyor" |

**Kartın gövdesi (her durumda aynı sıra):** (1) tek cümle beklenti (motor kurar, K72); (2)
parçalar tablosu: Olay · Kaynak · Periyot · Tekrar · Süre · Tutunma, her biri değer + kaynak
sözcüğü: "senin notundan" / "çizimden" / "**varsayılan**" (Y37 "neden?" bağı); (3) dayandığı not
alıntısı; (4) eylemler. İşaretsiz varsayılan yoktur; `hold_bars` söylenmediyse kart soru olur (§4.2).

**Köken:** Çizim insanın (düz); kartı ajan yazar (kesik sol kenar); cümleyi motor kurar (cümle
yanında "Motor cümlesi" küçük noktalı işaret). Kullanıcı düzeltmesi insan rengine geçer.

**Veri durumu:** Bayatta kart açılır ama "kur" eylemi `İ-değerlendirilemiyor` ile başlayacağını söyler;
yetki yokta kart yalnız çizim düzeyinde konuşur, değer yazmaz; simülede "simüle kayda kurulur".

**Akış:** Çizim biter → 1–2 sn çalışma satırı → hap belirir (kur ve göster) → kullanıcı (a) yok
sayar: hap 8 sn sonra soluklaşır, çizim sessiz zilde (Y31); (b) hapa dokunur: tam kart; (c) bir
parçaya dokunur: o parça yerinde seçenek listesine döner; (d) "İzlemeyi kur": çip `İ-kuruldu`,
kart durum satırına katlanır, ajan günlüğüne (Y14) satır.

**Masaüstü / telefon:** Masaüstünde hap grafikte + kart panelde; telefonda hap yok, bildirimden
açılan tam sayfa; parçalar dikey liste, eylemler altta yapışık.

**Uyarlanan:** OV-12 (hap, öncü çizgi), OV-18, AT-C3, VV-B3 (varsayılan rozeti). **Roller:**
`--qp-origin-agent`, `--qp-agent-dash`, `--qp-origin-human`, `--qp-origin-engine`, `--qp-warn`,
`--muted`, `--card`, `--qp-radius-card`, `--pill-radius`.

### Y05 — Netleştirme sorusu ve seçenek yüzeyi

**Ne:** Ajanın tek sorusu ve en çok üç seçenek; her seçenek tam koşul cümlesi; biri hep
"İzleme, yalnız not kalsın". · **Kademe:** 1 · **Kararlar:** K72 (yalnız okumalar yön olarak
ayrışıyorsa sorar).

**Yerleşim:** Masaüstünde ajan panelinde, bestecinin hemen üstünde ayrı kart (OV-18); grafikte
ilgili çizimin çipinde "?". Telefonda bildirimden tek dokunuşla cevaplanır (bildirim eylemleri =
seçenekler), uygulamada alt sayfa.

**Durumlar**

| Durum | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|
| Soru bekliyor | Başlık "Soru 1 / 2", soru, ≤ 3 radyo, serbest metin, "Atla" / "Sonraki" | `--card`, kenar `--qp-origin-agent` kesik | "Kırılımı neye göre sayayım?" |
| Seçenek örneği | Tam cümle | `--fg` | "Günlük kapanış 112,40'ın altında olursa haber ver" · "Fitil 112,40'a değince haber ver" · "İzleme, yalnız not kalsın" |
| Cevaplandı | Kart katlanır: soru + seçilen cevap tek satır | `--muted` + `--qp-origin-human` | "Kırılım: günlük kapanış · Sen" |
| Serbest metinle cevap | Ajan cevabı tekrar cümleye çevirir, onay ister | `--qp-origin-agent` | "Şunu mu demek istedin: …?" |
| Ertelendi | Kart gelen kutusuna iner | `--muted` | "Sonra cevaplanacak · izleme kurulmadı" |
| Zaman aşımı | Durum satırı | `--qp-closed` | "Cevap gelmedi · izleme kurulmadı" |
| Ardışık ikinci soru | "Soru 2 / 2"; üçüncüsü sorulmaz (**Varsayım (S17)**) | `--card` | — |
| Ajan kökenli bağlantı | Seçenekler salt okunur | `--muted` | "Yalnız insan cevaplar" |

**Köken:** Soru ajan (kesik kenar); cevap insan (cevap satırının sol kenarı düz).

**Veri durumu:** Seçenek cümlelerindeki sayılar motordan; bayatta soru yine sorulur, cümlenin sonuna
"(veri bayat, son 14:35)" eklenir.

**Akış:** Ajan belirsizlik görür → çipte "?" + kart → seçenek → "Sonraki" → ikinci soru ya da kart
katlanır → niyet kartı (Y04) güncellenir → izleme kurulur ya da yalnız not kalır.

**Masaüstü / telefon:** İçerik aynı; telefonda seçenekler tam genişlik dokunma satırları; bildirimde
en çok üç eylem düğmesi (seçenekler) ve "Aç".

**Uyarlanan:** OV-18, VV-A5, AT-C6. **Roller:** `--card`, `--qp-origin-agent`, `--qp-agent-dash`,
`--qp-origin-human`, `--muted`, `--qp-closed`, `--accent` (seçili radyo).

### Y06 — Öneri onay / ret yüzeyi

**Ne:** Ajanın yazdığı ama uygulanmamış değişiklik (izleme, çizim, gösterge, yerleşim) ve onun
kararı (`Proposal`: bekliyor → kabul | ret | süresi doldu). · **Kademe:** 1 · **Kararlar:**
K72, `human_only`.

**Yerleşim:** Masaüstünde ajan panelinde kart + grafikte önizleme (Y11; `preview` sorgusu);
kart ile önizleme "Grafikte" bağıyla eşleşir. Telefonda kontrol kulesinde kart + küçük grafik.

**Kartın yapısı (AT-C3):** tür etiketi ("Ajan önerisi · Çizim") · durum hapı ("Onay bekliyor") ·
tek cümle özet ("117,50'ye yatay destek ekle") · yük (önizleme açıklaması, toplu öneride
değişiklik listesi) · "Neye dayanıyor" satırı (olgu bağları, Y16) · kapsam notu ("Onaylarsan çizim
senin adına kaydedilir") · eylemler: "Grafikte" (bağ), nötr "Reddet", birincil **fiil + nesne**.

**Durumlar**

| Durum | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|
| Öneri bekliyor | Tam kart, kesik sol kenar | `--qp-origin-agent`, birincil `--accent` | "Grafiğe ekle" · "Reddet" |
| Uygulanıyor | Düğmeler kalkar, satır | `--muted` | "Uygulanıyor…" |
| Onaylandı | Durum satırı + iz | `--qp-origin-human` | "Onaylandı · Sen, 14:35 · Grafikte" |
| Reddedildi | Durum satırı; grafikte iz yok (S18) | `--muted` | "Reddedildi · Sen, 14:36" |
| Süresi doldu | Durum satırı, sakin (geri sayım yok, S19) | `--qp-closed` | "Süresi doldu (seans sonu) · yeniden iste" |
| Çakışma | Düğmeler kalkar; neden | `--qp-warn` | "Öneri yazıldıktan sonra doküman değişti · yeni öneri gerekir" |
| Yetki yok | Düğmeler yerine neden | `--muted` | "Bu belgede karar yetkin yok (rol: viewer)" |
| Ajan kökenli bağlantı | Düğme hiç yok | `--muted` | "Kararı yalnız insan verir" |
| Toplu öneri | Yükte liste + tek karar; tek tek açılabilir | `--qp-origin-agent` | "3 değişiklik: 2 seviye, 1 izleme · Hepsini ekle" |
| Doğrulanamayan sayı | Yükte sayı başına biçim rozeti (✓ / ≠ / ?) | `--qp-warn` (≠) | "1 sayı kanıtla uyuşmuyor · yayımlanmadı" |

**Köken:** Öneri ajan; karar insan; kart durum izi ikisini sırayla tutar ("Ajan önerdi 14:31 →
Sen onayladın 14:35").

**Veri durumu:** Bayatta karar verilebilir ama kart uyarır ("veri bayat; önizleme 14:35'e göre");
simülede "simüle kayda uygulanır".

**Akış:** Öneri gelir → panelde kart + grafikte kesik önizleme → "Grafikte" ile ikisi vurgulanır →
karar → düğmeler kalkar, durum satırı; onayda grafikte çizgi düz mürekkebe döner; günlükte (Y14) satır.

**Masaüstü / telefon:** Telefonda özet + küçük grafik + "Kanıta aç"; eylemler altta yapışık, tam genişlik.

**Uyarlanan:** AT-C3, OV-18, VV-A5, AT-C2. **Roller:** `--card`, `--qp-origin-agent`,
`--qp-agent-dash`, `--qp-origin-human`, `--accent`, `--on-accent`, `--qp-warn`, `--qp-closed`, `--muted`.

### Y14 — Ajan etkinlik günlüğü

**Ne:** Ajanın yaptığı her şey (okudu, yorumladı, sordu, önerdi, kurdu, haber verdi); zamanı ve
dayanağıyla. Satır = günlük girdisi (`LogEntry`), sıra günlükten. · **Kademe:** 2 · **Kararlar:** K72.

**Yerleşim:** Masaüstünde ajan panelinde "Etkinlik" sekmesi; her ajan işi tek satırlık özet
(katlı), açılınca adım satırları. Telefonda Ajan sekmesinde özet satırlar.

**Durumlar**

| Durum | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|
| Boş | `O-boş` | `--muted` | "Ajan henüz bir şey yapmadı" |
| Akıyor | Canlı satırlar: durum şekli + adım + ayrıntı + süre; liste takip eder, yukarı kaydırınca durur | `--qp-origin-agent` | "Grafiği okudu · SYN20 günlük · ✓ 1,2 sn" |
| Bitti | İş tek satıra katlanır, sonuç bağıyla | `--muted` + bağ `--accent` | "Seviye önerdi · 4 adım, 6 sn · Öneriye git" |
| Hata | İş açık kalır, başarısız adım ✗ | `--qp-warn` | "✗ Veri okunamadı: yetki yok" |
| Reddedilen yorum | Satır + neden; yorum yayımlanmadı | `--qp-warn` | "Yorum yayımlanmadı: 1 sayı kanıtla uyuşmuyor" |
| Süzülmüş | Başlıkta süzgeç hapları | `--accent` | "SYN20 · izlemeler" |
| Geri alındı | Satır üstü çizili değil, ek satır | `--muted` | "Geri alındı · Sen, 14:40" |

**Köken:** Bütün satırlar ajan (kesik sol kenar); insanın kararları aynı akışta düz kenarlı satır olarak araya girer.

**Veri durumu:** Satırlar o anki veri durumunu taşır ("veri bayatken"); simüle iş "simüle" etiketli, canlı işlerle aynı listede ama ayrı süzgeç.

**Akış:** Ajan çalışır → canlı satırlar → bitince katlanır → özete tıkla → adımlar → adıma tıkla → dayanak (Y16).

**Masaüstü / telefon:** Telefonda yalnız katlı özetler; adımlar tam sayfada.

**Uyarlanan:** AT-C4, VV-A2, OV-17. **Roller:** `--qp-origin-agent`, `--qp-agent-dash`, `--muted`, `--qp-warn`, `--accent`, `--qp-font-figure` (süre).

### Y15 — Onay basamağı ayarı

**Ne:** Ajanın yetki düzeyi: izle / öner / onayla / kendi başına; eylem türü başına ayrılabilir.
· **Kademe:** 3 · **Kararlar:** K72, K77 (otomatik pozisyon en üst basamak, varsayılan kapalı).

**Yerleşim:** Ayarlar > Ajan (Y28 ile aynı sayfa); satır başına eylem türü (izleme kurma, çizim
ekleme, gösterge, yerleşim), sütunlarda dört basamak (segment denetim).

**Durumlar**

| Durum | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|
| Varsayılan | "Öner" seçili + "varsayılan" (**Varsayım (S14)**) | `--accent`, `--muted` | "Öner · varsayılan" |
| Kullanıcı değiştirdi | Seçim + kaynak | `--fg` | "Onayla · senin seçimin, 12 Kas" |
| Kilitli | Segment soluk + kilit + neden | `--muted` | "Kurum kuralı: kendi başına kapalı" |
| Geçici yükseltme | Satırda süre sözcüğü | `--qp-warn` | "Bu oturum için: onayla" |
| İlk kullanım | Seçim yapılmamış; Y30 akışı | `--muted` | "Ajan ne kadar yapsın?" |

**Köken:** Ayarın kendisi insanındır. İnsan çizimine dokunma hiçbir basamakta onaysız değildir; satır bunu sabit metinle söyler.

**Veri durumu:** Uygulanmaz (veri bağsız). **Akış:** Segment seç → anında yazılır → ajan panelinin başlığı yeni basamağı gösterir.

**Masaüstü / telefon:** Aynı içerik; telefonda satır başına alt sayfa seçici.

**Uyarlanan:** VV-A5 (ayrıcalıklı onay yolu). **Roller:** `--accent`, `--muted`, `--qp-warn`, `--control`.

### Y24 — Ajan paneli (soru ve cevap)

**Ne:** Grafik bağlamını bilen soru-cevap alanı; cevaplar kaynaklı ve grafikteki öğeye bağlı;
ayrı sohbet ürünü değildir. · **Kademe:** 2 · **Kararlar:** K71, K77; A61.

**Yerleşim:** Masaüstünde sağ sütun (≤ %30, AT-C1): başlık (bağlam: "SYN20 · günlük", basamak) →
akış (soru, katlı çalışma, cevap, kartlar) → bestecinin üstünde bekleyen soru kartı → besteci
("Sor ya da @ ile bir izlemeyi an"). Telefonda Ajan sekmesi ya da kapsül/görev bağlamından alt sayfa.

**Akışın bir turu (OpenChart düzeyi, Ç1 maketi):**

1. Kullanıcı sorusu: küçük "Sen" etiketi, düz sol kenar.
2. Çalışma satırı canlı: "Grafiği okudu ✓ · Olguları sorguladı ✓ · Sicili okudu ✓ · …"; bitince
   **"Çalışma: 4 adım, 6 sn ›"** tek satır.
3. Cevap açık: ilk cümle sonuç ("Kapanış 113,05 ile 112,40'ın üstünde; üç barlık tutunmanın
   ikinci barı."); sayılar tablo rakamıyla, her sayı kanıta bağlı; sicil cümlesi Y33 kurallarıyla
   ("tutma %64, taban oran %51, n = 42").
4. Gerekiyorsa onay kartı (Y06) ve/veya soru kartı (Y05); "Grafikte" bağları.
5. Cevabın altında küçük bağlar: "Grafikte göster" · "İzle" · "Kanıt".

**Durumlar**

| Durum | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|
| Boş | Bağlamdan üç önerilen soru (tam cümle) + besteci (OV-19) | `--muted` | "112,40 kırılımı tutunur mu?" |
| Ajan çalışıyor | Canlı adım satırları, "Durdur" | `--qp-origin-agent` | "Olguları sorguluyor · 2 sn" |
| Cevap geldi | Katlı çalışma + açık cevap | `--fg` | — |
| Cevap doğrulanamadı | Cevap yayımlanmaz; neden satırı | `--qp-warn` | "Cevap yayımlanmadı: 1 sayı kanıtla uyuşmuyor" |
| Ajan soruyor | Y05 kartı bestecinin üstünde | `--qp-origin-agent` | "Soru 1 / 2" |
| Hata | Çalışma açık kalır, ✗ adım, "Yeniden dene" | `--qp-warn` | "Model yanıt vermedi · yeniden dene" |
| Ajan kapalı | `O-ajankapalı`, besteci devre dışı | `--muted` | "Ajan kapalı" |
| Sınır aşıldı | `O-sınır` | `--qp-warn` | "Bu ay 50 / 50 soru · haklar" |
| Tavsiye isteği | Ajan sinyal diliyle cevaplar (K77) | `--fg` + `--muted` ibare | "Bunu söyleyemem; RSI stratejisinin sinyali ve sicili şu: … · Sinyal, tavsiye değil" |

**Köken:** Soru insan (düz), çalışma ve cevap ajan (kesik), cevaptaki sayılar motor (noktalı
alt çizgiyle kanıt bağı). Ajanın grafiğe yazdığı her şey panelde köken rozetiyle bağlıdır.

**Veri durumu:** Başlıkta `D-*` sözcüğü; bayatta cevap "son veri 14:35'e göre" der; simülede başlıkta "simüle".

**Akış:** Yukarıdaki tur. Paneldeki her öğe "Grafikte" ile grafiğe iner; grafikte seçilen öğe besteciye "@" bağlamı olarak girer.

**Masaüstü / telefon:** Telefonda kapsül ya da görevden açılır, bağlam başlıkta sabit; adım satırları hep katlı.

**Uyarlanan:** OV-17, OV-18, OV-19, AT-C1, VV-A1, VV-A2, VV-A4. **Roller:** `--qp-origin-agent`,
`--qp-agent-dash`, `--qp-origin-human`, `--qp-origin-engine`, `--fg`, `--muted`, `--qp-warn`,
`--qp-font-figure`, `--card`, `--line`.
