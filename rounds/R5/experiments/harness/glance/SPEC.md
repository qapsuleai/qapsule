# R5/T2 — Tek bakış düzeneği şartnamesi (§G: görsel ölçümler M16, M18, M20, M21)

Yazan: `r5/t2-glance` alt ajanı, 2026-11-17, **hiçbir alternatif yazılmadan önce** (GOAL
"Ölçümle karar verilecek konular", K39; VERIFY 3). Bu dosyanın ve düzenek kodunun commit'i
her M16/M18/M20/M21 alternatif dalının ilk commit'inden önce olmalıdır. Dil ve akış
ölçümleri (M17, M19) başka bir ajanın `harness/lang/`, `harness/feed/` altındaki
şartnamesindedir; ortak `harness/SPEC.md` bu dosyaya "§G" diye bağlanır.

Girdiler: `rounds/R5/GOAL.md` (şartname), `rounds/R5/RESEARCH.md` §14–§17 (U1–U12, F1–F22),
R4 düzeneği ve dersleri (`rounds/R4/experiments/harness/SPEC.md`, `M8/RESULTS.md`,
`M10/RESULTS.md`, `docs/FACE.md` "Açık sorunlar", K46).

**Seçim kuralları sonuç görülmeden burada yazılıdır.** Sonradan değişirse değişiklik ve nedeni
sonuç notunda açıkça yazılır (R3 sapma 1, R4 E6 gibi) ve bu dosyaya "Ek" olarak eklenir.

## §G0 Ortak ilkeler ve R4'ün hatası

R4'te persona ajanları görüntüyü büyütüp uzun okuyabildi; ölçüm metin ağırlıklı tasarımı
kayırdı (K46). Bu düzenek bunu beş yoldan engeller:

1. **Gerçek boyut:** okur ekranı 402×874 CSS px, DPR 1 görüntü (iPhone 17 Pro'nun pt
   boyutu; büyütme yok). Model görüntüyü küçültmez (uzun kenar < 1568 px) ve büyütemez.
2. **Tek bakış:** tek görüntü, tek `Read`, `Bash` yok (kırpma/büyütme/kod yok), en çok 3 tur;
   her çağrı kaydında `one_look` (tam olarak bir `Read`, o tek dosya, `offset/limit` yok)
   denetlenir; tutmayan çağrı puanlamadan çıkarılır ve raporda sayılır (Ek G1-6).
3. **Kısa cevap:** soru başına EN ÇOK BİR CÜMLE; ilk cümle sonundan ve 200 karakterden
   kesilir, kesilme kaydedilir, puanlama kesilmiş metindendir (harf ilk karakterdedir).
4. **Kısma sınaması** (bulanık + küçük; metin okunmaz) **birincil ölçüdür** (§G3, Ek G1-2).
5. **Hesapla ölçülenler önce gelir:** sert eşikler (§G7) ajanlardan önce bir eleme
   kapısıdır (RESEARCH U3, "deterministik vekiller").

**Sınır** (her sonuç notu taşır): ajan okuyucu insan değildir; küçük metni insandan iyi
okur, renk/açıklık hiyerarşisini zayıf okur (RESEARCH §14.6). Bu ölçümler **ön elemedir**;
her konuda ilk iki aday gösterimde seçilebilir kalır, son söz Arkin'in telefondaki
karşılaştırması ve insan testi v3'tür. Tasarımları ve okuyucuları aynı model ailesi
üretiyor; kendini kayırma (U4) yazar bilgisi gizlenerek azaltılır ama giderilemez. İkinci
model ailesi: `OPENAI_API_KEY` bu ortamda **yok** → kullanılmadı; bu bir sınırdır (anahtar
istenmez, `.env.local` açılmaz). Aynı aileden iki boy (haiku + sonnet) arası uyum raporlanır.

## §G1 Görüntü üretimi

Araç: `shoot.mjs` (Playwright Chromium, `qp demo --synthetic --paused`, `shells/web/tests/lib.mjs`).

