# R4/E — Ajan yazabilirlik yeniden ölçümü: kapsül görevleri (C01–C20)

Dal `r4/e-ergo` (taban `6b0c3f1`). Ölçüt: `rounds/R4/GOAL.md` 13 (ikinci yarı). Görevler ve istek
metinleri R3'ünkiler (`rounds/R3/tasks/TASKS.md`, `rounds/R3/ergonomics/RESULTS.md` §1.3).
Yöntem R3/E'ninki aynen: taze `claude -p` ajanları, model `sonnet` (`claude-sonnet-5-5`), en
çok 5 eşzamanlı, paket = bu dalın `docs/PROTOCOL.md` + `docs/GRAMMAR.md` + `qp schema export`
(28 şema) + `run.sh`, dosya erişimi işletim sistemiyle (`sandbox-exec`) sınırlı, ajan başına
rastgele kimlik/kurulum, doğruluk gizli durumdan puanlanır, ham kayıtlar `raw/`'da.
Ölçüm sırasında protokol ve şartname **değiştirilmedi**; düzeltmeler §7'de yalnız öneri.

## 1. Kurulum ve R3'ten farklar

### 1.1 Düzenek (`harness/`, R3'ten kopya)

`rounds/R3/ergonomics/harness/` olduğu gibi kopyalandı. Puanlayıcı (`score.py`), istek
metinleri, yönerge (`PROMPT`), `run.sh`, gizli kurulum (`setup.py`: veri tohumu 2, pencere
`[off, 320)`, `off` 0–5, iki bağlı seri, kapsül motoru, sentetik lisanslı akış, kesin kaynak,
dört kişi) ve ajan bölümü (A: C01 C06 C11 C16 … E: C05 C10 C15 C20) **değişmedi**.
Uyarlamalar (`git diff --no-index rounds/R3/ergonomics/harness rounds/R4/ergonomics/harness`):

