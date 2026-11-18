# M18 — hikâye çizelgesi: ölçüm sonucu

Ölçüm ajanı `r5/m18-measure`, 2026-11-18. Düzenek: `rounds/R5/experiments/harness/glance/`
(SPEC §G değişmedi; M18 için eklenen kod aşağıda "Düzenek ekleri"). Ham koşu (ekran görüntüleri
hariç): [`run/`](run/); bütün tablolar `run/tables.md` (aynı sayılar, `tables.py` ile üretildi).

> Ajan okuyucu insan değildir; bu ölçüm bir ön elemedir (SPEC §G0). Son söz Arkin'in telefondaki
> karşılaştırması ve insan testindedir.

## Özet

| | `korda` | `selen` | `pavka` | `r4` (başvuru) | bozuk |
|---|---|---|---|---|---|
| Birincil: `chart` doğruluğu (F2:H1, %95) | 0,928 [0,907, 0,948] | 0,868 [0,842, 0,892] | **0,987 [0,977, 0,996]** | 0,411 [0,384, 0,438] | 0,406 [0,377, 0,436] |
| Canlıyı bulma (`c_live`, kapanmışlar görünürken) | 0,96 | **0,52** | 1,00 | 0,01 | 0,00 |
| Eleştirmen BT (%95) | 1,37 [0,90, 2,01] | −0,75 [−1,45, −0,22] | **3,82 [3,18, 4,63]** | −0,25 [−0,91, 0,36] | −4,20 [−4,70, −3,82] |
| K52 JIT açık (bağlayıcı; 5/50/200) | geçti | geçti | geçti | geçti | — |
| K52 JIT kapalı: sürüklemede uzun kare ≤ %10 (5/50/200) | geçti (0 / 0 / 0) | geçti, sınırda (0 / 0 / **8,6 %**) | **kaldı** (0 / **13,5 %** / 8,6 %) | geçti (0) | — |

