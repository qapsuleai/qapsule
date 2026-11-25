# Programlar — hesap dili şartnamesi (`qp-ir/0`)

Durum: **RL1 şartnamesi** (koddan önce; normatif). Kavramlar `docs/CONCEPTS.md` "Hesap dili
(RL1)"; oturuma bağlanma ve protokol `docs/PROGRAMS-SESSION.md` (bundan sonra **PS §n**;
`PROTOCOL.md` §26 oraya işaret eder); parametreler `docs/PARAMETERS.md` "RL1". Karar
dayanakları: K79, X30 (on karar), K77, K78, K75; Arkin'in D1 kararı
(`rounds/RL1/DECISION.md`). Taslak: `../refs/experiments/rx4/SPEC.md` (`rx4-ir/0`, bundan
sonra **RX4 §n**); metin dil taslağı `../refs/experiments/rx4/surface-a/DOC.md`.

Ajanın ve insanın yazdığı her hesap tek bir tipli veri akışı ara gösterimine (`qp-ir`) iner.
Aynı ara gösterim göstergeyi, koşulu, izleme koşulunu, strateji sinyalini (sonuç kuralıyla)
ve durum tanımını ifade eder. Motor koşar, maliyeti önceden bilir, her değeri kökenine
bağlar. Model hiçbir sayıyı hesaplamaz.

Bu belgede "bar" bağlı bar serisinin barıdır. Uygulama yerleri: tipler `qp-schema`; denetim,
çalıştırıcı, bütçe ve metin dil `qp-lang` (alan katmanı); yeni çekirdekler `qp-compute`;
oturum alt sistemi `qp-program`; değerlerin hesabı `qp-plane` (`qp-lang` ile).

## 1. Veri

- **Bar:** Qapsule `Bar {t, o, h, l, c, v}`; `t` barın açılış zamanı (epoch ms, UTC).
- **Taban periyot = bağlı serinin aralığı.** Program tek bir bar serisine bağlanır
  (`inputs: [<bar serisi>]`); taban saat o serinin `interval`'idir (`1m` … `1M`). RX4'teki
  sabit `1h` yoktur (§15).
- **Kapanmış bar ve canlı bar.** Bar serisinin son barı **canlıdır**; bir bar, ondan sonra bar
  geldiğinde ya da seri kapalı pencereli bağsa (`window.to_t`) **kapanmış** sayılır. Bu,
  otoritenin iddia olgunluğu kuralıyla aynıdır (`PROTOCOL.md` §19.2 `maturity`;
  `qp-authority` `claims::closed`). Olay, sinyal ve durum kaydı yalnız kapanmış barda doğar
  (X30/3). Canlı barda yalnız gösterim için geçici değer hesaplanır (§5.3). **Kapalı
  pencere:** bütün barları kapanmış veri (kapalı bağ ya da sınama verisi); belirlenimcilik
  özellikleri bunun üstünde tanımlıdır (§10).
- **`known_t`** (bilinme anı): kapanmış bir barın kapanış zamanı — takvim kapanışı
  (`bar_at(base, t).close`), UTC hizasında `t + taban`; aralık bilinmiyorsa sonraki barın
  `t`'si. Kayıtlar ve özetler barın açılışını (`t`) ve `known_t`'yi ayrı taşır (RS1 as-of).
- **Üst periyotlar seansa hizalıdır** (X30/5; §3.2): enstrümanın borsa takvimi
  (`qp-calendar`), takvimi olmayan ya da 7/24 enstrümanda UTC.

## 2. Program

```json
{
  "version": "qp-ir/0",
  "params": [ {"key": "len", "kind": "int", "min": 2, "max": 500, "default": 20} ],
  "nodes":  [ {"id": "c", "op": "input", "field": "close"}, … ],
  "outputs": [ {"name": "kirilim", "kind": "event", "node": "k"} ]
}
```

- `version` tam olarak `"qp-ir/0"`; başkası `E_VERSION`.
- `params` (isteğe bağlı, boşsa yazılmaz): program parametreleri (§2.1).
- `nodes` topolojik sıradadır: bir düğüm yalnız **kendinden önceki** düğümlere başvurur
  (döngü yapı gereği yok). Kimlik deseni `^[A-Za-z_][A-Za-z0-9_]{0,63}$`, benzersiz
  (`E_DUPLICATE_NAME`). `_` ile başlayan kimlikler metin dilin adsız düğümleri içindir (§12.3);
  ara gösterimde serbesttir.
- **Başvuru:** düğüm alanındaki başvuru dizgidir: `"id"` ya da çok çıktılı düğümde
  `"id.çıktı"` (§4.2). Tanımsız ya da sonraki düğüm `E_UNKNOWN_REF`.
- Bilinmeyen alan reddedilir (`E_UNKNOWN_FIELD`); şema `deny_unknown_fields`.
- **Ad uzayları:** düğüm kimlikleri, çıktı adları ve parametre anahtarları üç ayrı ad
  uzayıdır; tekrar her birinin içinde `E_DUPLICATE_NAME`. Metin dilde parametre ve bağlama
  adları tek ad uzayını paylaşır (§12.1).
- **Kanonik JSON:** alan sırası tip tanımının sırası, tek satır (`serde_json::to_string`).
  Sayı yazımı tiptendir: tamsayı alanlar (`period`, `n`, `bars`, `max_bars`, `int` değer, `t`)
  JSON tamsayısı; ondalık alanlar (`f64`) `serde_json` yazımıyla (en kısa gidiş-dönüş, tamsayı
  değerli ondalık `2.0`). **Program kimliği** = `sha256:` + kanonik JSON'un SHA-256'sının 64 küçük
  onaltılığı (`program_id`). SHA-256 için yeni bağımlılık yok (elle yazılmış uygulama var:
  `qp_capsule::trust::seal::sha256`).
- **Kimlik kararlılığı (normatif, `qp-ir/0` boyunca):** (1) her isteğe bağlı alan `Option`'dır ve
  yokluğa eşit değer (boş liste, varsayılan) **yazılmaz**; sonradan eklenen alan da böyledir, var
  olan kimlikler değişmez. (2) Parametre tanımı kayıt defterinin `ParamDef`'i değil, `qp-ir/0`
  ile donmuş kendi tipidir (`ProgramParamDef`); kayıt defterine eklenen alan kimliği değiştirmez.
  (3) Açıklama alanları (`unit`, `description`) kimliğe **girmez**: `program_id` bu alanlar
  çıkarılmış kanonik biçimden hesaplanır; açıklama düzeltmesi sicil kimliğini sıfırlamaz.
  (4) Bağlı seri ve örnek adı kimliğe girmez.
- **Boyut sınırları** parametredir: düğüm sayısı `program.max_nodes`, kanonik JSON baytı
  `program.max_ir_bytes`, çıktı `program.max_outputs`, parametre `program.max_params`
  (`E_TOO_MANY_NODES`, `E_TOO_LARGE`). Bayt ve çıktı sınırı günlük büyümesini de sınırlar
  (PS §1.1).

### 2.1 Program parametresi (K78, X30/9)