| Parametre | Değer | Gerekçe |
|---|---|---|
| Görünüm | 402×874 CSS px, DPR 1 | okurun gördüğü boyut (GOAL); `calibrate.mjs` `VIEW_PHONE` ile aynı |
| Boyama yolu | `?paint=canvas&canvas=run` (parametre: `--paint`) | ürünün varsayılan yolu (RP3/S) Arkin'in telefonda gördüğüdür; `run` metni `fillText` ile çizer (bozuk tasarım süzgeci buna bağlanır, §G8). Bütün adaylar aynı yolla çekilir. `ref` (deterministik) `--paint ref` ile seçilebilir |
| Tasarım seçimi | `?look=<ad>` (T3; kabuk tanımıyorsa yok sayılır), `?face=<ad>` | düzenek parametresi; adaylar `designs.json`'da |
| Dil, tema | `tr`/`en`, `light`/`dark` | dört birleşim; kuru koşu `tr`+`light` |
| Hareket | `?anim=0`; M21 ara kareleri `window.qp.freeze(p)` ile | durağan ve tekrarlanabilir |
| Duyarlılık | `all` (Ayarlar seçicisiyle) | akış kuralı M19'un konusudur; M16 yüzü ölçer |
| İzleme listesi | durumun gerektirdiği yerde `?watch=<sembol>` | varsayılan akışın 40 kapsül sınırı az örnekli/karnesizleri dışarıda bırakıyor (kuru koşuda bulundu); bütün adaylara aynı |
| Gösterim iskeleti | `#controls`, `#banner`, `#toast` gizlenir | oynatma düğmeleri ve "sentetik" bandı ürün değil; her adaya aynı |

Her görüntü için `<kod>.json` kaydı: hedef kapsül, görünen kapsüller, **motorun verisi**
(sayfanın aldığı SSE çerçevelerinden: okurun akış öğeleri `CapsuleItem`, yüz sahneleri
`SceneDesc` katman adları, isabet kutuları ve renkleriyle, oynatma anı), DOM metin öğeleri
(renk, zemin, saydamlık, boy), dokunma hedefleri (DOM kutuları), açık derinlik, gereken
dokunma sayısı. Sayfa değişmez: düzenek (1) sunulan `pkg/qp_wasm.js`'in sonuna çekirdeğe
başvuruyu tutan iki satırlık bir kanca ekler (yalnız okumak için), (2) `EventSource`'u
mesajları kopyalayan bir sarmalayıcıyla değiştirir (çerçeveler değişmeden geçer).

Aynı içerik: durum kümesi ve yeniden oynatma adımı her adayda aynıdır; aynı kapsül hedeflenir
(`candidates` listesinde sayfanın listelediği ilk kapsül; adaylar arasında farklı çıkarsa
kayıtta görünür, puanlama hedefin kendi verisinden anahtar üretir).

## §G2 Durum kümesi (sentetik, sabit)

`glance.py probe` her aday adımda (1, 20, 40, …, 1260) sayfanın akışını ve öğelerini kaydeder
(`arkin` ve `visitor`); `glance.py states` kuralları uygular. S* = en çok kuralı sağlayan
adım (eşitlikte en erken); S*'ta bulunamayan kural en erken sağlayan adımdan; okurun
akışında hiç yoksa motorun tam listesinden (`qp replay --steps N`) ve `?watch=` ile.

| Durum | Kural (`CapsuleItem` üzerinde) |
|---|---|
| `live_event` | olay (`time=event`), kırılım ya da dönüş, `valid`, karne `n ≥ 20`, son geçiş ≤ 2 işlem günü |
| `no_longer` | `validity = no_longer` (hikâye açık) |
| `closed_realized` | `stage = realized` |
| `closed_failed` | `stage = invalidated` |
| `few_cases` | `valid`, `0 < n < 20` (ürünün "az örnek" eşiği `GLYPH_MIN_LEDGER_N`) |
| `no_record` | `valid`, `n = 0` |
| `state_live` | durum (`time=state`), `valid`, `n ≥ 20` |
| `visitor` | yetkisiz okuyucu (`?principal=visitor`), S*'ta geçerli ilk kapsül; rakamsız (K33) |
| `feed` | S*'ta akış ekranı (kaydırmasız) |
| M18 `chart5/50/200` | `qp replay --days {260,1000,2600}` ile kapsül sayısı hedefe en yakın (≥) enstrüman (`states --charts`); grafik derinliği açılır, `wheel_out` çentikle uzaklaşılır |
| M20 `evidence` | `live_event` ve `visitor` kapsülü kanıt derinliğinde |
| M21 `motion` | `live_event` ve `closed_failed` kapsülünün akıştan açılması; ara kareler p = 0,25 / 0,5 / 0,75 |

