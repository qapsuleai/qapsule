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
| Kapsül dili (Ek G4-1) | `?voice=<ad>` (`--voice`; verilmezse ürünün gösterim varsayılanı) | bütün adaylar aynı metinle; kayıt `voice` {istenen, çekirdeğin gönderdiği} |
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
   "Bilmiyorum" ve harfsiz cevap 0. Dil modeli puanlamaz (U6). `t_valid` sınıf dengeli ağırlıkla (Ek G3).
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
   zemine karşı ≥ 3:1, **kapanmış işaret dahil** (A31). En kötü işaret. (Ek G4-3: uygulama bu
   tanıma uyduruldu; eşitlikte en büyük alanlı renk.)
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
   **Ek G4-3:** işaret rengi = gövde rengi (madde 2'deki en opak ilkel); alfa ağırlıklı alanın
   baskın rengi gövdeden farklıysa "metin dışı yardımcı" olarak raporlanır, kapıya girmez.
   **Ek G4-2:** çiftler yalnız aynı tema içinde kurulur; kapı her temaya ayrı uygulanır.
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
   **Roller (Ek G2):** her yüz metninin yazı rolü, tasarımın görünüş kaydından (`qp look show`,
   `looks/<ad>.json`) boyla eşlenir (T3 `mark_role` kuralı); `computed.json` `roles` rol başına
   boy ve en kötü karşıtlığı verir. Gösterimin sahnesi rol taşımaz (`qp scene --roles` yalnız
   oturum görünümlerinde); kabuk renkleri `core.css()` değişkenleri olarak DOM'a yazılır ve
   düzenek onları hesaplanmış biçemden (`getComputedStyle`) okur.
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

**Görünüş kaydı (Ek G2; T3 sonrası geçerli yol):** `broken/look.patch` `look.rs`'e `KESTIM`
kaydını ekler: yazı rolleri ters (başlık cümlesi `headline` 11 px; üst satır/ek bilgi `title`
24 px, karne/etiket `label` 22 px; `body` 13, `figure` 12, `note` 10), iki temada metin, işaret
ve anlam renkleri (`text`, `muted`, `axis`, `up`, `down`, `accent`/`s1`, `warn`, `neutral`,
`valid`, `invalid`, `closed`, `focus`) ve kabuk renkleri (`fg`, `muted`, `accent`, `note_fg`,
`alert_fg`, `req`) %65 zemine çekilir, işaret boyu yüzde 12 px (kartta 14, grafikte 12).
**Ürüne girmez:** `broken/build.sh <dış klasör>` HEAD'in `git archive` kopyasına yamayı uygular,
`qp` ve WASM paketini o kopyada derler; çalışma ağacı ve dallar değişmez, Ayarlar'da yalnız o
geçici derlemede görünür. O koşudaki **bütün adaylar aynı derlemeyle** çekilir (`QP_BIN`,
`QP_WEB`); başvuru yüzleri `r4` görünüşüyle `main` ile aynı çizilir. `compute` bozuk değerleri
artık sahne tarifinden okur. Süzgeç (`"broken": true`) yalnız yedek olarak kalır.

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

## Ek G2 — bozuk tasarım görünüş kaydı olarak (2026-11-18)

Metni §G8 ("Görünüş kaydı") ve §G7.8 ("Roller") içinde; sonuç `BROKEN.md`.

## Ek G3 — cevap yanlılığı, tekrar sayısı, bozuk tasarımın kalıcılığı (2026-11-18; orkestratör kararı, M16 alternatiflerinden önce)

Gerekçe: `BROKEN.md` "Açık riskler 1–2". `t_valid` ikili bir sorudur ve durum kümesinde sınıflar
dengesizdir: 8 yüz durumunun 5'i açık, 3'ü kapanmış ya da artık geçerli değil. Hep "bitmiş" diyen
okuyucu kapanmışların payı kadar, hep "sürüyor" diyen okuyucu açıkların payı kadar puan alıyordu.
3 tekrarla bozuk tasarım `fotolay`'dan eşleştirilmiş farkla ayrılmıyordu.

1. **Sınıf dengeli puan** (`gl/select.py`: `BALANCED`, `balance_weights`, `unit_means`):
   - `t_valid` sorusunda her anahtar sınıfı toplamda eşit ağırlık alır. Ağırlık = toplam /
     (sınıf sayısı × sınıfın satır sayısı); paylar sorunun bütün satırlarından hesaplanır (durum
     kümesi her adayda aynı).
   - Birim puanı ağırlıklı ortalamadır; tek cevaba saplanan okuyucu bu soruda 1/2 alır.
   - Öteki sorular ağırlıksızdır.
   - Birincil ölçü, eşleştirilmiş fark ve bütün aralıklar bu birim puanından hesaplanır.
   - Durum kümesi değişmedi: açık/kapalı sayısını eşitlemek kümeyi ve "aynı içerik" kuralını
     bozardı; ağırlık eşdeğer düzeltmedir.
2. **Birincil ölçüde 6 tekrar varsayılan:** `glance.py read --reps 0` küme varsayılanını kullanır
   (`throttle` 6, öteki kümeler 3). README maliyet tablosu buna göre güncellendi.
3. **Bozuk tasarım her M koşusunda yarışır** (M16, M18, M20, M21):
   - `broken/look.patch` ile o koşunun derlemesine eklenir; gizli ad yalnız anahtar dosyasındadır.
   - §G8 kabul kuralı **her koşuda** uygulanır. Geçmeyen koşunun sonucu kullanılmaz, düzenek
     düzeltilir ve ek yazılır.
   - Eşleştirilmiş farkın ayrıklığı (`paired_vs_broken`) her koşuda bilgi olarak raporlanır.
4. **Yeniden puanlama** (yeni çağrı yok; `BROKEN.md` "Ek G3 sonrası"):
   - Bozuk tasarım birincilde hâlâ kesin son, BT'de son → kabul kuralı geçiyor.
   - Ama `fotolay` ile eşleştirilmiş fark artık ayrık değil: +0,015 [−0,010, +0,041].

## Ek G4 — M16 ölçümünden önce; uygulama hataları (2026-11-18)

Üçü de kural değişikliği değil: uygulama SPEC'in yazılı cümlelerine uyduruldu. Bulan: bir aday
ajanın raporu (M16 alternatifi). Testler `tests/test_g4.py`.

1. **Kapsül dili (`voice`) aktarılmıyordu.** §G1 "Aynı içerik: ... her adayda aynıdır" der; düzenek
   `?voice=` geçiremiyordu, okunan metin ürünün o anki varsayılanına bağlı kalıyordu. Şimdi
   `probe`, `chartstates`, `shoot`, `k52` (ve `k52chart.mjs`) `--voice <ad>` alır; verilmezse
   adrese parametre konmaz (ürünün gösterim varsayılanı). Bütün adaylara aynı dil. Kayıt:
   `<kod>.json` `voice` = {`requested`: istenen ya da null, `used`: çekirdeğin hedef kapsülle
   gönderdiği dil (`CapsuleItem.voice.voice`; yoksa şemanın varsayılanı `r4`)}; `key.json` ve K52
   çıktısı da `voice` yazar. Deneme (bu dalın derlemesi, sentetik gösterim, `live_event` yüzü):
   parametresiz → `used: marla`; `--voice marla` → `marla`; `--voice r4` → `r4`. **M16 `--voice marla`
   ile ölçülür.**
2. **Anlam rengi tablosu temaları karıştırıyordu.** §G7.4 "her tasarımın görüntülerinden işaret gövde
   rengi anlam sınıfına göre toplanır" — ayrım bir ekranın içindeki ayrımdır, ekran tek temadadır.
   Uygulama sınıf başına ilk görülen kaydı alıyordu (tema fark etmeksizin), çiftler açık tema
   rengini koyu tema rengiyle karşılaştırabiliyordu; M18 grafik tablosunda (`chart_gates`) aynı rol
   iki temanın renkleri olarak kendisiyle eşleşiyordu (`up ↔ up`, `down ↔ down`). Şimdi çiftler
   yalnız aynı tema içinde kurulur (`pairs[].theme`), kapı her tema için ayrı uygulanır
   (`colour.themes.<tema>.pass`); tasarım ancak her temada geçerse geçer.
3. **İşaret rengi gövde değildi.** §G7.2 gövdeyi "en opak ilkel" diye tanımlar; uygulama işaret
   katmanının alfa ağırlıklı en büyük alanlı rengini alıyordu (§G7.4'ün son cümlesi, Ek G1-4),
   böylece büyük, saydam bir güven plakası gövdenin yerine geçebiliyordu (adayın raporunda
   ≈ 1,3:1). Şimdi gövde = işaret katmanındaki en yüksek alfalı dolgu/çizgi ilkeli (`fill`,
   `polyline`, `rect`, `circle`); zemine karışan ilkel (ΔE00 < 2, ör. zemin renginde delik) yine
   atlanır; aynı alfada birden çok renk varsa o ilkellerin toplam alanı en büyük olan renk.
   1.4.11 kapısı ve §G7.4 anlam rengi gövdeden. Alfa ağırlıklı alanın baskın rengi gövdeden
   farklıysa `aux` olarak, `nontext_contrast.worst_aux` "metin dışı yardımcı" diye raporlanır;
   hiçbir kapıya girmez. §G7.4'ün son cümlesi ve Ek G1-4'ün ikinci yarısı bu maddeyle §G7.4'ün
   ilk cümlesine ("işaret gövde rengi") ve §G7.2 tanımına bağlanır.

**Kayıtlı sonuçlara etkisi** (yeni çağrı yok; `compute` kayıtlı sahne verisinden yeniden koşuldu):

- *Bozuk tasarım koşusu* (`broken/run`; run2 çekim kayıtları; `computed.json`/`computed.md`
  güncellendi): işaret karşıtlığı en kötü `vadrel` 1,27 → 2,06, `fotolay` 1,27 → 2,06, `kestim`
  1,07 → 1,26 (eski değerler şimdi `worst_aux`); üçü de < 3, kapı sonucu değişmedi. Renk: eski
  karışık tablo 15 çift (başarısız `fotolay` 8, `kestim` 7, `vadrel` 4); yeni tema başına 15'er
  çift, başarısız açık/koyu `fotolay` 3/2, `kestim` 10/11, `vadrel` 3/2 — hiçbiri iki temada da
  geçmiyor (yalnız tema ayrımıyla, eski renk kuralında: 5/5, 13/14, 5/5). Bütün tasarımlar sert
  eşiklerde önce de sonra da kalıyor; `score` aynı verilerle yeniden koşuldu: sayılar ve karar
  aynı (`scores.md` yalnız sonradan değişen etiket metninde farklı; dosyaya dokunulmadı).
- *M18* (`M18/run`): grafik çekim kayıtları (sahne boyama listeleri) saklanmamış; tam yeniden
  hesap yapılamadı. Renk kapısı kayıtlı `computed.json`'daki boyanan roller ve görünüş kayıtları
  (`qp look show r4`; bozuk için `broken/run/look-kestim.json`) ile yeniden kuruldu (eski karışık
  tablo bu yolla birebir yeniden üretildi). Tema başına başarısız çift açık/koyu: `korda`, `pavka`,
  `selen` 5/6 (eski 45 çiftte 23), `r4` 1/2 (eski 15 çiftte 7), `kestim` 3/3 (eski 15 çiftte 6).
  Hiçbir tasarım geçmiyor; M18 sonucu ve kararı değişmez. Gövde düzeltmesi grafik yolunu etkilemez
  (grafik kapısı işaret katmanını değil rol renkli ilkelleri ölçer).

## Ek G5 — M16 ölçümünden önce; anlam rengi kapısı rollerden, K52 JIT'siz madde (2026-11-18)

Tetikleyen: bir aday ajanın raporu. Ek G4-3'ten sonra işaret rengi "en opak çizgi" oldu; `siluet`
alfabesi gövde kenarını metin renginde (`text`) çizdiği için yönlü ve yönsüz işaretler aynı
"mürekkep" rengine ölçülüyor, §G7.4 tablosu işaretin dış çizgisini karşılaştırıyordu, anlam rengini
değil. GOAL'ın ölçütü ("anlam renkleri gri tonlamada ve renk körlüğü benzetimlerinde ayrılıyor") ve
VERIFY 7 ("ayrım eşiğin üstünde; biçimle de okunuyor") renk rolünü ölçmeyi ister. Testler
`tests/test_g5.py`; yeniden puanlama `g5_rescore.py`.

1. **Renk kapısı (§G7.4) görünüş kaydının anlam rengi ROLLERİNDEN.** Tema başına (`qp look show
   <ad>`, `looks/<ad>.json`) roller: `up`, `down`, `neutral`, `valid`, `invalid`, `closed` ve
   kaydın işaret için kullandığı roller (çekim kayıtlarındaki işaret katmanı ilkellerinin RGB'si
   kaydın bir rolüne eşitse o rol; zemin, yüzey, ızgara, çizgi, eksen, kabuk ve dizi rolleri
   `s1…s8` sayılmaz; ör. `siluet` kenarı ve `tg5` mürekkebi → `text`). Her rol kendi alfasıyla
   tema zeminine bindirilir. Çiftler aynı tema içinde; ΔE00 beş koşulda (normal, gri, Viénot
   protan/deutan, Brettel tritan; Machado ile küçüğü, §G7.4 gibi). Eşik ≥ 10; yüzdeki en küçük
   işaret ≤ 16 px ise 15 (işaret boyu çekimlerin isabet kutularından; yoksa kaydın `glyph.face_px`).
   **Aynı rengi bilerek paylaşan roller** (normal görüşte ΔE00 < 5, ör. `invalid` = `closed`)
   çift sayılmaz, raporda listelenir (`colour.shared`); o çiftin anlamının biçimle ayrıldığı
   gri kopya okuyucu sorusuyla sınanır (madde 3). Tasarım her temada geçerse geçer. Sahne tabanlı
   eski tablo (Ek G4-3 gövde rengi, anlam sınıfı başına) `colour_scene` olarak bilgi kalır;
   görünüş kaydı olmayan koşuda kapı odur (önceki davranış).
2. **İşaret karşıtlığı kapısı (1.4.11)** değişmedi: Ek G4-3'teki gibi en opak çizgi, tazelik
   dahil sahneden.
3. **"Biçimle de okunur":** gri kopyadaki `dir` ve `valid` doğruluğu (§G3'te var olan gri koşu)
   birincil ölçüden ayrı raporlanır; paylaşılan renk çiftlerinin durumlarında (ör. `no_longer`,
   `closed_failed`) ayrıca durum başına.
4. **K52 JIT'siz "kullanılabilir" maddesi** (K52: "JIT'siz en kötü durumda gösterim kullanılabilir
   kalır, sürüklemede uzun kare ≤ %10"): `k52.mjs` `phone` koşusunda bu madde kayıttı, kapı
   değildi. M18'de grafik için kapı olarak uygulandı (`k52chart.mjs`); yüz ölçümünde (akış
   kaydırması) aynı biçimde: çekirdeğin kendi uzun kare payı `frame_ms.scroll.long` (koşuların
   ortancası) ≤ 0,10. JIT açık tahmini (bağlayıcı) değişmedi (p99 vekili; `scroll_long` kayda girer).

**Kayıtlı sonuçlara etkisi** (yeni çağrı yok; `g5_rescore.py`; çekim kayıtları saklanmadığından
işaretin rolleri sahneden okunamadı: kayıtlı alfabelerin mürekkebi/kenarı `text` olduğu için altı
role `text` eklendi; işaret boyu kayıtlı tablonun `mark_px`'i, M18 grafiği için 16 → eşik 15). Roller
`main`'in `qp look show r4` çıktısından ve `broken/run/look-kestim.json`'dan (renkleri bugünkü
yamalı derlemeyle birebir aynı). Her temada 7 rol → 21 çift; `valid` = `text` ve `invalid` =
`closed` iki temada da bilerek paylaşılıyor (listelendi, sayılmadı) → 19 çift.

- *Bozuk tasarım koşusu* (`broken/run/computed.json` yeniden yazıldı; eski tablo `colour_scene`):
  başarısız çift açık/koyu `vadrel` ve `fotolay` (ikisi de `r4` görünüşü) 4/8 (en kötü koyu
  `down ↔ invalid` 0,50; açık `up ↔ down` 4,66), `kestim` 19/19. Hiçbiri geçmiyor (önce de
  geçmiyordu); bütün tasarımlar sert eşiklerde önce de sonra da kalıyor, `score` ve karar aynı.
- *M18* (`M18/run`; dosyaya dokunulmadı, grafik kapısı ayrı kural): yüz kuralıyla eşik 15'te
  `r4` görünüşü (korda, pavka, selen, r4) açık/koyu 7/13 başarısız, `kestim` 19/19. Hiçbir tasarım
  geçmiyor → E6 ile renk eşiği yine uygulanmaz; M18 sonucu ve kararı değişmez.

## Ek G7 — M20 ölçümünden önce: derinlik, dokunma sayısı, K52 ekranı (2026-11-18)

Tetikleyen: M20 aday ajanlarının raporları (iki aday da M20 sorularının cevabını kanıt
derinliğinde değil açılmış kapsülde veriyor; kalibrasyonun sabit dokunuş noktası tasarıma göre
farklı ekran açıyor). Yazan: `r5/m20-measure`, **hiçbir M20 görüntüsü çekilmeden ve hiçbir okuyucu
çağrısı yapılmadan önce**. Tekrar, persona, kısma kopyası, eleştirmen çiftleri (iki sıra +
tekrar), BT + önyükleme, "ayırt edilemedi" kuralı ve §G9 seçim kuralı aynen geçerlidir.

**(a) İki derinlik; birincil kapsül derinliği.** M20 görüntüleri §G2'nin iki durumunda
(`live_event`, `visitor`; aynı hedef kapsül) iki derinlikte çekilir:
- **kapsül derinliği** (`capsule`): akıştan bir dokunuş — hedef yüze dokunulur, açılmış kapsül;
- **kanıt derinliği** (`evidence`): §G2'deki gibi (`tapTo(..., 'evidence')`).

Soru kümeleri iki derinlikte aynı (`evidence`: `e_rests`, `e_who`, `valid`; `evidence_visitor`:
`numbers`, `e_who`). §G9'un M20 satırındaki birincil ölçü (`evidence` + `evidence_visitor`
doğruluğu, F 2 : H 1, %95 önyükleme) **kapsül derinliğindeki** görüntülerden hesaplanır (6 tekrar,
Ek G3-2). Kanıt derinliği **ikincildir** (3 tekrar): §G9 adım 4'ün ("tam boy okuma") yerine geçer
ve varsayılan bağ çözme sırası birincil → BT → kanıt derinliği doğruluğu → daha az ekran metni → ad
olur. Adım 1b (yüz okuma kapısı) M20'de uygulanmaz: M20 tasarımları yüzü değiştirmez (M18 gibi).
Kapsül derinliği görüntülerinin **kısma kopyası** (§G3, σ 4,5; `t_first`, `t_dir`, `t_valid`, 6
tekrar) sorulur ve bilgi olarak raporlanır. **Eleştirmen çiftleri** kapsül derinliği görüntülerinden
(iki durum, tr/açık), iki sıra, 3 tekrar.

**(b) Dokunma sayısı deterministik.** Her soru için: akıştan, cevabın göründüğü ekrana en az
dokunuş. Akıştaki hedef yüze kaydırmak dokunuş sayılmaz; açılmış katmanın içinde kaydırma gerekirse
"n + kaydırma" yazılır. Sayım düzeneğin kestirmesiyle değil çekirdeğin kararıyla yapılır (`taps.mjs`):
her ekranda aday dokunuşlar = açık sahne üzerinde 20 × 16 CSS px ızgaranın her noktası (sonucu
çekirdeğin `tap_at`/`hit` bölgeleri verir: derinlik, açılan soru kartı, sözlük açıklaması) ve açık
katmandaki DOM dokunma hedefleri (düğme, `summary`; Geri, sekmeler ve Ayarlar hariç). Genişlik
öncelikli arama, en çok 4 dokunuş; her dalda durum akıştan yeniden kurulur. `nobra`'da bir kartı
açmak bir dokunuştur (çekirdeğin `ask` durumu). tr/açık, iki okur (`arkin`, `visitor`).
"Cevap görünür" yüklemi, görünüm penceresindeki metin üzerinde (DOM metni + açık sahnenin metin
ilkelleri, 402×874 içinde), büyük/küçük harf duyarsız:
- `e_rests`: anahtar ailenin adı (kök): price → `fiyat`/`price`; volume → `hacim`/`volume`;
  flow → `akış`/`takas`/`flow`; news → `haber`/`bilanço`/`news`;
- `e_who`: çekirdeğin güven metinlerinden biri (`trust_text`, dört durum, TR/EN) ya da
  `sunucu doğruladı`, `cihazda sınan`, `verified by the server`, `checked on this device`;
- `numbers` ("neden rakam göremiyorum", yalnız `visitor`): `yetki`, `lisans`, `abonelik`,
  `entitle`, `licen`, `subscri`.

**(c) K52 iki aday ve başvuru için aynı ekranda** (`k52open.mjs`): kalibrasyonun sabit dokunuş
noktası `r4`'te grafiği, `nobra`'da bir soru kartını açıyor; bu yüzden ölçü ekranı herkese aynı
kurulur: sentetik gösterim adım 700'e aranır, okurun akışındaki ilk kapsül `open_capsule` ile
**kapsül derinliğinde** açılır (`?open=<ad>`; `r4` parametresiz), (1) oynatma 10 s (tik, veriyle
sahne boyama, dururken uzun kare), (2) duraklatılıp açık katmanın üstünde 6 s tekerlek kaydırma
(100 ms'de bir 120 px, her saniye yön değişir). Kayıt: tik p95, sahne boyama p95 (iki evre),
tekerlek evresinde uzun kare payı (çekirdeğin `frame_ms` `long` payı; kabuğun o evreye verdiği kip —
`scroll` ya da `zoom` — hangisinde kare varsa), ilk kapsül. Eşikler K52: `phone-jit-est` tik p95 ≤ 8 ms,
sahne p95 ≤ 8 ms, uzun kare ≤ %1, ilk kapsül ≤ 1,5 s; `phone` (JIT kapalı) uzun kare ≤ %10
(M18'deki K52 okuması). 3 tekrarın ortancası.

**(d) `e_who` anahtarı SPEC metnine uyduruldu** (Ek G4 gibi uygulama düzeltmesi): §G4 "`e_who` = açık
kapsülün güven iddiası (`open().trust`, sunucu dediyse sunucu)" der; uygulama `open().trust` sunucu
demediğinde akış öğesinin güvenine düşüyordu, oysa akış öğeleri hep `server` gelir (Ö52) — anahtar
"Bu cihazda sınandı" diyen kapsülde de `server` oluyordu. Şimdi: `open().trust` "sunucu/server" →
`server`; "cihaz/device" (sınandı, sınanıyor) → `device`; "farklı/differ" → `device`;
`open().trust` yoksa (veri yetkisiz okur) akış öğesinin güveni. K48'e göre açılmış açık-veri kapsülünde
iki ifade de doğru olduğundan (`server` ∪ `device`) gevşek puan da **bilgi** olarak raporlanır.

**(e) Yetkisizde rakam taraması (sert).** `visitor` görüntülerinde (iki derinlik) DOM metni ve açık
sahnenin metin ilkellerinde rakam içeren sözcük sayısı; enstrüman sembolü (`SYN\d+`) ve onu içeren
yetki adı (`<aile>/SYN\d+`) hariç. > 0 olan tasarım elenir (K33/K34, §G9 M20 satırı).

**(f) Sınırlar.** Sentetik dünyada "değerleri gizli ama barları açık" (lisanslı sunucu olgusu) okur
yoktur; yetkisiz okur yalnız `visitor` (veri yetkisi yok). Görünüş ve dil seçilmemiştir: bütün
adaylar `look=r4`, `voice=marla` ile çekilir. Bozuk tasarım Ek G3 gibi (başvurunun derlemesine
`broken/look.patch`, `?look=kestim`, açılmış kapsül `r4`).

## Ek G6 — M21 (hareket) ölçümünden önce (2026-11-18; ölçüm ajanı `r5/m21-measure`, hiçbir M21 okuyucu/eleştirmen çağrısından ve sayılan çekimden önce)

Gerekçe: §G'de M21 için yalnız açılış ara karesi (`motion`, §G6.4) yazılıydı; GOAL M21 satırındaki
"durum değişiminin fark edilmesi", "geçiş sırasında kare süresi" ve hareketi azalt bilgi kaybının
uygulaması yoktu. Kod: `m21.py` (sürücü), `shoot.mjs` (`change` çekimi, `m21probe`, `params`,
`reducedMotion`, sayfa saat kancası), `k52motion.mjs`, `k52.mjs --param`, `gl/keys.py`
(`n_where`, `n_where2`, `n_what`; `motion_final` = `m_same` + `m_where`), `gl/readers.py`
(`NOTICE_NOTE`, `NOTICE_PAIR_NOTE`, `CRITIC_M21`). Mekanik bir deneme (r4, surek, nabiz; tr/açık;
hiç okuma yok) kancaların çalıştığını gösterdi; o deneme sayılmaz, silindi.

**G6-0 Tasarımlar ve derlemeler.** Her aday kendi dalının `git archive` kopyasından ayrı derlenir;
başvurular ve bozuk tasarım `main` + `broken/look.patch` derlemesinden. Hepsi `look=r4`,
`voice=marla` (görünüş seçilmedi: adaylar renk/süreyi görünüşten okur — sonuç notu bunu sınır yazar).

| ad | derleme | adres | not |
|---|---|---|---|
| `r4` (bugünkü hareket) | main | `anim=1` | açılış `zumlak`, yüz değişimi hareketsiz |
| `still` (hareketsiz) | main | `anim=0` | ara kare yok: her "kare" ekranın kendisi (son kare) |
| `surek` | `r5/m21-surek` | `anim=1&motion=surek&reduce=0` | |
| `nabiz` | `r5/m21-nabiz` | `anim=1&motion=nabiz` | |
| `kestim` (bozuk, gizli) | main | `anim=1&look=kestim` | Ek G3; bugünkü hareketle |
| `surek-az`, `nabiz-az` | aday dalı | aday adresi + sistemin `prefers-reduced-motion: reduce` ayarı (Playwright `reducedMotion`) | yalnız bilgi ve bilgi kaybı; seçime girmez |

`r4`'ün hareketi azalt hâli `still`'dir (bugün ürünün tek azaltma yolu `anim=0`). Öteki çekimler
`reducedMotion: no-preference` ile.

**G6-1 Süreklilik (birincil, §G9 M21) ve geçerlilik sınaması.** `motion` kümesi §G1/§G4'teki gibi
(`live_event`, `closed_failed` açılışı; önce + p = 0,25 karesi; 2 dil × 2 tema). Ek:
(a) `motion_final` kümesi `m_same`'i de sorar (önce + son kare); (b) geçerlilik **soru × tasarım**
başına uygulanır: son kareyle doğruluğun Wilson %95 alt sınırı şansın (m_same 0,5, m_where 0,25)
**üstündeyse** o soru o tasarım için "son kareyle cevaplanabilir" = geçersizdir ve o tasarımın
birincilinden çıkarılır. Sınama tek yönlüdür (son karenin şansın altında kalması, son karenin soruyu
cevaplayamadığını gösterir); §G6.4'ün lafzı (aralığın şansı içermesi) de raporlanır. `still`'in ara
karesi son karesidir: soruları yapısal olarak son kareyle cevaplanır. Geçerli sorusu kalmayan
tasarımın birincili yoktur; adım 2'de birincili olanların arkasına dizilir (sürekliliği
gösterilmemiş tasarım süreklilikte ilk gruba giremez). Tekrar 6 (`motion` ve `motion_final`).
Ayrıca bilgi: ara karede ve son karede "aynı nesne taşınıyor" deme payı.

**G6-2 Durum değişiminin fark edilmesi (`change` çekimi).** Durumlar tasarımdan bağımsız, okurun
bütün akışında ardışık adımları tarayan bir sondadan (`m21probe`, main derlemesi, adım 1–500,
duyarlılık `all`) seçildi; sabit liste `m21.py` `CHANGE_STATES`:

| durum | adım N → N+1 | değişen kapsül | ekrandaki yeri |
|---|---|---|---|
| `c_born` | 216 → 217 | `cap.SYN09.breakout.up.143` doğdu | ikinci yüz |
| `c_strengthened` | 225 → 226 | aynı kapsül güçlendi | ilk yüz |
| `c_failed` | 333 → 334 | `cap.SYN20.breakout.down.154` tutmadı | ikinci yüz |
| `c_realized` | 207 → 208 | `cap.SYN11.breakout.up.140` gerçekleşti | ilk yüz |
| `n_still1`, `n_still2` | 211 → 212, 300 → 301 | değişim yok (denetim) | — |

Yöntem: sayfa N'de açılır (ilk akış = taban; hiçbir şey işaretli değil), akışın N'deki
`anchor_index` satırı ekranın üstüne kaydırılır, düzenek sayfanın görüntü karesi saatini
(`requestAnimationFrame` zaman damgası) eline alır, tekrarı bir adım ilerletir ve saati 16 ms'lik
adımlarla sürer. Kareler: `before` (adımdan önce), `mid` (adımı uygulayan kareden sonra tasarımın
kendi değişim süresinin yarısı: `surek` 180 ms = `change_ms` 360 / 2; `nabiz` 450 ms = vuruş 900 / 2;
değişim hareketi olmayanlarda 180 ms), `after` (1000 ms; iki adayın da değişim hareketi bitmiş),
`end` (5000 ms; bütün geçici izler bitmiş; bilgi kaybı karşılaştırması). Tasarıma özgü kanca
gerekmez; kayıtta adayların kancaları (`changed()`, `faceChanges()`, `.mk` kutuları, `#fx`
parçaları, yüz saydamlığı) yalnız okunur.

Sorular (`gl/keys.py`; ekranın soluna düzeneğin çizdiği 1–4 numaralı dört eşit yatay şerit):
- `notice` (tek kare, `mid` ve `after`; **fark edilme ölçüsü**): `n_where` "Bu ekranda az önce değişen
  (yeni çıkan ya da durumu yeni değişen) bir kapsül var mı? Varsa çoğunlukla hangi numaralı şeritte?"
  (1–4 / hayır). Anahtar: değişen yüzün DOM kutusunun dikey orta yarısının değdiği şeritler (o
  karenin kaydından; ekranda değilse puanlanmaz); denetimde "hayır". **Sınıf dengeli** (Ek G3 gibi;
  sınıflar "değişim" / "yok"). Tekrar 6.
- `notice_pair` (önce | `mid`; bilgi): `n_where2` (aynı soru, sağdaki kare) ve `n_what` (yeni kapsül
  çıktı / var olan kapsülün durumu değişti / hiçbir şey değişmedi; anahtar motorun değişiminden).
  Tekrar 3.
Hareketi azalt hâlleri (`surek-az`, `nabiz-az`) aynı sorularla okunur (bilgi: okuyucu gözüyle
azaltmada fark edilme).

**G6-3 Eleştirmen.** Görüntü üç kare yan yana: "0" başlangıç (`r4`'ün `before` karesi; hepsinde aynı
içerik) | A | B (açılışta p = 0,25 karesi, değişimde `mid` karesi); 402 × 3 + 48 = 1254 px < 1568.
İstem `CRITIC_M21` (beş önceden yazılı ölçüt: izlenebilirlik, değişenin görülmesi, dikkat
dağıtmama, ara karede okunaklılık, eylem önermeme; "daha çok hareket daha iyi demek değildir").
Durumlar: 2 açılış + 4 değişim; tr/açık; seçime giren 5 tasarım → 10 çift × 6 × 2 sıra × 3 tekrar.
Birleştirme, çelişki, BT ve önyükleme §G5'teki gibi.

**G6-4 K52.** (a) `k52.mjs` tasarımın adres parametreleriyle (`--param`, kabuk kopyasında
`look-shim.js`), §G7.6'daki gibi; (b) `k52motion.mjs` geçiş sırasında kare aralıkları: kalibre
benzetim (`phone-jit-est`, `phone` JIT kapalı), adım 120'de 6 kez akış → kapsül → grafik → geri →
geri (her dokunuştan sonra 700 ms) ve 8 s oynatma (akıştaki yüz değişimleri); uzun kare > 33,4 ms;
3 koşunun ortancası. **Kapı:** `phone-jit-est` (bağlayıcı) (a) geçer ve (b) geçiş pencerelerinde ve
oynatmada uzun kare payı ≤ %1; `phone` (JIT kapalı, "kullanılabilir") (b) ≤ %10 (M18 örneği).
Hiçbir tasarım (başvurular dahil) bir eşiği geçemiyorsa o eşik uygulanmaz (§G9.1).

**G6-5 Hareketi azalt bilgi kaybı (sert, 0 olmalı) — tek yönlü.** Tasarım d ve azaltılmış hâli d′
(`r4`→`still`, `surek`→`surek-az`, `nabiz`→`nabiz-az`), aynı durum/dil/tema:
(a) §G7.7: son ekranların (açılış sonu; değişimde `end`) durağan içeriği — çekirdeğin boyadığı yüz
sahnelerinin (`face_scene`) metinleri ve işaret varlığı + DOM metinleri — `içerik(d) − içerik(d′)`
boş olmalı; (b) hareketin taşıdığı olgular: `mid` karesinde hareket katmanı olan (üstünde `.mk`
kutusu, `#fx` parçası ya da saydamlığı < 1 olan) her yüz için d′'nün `end` karesinde o yüzün,
bugünkü (`r4`) `end` yüzünde olmayan en az bir durağan metni (etiket) bulunmalıdır; yoksa kayıp.
**Hareketi azalt'ta fazladan duran etiket kayıp değildir** (bir aday ajanın önerisi; tanım tek
yönlü). Açılış geçişi olgu taşımaz (son ekran aynı; (a) bunu denetler). `still` azaltılmış hâlin
kendisidir (kayıp tanım gereği 0).

**G6-6 Bozuk tasarım.** Ek G3 ve §G8 aynen: `kestim` birincilde (birincili olan tasarımlar arasında)
ve BT'de **kesin son** olmalı; değilse koşu kullanılmaz. Fark edilmede ve bütün `motion`
doğruluğunda son olup olmadığı bilgi olarak yazılır.

**G6-7 Seçim kuralı (M21).** 1. Sert eşikler: K52 (G6-4), bilgi kaybı = 0 (G6-5); §G7.1–5 görünüş
kapıları M21'de uygulanmaz (bütün tasarımlar aynı `r4` görünüşüyle; M16'nın konusu). 1b. Tam boy
okuma kapısı uygulanmaz (yüz okuma kümesi yok). 2. Birincil: süreklilik (G6-1, geçerli sorular, F2:H1,
%95 önyükleme); en iyinin aralığıyla örtüşenler ilk grup. 2b. Fark edilme (G6-2 `notice`, F2:H1);
grupta en iyinin aralığıyla örtüşenler kalır. 3. Eleştirmen BT. 4. Tek aday kalırsa kazanan; yoksa
"ayırt edilemedi". **Varsayılan için bağ çözme** (nokta tahminleri): birincili olan önce → birincil →
fark edilme → BT → daha az ekran metni (`end` ekranları) → ad. İlk iki bu sıradan.

**G6-8 (sonradan; bilgi kaybı sonuçları görüldükten sonra, okuyucu/eleştirmen sonuçlarından önce
yazıldı).** İki bulgu, ikisi de açıkça işaretlenir:
1. *Uygulama hatası (Ek G4 türü):* (b)'nin ilk uygulaması yüz metinlerini çıplak dizgi kümesi olarak
   karşılaştırıyordu; `nabiz`'in TR etiketi "gerçekleşti", `r4` yüzünde satır kırılmasıyla tek başına
   duran cümle satırıyla aynı dizgi olduğu için "yok" sayıldı. G6-5'in metni "yüzde olmayan durağan
   metin"dir (bir öğe); öğe = (metin, x, y). Düzeltilmiş metin kuralıyla kayıp: `nabiz` 4 (yalnız
   `c_born`), `surek` 0, `r4` 0. Çıplak dizgi sonucu (6) de raporlanır.
