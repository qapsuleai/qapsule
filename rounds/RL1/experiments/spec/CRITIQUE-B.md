# RL1 şartname eleştirisi B — R7 ve RS1 gözüyle

Taslak: `docs/PROGRAMS.md` (PR), PROTOCOL §26, CONCEPTS/PARAMETERS farkı. Hüküm: dil çekirdeği R7/RS1'i taşır;
**kimlik, zaman ve kayıt alanları** eksik — bugün eklemek ucuz, sonra eklemek günlüğü ve kimliği kırar.

## Bulgular
1. **ENGEL — Geriye dönük tetiklenme; örneğin "başlangıç anı" yok.** PR §13.2, §26.2 `add_program`/`set_program_params`.
   Koşul serinin son değerine bakar. Ekleme ya da parametre değişikliği anında son kapanmış bar olay barıysa
   `threshold` hemen doğru olur; iddianın `t`'si örnek **var olmadan önce** kapanmış bir bardır. R7'de seviyeyi
   fiyatın altına sürüklemek geçmiş bir bar için "kırıldı" doğurur; RS1'de kuraldan önce doğmuş kayıt çıkar
   (as-of ihlali, sicil şişirme yolu). Sonradan düzeltmek, var olan günlüklerin yeniden oynatmada farklı
   bildirim vermesi demektir. **Düzeltme (şimdi, eklemeli):** `program.since_t`, açılımda otoritenin yazdığı
   (`owner` gibi) bağlı serinin son barının `t`'si; belirlenimci, günlükten gelir. Kural: olay ve sinyal yalnız
   `t > since_t` barında iddia ya da kayıt doğurur, öncesi yalnız tarihtir (çizilir, `dene`de görünür).
   `set_program_params` `since_t`'yi yeniler.
2. **ÖNEMLİ — Parametre değişikliği geçmişi yeniden yazıyor, kanıt doğrulanamıyor.** PR §2.1, §13.1; PROTOCOL §19.2.
   Seviye düzenlemesi seri kimliğini korur (iyi: izlemeler kopmaz), ama seri baştan kurulur. Eski seviyeyle doğmuş
   iddiaların `values`'u yeniden üretilemez. `verify_evidence` bunu "veri değişti" sanar, "tanım değişti"yi
   ayıramaz; kanıtta tanımın sürümü yok. **Düzeltme (şimdi):** `program`'a otoritenin yazdığı `program_id` ve
   `instance_id` (özet). RS1/R7 notu: `ClaimEvidence`'a eklemeli `program {instance_id}`; parametre
   değişikliğinden önceki iddialar yeniden sınanmaz, "tanım değişti" diye işaretlenir.
3. **ÖNEMLİ — Kişisel izleme programı dokümanda herkese açık.** PR §13.6 ("add_study gibi"); PROTOCOL §25.5 gizlilik.
   R7 izleme görevi = program + niyet. İzleme kişisel veridir, ama program serisi (çapalar, stop seviyesi) her
   okuyucuya gider; stop pozisyonu ele verir (X28, K76). **Düzeltme (şimdi):** `program.scope` (bu round yalnız
   `"document"`; yokken o anlam); R7 `"personal"`ı `watch_visible` kuralıyla ekler. Bir cümle: "RL1'de
   pozisyondan türeyen seviye programa yazılmaz".
4. **ÖNEMLİ — Kapanış "sonraki bar gelince"; dokunma koşulu seans sonunda bir gün gecikir.** PR §1, §13.2 ile §3.2 farkı.
   Üst bar takvim kapanışıyla kapanır, taban bar ancak yeni bar gelince. Seansın son dakikasındaki stop dokunuşu
   (X30/3, en küçük periyot) ertesi sabah bildirilir. RX5 `born_t` = barın kapanışı, PR'de iddia `t`'si = barın
   açılışı: RS1 as-of iki ayrı zaman ister. **Düzeltme:** kayıt ve özet biçimlerine şimdiden `known_t` (kapatan
   verinin zamanı; takvim kapanışından ayrı). R7 notu: kapanışın veri düzleminden gelen, günlükte duran seans
   kapanışı olayıyla da tetiklenmesi.
