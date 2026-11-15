# R2/T9 — Ajan yazabilirlik ölçümü: anlamla başvuru (base, A seçiciler, B adlar)

Dal `r2/t9-ergo` (taban `r2/int` 0cfaa26). Görevler: `rounds/R2/tasks/TASKS.md` R01–R20.
Ölçüt: `GOAL.md` 9, `VERIFY.md` 12. Bu ölçümün koşu etiketi `v1`; düzeltme sonrası yeniden
ölçüm `v2` olacak.

## 1. Kurulum

| Değişke | Dal (uç) | Paket |
|---------|----------|-------|
| `base` | `r2/int` (0cfaa26): yalnız kimlik, `welcome.summary` var | `docs/PROTOCOL.md` + `docs/GRAMMAR.md` + `qp schema export` + `run.sh` |
| `A` | `r2/t8a-selectors` (d2f04e8): seçici nesneleri (`{"shows":{"study":"rsi"}}`) | aynısı, o dalınki |
| `B` | `r2/t8b-names` (86ce4c4): metin adları (`"@panel:rsi(14)"`) | aynısı, o dalınki |

Her değişkenin `qp`'si kendi geçici worktree'sinde, ayrı hedef klasöründe derlendi (dallar
değişmedi). `GRAMMAR.md` üç dalda aynı; farklar `PROTOCOL.md` §17'de ve şemada.

- **Ajanlar:** değişke başına 5 ajan × 4 görev (A: R01 R06 R11 R16; B: R02 R07 R12 R17;
  C: R03 R08 R13 R18; D: R04 R09 R14 R19; E: R05 R10 R15 R20; üç değişkende aynı bölüm).
  Model `sonnet` (`claude-sonnet-5-5`), her ajan taze bir `claude -p` süreci, en çok 5 eşzamanlı.
  Ajan kimliği: `editor`, `headless`, kullanıcı `arkin` (masaüstünde bağlı) adına.
- **Yönerge:** RD1 `raw/v2r/AGENTS.md` metni; yollar uyarlandı (paket = şartname + gramer +
  şema; tek komut `run.sh`; "run.sh'ın hedefini okuma" cümlesi "başka hiçbir dosyayı okuma"
  oldu, çünkü `run.sh` artık yalnız bir istemci). Birebir metin: her ajanın `prompt.txt`'si.
  İstek metinleri: `harness/t9.py` `REQUESTS` (R01'de "RSI(7)", R04/R19'da "son kırılım
  bulgusu" açıkça yazıldı, çünkü görev tablosundaki parantez içi bağlamı ajanın başka yoldan
  bilemezdi).
- **Gizli kurulum** (`harness/setup.py`): RD1 `thyao` dokümanı, **kimlikleri her ajan için
  yeniden rastgele** (seri, görünüm, katman, olgu, bulgu, kapsül; 20 kimlik, 5 karakter,
  `random.Random(seed)`); ayrıca SMA(50) (görünümde değil), bar 120 ve 185'te iki `breakout`
  olgusu (`evidence.level` 103,90 / 122,40), onlara dayanan iki bulgu + canlı kapsül
  (doküman sırası bar sırası değil; yeni kırılımın güveni en yüksek değil, böylece bulgusuz
  `present` yanlış bulguyu seçer), `arkin` masaüstünde. Tohum = sha256(`r2-t9/v1/<değişke>/<ajan>`);
  tohum ve kimlik eşlemesi `raw/v1/<değişke>/agent<X>/setup.json`'da. Şartname örneklerindeki
  kimliklerin (`px`, `rsi14`, `main`…) hiçbiri dokümanda yok. Panel numaraları 0/1/2 kaldı
  (bunlar konumdan türeyen `qp/0` numaraları; özet de veriyor).
- **Gizli son ek:** `document`, `screen` (arkin), `log` ve R14 için başvuru noktası sorgusu
  (`hit` 900,200 @1200×600, taze bir `headless` bağlantıdan).

