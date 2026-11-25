# RL1 — Hesap dili

## Neden

K70: AI-native'in ikinci basamağı "ajan yönlendirir": neyin hesaplanacağını ve izleneceğini ajan
kurar, motor çalıştırır ve kanıtlar. Bugün ajan yalnız hazır beş göstergeyi parametreyle
çağırabiliyor. K79: ajanın ve profesyonelin yazdığı her hesap **tek bir tipli ara gösterime**
iner; aynı ara gösterim göstergeyi, koşulu, izleme koşulunu, strateji sinyalini ve durum
tanımını ifade eder.

Bu round o motoru Qapsule'e koyar. Seçim yapılmış durumda (X30; yeniden ölçülmez, K82): ara
gösterim + küçük metin dil. Şartname taslağı `../refs/experiments/rx4/SPEC.md` ve
`../refs/reports/COMPUTE-LANGUAGE.md` §8; metin dilin tarifi `../refs/experiments/rx4/surface-a/DOC.md`.
Deney kodu kopyalanmaz, şartname olarak alınır (K91).

RF1'den sonraki ilk özellik round'u: alt sistem sözleşmesinin gerçek sınavı. Çekirdek donuk.

## Sonuç ne olacak

1. İnsan ve ajan aynı yüzeyden bir hesap yazar; `check` bütün hataları ve maliyeti
   çalıştırmadan söyler; `add` dokümana ekler; motor koşar; her değer kanıtına izlenir.
2. Programın **seri** çıktısı grafikte çizilir ve başka koşullar onu okur; **olay** çıktısı
   yalnız kapanmış barda doğar ve canlı yolun koşulu olur.
3. **Sinyal** ve **durum tanımı** çıktıları yazılır, denetlenir ve yeniden oynatmada koşar;
   sicile bağlanmaları RS1'in işidir.

## Değişmezler

- Hesap kodunda model, ağ, saat, rastgelelik yok; yalnız IEEE temel işlemler ve `sqrt`. Aynı
  program + aynı veri → bayt eşit çıktı; toplu = akışkan; aarch64 = x86_64 = wasm32.
- **Geleceğe bakma yok:** ilk `n` barla koşunun çıktısı tam koşunun ilk `n` barına eşit
  (özellik testi). Olay, sinyal ve durum kaydı yalnız kapanmış barda doğar; canlı barda yalnız
  gösterim için geçici değer hesaplanır.
- Günlüğe yalnız ara gösterim yazılır; metin dil bir yazım yüzeyidir. Program kimliği kanonik
  biçimin özetidir.
- Dokümanı değiştiren her şey komut yolundan geçer. Ajanın eklediği program öneri kipine
  tabidir; kararı insan verir (R5B).
- Şemanın tek kaynağı `qp-schema`. Protokole **ekleme** yapılır; var olan mesaj, döküm, altın
  ve hesap özeti değişmez. Var olan `add_study` kalır.
- **Yapı:** `qp-authority`'ye satır eklenmez (şema kolunun zorunlu kıldığı tek "yumuşak" dosya
  dışında; `docs/ARCHITECTURE.md` "Yeni doküman nesnesi nasıl eklenir"). Yeni iş kendi
  crate'lerinde doğar ve bileşime kayıt satırıyla girer. `arch-check` yeşil, taban çizgisi
  büyümez, yeni istisna yok.
