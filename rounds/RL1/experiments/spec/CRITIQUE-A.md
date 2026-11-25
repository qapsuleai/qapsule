# RL1 şartname eleştirisi A (bağımsız)

**Doğrulanan iddialar (sorun yok):** `qp-authority/src`'de `SeriesSource::Study` yalnız `..` ile
desende geçiyor (`capsule/claims.rs:572`, `capsule/story.rs:67`, `data/live.rs:282,426`); orada
`ProtocolError {…}` değişmezi yok (tek değişmez `qp-plane/src/error.rs:15`); `agent.rs:207` joker
kollu. Yani `program` ve `ProtocolError.errors` alanları otoriteye satır eklemeden derlenir.
`claims.rs` `at_index`/`at_time` `Computed.t` ile eşliyor. `highest lowest atr` `qp-compute`'ta var
(`studies/extremum.rs` tekdüze kuyruk, `studies/atr.rs`); yeni olan yalnız `stdev percentrank`.
`qp_calendar::bar_at` `Nm/1d/1w/1M` hepsini taşıyor. D1'in harfi harfine okuması (canlı barda
`null`) gerçekten çalışmaz: `values::eval_on` son değer `None` ise `false` döner (`values.rs` ~575).

## Bulgular

1. **ENGEL — K13 istisnası onaysız (PR §13.2, §17/1; CONCEPTS "Olay çıktısı").** Taslak kayıt türü
   serinin `Computed`'unu kapanmış bar sayısı uzunluğunda tutuyor. K13 "her zaman aynı uzunluk"
   diyor; bu bir karar değişikliği ve kabul ölçütü 2 şartnamenin koddan önce commit'ini istiyor.
   Ayrıca bütün `Computed` tüketicileri (`qp-grammar compile/scene.rs, axis.rs`, `qp-ops present.rs,
   checks.rs, canon/origin.rs`, `qp-read query.rs, bindq.rs`) dolgu yapmak zorunda kalır.
   **Düzeltme:** K13'ü koruyan, yalnız `qp-plane`'e dokunan seçenek var: vektörler tam uzunlukta,
   canlı bar `null` (K13 önbellekte ve telde aynen); `Computed`'a "değerlendirilebilir uzunluk"
   (kayıt türünde kapanmış bar sayısı) eklenir, `Computed::len()` onu döner, `eval_on` `last_two`'yu
   `main_values()[..len]` üstünde alır. `at_index` zaten `c.len()` ve `i < n` kullanıyor
   (`claims.rs:94-97`), yani otoritede sıfır satır. `len()`'in anlamı değişeceği için tüketiciler
   yine taranmalı. İki seçenekten biri koddan önce bir D satırıyla Arkin'e sorulmalı. Taslak
   seçenek kalırsa dolgunun yapıldığı crate ve sınaması (döküm 71) yazılsın.
2. **ÖNEMLİ — Önek kararlılığı canlı barda tanımsız (PR §10, §3.2, §5.3).** "İlk `n` barla koşu"da
   son bar canlıdır. `align`/`tf` canlı taban barı üst periyoda katmaz (§3.2). Örnek: günün son 1h
   barı ön ek koşusunda canlı olur, `align(close@1d)` orada D−1'i verir, tam koşuda D'yi. Yani
   söylendiği haliyle özellik yanlış. **Düzeltme:** özellik "kapalı pencere (bütün barlar kapanmış)
   ön ek koşusu = tam koşunun ilk `n` barı" diye yazılsın. Ayrıca "canlı barın geçici değeri, aynı
   veriyle kapanmış değerine eşit olmak zorunda değil (`align`/`tf` alt ağaçları)" diye ayrı
   bir cümle eklensin. Toplu = akışkan da kapalı pencere üstünde tanımlansın.
3. **ÖNEMLİ — Toplu bar eklemede bildirim birleşmiyor, kayboluyor (PR §13.2, D1 şart 2).** Canlı yol
   koşulu tik başına bir kez, yalnız son değerde değerlendirir (`data/live/eval.rs` `settle_live`
   → `eval_on`). Tek girdide birden çok bar kapanırsa (`append_bars` yığını, yeniden eşleme,
   hızlı yeniden oynatma), seride 0,1,0 deseni bildirimsiz ve iddiasız geçer. D1 "yalnız bildirim
   birleşir" diyor, bu ise kayıp. **Düzeltme:** §13.2'ye sınır olarak yazılsın. Kanıtta iddia
   doğmaz, seri yine doğrudur; `try_program` bunu gösterir. Ayrıca D1 şart 3 raporuna "ConditionKind
   bar başına değerlendirme" kancası olarak eklensin. Döküm 75'e çok barlı ekleme vakası girsin.
