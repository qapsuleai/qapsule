# RF1 — Oturumun ve gramerin bölünmesi

## Neden

K89: `qp-session`'ın bölünmesi ayrı bir round'dur ve atlanmaz. RG1 kodu taşıdı ama gövdelere
dokunamadığı için üç crate bütçenin üstünde kaldı (`qp-session` 32,0 bin, `qp-grammar` 21,0 bin,
`qp-capsule` 13,2 bin; bütçe 12 bin). RD2 `qp-grammar`'ı tavana getirdi: 20.964 / 21.000.
Sıradaki round'lar (RL1 hesap dili, R7 oyun alanı, RS1 sicil, RA1 ortak analiz) en çok bu iki
crate'e yük bindirecek; bugünkü haliyle ilk gün tavana çarparlar.

Bu round **yeni özellik eklemez.** RG1'den farkı: fonksiyon gövdeleri ve tip sınırları
değişebilir. Değişmeyen şey davranıştır.

## Değişmez: davranış değişmez

Round başında yeni bir **başlangıç tutanağı** alınır (RG1 düzeneği; RD2'nin getirdikleri dahil:
`rd2-*` altınları, `design-check` çıktısı, `design-contract --check`, tasarım galerisi dökümü).
Round sonunda her kalem **bayt eşit.** Protokol (`qp/0`), şema, komut satırı ve MCP yüzeyi
değişmez. Hiçbir altın, döküm ya da özet güncellenmez. Performans (K17, K52) gürültü içinde
aynı; WASM boyutu ±%2 (crate bölmesi kopya gömme yaratabilir; RG1 İ7 dersi).

Rust'ın dışa açık yolları (`qp_session::…`) değişebilir; değişenler raporda listelenir.

## Sonraki round'ların yükü (sınırları buna göre çiz)

| Round | Oturuma ve gramere ne ekleyecek |
|---|---|
| RL1 | Program nesnesi: `add_program` / `check_program` komutları, maliyet bütçesi, programın canlı değerlendirmesi |
| R7 | Çizim nesneleri, çizim → koşul → olay, izleme görevleri ve yaşam döngüsü, niyet ve netleştirme, kur ve göster; grafikte yeni işaretler (seviye etiketi, kırılım çentiği, kanıt bandı, niyet etiketi) |
| RS1 | Tek sicil (iddia + sonuç), yeniden oynatma kaydı, tutma yüzdesi, kurumsal işlemde taşıma (K88) |
| RA1 | Hazırlık görevi, ortak analiz ve üyelik, zaman aralığı okuması |

Kabul testi şudur: bu dördünün her biri **kendi crate'inde doğabilmeli**; otoriteye yalnız
kayıt satırı eklemeli, `Session`'ın gövdesine yeni alan ya da yöntem eklememeli.

## Kapsam

### 0. Ön işler (küçük)

- Ana akış belge değişikliklerini kaydet (`.claude/skills/qapsule-design/SKILL.md` dahil).
- RD2 denetim koşulu K1: P8'in çizilen değeri ölçülsün (işaret taşıyan **ayrı** bir tasarım
  fikstürü; var olan fikstür ve altınlar değişmez), P9 `design-check`'e bağlansın.
- Depo dışında `$TMPDIR` altındaki üç eski Playwright klasörü (14 ve 16 Kasım; RD2 raporu §17)
  silinir (Arkin onayladı). Başka hiçbir depo dışı klasöre dokunma.

### 1. Araçlar (K87)

- RG1'in taşıma araçları kalıcı olur: `scripts/refactor/` (tutanak, karşılaştırma, öğe parmak
  izi); `use`, `impl` başlığı, öğe düzeyi makro ve `cfg` denetimi eklenir.
- `arch-check`: aynı katmanda **bildirilmiş, döngüsüz sıra** (K-2; İ2 istisnaları kapanır);
  sayım ve üretilmiş dosya kuralları metindeki gibi; `verify.sh`'ta `main`'e göre
  `arch-config.json` / `arch-baseline.json` farkını basan adım (K-9); fonksiyon boyu
  istisnalarına tavan.