Programın adlı, tipli, aralıklı girdisidir. Tanım biçimi parametre kayıt defterinin satırıdır
(`qp_schema::params::ParamDef`'in programa anlamlı alt kümesi), ama tipi ayrıdır:
`ProgramParamDef` (sahipli, serde; `qp-ir/0` ile donuk). Değer tipi `ProgramParamValue`
(etiketsiz: `Int(i64)`, `Float(f64)`, `Bool`):

| Alan | Kural |
|---|---|
| `key` | `^[a-z][a-z0-9_]{0,31}$`, program içinde benzersiz (`E_DUPLICATE_NAME`) |
| `kind` | `int`, `float`, `bool` (kayıt defterinin `text` türü programda yok) |
| `unit` | isteğe bağlı (`bars`, `ratio`, `pct`, `price`, `ms` …), yalnız açıklama |
| `min`, `max` | `int` ve `float` için **zorunlu**, kapsayıcı; `bool`'da yok |
| `default` | `kind`'a uygun ve aralık içinde (`E_PARAM_RANGE`) |
| değer yazımı | `int`: JSON tamsayısı (`20.0` → `E_TYPE`); `float`: sayı, tamsayı yazımı kabul edilir ve ondalığa çevrilir (`2` → `2.0`); `bool`: `true`/`false`. Sonlu olmayan değer `E_BAD_PARAM` |
| `description` | isteğe bağlı kısa metin |

- **Başvuru:** sayı yazılabilen her düğüm alanı yerine `{"param": "<key>"}` alır. Tamsayı
  alanı (`period`, `n`, `bars`, `fast`, `slow`, `signal`, `max_bars`) `int` parametre ister
  (`E_TYPE`). Tanımsız anahtar `E_UNKNOWN_PARAM`.
- **Aralık denetimi iki kat:** (1) tanımın aralığı başvurulduğu alanın geçerli aralığının
  içinde olmalı (ör. `period` için `1 … program.max_window`) — değilse `E_PARAM_RANGE`,
  statik; (2) bağlanan değer tanımın aralığında olmalı — değilse `E_PARAM_RANGE`.
- **Program örneği** = program + bağlanmış parametre değerleri (eksikler varsayılanla
  doldurulmuş, tam küme). **Örnek kimliği** (`instance_id`) = `sha256:` + SHA-256(`program_id` +
  `\n` + bağlanan değerlerin kanonik JSON'u, anahtar sırasıyla, çözülmüş değerlerden). Aynı
  program farklı değerlerle farklı örnektir;
  `program_id` aynı kalır. Tarama ve yeniden kurma bu tanımdan çalışır (aralık = tarama
  uzayı).
- Maliyet bağlanan değerlerle hesaplanır (§8); değer değişince yeniden denetlenir.

## 3. Tipler ve saat

### 3.1 Tipler ve boş değer

RX4 §3 aynen: `num` (`Option<f64>`), `bool` (`Option<bool>`), `cat` (`Option<u32>`, kutu
indeksi). Aritmetik, karşılaştırma ve mantıkta boş yayılır; durum düğümleri boş `bool`'u
yanlış sayar; çekirdekler boş girdiyi atlar. `cat` yalnız durum tanımı çıktısına ve
`defined`'a girer; başka yerde `E_TYPE`.

### 3.2 Saat ve üst periyot hizası

- Her düğümün saati vardır: `base` (taban) ya da kanonik bir aralık (`4h`, `1d`, `1w` …;
  `PROTOCOL.md` §18.2 yazımı). Düğümün saati girdilerinin saatidir; farklı saatli girdiler
  `align`'sız birleşirse `E_CLOCK_MISMATCH`. `const` saatsizdir, birleştiği girdinin saatini
  alır; doğrudan çıktıya ya da durum düğümüne bağlanırsa `base`.
- **İzinli üst periyot:** tabandan büyük ve tabanı bölecek biçimde: gün içi taban `Nm` için
  gün içi `Mm` ancak `M` `N`'nin katıysa; `1d`, `1w`, `1M` her gün içi tabanın üstünde; `1d`
  tabanın üstünde `1w`, `1M`; `1w` tabanın üstünde yok. Değilse `E_UNKNOWN_TF`. Bağlı serinin
  aralığı yoksa ve program üst periyot kullanıyorsa `E_UNKNOWN_TF`.
- **Hiza kaynağı:** bağlı serinin `instrument.calendar`'ı (`qp_calendar::builtin`, ör. `XIST`)
  varsa **seans hizası**: üst bar `qp_calendar::ExchangeCalendar::bar_at(tf, t)` aralığıdır
  (gün içi barlar her seansın açılışına hizalı, seans kapanışında kesilir; `1d` işlem günü;
  yarım günler takvimden). Takvim yoksa **UTC hizası**: `Nm`/`Nh` için `T = ⌊t / len⌋ · len`,
  `1d` UTC günü, `1w` ISO haftası (pazartesi 00:00 UTC), `1M` UTC ayı.
- **Üst barın kuruluşu:** taban bar `i`, `bar_at(tf, t_i)` aralığının barına girer (açılış
  zamanıyla; `qp_calendar::ExchangeCalendar::resample` ile aynı kural): `o` ilk, `h` en büyük,
  `l` en küçük, `c` son, `v` toplam, girdi sırasıyla. Hiçbir seansa düşmeyen taban bar üst
  periyoda girmez (`dene` özetinde `dropped` sayılır).
- **Yalnız kapanmış taban barı** üst bara girer; canlı taban bar üst periyodu hiç etkilemez.
- **Üst barın kapanması:** üst bar `k` (aralık `[open_k, close_k)`), `close_k ≤ kapanış_i`
  olan ilk kapanmış taban barda ya da daha sonraki bir üst bara düşen ilk kapanmış taban barda
  kapanır. `kapanış_i` = taban barın takvim kapanışı (`bar_at(base, t_i).close`; UTC hizasında
  `t_i + taban`). Açılışı ilk taban bardan (çapadan) önce olan üst bar **eksiktir** ve hiç
  oluşmaz.
- Üst periyot serisi yalnız kapanmış üst barlardan oluşur; `align` kapanmamış bar okuyamaz
  (RX4 §4.7 ilkesi): geleceğe bakma yapı gereği imkânsızdır.

## 4. Düğümler

Alanlar: `x`, `a`, `b`, `c`, `trigger`, `reset` başvurudur; sayısal alanlar sabit sayı ya da
parametredir (düğüm değildir). `w` bar başı maliyet ağırlığıdır (§8).

### 4.1 Girdi ve sabit

| `op` | Alanlar | Tip, saat | `w` | Anlam |
|---|---|---|---|---|
| `input` | `field` ∈ `open high low close volume`, `tf?` | num @`tf` (yoksa `base`) | 0 | O periyodun kapanmış barlarının alanı (tabanda canlı bar §5.3) |
| `const` | `value` (sayı, `true`/`false` ya da parametre) | num/bool, saatsiz | 0 | Sabit |
| `bar_index` | — | num @base | 0 | `i` |
| `time` | — | num @base | 0 | `t_i` (açılış, ms) |

`"partial": true` (kapanmamış üst bar isteği) `E_LOOKAHEAD`. `tf` tabandan küçük ya da
izinsizse `E_UNKNOWN_TF`.

### 4.2 Çekirdekler

Bütün çekirdekler `qp-compute`'tadır (`Core`: `peek`/`commit`; toplu = akışkan = son barı
düzelt, bit eşit; yalnız IEEE temel işlemler ve `sqrt`; üç mimaride özet). Tek girdili
çekirdek girdinin **tanımlı** değerler dizisini görür; girdi boşsa çıktı boştur ve durum
ilerlemez — `qp_compute::Chain` kuralı (canlı barda girdinin boş/tanımlı arasında değişmesi
dahil). Tamsayı alanlar `1 … program.max_window` (`E_WINDOW_LIMIT`; 0 ya da kesirli
`E_BAD_PARAM`).

| `op` | Alanlar | Çıktılar (ana çıktı **kalın**) | `w` | Tanım | Yer |
|---|---|---|---|---|---|
| `sma` | `x, period` | **`sma`** | 2 | RX4 §4.2; `add_study` `sma` ile aynı çekirdek | var |
| `ema` | `x, period` | **`ema`** | 2 | RX4 §4.2 | var |
| `rsi` | `x, period` | **`rsi`** | 2 | RX4 §4.2 | var |
| `macd` | `x, fast, slow, signal` | **`macd`**, `signal`, `histogram` | 3 | `qp-compute` `Macd` (`studies/mod.rs` tablosu) | var |
| `bollinger` | `x, period, k` (`k` float, > 0) | `upper`, **`middle`**, `lower`, `bandwidth` | 3 | `qp-compute` `Bollinger` (nüfus std; bandwidth `2k·std/middle`, middle 0 ise 0) | var |
| `highest` | `x, period` | **`highest`** | 2 | Son `p` tanımlı değerin en büyüğü (bu dahil) | var (R3/T3) |
| `lowest` | `x, period` | **`lowest`** | 2 | En küçüğü | var (R3/T3) |
| `atr` | `period`, `tf?` | **`atr`** | 2 | Saatin `h,l,c`'si; `qp-compute` `Atr` (RX4 §4.2 ile aynı) | var (R3/T3) |
| `stdev` | `x, period` | **`stdev`** | `period` | RX4 §4.2: son `p` değerin nüfus std'si, `m = Σx/p`, `sqrt(Σ(x−m)²/p)`, toplamlar pencere sırasıyla (eskiden yeniye), iki geçiş | **yeni** |
| `percentrank` | `x, period` | **`percentrank`** | `period` | RX4 §4.2: `100 × #{k ∈ 1..p : x_{j−k} ≤ x_j} / p`; ilk değer `x_p`'de | **yeni** |

- **Çok çıktılı düğüme başvuru:** `"m.signal"`; yalın `"m"` ana çıktıdır (`PROTOCOL.md` §5.1
  ana çıktı kuralıyla aynı: hesabın adı, `bollinger` için `middle`). Olmayan çıktı
  `E_UNKNOWN_REF` (`did_you_mean` çıktı adlarından).
- `highest`, `lowest`, `atr` R3/T3'ten beri `qp-compute`'tadır (olgu dedektörleri); yeniden
  yazılmaz. `stdev` ve `percentrank` aynı kurallarla eklenir; `STUDY_NAMES`, `add_study`
  kayıt defteri ve var olan hesap özeti değişmez; yeni çekirdeklerin özeti ayrı
  (`r3_studies_digest` gibi).
- Bollinger'in kendi std'si ile `stdev` düğümü ayrı çekirdektir; bit eşitlik gereken yerde
  (§14) `bollinger` düğümü kullanılır.

### 4.3 Aritmetik, geçmiş, karşılaştırma ve mantık

RX4 §4.3–§4.5 aynen (`w` = 1): `add sub mul div min max` (`a, b`), `neg abs` (`x`); `div`
sıfıra bölmede boş; sonlu olmayan sonuç boş. `min(a,b)` = `b < a ? b : a`, `max(a,b)` =
`b > a ? b : a` (işaretli sıfırda belirlenimci). `lag {x, n}` (`n` 0 … `program.max_window`;
negatif `E_LOOKAHEAD`). `gt ge lt le eq ne` (num, num → bool), `and or not`.
`cross_above`, `cross_below`: RX4 §4.5 (Qapsule `PROTOCOL.md` §5.1 eşitlik kuralı).

### 4.4 Durum düğümleri

RX4 §4.6 aynen (boş `bool` = yanlış; `w` = 1): `streak {c}` → num, `bars_since {c}` → num,
`once {c}` → bool, `rearm {trigger, reset}` → bool, `cooldown {trigger, bars}` → bool.

### 4.5 Tanımlılık (yeni)

| `op` | Alanlar | Tip | `w` | Anlam |
|---|---|---|---|---|
| `defined` | `x` (num, bool, cat) | bool, girdinin saati | 1 | `x` bu barda tanımlıysa doğru, değilse yanlış; **hiç boş değildir** |

RX4'te tanımlılık örtüktü; yanlış taban seçimi sessiz hata doğuruyordu (RX4 W bulgusu,
`COMPUTE-LANGUAGE.md` §6). `line` boşken olay doğmamasını açık yazmanın yolu budur.