2. *Tanım boşluğu:* (b) "mid karesinde hareket katmanı olan her yüz"ü sayıyor; `nabiz`'in `c_born`
   yüzündeki katman doğumun vuruşu değil, her canlı kapsüldeki sürekli **nabız**dır (vuruş yok; yüz
   önceden yoktu). Nabzın taşıdığı olgu "canlı"dır ve azaltılmış hâlde metin değil **canlı noktası**
   (dolu daire ilkeli) olarak durur (yüzde `r4`'te olmayan bir ilke). Orkestratörün tanımı ("hareketle
   taşınıp hareketi azalt'ta eksik olan olgu = kayıp") buna kayıp demez. **Ana karar** işaret olarak
   metin ya da metin dışı herhangi bir durağan ilkeyi kabul eder (`--loss-rule any_prim`: `nabiz` 0);
   **duyarlılık** G6-5'in lafzını uygular (yalnız metin: `nabiz` 4 → elenir). İki sonuç da raporlanır.

## Ek G8 — M21 yeniden ölçüm (2026-11-18; ölçüm ajanı `r5/m21-remeasure`, hiçbir yeni okuyucu/eleştirmen çağrısından ve sayılan çekimden önce)

Gerekçe: ilk M21 koşusu geçersizdi (`rounds/R5/experiments/M21/RESULTS.md`): (a) süreklilik sorusu
(`m_same`) son kareyle de cevaplanıyordu (hareketsiz başvuruda bile 0,98 "aynı nesne": açılan kapsül aynı
cümleyi taşıdığı için okuyucu metin benzerliğiyle cevaplıyordu); `m_where` hiçbir tasarımda puanlanamadı;
(b) bozuk tasarım bir *görünüş*tü (`kestim`), hareketi bozmuyordu, fark edilmede son değildi; (c) denetim
karelerinde yanlış alarm %77–79 idi ve denetim kareleri değişim karelerinin yarısıydı; `nabiz`'in fark
edilme puanının bir kısmı değişimden değil canlı kapsüllerdeki sürekli nabızdan geliyordu. Bu ek o üç
sorunu giderir; geri kalanı (Ek G6) aynen geçerlidir, aşağıda değişen her şey yazılıdır. Ek G6'nın
kayıtlarına bakılarak yazıldı; yeni hiçbir okuma yapılmadı. Kod: `m21v2.py` (sürücü), `shoot.mjs`
(`band` yerleştirme, `brokenMotionInit` süzgeci), `gl/keys.py` (`m_from`, `motion2`, `motion2_final`),
`gl/readers.py` (`MOTION2_NOTE`).

**G8-0 Tasarımlar ve tek derleme.** Hareket tasarımları artık `main`'de birleşik: bütün tasarımlar
`main`'in **tek** derlemesinden (`qp` release + `scripts/demo.sh` WASM paketi), adres parametresiyle,
aynı içerikle. Görünüş seçildi: hepsi `look=oltan`, `voice=marla` (gösterim varsayılanları; öteki
seçimler — yüz, akış, açılmış kapsül — gösterim varsayılanı). Hareketi azalt adresle verilir
(`reduced_motion=0|1`; sistem ayarı Playwright'ta hep `no-preference`).

| ad | adres | rol |
|---|---|---|
| `surek` | `motion=surek&reduced_motion=0` | aday |
| `nabiz` | `motion=nabiz&reduced_motion=0` | aday |
| `r4` | `motion=r4&reduced_motion=0` | başvuru (bugünkü hareket: açılış geçişi, yüz değişimi hareketsiz) |
| `still` | `motion=r4&reduced_motion=1` | başvuru (hareketsiz; etiket de yok) |
| bozuk hareket (gizli ad, yalnız anahtar dosyasında) | `motion=surek&reduced_motion=0` + düzenek süzgeci (G8-2) | düzeneğin sınaması |
| `surek-az`, `nabiz-az` | aday adresi, `reduced_motion=1` | yalnız bilgi kaybı; seçime girmez |

`r4`'ün azaltılmış hâli `still`'dir (bilgi kaybı tanım gereği 0).

**G8-1 Konuma dayalı süreklilik sorusu (`m_from`; birincil).**
- *Uyaran (`motion2`):* yalnız iki ara kare yan yana: solda "I" = p = 0,25, sağda "II" = p = 0,5 (açılış
  geçişi `window.qp.freeze(p)` ile; hareketsizde kare = ekranın kendisi). **"Önce" karesi gösterilmez**
  (ilk koşuda okuyucu açılan kapsülü önceki ekrandaki aynı metinle eşliyordu). Her karenin soluna
  düzenek 1–4 numaralı dört eşit yatay şerit çizer (fark edilme sorusundaki şeritlerin aynısı; bölgeler
  sayıyla, seçenekler harfle — harf eşleme karışıklığı yok).
- *Soru:* "Bu iki kare aynı geçişin art arda iki anı (I önce, II biraz sonra). Açılan kapsül ekranın
  hangi numaralı şeridinden geliyor; geçiş nereden başlıyor?" Seçenekler: 1. şerit (en üst) · 2. şerit ·
  3. şerit · 4. şerit (en alt) · "bir yerden gelmiyor; olduğu yerde beliriyor" (`none`) · bilmiyorum.
- *Anahtar (tasarımdan bağımsız):* geçişten önceki ekranda hedef yüzün DOM kutusunun dikey ortasının
  bulunduğu şerit (çekim kaydından). Bunu tasarımdan bağımsız kılmak için **açılış durumları şeride
  yerleştirilir**: iki hedef kapsül (Ek G6'daki `live_event` ve `closed_failed` hedefleri, aynı adım) ve
  her biri için akış, hedef yüzün dikey ortası `k`. şeridin ortasına (y = 874·(k − ½)/4) gelecek biçimde
  kaydırılır, k = 1–4 → **8 açılış durumu** (`o_live_1..4`, `o_fail_1..4`). Kabul şartı (çekimden
  sonra kayıtla denetlenir): yüz kutusunun dikey orta yarısı tek bir şeritte (k) kalır; kalmazsa o durum
  için en yakın kaydırma kullanılır ve anahtar gerçekleşen kutudan alınır (kayıtta yazılır). Sınıflar
  dengeli: her şerit 2 durumda anahtardır.
- *Son kare (geçerlilik denetimi, `motion2_final`):* aynı soru, aynı biçimde, iki panel de **son kare**
  (açılış bitmiş ekran) ile. Aynı hedefin dört şerit durumunda son ekran aynı içeriği gösterir (açılan
  kapsül); son karede geçişin nereden başladığının cevabı yoktur.
- *Geçerlilik sınaması (tek yönlü, eşik önceden yazılı):* şans düzeyi **0,25** (anahtar her zaman dört
  şeritten biri ve şeritler dengeli: anahtardan bağımsız her cevap dağılımı beklenen değerde en çok 0,25
  alır; `none` ve bilmiyorum 0). (i) Tasarım başına: `motion2_final` doğruluğunun Wilson %95 **alt sınırı
  0,25'in üstündeyse** soru o tasarım için son kareyle cevaplanabilir = **geçersiz**, o tasarım için
  puanlanmaz (birincili yok). (ii) Hareketsiz başvuru: `still`'in `motion2` doğruluğu (kareleri zaten
  son kare) aynı sınamadan geçer; alt sınırı 0,25'in üstündeyse soru **bütün tasarımlar için**
  geçersizdir ve M21 sonucu "geçersiz soru" yazılır. Sınama tek yönlüdür; iki yönlü lafız (aralığın
  0,25'i içermesi) bilgi olarak raporlanır.
- `still`'in ara karesi yoktur: yapısal olarak birincili yoktur (ara karesi olmayan tasarım süreklilik
  gösteremez; Ek G6-1'deki gibi adım 2'de birincili olanların arkasına dizilir). `still`'in `motion2`
  okumaları yalnız geçerlilik sınamasının (ii) girdisidir.
- Tekrar 6 (`motion2` ve `motion2_final`), personalar F ve H, haiku, 2 dil × 2 tema. `m_same` ve
  `m_where` artık sorulmaz.

**G8-2 Bozuk hareket (düzenek süzgeci; ürüne girmez).** `shoot.mjs` `brokenMotionInit`: yalnız bozuk
tasarımın sayfasına eklenen bir başlatma betiği; temel tasarım `surek` (taşınan parçaları olan
tasarım). Sayfa her hareket parçasının (`#fx canvas`: açılış geçişinin ve yüz değişiminin parçaları)
biçemini her ayarladığında süzgeç o parçayı (a) ekranın yatay orta çizgisine göre **aynalanmış yere
sıçratır** (y → 874 − y − h; 1↔4, 2↔3. şerit) ve (b) **titretir**: tohumlu sapma (yatay ±24 px, dikey
±16 px, dönme ±6°; tohum = parçanın sırası ve sayfanın o parçaya biçem ayarlama sayısı; aynı kare her
çekimde aynı). (c) Yüz tuvallerinin saydamlığı 1'de tutulur (değişen yüzün yerinde solması kalkar).
Sonuç: açılış yanlış şeritten başlıyor görünür, durum değişiminde vuruş **yanlış yerde** olur. Süzgeç
CSS'in bağımsız `translate`/`rotate` özelliklerini kullanır; sayfanın kendi kodu ve çizimi değişmez.
**Kabul kuralı (her koşuda):** bozuk hareket (1) birincil ölçüde (`m_from`, birincili olan tasarımlar
arasında, nokta tahmini) ve (2) eleştirmen BT'de **kesin son** olmalıdır (eşitlik son sayılmaz;
birincili yoksa (1) sağlanmamış sayılır). Biri tutmazsa **koşu geçersizdir**, seçim yapılmaz. Fark
edilme puanında (G8-3) son olup olmadığı bilgi olarak yazılır.

**G8-3 Fark edilme (`notice`; seçimde 2b).**
- *Durumlar:* 4 değişim + **4 denetim** (dengeli; ilk koşuda 4 + 2). Değişimler Ek G6-2'deki dört sınıf:
  doğum, güçlenme, tutmama, gerçekleşme; denetimler değişim olmayan adımlar. Durumlar bu derlemenin
  (look `oltan`) ardışık adım sondasından (`m21probe`, adım 1–500) şu kuralla seçilir: değişen kapsül
  ekranda (çapa satırı en üste kaydırılınca) ilk ya da ikinci tam görünen yüzdür; Ek G6-2'nin adımları
  hâlâ kurala uyuyorsa aynen kullanılır. **Canlılık ile değişim ayrı sınıflarda:** denetimlerin ikisinde
  ekranda tam görünen en az bir canlı kapsül vardır (`nabiz`'in nabzı denetim karesinde de görünür;
  orada "değişti" demek yanlış alarmdır), ikisinde ekranda canlı kapsül yoktur; bu derlemede canlısız
  ekran yoksa dördü de canlılı olur (yazılır). Değişimler de iki sınıfta raporlanır: hedef canlı
  (doğum, güçlenme) / hedef kapanmış (tutmama, gerçekleşme). Seçilen liste `m21v2.py`
  `CHANGE_STATES`'e yazılır ve bu eke tablo olarak eklenir (okumalardan önce).
- *Kareler:* `mid` (tasarımın değişim süresinin yarısı, `oltan`'ın `duration_scale` 1,25'iyle:
  `surek` ve bozuk 225 ms = 360·1,25/2; `nabiz` 562 ms = 900·1,25/2; değişim hareketi olmayanlarda
  225 ms) ve `after` (1000 ms) tek kareler, 1–4 şeritli; `end` (5000 ms) yalnız bilgi kaybı için.
  Soru `n_where` (Ek G6-2 metni aynen). `notice_pair` sorulmaz (bütçe).
- *Birincil fark edilme puanı (önceden yazılı):* **isabet − yanlış alarm**. İsabet = değişim
  karelerinde doğru şerit payı (durum başına ortalama, sonra 4 değişim durumunun ortalaması); yanlış
  alarm = denetim karelerinde herhangi bir şerit deme payı (aynı biçimde 4 denetim durumunun
  ortalaması); bilmiyorum/harfsiz cevap denetimde yanlış alarm sayılmaz, değişimde isabet sayılmaz.
  Persona başına hesaplanır, F2:H1 ağırlıklı; %95 önyükleme (2000; birimler persona × durum katmanında
  yeniden örneklenir; tohum `20261118`). Sınıf ayrıntısı raporlanır: isabet (hedef canlı / kapanmış),
  yanlış alarm (canlılı / canlısız denetim).
- Tekrar 6, F ve H, haiku, 2 dil × 2 tema; `surek-az` ve `nabiz-az` fark edilmede okunmaz (bütçe;
  bilgi kaybı hesapla ölçülür).

**G8-4 Eleştirmen.** Ek G6-3 aynen (üç kare "0" | A | B, `CRITIC_M21`, tr/açık), durumlar: 2 açılış
(`o_live_2`, `o_fail_3`; ara kare p = 0,25; "0" = `r4`'ün geçişten önceki ekranı) + 4 değişim (`mid`;
"0" = `r4`'ün `before` karesi); seçime giren 4 tasarım + bozuk = 5 → 10 çift × 6 durum × 2 sıra × 3 tekrar
= 360 çağrı. Birleştirme, çelişki (> %30 → ayırt edilemedi), BT ve önyükleme §G5'teki gibi.

**G8-5 K52.** Ek G6-4 aynen, tek derlemede `surek`, `nabiz`, `r4`, `still` için (`k52.mjs --look oltan
--param ...`, `k52motion.mjs`).

**G8-6 Hareketi azalt bilgi kaybı (sert kapı).** Ek G6-5 **lafzıyla**: hareketin taşıdığı olgu
hareketi azalt hâlinde **metin olarak** durmalıdır (`r4` → `still` 0 tanım gereği; `surek` →
`surek-az`, `nabiz` → `nabiz-az`). Öğe = (metin, x, y) (Ek G6-8.1'in uygulama düzeltmesi). Ek G6-8.2'nin
"herhangi bir durağan ilke" okuması kapı değildir, yalnız duyarlılık olarak raporlanır
(`--loss-rule text`).

**G8-7 Seçim kuralı (Ek G6-7 aynen, yeni ölçülerle).** 1. Sert eşikler: K52 (G8-5), bilgi kaybı = 0
(G8-6, metin). Hiçbir tasarım bir eşiği geçemiyorsa o eşik uygulanmaz (§G9.1). 2. Birincil: süreklilik
`m_from` (G8-1; yalnız sorunun geçerli olduğu tasarımlar; F2:H1, %95 önyükleme); en iyinin aralığıyla
örtüşenler ilk grup; birincili olmayanlar arkada. 2b. Fark edilme (G8-3, isabet − yanlış alarm, F2:H1);
grupta en iyinin aralığıyla örtüşenler kalır. 3. Eleştirmen BT; en iyinin aralığıyla örtüşenler kalır.
4. Tek aday kalırsa "kazanan"; kalmazsa **"ayırt edilemedi"** (kazanan ilan edilmez). **Varsayılan için
bağ çözme** (nokta tahminleri): birincili olan önce → birincil → fark edilme → BT → daha az ekran metni
(`end` ekranları) → ad. **İlk iki** bu sıradan. Koşu G8-2 kabul kuralını geçmezse ya da G8-1 (ii)
soruyu bütün tasarımlar için geçersiz kılarsa seçim yapılmaz.

**G8-8 Bütçe ve kullanım sınırı.** Tahmin (Ek G6 koşusunun birim maliyetleriyle): okuyucu `motion2` +
`motion2_final` 5 × 8 × 4 × 2 × 2 × 6 = 3 840 haiku ≈ 4 $; `notice` 5 × 8 × 4 × 2 kare × 2 × 6 = 3 840
haiku ≈ 3 $; eleştirmen 360 sonnet ≈ 6 $; boylar arası uyum (sonnet, `motion2`, tr/açık, F, 1 tekrar,
`o_*_2` ve `o_*_3`, 5 tasarım = 20) ≈ 0,4 $; toplam ≈ 13,5 $ ≤ 15 $. Bütçe aşılacak gibi olursa önce
uyum alt kümesi, sonra eleştirmenin üçüncü tekrarı düşer (yazılır). Kullanım sınırına takılıp cevapsız
dönen çağrı (sonuç metni sınır iletisi ya da boş, maliyet 0) yanlış sayılmaz: kaydı silinir, yeniden
koşulur, sayısı raporlanır.

**G8-9 (mekanik denemeden sonra, hiçbir okumadan önce; 2026-11-18).** Kancaları sınamak için yalnız
tr/açık'ta 5 tasarımın açılış ve değişim çekimleri alındı (sayılmaz, silindi; hiçbir okuyucu ya da
eleştirmen çağrısı yok). Üç uygulama düzeltmesi ve durum listesi:
1. *Şerit geometrisi:* akış pencerenin değil sayfa öğesinin (`.page`) içinde kayar ve üst çubuğun
   (`#top`, alt kenarı y ≈ 91 px) altında başlar; yüz 226 px boyunda. Ekranın tamamına çizilen dört şeritte
   1. şeridin ortasına yüz yerleştirilemez (yarısı üst çubuğun altında kalır). Bu yüzden **`m_from`'un
   şeritleri akış alanının** (üst çubuğun alt kenarı `y0`'dan ekranın altına, kayıttaki `top_bar_bottom`)
   **dört eşit şerididir**; yerleştirme (`band_k`) yüzün dikey ortasını o şeridin ortasına getirir
   (`.page` kaydırılır), çizgiler ve anahtar aynı `y0`'la. Gerçekleşen kutular: yüz üstü 76 / 272 / 468 /
   663 px, orta yarısı tek şeritte (1–4). Fark edilme sorusunun şeritleri (Ek G6-2) değişmedi (tam ekran).
2. *`fail` hedefi:* `cap.SYN26.breakout.down.47` akışın 2. satırı; 4. şeride yerleştirmek için üstünde en az
   3 yüz gerekir. Ek G6 aday listesinde bu şartı sağlayan ilk aday **`cap.SYN26.breakout.down.51`** (5.
   satır, tutmadı). `live` hedefi (`cap.SYN06.breakout.up.118`, 15. satır) aynı.
3. *Bozuk hareket:* G8-2'nin aynalaması (y → 874 − y − h) ekranın ortasına binen ikinci yüzü (değişimde)
   yerinde bırakıyordu (orta çizgi yüzün içinden geçiyor). Süzgeç parçayı **yarım ekran sıçratır**: merkezi üst
   yarıdaysa +437 px, alt yarıdaysa −437 px (dört şeritte her zaman en az iki şerit öte), titreme ve yüz
   saydamlığı G8-2'deki gibi. Mekanik denemede bütün açılış ve değişim parçaları yanlış şeritte.
4. *Fark edilme durumları* (G8-3 kuralıyla; `m21v2_states.json`): Ek G6-2'nin dört değişim adımı ve iki
   denetimi kurala hâlâ uyuyor; canlılı iki denetim, adım ≥ 201'den başlayarak değişimsiz, Ek G6 adımlarına
   bitişik olmayan ve aralarında ≥ 10 adım olan ilk iki adım (canlı kapsül biri ilk, biri ikinci yüz):

| durum | adım N → N+1 | çapa satırı | sınıf | ekranda (tam görünen 3 yüz) |
|---|---|---|---|---|
| `c_born` | 216 → 217 | 18 | değişim, hedef canlı | `SYN09.143` doğdu, ikinci yüz |
| `c_strengthened` | 225 → 226 | 19 | değişim, hedef canlı | `SYN09.143` güçlendi, ilk yüz |
| `c_failed` | 333 → 334 | 10 | değişim, hedef kapanmış | `SYN20.154` tutmadı, ikinci yüz |
| `c_realized` | 207 → 208 | 19 | değişim, hedef kapanmış | `SYN11.140` gerçekleşti, ilk yüz |
| `n_live1` | 201 → 202 | 19 | denetim, canlılı | `SYN11.140` canlı, ilk yüz |
| `n_live2` | 218 → 219 | 18 | denetim, canlılı | `SYN09.143` canlı, ikinci yüz |
| `n_still1` | 211 → 212 | 18 | denetim, canlısız | — |
| `n_still2` | 300 → 301 | 10 | denetim, canlısız | — |

   Mekanik denemede (bilgi; tasarım farkı okunmadı, yalnız kancalar): `nabiz`'in canlılı denetimlerde nabız
   kutusu var, canlısızlarda yok; `surek` doğumda değişim parçası üretmiyor (ilk koşudaki gibi).
