# R3/E — Ajan yazabilirlik ölçümü: kapsül görevleri (C01–C20)

Dal `r3/e-ergo` (taban `r3/int` d7e87db). Görevler: `rounds/R3/tasks/TASKS.md` C01–C20.
Ölçüt: `GOAL.md` 16 (ikinci yarı). Yöntem R2/T9'unki (`rounds/R2/ergonomics/RESULTS.md`)
aynen: taze `claude -p` ajanları, model `sonnet` (`claude-sonnet-5-5`), en çok 5 eşzamanlı,
paket = şartname + gramer + şema + `run.sh`, dosya erişimi işletim sistemiyle sınırlı,
kimlikler/kurulum ajan başına rastgele, doğruluk gizli durumdan puanlanır, ham kayıtlar
saklanır. Ölçüm sırasında protokol ve şartname **değiştirilmedi**; düzeltmeler §6'da öneri.

## 1. Kurulum

### 1.1 Gizli kurulum (`harness/setup.py`, `harness/gen/`)

| Öğe | Değer |
|-----|-------|
| Veri | `qp_source::synthetic_market(2, 340, 2)` (veri tohumu **2**); 1. enstrüman `THYAO`, 2. `ASELS`; besleme `bist`, `1d` |
| Pencere | barlar `[off, 320)`; `off` ajanın tohumundan `0..5` → **315–320 bar** (görev metni "~300") |
| Seriler | iki bağlı seri (`source.kind: bind`), kimlikleri ajan başına rastgele (5 karakter) |
| Kapsül motoru | `init.capsules = {series: [THYAO, ASELS], flow: [{THYAO, seed 1}, {ASELS, seed 2}]}` (sentetik lisanslı akış, yetki `flow/*`) |
| Kesin kaynak | `init.data.settled` = aynı barlar (itiraz aynı adımda karara bağlanır; düzeltme yok → `rejected`) |
| Kişiler | `host` (host), `arkin` (owner, masaüstünde bağlı), `editor` (editor; ajan, `headless`), `viewer` (viewer; yalnız `bist/THYAO`, `bist/ASELS`, **`flow/*` yok**) |
| Saat | son bardan bir gün sonra (bütün barlar kapalı ve kesin) |

**Neden bu tohum ve pencere:** önce `END` (pencere sonu) 312–340 ve veri tohumu 1–6 tarandı
(`off` 0 ve 12 ile). Şartlar: THYAO'da tek açık kırılım, tek açık aşırılık; ASELS'te
geçersizleşmiş kapsül; `expired` kapsül; THYAO'da akış (lisanslı) kapsülü; THYAO'da sıkışma
kapsülü. Tohum 2, `END` 315–324 ilk dördünü her `off`'ta sağladı; sıkışma `off` ≥ 6'da
kayboluyor, bu yüzden `off` 0–5. Pencere başı kaydırıldığı için her motor kimliği
(`cap.THYAO.breakout.down.<doğum barı>`) ajandan ajana değişir; son barlar aynı olduğundan
açık hikâyeler (anlamca) aynı kalır. Kurulumdaki hikâyeler (`off` 0): 186 kapsül — açık 3
(THYAO kırılım `strengthened`, aşırılık `born`, devam `born`), `realized` 92, `invalidated`
74, `expired` 17; bunların 52'si akış (lisanslı); sıkışma 1 (`invalidated`).

Protokol motor kimliklerini (`cap.<SEMBOL>.<tür>.<yön>.<bar>`, `fnd.…`, `fx.…`) kendisi
atar; bunlar yeniden adlandırılamaz. "Ajan başına rastgele" bu yüzden iki yoldan:
seri kimlikleri rastgele, motor kimlikleri pencere kaydırmasıyla değişken. Ajan başına tohum
`sha256("r3-e/<koşu>/<ajan>")`; tohum, `off`, seri kimlikleri `raw/<koşu>/agent<X>/setup.json`'da;
puanlayıcının gerçeği (`capsules` ve `ledger by_kind`, ajan satırlarından önce host'un
sorgusu) `truth.json.gz`'de.

| Koşu/ajan | A | B | C | D | E |
|---|---|---|---|---|---|
| r1 `off` / bar | 3 / 317 | 4 / 316 | 1 / 319 | 1 / 319 | 1 / 319 |
| r2 `off` / bar | 3 / 317 | 1 / 319 | 0 / 320 | 5 / 315 | 3 / 317 |

### 1.2 Paket ve düzenek (`harness/e3.py`)