### 4.6 Üst periyot, çizgi, kutulama

- `align {x}` (`w` 0): RX4 §4.7, hizası §3.2: taban bar `i`'nin kapanışında kapanmış son üst
  bar `k`'nin değeri; yoksa boş. Sonuç saati `base`, tip `x`'inki.
- `line {a: {t, price}, b: {t, price}}` (`w` 0): RX4 §4.8 aynen; `t` ve `price` parametre
  olabilir (`t` `int`, aralığı `0 … 2^53` ms içinde; `price` `float`, `> 0`). `idx(t)` bağlı
  serinin taban barlarından. **R7'nin çizim seviyeleri
  buradan bağlanır:** bir çizimin çapaları `line` parametreleri olur; çizim değişince
  (`rev`) parametre değişir, program aynı kalır. Bu round'da çizim bağı yok.
- `bucket {x, edges}` (`w` 1): RX4 §4.9; `edges` kesin artan 1…8 eleman (dil sabiti), eleman
  parametre olabilir (sıra bağlanan değerlerle denetlenir, `E_BAD_PARAM`). Yatay seviye
  `const`'tır.

## 5. Çıktılar

Her çıktının `name`'i `^[a-z][a-z0-9_]{0,31}$`, benzersiz (`E_DUPLICATE_NAME`); düğümü `base`
saatinde (`E_OUTPUT_CLOCK`); en az bir çıktı (`E_NO_OUTPUT`).

| `kind` | Alanlar | Değer | Dokümanda (PS §1) |
|---|---|---|---|
| `series` | `node` (num) | bar başına sayı ya da boş | hesap serisi |
| `event` | `node` (bool) | düğümün doğru olduğu kapanmış barlar | olay serisi (kapanmış barda 1/0) |
| `signal` | `node` (bool), `side` ∈ `al sat`, `rule`, `source {strategy, version}` | doğru olduğu her kapanmış bar bir sinyal kaydı (§6) | **girmez**: yalnız `check`, `dene`, `qp program run` |
| `state` | `dims: [{name, node (cat)}]` (1…4 boyut, dil sabiti) | bar başına anahtar `[k_1, …, k_d]` ya da (bir boyut boşsa) boş | **girmez**: aynı |

`add_program` sinyal ya da durum tanımı çıktılı programı `unsupported` /
`E_TRY_ONLY` ile reddeder (ipucu: `try_program`; canlı sicil RS1). Böylece sinyal canlıda
iddia doğuramaz.

### 5.1 Olay

Olay yalnız kapanmış barda doğar. Düğümün boş olduğu kapanmış bar olay değildir. Canlı barda
olay yoktur.

### 5.2 Sinyal

- `side` yalnız `al` ya da `sat`. **"Tut" kayıt doğurmaz** (X30/8): `tut` ya da başka değer
  `E_UNSUPPORTED`.