| # | Ne | Neden |
|---|----|-------|
| 1 | `setup.py` `SCR`: bu oturumun scratchpad'i | yol |
| 2 | `e3.py` ajan tohumu `sha256("r4-e/<koşu>/<ajan>")` (R3: `r3-e/…`) | yeni rastgele çekiliş (sürüm); `off` ve seri kimlikleri ajan başına yeni |
| 3 | `setup.json`'daki dal adı `r4/e-ergo`; belge dizgelerinde R3 → R4 | sürüm |
| 4 | `isotest`/`oscheck.sh` yazma yoklaması `rounds/R4/ergonomics/PWNED.txt`, `harness/setup.py` R4 yolu | yol |
| 5 | Kum havuzu profili: `~/.claude/projects/-private-tmp-claude-501--Users-arkns-Works-Mchart-qapsule-*` okuma/yazma **reddi**, ajanın **kendi** Claude Code proje klasörü açık | yalıtım: R2/R3 ajanlarının oturum klasörleri (aynı görevlerin dökümleri) artık makinede var; R3 profili bunları engellemiyordu. R3'te ajanların okuduğu tek şey kendi araç çıktılarıydı, o hâlâ açık. |
| 6 | **R3'ten sapma: güvenlik sertleştirmesi** (orkestratörün otomatik güvenlik incelemesi üzerine). `run.sh` isteğinde deneme dosyası ve `--png` hedefi: (a) önce tam `realpath` + `O_NOFOLLOW` (commit `24fef49`), (b) sonra çalışma dizininden `O_DIRECTORY|O_NOFOLLOW` ile parça parça yürüyüş; okumada `fstat` normal dosya ve `st_nlink == 1`, aynı tanımlayıcıdan okuma; PNG `O_CREAT|O_EXCL|O_NOFOLLOW` geçici adla yazılıp aynı `dir_fd` ile `rename` (commit `e118f6f`; `harness/sectest.py`: bağlı hedef, bağlı ara dizin, katı bağ, dizin dışı yol reddediliyor, normal yol çiziyor — 8/8 PASS) | düzenek, protokol değil. **r1 R3 ile özdeş kodla, r2 (a) ile koştu; (b) iki koşudan sonra uygulandı.** Ajanların kullandığı yollarda (kendi klasöründe düz dosyalar) davranış üçünde de aynı; hiçbir ajan bağ kullanmadı, düzenek reddi yok (`rejected.log` boş). |
| 7 | Yeni betikler (R3'te yok, ölçüme dokunmaz): `selftest_sel.py` (seçicili başvuru çözümleri), `score_r4.py` (ikinci okuma, §3.3), `sectest.py` | puanlayıcının yeni yolları ezmediğini göstermek |

Değişmeyen puanlayıcının R4 kodu üzerinde doğruluğu (`raw/SELFTEST.md`, gerçek daemon ve
`run.sh` üzerinden): kimlikli başvuru çözümleri **20/20**, boş denemeler **0/20**, seçicili
(kimliksiz) başvuru çözümleri **20/20** (18'i kimliksiz; C15 `dispute.claim` ve C20
`--present` seçici almıyor). Günlük ve cevaplar yalnız kimlik taşıdığı için (§17.4) seçicili
`reject_finding`, `present`, `verify_evidence` R3 ölçütleriyle doğru puanlanıyor.
`score.py r1 r2` iki kez koşuldu: aynı çıktı.

### 1.2 Paket farkı (bu dalın şartnamesi)

R3 paketine göre şartnamede yeni olanlar: R4/T3 (§17.1 `CapsuleSel`, bulgu seçicisinde
`instrument`/`meaning`/`stage`/`open`, olgu türünün tam olduğu açıklaması; §20.2 `capsule`
sorgusu seçici alır, `capsules` `latest`/`born_since_bar`/`born_since_t`, `ledger` `direction`,
`validity`; §20.3 `Audience` tablosu ve `set_audience` tam değiştirme), R4/T4 (§4.6.1 piksel
hareketleri ve `input` `open`/`close` ile kapsül açılması), R4/T2 (§21 yeniden oynatma).
Şema sayısı aynı (28).

### 1.3 Koşulan kurulumlar

| Koşu/ajan | A | B | C | D | E |
|---|---|---|---|---|---|
| r1 `off` / bar | 0 / 320 | 4 / 316 | 2 / 318 | 2 / 318 | 4 / 316 |
| r2 `off` / bar | 3 / 317 | 3 / 317 | 4 / 316 | 4 / 316 | 1 / 319 |

R3'teki tarama şartları R4 koduyla (K42) altı `off`'un hepsinde yeniden denetlendi: THYAO'da
tek açık kırılım (`strengthened`), tek açık aşırılık, ASELS'te geçersizleşmiş kapsüller,
`expired` kapsüller, THYAO akış kapsülleri, THYAO sıkışması (`invalidated`) — hepsi tutuyor;
hikâye sayısı 186–192 (R3 `off` 0: 186, aynı).

## 2. Yalıtım

- `raw/isotest/agentA/` (Claude Code içinden): depo `CLAUDE.md`, `harness/setup.py`,
  `TASKS.md` → `EPERM`; depoya ve başka ajanın klasörüne yazma reddedildi; başka ajanın
  dosyasıyla `run.sh` → `error: this file is not in your own work dir`; `cat` reddedildi.
  Maliyet 0,05 $.
- `raw/isotest/agentA/os-check.txt` (işletim sistemi düzeyi): aynı sonuçlar + `qp` ikilisi,
  gizli `init.json` okunamıyor, paket ve gizli klasöre yazılamıyor; **yeni**: R3 r1 agentA'nın
  Claude Code proje klasörü `Operation not permitted`, ajanın kendi proje klasörü okunabiliyor.
- Denetim (`harness/audit.py`, `raw/audit.txt`): 10 ajan, 303 araç çağrısı, 45'i reddedildi.
  Katı kural 56 yürütülmüş çağrıyı "sınır dışı" sayıyor; R3'teki gibi hepsi `run.sh` çıktısının
  `cut`/`tail`/`sed` ile kırpılması, kendi klasöründe heredoc/döngü, `B=…; W=…` değişkenli
  komutlar ve ajanın kendi proje klasöründeki araç çıktısı dosyalarının okunması. Dökümlerdeki
  her mutlak yol betikle tarandı: kendi klasörü, paket ve kendi proje klasörü dışında gerçek yol
  **0** (iki eşleşme `B=<scratchpad>/e3` değişken tanımı).

## 3. Sonuçlar

Tam tablo `raw/SCORE.md`; ham kayıtlar `raw/r1/`, `raw/r2/`; ikinci okuma `raw/<koşu>/score_r4.json`.

### 3.1 Toplamlar (R3 puanlayıcısı, katı okuma)

| Koşu | İlk denemede | Kimliksiz ilk deneme | Sonunda | Kimliksiz sonunda | Sonunda hoşgörülü (C13) | İlk denemede hata yanıtı | Deneme | Mesaj (ilk / hepsi) | Belirteç | Maliyet |
|---|---|---|---|---|---|---|---|---|---|---|
| r1 | **14/20** | 12/20 | 15/20 | 13/20 | 16/20 | 0 | 29 | 45 / 67 | 7 087 901 | 3,56 $ |
| r2 | **17/20** | 15/20 | 16/20 | 13/20 | 17/20 | 1 | 26 | 46 / 62 | 9 216 913 | 5,06 $ |

### 3.2 Görev matrisi (ilk / son (deneme sayısı), son denemede kimlik)

| Görev | r1 | r2 | Not |
|---|---|---|---|
| C01 bugün THYAO | ✗ / ✗ (3) | ✓ / ✗ (3) | "bugün" = son bar okuması (§4.3) |
| C02 kırılımın kanıtı | ✓ / ✗ (2) kimlik | ✓ / ✗ (2) kimlik | ilk deneme seçicili `capsule` sorgusu; son deneme `input open` (§3.3) |
| C03 neden geçersiz | ✓ / ✓ (1) kimliksiz | ✓ / ✓ (1) kimliksiz | `capsule {instrument: ASELS, stage: invalidated, latest}` |
| C04 reddet | ✓ / ✓ (1) kimliksiz | ✓ / ✓ (1) kimliksiz | bulgu seçicisi `{instrument, meaning, open}` |
| C05 en önemli üç | ✓ / ✓ (2) kimliksiz | ✓ / ✓ (1) kimliksiz | |
| C06 karne | ✓ / ✓ (1) kimliksiz | ✓ / ✓ (1) kimliksiz | |
| C07 hassasiyet | ✓ / ✓ (1) kimliksiz | ✓ / ✓ (1) kimliksiz | |
| C08 izleme listesi | ✗ / ✓ (2) kimliksiz | ✓ / ✓ (1) kimliksiz | r1: ilk deneme yalnız keşif (`document`) |
| C09 kapsül derinliği | ✓ / ✓ (1) kimliksiz | ✓ / ✓ (1) kimliksiz | `present` + bulgu seçicisi + `surface: phone` |
| C10 grafik derinliği | ✗ / ✗ (2) kimlik | ✗ / ✓ (2) kimlik | ilk deneme keşif; r1 son deneme `input open` (§3.3) |
| C11 yetkisiz kanıt | ✗ / ✗ (2) kimlik | ✗ / ✗ (1) kimliksiz | yeni hata: seçici yetkisize görünen en son (açık veri) akış kapsülüne çözülüyor (§4.1) |
| C12 abonelik | ✓ / ✓ (1) kimliksiz | ✓ / ✓ (1) kimliksiz | |
| C13 geçmiş | ✗ / ✗ (1) | ✗ / ✗ (2) | ikisi de "en son doğan kırılım" okuması (R3'teki istek belirsizliği); hoşgörülü ✓ |
| C14 kanıtı doğrula | ✓ / ✓ (1) kimliksiz | ✓ / ✓ (1) kimliksiz | `verify_evidence.item: {"finding": {…}}` |
| C15 itiraz | ✓ / ✓ (1) kimlik | ✓ / ✓ (1) kimlik | kimlik C10'un keşif koşusundan (`dispute.claim` seçici almıyor) |
| C16 sıkışma geçerli mi | ✓ / ✓ (1) kimliksiz | ✓ / ✓ (1) kimliksiz | |
| C17 son on barda doğan | ✓ / ✓ (2) kimliksiz | ✓ / ✓ (2) kimliksiz | `born_since_bar`; sayım istemcide |
| C18 pozisyon | ✗ / ✓ (2) kimliksiz | ✓ / ✓ (1) kimliksiz | r1: ilk deneme yalnız keşif |
| C19 gerçekleşenler | ✓ / ✓ (1) kimliksiz | ✓ / ✓ (1) kimliksiz | |
| C20 PNG | ✓ / ✓ (1) kimlik | ✓ / ✓ (1) kimlik | `--present <bulgu kimliği>` (C10'dan) |

### 3.3 İkinci okuma: `input open` yolu (`harness/score_r4.py`)

R4/T4 kapsülü bir derinliğe açan `input {gesture: {kind: open, capsule, to}}` hareketini getirdi
(§4.6.1); cevabı `opening.presentation` (o derinlikte sunucu çıktısı) ve `evidence`
derinliğinde `capsule` sorgusunun kanıt erişimi. "Kanıtını aç", "grafik derinliğinde göster"
isteklerinde ajanlar bunu kullandı (r1 C02, C10, C11; r2 C02, C10). R3 puanlayıcısı bu yolu
bilmiyor. `score_r4.py`, R3 kontrolü düşen C02/C09/C10/C11'i eşdeğer `open` cevabıyla yeniden
dener (doğru kapsül, doğru `to`; C02'de değerli kanıt; C10'da cevap görünümle tutarlı; C11'de
cevap yine `flow/…` adlandırmalı). Katı okuma değişmez; bu yalnız ek sütun.

| Koşu | İlk denemede | Sonunda | Sonunda hoşgörülü (C13) |
|---|---|---|---|
| r1 | 14/20 | **17/20** (+C02, +C10) | 18/20 |
| r2 | 17/20 | **17/20** (+C02) | 18/20 |

İkinci okumada sonunda kalan kayıplar: C01 (r1: cevap yanlış; r2: son deneme açıkları
listelemeyen bir sorguydu, cevap doğru), C11 (iki koşu), C13 (iki koşu, hoşgörülü ✓).

### 3.4 R3'te kimlik gerektiren dokuz görev

R3'te ilk denemede kimlik gerektiren ya da seçicisi motor bulgularına uymayan 9 görev: C02, C03,
C04, C09, C10, C13, C14, C15, C20 (R3: 18 ilk denemenin **4**'ü geçti, o 4'ü de başka görevin
keşif koşusundan alınan kimlikle; kimliksiz 0).

| Görev | R3 r1 ilk | R3 r2 ilk | R4 r1 ilk | R4 r2 ilk | R4 ilk denemenin yazımı |
|---|---|---|---|---|---|
| C02 | ✗ | ✗ | ✓ kimliksiz | ✓ kimliksiz | `capsule {instrument, meaning: breakout, open}` |
| C03 | ✗ | ✗ | ✓ kimliksiz | ✓ kimliksiz | `capsule {instrument: ASELS, stage: invalidated, latest}` (r1: `{instrument, latest}` — en son hareket eden zaten geçersizdi) |
| C04 | ✗ | ✗ | ✓ kimliksiz | ✓ kimliksiz | `reject_finding.finding {instrument, meaning, open}` (öneriye dönüşüyor) |
| C09 | ✗ | ✗ | ✓ kimliksiz | ✓ kimliksiz | `present.finding {instrument, meaning: extreme, open}`, `surface: phone` |
| C10 | ✗ | ✗ | ✗ | ✗ | ikisi de ilk denemeyi keşfe harcadı (`input open` kimlik istiyor) |
| C13 | ✗ | ✗ | ✗ | ✗ | seçici doğru yazıldı, istek "en son doğan" okundu (hoşgörülü ✓ ikisi) |
| C14 | ✗ | ✗ | ✓ kimliksiz | ✓ kimliksiz | `verify_evidence.item {"finding": {instrument, meaning, open}}` |
| C15 | ✓ kimlik | ✓ kimlik | ✓ kimlik | ✓ kimlik | `dispute.claim.capsule` kimlik (C10 keşfinden) |
| C20 | ✓ kimlik | ✓ kimlik | ✓ kimlik | ✓ kimlik | `run.sh --present <bulgu kimliği>` (C10 keşfinden) |
| **Toplam** | 2/9 | 2/9 | **7/9** (5 kimliksiz) | **7/9** (5 kimliksiz) | |

R3 4/18 → R4 **14/18** (kimliksiz 0/18 → **10/18**). Seçici kabul eden her yerde (C02, C03, C04,
C09, C14) dört ilk denemenin dördü de kimliksiz geçti. Kalan: C10 (`input open` seçici almıyor,
`present` + bulgu seçicisi mümkündü ama ajanlar açılma hareketini tercih etti), C13 (istek
belirsizliği), C15/C20 (seçici yolu yok; kimlik başka görevden).

## 4. Hata sınıfları

İlk denemedeki 9 kayıp (r1 6, r2 3):

### 4.1 Seçici yetkisize görünen kümede sessizce çözülüyor (C11 ×2) — yeni

`viewer` için `{"kind":"capsule","capsule":{"instrument":"THYAO","meaning":"flow","latest":true}}`
lisanslı (daha yeni) akış hikâyelerini görmeden, okuyucuya görünen **açık veri** akış kapsülüne
(`flow.volume_surge`, hacim patlaması; r1 `cap.THYAO.flow.down.298`, r2 `…295`) çözülüyor.
Gerçekte en son akış kapsülü lisanslı (`cap.THYAO.flow.up.307` / `…down.301`). Ajanlar kanıtı
"açık lisanslı, değerler görünüyor" diye verdi, gereken yetkiyi (`flow/*`) hiç anmadı. R3'te
ajanlar `capsules` listesiyle başladığı için mühürlü sunucu yorumlarını (`comments`,
`attachments.entitlement`) görüp yetkiyi adlandırmıştı. Seçici bu ipucunu atlatıyor: `capsule`
cevabında `comments` yok. Bu, gizlilik kuralının doğru sonucu (§20.5.1). Ama okuyucuya
"bu seçiciye uyan, sizin göremediğiniz daha yeni hikâyeler var; `flow/*` gerekir" bilgisi
verilmiyor, oysa aynı bilgi `capsules` sorgusunda mühürlü yorum olarak veriliyor.

### 4.2 İlk deneme keşfe harcandı (r1 C10, r2 C10, r1 C08, r1 C18)

- C10: iki ajan da "grafik derinliğinde göster" için `input open {capsule, to: chart}` seçti.
  `open.capsule` **yalnız kimlik** alıyor (§4.6.1 "kapsül ya da bulgu kimliği"; §17.2 tablosunda
  `input` yok; r2 B C02 seçiciyi denedi → `schema`). Bu yüzden ilk deneme yalnız keşifti. Oysa
  `present` + bulgu seçicisi kimliksiz çalışıyordu (C09'da iki ajan da öyle yazdı). r2 B: *"Spec
  §4.6.1 says capsule (kapsül ya da bulgu kimliği) … the schema CapsuleRef exists, which misleads."*
- C08/C18 (r1 C): `set_audience` tam değiştirdiği için ajan önce mevcut kitleyi okudu
  (`document`), sonra yazdı. Bu temkinli bir okuma, şartname kusuru değil. §20.3'teki yeni tablo
  sayesinde R3'teki `positions` biçim hatası ve bilerek hatalı yazılan grup adları kalktı
  (R3: 3 kayıp → R4: biçim hatası 0).

### 4.3 İstek okuması (r1 C01, C13 ×2)

- C01 "Bugün THYAO'da ne var?": r1 A "bugün"ü son bar okudu (`since_bar`), yalnız o gün hareket
  eden kırılımı verdi, açık aşırılığı ve devamı atladı. r2 A aynı okumayla başladı ama açıkları
  da listeledi (geçti). İki ajan da "bugün → bar" eşlemesini ve son bar sırasını bulmakta zorlandı:
  *"welcome.summary series len=317 but last processed bar is 315, off-by-one confusion"*. R3'te
  C01 iki koşuda da geçmişti; fark muhtemelen cümlelerin artık "3 bar önce … bu barda" demesi ve
  ajanın zamanı ciddiye alması (ölçülmedi).
- C13 "en son gerçekleşen kırılım": R3'teki istek belirsizliği (R3 RESULTS §1.3, kendi hatam).
  İki koşuda da ajan seçiciyi doğru ve kimliksiz yazdı, ama "en son doğan"ı seçti. Ayrıca bir
  **şartname tutarsızlığı** buldu (r2 C, r1 D C03 de not etti): §17.1'de `latest` "en son hareket
  eden" (son geçiş barı), §20.2'de `capsules.latest` "en son doğan". Aynı adın iki anlamı var.

### 4.4 Son durumda kalan katı kayıplar

C02 ×2 ve r1 C10, `input open` ile doğru tamamlandı; yalnız R3 puanlayıcısı bu yolu tanımıyor
(§3.3). r2 C01'in son denemesi açıkları listelemeyen bir sorguydu (cevap doğru, R3 kuralı son
denemeye bakıyor). Geri kalanlar §4.1 ve §4.3.

### 4.5 R3'te zorlanıp artık sorunsuz geçenler

- Bulgu seçicisinde `meaning` + `open` (R3 §4.2'deki `{"kind":"breakout","state":"live"}`
  hatası): R4 ilk denemelerinde hiç görülmedi.
- `verify_evidence.item` seçici (R3'te `schema`): iki koşuda ilk denemede geçti.
- Kitle biçimi (R3 §4.3): biçim hatası yok.
- C17: `born_since_bar` kullanıldı (R3'te istemci taraması). Sayım hâlâ istemcide.

### 4.6 Ajanların süregelen notları (geçen görevlerde)

- Son bar sırası doğrudan sorulamıyor (`welcome.summary` `len` ile cümlelerdeki "N bar önce"
  çelişiyor gibi görünüyor: son **kesin** bar mı, son bar mı) (C01, C17 ×2).
- `dispute` için `t` (doğum mu son geçiş mi) ve `values` belirsiz (C15 ×2; R3'te de).
- "En önemli" kapanmış hikâyeleri kapsıyor mu (C05 ×2; R3'te de).
- `present` derinliği yüzeyle seçiliyor (`surface: phone` = kapsül derinliği; C09 ×2; R3'te de).
- Çıktılar çok büyük (`welcome`, `spark` dizileri); ajanlar kırpıyor (R3'te de).
- `validity: closed` ile cümledeki "artık geçerli değil" arasındaki ilişki (C16, bir not).

## 5. K42 kuralları görevlerin doğruluk anahtarını değiştiriyor mu?

Altı `off`'ta (R3 ham kayıtlarındaki `truth.json.gz` ↔ aynı `off` için R4 koduyla gizli kurulum)
her görevin anahtarı karşılaştırıldı (açık THYAO kırılımı/aşırılığı, ASELS'in en son geçersizi,
en son gerçekleşen kırılım, sıkışma ve aşaması, en son akış, açıkların kümesi, önem sırasıyla ilk
üç (bütün / açık), son on barda doğanların tür sayıları, gerçekleşen sayısı, karnede kırılım
`n`/`realized`):

- **Değişen tek anahtar C05** (en önemli üç): yönlü karne (A23) `ledger_at_birth`'i, dolayısıyla
  önemi ve sırayı değiştiriyor. Beş `off`'un hepsinde ilk üç farklı (ör. `off` 0: R3
  `THYAO.breakout.up.174, THYAO.breakout.up.45, ASELS.breakout.up.66` → R4
  `ASELS.breakout.up.66, ASELS.breakout.up.135, ASELS.breakout.down.230`). Açıkların ilk üçü de
  yer değiştiriyor. Puanlayıcı gerçeği gizli durumdan aldığı için sorun yok; C05 iki koşuda da geçti.
- **C06 karne:** `by_kind` yönleri topluyor; kırılım satırı aynı (`off` 0: n 74, realized 42).
  Ama `by_kind`'sız `ledger` artık yön başına satır veriyor. Yön satırlarını ayrı veren bir
  cevap, toplamı söylemezse R3 ölçütüne (toplam `n` ve `realized`) uymaz. Bu ölçümde iki ajan da
  toplamı verdi.
- **A22 (olumsuz geçiş):** görevlerde `audience` süzgeci yok (C12 aboneliği yalnız `subscribed`
  ile puanlanıyor). Anahtarı etkilemiyor.
- **A28 (geçerlilik):** açık THYAO kapsüllerinin hepsi `valid`, sıkışma `invalidated`/`closed`.
  C16 ("hâlâ geçerli mi") ve C10 ("seviye çiziliyor mu") anahtarı aşamayla aynı. C10'da olgu
  kökenli `rule` katmanı hâlâ çiziliyor (`selftest-ref` ve `-sel` C10 ✓). Bir hikâye açık ama
  `no_longer` olsaydı C16'nın "geçerli" anahtarı aşamadan değil `validity`'den alınmalıydı; bu
  kurulumda böyle bir durum yok.

## 6. Ölçütün değerlendirmesi (GOAL 13, ikinci yarı) ve R3 ile karşılaştırma

| | İlk denemede | Kimliksiz ilk | Sonunda (katı) | Sonunda (ikinci okuma) | Hoşgörülü | Maliyet (5 ajan) |
|---|---|---|---|---|---|---|
| R3 r1 | 12/20 | 10/20 | 19/20 | — | 20 | 3,86 $ |
| R3 r2 | 11/20 | 9/20 | 20/20 | — | 20 | 4,36 $ |
| **R4 r1** | **14/20** | 12/20 | 15/20 | 17/20 | 18 | 3,56 $ |
| **R4 r2** | **17/20** | 15/20 | 16/20 | 17/20 | 18 | 5,06 $ |
| R3 → R4, dokuz "kimlik" görevi, ilk deneme | 4/18 → **14/18** | 0/18 → **10/18** | | | | |

- İlk denemede ortalama 11,5 → **15,5** /20, kimliksiz 9,5 → 13,5. Hedeflenen kayıp sınıfları
  (R3 §4.1 kimlik keşfi, §4.2 bulgu seçicisi sözlüğü, §4.3 kitle tipi) kapandı: R3'ün 17 ilk
  deneme kaybından bu sınıflara düşen 17'si R4'te 4'e indi (C10 ×2 ve C08/C18 keşifleri; dördü
  de seçici kabul etmeyen ya da tam değiştiren yollar).
- **Sonunda** katı sayı R3'ten düşük (19–20 → 15–16). Bunun nedeni şartname değil, ölçüm: R3
  puanlayıcısının bilmediği yeni `input open` yolu (3 görev) ve yetkisiz seçicinin yeni hata
  sınıfı (C11 ×2). İkinci okumada 17/20 (hoşgörülü 18/20). R3'te C11 sonunda 2/2 geçiyordu,
  R4'te 0/2: bu gerçek bir **gerileme**. Seçici, yetkisiz okuyucuyu sessizce yanlış hikâyeye
  götürüyor.
- İki koşu tutarlı: 20 görevin 17'sinde ilk deneme sonucu aynı; farklar C01, C08, C18 (istek ve
  keşif tercihi).

## 7. Kalan öneriler (yapılmadı; şartname/protokol)

1. **Yetkisiz seçici ipucu (C11).** `capsule` sorgusunda (ve seçicili her kapsül/bulgu
   başvurusunda), okuyucunun göremediği lisanslı eşleşmeler varsa `capsules`'taki gibi mühürlü
   yorum ya da `result.notes: [{requires: "flow/*", newer_matches: true}]` dönsün. Gizlilik
   kuralı (§20.5) zaten `capsules`'ta varlığı mühürlü yorumla söylüyor; aynı bilgi.
2. **`latest` adının iki anlamı.** §17.1 seçicide "son geçiş barı", §20.2 `capsules.latest`
   "en son doğan". Ya `capsules`'ta `latest_born` / `order: "born"` olsun, ya seçicide de
   ayrım (`latest` + `by: "born" | "moved"`) yazılsın; §17.1 ve §20.2 birbirine atıf versin.
3. **`input open.capsule` seçici alsın** (`CapsuleRef`; §17.2 tablosuna `input` `open.capsule`).
   C10'daki iki keşif denemesinin kaynağı bu; ajanlar "aç/göster" için açılma hareketini
   doğal buluyor.
4. **`dispute.claim.capsule` ve `run.sh --present` / `present.finding` seçici yolu.** C15 ve C20
   hâlâ başka görevin kimliğine dayanıyor (`dispute.claim` T3 önerilerinde de açık).
5. **Son bar.** `welcome` ya da `capsules` cevabı son (kesin) bar numarasını ve zamanını açıkça
   versin. "Bugün", "son on bar" eşlemesi ajanlar için tahmin (C01, C17).
6. **Önceki R3 önerilerinden açık kalanlar:** `capsules` `count_by: "meaning"` (C17), `present`
   derinlik parametresi (C09), `dispute` `t`/`values` örneği (C15), "en önemli" ile açıklık
   (C05), `set_audience.merge` / izleme listesine ekleme (C08/C18 keşifleri).
7. **Ölçüm düzeneği (sonraki ölçüm için):** puanlayıcı `input open` yolunu doğrudan tanısın
   (`score_r4.py` mantığı `check()`'e). C13 isteği "aşaması gerçekleşti olan en son kırılım"
   diye yazılsın (R3 önerisi 12; karşılaştırma için bu ölçümde değiştirilmedi). C16'nın anahtarı
   `validity`'den alınsın (K42/A28). Ajan başına ayrı `HOME` (R3 önerisi 12) hâlâ yapılmadı;
   onun yerine profil başka oturumların proje klasörlerini reddediyor (§1.1 #5).

## 8. Maliyet

| Koşu/ajan | A | B | C | D | E | Toplam |
|---|---|---|---|---|---|---|
| r1 | 0,700 $ | 0,743 $ | 0,618 $ | 0,728 $ | 0,768 $ | **3,56 $** |
| r2 | 0,796 $ | 0,650 $ | 2,113 $ | 0,712 $ | 0,793 $ | **5,06 $** |

| | Belirteç (girdi + önbellek + çıktı) | Çıktı belirteci | Ajan süresi (toplam) | Maliyet |
|---|---|---|---|---|
| r1 (5 ajan) | 7 087 901 | 49 685 | 434 s | 3,56 $ |
| r2 (5 ajan) | 9 216 913 | 50 197 | 453 s | 5,06 $ |
| Yalıtım testi | — | — | 14 s | 0,05 $ |
| **Toplam** | **16 304 814** | 99 882 | 901 s | **8,67 $** |

r2 C'nin 2,11 $'ı önbellek yazımından geliyor (448 k belirteç; ötekilerde 100–130 k). Ajan
`document` cevabını (~670 KB) iki kez okudu. Duvar saati koşu başına ~1,7 dakika.

## 9. Düzen ve yeniden üretim

```
harness/                     R3 kopyası + selftest_sel.py, score_r4.py, sectest.py (§1.1)
raw/<koşu>/agent<X>/         setup.json, truth.json.gz, prompt.txt, TASKS.txt, sandbox.sb, command.json,
                             transcript.jsonl (+ .ts), process.json, runs.log, stderr.txt,
                             C??/runs/<n>-<dosya>.jsonl (+ .out.jsonl.gz, .hidden.json, .png.json,
                             .transcript.json), C??/{attemptN.jsonl, notes.txt, answer.txt, *.png.sha256}
raw/<koşu>/score.json        R3 puanlayıcısı (katı); score_r4.json ikinci okuma
raw/SCORE.md, raw/SELFTEST.md, raw/audit.txt, raw/isotest/, raw/selftest-{ref,empty,sel}/
```

Yeniden üretim: `CARGO_TARGET_DIR=$SCR/target cargo build --release -p qp-cli` ve
`harness/gen` (aynı hedef klasör), `setup.py`'deki `SCR`'yi (ya da `E_SCR`) ayarla;
`e3.py packet`; `e3.py serve &`; `selftest.py`, `selftest_sel.py`, `sectest.py`;
`e3.py launch <A–E> --label <koşu>`; `score.py r1 r2`; `score_r4.py r1 r2`; `audit.py r1 r2`.
Tam otorite çıktısı depoda değil; PNG'ler (sentetik veri) yalnız boyut ve SHA-256.