5. **ÖNEMLİ — Sinyal kaydı RS1'in istediği kimlikleri taşımıyor.** PR §6, §26.3 `try_program`.
   Kayıt `{bar, t, side, entry, outcome, exit_bar, exit_price, mode}` + `source`, `rule`, `program_id`. Eksikler:
   (a) `instance_id` ve bağlanan `params` (kural değerleri parametre olabilir; aynı `program_id` farklı kurallarla
   toplanır); (b) sahip ve yazar kökeni (insan/ajan; RX5 `provenance.author`); (c) bar seri kimliği, `interval`,
   `since_t`/çapa; (d) doğum barındaki durum anahtarı (madde 11); (e) kimlik `bar` indeksiyle değil `t`'yle
   (indeks pencereye bağlı). **Düzeltme (şimdi):** RX5 `LedgerEntry` adlarıyla hizala: `id = <örnek>.<çıktı>@<t>`,
   `born_t`, `known_t`, `provenance {owner, author, program_id, instance_id, params}`,
   `evidence {series, from_t, to_t, anchor_t, feeds}`. Alanlar `try`'da şimdi doğsun; RS1 aynı biçimi
   `mode: "live"` ile yazar (PR §15/18 sözü).
6. **ÖNEMLİ — `dene` penceresi canlı serinin çapasıyla aynı değil; yeniden oynatma ≠ canlı (X26).** PR §8.3.
   `try` son `program.try.max_bars` barı okur; EMA ve RSI başlangıca duyarlı. Aynı bardaki sinyal canlı seride ve
   `dene`de farklı çıkabilir, ama RS1 eşitliği sınayacak. **Düzeltme:** `try` cevabı etkin başlangıcı (`from_t`)
   yazsın; kural yazılsın: "aynı `from_t` → aynı kayıtlar" (§10 önek kararlılığının çapa sürümü). Uygunluk 77'ye
   çapası aynı `try` ↔ canlı seri karşılaştırması, kayıt sayısı > 0 güvencesiyle (RX5 §7/8).
7. **ÖNEMLİ — Program kimliğinin kararlılık kuralı eksik.** PR §2 "Kanonik JSON", §2.1.
   (a) "İsteğe bağlı **boş** alan yazılmaz" yetmez: gelecek her alan `Option` olmalı, varsayılanı yazılmamalı
   (`rule.kind`, `line.tf`); yoksa var olan `program_id`'ler değişir. (b) Parametre tanımı
   `qp_schema::params::ParamDef`'in kendisiyse, kayıt defterine eklenen her alan bütün kimlikleri kırar.
   (c) `description`/`unit` kimliğe giriyor: açıklama düzeltmesi sicil kimliğini sıfırlar. **Düzeltme (şimdi),
   üç kural:** yokluk-varsayılanına eşit alan yazılmaz; `ProgramParamDef` ayrı tiptir, `qp-ir/0` ile donuk;
   kimliğin açıklama alanlarını kapsayıp kapsamadığı açıkça yazılır (öneri: `program_id` açıklamasız biçimden).
8. **ÖNEMLİ — Çizim geometrisi parametreye kopyalanıyor (K73: "geometri yalnız çizimde").** PR §4.6 `line`.
   (a) İki kopya senkron tutulmalı; K88'de kurumsal işlem katsayısı ikisine de uygulanmalı. (b) `int`/`float`
   parametrede `min`/`max` zorunlu, ama çapa `t`'si ve fiyat için anlamlı aralık yok; `line.t`'nin "geçerli
   aralığı" da tanımsız. (c) `line` taban bar indeksinde ara değer bulur: günlük grafikte çizilen eğik çizgi
   1m tabanda başka geometri verir (gece ve hafta sonu boşlukları). Görülen çizgi ile izlenen çizgi ayrılır; R7
   ölçütü "sessizce yanlış kurulmuyor" bunu yasaklar. (d) `t_i < b.t`'de değer boş: iki çapa arasında izleme yok.
   **Düzeltme:** (b) için şimdi `t`'ye zaman aralığı (`0..i64::MAX` ms) ve fiyat kuralı (`0 < p`, üst sınır
   `f64::MAX`). R7 notu: eklemeli `line.tf?` (ara değerin periyodu) ve parametre değerinde
   `{"bind": {drawing, anchor, field}}`. `instance_id`'nin çözülmüş değerden hesaplandığı şimdi yazılsın.
9. **ÖNEMLİ — Olay kanıtı yalnız 1/0 taşıyor.** PR §13.2, PROTOCOL §19.2.
   `threshold` kanıtı `p1.kirilim = 1, prev 0`; seviye ve fiyat yok. K73 "olay olgularıyla kaydedilir" ve RX4 W
   bulgusu (`level` bağlama) karşılanmıyor. **Düzeltme (şimdi, eklemeli):** olay ve sinyal çıktısına isteğe bağlı
   `evidence: [başvuru]`; `try` ve `explain_program at` bu düğümlerin olay barındaki değerlerini verir. R7 canlı
   kanıtta bunları `values`'a ekler.