- `rule` zorunludur (X25): yoksa `E_NO_OUTCOME_RULE`; değerleri §6.
- `source {strategy, version}`: dizgi; sinyal kaynağıyla söylenir (K77). Sinyal "tavsiye"
  diye sunulmaz; ajan kişiye uyarlanmış al/sat dili kurmaz.
- **Bu round'da sinyal canlıda iddia doğurmaz** (sicil RS1'in işi). Sinyal kaydı yalnız
  `dene`de (yeniden oynatma kipi, §8.3), `mode: "replay"` ve "simüle" etiketiyle doğar
  (X26, K77).

### 5.3 Canlı bar

Değer türü çıktılar canlı barda **geçici değer** taşır: çalıştırıcı
kapanmış barlardan kurulu durumun üstüne canlı barı `peek` ile hesaplar; durum değişmez,
kopya alınmaz (`qp-compute` `Streaming` deseni). Canlı barın her tiki yeniden `peek`'tir;
bar kapanınca `commit`. Geçici değer hiçbir zaman olay, sinyal ya da kayıt doğurmaz.

Olay ve sinyal canlı barda **değer taşımaz** (olay serisinde canlı bar `null`, PS §1.2).
Geçici değer, aynı veriyle bar kapandığındaki değere eşit olmak zorunda değildir: `align` ve
üst periyot alt ağaçları canlı taban barı görmez (§3.2).

### 5.4 Durum tanımı

RX4'ün `fingerprint` çıktısıdır; Qapsule adı **durum tanımı** (`state`). Her boyut bir
`bucket`; anahtar boyut sırasıyla, boyut adlarıyla (`{"momentum":2,"activity":0}`). Sayımı
(anahtar → bar sayısı) yalnız `dene`de, kapanmış barlardan. Aynı programın sinyal kaydı doğum
barındaki anahtarı taşır (§6).

## 6. Sinyal ve sonuç kuralı