### Düzenek düzeni (`rounds/R2/ergonomics/`)

```
harness/setup.py     gizli kurulum (tohumdan doküman, kurulum ve son ek satırları)
harness/t9.py        packet | serve <değişke> | launch <değişke> <ajan> | isotest
harness/score.py     puanlama (yalnız ham kayıtlardan; yeniden koşunca aynı sayılar)
harness/selftest.py  puanlayıcı öz-denetimi: kimlikli başvuru çözümleri 20/20 geçer, boş deneme 20/20 kalır (üç ikilide)
harness/audit.py     dökümlerden her araç çağrısı: okuma/yazma/komut sınırları
harness/oscheck.sh   işletim sistemi düzeyinde okuma/yazma yoklaması
raw/isotest/base/agentA/   yalıtım testi (döküm, os-check.txt, audit.txt, daemon-rejected.log)
raw/v1/<değişke>/agent<X>/ setup.json, prompt.txt, TASKS.txt, sandbox.sb, command.json,
                           transcript.jsonl (+ .ts: her satırın varış zamanı), process.json,
                           runs.log, R??/runs/<n>-<dosya>.jsonl (+ .full.jsonl: otoritenin tam çıktısı),
                           R??/{attemptN.jsonl, notes.txt, answer.txt} (ajanın bıraktığı)
raw/v1/<değişke>/score.json, raw/v1/SCORE.md, raw/v1/audit.txt
```

`run.sh` küçük bir istemcidir: deneme dosyasının yolunu ve ajanın belirtecini paketteki
Unix soketinden (`qp.sock`) korumalı alan dışındaki daemon'a gönderir. Daemon gizli kurulum +
ajanın satırları + gizli son eki o değişkenin `qp serve`'ünden geçirir, yalnız ajanın
bağlantılarına giden mesajları döner, tam çıktıyı (ajanın erişemediği) `raw/` altına yazar.

## 2. Yalıtım (dosya erişimi yönergeyle değil, işletim sistemiyle)

Her ajan `sandbox-exec` altında çalıştı (`sandbox.sb` her ajan klasöründe):
`/Users/arkns/Works`, bu oturumun `/private/tmp/claude-501/-Users-arkns-Works-Mchart-qapsule`
ağacı ve `~/.claude/projects/-Users-arkns-Works-Mchart-qapsule` için okuma reddi; yalnız
paket klasörü ve ajanın kendi çalışma klasörü açık. Claude Code `--tools Read,Write,Bash`,
`--allowedTools Read Edit(//<iş klasörü>/**) Bash(<paket>/run.sh:*)`, `--setting-sources local`
(kullanıcı eklenti kancaları yüklenmesin), `--strict-mcp-config`, `--disable-slash-commands`.

**Yalıtım testi** (`raw/isotest/base/agentA/`), korumalı bir `claude -p`'ye yaptırıldı:

| Deneme | Sonuç |
|--------|-------|
| Read depo `CLAUDE.md`, `harness/setup.py`, `rounds/R2/tasks/TASKS.md` | `EPERM: operation not permitted` (üçü de) |
| Read paket `PROTOCOL.md` | okundu |
| Write + `run.sh` kendi `R01/probe.jsonl` | çalıştı, `welcome` döndü |
| Write depoya (`rounds/R2/ergonomics/PWNED.txt`) ve başka ajanın klasörüne | reddedildi |
| `run.sh` başka ajanın dosyasıyla (`work/v1/base/agentB/R02/attempt2.jsonl`) | `error: this file is not in your own work dir` (daemon `daemon-rejected.log`) |
| Bash `cat CLAUDE.md` | reddedildi |

İşletim sistemi düzeyi (`os-check.txt`, Claude Code dışında aynı profille `head`/`ls`/`touch`):
depo dosyaları, gizli doküman, `qp` ikilisi, proje oturum klasörü okunamıyor
(`Operation not permitted`); depoya, başka ajanın klasörüne, gizli doküman klasörüne ve pakete
yazılamıyor; kendi klasörüne yazılabiliyor.

