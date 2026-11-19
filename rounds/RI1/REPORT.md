# RI1 — Rapor: Matriks bağlantısı ve Quantex servisleri (keşif)

Tarih: 2026-11-19. Tür: salt okunur keşif; Qapsule koduna, testlerine ve altınlarına
dokunulmadı. İncelenen depo: `quantex` (`/Users/arkns/LocalGit/quantex`, dal `main`,
HEAD `f3d95aea`).

## Özet

- Altı belge `docs/matriks/` altında: `CONNECTIVITY.md` (A), `MESSAGES.md` (B), `NATS.md` (C),
  `INDICATORS.md` (D), `ANOMALY.md` (E), `INTEGRATION.md` (F). Her iddia `services/...:satır`
  ya da Qapsule `crates/...`, `docs/...` kaynağıyla; varsayımlar ayrı işaretli.
- **Arkin'in beş bilgisi:** 2 doğrulandı (uç nokta başına tek bağlantı; ikinci bağlantı
  birincisini düşürür, sunucu kuralı). 1, 3, 4, 5 kısmen: belirteç kimliği sisteme değil
  örneğe (`hardwareId`) bağlı; "önce en geniş hal" yalnız market iletileri için; açıklık
  `mx/timestamp`'te ama yalnız Pay Piyasası ve zaman UTC epoch µs; göstergeler NATS'tan
  abonelikle ama motorlar iç Mosquitto'ya yazıyor, "çapraz gösterge" adıyla ayrı abonelik
  yok, anomali tür başına değil sembol başına birleşik olay (`docs/matriks/INTEGRATION.md` §2).
- **Matriks iletilerinde sıra numarası yok, QoS 0;** Quantex tüketicileri eksik alanı 0
  sayabiliyor; iç NATS yayını kalıcı değil (JetStream kodda kullanılmıyor).
