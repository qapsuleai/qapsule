# R5B — Ajan Yüzeyi

## Neden

`NORTH_STAR.md`: grafik, insanın ve yapay zekânın aynı anda okuyup yazdığı canlı bir
dokümandır. Bugüne kadar insan tarafını kurduk. Ajan tarafında elimizde protokol (`qp/0`)
ve şemadan üretilen bir MCP uyarlayıcısı var (RD1); ama gerçek bir ajan, canlı bir
oturuma bağlanıp iş görmedi.

Baştan beri karar şu (K2, K3): yapay zekâ matematik hocası değildir. Hesap ve tespit
deterministik motorundur; ajan onu **okur, yorumlar, izler, önerir**. Ajan tek parça da
değildir: dinleyen, yorumlayan, haber veren, kişiye göre süzen ayrı roller olabilir.

Bu round ajanın Qapsule'e bağlandığı yüzeyi kurar ve ölçer. Ortam kararı verildi (K67):
ajanlar MCP ile bağlanır; aynı ajan bağlam için Matriks'in MCP'sini (`mcp.matriks.ai`:
temel veriler, haberler, kurum raporları) de kullanır.

## Temel ilkeler

1. **Motor söyler, ajan anlatır.** Ajan olgu, bulgu, kapsül ya da karne **üretemez**;
   bunlar yalnız deterministik motordan gelir. Ajanın yazabildikleri: **yorum** (bir
   kapsüle ya da enstrümana bağlı anlatı), **niyet** (kalıcı izleme isteği), **öneri**
   (dokümana eklenmesi teklif edilen öğe: çizgi, not, gösterge, görünüm). Hepsi kökeniyle
   (`origin`: hangi ajan, kimin adına) kayıtlıdır.
2. **Her cümle kanıtına bağlanır.** Ajanın yorumu, dayandığı olgu ve kanıt kimliklerini
   taşır. Motor yorumu kabul etmeden önce **deterministik olarak doğrular**: atıf yapılan
   olgular var mı, yorumdaki her sayı atıf yapılan kanıtta ya da adı verilen dış kaynakta
   geçiyor mu, tavsiye dili var mı (R5 dil taramaları), okuyucunun lisansı o sayıyı
   görmeye yetiyor mu. Doğrulanmayan yorum yayımlanmaz; neden reddedildiği ajana söylenir.
3. **Ajan da bir okuyucudur.** Lisans kuralı (K33) ajan için de geçerlidir: ajan, adına
   çalıştığı kişinin yetkisinden fazlasını göremez; lisanslı sayıyı yetkisiz okura
   taşıyamaz (girişimsizlik testleri ajan yolunda da geçer).
4. **Gözlem, tavsiye değil** (`docs/CAPSULE.md` §8). Ajanın yorumu "al", "sat", "fırsat" demez.
5. **Dış içerik veridir, talimat değil.** Haber metni, rapor özeti, araç çıktısı ajana
   komut veremez; başvuru ajanları buna göre kurulur ve bu sınanır.
6. **Ürün yolu deterministik kalır** (K3): niyetin eşleşmesi, bildirimin tetiklenmesi,
   yorumun doğrulanması motorda ve dil modelsizdir. Dil modeli yalnız ajanın içindedir.

## Kapsam

1. **Qapsule MCP sunucusu:** `qp mcp serve` (stdio ve HTTP), çalışan bir otorite
   oturumuna bağlı (hızlandırılmış benzetim, yeniden oynatma ya da canlı). Okuyucu kimliği
   ve yetkisi bağlantıda belirlenir. Yüzey:
   - **Oku:** akış ("son baktığımdan beri ne değişti"), kapsül ve hikâyesi, kanıt (yetkiye
     göre), karne, enstrümanın hikâye listesi, piyasa durumu, sözlük, anlamsal ekran okuması.
   - **İzle:** abonelik (kapsül doğdu / durum değiştirdi / kapandı) ve **niyet** ("X'te
     yukarı kırılım olursa haber ver", "elimdekilerde olumsuz bir şey olursa"); eşleşme
     motorda; bildirim MCP üzerinden.
   - **Yaz:** yorum (doğrulamalı), öneri (insan onayına düşer), görünüm kurma (R2'nin
     gramer komutları; ekransız çizim).
2. **Araç yüzeyinin tasarımı ölçümle seçilir** (M28): bugünkü, protokolü birebir yansıtan
   ince araçlar tek aday değildir.
3. **Yorum doğrulayıcı** (ilke 2) ve reddetme nedenlerinin ajana geri bildirimi.
4. **Niyet:** şeması, deterministik eşleşmesi, yaşam döngüsü (kuruldu, eşleşti, süresi
   doldu, kaldırıldı), hassasiyet seviyesiyle ilişkisi (K31), kişi başına sınır.
5. **Başvuru ajanları** (depoda örnek ve ölçüm düzeneği olarak; ürün değil):
   - **Yorumcu:** bir kapsülü, kanıtını ve Matriks MCP'sinden gelen bağlamı (ör. aynı gün
     haber, bilanço takvimi) birleştirip kısa, kaynaklı bir yorum yazar.
   - **Nöbetçi:** bir kişinin niyetlerini kurar, eşleşmeleri izler, yalnız anlamlı
     olduğunda haber verir (önem süzgeci; "her şeyi bildirme").
   - **Soru cevaplayan:** "ISCTR'de ne oluyor?", "bugün elimdekilerde ne değişti?" gibi
     soruları Qapsule ve Matriks araçlarıyla, kaynak göstererek cevaplar.