Paket: `docs/PROTOCOL.md`, `docs/GRAMMAR.md` (bu dalın), `qp schema export` (28 şema),
`run.sh`. `run.sh` R2'deki istemcinin aynısı, bir ekle: `--png <çıktı.png> (--view <görünüm> |
--present <bulgu> [--surface <yüzey>])` verilirse daemon ajanın satırlarından sonra
`qp render` ile (başvuru ekransız çizici) o görünümü ya da sunucunun `present` görünümünü
ajanın klasörüne çizer. C20'nin "ekransız çizim"i için ajanın elindeki tek yol budur
(protokolde çizim mesajı yok); bayraklar `run.sh`'ın başlığında ve yönergede yazılı.

Daemon her koşuda: gizli kurulum (host + arkin bağlanır, host başlangıç durumunu sorar) +
ajanın satırları + gizli son ek (host: `document`, `log`; `editor` olarak taze bağlantı:
`document` — kişinin kitlesi yalnız kendisine görünür). Ajana yalnız kendi bağlantılarına
giden mesajlar döner. Host çerçeveleri (`{"host":…}`) ve `zz-` bağlantıları reddedilir.

Yönerge: R2'nin metni; değişiklik yalnız `run.sh` satırındaki `--png` açıklaması ve 3.
adımda "soru **ya da göster/listele/aç** isteği varsa `answer.txt`". Birebir metin her
ajanın `prompt.txt`'si. Rol satırı: `editor`, `headless`, "ben/benim = `editor`"; C11'de
`viewer` ("akış veri yetkisi olmayan kullanıcı").

### 1.3 İstek metinleri

TASKS.md'deki metin; ajanın başka yoldan bilemeyeceği gönderge (186 hikâye arasında "bu
kapsül", "kırılım kapsülü") R2'deki gibi isteğe yazıldı. Ajan bölümü R2'ninki: A: C01 C06
C11 C16; B: C02 C07 C12 C17; C: C03 C08 C13 C18; D: C04 C09 C14 C19; E: C05 C10 C15 C20.

| # | TASKS.md | Ajana verilen (`e3.py` `REQUESTS`) |
|---|----------|-------------------------------------|
| C01 | Bugün THYAO'da ne var? | aynı |
| C02 | THYAO'daki kırılım kapsülünün kanıtını aç. | THYAO'daki **açık** kırılım kapsülünün kanıtını aç. |
| C03 | ASELS'teki geçersiz olan kapsül neden geçersiz oldu? | ASELS'te **en son** geçersiz olan kapsül neden geçersiz oldu? |
| C04 | Bu kırılım kapsülünü reddet. | **THYAO'daki açık** kırılım kapsülünü reddet. |
| C05 | En önemli üç kapsülü sırala. | aynı |
| C06 | Kırılım türünün karnesi ne? | aynı |
| C07 | Hassasiyetimi "yalnız önemliler" yap. | aynı |
| C08 | ASELS'i izleme listeme ekle. | aynı |
| C09 | THYAO'daki aşırılık kapsülünü kapsül derinliğinde göster. | THYAO'daki **açık** aşırılık kapsülünü … |
| C10 | Aynı kapsülü grafik derinliğinde göster; kırılan seviye çiziliyor mu? | **THYAO'daki açık kırılım kapsülünü** grafik derinliğinde göster; kırılan seviye çiziliyor mu? |
| C11 | Akış kapsülünün kanıtını aç (yetkisiz kullanıcı olarak). | THYAO'daki **en son** akış kapsülünün kanıtını aç. (rol: `viewer`) |
| C12 | THYAO kapsüllerine abone ol; yeni geçişleri bildir. | aynı |
| C13 | Bu kapsülün geçmişini (bütün geçişlerini) listele. | **THYAO'da en son gerçekleşen kırılım kapsülünün** geçmişini (bütün geçişlerini) listele. |
| C14 | Bu olgunun kanıtını bugünkü veriyle doğrula. | **THYAO'daki açık kırılım kapsülünün olgusunun** kanıtını bugünkü veriyle doğrula. |
| C15 | Yansıdaki veri bu kesin iddiayı tutmuyor; itiraz et. | Yansıdaki veri, **THYAO'daki açık kırılım kapsülünün** kesin iddiasını tutmuyor; itiraz et. |
| C16 | Sıkışma kapsülü hâlâ geçerli mi? | **THYAO'daki** sıkışma kapsülü hâlâ geçerli mi? |
| C17 | Son on barda doğan kapsülleri, türüne göre say. | aynı |
| C18 | THYAO'da pozisyonum var (100 adet); bunu kaydet. | aynı |
| C19 | Gerçekleşen kapsülleri listele. | aynı |
| C20 | Bu kapsülü işaretli grafik olarak PNG'ye çiz. | **THYAO'daki açık** kırılım kapsülünü işaretli grafik olarak PNG'ye çiz. |

**Kendi hatam (C13):** "en son gerçekleşen kırılım" Türkçede iki türlü okunur: aşaması
`realized` olan en son kırılım (kastedilen) ya da "en son meydana gelen kırılım". İkinci
okumayı yapan ajan yanlış sayıldı. İki koşuda da metin aynı bırakıldı (kararlılık
karşılaştırması bozulmasın); puan tablosunda katı sayı esas, "hoşgörülü" sayı ayrıca
verildi (§3).

### 1.4 Puanlama (`harness/score.py`)

Ölçütler TASKS.md "Doğru sayılan" sütununun yazılı hâli (`check()`); gerçek, her koşunun
gizli kurulum sorgusundan (koşudaki özetin `truth.json.gz` ile aynı olduğu da denetlenir).
"Açık" = aşama `born`/`confirmed`/`strengthened`/`weakened`. Özet:

| # | Geçer sayılan |
|---|---------------|
| C01 | `capsules` cevabı THYAO'nun bütün açık kapsüllerini içeriyor; cevap her birinin türünü adlandırıyor |
| C02 | `capsule` sorgusu açık THYAO kırılımında, kanıt değerleri döndü |
| C03 | `capsule` sorgusu ASELS'in en son (son geçiş barı) geçersizleşen kapsülünde; cevapta geçersizlik ve neden (`barrier.stop` ya da karşılığı) |
| C04 | günlükte (uygulanmış ya da öneri) o bulguda `reject_finding`; başka bulguda yok |
| C05 | `capsules` sorgusu; cevap ilk üçü sırayla adlandırıyor (bütün hikâyeler **ya da** açıklar) |
| C06 | `ledger` sorgusu; cevapta tür satırının `n` ve `realized`'ı |
| C07 / C08 / C18 | son durumda `editor`'ün kitlesi: `sensitivity: important` / `watchlist ∋ ASELS` / `positions.THYAO = 100` |
| C09 | `present` cevabı: açık THYAO aşırılığının bulgusu, derinlik `capsule` |
| C10 | `present` cevabı: açık THYAO kırılımı, derinlik `chart`; cevap "seviye çiziliyor mu"yu görünümle tutarlı söylüyor (olgu kökenli `rule` katmanı) |
| C11 | yalnız `viewer` bağlantısı; bir `capsule`/`capsules`/`verify_evidence` sorgusu; cevap gereken yetkiyi (`flow/…`) adlandırıyor |
| C12 | `subscribe` `{kind: capsules, instrument: THYAO}` → `subscribed` |
| C13 | `capsule` sorgusu en son `realized` THYAO kırılımında; cevap bütün geçişleri (aşama adları, bar ya da satır başına) listeliyor |
| C14 | `verify_evidence` (`evidence_check`) o kapsülün olgusunda ya da bulgusunda |
| C15 | `dispute` `claim.kind: capsule` o kapsülde, karar `rejected`/`upheld`/`pending` |
| C16 | sıkışma kapsülünü döndüren bir sorgu; cevap "geçerli değil" (`invalidated`) |
| C17 | `capsules` sorgusu; cevapta doğum barı son on barda olanların tür başına sayıları |
| C19 | `capsules` `stages: ["realized"]` |
| C20 | `run.sh --png` başarılı, çizilen görünüm o kapsülün bulgusunu/olgularını gösteriyor |

Tanımlar R2'deki gibi: **ilk deneme** = `attempt1` geçiyor; **sonunda** = son koşulan
deneme geçiyor (≤ 3); cevap isteyen görevde `answer.txt` tektir ve son koşudan sonra
yazılır, ilk deneme de o cevapla puanlanır (R2 kuralı). **Kimliksiz** = denemenin
dosyasında (ve `--present/--view` bağımsız değişkeninde) dokümanın hiçbir kimliği yok
(`cap.…`, `fnd.…`, `fx.…`, `prop-…`, rastgele seri kimlikleri).

**Öz-denetim** (`harness/selftest.py`, `raw/SELFTEST.md`): gerçek daemon ve `run.sh`
üzerinden keşif koşusundan kimlik alan başvuru çözümleri **20/20**, yalnız `hello` içeren
boş denemeler **0/20**. `score.py r1 r2` iki kez koşuldu: `score.json` md5'leri aynı
(`89b17367…`, `daf4993c…`).

## 2. Yalıtım

R2'nin sertleştirilmiş profili aynen (`sandbox-exec`: depo, oturum ve proje klasörlerine
okuma/yazma reddi; yalnız paket ve ajanın klasörü açık; ajan başına belirteç).
`raw/isotest/agentA/` (Claude Code içinden) ve `os-check.txt` (işletim sistemi düzeyi):
depo `CLAUDE.md`, `setup.py`, `TASKS.md`, gizli `init.json` ve `qp` ikilisi okunamıyor
(`EPERM` / `Operation not permitted`); depoya, başka ajanın klasörüne, gizli kurulum
klasörüne ve pakete yazılamıyor; başka ajanın dosyasıyla `run.sh` → `error: this file is
not in your own work dir` (daemon `rejected.log`'da yalnız bu bilinçli yoklama var).

Denetim (`harness/audit.py`, `raw/audit.txt`): 10 ajan, 340 araç çağrısı; 43'ü reddedildi
(çoğu `ls schema/`, `grep PROTOCOL.md`, heredoc). Katı kural 67 yürütülmüş çağrıyı
"sınır dışı" sayıyor; hepsi elle ve betikle incelendi: (a) `run.sh <kendi dosyası>`
çıktısının `cut`/`grep`/`sed`/`tail` ile kırpılması, `cd <kendi klasörü> && run.sh …`,
kendi klasöründe döngü/heredoc; (b) Claude Code'un **bu ajanın kendi** büyük araç
çıktılarını yazdığı `~/.claude/projects/-private-tmp-…-work-<koşu>-agent<X>/…/tool-results/*.txt`
dosyalarının okunması (ajanın kendi `run.sh` çıktısı; profil bu klasörü engellemiyor).
Dökümlerdeki her yol taranan betikle denetlendi: kendi klasörü, paket ve kendi oturum
klasörü dışında yol **0**.

## 3. Sonuçlar

Tam tablo `raw/SCORE.md`, ham kayıtlar `raw/r1/`, `raw/r2/`.

### 3.1 Toplamlar

| Koşu | İlk denemede | Kimliksiz ilk deneme | Sonunda | Kimliksiz sonunda | İlk denemede hata yanıtı | Deneme | Mesaj (ilk / hepsi) | Belirteç | Maliyet |
|---|---|---|---|---|---|---|---|---|---|
| r1 | **12/20** | 10/20 | **19/20** (hoşgörülü 20) | 10/20 | 5 | 33 | 46 / 81 | 7 167 389 | 3,86 $ |
| r2 | **11/20** | 9/20 | **20/20** | 11/20 | 4 | 36 | 53 / 100 | 8 754 979 | 4,36 $ |

İki koşu birbirine çok yakın: 19 görevde ilk deneme sonucu aynı; tek fark C08 (r2'de ajan
ilk denemeyi geçerli grup adlarını öğrenmek için bilerek hatalı yazdı). Sonunda: r1 C13
(yukarıdaki okuma farkı) dışında hepsi.

### 3.2 Görev matrisi (ilk / son, deneme sayısı, son denemede kimlik)

| Görev | r1 | r2 | Kimliksiz yazılabilir mi? |
|---|---|---|---|
| C01 bugün THYAO | ✓ / ✓ (2) kimliksiz | ✓ / ✓ (2) kimliksiz | evet (`capsules {instrument, open}`) |
| C02 kırılımın kanıtı | ✗ / ✓ (2) kimlik | ✗ / ✓ (2) kimlik | **hayır**: `capsule` sorgusu yalnız kimlik alır |
| C03 neden geçersiz | ✗ / ✓ (3) kimlik | ✗ / ✓ (2) kimlik | **hayır** (neden yalnız `capsule` geçmişinde) |
| C04 reddet | ✗ / ✓ (2) kimlik | ✗ / ✓ (2) kimlik | kırılgan: bulgu seçicisi `{"kind":"breakout.range.down","on":…,"state":"live","latest":true}` |
| C05 en önemli üç | ✓ / ✓ (1) kimliksiz | ✓ / ✓ (1) kimliksiz | evet |
| C06 karne | ✓ / ✓ (1) kimliksiz | ✓ / ✓ (1) kimliksiz | evet |
| C07 hassasiyet | ✓ / ✓ (1) kimliksiz | ✓ / ✓ (1) kimliksiz | evet |
| C08 izleme listesi | ✓ / ✓ (2) kimliksiz | ✗ / ✓ (2) kimliksiz | evet |
| C09 kapsül derinliği | ✗ / ✓ (2) kimlik | ✗ / ✓ (2) kimlik | kırılgan (C04 gibi) |
| C10 grafik derinliği | ✗ / ✓ (2) kimlik | ✗ / ✓ (2) kimlik | kırılgan (C04 gibi) |
| C11 yetkisiz kanıt | ✓ / ✓ (2) kimlik | ✓ / ✓ (3) kimliksiz | evet (`capsules` → mühür yorumları) |
| C12 abonelik | ✓ / ✓ (1) kimliksiz | ✓ / ✓ (1) kimliksiz | evet |
| C13 geçmiş | ✗ / ✗ (2) | ✗ / ✓ (3) kimlik | **hayır** (geçmiş yalnız `capsule`'da) |
| C14 kanıtı doğrula | ✗ / ✓ (2) kimlik | ✗ / ✓ (2) kimlik | **hayır**: `verify_evidence.item` seçiciyi reddediyor (§4) |
| C15 itiraz | ✓ / ✓ (1) kimlik | ✓ / ✓ (2) kimlik | **hayır** (`dispute.claim` kimlik) |
| C16 sıkışma geçerli mi | ✓ / ✓ (1) kimliksiz | ✓ / ✓ (2) kimliksiz | evet (`capsules {meaning: squeeze}`) |
| C17 son on barda doğan | ✓ / ✓ (2) kimliksiz | ✓ / ✓ (2) kimliksiz | evet (istemci tarafında `born_bar` süzgeci) |
| C18 pozisyon | ✗ / ✓ (2) kimliksiz | ✗ / ✓ (2) kimliksiz | evet |
| C19 gerçekleşenler | ✓ / ✓ (1) kimliksiz | ✓ / ✓ (1) kimliksiz | evet |
| C20 PNG | ✓ / ✓ (1) kimlik | ✓ / ✓ (1) kimlik | **hayır** (`--present` bulgu kimliği; `show_finding` + `--view` yolu bağlı seride hata, §4) |

C15 ve C20'nin ilk denemeleri geçti ama kimlikle: ajan E kimliği aynı koşudaki C10'un keşif
koşusundan aldı. Kimliksiz ilk denemelerin hepsi (10 ve 9) **"kimliksiz yazılabilir"
satırlarından**; kimlik isteyen ya da seçicisi kırılgan 9 görevin hiçbiri ilk denemede
kimliksiz geçmedi.

### 3.3 R2 ile karşılaştırma

| | Görevler | İlk denemede | Kimliksiz ilk | Sonunda | Maliyet (5 ajan) |
|---|---|---|---|---|---|
| R2 base (yalnız kimlik) | R01–R20 gramer | 7/20 | 2/20 | 20/20 | 2,06 $ |
| R2 A v1 (seçiciler) | R01–R20 | 19/20 | 19/20 | 19/20 | 1,89 $ |
| R2 A v2 (seçiciler, düzeltme) | R01–R20 | 19/20 | 16/20 | 20/20 | 1,76 $ |
| **R3 r1** | C01–C20 kapsül | **12/20** | 10/20 | **19/20** | 3,86 $ |
| **R3 r2** | C01–C20 kapsül | **11/20** | 9/20 | **20/20** | 4,36 $ |

Görev kümeleri farklı. Örüntü açık: kapsül görevleri, kimliksiz yazılabildiği yerde R2 A
gibi davranıyor (§3.2'de "evet" olan 11 görev: 22 ilk denemenin 19'u), kimlik istediği ya da
seçicinin motor bulgularına uymadığı yerde R2 `base` gibi (9 görev: 18 ilk denemenin 4'ü,
o 4'ü de başka görevin keşif koşusundan alınan kimlikle — C15, C20). Sonunda
başarı yüksek (39/40, hoşgörülü 40/40): spesifikasyon yeterli, ama ilk mesaj için değil.
Maliyet R2'nin ~2 katı: kapsül cevapları büyük (`welcome.summary` bütün olguları listeliyor,
~32 KB; tam `capsules` ~200 KB; `document` ~670 KB) — ajanlar bunu sık yazdı ("output
huge") ve `cut`/`grep` ile kırptı.

## 4. Başarısızlık çözümlemesi

İlk denemedeki 17 kayıp (r1 8, r2 9) üç nedene ayrılıyor:

### 4.1 Kapsül kimliği keşfi zorunlu (8 kayıp: C02×2, C03×2, C13×2, C09 r2, C10 r1)

`capsule` sorgusunun alanı **yalnız kimlik** (`Query::Capsule { capsule: String }`; §17.2
tablosunda `capsule` sorgusu yok). Ayrıntı (kanıt, geçmiş, geçersizlik nedeni) yalnız
oradan geliyor; bir koşu tek atış olduğundan ajan önce `capsules` ile kimliği öğreniyor.
Ajanlar bunu açıkça yazdı:

- r2 B C02: *"capsule sorgusu secici almiyor, id once ayri sorguyla ogrenilmek zorunda (tek atista zincirlenemiyor); welcome.summary kapsul listelemiyor."*
- r1 A C11: *"capsule query with selector {"latest":true} -> schema error (capsule field takes a string id only, although PROTOCOL §17.2 suggests id/finding fields take selectors)."*

Ek zorluk: `capsules` yalnız önem sırasıyla dönüyor; "en son" için istemci `last_bar`
taramak zorunda; `since_bar` doğumu değil **geçişi** süzüyor (şartnamede yazıyor ama
ajanlar tahmin etti). r1 C03'te ajan kırpılmış listeden yanlış kapsülü seçti (3 deneme);
r2 C13'te önem sırasındaki ilkini seçti.

### 4.2 Bulgu seçicisi motor bulgularına uymuyor (6 kayıp: C04×2, C14×2, C09 r1, C10 r2)

Dört ajanın ilk denemesi aynı seçiciyi yazdı: `{"kind":"breakout","state":"live"}`.

- `FindingSel.kind` olgunun **tam** türü (`breakout.range.down`), kapsül türü
  (`MeaningKind`: `breakout`) değil. §17.1 örneği `{"kind":"breakout","latest":true}`
  "son kırılım" R2 dokümanına göre yazılmış; motor olgularında `not_found` (öneri
  `did_you_mean: ["breakout.range.up","breakout.range.down"]`). r2 D C04: *"spec example
  kind=breakout is misleading."*
- `state: live` `realized` hikâyeleri de kapsar (durum eşlemesi §20.1); "açık" için
  seçicide `stage`/`open` yok. r1 D C04: *"'state: live' also matches realized capsules,
  so it can't mean 'open'."*
- **`verify_evidence.item` seçici kabul etmiyor** (C14×2): JSON şeması `ItemRef` (kimlik ya
  da seçici) diyor, otorite `{"finding":{…}}` için `schema` hatası veriyor: `"invalid type:
  map, expected a string"`, `path: ""` (yol da boş). Doğrulandı (doğru tür adıyla da aynı).
  Şema ile otorite çelişiyor; §17.2 tablosu da `verify_evidence`'i saymıyor.

### 4.3 Kitle tipi şartnamede yok (3 kayıp: C18×2, C08 r2)

§20.3 `audience {group, sensitivity?, watchlist?, positions?}` diyor; `group` değerleri
(`habituated`/`future`) ve `positions` biçimi (sembol → adet nesnesi) yalnız şemada.
r1 C C18: `positions` dizi olarak yazıldı → `schema`. r2 C: ilk denemeyi grup adlarını
öğrenmek için bilerek `"zzz"` ile yazdı (C08, C18). Ajanların ortak notu: `set_audience`
kitlenin tamamını mı değiştiriyor, kendi kitlem nereden okunur (§20.3 "yalnız kendisi
görür" diyor ama yer söylemiyor; aslında `document.audiences`'ta).

### 4.4 Son durumda kalan (r1 C13)

Ajan "en son gerçekleşen kırılımı" en son doğan kırılım olarak okudu (açık, `strengthened`)
ve onun geçmişini doğru listeledi. İstek metnimin çift anlamlılığı (§1.3); şartname
kusuru değil. Hoşgörülü okumada geçer.

### 4.5 Geçen ama zorlanan görevler

- **C04 reddet:** `editor`'ün `decide` yetkisi yok; `reject_finding` sessizce **öneriye**
  dönüşüyor (`ack status: proposed`, `prop-4`). §4.7 "öneri olamayan işlemler" listesinde
  `reject_finding` yok, ama ajanlar bunun beklenen sonuç olup olmadığını bilemedi (r2 D:
  *"Unclear whether the task expects actual rejection; impossible for this role."*).
- **C11 yetkisiz kanıt:** lisanslı hikâye `viewer`'a hiç gitmiyor (§20.5.1), "en son akış
  kapsülü" yetkisiz için bilinemez; iki ajan da cevabı mühür yorumlarının
  `attachments.entitlement` (`flow/*`) alanından verdi. Bir ajan `editor` koşusundan
  kimlik tahmin edip `not_found` aldı (beklenen). r2: §5.4 tablosunda `comments` sorgu alanı
  gibi okundu → `schema`.
- **C15 itiraz:** iki ajan da `t`'nin ne olduğunu (doğum mu son geçiş mi), `values`'a ne
  yazılacağını bilemedi (*"dispute is described as a replica/host message, not clear an
  agent should send it"*); karar `rejected`, cevap doğru.
- **C09:** "kapsül derinliği" için `present.surface: "phone"` (profil tablosundan çıkarım);
  `present.surface` dizge, `hello.surface` nesne.
- **C05:** önem sırası kapanmış hikâyeleri de içeriyor; iki ajan da "en önemli üç" olarak
  aylar önce gerçekleşmiş/süresi dolmuş kapsülleri verdi ve bunu not etti.
- **Düzenekte bulunan, ajanların çarpmadığı:** `show_finding` bağlı (bind) seride
  `invalid: series … has no bars` (sunucu görünümü `presenter_view` veri düzlemi olmadan
  dokümanın satır içi barlarına bakıyor; `present` sorgusu ise çalışıyor). C20'nin
  "`show_finding` + `--view`" yolu bu yüzden kapalı; ajanlar `--present` kullandı.

## 5. Ölçütün değerlendirmesi (GOAL 16, ikinci yarı)

Yirmi kapsül görevi R2 düzeneğiyle ölçüldü: **sonunda 19/20 ve 20/20** (hoşgörülü 20/20
iki koşuda), **ilk denemede 12/20 ve 11/20**, kimliksiz ilk deneme 10/20 ve 9/20. İki
bağımsız koşu tutarlı. Ajanlar kapsül protokolünü şartnameden öğrenip her görevi bitirebiliyor;
ilk mesajda bitirememenin nedeni ölçülebilir ve dar: kapsül ayrıntısına seçiciyle
başvurulamıyor (§4.1), bulgu seçicisi motor bulgularının sözlüğünü konuşmuyor (§4.2), kitle
tipi şartnamede yazılı değil (§4.3).

## 6. Öneriler (şartname/protokol; bu ölçümde yapılmadı)

1. **Kapsül seçicisi.** `CapsuleSel {instrument, meaning (MeaningKind), direction, stage,
   open, born_bar, first/latest}` — sıra: son geçiş barı, eşitlikte doğum barı. `capsule`
   sorgusunun `capsule` alanı, `dispute.claim`, ve bulgu alan her yer (`present.finding`,
   `reject_finding`, `verify_evidence.item`, `show_finding`) bunu kabul etsin
   (`{"capsule":{"instrument":"THYAO","meaning":"breakout","open":true}}`). Bu, §4.1 ve
   §4.2'deki 14 ilk deneme kaybının hedefi; C02, C03, C09, C10, C13, C14, C15, C20 kimliksiz
   yazılabilir olur.
2. **`FindingSel`:** `kind`'in olgu türü olduğunu ve motor türlerinin noktalı olduğunu
   (`breakout.range.down`) §17.1'e yaz; `meaning`, `instrument`, `stage`/`open` anahtarları
   ekle; §17.1 "son kırılım" örneğini motor dokümanında çalışan biçimle ver.
3. **`verify_evidence.item`:** otorite şemanın dediği gibi seçici kabul etsin (ya da şema
   düzeltilsin); hata yolu `""` değil `/query/item` olsun. §17.2 tablosuna ekle.
4. **§20.3 kitle:** `Audience` tipini tabloyla yaz (`group`: `habituated` | `future`,
   varsayılan hassasiyetleri; `watchlist`: sembol dizisi; `positions`: `{"THYAO": 100}`),
   `set_audience`'ın tamamen değiştirdiğini söyle; izleme listesine ekleme/çıkarma için
   birleştiren bir işlem (`update_audience` ya da `set_audience.merge`); kişinin kendi
   kitlesini nereden okuyacağını yaz (`document.audiences`, yalnız kendi girdisi).
5. **`capsules` sorgusu:** `order: "importance" | "recent"`, doğum süzgeci
   (`born_since_bar`), `meaning` değerlerinin listesi §20.2'de; `since_bar`'ın geçişi
   süzdüğünü tabloda belirt; tür başına sayım (`count_by: "meaning"`) C17'yi istemci
   taramasından kurtarır. `comments`'in cevap alanı olduğunu sorgu sütunundan ayır.
6. **§4.7 / §9:** `decide` yetkisi olmayanın `approve_finding`/`reject_finding`'inin öneriye
   dönüştüğünü ve `ack.status: proposed`'un bu durumda beklenen sonuç olduğunu yaz.
7. **`present`:** derinlik parametresi (`depth: "capsule"`) ya da §10'da "kapsül derinliği =
   `surface: \"phone\"`" örneği; `present.surface` dizgesinin `hello.surface` nesnesinden
   farkını not et.
8. **§19.9 / §20.7 itiraz:** kapsül iddiasında `t`'nin anlamı (hikâyenin herhangi bir geçiş
   barı mı), `values` boş gönderilebilir mi, ajanın da gönderebileceği; örnek bir kapsül
   itirazı.
9. **Cevap boyutu:** `welcome.summary.facts` yüzlerce olguyu listeliyor (~32 KB); bir üst
   sınır ya da yalnız sayılar. `capsules` için `fields`/`limit` önerisi örneklerde.
10. **Hata (kod):** `show_finding` bağlı seride `series … has no bars` — sunucu görünümü veri
    düzlemi üzerinden kurulmalı (`present` sorgusu gibi).
11. **Önem sırası:** "en önemli" sorusunda kapanmış hikâyelerin başta gelmesi okuyucuya
    yararsız; varsayılan sıraya açıklık katılsın ya da şartname bunu açıkça yazsın.
12. **Düzenek:** isteklerde kastedilen aşama adı açık yazılmalı (C13 "aşaması
    gerçekleşti olan"); ajan başına ayrı `HOME` (Claude Code'un kendi araç çıktısı dosyaları
    `~/.claude/projects/…` altında okunuyor; bu ölçümde yalnız ajanın kendi çıktısı).

## 7. Maliyet

| | Belirteç (girdi + önbellek + çıktı) | Çıktı belirteci | Ajan süresi (toplam) | Maliyet |
|---|---|---|---|---|
| r1 (5 ajan) | 7 167 389 | 57 692 | 573 s | 3,86 $ |
| r2 (5 ajan) | 8 754 979 | 64 306 | 613 s | 4,36 $ |
| Yalıtım testi | — | — | — | 0,05 $ |
| **Toplam** | **15 922 368** | 121 998 | 1 186 s | **8,27 $** |

Ajan başına 0,61–1,32 $; duvar saati koşu başına ~2,5 dakika (5 ajan eşzamanlı).

## 8. Düzen ve yeniden üretim

```
harness/gen/          sentetik barları yazan küçük ikili (qp_source::synthetic_market; çalışma alanı dışı)
harness/setup.py      gizli kurulum (tohumdan init, kurulum ve son ek satırları)
harness/e3.py         packet | serve | launch <ajan> --label <koşu> | isotest
harness/score.py      puanlama (yalnız ham kayıtlardan; yeniden koşunca aynı score.json)
harness/selftest.py   puanlayıcı öz-denetimi (başvuru 20/20, boş 0/20; daemon gerekli)
harness/audit.py      dökümlerden araç çağrısı denetimi
harness/oscheck.sh    işletim sistemi düzeyinde okuma/yazma yoklaması
raw/<koşu>/agent<X>/  setup.json, truth.json.gz, prompt.txt, TASKS.txt, sandbox.sb, command.json,
                      transcript.jsonl (+ .ts), process.json, runs.log, stderr.txt,
                      C??/runs/<n>-<dosya>.jsonl (+ .out.jsonl.gz: ajanın gördüğü,
                      + .hidden.json: gizli son ekin özeti, + .png.json/.transcript.json: çizim),
                      C??/{attemptN.jsonl, notes.txt, answer.txt, *.png.sha256}
raw/<koşu>/score.json, raw/SCORE.md, raw/SELFTEST.md, raw/audit.txt, raw/isotest/
```

Yeniden üretim: `cargo build --release -p qp-cli` ve `harness/gen` (hedef klasör
`$E_SCR/target`); `e3.py packet`; `e3.py serve &`; `e3.py launch <A–E> --label <koşu>`;
`score.py <koşu>`. Tam otorite çıktısı (koşu başına ~3 MB) depoya alınmadı; puanlama için
gerekenler `.hidden.json` ve `truth.json.gz`'de. Çizilen PNG'ler (sentetik veri) depoda
değil, yalnız boyut ve SHA-256.

## Sonradan düzeltmeler

Commit `544588d` (dal `r3/fix-e-bugs`):

- **`verify_evidence.item` seçici** (§4.2, öneri 3): şema `ItemRef` diyordu ama seçici
  yerleri listesinde (`select.rs`) `verify_evidence` yoktu; seçici çözülmeden tipli
  ayrıştırmaya gidip `"invalid type: map, expected a string"`, `path: ""` veriyordu. Artık
  öğe seçicisi (`{"finding":…}` / `{"fact":…}`) §17 kurallarıyla çözülüyor (`resolved`,
  `ambiguous`, `not_found`, `schema` hep `/query/item/…` yolunda). Aynı durumdaki
  `binding.series` de eklendi. §17.2 tablosu ve §18.5 güncellendi.
- **`show_finding` bağlı seride** (öneri 10): sunucu görünümü komutun taslak dokümanından
  kuruluyordu; bağlı serinin barları orada yok (yalnız okuma dokümanında), bu yüzden
  `series … has no bars`. Sağlayıcı artık bağlı barları okuma dokümanından okuyor
  (`add_fact`, `set_view_range` gibi); görünüm bar zamanlarını okuduğu için konuşanın
  veri yetkisi denetleniyor (`no_entitlement`, `/ops/i/finding`). Lisanslı olgu değeri
  görünüme sabit olarak yazılmıyor (S-C kuralı, `canon` değişmedi); yetki taraması testi
  artık saklanan görünümle gerçekten koşuyor (önceden `show_finding` viewer'dan
  gönderildiği için reddediliyordu).
- Yeni döküm `44-capsule-selectors-bound.json` (üç taşımada geçiyor), 01–43 bayt eşit;
  `capsules.rs` testi `show_finding_on_a_bound_series_and_verify_evidence_by_selector`.