4. **ÖNEMLİ — `qp-plane` artımlı güncelleme yolu tanımsız (PR §13.4).** `update_study` `Live.study:
   Box<dyn qp_compute::Study>` ve `c.t.len() == from` ister (`values.rs:340-366`). `Study::push`
   yalnız kapanışı alır (`&[b.c]`), oysa `atr` h/l/c ister. Program bu yola uymaz. Yeni yol
   yazılmazsa her tikte ve her yeni barda program baştan koşar. **Düzeltme:** `Live`'a program
   kolu yazılsın: `Appended` → `push`, `Revised(son)` → `update_last` + `peek`, öbürü baştan.
   Kabul ölçütü 17'nin ölçüsü de verilsin (ör. 500 bar serisi × N program, tik başına µs, K17'ye
   oranla).
5. **ÖNEMLİ — Kabulde bütçe yeniden denetimi ve `owner` kaynağı (PR §8.2, §13.1, §26.2).** Öneri
   kabulünde `Check` fazı `accept_proposal`'ı görür, `add_program`'ı görmez. `OpHandler::expand`
   `qp_ops::Ctx` alır (`expand.rs:37-60`): içinde `SessionView::param` de `on_behalf` da yok.
   Dolayısıyla "aynı fonksiyon açılımda da çağrılır" ancak kayıt defteri varsayılanlarıyla
   çalışabilir. "Otorite açılımda yazar" sözü de yanlış, açılımı yazan `qp-program`. **Düzeltme:**
   `owner` = `ctx.approved_by` (kabul edilen öneri) ya da `ctx.speaker` (doğrudan) olsun; ikisi de
   `Ctx`'te var ve belirlenimci. Açılımdaki bütçe denetimi varsayılanla yapılır diye yazılsın;
   eksik kanca (`param_for(principal)` açılımda) Açık sorular 2'ye eklensin.
6. **ÖNEMLİ — Günlük şişirme (PR §13.1, §26.2; GOAL §7 kötü niyet).** Ara gösterim her çıktıda satır
   içi duruyor ve `set_program_params` her çıktı için `put_series` yazıyor: 32 KB × 8 = 256 KB
   komut başına, sınırsız tekrar (geri alma kopyası da cabası). **Düzeltme:** varsayılanlar
   düşürülsün (`max_ir_bytes` 8192, `max_outputs` 4). Ya da ara gösterim yalnız ilk çıktı
   serisinde dursun, öbürleri `program.ir_from: "<seri>"` taşısın; çözüm `qp-plane values()`'ta
   tek adımdır, otoriteye dokunmaz. Kötü niyet sınamasına "tekrarlı `set_program_params`" eklensin.
7. **ÖNEMLİ — Yalınlık: sinyal ve durum tanımı dokümana girmesin (PR §5, §5.4, §13.2-13.3, §11.2,
   §17/3).** GOAL §5 bu ikisinden yalnız denetim, "dene" ve yeniden oynatma istiyor. Dokümanda
   sinyal serisi `E_SIGNAL_NOT_LIVE`'ı, abonelik açığını (§17/3) ve `bar` katmanını doğuruyor.
   Durum serisi de karışık tabanlı anahtar indeksini. **Düzeltme:** `add_program` `signal`/`state`
   çıktılı programı `unsupported` (`reason: E_UNSUPPORTED`, ipucu `try_program`) ile reddetsin. Böylece
   §13.3, §5.4'ün anahtar formülü, `E_SIGNAL_NOT_LIVE`, Açık soru 3 ve döküm 75'in son maddesi
   düşer. GOAL kesme sırası (2) ile de uyumlu.
8. **ÖNEMLİ — "Yeniden oynatmada koşar" tanımsız (PR §5.2, §8.3, §13.3).** Sinyal kaydı dokümana
   yazılmıyor. `SessionView::run_kind`/`replay_position` `None` dönüyor (`view.rs:95-101`). Hangi
   yüzeyin "yeniden oynatma" sonucu ürettiği belirsiz. **Düzeltme:** RL1'de yeniden oynatma =
   `try_program` + `range` (geçmiş aralık) diye yazılsın. Yeniden oynatma barındırıcısında yalnız
   salınmış barlar okunur. Sınaması: aynı aralıkta iki koşu bayt eşit.