6. **Gösterimde en az iz:** açılmış kapsülde doğrulanmış ajan yorumu, kökeni ve "ajan
   yorumu" etiketiyle görünür; bekleyen öneriler listelenir ve onaylanıp reddedilebilir.
   Tasarım işi değildir (RD2); yalnız var olan görünüşle, sade.
7. **Bağlantı kılavuzu:** Claude Code, Claude masaüstü ve MCP konuşabilen başka bir
   ortamdan (QODI dahil) Qapsule MCP'sine nasıl bağlanılır.

## Kimlik ve gizlilik

- Matriks MCP kimliği `.env.local`'da: `MATRIKS_MCP_URL`, `MATRIKS_MCP_CLIENT_ID`,
  `MATRIKS_MCP_API_KEY` (başlıklar `X-Client-ID`, `X-API-Key`). Dosya açılmaz; değerler
  depoya, günlüğe, dökümlere, MCP yapılandırma dosyasına **yazılmaz** (yapılandırma ortam
  değişkenine başvurur; biçimi güncel belgeden doğrula). Sorgular Arkin'in lisansıyla koşar:
  ölçülü kullan, sorgu sayısını raporla.
- Dil modeli çağrıları Claude Code alt ajanlarıyla yapılır; ayrı anahtar gerekmez
  (`docs/MODELS.md`). Başka bir anahtar varsa kullanımı raporda; yoksa istenmez.
- Gerçek piyasa verisi içeren ajan dökümleri depoya girmez (`fixtures/local/` altında
  kalır); depodaki dökümler sentetik oturumdan üretilir. Rapora yalnız toplu sayı.
- Qapsule MCP sunucusu varsayılan olarak yalnız yerel makineyi dinler; ağdan erişim
  kapalıdır ve açılması açık bir seçenek ile kimlik ister.

## Kapsam dışı

- Ajanın emir göndermesi ya da herhangi bir işlem yapması. Bu yüzey yalnız okuma, izleme,
  yorum ve öneridir.
