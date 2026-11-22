# RG1 — Yönetişim Turu: kurallar ve mevcut kodun yeniden düzenlenmesi

## Neden

K80: yapı da kabul ölçütüdür. K81: kuralları yalnız sonraki işlere uygulamak yetmez; mevcut
kod yeniden düzenlenmeden üstüne yeni iş kurulmaz. Ölçüm (2026-11-22): depo ≈ 135 bin satır
Rust; `qp-session` 32 bin satır, tek düz klasörde; `qp-schema/src/protocol.rs` 4.916 satır;
3 bin satırı geçen beş dosya; "ince" olması gereken `qp-cli` 14 bin, `qp-wasm` 11 bin satır;
`DECISIONS.md` tek dosyada 100 KB'a yaklaştı. Round'lar davranışı ve ölçümü denetledi, yapıyı
denetleyen kural yoktu.

Bu round **yeni özellik eklemez**. Kuralları ve otomatik denetimi depoya koyar, mevcut kodu
`docs/ENGINEERING.md`'ye göre yeniden düzenler ve parametre kayıt defterinin iskeletini açar.
Sonraki round'lar (RD2, RL1, R7, RS1) temiz zemine oturur.

## Değişmez: davranış değişmez

Bu round'un en önemli kuralı. Round başında alınan **başlangıç tutanağı** ile round sonu
karşılaştırılır; hepsi **bayt eşit** olmalı:

- bütün altın görseller (`fixtures/golden/**`), görünüm açıklamaları (`*.explain.txt`);
- hesap özeti (`fixtures/compute/digest.txt`; aarch64, x86_64, wasm32);
- `qp/0` uygunluk dökümleri üç taşımada (süreç içi, `qp serve`, MCP), R5B dökümleri dahil;
- yansı ve görüntüleyici bantları, yeniden oynatma satır akışı (`verify.sh` 15–16);
- dışa aktarılan şema (`qp schema export`) ve MCP araç tanımları;
- R5B ölçüm düzeneğinin puanlayıcı testleri ve `agents/test_score.py`.

Protokol (`qp/0`), şema, komut satırı arayüzü ve MCP yüzeyi **değişmez**. Performans bütçeleri
(K17 hesap, K52 telefon) round öncesine göre kötüleşmez (ölçüm gürültüsü içinde); WASM boyutu
±%2. Bir düzenleme bunlardan birini bozuyorsa geri alınır ya da dur ve sor.

## Zaman kuralı (K82)

Yalın. Hedef borcu sıfırlamak değil, kesin sınırları tutturmak ve kalanını görünür kılmak.
Sıkışırsan şu sırayla **kes ve raporda yaz**: (1) veri tablolarının koddan ayrılması (ölçüt
8), (2) test klasörlerinin konuya göre düzenlenmesi, (3) lint sıkılaştırmasında mevcut
ihlallerin düzeltilmesi (taban çizgisine yazılır, yenisi yasaklanır). Katman yönü, kabukların
inceltilmesi, 800 satır sınırı ve `arch-check` kesilemez.

## Kapsam

### 1. Kurallar ve otomatik denetim

- `docs/ENGINEERING.md` bu round'la yürürlüğe girer (metni değiştirme; eksik ya da
  uygulanamaz bulduğun kuralı raporda "Kural önerileri"ne yaz).
