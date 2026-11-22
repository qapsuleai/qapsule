# RD2 — Tasarım turu: tek görsel dil, profesyonel görünüm

## Neden

K57: R5'in tasarım seçimleri kabul edilmedi. Arkin: "şekilleri ve yapıyı beğenmedim; profesyonel
bir görünüme uygun değil." R5 okunabilirliği ölçtü; "profesyonel görünüm ve güven" ölçütü yoktu
ve ajanla ölçülemiyor (K57 "Ders"). K58: kapsül görünümü ile profesyonel çalışma alanı tek
turda, tek dille, gerçek veriyle tasarlanır; son söz insandadır. K70: profesyonelin oyun alanı
kalır; alışılmış grafik dili alfabedir (`NORTH_STAR.md` "Ne değiliz").

`../refs/reports/VISUAL.md` §3.2 farkın nedenlerini sayıyor: sözleşmeye uyum, renk disiplini,
yoğunluk ve ölçek, hizalama ve çakışma, biçim dili, sunum biçimi. Bu round o farkı kapatır.

## Sonuç ne olacak

1. **Tasarım sözleşmesi** (`docs/design/CONTRACT.md`): kısa, kural odaklı; roller ve değerler
   tek kaynaktan (çekirdekteki görünüş kaydı). İki profil: **masaüstü (yoğun)** ve **telefon**.
2. **Seçilmiş yön**, çekirdekte görünüş kaydı olarak; grafiğin alışılmış öğeleri gramerde.
3. **Yüzey şartnamesi** (`docs/design/SURFACES.md`) ve **maketler** (`docs/design/mockups/`):
   R7, RS1, RA1 ve R8'in kuracağı yüzeylerin görünüşü ve durumları. Bu round o yüzeylerin
   davranışını kurmaz.

## Değişmezler

- Çizim, etkileşim ve veri mantığı çekirdektedir; kabukta metin kurulmaz, aritmetik yapılmaz.
  Maketler şartnamedir, ürün kodu değildir: durum değiştiren birkaç satır dışında mantık taşımaz.
- Protokol (`qp/0`), şema ve MCP yüzeyi değişmez. Protokol varsayılanı görünüş aynı kalır;
  yeni görünüş **gösterim varsayılanı** olur (Ö69, Ö79 kalıbı), o da Arkin'in son kabulünden sonra.
- Var olan altın görseller ve dökümler güncellenmez. Yeni altın yalnız yeni görünüm için.
- Gerçek veri yalnız `fixtures/local/` altında; ondan üretilen görsel commit edilmez. Karşılaştırma
  galerisi yerelde gerçek veriyle çalışır; depoya giren kanıt görselleri kayıtlı sentetik veriyle.
- Dış projelerden (`../refs/src`) kod, CSS ve değer tablosu kopyalanmaz (K83); roller ve ölçüler
  temiz odada yeniden kurulur. Başvuru ekran görüntüleri depoya girmez.
- **Yapı (K89):** `qp-session`'a net satır eklenmez. `qp-grammar` test dışı en çok 21 bin satır.
  `scripts/arch-check` yeşil, borç taban çizgisi büyümez.
- K52 telefon bütçesi ve K17 hesap bütçesi tutar. WASM boyutu raporlanır (hedef ≤ +%5).

## Yöntem: önce uyarla, sonra farklılaş (Arkin: "Amerika'yı yeniden keşfetmeyelim")

İyi uygulanmış arayüz kalıpları yeniden icat edilmez; incelenmiş projelerden **uyarlanır**.
Özgün tasarım yalnız Qapsule'e özgü katmanda yapılır: kapsül, köken ayrımı, çizimden niyete,
kur ve göster, sicil.

- **Kaynaklar** (hepsi `../refs` içinde; ayrıca görsel beklenmez): `reports/VISUAL.md`
  (OpenChart'ın tasarım sözleşmesi, yerleşimi, tipografisi, renk rolleri, grafik stili, ajan
  paneli), `reports/PATTERNS.md`, `reports/repos/*.md` ve `src/` altındaki arayüz kodu ile
  ekran görüntüleri: OpenChart, Vela, Vibe-Trading, valuecell, OpenAlice, trading-command-center,
  OpenTerminalUI, nofx. Başvuru ekranı olarak bu projelerin kendi görselleri kullanılır.