- Lint: `unimplemented`, `unreachable` açılır; mevcutlar öğe başına izinle, mandallı. *(kesilebilir)*
- `arch-check`'in sözdizimi ağacına geçmesi (K-10). *(kesilebilir)*

### 2. Bölme planı ve karar noktası

`rounds/RF1/PLAN.md` kod değişmeden önce yazılır: bugünkü bağımlılık grafiği (modül ve tip
düzeyinde, ölçülmüş), hedef crate'ler ve katman içi sıraları, her alt sistemin durumu ve
sahibi, otoriteyle arasındaki arayüz, taşıma ve yeniden yazma sırası, her adımın riski ve geri
dönüşü, "yeni alt sistem nasıl eklenir" taslağı. İki bağımsız alt ajan eleştirir (biri yukarıdaki
yük tablosuna göre: RL1, R7, RS1, RA1 bu sınırlarla nereye oturur).

**Karar noktası:** plan ve eleştirilere cevaplar yazılınca **dur ve Arkin'e sor.** Arkin planı
ana akış oturumuna gösterir; onay ya da düzeltme `rounds/RF1/DECISION.md`'ye yazılır. Onaydan
önce yalnız bölüm 0, 1 ve 4 (ses crate'i; saf taşıma) yapılabilir.

### 3. `qp-grammar` (İ4) — ilk sırada

- Çekirdek derleyici ile tasarım klasörleri (faces, open, stories, glyph, look, feeds,
  transitions) arasındaki adla kayıt döngüsü **enjeksiyona** çevrilir: derleyici kaydı
  dışarıdan alır, tasarım tarafını tanımaz.
- Tasarım tarafı ayrı crate(ler)e çıkar. Yeni kavram adı gerekiyorsa önce `docs/CONCEPTS.md`.
- 885 satırlık `prepare` adlandırılmış adımlara bölünür (İ6).
- Hedef: `qp-grammar` ≤ 12 bin ve R7'nin grafik işaretlerinin gireceği crate'te en az 3 bin
  satır pay.

### 4. `qp-capsule` (İ5)

`voice/` ayrı crate (`qp-voice`; saf taşıma). `qp-capsule` ≤ 12 bin.

### 5. `qp-session` (İ3) — asıl iş

- `Session` ince bir **otorite** olur: komut yolu, günlük, saat, bağlantılar, kabul ve yerleşim.
- Bugün `impl Session` bloklarıyla 18 dosyaya dağılmış işler **kendi durumunu taşıyan alt
  sistemlere** ayrılır (aday sınırlar bugünkü konu klasörleri: işlem genişletme ve seçiciler;
  kapsül yaşamı; iddia, itiraz, yorum; izleme niyeti; veri düzlemi; okuma ve sunum; yansı ve
  görüntüleyici; barındırma ve uygunluk). Sınırları ölçümle sen belirle.
- Alt sistemler otoriteyi tanımaz; ihtiyaç duyduklarını dar bir arayüzle alır. Otorite alt
  sistemleri bir kayıt üstünden çağırır (komut, sorgu, olay, saat adımı).
- 905 satırlık `expand_op` adlandırılmış adımlara bölünür (İ6).
- Hedef: 12 bin üstü crate kalmaz. Kalıyorsa en çok **bir** adlı istisna, tavanı bugünkünden
  küçük, gerekçesi ölçümle; kararı Arkin verir.
- **Ek noktası belgesi ve sınaması:** `docs/ARCHITECTURE.md`'de "Yeni alt sistem nasıl eklenir"
  (kayıt, komut, sorgu, olay, günlük, yeniden oynatma, yansı). Sınama: yalnız testte yaşayan
  örnek bir alt sistem, otorite crate'ine tek satır eklenmeden kaydolur, komut alır, günlüğe
  yazar ve yeniden oynatmada aynı sonucu verir.

### 6. Küçükler

- `qp-source`: dosya okuma uyarlayıcıya ayrılır; `qp-source` alan katmanında kalır, İ1 istisna
  olmaktan çıkar. *(kesilebilir)*
- RG1'in geçici yeniden dışa aktarmaları, dokunulan crate'lerde kaldırılır. *(kesilebilir)*
- RG1 "Fark edilenler"deki eskimiş belge yorumları, dokunulan dosyalarda düzeltilir.

## Yöntem

- **Sıra:** tutanak → araçlar → plan → karar noktası → gramer → ses → oturum → küçükler.
- Her adım kendi dalında; birleştirmeden önce ayrı denetçi; kesin hüküm iş başına ayrı derleme
  klasöründe; her birleştirmeden sonra tutanak karşılaştırması `main`'de.
- Gövde değişen yerde (enjeksiyon, alt sistem sınırı, iki dev fonksiyon) önce **karakterizasyon
  sınaması** yazılır: bugünkü çıktıyı kilitleyen, tutanaktan daha ince taneli test (ör.
  `expand_op` için işlem türü başına, `prepare` için katman türü başına). Sonra yeniden yazılır.
- Aynı crate'e aynı anda iki alt ajan yazmaz. Oturumun bölünmesi dalgalar halinde: önce
  yapraklar ve tepeler, sonra güçlü bağlı bileşen alt sistem alt sistem.
- Taşınabilen taşınır (öğe parmak izi "değişmedi"); yeniden yazılan her öğe raporda sayılır.

## Zaman kuralı (K82)

Hedef üç istisnayı kapatmak ve sonraki dört round'a yer açmak; borcu sıfırlamak değil. Uzun
fonksiyon borcu (177 kalem) yalnız dokunulan yerde azalır. Sıkışırsan şu sırayla **kes ve
raporda yaz:** (1) `arch-check`'in sözdizimi ağacına geçmesi, (2) `qp-source` dosya okuması,
(3) geçici yeniden dışa aktarmalar, (4) yeni lint'ler. Tutanak, plan ve karar noktası, gramer,
ses, oturumun bölünmesi, iki dev fonksiyon, sıralı katman kuralı ve ek noktası kesilemez.

## Kabul ölçütleri

1. Başlangıç tutanağı ilk iş, commit; RD2 kalemleri dahil.
2. Round sonunda tutanak bayt eşit; `verify.sh` yeşil; CI yeşil; hiçbir altın, döküm, özet,
   şema ya da araç tanımı değişmedi.
3. K17 ve K52 gürültü içinde; WASM (varsayılan ve kabuk paketi) ±%2.
4. Ön işler: P8 çizilen ölçülüyor, P9 `design-check`'te; eski Playwright klasörleri silindi.
5. `scripts/refactor/` kalıcı; `arch-check` sıralı katman kuralını uyguluyor; yapılandırma
   farkı adımı `verify.sh`'ta; öz-sınama güncel.
6. `PLAN.md` iki eleştiriyle; `DECISION.md`'de onay; kod değişikliği onaydan sonra.
7. `qp-grammar` ≤ 12 bin; kayıt enjeksiyonla; tasarım tarafı ayrı crate'te; R7 için ≥ 3 bin pay.
8. `qp-capsule` ≤ 12 bin; `qp-voice` ayrı.
9. `qp-session` bölündü: 12 bin üstü crate yok ya da tek adlı istisna (tavan < 32 bin, gerekçeli).
10. `expand_op` ve `prepare` bölündü; 800 üstü kaynak dosya 0 (üretilmiş tablo dışında).
11. İstisnalar: İ2–İ6 kapandı ya da küçüldü; yeni istisna yok (varsa Arkin'in kararıyla).
12. Ek noktası belgesi ve örnek alt sistem sınaması geçiyor.
13. Borç taban çizgisi büyümedi; kalem sayısı ve en uzun fonksiyonlar önce/sonra raporda.
14. Yeni ve değişen crate'lerde README; `docs/ARCHITECTURE.md` ve `docs/ENGINEERING.md` §2
    katman tablosu güncel; dışa açık Rust yolu değişiklikleri listeli.
15. Bağımsız denetçi `AUDIT.md` (tutanağı kendisi koşar; yeniden yazılan öğelerden örneklem;
    ek noktasını kendi örneğiyle dener); `REPORT.md`, `STATUS.md`, `ROADMAP.md`; kapanış temizliği.