- **`scripts/arch-check`** (depo geneli; `verify.sh`'ın yeni adımı): boyut bütçeleri (dosya,
  fonksiyon, klasördeki dosya sayısı, crate, kabuk crate'i, test dosyası, belge), katman
  bağımlılık yönü (`cargo metadata` üstünden), yasak dosya adları (`util`, `common`, `misc`,
  `helpers`), crate ve modül `README.md` varlığı, `lib.rs`/`mod.rs`'te mantık olmaması.
  Kendi kendini sınar (bilinen ihlal örnekleri yakalanıyor).
- **Borç taban çizgisi** (`docs/DEBT.md` + makine okunur `scripts/arch-baseline.json`):
  round sonunda kalan her aşım adıyla, nedeni ve hedefiyle; `arch-check` taban çizgisindeki
  sayıdan **fazlasına** hata verir (mandal: yalnız azalabilir).
- Lint: `unwrap_used`, `expect_used`, `panic`, `too_many_lines`, `cognitive_complexity`
  kütüphane kodunda açılır; mevcut ihlaller ya düzeltilir ya taban çizgisine yazılır.

### 2. Yeniden düzenleme (öncelik sırasıyla)

1. **Kabukların inceltilmesi.** `qp-cli` ve `qp-wasm` içindeki alan ve barındırma mantığı
   (ör. `livehost.rs`, `live.rs`, `demo.rs`, `mcp_serve.rs`, `shell.rs`) uygun katmana taşınır
   (uyarlayıcı ya da alan crate'i; gerekirse yeni crate). Kabukta yalnız bağlama kalır.
2. **`qp-session`'ın konu sınırlarından bölünmesi** (klasörler; bütçe gerektiriyorsa crate'ler:
   ör. otorite/komut yolu, kapsül yaşamı, ajan yüzeyi, yansı). Sınırları sen öner, gerekçesini yaz.
3. **`qp-schema/protocol.rs`'in mesaj ailesine göre modüllere ayrılması**; dışa açık tip
   yolları korunur (yeniden dışa aktarma), üretilen şema bayt eşit.
4. **800 satırı geçen bütün kaynak dosyalarının bölünmesi** (üretilmiş dosyalar hariç).
5. **Katman yönü ihlallerinin giderilmesi** (`docs/ENGINEERING.md` §2).
6. Bir klasörde 15'ten çok kaynak dosya kalmaması; `util`/`misc` türü dosyaların dağıtılması.
7. Test klasörlerinin konuya göre düzenlenmesi (kesilebilir).
8. Veri tablolarının (kapsül dili sözlükleri, görünüş kayıtları) koddan ayrılması (kesilebilir).
9. Her crate'e kısa `README.md`: ne işe yarar, katmanı, bağımlılıkları, dışa açtığı, ne değildir.

Yöntem: **taşı, yeniden yazma.** `git mv` + mekanik düzenleme; mantık, ad ve davranış
değişmez. Bir taşıma sırasında hata fark edersen düzeltme, raporda "Fark edilenler"e yaz.

### 3. Parametre kayıt defterinin iskeleti (K78)

- `qp-schema` içinde parametre tanım tipi (anahtar, tip, birim, aralık, varsayılan, kapsam,
  kim değiştirir, hak mı, açıklama, sürüm) ve çözümleme sırası (nesne > kullanıcı > plan >
  sistem); belgesi tiplerden üretilir.
- **Envanter:** koddaki davranış sabitlerinin taraması (eşikler, süreler, sınırlar,
  varsayılanlar; ör. nöbetçi süzgeci, karne eşiği 20, yorum doğrulayıcı sıkılığı, hassasiyet
  seviyeleri, kişi başına sınırlar). `docs/PARAMETERS.md`: her sabit için bugün nerede,
  önerilen anahtar, kapsam.
- **Bağlama:** en az 10 sabit kayıt defterinden okunur hale gelir (davranış aynı; varsayılan
  = eski sabit). Kalanı borç listesinde. Kullanıcı ya da plan kapsamında değer saklama
  (profil) bu round'un işi değildir; yalnız tanım, sistem varsayılanı ve okuma yolu.

### 4. Karar kaydının bölünmesi

- `DECISIONS.md` belge bütçesini aşıyor: `docs/decisions/` altında bölümlere ayrılır (verilmiş
  kararlar, açık kararlar, round önerileri round başına), kökte kısa bir dizin kalır.
  **K, Ö, A numaraları ve metinleri değişmez**; depo içindeki bütün başvurular çalışır kalır.
- `STATUS.md` round geçmişi tablosu uzunsa aynı biçimde arşivlenir.

### 5. R5B artıkları (küçük)

- `recovered/worktree-agent/*` 24 dal: her birinin `main`'e göre farkını özetle (dosya sayısı,
  birleşmemiş commit var mı); **silme**, raporda Arkin'e karar listesi ver.
