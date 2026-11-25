# Qapsule hesap dili — ajan kartı (`qp-ir/0`)

Bu kart ölçülen ajanın **tek belgesidir**. `docs/PROGRAMS.md`'den (normatif şartname; bölüm
numaraları `PR §n`) ve `docs/PROGRAMS-SESSION.md` §3'ten (`PS §3`) türetildi; yeni kural
eklemez, yalnız kısaltır. Çelişki olursa şartname geçerlidir ve çelişki puanlamada not edilir.

## 1. Veri ve saat (PR §1, §3)

- Program **tek bir bar serisine** bağlanır (`inputs`). Bar `{t, o, h, l, c, v}`; `t` barın
  açılış zamanı (epoch ms, UTC); bar indeksi 0'dan başlar. Taban periyot = serinin aralığı.
- Tipler: `num` (sayı ya da boş), `bool` (doğru/yanlış ya da boş), `cat` (kutu indeksi; yalnız
  durum tanımında ve `defined`'da). Aritmetik, karşılaştırma ve mantıkta **boş yayılır**;
  durum düğümleri boş `bool`'u yanlış sayar; çekirdekler boş girdiyi atlar (durumları ilerlemez).
- **Üst periyot:** `close@4h`, `high@1d`. Takvimsiz seride UTC hizası (`4h` = UTC 00/04/08…,
  `1d` = UTC günü). Üst periyot serisi yalnız **kapanmış** üst barlardan oluşur; tabana
  `align(x)` ile iner: taban bar `i`'nin kapanışında kapanmış son üst barın değeri (yoksa boş).
  Farklı saatli değerler `align`'sız birleşemez (`E_CLOCK_MISMATCH`); çıktı tabanda olmalı.
- **Geleceğe bakma ifade edilemez:** negatif gecikme, kapanmamış bar, gelecek değer → `E_LOOKAHEAD`.

## 2. Metin dil (PR §12.1)

- Bir satır bir deyim; `#` yorum; boş satır serbest.
- **Bağlama:** `ad = ifade` (ad `^[A-Za-z][A-Za-z0-9_]{0,63}$`, `_` ile başlamaz; her ad bir
  kez; yalnız önceki satırların adları).
- **Değerler:** `open high low close volume`, `bar_index`, `time`, sayı, `true`, `false`,
  parametre adı; üst periyot `close@4h`.
- **İşleçler** (yüksekten alçağa): `x[n]` (n bar önceki değer, `n ≥ 0`), `-x`, `* /`, `+ -`,
  `< <= > >= == !=` (zincir yok), `not`, `and`, `or`. Parantez serbest.
- **Çok çıktı:** `macd(close, 12, 26, 9).signal`, `bollinger(close, 20, 2).upper`; bağlanmış
  adla da: `m = macd(close, 12, 26, 9)` sonra `m.histogram`.
- **Parametre (isteğe bağlı):** `param <ad> int|float|bool = <varsayılan> [in <min>..<max>]`;
  `int`/`float`'ta `in` zorunlu; bağlamalardan önce yazılır.
- Tamsayı argümanlar (`period`, `n`, `bars`, `max_bars` …) sayı yazımı ya da `int` parametre.
- **Yok:** döngü, `if`/`?:`, kullanıcı işlevi, `var`, dizi, çoklu sembol, emir (`E_UNSUPPORTED`).
  Türkçe işlev adı kabul edilmez (öneri olarak İngilizcesi döner).

## 3. İşlevler (PR §4; tanımlar tanımlı değer dizisi üzerinde)

| İşlev | Tip | Anlam |
|---|---|---|
| `sma(x, p)` | num | Son `p` değerin ortalaması; ilk değer `p`'inci tanımlı değerde |
| `ema(x, p)` | num | `a = 2/(p+1)`; ilk değer ilk `p` değerin ortalaması; sonra `x·a + önceki·(1−a)` |
| `rsi(x, p)` | num | Wilder; ilk değer `p+1`'inci değerde; kayıp yoksa 100, hareket yoksa 50 |
| `highest(x, p)` / `lowest(x, p)` | num | Son `p` değerin (bu bar dahil) en büyüğü / en küçüğü |
| `stdev(x, p)` | num | Son `p` değerin **nüfus** standart sapması (`/p`) |
| `percentrank(x, p)` | num | `100 × #{k ∈ 1..p : x[k önce] ≤ x} / p`; ilk değer `p+1`'inci değerde |
| `atr(p)` / `atr(p, 4h)` | num | O saatin `h,l,c`'si; TR ilk barda `h−l`, sonra `max(h−l, abs(h−pc), abs(l−pc))`; ilk değer ilk `p` TR'nin ortalaması, sonra Wilder. Üst periyotlu sonuç `align` ister |
| `macd(x, fast, slow, signal)` | num | Çıktılar `.macd` (ana), `.signal`, `.histogram` |
| `bollinger(x, p, k)` | num | Çıktılar `.upper`, `.middle` (ana), `.lower`, `.bandwidth`; nüfus std |
| `min(a, b)`, `max(a, b)`, `abs(x)` | num | `/` sıfıra bölmede boş; sonlu olmayan sonuç boş |
| `cross_above(a, b)` | bool | `a[1] < b[1]` ve `a ≥ b`; `a` ya da `b` boşsa boş; önceki değerlerden biri boşsa (ya da ilk barda) yanlış |
| `cross_below(a, b)` | bool | `a[1] > b[1]` ve `a ≤ b`; aynı boş kuralı |
| `streak(c)` | num | `c` üst üste kaç bardır doğru (bu bar dahil); `c` yanlışsa 0 |
| `bars_since(c)` | num | Son doğru bardan beri bar sayısı (doğru barda 0); ilk doğrudan önce boş |
| `once(c)` | bool | Yalnız `c`'nin ilk doğru olduğu barda doğru |
| `rearm(trigger, reset)` | bool | Başta kurulu. Kuruluysa ve `trigger` doğruysa → doğru, kurulum düşer. Kurulu değilse ve `reset` doğruysa → yeniden kurulur (o barda tetiklemez) |
| `cooldown(trigger, bars)` | bool | `trigger` doğru ve (önceki ateşleme yok ya da `i − i₀ > bars`) → doğru; `i₀` son ateşleme barı; düşen tetik ertelenmez |
| `defined(x)` | bool | `x` bu barda tanımlıysa doğru; hiç boş değildir |
| `align(x)` | x'in tipi | Üst periyot değerini tabana indirir (§1) |
| `line(t1, p1, t2, p2)` | num | Çapalar `(t1, p1)`, `(t2, p2)` (`t1 < t2`, ms); bar indeksinde doğrusal; yalnız `t ≥ t2` olan barlarda tanımlı |
| `bucket(x, [e1, …])` | cat | Kesin artan 1…8 sınır: `x < e1` → 0; `e(k) ≤ x < e(k+1)` → k; `x ≥ son` → sınır sayısı |

## 4. Çıktılar (PR §5, §6)

```
series ad = <num ifade>
event  ad = <bool ifade>
signal ad = <bool ifade> side al|sat [target 5%] [stop 3%] max_bars 20 source "strateji" "1"
state  ad { boyut1 = bucket(...), boyut2 = bucket(...) }
```

- Çıktı adı `^[a-z][a-z0-9_]{0,31}$`; en az bir, en çok 4 çıktı; çıktı düğümü tabanda.
- **Olay:** düğümün doğru olduğu kapanmış barlar (boş olay değildir).
- **Sinyal:** düğümün doğru olduğu her bar bir kayıt; `side` yalnız `al`/`sat`; sonuç kuralı
  zorunlu (`max_bars` 1…500; `target`, `stop` > 0 isteğe bağlı). Giriş = sinyal barının
  kapanışı; sonuç sonraki barlardan: önce durdurma, sonra hedef, aynı barda ikisi → `stop`;
  `max_bars` dolarsa `expired`; veri biterse `open`. Sinyal ve durum tanımı **dokümana
  eklenmez** (yalnız `check`/`try`; `add_program` → `E_TRY_ONLY`).
- **Durum tanımı:** 1…4 boyut, her boyut bir `bucket`; bar başına anahtar boyut sırasıyla;
  bir boyut boşsa anahtar boş.

## 5. Ret ve hata kodları (PR §11, §12.1)

- İstek ifade edilemiyorsa ya da edilmemeliyse program yerine **ret**: dosyanın tek deyimi
  `reject <E_KOD> "kısa gerekçe"` (IR'de `{"reject": {"code": "E_…", "reason": "…"}}`).
  `check` onu geçerli ret olarak tanır (`ok: true`, `reject`). Gereksiz ret yanlıştır.
- Hata nesnesi: `code` (kapalı kod), `reason` (alt kod), `message`, `at` (IR yolu), `span
  {line, col, len}` (metin), `did_you_mean`, `hint`. `check` **bütün hataları birden** döndürür.

| Alt kod | Ne zaman |
|---|---|
| `E_PARSE` | Metin sözdizimi bozuk |
| `E_UNKNOWN_OP` | Bilinmeyen işlev (dilde olmayan gösterge) |
| `E_UNKNOWN_FIELD` / `E_UNKNOWN_REF` / `E_UNKNOWN_PARAM` | Bilinmeyen alan / tanımsız ad ya da olmayan çıktı / tanımsız parametre |
| `E_TYPE`, `E_MISSING_ARG`, `E_BAD_PARAM`, `E_PARAM_RANGE` | Tip; eksik argüman; geçersiz sabit (0 periyot, sırasız sınır, `t1 ≥ t2`); aralık dışı |
| `E_UNKNOWN_TF`, `E_CLOCK_MISMATCH`, `E_OUTPUT_CLOCK` | İzinsiz periyot; `align`'sız saat karışımı; çıktı tabanda değil |
| `E_NO_OUTPUT`, `E_DUPLICATE_NAME` | Çıktı yok; ad tekrarı |
| `E_NO_OUTCOME_RULE`, `E_BAD_RULE` | Sinyalde kural yok; kural değeri geçersiz |
| `E_WINDOW_LIMIT`, `E_TOO_MANY_NODES`, `E_TOO_LARGE`, `E_COST_LIMIT`, `E_BUDGET` | Pencere > 2000; düğüm > 200; boyut; bar başı maliyet > 666; kişi bütçesi |
| `E_LOOKAHEAD` | Geleceğe bakma: negatif gecikme, kapanmamış bar, gelecek değer |
| `E_UNSUPPORTED` | Bilerek olmayan yapı: döngü, koşullu ifade, kullanıcı işlevi, çoklu sembol, emir, `tut` |
| `E_TRY_ONLY` | `add_program`'da sinyal ya da durum tanımı çıktısı |

## 6. Ara gösterim (JSON) kısaca (PR §2, §4)

Metin yerine doğrudan IR de yazılabilir:

```json
{"version":"qp-ir/0",
 "nodes":[{"id":"c","op":"input","field":"close"},
          {"id":"r","op":"rsi","x":"c","period":14},
          {"id":"k","op":"const","value":70.0},
          {"id":"x","op":"cross_above","a":"r","b":"k"}],
 "outputs":[{"name":"ust","kind":"event","node":"x"}]}
```

Düğümler topolojik sırada (yalnız öncekilere başvuru). İşlem adları: `input {field, tf?}`,
`const {value}`, `bar_index`, `time`, `sma|ema|rsi|highest|lowest|stdev|percentrank {x, period}`,
`atr {period, tf?}`, `macd {x, fast, slow, signal}`, `bollinger {x, period, k}`,
`add|sub|mul|div|min|max {a, b}`, `neg|abs {x}`, `lag {x, n}`, `gt|ge|lt|le|eq|ne {a, b}`,
`and|or {a, b}`, `not {x}`, `cross_above|cross_below {a, b}`, `streak|bars_since|once {c}`,
`rearm {trigger, reset}`, `cooldown {trigger, bars}`, `defined {x}`, `align {x}`,
`line {a:{t,price}, b:{t,price}}`, `bucket {x, edges}`. Çok çıktıya başvuru `"m.signal"`.
Çıktılar: `{"name","kind":"series|event","node"}`; sinyal `+ "side","rule":{"target_pct"?,
"stop_pct"?,"max_bars"},"source":{"strategy","version"}`; durum `{"name","kind":"state",
"dims":[{"name","node"}]}`. Bilinmeyen alan reddedilir.

## 7. Araçlar (PS §3.2–3.3)

Programı `source` (metin) **ya da** `program` (IR) ile verirsin; `inputs` bağlı seri.

| Araç | Ne verir |
|---|---|
| `check_program` | `ok`, `errors[]` (hepsi birden), `warnings[]`, `cost`, `outputs`, `ir` (metinden indiyse), `reject?`. Çalıştırmaz |
| `try_program` | Kapanmış barlarda koşar, **özet** döner: seri (tanımlı sayısı, son değerler), olay (sayı, ilk/son olaylar), sinyal (sayı, sonuç dağılımı, ilk kayıtlar), durum (anahtar sayısı). Tam çıktı vermez |
| `explain_program` | Kanonik metin, düğüm başına maliyet, çıktılar |
| `add_program` | Programı dokümana **öneri** olarak ekler (kararı insan verir); yalnız seri/olay çıktılı programlar |

## 8. Örnekler (PR §12.5)

```
param len int = 20 in 2..500
top = highest(high, len)[1]
event kirilim = close > top and defined(top)
```

```
r = rsi(close, 14)
entry = cooldown(cross_below(r, 30), 12)
signal rsi_dip = entry side al target 3% stop 1.5% max_bars 24 source "rsi_dip" "1"
```

```
daily_ma = align(sma(close@1d, 10))
event above_daily = cross_above(close, daily_ma)
```

```
atr4 = align(atr(14, 4h))
event wide = (high - low) > 0.5 * atr4
```

```
r = rsi(close, 14)
vol_ratio = volume / sma(volume, 20)
state regime {
  momentum = bucket(r, [30, 50, 70]),
  activity = bucket(vol_ratio, [0.8, 1.2])
}
```

`reject E_LOOKAHEAD "the next bar's close is not known at this bar's close"`