### Bütünlük düzeltmesi (orkestratörün güvenlik incelemesi) ve zaman çizelgesi

İnceleme iki açık buldu: (1) yazmalar kısıtsızdı, (2) ajan başka bir ajanın/değişkenin yoluna
deneme gönderebilirdi. Düzeltmeler: profile depo ve oturum ağaçları için `file-write*` reddi +
yalnız kendi klasörüne izin; Write aracına `Edit(//<iş klasörü>/**)` kuralı (Claude Code yol
kalıbını yalnız `Edit(...)` kurallarında uyguluyor, `Write(...)` kalıbı eşleşmiyor — denendi);
ajan başına rastgele belirteç (`<iş klasörü>/.qp-token`, yalnız o ajan okuyabilir; daemon
belirteci yolun ajanıyla eşleştirir, değilse reddeder). Yalıtım testi yazma ve çapraz gönderim
yoklamalarıyla yeniden koşuldu (yukarıdaki tablo).

- **`base` ve `A` eski profille koşuldu** (yazma reddi ve belirteç yoktu; okuma reddi vardı).
  Dökümler denetlendi: depo dışına ya da kendi klasörü dışına **yürütülmüş yazma 0**,
  başka ajanın yoluna **gönderim 0** (her `run.sh` çağrısı kendi klasöründeki bir dosyaya;
  daemon günlükleri de yalnız kendi ajan klasörlerine düşüyor). Bu nedenle yeniden koşulmadı.
- **`B` yeni profille koşuldu.**
- `audit.py` katı kuralla (shlex; yalnız `<paket>/run.sh <kendi dosyası>`, isteğe bağlı
  `| head`/`| tail` + `-n N`/`-c N`) 34 çağrıyı "sınır dışı" sayıyor; hepsi elle incelendi:
  `run.sh <kendi dosyası> | cut -c…`, `cd <kendi klasörü> && run.sh …` / `cd …; run.sh …`
  ve bir kez `echo … > R05/notes.txt` (kendi klasörü). Kendi klasörü dışına dokunan yok.
  Reddedilen çağrılar 58 (çoğu `ls schema/`, heredoc ile dosya yazma, `cp`).
- **Sınırın niteliği:** yazmalar yalnız depo ve oturum ağaçlarında reddediliyor (profil
  bunların dışında varsayılan izinli); zorlanan sınır **okumadır**. Öneri: ajan başına ayrı
  `HOME` ve varsayılan-red yazma profili.

## 3. Sonuçlar

Puanlama `harness/score.py v1` (tam tablo `raw/v1/SCORE.md`; `score.json`'lar yeniden
koşuda bayt aynı). Tanımlar:

- **Deneme** = ajanın kendi numaralandırması (`attempt1..3`). Deneme, o görev klasöründeki
  ilk koşusuyla puanlanır; ajan aynı dosyayı başka bir görev klasöründen koşmuşsa o koşu
  kullanılır; hiç koşulmamış deneme başarısızdır (ör. `base` ajanlarının "ortak keşif koşusu
  R02'de, burada çoğaltılmadı" dediği `attempt1`'ler). Numarasız koşular (`probe.jsonl`,
  `disc.jsonl`) deneme sayılmaz ama aşağıda ayrıca gösterilir.
- **İlk denemede** = `attempt1` ölçütü geçiyor. **Kimliksiz ilk deneme** = ayrıca `attempt1`
  dokümanın (rastgele) hiçbir kimliğini içermiyor, yani bir keşif koşusundan kopyalanmadan
  yazıldı. **Sonunda** = son koşulan deneme geçiyor (≤ 3).
- **Belirteç** = ajanın son `result` olayındaki kullanım (girdi + önbellek yazma + önbellek
  okuma + çıktı). Ajanlar dört görevini iç içe yürüttüğü için görev başına belirteç/süre
  ajanın toplamının dörtte biri olarak verilir.

### 3.1 Değişke tablosu