- Mobil kontrol kulesi ve bildirimin telefona ulaşması (R8). Kişiselleştirme ve sesli
  anlatım (R9). Tasarım (RD2). Profesyonel çalışma alanında insan–ajan ortak düzenleme (R7;
  bu round'un "öneri" yolu onun temelidir).
- Ajanın kendi karnesi (yorumlarının sonradan tutup tutmadığı): şema hazırlanır, ölçüm sonraya.

## Ölçümle karar verilecek konular (K39)

Düzenek, görev kümesi, doğru cevaplar ve puanlama kuralı alternatiflerden **önce** yazılır.
Doğru cevaplar deterministik motordan üretilir (ajan değerlendiricinin görüşü değil).

| # | Konu | Alternatif (en az) | Ölçüm |
|---|------|--------------------|-------|
| M28 | **Araç yüzeyi** | (a) protokolü yansıtan ince araçlar (bugünkü); (b) az sayıda görev düzeyinde araç ("ne değişti", "bunu açıkla", "şunu izle"); (c) tek sorgu aracı (küçük bir sorgu dili ya da seçici); (d) karma | 40+ görevlik küme (bul, açıkla, doğrula, karşılaştır, izle, öner, yetkisiz okur): doğruluk, araç çağrısı sayısı, belirteç ve süre, ilk denemede başarı; üç model boyunda (küçük model de kullanabilmeli); araç tanımlarının toplam boyutu |
| M29 | **Kapsülün ajana veriliş biçimi** | Tam JSON; sıkıştırılmış "kapsül kartı" metni; katmanlı (özet + isteğe bağlı kanıt) | Aynı görevlerde doğruluk ve belirteç; sayı aktarım hatası (ajanın yanlış yazdığı sayı) oranı |
| M30 | **Yorum doğrulayıcının sıkılığı** | Yalnız atıf varlığı; atıf + sayı denetimi; atıf + sayı + cümle başına atıf zorunluluğu | Bilerek bozulmuş yorumlarda yakalama oranı (uydurma sayı, yanlış yön, olmayan olgu, tavsiye dili, lisans sızıntısı); doğru yorumlarda yanlış ret oranı; ajanın düzeltip yeniden gönderme başarısı |
| M31 | **Nöbetçinin bildirim süzgeci** | Her eşleşme; önem + hassasiyet seviyesi; önem + tekrar bastırma + kişinin elindekiler | Benzetimde gün başına bildirim sayısı; kaçırılan önemli olay (elindeki hissede olumsuz geçiş: 0 olmalı, K42); gereksiz bildirim oranı |

**Sınır:** değerlendirilen de değerlendiren de aynı model ailesi olabilir; bu yüzden
puanlama mümkün olan her yerde deterministiktir (doğru cevap motordan, sayı denetimi
kodla). Dil modeliyle puanlanan tek şey yorumun okunurluğudur ve o seçimde belirleyici değildir.

## Kabul ölçütleri

| # | Ölçüt |
|---|-------|
| 1 | `qp mcp serve` çalışan bir oturuma bağlanıyor (benzetim / yeniden oynatma; canlı için hazır); stdio ve HTTP; varsayılan yalnız yerel. Araçlar şemadan üretiliyor (elle yazılmış ikinci şema yok). MCP uygunluk dökümleri depoda (sentetik). |
| 2 | Okuma yüzeyi eksiksiz: akış, kapsül, hikâye, kanıt, karne, hikâye listesi, piyasa durumu, sözlük, ekran okuması. Her cevap olgu ve kanıt kimlikleri taşıyor: **her öğe olguya izlenebiliyor** (rastgele 50 cevapta denetim). |
| 3 | İzleme: abonelik ve niyet; eşleşme motorda ve deterministik (aynı benzetim iki kez → aynı bildirimler, bayt eşit); niyetin yaşam döngüsü; kişi başına sınır. Protokol eki şartnamede, dökümleri var. |
| 4 | Yorum: doğrulayıcı ilke 2'nin dört denetimini yapıyor; M30 ölçülmüş; reddedilen yorumun nedeni makinece okunur; doğrulanmış yorum kökeniyle saklanıyor ve gösterimde etiketli görünüyor. |
| 5 | Öneri: ajan görünüm ve çizim öğesi öneriyor; insan onaylayana kadar dokümana girmiyor; onay ve ret komut yolundan; köken korunuyor. Ajan ekransız bir görünüm kurup görüntüsünü alabiliyor. |
| 6 | Lisans: ajan yolunda girişimsizlik testleri geçiyor; yetkisiz okur adına çalışan ajan lisanslı sayıyı hiçbir araçtan alamıyor ve yorumuna koyamıyor (doğrulayıcı reddediyor). |
| 7 | M28 ve M29: düzenek ve görev kümesi önce commit; alternatifler ölçülmüş; üç model boyunda tablo; seçim gerekçeli. Küçük modelin başarı oranı ayrıca raporlu. |
| 8 | Başvuru ajanları üçü de çalışıyor; görev kümesinde ölçülmüş: doğruluk, kaynak gösterme oranı, uydurma sayı (0 hedef; doğrulayıcıdan geçen yorumlarda 0 şart), tavsiye dili (0), ortalama araç çağrısı ve belirteç. |
| 9 | Matriks MCP'siyle bağlam: Yorumcu ve Soru cevaplayan, Matriks araçlarından gelen bilgiyi kaynağını adlandırarak kullanıyor; dış sayı "dış kaynak" diye ayrı işaretli; sorgu sayısı raporda. Dış içerikteki gömülü talimat sınaması: ajan uymuyor (en az 10 kurgulanmış örnek). |
| 10 | M31: nöbetçi süzgeci ölçülmüş; elindeki hissede olumsuz geçiş kaçırma 0. |
| 11 | Bağlantı kılavuzu: `docs/AGENTS.md` (Claude Code, Claude masaüstü, genel MCP istemcisi); Arkin için beş dakikalık deneme: kendi Claude'una Qapsule'ü bağlayıp üç soru sormak. |
| 12 | Var olan altınlar, dökümler ve hesap özeti değişmedi; `scripts/verify.sh` yeşil ve ağsız (ajan ölçümleri verify'ın parçası değil; doğrulayıcı ve MCP testleri parçası); CI yeşil; gizli bilgi taraması temiz. |
| 13 | **Kapanış temizliği (K68):** geçici dosyalar, ajan dökümü yığınları silindi; `target/` ≤ 6 GB; raporda silinenler, bırakılanlar, kalan boyutlar. |
| 14 | `REPORT.md` (ölçüm maliyeti dahil), `AUDIT.md`, `docs/AGENTS.md`, `docs/PROTOCOL.md` eki; `STATUS.md`, `ROADMAP.md`; önerilen kararlar ayrı bölümde. |