- **Quantex gösterge servisleri Qapsule'e referans olamaz:** formül tanımları büyük ölçüde
  aynı (SMA, EMA, RSI Wilder, ATR, MACD kararlı bölge), ama bar hizası UTC epoch'a (K14'e
  aykırı), ısınmada 0 (K13'e aykırı), 300 bar tavanı, Bollinger `bandwidth` ×100, düz seride
  RSI 100 ↔ 50, tik zamanı varış anı. Akış ve derinlik sinyallerinin çoğu kod okumasına göre
  tetiklenmiyor (yer tutucu parametreler çözülmüyor, soğuma bitmiyor).
- **Anomali motoru deterministik değil** (rastgele kimlik, duvar saati, iş parçacığı sırası,
  çalışırken değişen eşik/ağırlık, dolaylı LLM geri bildirimi) ve eşik taşımıyor; çıktısı
  olduğu gibi sunucu olgusu sayılamaz. Kuralları Qapsule dedektörü olarak yeniden yazılabilir.
- **R6 önerisi (karar değil):** kayıt-önce (d1) → Qapsule'ün kendi kimliğiyle doğrudan
  Matriks bağlantısı (a); anomali bağlamı sonraya. Ön koşul: Matriks'ten ayrı kimlik ve
  mümkünse test/gecikmeli uç nokta, lisans cevabı.

## Ne yapıldı

| Görev | Yürüten | Çıktı |
|---|---|---|
| T0 Hazırlık | ana akış | Ana akış doküman değişiklikleri commit edildi (`c5fb2aa`, "ana akış: K57–K59, RI1 round paketi…"); alt ajan denemesi başarılı; quantex okunabildi; `rounds/RI1/experiments/MAP.md`; quantex `git status` kaydı `experiments/quantex_status_before.txt` |
| T1 Bağlantı | alt ajan | `docs/matriks/CONNECTIVITY.md` |
| T2 Mesajlar | alt ajan | `docs/matriks/MESSAGES.md` |
| T3 NATS | alt ajan | `docs/matriks/NATS.md` |
| T4a/b/c Göstergeler | üç alt ajan | parçalar ana akışta `docs/matriks/INDICATORS.md` içinde birleştirildi (özet ve fark tablosu eklendi) |
| T5 Anomali | alt ajan | `docs/matriks/ANOMALY.md` |
| T6 Bütünleştirme | ana akış | `docs/matriks/INTEGRATION.md` |
| A Denetim | bağımsız alt ajan | `rounds/RI1/AUDIT.md` |

Alt ajanlar başlatılabildi; her biri yalnız kendi belgesine yazdı (TASKS notu). Kesin sınırlar
her alt ajana aynen iletildi.

## Kabul ölçütleri

9/9 ✅. Kanıtlar `rounds/RI1/GOAL.md` "Kabul ölçütleri" tablosunun "Kanıt" sütununda.

| # | Ölçüt (kısa) | Durum |
|---|---|---|
| 1 | Altı belge; her soru cevaplı ya da "bilinmiyor + kime" | ✅ |
| 2 | Kaynak satırları; varsayımlar ayrı; beş bilgiye hüküm | ✅ |
| 3 | Proto kataloğu ve lisans sınıfları | ✅ |
| 4 | Gösterge listesi ve `qp-compute` fark tablosu | ✅ |
| 5 | Anomali türleri, anlam türü eşlemesi, determinizm | ✅ |
| 6 | R6 seçenekleri, deneme yolları, gerekçeli öneri | ✅ (denetim koşulu: (d2) deneme yolu sonradan eklendi) |
| 7 | Sınırlar: quantex değişmedi, ağ yok, tarama temiz | ✅ |
| 8 | Bağımsız denetim ≥ 40 iddia | ✅ 54 iddia |
| 9 | REPORT, STATUS, ROADMAP; kod değişmedi | ✅ |

Denetim hükmü: **koşullu kabul**; üç koşul (altı kısmen doğru iddianın düzeltilmesi, (d2)
deneme yolu, REPORT) karşılandı (`rounds/RI1/AUDIT.md` "Düzeltmeler").

## Ne yapılamadı / sınırlar

- Hiçbir davranış çalıştırılarak doğrulanmadı (kural gereği). "Koddan çıkarım" diye
  işaretli kusurlar (ör. sinyallerin tetiklenmemesi, eksik alanın 0 olması) canlı kayıtla
  ya da Quantex ekibiyle doğrulanmalı.
- protobufjs `defaults: true` davranışı kitaplık belgesine dayanan varsayım; denenmedi
  (quantex'te çalıştırma yasak, ağ yok).
- Üretim ortam değerleri (`.env`, k8s Secret, üretim NATS yapılandırması) okunmadı; kopya
  sayıları, `hardwareId` değerleri, etkin bayraklar bilinmiyor.
- Matriks'in belgesi quantex deposundaki bir worktree kopyasından okundu
  (`MESSAGES.md` `MD/` öneki); güncelliği bilinmiyor.

## Ölçümler

| Ölçü | Değer |
|---|---|
| Belge | 6 (`docs/matriks/`), toplam ≈ 3.300 satır |
| Proto kataloğu | 22 Matriks protosu; 13'ü Quantex'te çözülüyor, 9'u kullanılmıyor |
| Fiyat göstergesi | 30 sınıf (36 ad), 44 hazır sinyal, 27 formasyon |
| Akış / derinlik göstergesi | 6 / 5; sinyal 12 / 7 |
| Anomali dedektörü | 25 (+ rejim) |
| Gösterge fark tablosu | 7 satır (SMA, EMA, RSI, MACD, Bollinger, ATR, Donchian/en yüksek–en düşük) + 4 girdi farkı |
| Denetim | 54 iddia: 48 doğru, 6 kısmen, 0 yanlış, 0 doğrulanamadı; sınır taraması 0 bulgu |
| quantex `git status --porcelain` | round öncesi ve sonrası aynı (3 satır, round öncesinden: iki `.env` değişikliği ve izlenmeyen bir servis klasörü; içerikleri açılmadı) |

## Gömülü gizli değerler (yalnız yer; değer yazılmadı)

- `services/quantex-stream/docker/docker-compose.scale-test.yml:78,81,112,113` — JWT ve HMAC
  gizli anahtarı (yük testi yığını); `:159` — Grafana yönetici parolası.
- `services/quantex-stream/matriks-user-adapter/src/services/userPool.ts:652-654` — yorumda
  gerçek olabilecek bir Matriks kullanıcı kimliği (`sub`).
- `services/quantex-anomaly-engine/src/services/settlement_service.hpp:426-429` — hata ayıklama
  günlüğüne belirtecin ilk karakterleri yazılıyor (değer değil, davranış); `:413` üretim dışı
  TLS doğrulaması kapalı.
- Parolasız kullanıcı adları birkaç docker-compose dosyasında (`CONNECTIVITY.md` Ek).

Matriks servis hesabı, belirteç ve MQTT kimliği için gömülü değer görülmedi.

## Açık sorular

Ayrıntılı listeler belgelerin sonundadır: `CONNECTIVITY.md` §7 (13), `MESSAGES.md` §9 (14),
`NATS.md` §10 (8), `INDICATORS.md` (8 + 8 + 7), `ANOMALY.md` §8 (9). En önemlileri aşağıda
"Arkin'e ve Matriks ekiplerine sorular" bölümünde kişiye göre toplandı.

## Öneriler (kapsam dışı; yapılmadı)

1. **R6 kapsam taslağı** (`INTEGRATION.md` §8): ön koşullar (ayrı Matriks kimliği, test uç
   noktası, lisans cevabı) → Matriks çözücü (`qp-source`, alan varlığını koruyan) → K14 hizalı
   canlı bar üretici → bağlantı (uç nokta başına tek, `MULTICONNECT`'te dur, kopmada `stale`
   + REST'ten yeniden çözüm, yerel `seq`) → ham ileti kaydı ve `qp-source::replay` ile oynatma
   → kısa canlı deneme → gerçek yetki adlarıyla okuyucuya göre izdüşüm. Kapsam dışı: Quantex
   gösterge tüketimi, anomali bağlamı, derinlik olguları.
2. **Sözlük adayları** (CONCEPTS'e önce eklenmeli): "emir defteri / derinlik" kanıt ailesi
   (ad çakışması: `Depth` zoom derinliği); "seans evresi" (`stockStatus`); "Matriks kimliği"
   (servis hesabı + `hardwareId`).
3. **Quantex dedektör kurallarının taşınması** (R6 sonrası): PGC_ACCEL, PGC_DIVERGENCE,
   AGENT_SHIFT, BLOCK_TRADE, OFI → bar hizalı, deterministik Qapsule dedektörleri;
   lisanslı olanlar sunucu olgusu + mühür.
4. **Quantex ekibine iletilebilecek kusurlar** (Qapsule kapsamı dışı; bilgi için): eksik
   alanın 0 yazılması, gösterge bar hizası, `Period::parse` 5m'ye düşme, parametrelerin
   tam sayıya kesilmesi, sinyal yer tutucularının çözülmemesi, soğumanın bitmemesi, anomali
   ad uyuşmazlığı, `matriks-adapter`'in kopyalar arasında belirteç paylaşması, NATS'ta izin
   yokluğu. Bunlar Arkin'in kararıyla iletilir.
5. **DECISIONS önerisi** (yazılmadı, ana akışa): "R6 veri yolu" açık kararı — seçenekler
   (a)/(b)/(c)/(d1–d3), öneri (d1)→(a); "Qapsule için ayrı Matriks kimliği" ön koşulu.

## Disk (K56)

Worktree açılmadı (yalnız belge); derleme yapılmadı. `scripts/clean.sh` round sonunda
çalıştı (silinecek bir şey yoktu). Kalan boyut: `target/` 15 G (önceki round'lardan),
`.claude/worktrees/` 28 K, depo toplamı 15 G. Geçici dosya bırakılmadı.

---

## Arkin'e ve Matriks ekiplerine sorular

### Arkin'e

1. Qapsule için Matriks'ten **ayrı kimlik** (servis hesabı / `typeCode` / `hardwareId`) istenebilir mi; yoksa Quantex hesabı farklı `hardwareId` ile mi kullanılacak? (Ön koşul; `CONNECTIVITY.md` §3.)
2. Bilgi 5'teki **"çapraz gösterge"** neyi kastediyor: iç içe gösterge (`SMA[RSI:14]`), kesişim sinyali, yoksa iki sembol arası ilişki mi? (`NATS.md` §9.)
3. **R6 veri yolu** için tercih: (d1)→(a) önerisi uygun mu; Quantex NATS'ı (b) yalnız yedek mi kalsın? (`INTEGRATION.md` §4.)
4. Quantex kimliğiyle alınmış verinin Qapsule kullanıcılarına gösterilmesinin lisans durumu ne; Matriks'e kim soracak? (`MESSAGES.md` S9.)
5. Derinlik/emir defteri için yeni **kanıt ailesi** açılsın mı (sözlük kararı)? (`ANOMALY.md` soru 4.)
6. `matriks-user-adapter` ve **mqlaw** (kullanıcı adına belirteç alan ayrı depo) Qapsule'ün kişi başına veri yetkisi için kullanılabilir mi? (`NATS.md` §10; `CONNECTIVITY.md` §3.1.)
7. Quantex'te bulunan kusurlar (Öneriler 4) Quantex ekibine iletilsin mi?
8. Anomali sonuç geçmişi (`anomaly_outcomes`, 180 gün) Qapsule karnesiyle karşılaştırma için okunabilir mi? (`ANOMALY.md` soru 8.)

### Matriks veri ekibine

1. Çoklu bağlantı kuralının anahtarı nedir: belirteç, JWT `sub`, `hardwareId`, servis hesabı? Aynı `hardwareId` ile yeni alınmış ikinci belirteç eskisini düşürür mü? (`CONNECTIVITY.md` soru 1–2.)
2. Bir kullanıcının kendi Matriks uygulaması ile sunucuda o kullanıcı adına açılan bağlantı aynı uç noktada çakışır mı? (soru 7.)
3. Süresi dolan belirteçle açık MQTT bağlantısı kesilir mi? Bağlantı başına abonelik sayısı / hız sınırı var mı; toplu SUBSCRIBE destekleniyor mu? (soru 4–5.)
4. Geliştirme için **test ya da gecikmeli uç nokta** ve test kimliği verilebilir mi? (soru 13.)
5. `publishReason = REFRESH` ne zaman gelir; yeniden abonelikte ilk görüntü yeniden gelir mi; kayıp güncellemeyi anlamanın yolu (sıra, sürüm) var mı? (`MESSAGES.md` S1–S2.)
6. Zaman alanlarının birimi ve dayanağı: `TimeMessage.timestamp` µs mi ms mi; işlem/derinlik/PGC `timestamp` gece yarısından mı epoch mu; `isBistPPOpen` hangi seans evrelerinde true? (S4–S5.)
7. `mx/symbol/<SYM>` ile `@lvl2` farkı (ve `trade`, `depth` için); derinlikte ilk tablo nasıl işaretlenir? (S6; `CONNECTIVITY.md` soru 6.)
8. `incrementalQuantity` ne (adet mi, son iletiden beri mi); canlı hacmin TL karşılığı hangi alan? (S3.)
9. `pgc5/pgc8/pgc10` tanımı (ilk N kurumun net akışı mı, birikimli mi, birim) ve PGC10/PGC15 farkı. (`ANOMALY.md` soru 2; `INDICATORS.md` akış soruları.)
10. BIST sembollerinde lot ve para birimi nereden; fiyat adımı için canlı `priceStep` mi `price-step-v2.json` mı esas? (S10.)
11. `derived-bar` 240 dk ve haftalık barların hizası; kapanış seansının türetilmiş periyotlarda ele alınışı. (S8.)
12. Matriks `.proto` dosyalarının Qapsule'de kullanılması için resmi kaynak ve lisans (kopya yerine Matriks'ten alınmış dosya).

### Matriks lisans / hukuk ekibine

1. Veri sınıfı başına **gösterim** ve **gösterimsiz kullanımdan hesaplanmış yorum** hakkı (K33'ün dayanağı); hiçbir belgede bu ayrım yok. (S9.)
2. Derinlik (10 / 25 kademe), alıcı/satıcısız işlem tiki, AKDE (PGC, kurum bazlı), takas, AINEWS sınıflarının lisans kodları ve yetkisiz kullanıcıya rakamsız yorum serbestliği. (`ANOMALY.md` soru 3; S14.)
3. Bir sistem kimliğiyle alınan verinin birden çok son kullanıcıya gösterilmesi koşulları.

### Quantex ekibine

1. Üretimde `MATRIKS_SERVICE_HARDWARE_ID` dolu mu; anomali işçileri aynı değeri mi alıyor; `matriks-adapter` kopya sayısı ve `DATA_TYPES`? (`CONNECTIVITY.md` soru 8–9.)
2. Üretim NATS'ında hesap/izin ve JetStream tanımlı mı; Qapsule'e salt okuma kullanıcısı verilebilir mi? (`NATS.md` §10.)
3. Akış ve derinlik sinyallerinin (yer tutucu, soğuma) ve fiyat kesişim sinyallerinin üretimde tetiklenip tetiklenmediği; kapsayıcı saat dilimi. (`INDICATORS.md` soruları.)
4. Anomali dağıtım bayrakları (`CLICKHOUSE_ENABLED`, `AI_ENRICHMENT_ENABLED`, `AUTO_APPLY`, `REGIME_LATENT_ENABLED`) ve gösterge kuralı ad uyuşmazlığı. (`ANOMALY.md` soru 1, 5, 9.)
5. Geçidin önbellekten "snapshot" diye delta gönderebilmesi ve `streamx.symbol.refresh` yan etkileri bilinçli mi? (`NATS.md` §10.)