- `qp-source` yerel bar yolunun derleme zamanı yerine çalışma dizininden çözülmesi (R5B §6):
  davranış değişikliği sayılır; yalnız raporda öner.

## Kapsam dışı

- Yeni özellik, yeni protokol mesajı, yeni araç, tasarım değişikliği.
- Hesap dili, çizim, sicil, ortak analiz (sonraki round'lar).
- Profil ve hak saklama, ödeme, kullanıcı yönetimi.
- R5B ve RP4 "Önerilen" kararlarının uygulanması (ana akışta karara bağlanacak).
- `NORTH_STAR.md` değişikliği.

## Kabul ölçütleri

| # | Ölçüt |
|---|-------|
| 1 | **Başlangıç tutanağı** (bütün altınlar, özetler, dökümler, şema, MCP araç tanımları, performans ve WASM boyutu, satır sayıları) round'un ilk işi olarak alındı ve commit edildi (`rounds/RG1/experiments/baseline/`). |
| 2 | **Davranış değişmedi:** round sonunda tutanaktaki her kalem bayt eşit; `scripts/verify.sh` yeşil; CI yeşil. Tek bir altın, döküm ya da özet güncellenmedi. |
| 3 | Performans (K17, K52 ölçümleri) ve WASM boyutu tutanağa göre sınırlar içinde; tablo raporda. |
| 4 | `scripts/arch-check` depo genelinde çalışıyor, `verify.sh` adımı; kendi kendini sınıyor; katman yönü ihlali **0**. |
| 5 | 800 satırı geçen kaynak dosya **0** (üretilmiş dosyalar hariç ve işaretli). |
| 6 | Kabuk crate'leri (`qp-cli`, `qp-wasm`) 4 bin satırın altında **ya da** kalan kısım için gerekçeli istisna isteği raporda (Arkin onayına). |
| 7 | 12 bin satırı geçen crate yok **ya da** gerekçeli istisna isteği; bir klasörde 15'ten çok kaynak dosya yok; `util`/`common`/`misc`/`helpers` adlı dosya yok. |
| 8 | Her crate'te `README.md`; `lib.rs`/`mod.rs` yalnız bildirim ve dışa aktarma. |
| 9 | Borç taban çizgisi yazıldı (`docs/DEBT.md`, `scripts/arch-baseline.json`); mandal çalışıyor (taban çizgisinin üstüne çıkan değişiklik `arch-check`'te hata veriyor; testi var). |
| 10 | Lint'ler açık; yeni ihlal derlemeyi kırıyor; mevcut ihlaller düzeltildi ya da taban çizgisinde sayısıyla. |
| 11 | Parametre kayıt defteri: tanım tipi, üretilen belge, `docs/PARAMETERS.md` envanteri, en az 10 sabit kayıt defterinden okunuyor ve davranış aynı (ölçüt 2). |
| 12 | `DECISIONS.md` bölündü; numaralar ve metinler değişmedi (otomatik karşılaştırma: bölünmeden önceki ve sonraki satır kümesi eşit); depo içi başvurular çalışıyor. |
| 13 | `recovered/*` dalları için karar listesi; "Fark edilenler" ve "Kural önerileri" bölümleri. |
| 14 | Bağımsız denetçi: tutanak karşılaştırmasını kendisi yeniden koşar; rastgele 20 taşımada "yalnız taşıma mı, mantık değişmiş mi" inceler; `arch-check`'i kandırmayı dener; `AUDIT.md`. |
| 15 | Kapanış temizliği (K68): worktree'ler kaldırıldı, `target/` ≤ 6 GB, silinenler / bırakılanlar / kalan boyutlar raporda. `REPORT.md`, `STATUS.md`, `ROADMAP.md`. |