Kuru koşuda bulunan küme (S* = 1): `live_event` SYN06 kırılım yukarı; `no_longer` SYN28 (adım
40); `closed_realized` SYN21; `closed_failed` SYN26; `few_cases` SYN04 akış (adım 80,
`watch`); `no_record` SYN09 (adım 40, `watch`); `state_live` ve `visitor` SYN28 devam aşağı.
Gerçek veri kullanılmaz (`fixtures/local/` yok sayılır).

## §G3 Kısma sınaması

Aynı ekranın Gauss bulanık (**σ = 4,5 CSS px**, Ek G1-1) ve 0,5× küçültülmüş (Lanczos; 201×437)
kopyası. Gerekçe: RESEARCH U3 σ ≈ 2–3 px önerir; ama ajan okuyucu σ 2,5'te başlık cümlesini
harfi harfine okudu (benzerlik 0,99) — kısma kopyası metni okutursa hiyerarşi değil OCR ölçülür.
σ, başvuru yüzlerinde (`vadrel`, `fotolay`) ana cümlenin okunamadığı (benzerlik < 0,5) en küçük
denenen değerdir (`calibration/sigma.json`: 2,5 → 0,99/0,69; 3,5 → 0,88/0,54; 4,5 → 0,18/0,42);
işaret ve blok yapısı seçilir kalır — "uzaktan bir anlık bakış". **Bu kopya birincil ölçüdür**
(§G9). Ayrıca yarım boy kopya (`face_half`, 0,5×, bulanıksız; RESEARCH F10) okuma kümesiyle
sorulur ve bilgi olarak raporlanır.
Sorular (`throttle`): `t_first` ilk göze çarpan öğe (işaret = 1, cümle = 0,5 puan, öteki 0;
K7: işaret en hızlı okunan öğe olmalı), `t_dir` yön, `t_valid` sürüyor mu / bitmiş mi.
Anahtar motor verisinden. Okuyucuya "görüntü bilerek bulanık ve küçük" denir.

Ayrıca **gri kopya** (`gray`, BT.709 ışıklığı): `dir`, `valid` soruları. "Renk tek başına
anlam taşımaz" kontrolü: gri kopyadaki doğruluk renkli kopyadakinden (aynı sorular) aralık
dışında düşükse bulgu yazılır (sert eşik değil; §G7.4 ΔE kapısı renk tarafını ölçer).

## §G4 Kör okuyucu, sorular ve cevap anahtarı

**Koşturucu** (`gl/readers.py`): `claude -p --model <m> --tools Read --allowedTools Read
--restricted --strict-mcp-config --permission-mode dontAsk --no-session-persistence
--max-turns 3 --output-format stream-json --verbose`; çalışma dizini her çağrıda yeni geçici
klasör, içinde yalnız rastgele adlı (8 karakter) tek PNG (meta verisiz yeniden kodlanmış).
Bir çağrı = bir görüntü + o görüntünün soru kümesi (maliyet için soruları tek çağrıda sorar).
Seçenek sırası her çağrıda tohumla karıştırılır; "bilmiyorum" hep sondadır (konum yanlılığı).

**Modeller:** okuyucu `haiku` (claude-haiku-5-5): düşük maliyetle tekrar sayısı (≥ 3) ve
daha zayıf görsel çözünürlük tek bakışa daha yakın; ikinci boy `sonnet` alt kümede (yüz
kümesi, `tr`/`light`, 1 tekrar) — boylar arası uyum (aynı soruya aynı cevap oranı, Cohen κ)
raporlanır. Eleştirmen `sonnet` (ikili karşılaştırmada daha tutarlı, Chen 2024).

**Soru kümeleri** (`gl/keys.py`, metinler TR/EN birebir orada):