**Seçim (K52'nin tamamı kapı olarak; ana karar):** `pavka` K52'nin JIT'siz koşulunda (50 hikâyede
sürüklemede uzun kare %13,5; iki ayrı koşuda da > %10) elenir → birincilde `korda` tek başına ilk
grupta (aralığı `selen`'inkiyle örtüşmüyor) → **kazanan `korda`**, ayırt edildi. İlk iki
(gösterimde seçilebilir): **`korda`, `selen`**. Bütünleştirmeye girecek varsayılan: **`korda`**.

**Duyarlılık (SPEC §G7.6'nın lafzı: K52 kapısı yalnız JIT açık tahmini):** hiçbir aday elenmez →
birincilde `pavka` tek başına ilk grupta → kazanan **`pavka`**, ilk iki `pavka`, `korda`. İki okuma
arasındaki tek fark `pavka`'nın JIT'siz sürükleme payıdır; karar orkestratörün (aşağıda "Açık
soru 1").

Bozuk tasarım birincilde ve BT'de kesin son → koşu geçerli (§G8). Ama birincilde `r4`'ten ayrımı
yok denecek kadar küçük (eşleştirilmiş fark +0,006 [−0,028, +0,039]); aşağıda "Sorunlar 1".

Maliyet: **3,84 $** (okuyucu 750 çağrı 1,23 $; eleştirmen 180 çağrı 2,60 $); bütçe 25 $.

## Düzenek ve protokol (okumalardan önce sabitlendi)

**Adaylar ve derlemeler.** Dallar birleştirilmedi; her aday kendi dalının `git archive` kopyasından
(scratchpad, depo dışı) ayrı derlendi (`qp` release + `scripts/demo.sh` WASM paketi;
`CARGO_TARGET_DIR=target/wt/m18-measure-<ad>`):

| ad | kaynak | seçim |
|---|---|---|
| `korda` | `r5/m18-korda` bc8fc37 | `?story=korda` |
| `selen` | `r5/m18-selen` 27f0ea2 | `?story=selen` |
| `pavka` | `r5/m18-pavka` d62207a | `?story=pavka` |
| `r4` (başvuru) | `main` 2e06721 + `broken/look.patch` (yalnız bozuk kayıt eklenir; `r4` görünüşü `main` ile aynı) | yok (bugünkü tek kapsüllü grafik) |
| `kestim` (bozuk, Ek G3; gizli ad) | aynı derleme | `?look=kestim` |

Üç aday dalı `c414e44`'ten (M18 protokol eki) çıkıyor; `main` sonra M19'u aldı (akış sırası, yazı
tura kuralı gösterim varsayılanı). Aynı içerik denetlendi: 60 görüntünün hepsinde açılan kapsül,
görünen bar aralığı ve cihazın tuttuğu hikâye listesi (533 satır: kimlik, doğum, kapanış, sonuç,
geçerlilik) beş derlemede birebir aynı (`run/consistency.txt`).

**Durum (SPEC §G2 `chart5/50/200`, GOAL "görünen aralıkta 5 / 50 / 200 kapsül").** Tek sentetik
pazar: `qp demo --synthetic --days 2600 --history 2500`, yeniden oynatmanın son adımı (900/900).
Kural (`glance.py chartstates`, okumadan önce yazıldı): okurun akışındaki enstrümanlardan sembol
sırasıyla ilk, listesinde ≥ 200 hikâye, ≥ 1 canlı hikâye olan ve en son kapanan hikâyesi son 10
bar içinde kapanmış olan → **SYN02** (533 hikâye; canlı: `continuation.down.2582`,
`reversal.down.2589`, `reversal.up.2594`). Açılan kapsül = en son kapanan hikâye
**`cap.SYN02.flow.up.2586`** (tutmadı). Derinlikler aynı açık grafiğin yakınlaştırılmasıyla
(tekerlek fiyat alanının sağ ucunda, son bar görünür kalır): görünen aralıkla ömrü kesişen hikâye
sayısı (PROTOCOL §23.1 kuralı) hedefe ulaşan en küçük değer → **5** (bar 2582–2599), **50**
(2431–2599), **212** (1592–2599; tekerlek adımı 200'ü tam vermiyor). 5 tasarım × 3 derinlik × 2 dil
× 2 tema = **60 görüntü** (402×874, DPR 1, `canvas=run`). Örnekler: [`samples/`](samples/) (50
hikâye, tr/açık, kırpılmış, sentetik).

**Sorular (`chart` kümesi, `gl/keys.py`; anahtar motorun hikâye listesinden ve görünen aralıktan,
tasarımdan değil):**

| kod | soru (TR) | anahtar | şans |
|---|---|---|---|
| `c_recent` | Grafikte görünen dönemde bu hissede kaç olay (hikâye) var? (hiç / 1–9 / 10–29 / 30–99 / 100+) | ömrü aralıkla kesişen hikâye sayısının kovası | 0,20 |
| `c_held` | Grafikte görünen, bitmiş olayların kaçı tuttu? (çoğu / yarısı / çoğu tutmadı / bitmiş yok) | görünen kapanmışlarda gerçekleşti payı (≥ 0,6 / 0,4–0,6 / < 0,4; geri çekilen sayılmaz) | 0,25 |
| `c_live` | Şu an hâlâ süren (canlı) olay hangi numaralı bölgede başladı? (birden çoksa herhangi biri; 1–4 / süren yok) | canlı hikâyelerin doğum barının görünen aralıktaki çeyreği (çizgiye %10 yakın ya da aralık dışı doğum puanlanmaz) | 0,20 |
| `c_evid` | Vurgulanan (seçili) işaret neye dayanıyor; ne tür bir şey gösteriyor? (6 sade tür) | açık kapsülün türü (`flow`) | 0,167 |

Numaralı sütunlar düzeneğin çizgileridir (fiyat alanının x kapsamı, mum gövdelerinden; dört eşit
bar dilimi); okuyucuya bu söylenir. Anahtarlar: 5 → `1-9`, `few`, {1,2,3}, `flow`; 50 → `30-99`,
`half`, {4}, `flow`; 212 → `100+`, `half`, {4}, `flow`.

Okuyucu: haiku, personalar F ve H, **6 tekrar** (birincil, Ek G3), seçenek sırası tohumla karışık,
tek bakış (tek `Read`, `Bash` yok; 750 çağrının hepsi `one_look`). Sonnet alt kümesi: tr/açık, 1
tekrar (30 çağrı). Eleştirmen: sonnet, 10 çift × 3 derinlik × 2 sıra × 3 tekrar, tr/açık (180
çağrı); istemde tasarım adı yok (`check_prompt` her istemde).

### SPEC'ten sapmalar (gerekçeleriyle; hepsi okumalardan önce)

1. **`--history 2500` ve tek pazar.** `states --charts` (`qp replay --days {260,1000,2600}`, toplam
   kapsül sayısı) 2600 günde 3 dakikadan uzun sürüyor ve en küçük enstrüman 260 günde bile 21
   kapsüllü (5 yok). GOAL'ın ölçüsü *görünen aralıktaki* sayı olduğu için derinlik yakınlaştırmayla
   kuruldu; hikâye listesi geçmişin hikâyelerini taşıdığından yeniden oynatma kısa (arama ≈ 16 s).
   Üç derinlik aynı açık kapsülü gösterir.
2. **"Son üç ayda kaç olay" → "görünen dönemde kaç olay".** Görünen pencere 5/50/200 kuralıyla
   belirlenince (5 hikâye ≈ 18 bar) "son üç ay" 5 derinliğinde görünmüyor; soru görünen döneme
   bağlandı, kovalar 5/50/200'ü ayıracak biçimde. GOAL'ın "son üç ay" görevi bu yüzden doğrudan
   ölçülmedi.
3. **Harfli değil numaralı bölge.** Seçenekler harfli olduğu için bölge harfleri karışıyordu
   ("A) B"); bölgeler 1–4. İlk iki deneme çağrısında okuyucu üç canlı hikâye görüp "hangisi
   sorulduğu net değil" dedi; soruya "(birden çok varsa herhangi biri)" eklendi, iki deneme kaydı
   silindi (anahtar zaten herhangi birini kabul ediyordu).
4. **Birincil ölçü tam boy `chart` doğruluğu** (SPEC §G9 M18 satırı); kısma kopyası sorulmadı (yüz
   soruları grafiğe uymuyor). Adım 1b ve adım 4 (yüz okuma) M18'de yok: yüzler tasarımlarla
   değişmiyor (`score --primary chart --reading ""`).
5. **Deterministik kapı açık sahnenin boyama listesinden** (`scene_desc`): katman ve isabet kutusu
   yok (SPEC §G11.5). Metin dışı karşıtlık = anlam rengindeki (up, down, closed, invalid, valid,
   focus, neutral, warn), alfa ≥ 0,5 her metin dışı ilkel, altındakine karşı; renk ayrımı = grafiğin
   metin dışı boyadığı anlam rolü çiftleri (§G7.4 beş koşul; işaret boyu bilinemediği için sıkı
   eşik 15); dokunma = DOM kutuları (hikâye isabet kutuları ölçülemedi).
6. **K52 senaryosu** (`k52chart.mjs`): aynı pazar, `steps − 400`'e arama (yeniden oynatma
   oynayabilsin), geçmişte doğmuş bir SYN02 kapsülü açılır, aynı yakınlaştırma kuralı, sonra 5 s
   oynatma (tik, veriyle sahne boyama) ve 6 s sürükleme (±40 px üçgen dalga, ≈ 0,8 s basılı
   sürüklemeler; `frame_ms.drag`). Uzun kare payı çekirdeğin kendi `long` ölçüsü (p99 vekili
   gerekmedi).

## Deterministik kapı (§G7; ajan ölçümünden önce)

| tasarım | metin karşıtlığı en kötü / eşik (öğe) | metin dışı en kötü (rol, ilkel) | en küçük yazı | dokunma < 44 (DOM) | renk ΔE00 en kötü çift (koşul) | sahne metni ort. karakter | geçti |
|---|---|---|---|---:|---|---:|---|
| `korda` | 3,57 / 4,5 (`36.32`, 11 px) | 1,10 (focus, çizgi) | 11 | 60 | up ↔ closed: 0,13 / 15 (gri) | 110 | hayır |
| `selen` | **1,88** / 4,5 (`5`, 11 px; koyu tema küme sayısı) | 1,02 (closed, daire) | 11 | 60 | up ↔ closed: 0,13 / 15 (gri) | 114 | hayır |
| `pavka` | 3,57 / 4,5 (`36.32`, 11 px) | 1,08 (focus, dolgu) | 11 | 60 | up ↔ closed: 0,13 / 15 (gri) | 180 | hayır |
| `r4` | 3,57 / 4,5 (`C 36.32`, 16 px) | 2,80 (up, mum) | 11 | 60 | up ↔ down: 0,63 / 15 (gri) | 49 | hayır |
| bozuk | 1,53 / 4,5 | 1,27 | 11 | 60 | up ↔ down: 1,79 / 15 (gri) | 48 | hayır |

**E6 (§G9.1) uygulanışı:** metin karşıtlığı, metin dışı karşıtlık, dokunma ve renk eşiğini hiçbir
aday, `r4` dahil, geçemedi → bu dört eşik **uygulanmadı**. Ortak nedenler: grafiğin başlığındaki ve
eksendeki son fiyat yazısı `up` renginde (3,57:1, her tasarımda aynı `r4` öğesi), kabuğun düğmeleri
(Akış / Ayarlar / Ölçüm 33 px, Geri / Kanıt 29 px yüksek), gri koşulunda `up` ile `closed` (ve `up`
ile `down`) aynı ışıklıkta. En küçük yazı (11 px) herkeste geçti. Kalan sert eşik K52.

Bilgi (eşik değil, tasarıma özgü): `selen` koyu temada küme sayısını (`5`) kapanmış renkli dairenin
üstüne 1,88:1 ile yazıyor ve kapanmış daireleri üst üste bindirince 1,02:1'e düşüyor; `korda` ve
`pavka`'nın en kötü metin dışı öğesi odak renginin kendi odak bandı üstündeki kılavuzu/satır
zemini (dekor; anlam taşıyan işaret değil). Hikâye işaretlerinin dokunma kutuları ölçülemedi
(tasarım notları: `korda` şerit 6–13 px, `selen` en az 32 px kutu, `pavka` şerit 14–28 px; hepsi
44'ün altında, beyan).

## K52 (5 / 50 / 200 hikâye görünürken; ms; ortanca)

| tasarım | kip | görünen | tik p95 | sahne boyama p95 | sürüklemede kare p99 | sürüklemede uzun kare payı | geçti |
|---|---|---|---|---|---|---|---|
| `korda` | JIT açık (`phone-jit-est`) | 6 / 52 / 219 | 1 / 1 / 1 | 1 / 2 / 2 | 19 / 19 / 19 | 0 / 0 / 0 | evet |
| `selen` | JIT açık | 6 / 52 / 219 | 1 / 1 / 1 | 1 / 2 / 2 | 19 / 18 / 19 | 0 / 0 / 0 | evet |
| `pavka` | JIT açık | 6 / 52 / 219 | 1 / 1 / 1 | 1 / 3 / 2 | 19 / 19 / 18 | 0 / 0 / 0 | evet |
| `r4` | JIT açık | 6 / 52 / 219 | 1 / 1 / 1 | 1 / 2 / 1 | 19 / 19 / 19 | 0 / 0 / 0 | evet |
| `korda` | JIT kapalı (`phone`), 1. koşu | 6 / 52 / 222 | 1 / 1 / 1 | 5 / 13 / 13 | 19 / 23 / 22 | 0 / 0 / 0 | evet |
| `selen` | JIT kapalı, 1. koşu | 6 / 52 / 219 | 1 / 1 / 1 | 5 / 13 / 20 | 19 / 24 / 33 | 0 / 0,003 / **0,133** | hayır |
| `pavka` | JIT kapalı, 1. koşu | 6 / 52 / 222 | 1 / 1 / 1 | 11 / 18 / 23 | 20 / 32 / 38 | 0 / **0,218** / **0,140** | hayır |
| `r4` | JIT kapalı, 1. koşu | 6 / 52 / 222 | 1 / 1 / 1 | 4 / 9 / 10 | 18 / 19 / 20 | 0 / 0 / 0 | evet |

İlk koşu 3 tekrar ve okuyucu çağrılarıyla eşzamanlıydı; JIT kapalı kip bu yüzden okumalar bittikten
sonra **5 tekrarla yeniden koşuldu** ve kapıda yeniden koşu kullanıldı (kural yeniden koşu
sonuçlanmadan `glance.py compute`'a yazıldı):

| tasarım | uzun kare payı 5 / 50 / 200 (ortanca) | 5 koşunun tek tek değerleri (50; 200) | sahne boyama p95 5 / 50 / 200 | geçti (≤ %10) |
|---|---|---|---|---|
| `korda` | 0 / 0 / 0 | 0, 0, 0, 0,003, 0 ; 0,003, 0, 0, 0,006, 0 | 5 / 13 / 13 | evet |
| `selen` | 0 / 0 / **0,086** | 0 × 5 ; 0,086, 0,096, 0,086, 0,063, 0,066 | 6 / 13 / 16 | evet (sınırda) |
| `pavka` | 0 / **0,135** / 0,086 | 0,175, 0,15, 0,135, 0,121, 0,10 ; 0,106, 0,101, 0,066, 0,086, 0,075 | 11 / 18 / 22 | **hayır** (50'de) |
| `r4` | 0 / 0 / 0 | 0 × 5 ; 0 × 5 | 4 / 9 / 10 | evet |

Okuma: K52'nin bağlayıcı koşulunda (HTTPS, JIT açık) bütün tasarımlar 200 hikâyede bile bütçenin
çok altında (sahne p95 ≤ 3 ms). JIT kapalı kalibre kipte maliyet sırası `r4` < `korda` < `selen` <
`pavka`; `pavka` 50 hikâyede beş yeniden koşunun beşinde de %10'un üstünde (sürüklemede sahne
p95 18 ms, kare p99 32 ms). `PHONE_BIAS` (×1,85) ile telefon tahmini sahne p95: `korda` 24, `selen`
30, `pavka` 41, `r4` 19 ms (200 hikâye). Makine başka ajanlarla paylaşılıyordu (yük 7–15).

## Görev doğruluğu (birincil) ve dokunma sayısı

| tasarım | hepsi (F2:H1, %95) | 5 hikâye | 50 hikâye | 212 hikâye | F | H |
|---|---|---|---|---|---|---|
| `korda` | 0,928 [0,907, 0,948] | 0,987 [0,967, 1,000] | 0,863 [0,820, 0,903] | 0,933 [0,903, 0,960] | 0,953 | 0,877 |
| `selen` | 0,868 [0,842, 0,892] | 0,877 [0,840, 0,913] | 0,890 [0,847, 0,927] | 0,837 [0,787, 0,883] | 0,883 | 0,837 |
| `pavka` | **0,987 [0,977, 0,996]** | 0,993 [0,980, 1,000] | 0,983 [0,960, 1,000] | 0,983 [0,960, 1,000] | 0,987 | 0,987 |
| `r4` | 0,411 [0,384, 0,438] | 0,540 [0,500, 0,577] | 0,340 [0,303, 0,373] | 0,353 [0,310, 0,403] | 0,437 | 0,360 |
| bozuk | 0,406 [0,377, 0,436] | 0,573 [0,530, 0,617] | 0,310 [0,280, 0,343] | 0,333 [0,300, 0,370] | 0,410 | 0,397 |

Birim = görüntü × persona × tekrar × model (75 + 75 birim/tasarım; seçim kuralının girdisi). Birim
şansı ≈ 0,20.

Soru başına (haiku, F+H, n = 144; Wilson %95):

| tasarım | `c_recent` (kaç olay) | `c_held` (kaçı tuttu) | `c_live` (canlı nerede) | `c_evid` (seçilinin kanıtı) |
|---|---|---|---|---|
| `korda` | 0,97 [0,92, 0,99] | 0,74 [0,67, 0,81] | 0,96 [0,91, 0,98] | 0,98 [0,94, 0,99] |
| `selen` | 0,91 [0,85, 0,95] | 1,00 [0,97, 1,00] | **0,52 [0,44, 0,60]** | 0,99 [0,95, 1,00] |
| `pavka` | 0,99 [0,96, 1,00] | 0,96 [0,91, 0,98] | 1,00 [0,97, 1,00] | 1,00 [0,97, 1,00] |
| `r4` | 0,31 [0,24, 0,39] | 0,29 [0,22, 0,37] | 0,01 [0,00, 0,04] | 1,00 [0,97, 1,00] |
| bozuk | 0,28 [0,22, 0,36] | 0,34 [0,27, 0,42] | 0,00 [0,00, 0,03] | 0,99 [0,96, 1,00] |

Derinliğe göre zayıf noktalar (oran, n = 24): `korda` `c_held` 50'de 0,48 (özet kipinde başlık
yalnız "canlı 3 · kapanmış 47", sonuç dağılımı histogram dolgusunda); `selen` `c_recent` 212'de
0,75 (sayı lejantta sonuçlara bölünmüş, toplam yok) ve `c_live` her derinlikte ≈ 0,5 (144
cevabın 66'sı "bilmiyorum": canlı halkalar fiyatın üstünde, kümelerin arasında seçilmiyor). `r4`
5'te `c_recent` 0,85 (anahtar `1-9`; okuyucu tek işaret görüp "1–9" diyor), canlı sorusunda
neredeyse hep "süren olay yok" (grafikte yalnız açık kapsül var: beklenen).

**Kapanmışlar görünürken canlının bulunması** (`c_live`, haiku, Wilson %95; insan testinde süre):

| tasarım | 5 | 50 | 212 |
|---|---|---|---|
| `korda` | 0,98 [0,89, 1,00] | 0,94 [0,83, 0,98] | 0,96 [0,86, 0,99] |
| `selen` | 0,48 [0,34, 0,62] | 0,56 [0,42, 0,69] | 0,52 [0,38, 0,66] |
| `pavka` | 1,00 [0,93, 1,00] | 1,00 [0,93, 1,00] | 1,00 [0,93, 1,00] |
| `r4` | 0,00 [0,00, 0,07] | 0,00 [0,00, 0,07] | 0,02 [0,00, 0,11] |

**Dokunma sayısı:** akıştan grafik derinliğine **2** (yüz + sahne), beş tasarımda aynı (tasarımlar
grafiğe giden yolu değiştirmiyor; hedef kapsül okurun 40 satırlık akışında olmadığı için düzenek
onu çekirdeğin `open_capsule` yoluyla açtı, sayım aynı). Grafik içi görevlerin dokunuşu (başka
hikâyeye odak, kümeye yakınlaşma, kapanmışları gizle) **ölçülmedi**: hikâye isabet kutuları kayda
girmiyor (§G11.5). `r4`'te canlı olana ulaşmak için akışa dönmek gerekir; adaylarda tasarım
notlarına göre tek dokunuş — bu beyan, ölçüm değil.

**İkinci boy (sonnet, tr/açık, 1 tekrar):** `korda` 1,000, `selen` 1,000, `pavka` 0,944, `r4`
0,333, bozuk 0,361; haiku ile aynı uyaran/persona/soruda uyum 0,875 (n = 120). Sıralama haiku'yla
aynı yönde (adaylar ≫ `r4` ≈ bozuk); adaylar arası fark bu küçük alt kümede ayrılmıyor.

## Eleştirmen (ikili kör, sonnet)

| tasarım | BT gücü [%95 önyükleme] | ham kararlar (kazandı / kaybetti; "ayırt edemiyorum" 0) |
|---|---|---|
| `pavka` | **3,82 [3,18, 4,63]** | 71 / 1 |
| `korda` | 1,37 [0,90, 2,01] | 51 / 21 |
| `r4` | −0,25 [−0,91, 0,36] | 32 / 40 |
| `selen` | −0,75 [−1,45, −0,22] | 26 / 46 |
| bozuk | −4,20 [−4,70, −3,82] | 0 / 72 |

**Çelişki oranı** (iki sırada farklı kazanan): bütün çiftlerde ≤ 0,111, yalnız `korda | r4` 3/9 =
**0,333 > %30 → bu çift ayırt edilemedi** (5'te eleştirmen `r4`'ün daha büyük grafiğini, 50/200'de
`korda`'nın şeritlerini seçiyor). **Uzunluk yanlılığı:** BT ile ekran karakteri Spearman 0,70;
kesin sonuçlarda daha çok metinli olanın kazanma payı 0,845 (n = 84) > 0,7 → **eleştirmen sonucu
"uzunluk yanlılığı şüpheli"** (§G5). BT seçimde kullanılmadı (birincil tek aday bıraktı), ama `pavka`
en çok metinli tasarım (sahne metni ort. 180 karakter; `korda` 110, `selen` 114, `r4` 49) ve
eleştirmenin gerekçeleri sık sık "457'de 207 tuttu", "canlı / tuttu / tutmadı" yazılarını anıyor.
`selen`'e karşı tekrarlanan gerekçe "üst üste binen sayılı daireler grafiği dolduruyor".

## Bozuk tasarımın yeri (§G8, Ek G3)

`glance.py broken-check`: birincilde **kesin son** (0,406; `r4` 0,411), BT'de **kesin son**
(−4,20), birincil aralığı en iyinin aralığının altında → **koşu geçerli**. Bilgi: eşleştirilmiş
birincil farkı adaylara karşı ayrık (`korda` +0,52, `selen` +0,46, `pavka` +0,58; alt sınırlar
> 0,42), **`r4`'e karşı ayrık değil** (+0,006 [−0,028, +0,039]).

## Seçim kuralının adım adım uygulanışı (§G9)

1. **Sert eşikler.** Metin karşıtlığı, metin dışı karşıtlık, dokunma, renk: hiçbir aday (r4 dahil)
   geçmedi → uygulanmadı (E6). En küçük yazı: herkes geçti. **K52:** JIT açık herkes geçti; JIT
   kapalı "sürüklemede uzun kare ≤ %10" (yeniden koşu): `pavka` 50 hikâyede 0,135 → **`pavka`
   elendi**. Kalan: `korda`, `selen`, `r4`. Bozuk seçime girmez.
   1b. Okuma aşağı kalmama kapısı: M18'de yüz okuma kümesi yok → uygulanmadı.
2. **Birincil (`chart`, F2:H1, %95):** `korda` 0,928 [0,907, 0,948] > `selen` 0,868 [0,842, 0,892]
   > `r4` 0,411. En iyinin aralığıyla örtüşen yalnız `korda` → ilk grup {`korda`}.
3. (İkincil BT) ve 4. (tam boy okuma): ilk grupta tek aday olduğu için gerekmedi.
5. **Kazanan `korda`** (ayırt edildi). Bağ çözme sırası (birincil → BT → okuma → daha az metin →
   ad) nokta tahminleriyle: `korda`, `selen`, `r4` → **ilk iki `korda`, `selen`**; varsayılan
   **`korda`**.

**Duyarlılık — K52 kapısı yalnız JIT açık (SPEC §G7.6 lafzı; `run/sensitivity-jit-only.json`):**
1: kimse elenmez. 2: `pavka` 0,987 [0,977, 0,996]; `korda`'nın üst sınırı 0,948 < 0,977 → ilk grup
{`pavka`}. Kazanan **`pavka`**; ilk iki `pavka`, `korda`; varsayılan `pavka`.

Hangi okumanın geçerli olduğu bir karar meselesi: SPEC §G7.6 JIT'siz kipi "kayıt, kapı değil"
diye yazdı (o gün uzun kare payı ölçülemiyordu, p99 vekili vardı); K52 ise "JIT'siz en kötü
durumda gösterim kullanılabilir kalır (sürüklemede uzun kare ≤ %10) … bütçeyi aşan tasarım
seçilmez" diyor ve görev tarifi iki kipi de istedi. Ana kararı K52'nin metnine göre verdim; bu
yorum JIT'siz ilk koşunun sayılarını gördükten sonra yapıldı (yeniden koşu sonucundan önce) —
bunu açıkça belirtiyorum.

## Açık sorular

1. **`pavka`'nın yeri.** Anlamada (0,987) ve eleştirmende (3,82) açık ara önde; tek kusuru JIT'siz
   kipte 50 hikâyede sürükleme (%13,5 uzun kare; JIT açıkta 3 ms). K52 metniyle elenir. Orkestratör:
   (a) K52'yi uygulayıp `korda`'yı varsayılan, `korda` + `selen`'i seçilebilir bırakmak; (b) `pavka`'yı
   da seçilebilir bırakıp önerdiği önbelleği (aralık değişmedikçe şerit yerleşimini yeniden kurmamak)
   bütünleştirmede denemek ve K52'yi yeniden ölçmek. Ölçüm (b)'yi destekler ama kural (a) diyor.
2. `selen` JIT'siz 212 hikâyede %8,6 (sınırda; ilk koşuda %13,3). Bütünleştirmede yeniden ölçülmeli.
3. GOAL'ın "son üç ayda ne oldu" görevi sapma 2 yüzünden doğrudan ölçülmedi; insan testinde
   sorulmalı.

## Sorunlar ve sınırlar

1. **Bozuk tasarım `r4`'ten ayrılmıyor.** Bozuk tasarım `main` üzerine (hikâye çizelgesi yok)
   kuruldu; `r4` de bu görevlerde çoğunu cevaplayamıyor (canlı/kaç olay/kaçı tuttu grafikte yok), iki
   tasarım da yalnız seçilinin türünü (cümleden) okuyor. Kesin son kuralı tuttu, ama farkın ayrık
   olmaması bu koşunun bozuk tasarımı ancak `r4` düzeyinde ayırdığını gösterir; adaylar arasındaki
   duyarlılığı sınamıyor. Sonraki M ölçümünde bozuk kayıt bir aday tasarımın üstüne kurulmalı.
2. Deterministik kapı boyama listesinden (katmansız) ölçüldü; hikâye işaretleri ile fiyat ve odak
   bandı ayrılamıyor, renk tablosu mum renklerini de içeriyor. Dört eşiğin herkesçe kalınması bu
   ölçünün kaba olduğunu da gösterir; açık sahnenin `SceneDesc`'i (katmanlar, isabet kutuları)
   kayda girmeli (§G11.5).
3. Kesilen cevaplar (ilk cümle/200 karakter): `korda` 77, `pavka` 76, `selen` 46 — okuyucu adaylarda
   uzun gerekçe yazıyor; puan harften, etkisi yok.
4. Tek model ailesi; okuyucu küçük metni okuyor (`c_evid` her tasarımda ≈ 1: üstteki cümle türü
   söylüyor, soru çizelgeyi ayırmıyor).
5. K52 makinesi paylaşımlı (yük 7–15); JIT'siz değerler WebKit simülasyonu (RP3 kalibrasyonu).
6. Hedef kapsül okurun akışında değildi (40 satır sınırı); grafiğe çekirdeğin `open_capsule` yoluyla
   inildi (dokunma sayısı aynı).

## Düzenek ekleri (bu dalda; SPEC değişmedi)

- `gl/keys.py`: `chart` soru kümesi (`c_recent`, `c_held`, `c_live`, `c_evid`) ve anahtarları
  (`count_key`, `held_key`, `live_keys`); §G9'un "M18 soru kümesi … `keys.py`'ye eklenir" maddesi.
- `glance.py`: `chartstates` (durum kuralı), `shoot --kinds chart` (tasarım başına kendi derlemesi:
  `designs.json`'da `qp`, `web`, `story`), `derive` (numaralı sütunlar, grafik eleştirmen çiftleri),
  `key_record` (hikâye listesi + görünen aralık), `compute` (grafik kapısı, K52 grafik biçimi ve
  yeniden koşu), `score --primary/--reading`, `k52 --charts`; okuyucu notu `CHART_NOTE`.
- `shoot.mjs`: `?story=`, son adıma arama (`step: -1`), `zoomTo` (görünen hikâye sayısına
  yakınlaştırma), `open_direct`, grafik-durum yoklaması (`chart_probe`).
- `gl/compute.py`: `chart_gates`, `chart_elements`, `chart_plot`; `gl/imageops.py`: `columns`;
  `gl/select.py`: birincil/okuma kümeleri parametre; `k52chart.mjs` (yeni); `tests/test_m18.py`.
- Birim testleri: 38 test geçti (`python3 -m unittest discover -s rounds/R5/experiments/harness/glance/tests`).

Yeniden üretim (özet): `glance.py chartstates` (main derlemesi) → `shoot --kinds chart` (tasarım
başına) → `derive` → `compute` → `k52 --charts` → `read --sets chart` (haiku; sonnet tr/açık 1
tekrar) → `critic` → `score --broken <gizli> --primary chart --reading ""` → `broken-check` →
`tables.py <run>`.