| Değişke | İlk denemede | Kimliksiz ilk deneme | Sonunda | İlki başarısız → sonunda | İlki hata aldı → sonunda | Deneme | Mesaj (ilk / hepsi) | Numarasız keşif koşusu | Belirteç | Çıktı belirteci | Süre (ajan toplamı) | Maliyet |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| base | **7/20** | 2/20 | **20/20** | 13/13 | 2/2 | 34 | 66 / 107 | 0 (+1 üzerine yazılmış) | 3 189 676 | 47 124 | 387 s | 2,06 $ |
| A | **19/20** | **19/20** | **19/20** | 0/1 | 0/0 | 20 | 55 / 55 | 1 | 2 506 378 | 36 482 | 293 s | 1,89 $ |
| B | **17/20** | 10/20 | **20/20** | 3/3 | 1/1 | 25 | 53 / 67 | 1 | 2 642 064 | 36 264 | 302 s | 1,87 $ |

Ölçülen ajanların toplam belirteci: **8 338 118** (15 ajan, 5,82 $); yalıtım testleri ayrıca
≈ 26 000 belirteç/koşu.

### 3.2 Görev matrisi (ilk / son, deneme sayısı)

| Görev | base | A | B |
|---|---|---|---|
| R01 RSI(7) ayrı panel, fiyatın altı | ✗ / ✓ (3) | ✓ / ✓ (1) | ✓ / ✓ (1)\* |
| R02 üst bant kırmızı | ✗ / ✓ (2) | **✗ / ✗ (1)** | ✗ / ✓ (2) |
| R03 son kırılımı vurgula | ✗ / ✓ (2) | ✓ / ✓ (1) | ✓ / ✓ (1) |
| R04 saat: ne söylenir | ✗ / ✓ (2) | ✓ / ✓ (1) | ✓ / ✓ (1) |
| R05 fiyat paneli log | ✗ / ✓ (2) | ✓ / ✓ (1) | ✗ / ✓ (2) |
| R06 RSI'a 30 çizgisi | ✓ / ✓ (1) | ✓ / ✓ (1) | ✓ / ✓ (1)\* |
| R07 MACD panelini kaldır | ✗ / ✓ (2) | ✓ / ✓ (1) | ✓ / ✓ (1) |
| R08 hacim çubuk, fiyatın altı | ✗ / ✓ (2) | ✓ / ✓ (1) | ✓ / ✓ (2) |
| R09 Bollinger dolgulu | ✗ / ✓ (2) | ✓ / ✓ (1) | ✓ / ✓ (1) |
| R10 histogram çubuk, işarete göre renk | ✓ / ✓ (1) | ✓ / ✓ (1) | ✓ / ✓ (1)\* |
| R11 kapanış SMA50'ye göre renk | ✓ / ✓ (1) | ✓ / ✓ (1) | ✓ / ✓ (1)\* |
| R12 son 60 bar, kalıcı | ✗ / ✓ (2) | ✓ / ✓ (1) | ✓ / ✓ (1) |
| R13 ekranda ne var | ✓ / ✓ (1) | ✓ / ✓ (1) | ✓ / ✓ (1) |
| R14 (900,200)'de ne var | ✓ / ✓ (1) | ✓ / ✓ (1) | ✓ / ✓ (1) |
| R15 RSI çizgisi 2 px | ✓ / ✓ (1) | ✓ / ✓ (1) | ✓ / ✓ (1)\* |
| R16 eski kırılım olgusu nokta | ✓ / ✓ (1) | ✓ / ✓ (1) | ✓ / ✓ (1)\* |
| R17 123,45'te yatay çizgi | ✗ / ✓ (2) | ✓ / ✓ (1) | ✗ / ✓ (2) |
| R18 MACD'yi RSI'ın üstüne | ✗ / ✓ (2) | ✓ / ✓ (1) | ✓ / ✓ (2) |
| R19 telefonda kaç bar | ✗ / ✓ (2) | ✓ / ✓ (1) | ✓ / ✓ (1) |
| R20 bant genişliği ayrı panel | ✗ / ✓ (2) | ✓ / ✓ (1) | ✓ / ✓ (1) |