- **Uyarlama envanteri** (`docs/design/ADOPTED.md`; ilk iş, yönlerden önce): her kalıp için bir
  uyarlama kartı (K83): ne, hangi projede, neden iyi çalışıyor, Qapsule'de nereye oturuyor
  (çekirdek / kabuk), bizde ne değişiyor. En az şunlar: tasarım sözleşmesinin biçimi, rol
  tabanlı renk (krom nötr, kroma yalnız veride), ajan yazımını ayıran renk rolü, tip ve boşluk
  ölçeği, eksen ve lejant düzeni, panel yerleşimi ve yeniden boyutlama, ajan paneli (çalışma
  katlanır, cevap açık; soru kartı), uyarı listesi ve sağlık göstergesi, çizim araç çubuğu,
  izleme listesi, komut paleti, boş ve hata durumları.
- **Temiz oda sınırı (K83):** kalıp, ölçü mantığı ve rol adları uyarlanır; kaynak kod, CSS dosyası
  ve değer tablosu kopyalanmaz. Lisansı AGPL ya da ticari olmayan projelerden yalnız fikir
  düzeyinde yararlanılır (`../refs/reports/LICENSES.md`).
- **Tasarım becerileri:** round başında oturumdaki tasarım becerilerini listele (arayüz tasarımı,
  veri görselleştirme ve benzeri) ve maket, sözleşme ve grafik işlerinde kullan; hangisini
  nerede kullandığını rapora yaz. Uygun beceri yoksa bunu round başında rapora yaz ve devam et.
- **Proje becerisi (çıktı):** `.claude/skills/qapsule-design/SKILL.md`. Sözleşmeden üretilir;
  sonraki round'ların (R7, RS1, RA1, R8) her alt ajanı arayüz işinden önce onu yükler: roller,
  ölçek, yoğunluk profilleri, köken gösterimi, durum kalıpları, yapılmayacaklar, yeni yüzey
  eklerken kontrol listesi. Değerleri elle taşımaz; görünüş kaydına ve `CONTRACT.md`'ye işaret eder.

## Kapsam

### 0. Ön işler (küçük; ilk gün)

- **A62:** eski `qp mcp --init --principal` bağdaştırıcısı kaldırılır; "karar yalnız insandan"
  testi (`crates/qp-mcp/tests/human_decision.rs`) bu yolun yokluğunu da sınar. İlgili döküm ve
  belge güncellenir; kaldırılan yüzey raporda adıyla.
- **İ7:** `qp_render` yazı tipi tabloları `const` → `static`; kabuk paketi boyutu önce/sonra.
- (Başvuru görseli için ayrı klasör yok; kaynaklar `../refs` içinde.)
- **Silmeler (K87):** `recovered/worktree-agent/*` 24 dal (önce `rounds/RG1/experiments/recovered.md`
  ile son kez karşılaştır; `r5/m16-bertan`, `r5/m16-kivra` kalır), `rounds/R3/experiments/harness/target/`.
- İstisna değerlerinin mandalı (`arch-check`: İ3–İ5 değerleri büyüyemez; `qp-grammar` 21 bin).

### 1. Tasarım sözleşmesi ve otomatik ölçütler

- `docs/design/CONTRACT.md`: renk rolleri (krom nötr, veri, anlam, **köken**: insan / ajan / motor),
  tip ölçeği, boşluk, yarıçap, kenar, yoğunluk, hareket; her kural bir cümle ve bir değer.
- `scripts/design-check` (ya da çekirdekte test): `../refs/reports/VISUAL.md` §5'teki P1–P13
  görünüş kaydından ve sahne tarifinden otomatik ölçülür. Eşikler başlangıçtır; Arkin'in ara
  kararından sonra bir kez ayarlanır ve sözleşmeye yazılır.
- **İki profil:** masaüstünde yoğun (gövde 13–14 px, grafik 11–12 px, imleç hedefi), telefonda
  dokunma (44 px hedef korunur). R5'in sert eşiklerinden karşıtlık ve renk körlüğü ayrımı iki
  profilde de kalır; en küçük yazı ve hedef boyu profile göre.

### 2. Üç yön, dört çapa ekran, Arkin'in ara kararı

Üç yön de aynı **uyarlanmış alfabeyi** kullanır (`ADOPTED.md`): grafik dili, yerleşim iskeleti ve
rol tabanlı renk ortaktır. Yönler yalnız Qapsule'e özgü katmanda ayrışır:

- **Yön A — sözleşmeye sadık:** uyarlanan kalıp neredeyse olduğu gibi; kapsül ve ajan katmanı
  en sessiz haliyle (hap, kenar paneli).
- **Yön B ve C:** aynı taban; kapsülün, kökenin ve izleme durumunun grafikte ve akışta nasıl
  öne çıktığı farklı (alt ajanlar kendi önerisini getirir; birbirini görmeden).

Her yön dört çapa ekranda, iki temada (koyu, açık) gösterilir:

| Çapa | İçerik |
|---|---|
| Ç1 | Masaüstü çalışma alanı 1440×900: grafik (mum, hacim, iki gösterge), izleme listesi, izleme görevleri, ajan paneli, durum şeridi |
| Ç2 | Grafik hücresi yakın: trend çizgisi + niyet kartı + izleme işareti + ajan önerisi + kapsül işareti |
| Ç3 | Telefon kontrol kulesi 390×844: kapsül akışı, bekleyen öneri, izleme özeti |
| Ç4 | Telefonda grafik ve açılmış kapsül |

Karşılaştırma kuralları (P14): aynı veri, aynı boyut, masaüstü DPR 2, telefon DPR 3; başvuru
ekranı yan yana; R5'in `oltan` görünüşü de aynı düzenekte. Grafik çizimi çekirdekten (görünüş
kaydı + gramer seçenekleri); çevredeki paneller maket.

**Ara teslim:** galeri (`scripts/demo.sh --gallery` genişletilir) ve tek sayfalık karşılaştırma
hazır olunca **dur ve Arkin'e sor.** Arkin bir yön seçer, karıştırır ya da hepsini reddeder
(o zaman gerekçesiyle bir tur daha; en çok iki tur). Ajan eleştirmeni yalnız yardımcıdır;
puanı seçim gerekçesi olamaz.

### 3. Seçilen yönün çekirdeğe işlenmesi

- Görünüş kaydı (iki profil, iki tema) ve `css_vars`.
- Grafiğin alışılmış öğeleri gramerde: sağ fiyat ekseni ve etiket yoğunluğu (P4), iki düzeyli
  zaman ekseni (P5), çakışma önleme (P6), son değer etiketi ve değer çizgisi, imleç ve eksen
  hapları, OHLC lejantı (metni çekirdek kurar), alt bölmede hacim (P7), veri üstü işaret boyu (P8).
- Kapsül yüzü, akış, açılmış kapsül ve hikâye çizelgesi (`korda`, K57) aynı dille.
- Web kabuğu var olan ekranlarda yeni değerleri uygular. Yeni altınlar yalnız yeni görünümler için.

### 4. Yüzey şartnamesi ve maketler (R7, RS1, RA1, R8 için)

Kaynak: `../refs/reports/PLAYGROUND.md` §5 (Y01–Y30) ve aşağıdaki ekler. Her yüzey için:
ne, yerleşim, **bütün durumlar**, köken ve veri durumu gösterimi, masaüstü / telefon farkı.

Sonradan verilen kararlardan doğan ek yüzeyler:

| No | Yüzey | Karar |
|---|---|---|
| Y31 | Sessiz zil ve üstüne gelince açıklama; izlemenin sağlık göstergesi | K72, K73 |
| Y32 | İki aşamalı kırılım: kırıldı → tutundu / geri döndü; periyot yazısı | K73 |
| Y33 | Tutma yüzdesi ve taban oran; "düşük güven" etiketi; canlı ↔ yeniden oynatma ayrımı | K75, K86 |
| Y34 | Zaman gezintisi: o aralıktaki gerçekleşmiş ve gerçekleşmemiş tespitler | K75 |
| Y35 | Hazırlık dosyası; ortak analiz ve üyelik (üye sayısı); "sinyal, tavsiye değil" ibaresi | K76, K77 |
| Y36 | Kurumsal işlemde "beklemede" izleme ve taşıma mesajı | K88 |
| Y37 | Kullanım hakları ve parametre kaynağı ("neden 3? → sistem varsayılanı") | K78 |

Üç kademe (yalınlık, K82):

- **Kademe 1 — tam maket, bütün durumlar, iki profil:** Y01, Y04, Y05, Y06, Y07, Y09, Y11, Y12,
  Y18, Y25, Y26, Y27, Y31, Y32, Y33.