9. **ÖNEMLİ — RX4 uygunluk yöntemi eksik (GOAL §7, ölçüt 15; PR §15).** Şunlar yazılmamış:
   `rx4-ir/0` → `qp-ir/0` çevirisi (sürüm, `fingerprint` → `state`, sabit `1h` tabanın bağlı
   seriye eşlenmesi, RX4 kodu → `reason`), karşılaştırılan alanlar (değerler bit eşit mi, sinyal
   kayıtları, maliyet birimi) ve koşturan yer (sınama crate'i, `seen` verisinin sentetik kaydı).
   **Düzeltme:** §15'e 6-8 satırlık "Uygunluk yöntemi" eklensin.
10. **ÖNEMLİ — Parametre tipleri belirsiz, program kimliği buna bağlı (PR §2, §2.1, §13.1).**
    `ParamDef` `&'static str` alanlı, serde tipi değil (`params.rs:127`). `program.params` hangi
    Rust tipi? `BTreeMap<String,f64>` olursa `20` `20.0` diye yazılır, örnekteki `{"len":20}` olmaz.
    `float` parametreye `2` yazılırsa ne olur? **Düzeltme:** sahipli `ProgramParam` ve
    `ParamValue {Int(i64), Float(f64), Bool}` etiketsiz; kanonik yazım: int tamsayı, float her zaman
    ondalıklı (ryu). Sonlu olmayan değer `E_BAD_PARAM`; `float`'a tamsayı yazımı kabul ve float'a
    normalize edilir.
11. **ÖNEMLİ — Kayıt türü seri başka yerde işlenen olunca (PR §13.2).** `compare` koşulunda
    kayıt serisinin son kapanmış barı öbür serinin canlı barıyla karşılaştırılır (farklı barlar).
    `assemble` operand başına farklı `t` toplar. **Düzeltme:** `qp-program` `Check` fazı kayıt türü
    seriyi `compare` işleneni olarak `unsupported` ile reddetsin. Yalnız `threshold` izinli olsun ve
    bu yazılsın.
12. **ÖNEMLİ — Otorite dışı zorunlu dokunuşlar listelenmemiş (PR §13.6, §26 giriş).** Yeni `Op`
    kolları `qp-ops expand/op.rs` (`unserved` listesi, `AddWatch` gibi) ve `expand/describe.rs`
    (kapalı) ister. `SeriesSource::Study` değişmezleri `qp-ops expand/op/series.rs:93,142,236` ve
    `qp-plane values.rs:491` deseninde var. `ProtocolError` `qp-plane error.rs:15`'te kuruluyor.
    `arch-config.json` `layers` girdileri gerekiyor: `qp-lang` alan sırasına, `qp-program` oturum
    sırasına (`qp-watch` yanında); bu da `config` adımının `rule_changes` satırını gerektirebilir.
    **Düzeltme:** §13.6'ya dosya listesi olarak eklensin (ARCHITECTURE "Yeni doküman nesnesi"
    tablosu biçiminde).
13. **ÖNEMLİ — Belge bütçesi (arch-check `doc.size` kırmızı, doğrulandı).** Durum: PROTOCOL 3085
    satır (taban 2956, +129); CONCEPTS 62249 B (> 61440, +809). PROGRAMS 49761 B; §26 olduğu gibi
    taşınırsa ~61,6 KB olur, o da sınırı aşar. **Önerilen yapı:** §26.1 PR §13.1-13.3'ün, §26.2
    §13.6'nın, §26.3 §13.7'nin kopyası. Bunlar PR §13'e birleştirilsin; yalnız yeni olan cevap
    alanları ve döküm tablosu PR §13.8 olsun (+~4 KB, PROGRAMS ~54 KB). PROTOCOL'de katalogdaki üç
    işlem tek satıra, üç sorgu tek satıra insin; §15 satırı kalsın; §26 üç satırlık işaretçi olsun
    (+6). "Ek A — Eski Command tipi" (5 satır) `docs/LEGACY.md`'ye taşınsın, bir satır daha
    sıkıştırılsın: net 0. CONCEPTS'te seri/olay/sinyal çıktısı satırları "Çıktı türleri"ne
    birleşsin; "Taban periyot", "Kanonik metin" (Metin dil'e), "Dene", "Ret" (protokol terimleri,
    PR'de kalır) silinsin: ≈ −2,5 KB.
14. **KÜÇÜK — Ufak düzeltmeler.** (a) PARAMETERS.md RL1 tablosu, `REGISTRY`'den üretilen
    `PARAMETERS.generated.md`'yi yineliyor; 3 satırlık işaretçiye insin. (b) `stdev`/`percentrank`
    `w = period` ve sınır 666, yani etkin pencere ≈ 660 (`max_window` 2000 değil); bu §8.1'e ve
    ipucuna yazılsın. (c) Seansın son barındaki olay ertesi seansın ilk barında doğar (BIST 1h);
    bu §13.2'ye sınır olarak yazılsın. (d) MCP adı `program_check`, VERIFY 16'da `check_program`:
    biri seçilsin. VERIFY 1'deki `program` adımı şartnamede yok. (e) Kesilebilir:
    `explain_program.summary`, `instance_id` (kullanılmıyor), `add_program.show` (aynı komutta
    `insert_layer` yeter).

## Hüküm
Temel tasarım D1 ile tutarlı. Satır içi seri ve `threshold ge 1` seçimi otoriteye sıfır satırla uygulanabilir; bunu kodla doğruladım.
Commit'ten önce şunlar karara bağlanmalı: K13 istisnası ya da onu koruyan seçenek (1); kapalı pencereyle önek kararlılığı (2); toplu eklemede kayıp sınırı (3); açılım bağlamı ve `owner` (5).
Sinyal ve durum tanımı dokümandan çıkarılır, §26 PROGRAMS'a birleştirilirse şartname hem yalınlaşır hem belge bütçesine sığar.