| Küme | Ekran | Sorular |
|---|---|---|
| `face` | yüz (hedef en üstte) | `dir` yön · `kind` ne tür (6 sade karşılık) · `valid` geçerlilik (5) · `when` ne zaman (4 kova) · `trust` geçmişte ne kadar tuttu (5) · `advice` ne yapmamı söylüyor (doğru: hiçbir şey) |
| `face_visitor` | yetkisiz yüz | `dir`, `kind`, `valid`, `trust` (doğru: "bana gösterilmiyor"), `numbers` rakam görüyor musun (doğru: "hayır; yetkim yok diyor"), `advice` |
| `throttle` | kısma kopyası | `t_first`, `t_dir`, `t_valid` |
| `gray` | gri kopya | `dir`, `valid` |
| `feed` | akış ekranı | `f_live` tam görünenlerden kaçı sürüyor · `f_down` aşağı yönlü var mı |
| `evidence` | kanıt (M20) | `e_rests` neye dayanıyor (kanıt ailesi) · `e_who` kim doğruladı (sunucu / bu cihaz / kimse) · `valid` |
| `evidence_visitor` | yetkisiz kanıt | `numbers`, `e_who` |
| `motion` | önce + ara kare (M21) | `m_same` aynı nesne mi taşınıyor · `m_where` taşınan parçalar hangi harfli bölgede |
| `motion_final` | önce + **son** kare | `m_where` (geçerlilik sınaması, §G6.4) |

**Anahtar** (deterministik; `keys_for`): yön `direction`; tür `kind`; geçerlilik `validity` +
`stage` (realized → gerçekleşti, invalidated → tutmadı, expired/retracted → süresi doldu);
ne zaman = son geçişin günü ile oynatma anı arasındaki **takvim günü ve işlem günü**
kovaları (bugün / dün / 2–5 gün / > 5 gün; ikisi farklıysa ikisi de doğru — R4 Açık sorun
2'nin dersi); güven = doğumdaki karne (`n < 20` → az örnek; oran ≥ 0,6 / 0,4–0,6 / < 0,4;
`redacted` → gösterilmiyor); `numbers` = `redacted`; akış = ekranda **tam** görünen
kapsüllerin öğeleri (her adayın kendi görüntüsünden); `e_rests` = kanıt satırlarının aileleri
(price/volume/flow/news); `e_who` = açık kapsülün güven iddiası (`open().trust`, sunucu
dediyse sunucu); `m_same` = çekirdeğin geçiş karesinde taşınan öğe var mı; `m_where` = p = 0,5
anındaki taşınan parçaların (`#fx` tuvalleri) ağırlık merkezinin çeyreği. Anahtarı boş soru
puanlanmaz (ör. seçeneği olmayan kanıt ailesi).

**Personalar** (birebir, `gl/readers.py` `PERSONAS`; EN karşılıkları orada):

- `F` geleceğin yatırımcısı: "Yirmi iki yaşındasın, birkaç aydır telefondan hisse alıp
  satıyorsun. Teknik analiz bilmiyorsun: grafik, indikatör, mum, direnç gibi sözcükler sana
  bir şey ifade etmiyor. Telefonuna bakıyorsun ve ekrana yalnızca beş saniye bakacaksın; uzun
  uzun okumazsın, ilk bakışta gözüne çarpanla cevap verirsin."
- `H` alışkın kullanıcı: "Kırk beş yaşındasın, yirmi yıldır borsada işlem yapıyorsun; terminal
  ekranlarına, mum grafiklerine ve indikatörlere alışkınsın, ayrıntıya güvenirsin.
  Telefonunda bir piyasa uygulamasına kısa bir bakış atıyorsun."

"Uygulama tavsiye vermez" okuyucuya **söylenmez** (`advice` bunu ölçer).

## §G5 İkili kör karşılaştırma (eleştirmen)

Aynı durum, iki tasarım, tek görüntüde yan yana (her biri 1×, arada 24 px, üstte "A" / "B";
toplam 828 px < 1568). Her çift **iki sırayla** ve **≥ 3 tekrarla** (farklı tohum; U1, U11).
İstem (`CRITIC`, TR/EN): önce ölçüte bakmadan **tek cümlelik kendi gözlemi** (U6), sonra yedi
önceden yazılı ölçütle "A / B / ayırt edemiyorum" ve bir cümle neden; "daha çok yazı daha iyi
demek değildir" açıkça yazılı (U2). Eleştirmene okuyucu cevapları gösterilmez. Etiketler
yalnız A/B; tasarım adı, kaç aday olduğu, hangisinin eski olduğu söylenmez.