\* B'de ilk deneme geçti ama dokümanın kimlikleriyle yazıldı (ajan önce numarasız bir
`probe.jsonl` koştu ya da başka görevin keşif koşusundan kimlik aldı). `base`'in ilk deneme
başarılarının 5'i de böyle (R06 R10 R11 R15 R16).

### 3.3 Ajan başına

| Değişke/ajan | Belirteç | Süre | Tur | Araç çağrısı |
|---|---:|---:|---:|---:|
| base A / B / C / D / E | 806 334 / 683 876 / 481 893 / 515 467 / 702 106 | 95 / 69 / 72 / 75 / 76 s | 28 / 23 / 26 / 29 / 30 | 27 / 22 / 25 / 28 / 29 |
| A A / B / C / D / E | 487 848 / 489 496 / 473 694 / 480 641 / 574 699 | 59 / 63 / 53 / 59 / 60 s | 21 / 21 / 22 / 24 / 24 | 20 / 20 / 21 / 23 / 23 |
| B A / B / C / D / E | 656 073 / 490 624 / 631 869 / 415 983 / 447 515 | 63 / 66 / 68 / 52 / 52 s | 25 / 23 / 25 / 23 / 19 | 24 / 22 / 24 / 22 / 18 |

### 3.4 RD1 v2r ile karşılaştırma

RD1 v2r (kimlikler yeniden adlandırılmış ama sabit; erişim yönergeyle): ilk deneme 6/20,
sonunda 19/20. Görev kümeleri **farklı** (RD1: protokol görevleri G01–G20 — öneri, geri
alma, abonelik, karne; R2: gramer ve anlamla başvuru R01–R20), ve bu ölçümde kimlikler ajan
başına rastgele, erişim işletim sistemiyle zorlanıyor.

| | İlk denemede | Sonunda |
|---|---|---|
| RD1 v2r (G01–G20) | 6/20 | 19/20 |
| R2 base (R01–R20, kimlik + `welcome.summary`) | 7/20 (kimliksiz 2/20) | 20/20 |
| R2 A seçiciler | 19/20 | 19/20 |
| R2 B adlar | 17/20 (kimliksiz 10/20) | 20/20 |