- **Kademe 2 — ana durumun maketi + durum listesi:** Y02, Y03, Y08, Y10, Y13, Y14, Y16, Y17,
  Y20, Y21, Y23, Y24, Y34, Y35, Y36.
- **Kademe 3 — yalnız şartname:** Y15, Y19, Y22, Y28, Y29, Y30, Y37.

`PLAYGROUND.md` §6'daki sorulardan tasarımı etkileyip cevapsız kalanlar için en olası
varsayımı yap, şartnamede işaretle ve raporda "Arkin'e sorular"a yaz; bunun için durma.

### 5. Proje tasarım becerisi

`.claude/skills/qapsule-design/SKILL.md` (yukarıdaki "Yöntem"). Sınaması: beceriyi yükleyen
taze bir alt ajana kademe 3'ten bir yüzeyin maketi yaptırılır; `design-check`'ten geçiyor ve
sözleşmeyle çelişmiyor.

### 6. İnsan testi paketi (v4)

P15: Arkin ve en az iki işlem yapan kullanıcı; 5 saniyelik bakış, "profesyonel araç mı"
(evet/hayır), "güvenir miyim" (1–5); yanında başvuru ekranı. Paket `rounds/RD2/human-test/`.
Round paketi hazırlar; testi Arkin yürütür. Round, Arkin'in kendi cevabıyla kapanır; öbür
kullanıcıların sonucu sonradan eklenir.

## Zaman kuralı (K82)

Aynı ölçüm tekrar edilmez: R5'in okunabilirlik, karşıtlık ve renk körlüğü düzeneği yalnız
seçilen yönde, bir kez koşar. Sıkışırsan şu sırayla **kes ve raporda yaz:** (1) kademe 3,
(2) açık tema maketleri (görünüş kaydında açık tema kalır), (3) kademe 2'de telefon profili,
(4) hikâye çizelgesinin yeniden çizimi. Uyarlama envanteri, sözleşme, üç yön, ara karar, seçilen
yönün çekirdeğe işlenmesi, kademe 1 ve proje becerisi kesilemez.

## Kabul ölçütleri

1. Ön işler bitti: eski bağdaştırıcı yok ve testi var; İ7 düzeltildi (boyut önce/sonra);
   24 dal ve R3 deney derlemesi silindi; istisna mandalı çalışıyor.
2. `docs/design/ADOPTED.md` (uyarlama kartları, yönlerden önce commit) ve `docs/design/CONTRACT.md`
   yazıldı; her değer görünüş kaydından geliyor (elle kopya yok); dış kaynaktan kopyalanmış kod,
   CSS ya da değer tablosu yok. Proje becerisi `.claude/skills/qapsule-design/` yazıldı ve sınandı.
3. `design-check` P1–P13'ü ölçüyor, `verify.sh` adımı; seçilen yön hepsini geçiyor ya da
   geçmediği ölçüt Arkin'in onayıyla sözleşmede gerekçeli.
4. Üç yön dört çapa ekranda, kurallara uygun karşılaştırmayla sunuldu; Arkin'in ara kararı
   `rounds/RD2/DECISION.md`'de kendi sözleriyle.
5. Seçilen yön çekirdekte görünüş kaydı; iki profil, iki tema.
6. Alışılmış grafik öğeleri gramerde (P4–P8); çakışma 0.
7. Kapsül yüzü, akış, açılmış kapsül ve hikâye çizelgesi aynı dille.
8. `docs/design/SURFACES.md`: Y01–Y37, her birinde durumlar, köken ve veri durumu.
9. Kademe 1 maketleri tam; kademe 2 ana durum; maketler değerleri `css_vars`'tan alıyor.
10. Seçilen yönde karşıtlık ve renk körlüğü eşikleri iki profilde tutuyor.
11. K52 ve K17 tutuyor; WASM boyutu raporda.
12. Var olan altınlar, dökümler ve hesap özeti değişmedi; yeni altınlar listeli.
13. `qp-session` net +0; `qp-grammar` ≤ 21 bin; `arch-check` yeşil; taban çizgisi büyümedi.
14. İnsan testi v4 paketi hazır; Arkin'in son kabulü `DECISION.md`'de ("profesyonel ve güvenilir").
15. Bağımsız denetçi `AUDIT.md`; `REPORT.md`, `STATUS.md`, `ROADMAP.md`; kapanış temizliği.