Birleştirme (`gl/stats.py`): bir (çift, durum, tekrar) için iki sırada **aynı kazanan =
galibiyet**, bölünme ya da herhangi bir sırada "ayırt edemiyorum" = beraberlik (½–½).
**Çelişki oranı** çift başına raporlanır; > %30 ise o çift "ayırt edilemedi". Sonuçlar
**Bradley–Terry** (MM algoritması, beraberlik yarım galibiyet, ortalamaya karşı 0,5'lik sanal
beraberlik önseli) ile birleştirilir; **%95 önyükleme aralığı** (2000 yeniden örnekleme,
tohum `20261117 + 999`). Uzunluk yanlılığı (U2): BT gücü ile ekran karakter sayısının
Spearman korelasyonu ve kesin sonuçlarda "daha çok metinli olan kazandı" payı raporlanır;
pay > 0,7 ve n ≥ 10 ise eleştirmen sonucu "uzunluk yanlılığı şüpheli" diye işaretlenir.
Tercih (BT) ile anlama doğruluğu **ayrı** raporlanır; seçimde anlama önce gelir (U9).

## §G6 Puanlama

1. Soru puanı 1/0 (anahtardaki kodlardan biri mi; `t_first` ağırlıklı: işaret 1, cümle 0,5).
   "Bilmiyorum" ve harfsiz cevap 0. Dil modeli puanlamaz (U6).
2. Birim = (görüntü, persona, tekrar, model); birim puanı = sorularının ortalaması.
3. **Şans düzeyi** soru başına 1/k (k = "bilmiyorum" hariç seçenek sayısı): `dir` 0,333 ·
   `kind` 0,167 · `valid` 0,2 · `when` 0,25 · `trust` 0,2 · `advice` 0,25 · `numbers` 0,333 ·
   `t_first` 0,25 (ağırlıklı) · `t_dir` 0,333 · `t_valid` 0,5 · `f_live` 0,25 · `f_down` 0,5 ·
   `e_rests` 0,2 · `e_who` 0,333 · `m_same` 0,5 · `m_where` 0,25. `face` kümesinin birim şansı
   ≈ 0,233. Soru başına oran + Wilson %95 aralığı raporlanır.
4. **M21 geçerlilik sınaması** (U10): `m_where` sorusu ara kare **p = 0,25** ile sorulur ve
   yalnız p = 0,25'teki çeyreği son konumun (p = 1,0) çeyreğinden farklı (ve çizgilere %10'dan
   uzak) olan durumlarda puanlanır (Ek G1-3); aynı soru son kareyle (`motion_final`) de sorulur:
   orada doğruluk şans düzeyinin (0,25) Wilson aralığı içinde olmalıdır; değilse soru geçişi
   ölçmüyor demektir, M21 sonucu "geçersiz soru" diye yazılır ve soru değiştirilir.

## §G7 Hesapla ölçülenler (tek komut: `glance.py compute`; `computed.json` + `computed.md`)

Kaynak: yüz sahneleri (`SceneDesc`, motorun gönderdiği; öğe düzeyinde) + DOM metinleri.
Piksele inilmez (yalnız bozuk süzgeç ve kısma kopyası piksel işidir).

1. **Metin karşıtlığı (WCAG 1.4.3)**, öğe başına: metin rengi (alfa ile) altındaki zemin
   üstüne bindirilir; zemin = sahne arka planı + boyama sırasında metinden önce gelip metnin
   çapa noktasını içeren her dolgu. Eşik 4,5:1, büyük metin (≥ 24 px ya da ≥ 18,66 px kalın)
   3:1. DOM metninde saydamlık zincirlenir. Ölçü: en kötü öğe (oran/eşik).
2. **Metin dışı karşıtlık (1.4.11)**: işaret katmanının (`glyph`) gövdesi (en opak ilkel)
   zemine karşı ≥ 3:1, **kapanmış işaret dahil** (A31). En kötü işaret.