`base` RD1'deki örüntüyü tekrarlıyor: ajanlar kimlikleri bilmediği için ilk denemeyi
keşfe harcıyor (5 ajandan 4'ü açıkça "discovery-only run" yazdı) ve ikinci denemede
neredeyse her şeyi bitiriyor. `welcome.summary` tek başına ilk denemeyi kurtarmıyor, çünkü
ajan cevabı koşudan önce göremiyor. Anlamla başvuru bu turu kaldırıyor.

## 4. Başarısızlık çözümlemesi

### Ortak: R02 "üst bandı kırmızı yap" (A ✗/✗, B ✗/✓, base ✗/✓)

Bollinger tek bir `line` katmanıyla çiziliyor (üç çıktı). Her iki tasarımda "üst bant"
başvurusu (A: `{"shows":{"study":"bollinger"},"output":"upper"}`, B: `"@line:bollinger.upper"`)
**o tek katmana** çözülüyor ve `set_encoding color` üç bandı birden boyuyor. Otorite bunu
başarı olarak onaylıyor (`ack applied`).

- A/ajan B (`notes.txt`): *"that single line layer draws upper/middle/lower, so all three
  bands became red, not only the upper; spec gives no way to color one output of a
  multi-output line layer except splitting layers."* — sorunu gördü ama durdu (son ✗).
- B/ajan B: *"set_encoding color on "@line:bollinger.upper" -> applied, but the layer draws
  upper/middle/lower so ALL three lines turned red (wrong)."* → ikinci denemede üstüne kırmızı
  bir `upper` katmanı ekledi (geçti; orijinal üst çizgi altta kalıyor).
- base/ajan B keşiften sonra katmanı böldü (`y: upper` + iki yeni katman) — kimlikle
  çalışırken bölme zorunluluğunu baştan gördü.

Bu, T8a'nın "bilinen zayıflık 3"ü ve görev notunun beklediği zorluk: başvuru doğru katmanı
buluyor, ama katman istenen birimden büyük ve otorite bunu sessizce kabul ediyor.

### B'ye özgü

- **Adlara güvenmeyip kimlik keşfi (2/5 ajan).** B/ajan A önce `probe.jsonl` ile kimlikleri
  öğrenip dört görevi kimlikle yazdı (*"I probed state with a list query in a probe.jsonl
  first (needed real ids, which are random per doc)"*); B/ajan E R05'te `attempt1`'i keşfe
  harcadı (*"probe only … to learn ids, request not attempted"*), sonra R10/R15/R20'yi
  kimlikle yazdı. B'de ilk denemelerin yalnız 11/20'si ad kullandı; A'da 19/20 seçici
  kullandı (20. görev R13 başvuru gerektirmiyor). B şartnamesinde adlar "kimlik yerine
  yazılabilir" diye sunuluyor ama ajanlar dizge biçimli bir kimlik alanına dizge biçimli bir
  ad yazmanın güvenli olduğuna ikna olmadı; nesne biçimli seçici, alanın tipinden ayrıştığı
  için daha "görünür" bir seçenek.
- **R17 `no_origin`** (B/ajan B): önce kökensiz 123,45 kuralı denedi; hata ileti ve ipucu
  yeterliydi, ikinci denemede `add_fact` + köken. Tasarımla ilgisiz (A ve base ajanları §10'u
  ilk okumada uyguladı).

### Değişkenlerden bağımsız (otorite/şartname)

- **`screen` panelleri yığın sırasıyla değil, kimlik sırasıyla veriyor** (PROTOCOL §5.4
  "yığın sırasıyla" diyor). Doğrulandı: `move_panel` sonrası doküman `[0,2,1]`, `screen`
  `[0,1,2]` (üç ikilide aynı). Ajanlar doğrulama için `screen`'e baktı ve fazladan deneme
  harcadı: base R01 (3 deneme), B R08 ve B R18 (2 deneme; ilk deneme zaten doğruydu). Hata
  `r2/int`'te; düzeltilmeli (Öneriler).
- **`insert_panel.panel` tipi:** PROTOCOL §5.1 tablosu `panel` diyor ("panel id'si" gibi
  okunuyor), şema `Panel` nesnesi istiyor. base'de iki ajan `panel: 3` yazıp `schema` hatası
  aldı (R01, R20); B/ajan C de not etti.
- **Bulgusuz `present`** en yüksek güvenli bulguyu seçiyor; base/ajan D R04 ve R19'da bulgu
  vermeden sordu ve RSI bulgusunu aldı (ikinci denemede düzeldi). A ve B'de "son kırılım"
  başvurusu bunu doğrudan çözdü.
- Headless `screen` boş döner (ekranı yok); A/B ajanları doğrulama için bunu denedi, sonra
  `list views` kullandı.

## 5. Öneri: A (yapılandırılmış seçiciler)

**Kanıt:**

1. **İlk deneme 19/20 (kimliksiz 19/20)**, B 17/20 (kimliksiz 10/20), base 7/20 (kimliksiz
   2/20). Ölçütün asıl sorusu "ajan dokümanı görmeden doğru mesajı yazabiliyor mu"ysa A,
   B'nin neredeyse iki katı kimliksiz ilk deneme veriyor.
2. **Benimsenme:** A'da beş ajanın beşi seçicileri ilk denemede kullandı; B'de beş ajanın
   ikisi adları bırakıp kimlik keşfine döndü.
3. **Hata yanıtı 0** (B 1, base 3), en az deneme (20; B 25, base 34), en az belirteç
   (2,51 M; B 2,64 M, base 3,19 M).
4. **Sonunda** A 19/20, B 20/20: tek fark R02, ve R02'nin kök nedeni iki tasarımda aynı
   (aşağıdaki düzeltme ikisine de uygulanabilir). B'nin R02'deki toparlanması tasarımdan değil
   o ajanın ikinci deneme yapmasından.

**Sınırlar:** n = 20, görev başına tek ajan, tek koşu; 19 ile 17 arasındaki fark tek başına
anlamlı değil. Asıl ayrım kimliksiz ilk deneme (19 vs 10) ve benimsenme. A ve B
şartnamelerinin örnekleri görev listesi bilinerek yazılmış ("son kırılım",
`{"kind":"breakout","latest":true}` / `@finding:latest(breakout)`); bu ikisine eşit avantaj,
base'e değil.

## 6. Önerilen tasarım düzeltmesi (A'nın en sık başarısızlığı: R02)

**Sorun:** `output` anahtarlı bir katman seçicisi, o çıktıyı **başka çıktılarla birlikte**
çizen katmana da uyuyor; stil işlemi (`set_encoding` ile `color`/`width`/`opacity`) o
katmanın bütün çıktılarına uygulanıyor ve otorite `applied` diyor. Ajan "üst bandı" boyadığını
sanıyor, üç bant boyanıyor.

**Düzeltme (otorite + şartname, A'da):**

1. **`output` tam eşleşme:** katman seçicisinde `output: X`, yalnız **X'i tek başına çizen**
   katmana uyar (`y` = X ve `y2` yok; ya da katmanın varsayılan çizdiği tek alan X). Çok
   çıktılı katman uymaz.
2. **Yol gösteren hata:** böyle bir katman varsa sonuç `not_found` değil `invalid` olur:
   `"layer `l1` draws upper, middle, lower; `output: upper` cannot select one output of a
   multi-output layer"`, `candidates` o katman, `hint` bölme yolunu verir.
3. **Yeni işlem `split_layer {view, layer}`:** çok çıktılı bir katmanı, aynı kodlamayla, çizim
   çıktısı başına bir katmana böler (aynı yerde, sırayla; `ack.created` yeni katman kimlikleri;
   değişim `delete_layer` + `insert_layer`'lar, geri alınabilir). `hint` iki işlemlik dizilimi
   hazır verir:
   `[{"op":"split_layer","view":{},"layer":{"shows":{"study":"bollinger"}}},
     {"op":"set_encoding","view":{},"layer":{"shows":{"study":"bollinger"},"output":"upper"},"encoding":{"color":"down"}}]`
   (A'nın komut içi sıralı çözümü sayesinde ikinci işlemin seçicisi bölünmüş taslağa karşı
   çözülür ve tek katmana uyar).

Böylece R02 tek komutta yazılabilir; yanlış (her şeyi boyayan) yazım sessizce uygulanmak
yerine düzeltme yolunu söyleyen bir hata verir. Aynı kural B'nin `@line:bollinger.upper`
adına da uygulanabilir. Yeniden ölçüm: A değişkeni `v2` olarak (aynı düzenek, yeni tohumlar).

## 7. Öneriler (kapsam dışı)

- `screen` panel sırasını düzelt (yığın sırası; §5.4) — üç değişkende fazladan deneme
  maliyeti ölçüldü.
- PROTOCOL §5.1'de `insert_panel.panel`'in `Panel` nesnesi olduğunu tabloda yaz.
- Düzenek: ajan başına ayrı `HOME` ve varsayılan-red yazma profili (şu an yazma yalnız depo
  ve oturum ağaçlarında reddediliyor); Claude Code'un `cd <iş klasörü> && …` ve
  `echo … > <iş klasörü>/…` biçimlerini izinli saydığı not edilmeli (dökümde görüldü,
  sınır içinde kaldı).
- Puanlama: ajanlar dört görevin keşfini tek koşuda paylaşıyor; ileride "görev başına ayrı
  süreç" ilk deneme ölçüsünü daha temiz yapar (maliyet ×4).