RX4 §6 aynen: `rule {target_pct?, stop_pct?, max_bars}`; `max_bars` zorunlu,
`1 … program.signal.max_horizon_bars`; `target_pct`, `stop_pct` > 0 isteğe bağlı (değilse
`E_BAD_RULE`); değerler parametre olabilir. Her sinyal barı ayrı iddiadır; giriş `c_i`; önce
durdurma, sonra hedef, aynı barda ikisi → `stop`; `expired`; veri biterse `open`. Sonuç yalnız
sinyal barından **sonraki kapanmış** barlardan kapanır; canlı bar sonuca girmez. `qp-ir/0`'da
`rule.kind` alanı yoktur: kural tek aşamalıdır; açık `"kind"` yazılırsa `E_UNSUPPORTED` (iki aşamalı
kırılım RS1/R7'de alan olarak eklenir, yokluğu tek aşamalı sayılır; kimlik değişmez, PS §8).

**Kayıt biçimi** (RX5 `LedgerEntry` adlarıyla; kimlik bar indeksiyle değil zamanla):

```json
{"id":"p1.rsi_dip@1704164400000","source_kind":"signal","mode":"replay","label":"simulated",
 "provenance":{"owner":"arkin","author":{"kind":"human","name":"arkin"},"program_id":"sha256:…",
   "instance_id":"sha256:…","params":{…},"strategy":"rsi_dip","program_version":"1"},
 "instrument":"THYAO","period":"1h","born_t":1704164400000,"known_t":1704168000000,
 "side":"al","entry":101.25,"rule":{"target_pct":3.0,"stop_pct":1.5,"max_bars":24},
 "state":{"output":"regime","key":{"momentum":1}},
 "outcome":{"kind":"stop","exit_t":1704189600000,"exit_price":99.73},
 "evidence":{"series":"px","from_t":…,"to_t":1704164400000,"anchor_t":…,"feeds":[…]}}
```

`born_t` sinyal barının açılışı, `known_t` kapanışı (§1); `outcome.kind` ∈ `target stop expired
open` (`open`'da `exit_*` yok); `state` aynı programda durum tanımı varsa; `evidence.from_t`
kökenin `lo` barı (§9). Bar indeksi (`bar`, `exit_bar`) yalnız RX4 uygunluk çıktısında.

## 7. Çalıştırıcı

- Girdi bağlı serinin bütün bar alanlarıdır (`o h l c v`; `atr` ve üst periyot için), yalnız
  kapanış değil. Barları tek tek iter (akışkan). Kapalı pencerede toplu verilse de bar bar
  verilse de çıktı bayt eşittir.
- **API ilkesi:** `push(bar)` (önceki canlı barı kapat, yeni barı canlı yap), `update_last(bar)`
  (canlı barı değiştir), `peek` (geçici değer, kopyasız). Geçmiş bar düzeltmesi örneği
  baştan kurar (`qp-plane` hesap kuralı; `PROTOCOL.md` §18.8).
- **Çıktı imleci** (RX5 §7/2): `value_at(output, i)`, `events_at(i)`, `signals_at(i)`; okumalar
  artımlıdır (doğum başına O(n) okuma yok) ve bütçede sayılır (§8.3).
- **Hiza kipi:** çalıştırıcı bir hiza kaynağı alır (takvim ya da UTC); oturum onu bağlı
  serinin enstrümanından seçer, RX4 uygunluğu UTC kipini zorlar.

## 8. Maliyet ve hesap bütçesi

### 8.1 Statik maliyet (çalıştırmadan önce)

- **Maliyet birimi:** bir düğüm ağırlığının bir bardaki işi.
- Ağırlıklar §4 tablolarında. Ağırlık bağlanan değerlerle hesaplanır (`stdev` için `period`).
- `per_bar = Σ_düğüm w + çıktı sayısı` (tamsayı). Üst periyot düğümü **indirimsiz** sayılır
  (temkinli; §15).
- `live_per_bar = Σ_çıktı (çıktının bağımlılık konisindeki Σ w + 1)`: dokümanda her çıktı
  serisi yalnız kendi konisini koşar (PS §1.3); canlı bütçe bunu sayar.
- `stdev` ve `percentrank`'te `w = period` olduğundan etkin pencere `max_window`'dan önce bar
  başı sınırla dolar (varsayılanlarla tek başına ≈ 660); `E_COST_LIMIT` ipucu bunu söyler.
- `total(N) = per_bar × N` (bilgi için; `check` bağlı serinin bar sayısıyla verir).
- Sınır: `per_bar ≤ program.max_cost_per_bar` (`E_COST_LIMIT`). Bar sayısıyla ölçeklenir
  (X30/6): sınır bar başınadır.
- Döngü, özyineleme, dinamik pencere yok; maliyet yalnız düğüm türü, parametre ve bar
  sayısından gelir.

### 8.2 Kişi başına canlı bütçe (doküman içinde)

Her program örneğinin bir **sahibi** vardır (PS §1.1 `owner`). Doküman başına, sahip başına:
örnek sayısı ≤ `program.live.doc.max_instances`; `Σ live_per_bar` ≤
`program.live.doc.max_cost_per_bar`; program yazılarının günlükteki baytı ≤
`program.live.doc.max_log_bytes`. Üçü satılabilir haktır (plan; K78). Aşım `limit` /
`E_BUDGET`, `expected` = sınır. Denetim yeri ve kabuldeki yeniden denetim PS §2.3. Birden çok
doküman üstünden kişi başına sayım RS1'in işidir.

### 8.3 Çalışma bütçesi ve yeniden oynatma

**Yeniden oynatma** RL1'de iki şeydir: (a) **program yeniden oynatması** = `try_program`'ın bir
geçmiş aralıkta koşması (`range`; geriye dönük test kipi): kayıtlar `mode: "replay"`, aynı
aralık ve aynı başlangıç (`from_t`) → bayt eşit sonuç; yeniden oynatma barındırıcısında
(`PROTOCOL.md` §21) yalnız salınmış barlar okunur. (b) **oturum günlüğünün yeniden oynatılması**
(`qp_ops::apply::replay`): program serileri günlükteki ara gösterimden aynı değerleri verir,
doküman bayt eşit. Ayrı bir yeniden oynatma motoru yoktur (X26).

`dene` cevabı etkin başlangıcı (`from_t`, okunan ilk barın `t`'si) yazar. Kural: aynı `from_t`
ve aynı barlarla `dene`nin olay barları, dokümandaki olay serisinin aynı barlardaki değerleriyle
eşittir (kapalı pencerede önek kararlılığı, §10). Dokümandaki seri bağın çapasından (`anchor_t`)
başlar; `dene` varsayılan olarak son `program.try.max_bars` barı okur, yani çapadan farklı
başlayabilir (EMA, RSI başlangıca duyarlı) — karşılaştırma aynı `from_t` ile yapılır.

Bütçe **işi sayar** (RX5 güvenlik bulgusu, `../refs/docs/rounds/RX5/REPORT.md` §7/1):

```
iş = Σ_bar per_bar
   + doğan kayıt sayısı          × program.work.record
   + Σ_bar açık kayıt sayısı      × program.work.open_record
   + çıktı imleci okuma sayısı   × program.work.read
```

- Sınır `program.try.max_work`; bar sınırı `program.try.max_bars`.
- Aşınca çalışma o barda **durur**; sonuç o bara kadarki özet + `stopped {reason: "budget",
  bar, t, work, limit}` taşır (hata değil). Durduktan sonra kayıt doğmaz.
- Canlı hesap (dokümandaki seriler) kayıt doğurmaz; işi statik olarak `live_per_bar` ile
  sınırlıdır.

### 8.4 Sınırların kaynağı

Hiçbir sınır koda gömülmez. Değerler parametre kayıt defterindedir (`docs/PARAMETERS.md`
"RL1"); alt sistem `SessionView::param(key)` ile okur, `None` ise kayıt defterinin
varsayılanını kullanır (kişiye göre çözüm yok; Açık sorular 1). Dil sabitleri (kutu kenarı 1…8, durum boyutu 1…4, ağırlıklar)
parametre değildir; değişmeleri dil sürümünü değiştirir.

## 9. Köken

RX4 §8 aynen, `data` alanı Qapsule'e uyarlanmış: her yayımlanan değer (seri değeri, olay barı,
sinyal kaydı, durum anahtarı) `{output, node, bar, t, range: [lo, i], program_id, data}`'ya
izlenir; `data` = bağlı serinin kimliği, çapası (`anchor_t`) ve beslemeleri (`PROTOCOL.md`
§18.3, §19.2 `feeds`) — RX4'teki "verinin SHA-256'sı" yerine. `lo` kuralları RX4 §8; üst
periyot düğümünde `lo` üst bar `k`'nin ilk taban barıdır. Sinyal kaydının sonuç kısmı ayrıca
`[i+1, exit_bar]`'a izlenir. Canlı yolda olay serisinin iddiası var olan kanıt düzeniyle
(`ClaimEvidence`) gelir ve yalnız 1/0 değerini taşır; seviye ve fiyat gibi olay anındaki
değerler `dene` ve `explain_program` (`at`) ile kökenden okunur (canlı kanıta eklenmesi R7
notu, PS §8).

## 10. Belirlenimcilik ve geleceğe bakma

RX4 §9 aynen, Qapsule kurallarıyla:

- Aynı program + aynı parametre + aynı veri → bayt eşit çıktı. İki koşu bayt eşit.
- **Kapalı pencerede** (§1) toplu = akışkan; canlı bar düzeltmesi (`update_last`) bar
  kapandığında aynı sonucu verir (son barı düzeltilmiş = toplu); aarch64 = x86_64 = wasm32
  (özet).
- **Önek kararlılığı (kapalı pencere):** her `n` için ilk `n` barın **kapalı pencere** koşusu,
  tam koşunun ilk `n` barına eşit (sinyal sonucu `open` kalabilir). Canlı barın geçici değeri
  bu özelliğin dışındadır (§5.3). Rastgele program, veri ve `n` ile özellik testi.
- Hesap kodunda model, ağ, saat, rastgelelik yok; yalnız IEEE temel işlemler ve `sqrt`
  (`exp`, `ln`, `powf`, `mul_add` yok).
- Geleceğe bakma isteği (`lag` negatif, `partial`, gelecek değer) programla değil kodlu
  retle karşılanır (`E_LOOKAHEAD`).

## 11. Hata modeli

### 11.1 Biçim

`check` bütün hataları **bir arada** döndürür (ilk hatada durmaz). Hata nesnesi
(`ProgramError`):

```json
{"code":"schema","reason":"E_UNKNOWN_OP","message":"unknown function `rsii`",
 "at":"/nodes/3/op","span":{"line":2,"col":5,"len":4},
 "expected":["sma","ema","rsi","…"],"did_you_mean":["rsi"],"hint":"…"}
```

- `code`: Qapsule'ün **kapalı** hata kodu (`PROTOCOL.md` §13); yeni kod eklenmez.
- `reason`: alt kod (RX4 kodu ya da aşağıdaki yeni `E_` kodları); `ProtocolError.reason`
  alanıyla aynı ad ve anlam ("kodun makinece okunur alt nedeni").
- `at`: ara gösterimde JSON işaretçisi (program köküne göre). Komut hatasında protokol
  `path`'i iletinin içine işaretçidir (`/ops/0/program/nodes/3/op`).
- `span {line, col, len}`: metin dilde; 1 tabanlı satır ve sütun, sütun ve uzunluk Unicode
  karakteri cinsinden.
- `did_you_mean`: en çok 5 (`PROTOCOL.md` §13 kuralı); ad uzayı işlev adları, düğüm
  kimlikleri, çıktı adları, parametre anahtarları, alan adları. Türkçe işlev adı yazılırsa
  (`ortalama`, `ussel_ortalama` …) sabit bir eşleme tablosundan İngilizce karşılığı önerilir;
  Türkçe ad kabul edilmez.
- **Sıra:** konum sırası (ara gösterimde `params`, `nodes`, `outputs`; metinde satır, sütun),
  sonra `reason`. Hatalı düğüme dayanan düğümler "hatalı" tiplenir ve ek tip hatası doğurmaz
  (gürültü yok). En çok `program.check.max_errors` hata; fazlası `truncated: true`.
- İletiler `lang` (`en` varsayılan, `tr`) ile iki dilde; kod ve alt kod dilden bağımsız.

### 11.2 Eşleme

| Alt kod (`reason`) | `code` | Ne zaman |
|---|---|---|
| `E_PARSE` | `schema` | Metin sözdizimi bozuk (`span` ile). Bozuk JSON ileti düzeyinde `parse`'tır |
| `E_VERSION` | `schema` | `version` ≠ `qp-ir/0` (`expected`) — **yeni** |
| `E_UNKNOWN_OP` | `schema` | Bilinmeyen düğüm / işlev |
| `E_UNKNOWN_FIELD` | `schema` | Bilinmeyen alan ya da bar alanı |
| `E_TYPE` | `schema` | Tip uyuşmazlığı (tamsayı alanına `float` parametre dahil) |
| `E_MISSING_ARG` | `schema` | Eksik alan / argüman |
| `E_UNKNOWN_TF` | `schema` | Bilinmeyen, tabandan küçük ya da izinsiz periyot (§3.2) |
| `E_CLOCK_MISMATCH` | `schema` | Farklı saatli girdiler `align`'sız |
| `E_OUTPUT_CLOCK` | `schema` | Çıktı düğümü `base` saatinde değil |
| `E_NO_OUTPUT` | `schema` | Çıktı yok |
| `E_UNKNOWN_REF` | `not_found` | Tanımsız / sonraki düğüm ya da olmayan çıktı (`m.xyz`) |
| `E_UNKNOWN_PARAM` | `not_found` | Tanımsız parametre anahtarı — **yeni** |
| `E_DUPLICATE_NAME` | `duplicate` | Aynı düğüm kimliği, çıktı adı ya da parametre anahtarı |
| `E_BAD_PARAM` | `invalid` | Geçersiz sabit (0 periyot, kesirli, sıralı olmayan kenar, `a.t ≥ b.t`) |
| `E_PARAM_RANGE` | `invalid` | Parametre tanımı alanın aralığını aşıyor ya da bağlanan değer aralık dışı — **yeni** |
| `E_NO_OUTCOME_RULE` | `invalid` | Sinyalde `rule` ya da `max_bars` yok |
| `E_BAD_RULE` | `invalid` | Kural değerleri geçersiz |
| `E_NOT_BAR_SERIES` | `invalid` | `inputs` tam bir bar serisi değil (yok, iki tane, hesap serisi) — **yeni** |
| `E_PROGRAM_SERIES` | `invalid` | Program serisine doğrudan yazı (`put_series`, `set_params`, tek çıktının `remove`'u) — **yeni** |
| `E_TRY_ONLY` | `unsupported` | `add_program`'da sinyal ya da durum tanımı çıktısı (yalnız `dene`; RS1) — **yeni** |
| `E_EVENT_OPERAND` | `unsupported` | Olay serisi `compare` işleneni ya da `threshold` dışında koşulda (PS §1.2) — **yeni** |
| `E_WINDOW_LIMIT` | `limit` | `period`/`n`/`bars` > `program.max_window` (`expected` = sınır) |
| `E_TOO_MANY_NODES` | `limit` | > `program.max_nodes` |
| `E_TOO_LARGE` | `limit` | Bayt, çıktı ya da parametre sınırı — **yeni** |
| `E_COST_LIMIT` | `limit` | `per_bar` > `program.max_cost_per_bar` |
| `E_BUDGET` | `limit` | Kişi başına canlı bütçe ya da günlük baytı aşımı (§8.2) — **yeni** |
| `E_LOOKAHEAD` | `unsupported` | Negatif gecikme, kapanmamış bar, gelecek değer |
| `E_UNSUPPORTED` | `unsupported` | Bilerek olmayan yapı (döngü, işlev, çoklu sembol, emir, `tut`) |

Veri hataları (`dene` için) var olan `no_entitlement` ve `no_data` kodlarıdır (alt kodsuz).

### 11.3 Uyarılar

`check` hata dışında **uyarı** da döndürür (`warnings[]`, aynı biçim, `code` yok, `reason`
`W_`); uyarı reddetmez:

- `W_CONSECUTIVE_EVENTS`: olay çıktısının düğümü kenar biçimli değil (`cross_above`,
  `cross_below`, `once`, `rearm`, `bars ≥ 1` olan `cooldown` ya da bir işleneni bunlardan olan
  `and` değil); olay art arda barlarda doğabilir ve bildirim birleşir (PS §1.2, D1 şart 2).
- `W_UNUSED_NODE`: hiçbir çıktıya ulaşmayan düğüm (maliyete yine sayılır).

## 12. Metin dil

Küçük, satır tabanlı yazım yüzeyi; her dosya bir `qp-ir/0` programına iner, anlam bu
belgededir. Taban: `surface-a/DOC.md`; farkları aşağıda. Ayrıştırıcı elle yazılır (yeni
bağımlılık yok).

### 12.1 Sözdizimi

- Bir satır bir deyim; `#` satır sonuna kadar yorum; boş satır serbest.
- **Parametre:** `param <ad> int|float|bool = <varsayılan> [in <min>..<max>] [unit "<u>"]
  ["açıklama"]`; `int`/`float`'ta `in` zorunlu. Parametreler bağlamalardan önce yazılır.
- **Bağlama:** `ad = ifade`; her ad bir kez; yalnız önceki satırların adları. Ad
  `^[A-Za-z][A-Za-z0-9_]{0,63}$` (`_` ile başlamaz; §12.5). Parametre ve bağlama adları tek ad
  uzayıdır. Parametre adı ifadede sayı yerine geçer (sayı alanında parametre başvurusu, ifadede `const`).
- **Değerler:** `open high low close volume` (taban), `bar_index`, `time`, sayı, `true`,
  `false`, parametre adı. **Üst periyot:** `close@4h`, `high@1d`; birleşmeden önce `align(...)`.
- **İşleçler** (öncelik yüksekten alçağa): `x[n]` (`lag`), `-x`, `* /`, `+ -`,
  `< <= > >= == !=` (zincir yok), `not`, `and`, `or`. Parantez serbest.
- **Çok çıktı:** `macd(close, 12, 26, 9).signal`, `bollinger(close, 20, 2).upper`;
  `ad.çıktı` bağlanmış çok çıktılı düğümde de geçerli.
- **İşlevler:** `sma ema rsi highest lowest stdev percentrank` (`x, period`),
  `macd(x, fast, slow, signal)`, `bollinger(x, period, k)`, `atr(period[, tf])`,
  `min max abs`, `cross_above cross_below`, `streak bars_since once rearm cooldown`,
  `defined(x)`, `align(x)`, `line(t1, p1, t2, p2)`, `bucket(x, [e1, …])`. Tamsayı
  argümanlar sayı yazımı ya da `int` parametre adı olmalı.
- **Çıktılar:**

```
series ad = <num ifade>
event  ad = <bool ifade>
signal ad = <bool ifade> side al|sat [target 5%] [stop 3%] max_bars 20 source "strateji" "1"
state  ad { boyut1 = bucket(...), boyut2 = bucket(...) }
```

- **Ret:** dosyanın tek deyimi `reject <E_KOD> "kısa gerekçe"`; kod §11.2 alt kodlarından
  (ör. `E_LOOKAHEAD`, `E_UNSUPPORTED`). Ara gösterimde karşılığı
  `{"reject": {"code": "E_LOOKAHEAD", "reason": "…"}}`. Ret bir program değildir: dokümana
  eklenmez; `check` onu geçerli ret olarak tanır (`ok: true`, `reject: {…}`).
- **Yok:** döngü, koşullu ifade (`?:`, `if`), kullanıcı işlevi, atamayla durum (`var`),
  dizi, çoklu sembol, emir. Bunlar maliyetin statik, kökenin yapısal kalmasının şartıdır
  (`E_UNSUPPORTED`, ör. `for`, `var`, `if`, `def` anahtar sözcükleri tanınır ve kodlu retle
  karşılanır).

### 12.2 Anahtar sözcük ve dil

Tek kanonik sözcük kümesi: İngilizce işlev ve anahtar adları (yukarıda) ve sinyal yanı
`al`/`sat` (X25 değerleri). Türkçe anahtar sözcük kabul edilmez; öneri olarak eşlenir
(§11.1). İletiler ve açıklamalar iki dilde (`tr`, `en`).

### 12.3 İki yön

- **Metin → ara gösterim** (`qp_lang::text::lower`): bağlama adları düğüm kimliği olur;
  iç içe ifadelerin adsız düğümleri `_1, _2, …` kimliğini, programın tamamında soldan sağa
  son-sıra (post-order) numarasıyla alır. Aynı alt ifade iki kez yazılırsa iki düğüm olur
  (ortak alt ifade birleştirilmez; belirlenimcilik için).
- **Ara gösterim → kanonik metin** (`qp_lang::text::print`): parametreler, sonra her düğüm
  ara gösterim sırasıyla; kimliği `_` ile başlayan ve tam bir kez kullanılan düğüm
  kullanıldığı yere gömülür, öbürleri `kimlik = ifade` satırıdır (kimliği `_` ile başlayıp
  gömülemeyen düğüm, kullanılmayan en küçük `v<k>` adıyla yazılır). İşleç çevresinde tek
  boşluk, gereksiz parantez yok, tamsayı değerli sayılar `.0`'sız, öbür sayılar en kısa
  gidiş-dönüş yazımıyla.
- **Gidiş-dönüş özelliği (normatif, sınanır):** (a) her geçerli metin `t` için
  `lower(print(lower(t))) == lower(t)` (bayt eşit kanonik JSON); (b) her geçerli ara
  gösterim `p` için `lower(print(p))`, `p`'ye `_` kimliklerinin yeniden adlandırılması
  dışında eşittir; `p`'de `_` ile başlayan kimlik yoksa ya da `p` metinden indiyse bayt
  eşittir; (c) `print` eşgüçlüdür.

### 12.4 Hata biçimi

Metin hataları §11 biçimindedir ve `span` taşır. Bir satırdaki sözdizimi hatası sonraki
satırları durdurmaz (satır başından yeniden eşlenir); tip, saat ve maliyet hataları kaynak
satırına eşlenir.

### 12.5 Örnekler

Gösterge (seri):

```
param fast int = 12 in 2..200
param slow int = 26 in 2..400
diff = ema(close, fast) - ema(close, slow)
series spread = diff
series trigger = ema(diff, 9)
```

Koşul (olay; canlı yolda `threshold`):

```
param len int = 20 in 2..500
top = highest(high, len)[1]
event kirilim = close > top and defined(top)
```

Sinyal (sonuç kuralıyla; yalnız `dene`/yeniden oynatmada kayıt):

```
r = rsi(close, 14)
entry = cooldown(cross_below(r, 30), 12)
signal rsi_dip = entry side al target 3% stop 1.5% max_bars 24 source "rsi_dip" "1"
```

Üst periyot (seansa hizalı günlük ortalama):

```
daily_ma = align(sma(close@1d, 10))
event above_daily = cross_above(close, daily_ma)
```

Ret: `reject E_LOOKAHEAD "the next bar's close is not known at this bar's close"`

## 13. Dokümanda program

Oturuma bağlanma (D1: olay serisi, `threshold` koşulu, bildirim birleşmesi ve sınırları),
`qp-plane` hesap yolu, `qp-program` alt sistemi, protokol işlemleri ve sorguları, dökümler ve
dokunulan dosyalar: `docs/PROGRAMS-SESSION.md`.

## 14. Var olan hesaplarla eşdeğerlik (normatif)

Beş `add_study` hesabının her biri, aynı çekirdeği kullanan bir program olarak **bit eşit**
sonuç verir: aynı bar serisinde, `add_study` serisinin her çıktısı programın aynı adlı `series`
çıktısıyla, her barda ve üç kipte (toplu, akışkan, son barı düzelt) bit eşittir. Sınama
zorunludur (kabul ölçütü 7). Parametre aralığı `program.max_window` ile sınırlıdır.

```
# sma
param period int = 20 in 1..2000
series sma = sma(close, period)

# ema
param period int = 20 in 1..2000
series ema = ema(close, period)

# rsi
param period int = 14 in 1..2000
series rsi = rsi(close, period)

# macd
param fast int = 12 in 1..2000
param slow int = 26 in 1..2000
param signal int = 9 in 1..2000
m = macd(close, fast, slow, signal)
series macd = m.macd
series signal = m.signal
series histogram = m.histogram

# bollinger
param period int = 20 in 1..2000
param k float = 2 in 0.0001..100
b = bollinger(close, period, k)
series upper = b.upper
series middle = b.middle
series lower = b.lower
series bandwidth = b.bandwidth
```

`sma` programının ara gösterimi (öbürleri aynı kalıpla):

```json
{"version":"qp-ir/0",
 "params":[{"key":"period","kind":"int","min":1,"max":2000,"default":20}],
 "nodes":[{"id":"_1","op":"input","field":"close"},
          {"id":"_2","op":"sma","x":"_1","period":{"param":"period"}}],
 "outputs":[{"name":"sma","kind":"series","node":"_2"}]}
```

**İç içe gösterge adresi** (K63; kesilebilir): `ISCTR:5m:EMA[RSI:18]:20` →
`series ema = ema(rsi(close, 18), 20)`, `ISCTR` `5m` bar serisine bağlı (`qp_compute::Address`
ayrıştırır, `qp-lang` programa çevirir).

## 15. RX4 taslağından farklar

| # | Konu | RX4 (`rx4-ir/0`) | Qapsule (`qp-ir/0`) | Neden |
|---|---|---|---|---|
| 1 | Taban periyot | Sabit `1h` | Bağlı serinin aralığı; saat adı `base` | Qapsule serileri `1m`…`1M` |
| 2 | Üst periyot hizası | UTC gece yarısı (`T mod tf = 0`) | Enstrümanın borsa seansı (`qp-calendar`); takvimsiz / 7/24'te UTC; izinli periyot kuralı; eksik ilk üst bar oluşmaz | X30/5; `1d` Qapsule'de seans barı (`PROTOCOL.md` §18.2). RX4 uygunluğu UTC kipinde koşar |
| 3 | Canlı bar | Yok (yalnız kesin bar) | Seri çıktısında kopyasız geçici değer; olay serisinde canlı bar `null`; canlı barda hiçbir zaman olay/sinyal/kayıt yok; belirlenimcilik kapalı pencerede | Qapsule canlı veriyle çalışır (X30/3, Karar 2) |
| 4 | Parametre | Yok (her değişiklik yeni program) | `params` (kayıt defteri biçimi), `{"param": k}` başvurusu; örnek = program + değerler; iki katlı aralık denetimi | K78, X30/9; tarama, R7 çizim bağı |
| 5 | Program ve örnek kimliği | Kanonik JSON SHA-256 | Aynı + `sha256:` öneki; örnek kimliği = program kimliği + bağlanan değerler | Aynı programın iki örneği ayırt edilir |
| 6 | Hata kodları | Kendi kapalı kümesi | Qapsule'ün kapalı kodları (`code`), RX4 kodu alt kod (`reason`); yeni alt kodlar `E_VERSION`, `E_UNKNOWN_PARAM`, `E_PARAM_RANGE`, `E_NOT_BAR_SERIES`, `E_PROGRAM_SERIES`, `E_TOO_LARGE`, `E_BUDGET`, `E_TRY_ONLY`, `E_EVENT_OPERAND`; `did_you_mean` liste (≤ 5); uyarılar | `PROTOCOL.md` §13; `COMPUTE-LANGUAGE.md` §8.3 |
| 7 | Maliyet sınırı | 3000 bar için 2 000 000 birim (toplam) | Bar başına `program.max_cost_per_bar` (varsayılan 666 ≈ 2 000 000 / 3000); üst periyot düğümü indirimsiz | X30/6 bar sayısıyla ölçek; sınır parametre. Uygunlukta sınıra çok yakın görevde fark çıkabilir; çıkarsa raporda açıklanır |
| 8 | Bütçe | Yalnız statik | Statik + doküman içinde kişi başına canlı toplam ve günlük baytı (`Check` fazında) + çalışma bütçesi işi sayar (kayıt, açık kayıt, okuma); aşınca durur ve nedenini söyler | RX5 §7/1; K78 satılabilir hak |
| 9 | Sınır değerleri | Koda gömülü (200, 2000, 500) | Kayıt defteri parametreleri | K78, BRIEF |
| 10 | `fingerprint` | Çıktı türü adı | **Durum tanımı** (`state`); dokümana girmez, yalnız `dene`; anahtar boyut adlarıyla | Qapsule terimi (K79) |
| 11 | Çok çıktılı çekirdek | Yok (`macd`/`bollinger` önerildi) | `macd`, `bollinger` düğümleri; `id.çıktı` başvurusu, yalın `id` ana çıktı | Beş `add_study` hesabının eşdeğerliği |
| 12 | `defined` | Yok (önerildi) | `defined(x)` → hiç boş olmayan bool | RX4 W bulgusu |
| 13 | Çekirdeklerin yeri | `stdev`, `percentrank` RX4'ün kendi kodu | `qp-compute`'ta, `Core` kurallarıyla (toplu = akışkan = düzelt, üç mimari özeti); `highest lowest atr` R3/T3'ten var | Qapsule hesap çekirdeği kuralı |
| 14 | Köken `data` | Veri kümesinin SHA-256'sı | Bağlı seri + çapa + beslemeler | `PROTOCOL.md` §18.3, §19.2 |
| 15 | Ret | RX4 kodları | Alt kodlar (§11.2); `check` reti `ok: true, reject` diye tanır | Tek kod uzayı |
| 16 | Ajan araçları | `check / run / submit` (3 teslim sınırı) | `check_program / try_program / explain_program` sorguları + `add_program` işlemi (ajanda öneri); teslim sınırı yok (ölçüm kuralıydı) | `COMPUTE-LANGUAGE.md` §8.4; R5B |
| 17 | `run` veri kümesi | `seen` | Bağlı serinin kapanmış barları (okuyucunun yetkisiyle), `range` ile | Gerçek oturum verisi |
| 18 | Sinyal kaydı `mode` | Hep `"replay"` (alan gereksiz bulundu) | Kalır, hep `"replay"` + "simüle" etiketi; alanlar RX5 `LedgerEntry` adlarıyla, kimlik `t` ile, `born_t`/`known_t`; canlı kayıt RS1'de aynı biçimle `live` | X26, K77; RS1 aynı biçimi bekliyor |
| 19 | Metin dil | Parametre yok, `fingerprint` | `param` satırı, `.çıktı` seçici, `defined`, `state`; kanonik yazıcı ve gidiş-dönüş özelliği; Türkçe ad önerisi | §12 |

### 15.1 RX4 uygunluk yöntemi

- **Yer:** `qp-lang` sınaması (ya da `rounds/RL1/experiments/rx4/`); RX4 deney kodu kopyalanmaz.
  Girdi: `../refs/experiments/rx4/harness/solutions/*.ir.json` (26 program görevi),
  `harness/data/{seen,heldout}.json` (sentetik, 3000 × 1h), beklenen
  `harness/expected/<id>/{seen,heldout}.json`.
- **Mekanik çeviri** (kod, elle değil): `version` → `qp-ir/0`; `tf: "1h"` silinir (taban),
  `4h`/`1d` kalır; `fingerprint` → `state`; RX4 hata kodları → `reason`; `rx4-ir/0`'da olmayan
  alan yok. Çeviri dışı fark çıkarsa hata ya da bu bölümde yazılı sapmadır.
- **Koşu:** belge içi bar serisi, aralık `1h`, takvimsiz (UTC kipi), **kapalı pencere** (RX4
  verisinde canlı bar yok); `try_program` çekirdeği, bütçe sınırsız.
- **Karşılaştırma** `harness/score.py` `compare_outputs` kuralıyla: yalnız görevin çıktıları;
  seri değerleri `|a−b| ≤ 1e-9·max(1,|b|)` (aynı çekirdek beklendiğinden ayrıca bit eşitlik
  sayılır ve raporlanır), olay barları birebir, sinyal `(bar, side, outcome, exit_bar)`
  birebir, durum anahtarları birebir. Ret görevleri: alt kod eşit. Maliyet ve hata sınırı
  karşılaştırılmaz (§15/7).
- **Sonuç tablosu** görev × veri kümesi; her fark açıklanır (kabul ölçütü 15).

## 16. Bilerek yok (`qp-ir/0`)

Döngü, özyineleme, koşullu ifade, kullanıcı işlevi, dizi/koleksiyon, değişkenle durum
(`var`), çoklu sembol (program tek bar serisine bağlı), emir, `tut` sinyali, oluşan
(kapanmamış) üst bar okuma, `exp/ln/pow`, Pine'dan alma (sonraya, yalnız sıkı kip; X30/10),
canlıda sinyal iddiası ve sicil (RS1), çizim bağı (R7), program yazma ekranı (R7).

## 17. Açık sorular

1. **Kişi başına parametre.** `SessionView::param(name)` kişi almıyor ve otoritenin
   uygulaması bugün `None` döner; açılım bağlamında (`qp_ops::Ctx`) `param` hiç yok. Plan
   kapsamlı `program.live.doc.*` değerleri kişiye göre çözülemez; bu round'da kayıt defterinin
   sistem varsayılanı geçerlidir. Eksik kanca: `param_for(principal, key)` (`SessionView` ve
   açılımda; R7/RS1 öncesi karar).
2. **Keşfedilebilirlik.** `welcome.limits` ve `describe` bu round değişmez (var olan dökümler);
   sınırlar `check_program` cevabında, yüzey `docs/AGENTS.md` ve MCP araç tanımlarında
   tanıtılır. `describe`'a ekleme var olan dökümleri bozmuyorsa uygulayıcı ekler, bozuyorsa
   eklemez ve rapora yazar.
3. **Takvimsiz BIST serisi.** Hiza `instrument.calendar`'dan gelir; takvimi yazılmamış BIST
   serisi UTC'ye hizalanır (yanlış `1d`/`4h`). Barındırıcı mı doldurur, borsa kodundan
   (`exchange: BIST` → `XIST`) çıkarım mı? Bu şartname çıkarım yapmaz.
4. **Pencere sınırı.** `program.max_window` varsayılanı 2000 (RX4); `add_study` 100 000'e
   kadar dönem kabul eder; daha uzun dönemli hesap varsayılan sınırla programa çevrilemez.
5. **GOAL'deki "yeni çekirdekler".** `highest lowest atr` R3/T3'ten beri `qp-compute`'ta; yeniden
   yazılmaz, yeni olan yalnız `stdev` ve `percentrank`.