- Yeni dış bağımlılık yok (ayrıştırıcı elle yazılır); gerekiyorsa dur ve sor.
- K17 ve K52 tutar; WASM boyutu İ8 tavanının içinde (ön iş 1'den sonra).

## Kapsam

### 0. Ön işler (K93)

1. **Kabuk WASM paketinde küçültme ölçümü** (`wasm-opt` ya da derleme ayarı): önce/sonra boyut,
   açılış süresi, K52; kazanç varsa uygulanır ve İ8 tavanı yeni değere göre sıkılaştırılır.
   Altınlar ve dökümler değişmemeli.
2. **İzleyici tipi:** yansının komut yolu dışından yazan gizli yöntemleri (`follow_start`,
   `mirror_apply`, `mirror_commit`, `follow_data`, `follow_resync`, `take_provider`) ayrı bir
   tipe kapatılır; barındırıcılardan çağrılamaz. Davranış aynı (RF1 tutanağıyla bayt eşit).
3. Lint: `unimplemented`, `unreachable` açılır; mevcutlar öğe başına izinle, mandallı.
4. CI tam geçmişle klonlar; `config` adımı CI'da da koşar.

### 1. Şartname ve kavramlar (koddan önce)

- `docs/CONCEPTS.md`: program, hesap dili, ara gösterim, çıktı türleri (seri, olay, sinyal,
  durum tanımı), program parametresi, maliyet birimi, hesap bütçesi.
- `docs/PROGRAMS.md`: Qapsule'ün şartnamesi (`qp-ir/0`). RX4 taslağından farklar açıkça:
  üst periyot hizası, canlı barda geçici değer, parametreler, hata eşlemesi, bütçe.
- `docs/PROTOCOL.md`'ye yeni bölüm.

### 2. Ara gösterim, denetim, çalıştırıcı (`qp-lang`, alan katmanı)

- **Tipler** `qp-schema`'da: program (düğümler topolojik sırada, çıktılar), `num` / `bool` /
  `cat`, düğümün saati. Bilinmeyen alan reddedilir.
- **Düğümler:** girdi ve sabit; var olan çekirdekler (`sma ema rsi macd bollinger`) + yeni
  çekirdekler (`highest lowest atr stdev percentrank`) `qp-compute`'ta, aynı kurallarla;
  aritmetik; gecikme; karşılaştırma ve mantık; kesişim; durum düğümleri (`streak bars_since
  once rearm cooldown`); `defined`; üst periyot hizalama (`align`); çizgi (`line`: R7'nin çizim
  seviyeleri buradan bağlanacak); kutulama.
- **Parametreler** (K78, X30/9): programın adlı, tipli, aralıklı girdileri; tanım biçimi
  parametre kayıt defteriyle aynı. Aynı program farklı parametrelerle farklı örnektir; tarama
  ve yeniden kurma bu tanımdan çalışır.
- **Denetim (`check`):** tip, saat, başvuru, parametre, geleceğe bakma, maliyet. **Bütün hatalar
  birden**, makine okunur, yerini göstererek, en yakın ad önerisiyle. Qapsule'ün kapalı hata
  kodlarına eşlenir; ayrıntı alt kodda.
- **Çalıştırıcı:** barları tek tek iter; toplu ve akışkan bayt eşit; canlı bar için kopyasız
  geçici değer; çıktı imleci (belirli barın değeri, o barda doğan olaylar).
- **Üst periyot barları** enstrümanın borsa seansına hizalanır (`qp-calendar`); 7/24 piyasada
  UTC (X30/5). Üst periyot yalnız kapanmış bardan okunur.
- **Köken:** her yayımlanan değer (seri değeri, olay barı, sinyal kaydı) düğüme ve bağlı
  olduğu bar aralığına izlenir; var olan kanıt düzeniyle (`evidence_ref`) uyumlu.
- **Var olan hesaplarla eşdeğerlik:** beş `add_study` hesabının her biri, aynı çekirdeği
  kullanan program olarak bit eşit sonuç verir.
- **İç içe gösterge adresi** (K63, `ISCTR:5m:EMA[RSI:18]:20`) programa çevrilir. *(kesilebilir)*

### 3. Metin dil

- Küçük, satır tabanlı yazım yüzeyi; aynı ara gösterime iner. Döngü, kullanıcı işlevi,
  atamayla durum, çoklu sembol, emir **yok** (maliyetin statik, kökenin yapısal kalmasının şartı).
- Hata konumu satır, sütun, uzunluk. İstek ifade edilemiyorsa ajan program yerine **ret**
  yazabilir (kodu ve nedeniyle).
- İki yön: metin → ara gösterim ve ara gösterim → kanonik metin (insan, ajanın yazdığını okur).
- Türkçe ve İngilizce anahtar sözcük sorusu: tek kanonik sözcük kümesi (İngilizce işlev adları);
  açıklama ve hata iletileri iki dilde.

### 4. Maliyet ve hesap bütçesi

- Maliyet çalıştırmadan önce bilinir: düğüm türü, parametre ve bar sayısından.
- Sınırlar parametredir (K78): program başına düğüm ve pencere sınırı; kişi başına **toplam**
  canlı program maliyeti; yeniden oynatma bütçesi. Üst sınırlar satılabilir haktır.
- **Bütçe işi sayar** (RX5 güvenlik bulgusu, `../refs/docs/rounds/RX5/REPORT.md` §7/1): yalnız
  bar × maliyet değil, doğan kayıt, açık kayıt ve okuma da sayılır; aşınca çalışma durur ve
  nedenini söyler.
- Değerler barındırıcıdan gelir (`SessionView::param`, `entitled`); kalıcı profil saklama bu
  round'un işi değil.

### 5. Oturuma bağlanma (`qp-program`, oturum alt sistemi)

- İşlemler: program ekle, değiştir (parametre), kaldır; sorgular: denetle (`check_program`),
  dene (kısa özetle çalıştır), açıkla (kanonik metin + maliyet + çıktılar).
- `OpHandler` açılım ve politika: kim ekleyebilir, önerilebilir mi, ajan sınıfı, kamu dünyası,
  okuyucu gizliliği. Bütçe denetimi `Check` fazında.
- **Seri çıktısı** `ValueSource` kaydıyla veri düzlemine girer: gramer katmanı onu kaynak
  olarak çizer (lejantı çekirdek kurar), başka koşullar okur.
- **Olay çıktısı** `ConditionKind` kaydıyla canlı yola bağlanır (paralel bir yol kurulmaz):
  kesin barda doğar, bildirimi ve kanıtı var olan düzenle.
- **Sinyal ve durum tanımı:** denetlenir, "dene" ve yeniden oynatmada koşar, sonuç kuralı
  zorunludur; canlıda iddia doğurmaz (RS1). "Tut" kayıt doğurmaz (X30/8).
- Yeniden oynatma ve yansı: aynı günlükten aynı doküman; yansı tarafında gerekip gerekmediği
  ölçümle (WASM payı) kararlaştırılır ve gerekçesi yazılır.

### 6. Yüzeyler

- **MCP:** denetle, dene, ekle (öneri olarak), açıkla. Araç tanımı ince yüzey kuralına uyar
  (K86/6). Ajan belgeleri (`docs/AGENTS.md`) ve başvuru ajanı güncellenir.
- **Komut satırı:** `qp program check | run | explain` (dosyadan; metin ya da JSON).
- **Grafik:** program serisi var olan görünüşle (`tavek`) çizilir; köken çizgi biçimi kuralı
  geçerli (ajanın eklediği kesik). Program yazma ekranı bu round'da yok (R7).

### 7. Uygunluk ve ajan sınaması

- **RX4 uygunluğu:** `../refs/experiments/rx4/harness` görev çözümleri Qapsule'ün motorunda
  koşar ve beklenen çıktılarla karşılaştırılır (UTC hizalı kipte). Farklar ya hata ya da
  şartnamede yazılı bilinçli sapmadır.
- **Ajan yazabilirlik teyidi** (seçim değil, gerçek uygulamanın sınaması; bir kez): RX4 görev
  kümesinden 20 görev, MCP üstünden, küçük ve büyük modelle. Ölçülen: ilk denemede doğru,
  hata iletisinden sonra düzeltme, ret gereken yerde ret, uydurma işlev adı. RX4 sonuçlarının
  belirgin altındaysa nedenini bul; yüzeyi yeniden seçme.
- **Kötü niyet:** bütçeyi aşmaya, geleceğe bakmaya, günlüğü şişirmeye çalışan programlar
  reddediliyor ya da duruyor (ayrı bir denetçi alt ajan dener).

## Zaman kuralı (K82)

Sıkışırsan şu sırayla **kes ve raporda yaz:** (1) iç içe gösterge adresi, (2) kutulama ve durum
tanımı çıktısı (şartnamede kalır, RS1'e), (3) ara gösterim → kanonik metin yönü yalnız okunur
özet olur, (4) `percentrank` ve `stdev`, (5) küçük modelle ajan teyidi. Ön işler 1–2, şartname,
denetim ve hata modeli, çalıştırıcı ve belirlenimcilik sınamaları, metin dil, bütçe, seri ve
olay çıktısının oturuma bağlanması, MCP ve RX4 uygunluğu kesilemez.

## Kabul ölçütleri

1. Ön işler: kabuk paketi ölçümü ve sonucu; izleyici tipi (RF1 tutanağı bayt eşit); lint'ler; CI.
2. Kavramlar sözlükte; `docs/PROGRAMS.md` ve `PROTOCOL.md` bölümü koddan önce commit.
3. Tipler `qp-schema`'da; şema dışa aktarımına eklenen yalnız yeni tipler; var olan dökümler,
   altınlar ve hesap özeti bayt eşit.
4. `check` bütün hataları birden, konumuyla ve öneriyle döndürüyor; hata kodu başına sınama.
5. Belirlenimcilik: iki koşu bayt eşit; toplu = akışkan; üç mimaride özet eşit; yeni çekirdekler dahil.
6. Geleceğe bakma: önek kararlılığı özellik testi (rastgele program, veri, `n`); olay yalnız kesin barda.
7. Beş `add_study` hesabı program olarak bit eşit.
8. Üst periyot seansa hizalı (XIST günlük ve 4 saatlik örnekleri; 7/24'te UTC); sınaması var.
9. Parametreli program: tanım, bağlama, aralık denetimi, aynı programın iki örneği.
10. Metin dil ↔ ara gösterim: gidiş-dönüş sınaması; hata konumu; ret biçimi.
11. Maliyet çalıştırmadan önce; bütçe işi sayıyor; kişi başı toplam sınır `Check` fazında;
    sınırlar parametre kayıt defterinde.
12. Oturum: ekle / değiştir / kaldır / denetle / dene; seri grafikte ve başka koşulda; olay
    canlı yolda kesin barda; yeniden oynatma bayt eşit; ajan eklemesi öneri olarak.
13. `qp-authority/src` farkı: yalnız belgede adı geçen yumuşak dosya (ya da boş); yeni
    istisna yok; `arch-check` yeşil.
14. MCP araçları ve `qp program` komutları; ajan belgeleri güncel.
15. RX4 uygunluğu: sonuç tablosu; her fark açıklanmış.
16. Ajan teyidi sonuçları; kötü niyet denemeleri.
17. K17, K52, WASM boyutu (İ8 içinde); program değerlendirmesinin tik maliyeti ölçüldü.
18. Bağımsız denetçi `AUDIT.md`; `REPORT.md`, `STATUS.md`, `ROADMAP.md`; kapanış temizliği.