10. **ÖNEMLİ — Bütçe RS1'in sürekli değerlendirmesine yetmez.** PR §8.2, PARAMETERS `program.live.*`.
    (a) Sınır doküman başına: kişi birden çok dokümanla aşar; RS1 sunucuda kişi başına sayım ister. (b) Birim
    `unit/bar`, zamana bağlı değil: aynı program 1m'de 1d'ye göre ≈ 1440 kat iş yapar, bütçe eşit sayar; X30/3 en
    küçük periyodu zorladığından R7'de fark büyük. (c) Canlı açık kayıt maliyeti yok (`max_horizon` 500 × sinyal).
    (d) K76'daki ortak sistem stratejileri kişi başına sayılmamalı. (e) RX5 HB ile `work` iki ayrı birim.
    **Düzeltme (şimdi):** kapsamı ada yaz (`program.live.doc.*`) ya da açıklamaya "doküman başına" ekle; maliyet
    cevabına `bars_per_day` (aralık ve takvimden). RS1 notu: kişi başına anahtarlar, örnek başına en çok açık
    kayıt, HB ile `work` tek birim.
11. **KÜÇÜK — Durum tanımı ilk sürüme (X34/1, program × yön taban oranı) yeter; kayıtla bağı eksik.** PR §5.4.
    Seri değeri karışık tabanlı indeks (boyut adı kaybolur); doğum barındaki anahtar kayda girmiyor; sayım bar sayıyor,
    kayıt × sonuç değil; sürekli özellik (RX5 d) yok. **Düzeltme:** kayıtta `state {output, key: [k_1..k_d]}`; RS1: `features`.
12. **KÜÇÜK — Sonuç kuralında tür etiketi ve iki aşamalı kırılım yok.** PR §6, §5.
    K73'te kırılım iki aşamalı ve sicile yazılır; kural yalnız tek aşamalı, olay çıktısında `rule` yok, `side`
    yalnız `al`/`sat` (çizim izlemesinin yönü yukarı/aşağı). **Düzeltme:** şimdi "`rule.kind` yoksa `single`"
    yazılsın (madde 7a). R7/RS1 notu: `breakout {level, hold_bars, max_bars, direction}`, olay çıktısına isteğe
    bağlı `rule` (`watch_task` kaydı), sonucu kapatan seri kayda (`resolve_interval`).
13. **KÜÇÜK — İzleme ile program arasındaki köprü belirsiz.** PROTOCOL §25.5.
    `add_watch` yalnız kapsül geçişini eşler; program olayı ancak `add_intent` tetikleyicisiyle izlenir. Kişi başına
    izleme sayacı (32) ile örnek sayacı (32) ayrı. R7 notu: izleme görevi `add_watch {match: {series:
    "<örnek>.<olay>"}}` mı, niyet mi; karar R7'den önce, D1 şart 3 raporunda. Ajanda "yalnız öneri" K72
    "kur ve göster"i iki adım yapar: önizleme gösterim sayılır mı, kendi izleme programına doğrudan yazma mı — R7 başında.
14. **KÜÇÜK — Günlük büyümesi; ara gösterim değiştirilemiyor.** PR §13.1. Her `set_program_params` çıktı başına tam
    ara gösterim yazar (≤ 32 KB × 8); sürükleme günlüğü şişirir; ara gösterimi yerinde değiştiren işlem yok. **Düzeltme:**
    madde 2'deki `program_id` başvuruyla taşımaya yol açar. R7: `replace_program`, sürükleme bitince tek komut.
15. **KÜÇÜK — Kimlik biçimi ve ad uzayı.** PR §2.1, §5.2, §13.1. `instance_id` öneki ve float yazımı (`2`/`2.0`)
    tanımsız; "örnek" hem doküman adı (`p1`) hem özet; bağlı seri özete girmiyor; `source {strategy, version}`
    serbest dizgi (iki kişinin "rsi_dip" "1"'i çakışır). **Düzeltme:** `instance_id` = `sha256:` + 64 hex,
    değerler `ProgramParamDef` türüyle kanonik; "seri kimliği özete girmez" ve "toplam kimliği
    `program_id`/`instance_id`, `source` görünen ad, ad uzayı sahip" bilerek yazılsın.
## Soru → bulgu
(1) Çizim bağı: 1, 2, 8, 14. (2) Dokunma/D1: 4, 9, 10b, 13. (3) Sicil: 1, 5, 6, 7, 12, 15. (4) Bütçe: 10.
(5) Durum tanımı: 11. (6) Kırıcı riskler: 1 (günlük anlamı), 7 (kimlik), 5 (kayıt biçimi), 2/3 (şimdi eklenecek alanlar).