3. **En küçük yazı** ≥ 11 CSS px (sahne `px` ve DOM `font-size`).
4. **Anlam renkleri ayrımı:** her tasarımın görüntülerinden işaret gövde rengi anlam sınıfına
   göre toplanır (açık: geçerlilik × yön; kapanmış: gerçekleşti / tutmadı / süresi doldu). Her
   sınıf çifti için ΔE00 beş koşulda: normal, gri tonlama, protan, deutan (Viénot, Brettel &
   Mollon 1999), tritan (Brettel, Viénot & Mollon 1997; DaltonLens yarı düzlem kuralı). Ayrıca
   Machado 2009 (şiddet 1,0) — CVD koşulunda iki yöntemin **küçüğü** alınır (RESEARCH §16:
   Machado tritanda zayıf; görev metni Machado'yu istiyordu, ikisi de koşulur, kapı tutucu).
   **Eşik ΔE00 ≥ 10**, işaret ≤ 16 CSS px (12 pt) ise ≥ 15 (Szafir 2017, RESEARCH F6). Yalnız
   **renkle kodlanmış** çiftlere uygulanır (normal görüşte ΔE00 ≥ 5); aynı renkte çizilen çiftin
   anlamı biçimle taşınmalıdır ve gri okuyucu sorusuyla denetlenir (Ek G1-4). İşaret rengi =
   işaret katmanının baskın görünür rengi (alfa ağırlıklı alan; zemine karışan ilkeller atlanır).
   Gri ışıklık oranı (çift başına, F7 ≥ 1,5) bilgi sütunudur.
5. **Dokunma hedefi** ≥ 44 CSS px (= pt; Apple HIG): düğmeler, alanlar, yüzler, açık sahne
   (DOM kutuları) ve yüz sahnelerindeki işaret isabet kutuları. Grafik derinliğindeki işaret
   isabet kutuları sahne tarifi kayda girdiğinde sayılır (açık risk, §G11).
6. **K52 bütçesi** (`k52.mjs`, `glance.py k52`): `calibrate.mjs` `runCalibrated` ile
   `phone-jit-est` (HTTPS, JIT açık tahmini; K52 eşikleri: tik p95 ≤ 8 ms, yüz ve sahne boyama
   p95 ≤ 8 ms, ilk kapsül ≤ 1,5 s, kaydırmada uzun kare ≤ %1 → vekil: kare p99 ≤ 33,4 ms) ve
   `phone` (kalibre, JIT kapalı; `PHONE_BIAS` uygulanır; kayıt, kapı değil). Tasarım
   parametresi, kabuk dizininin sembolik bağlı bir kopyasında `look-shim.js` ile adrese eklenir
   (depo değişmez). 3 tekrarın ortancası.
7. **Hareketi azalt:** aynı son ekranın `anim=1` ve `anim=0` kayıtlarında metin öğeleri +
   işaretler kümesi farkı (`reduced_motion_loss`) = 0 olmalı (M21 alternatifleri gelince koşulur).
8. **Metin yoğunluğu:** yüz başına karakter ve satır (sahne metin ilkelleri), ekran başına
   karakter (yüzler + DOM). Sert eşik değil; U2 için ortak değişkendir.
9. **Paket boyutu:** `scripts/demo.sh` `pkg/sizes.json` + yazı tipi dosyaları (aday dalında
   derlenir; `bundle/<ad>.json` olarak `compute`'a girer).

Sert eşikler: 1, 2, 3, 4, 5, 6 (JIT tahmini). Hepsi `computed.json`'da makine okunur.

## §G8 Bozuk tasarım sınaması (düzeneğin kendi sınaması)

Bozuk tasarım yarışa gizli bir adla (kuru koşuda `kestim`; ad yalnız anahtar dosyasında)
girer. İçerik: **hiyerarşi ters** (her metin boyu `s → 33 − s` CSS px, 9–30 aralığında: ek
bilgi büyük, cümle küçük), **karşıtlık düşük** (metin rengi %65 zemine çekilir), **işaret
minik ve soluk** (işaret ve grafik dahil, zemin dışı her dolgu/çizgi %30 opaklık), DOM
başlık/katman metinleri %45 opaklık. T3'ün `?look=` kaydı gelene kadar bu, düzenekte bir
Canvas 2D süzgecidir (`shoot.mjs` `brokenInit`, `canvas=run` yolu; temel yüz `vadrel`).
`?look=` gelince aynı tarif bir görünüş kaydı olarak kurulur (büyük ek bilgi, küçük cümle,
minik soluk işaret, düşük karşıtlık) ve süzgeç yerine o kullanılır.

**Kabul kuralı** (`glance.py broken-check`): düzenek geçerlidir ancak ve ancak bozuk tasarım
(a) birincil ölçüde (kısma kopyasında tek bakış doğruluğu, nokta tahmini) **kesin son** (başka
bir adayla eşitlik son sayılmaz), (b) eleştirmen BT'de **kesin son** ise; ayrıca birincil
aralığının en iyinin aralığının altında kalması raporlanır. Tam boy okumada son olması bilgi
olarak yazılır (kuru koşuda değildi: Ek G1-2). Bozuk tasarım sert eşiklerde elenir (beklenen) ama
sınama eşiklerden **önce** okuyucu ve eleştirmen sıralamasıyla yapılır — kapı onu zaten
elerdi; sınanan, ajan ölçümünün kendisidir. Geçmezse düzenek düzeltilir ve sınama yeniden
koşulur; düzeltme bu dosyaya ek olarak yazılır.

## §G9 Seçim kuralı (önceden yazılı; `gl/select.py`)

1. **Sert eşikler** (§G7: karşıtlık 1.4.3 ve 1.4.11, en küçük yazı, dokunma, renk ayrımı,
   K52). Geçemeyen elenir. Bir eşiği **hiçbir aday** (başvuru `r4` dahil) geçemiyorsa o eşik
   uygulanmaz ve yazılır (R4 E6 dersi). Bozuk tasarım seçime girmez.
1b. **Okuma aşağı kalmama kapısı** (Ek G1-2): tam boy okuma doğruluğunun (`face` +
   `face_visitor`, F2:H1) üst sınırı başvurunun (`r4`) nokta tahmininin altında kalan aday elenir
   (metnin söylediğini açıkça kaybediyor).
2. **Birincil: tek bakış okuyucu doğruluğu = kısma kopyası** (`throttle`, §G3; metin okunmaz,
   anlam görsel hiyerarşiyle taşınmalı), **F 2 : H 1** ağırlıklı; %95 önyükleme aralığı (F ve H
   birimleri ayrı yeniden örneklenir). Gerekçe: tek bakış yüzü F içindir (beş saniye, teknik
   analiz yok); H ayrıntının kaybolmadığını denetler; R4 ile süreklilik. En iyinin aralığıyla
   örtüşen herkes "ilk grup".
3. **İkincil** (ilk grupta birden çok aday varsa): eleştirmen BT; en iyi BT aralığıyla
   örtüşenler kalır. Çelişki oranı > %30 olan çiftler ayırt edilemedi sayılır.
4. **Tam boy okuma doğruluğu** (aynı yöntemle) — hâlâ birden çoksa.
5. Sonunda tek aday kalırsa "kazanan"; kalmazsa raporda **"ayırt edilemedi"** yazılır ve
   kazanan ilan edilmez. **Bütünleştirmeye girecek varsayılan** için ayrı, açık bağ çözme
   kuralı: nokta tahminleri sırasıyla birincil (kısma) → BT → tam boy okuma → daha az ekran
   metni → ad. İlk
   iki (bu sıraya göre) gösterimde seçilebilir kalır.

Başvuru `r4` her ölçümde yarışır ve kazanabilir (kazanırsa varsayılan değişmez).

**M18, M20, M21 görevleri** (aynı yöntem; birincil ölçü konu başına):

| Konu | Birincil | Ek ölçüler |
|---|---|---|
| M18 hikâye çizelgesi | `chart` kümesi doğruluğu (5/50/200): "son üç ayda kaç olay", "kaçı tuttu" (kova), "şu an canlı olan hangi harfli bölgede" (anahtar: canlı kapsülün doğum barının görünen aralıktaki konumu), "şu işaretin kanıtı ne" | dokunma sayısı (düzeneğin akıştan hedef derinliğe en kısa yolu, `tapTo`); 200 kapsülde K52; kapanmışlar görünürken canlının bulunması (doğruluk; insan testinde süre) |
| M20 kanıt ekranı | `evidence` + `evidence_visitor` doğruluğu | dokunma sayısı; yetkisizde rakam yok (DOM ve sahne metninde rakam taraması = 0) |
| M21 hareket | `motion` doğruluğu (yalnız ara kare + önce) | §G6.4 geçerlilik sınaması; K52; hareketi azalt bilgi kaybı = 0 (sert) |

M18 soru kümesi (`chart`) M18 alternatiflerinin ortak protokol eki (hikâye listesi) gelince
`keys.py`'ye eklenir; anahtar o listeden (kimlik, doğum/kapanış barı, sonuç) türetilir.
Bu şartnamenin kuralları o eklemeyle değişmez.

## §G10 Körlük kaydı

- `key.json`: görüntü kodu (tohumlu SHA-256, 10 hane) → tasarım, durum, dil, tema. Okuyucu ve
  eleştirmen dizinine **hiç** kopyalanmaz; `critic_key.json`: çift kodu → A/B tasarımları.
- Her okuyucu çağrısı `readers/<kod>.json`: istem (birebir), görüntü kimliği, geçici dosya adı,
  seçenek harf haritası, ham sonuç, kesilmiş cevaplar, araç çağrıları, tur sayısı, maliyet,
  kullanılan model(ler), `one_look`. Eleştirmen `critics/<kod>.json` aynı alanlarla.
- Her istem gönderilmeden `check_prompt` yasak sözcük denetiminden geçer (tasarım adları
  `key.json`'dan + sabit liste: r4, r5, eski, yeni tasarım, variant, alternatif, başvuru,
  bozuk, broken, look=, face=, bilinen yüz/akış/geçiş adları); bulunursa çağrı yapılmaz.
- Okuyucunun gördüğü tek şey rastgele adlı PNG ve istemdir; kayıtlar bunu gösterir.
- Ham görüntüler ve kayıtlar depoya girmez (`raw/` izlenmez ya da scratchpad); depoya yalnız
  özet tablolar ve en çok birkaç küçük örnek girer.

## §G11 Açık riskler

1. Ajan okuyucu küçük metni okuyabiliyor; 402 px'te bile metin ağırlıklı tasarım avantajlı
   olabilir. Karşı önlem kısma sınaması ve uzunluk yanlılığı raporu; yine de insan testi şart.
2. Tek model ailesi (U4); ikinci aile yok.
3. `canvas` yolu bayt bayt deterministik değildir (tarayıcı taraması); aynı makinede tekrar
   bayt eşitliği kuru koşuda ölçüldü (README). `--paint ref` deterministik seçenek.
4. Kaydırmada uzun kare payı çekirdek özetinde yok; p99 vekili kullanılıyor.
5. Grafik derinliğindeki işaret isabet kutuları kayda henüz girmiyor (yalnız yüzlerinki).
6. Yetkisiz okuyucuya yüz sahnesi gönderilmiyor (görünüm kuralı); yetkisiz yüzün renk/işaret
   ölçüleri yalnız DOM'dan.
7. Sentetik dünyada `server` güvenli kapsül yok; `e_who` anahtarı açık kapsülün güven
   iddiasından gelir, sınıf çeşitliliği az.
8. `?watch=` okurun izleme listesini yazar; yetkili okurun belgesinde kalıcıdır (düzenek
   adımları sıralar). Akışın görünüşünü (grup başlığı) değiştirebilir; her adaya aynıdır.

## Ek G1 — kuru koşudan düzeltmeler (2026-11-18; hiçbir alternatif yokken, yalnız başvuru yüzleri ve bozuk süzgeçle)

Kanıt ve sayılar: `DRYRUN.md`, `calibration/`, `dryrun/`.

1. **Kısma σ 2,5 → 4,5.** σ 2,5'te ajan cümleyi okudu (0,99); σ 4,5'te okuyamadı (0,18 / 0,42).
2. **Birincil ölçü değişti.** Yazıldığı hâliyle birincil (tam boy okuma doğruluğu) bozuk tasarımı
   son sıraya koyamadı (bozuk 0,882 — `vadrel` 0,893, `fotolay` 0,857); yarım boyda ve hafif
   bulanıkta da (σ 1–2 px) ajan soluk metni okudu. §G8'e göre düzenek düzeltildi: birincil =
   kısma kopyasında tek bakış doğruluğu; tam boy okuma = aşağı kalmama kapısı (adım 1b) ve
   4. sıra bağ çözücü. Düzeltilmiş kuralla bozuk tasarım birincilde ve BT'de son.
3. **M21 `m_where`** p = 0,5 yerine p = 0,25 ve son çeyrekten farklı olma şartı (bugünkü geçişte son
   kareyle doğruluk 1,0 çıktı; soru geçişi ölçmüyordu).
4. **Renk kapısı** yalnız renkle kodlanmış çiftlere; işaret rengi baskın görünür renk.
5. **Çekim sırası:** `?watch=` okurun belgesinde kalıcı; izleme listesiz çekimler önce.
6. **Tek bakış dışı çağrı** (birden çok `Read`) puanlamadan çıkarılır.
7. **Bozuk tasarım kabul kuralında "son" kesin sondur** (ilk uygulamada eşitlik son sayılıyordu).
