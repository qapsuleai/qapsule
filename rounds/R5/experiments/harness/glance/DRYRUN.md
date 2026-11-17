# R5/T2 — tek bakış düzeneği: kuru koşu (2026-11-17/18)

> Ajan okuyucu insan değildir; bu ölçüm ön elemedir (SPEC §G0). Bu bir **düzenek sınamasıdır**:
> tasarım kararı değildir; örneklem küçüktür (yalnız `tr` + açık tema, 1–2 tekrar).

Adaylar: bugünkü yüz `vadrel` (başvuru, `r4` sunumu), M8'in ikincisi `fotolay`, ve gizli adla
bozuk tasarım `kestim` (= `vadrel` + §G8 Canvas süzgeci). Kod: `r5/t2-glance`; koşu klasörü
scratchpad'de (ekranlar depoya girmedi); özetler `dryrun/`.

## Ne koşuldu

| Aşama | Sonuç |
|---|---|
| `probe` | 64 adım × 2 okur, ≈ 30 s |
| `states` | 8 durumun 8'i bulundu (S* = adım 1; `no_longer`, `no_record` adım 40; `few_cases` adım 80; son ikisi `?watch=`) — `dryrun/states.json` |
| `shoot` | 3 aday × 13 görüntü = 39, sorun 0. **Belirleyicilik:** 4 görüntü ayrı bir koşuda yeniden çekildi, 4/4 bayt eşit (`canvas` yolu, aynı makine) |
| `derive` | 117 okuyucu uyaranı, 48 eleştirmen çifti (8 durum × 3 çift × 2 sıra) |
| `compute` | `dryrun/computed.md` |
| `read` | 318 haiku + 6 sonnet çağrısı (tutulan); 372 kaydın hiçbirinin isteminde tasarım adı yok, hepsinde dosya adı rastgele 8 karakter |
| `critic` | 48 sonnet çağrısı (8 durum × 3 çift × 2 sıra × 1 tekrar) |
| `k52` | `vadrel`, `phone-jit-est`, 1 tekrar: tik/yüz/sahne p95 ≈ 1 ms, ilk kapsül 138 ms, kaydırma kare p99 18 ms → geçti (`dryrun/k52-vadrel.json`; WebKit zamanlayıcısı 1 ms taneli) |

**Maliyet:** tutulan çağrılar 1,105 $ (haiku 318 × 0,00091 $; sonnet okuyucu 6 × 0,020 $; sonnet
eleştirmen 48 × 0,0145 $) + σ ayarı 0,017 $ + σ değişince silinen eski kısma/hareket okumaları
≈ 0,04 $ → **toplam ≈ 1,16 $**. Ortalama süre: haiku 8,4 s, sonnet 6,9 s/çağrı.

## Bulgular (düzeneği değiştirenler; hepsi alternatiflerden önce — SPEC Ek G1)

1. **Kısma σ ayarı.** Önerilen σ = 2,5 px'te (+ 0,5×) haiku başlık cümlesini neredeyse harfi
   harfine okudu (benzerlik 0,987). σ 3,5 → 0,88; **σ 4,5 → 0,18 / 0,42** (`calibration/sigma.json`).
   Ajan küçük ve bulanık metni insandan iyi okuyor (U3); kısma kopyası metni okutursa hiyerarşi
   değil OCR ölçülür. σ = 4,5 seçildi.
2. **Tam boy okuma doğruluğu bozuk tasarımı ayırmadı.** Bozuk tasarımın (ters hiyerarşi, %65
   soluk metin, %30 işaret) tam boy okuma doğruluğu 0,882 [0,840, 0,926]; `vadrel` 0,893, `fotolay`
   0,857. Yarım boyda (0,5×) da aynı (0,898 / 0,894 / 0,852); 0,5× + σ 1–2 px'te bile soluk metin
   okundu (`calibration/sigma2.json`: bozuk 0,71–0,99). **R4'ün tuzağı (K46) burada ölçüldü:** okuma
   sorusu metni okuyabilen ajana görsel kaliteyi göstermiyor. Kabul kuralı (§G8) bu ölçüyle
   **geçmezdi** → düzenek düzeltildi: birincil ölçü **kısma kopyasındaki tek bakış doğruluğu**
   oldu; tam boy okuma, başvuruya karşı **aşağı kalmama kapısı** (adım 1b).
3. **M21 bölge sorusu bugünkü geçişte ölçülemez.** p = 0,5'te taşınan parçaların çeyreği son
   karedekiyle aynı ya da sınırda; son kareyle sorulduğunda doğruluk 1,0 (U10 sınaması geçmedi).
   Kural: ara kare p = 0,25 ve yalnız p = 0,25 çeyreği son çeyrekten farklı olan durumlar
   puanlanır; bugünkü geçişte 6/6 durum puanlanmadı (soru ancak parçaları ekranda gerçekten
   taşıyan alternatiflerde anlam kazanır; yalnız `m_same` kalır). M21 için ek, tasarımdan
   bağımsız bir süreklilik sorusu açık risktir (§G11).
4. **Renk kapısı yalnız renkle kodlanmış çiftlere.** Aynı renkte çizilen çiftler (ΔE00 normal <
   5) renkle kodlanmamıştır; anlamları biçimle taşınmalı (gri soru). Ölçüde işaretin baskın
   görünür rengi kullanılır (zemin rengindeki boşaltma ilkeli atlanır; ilk sürümde beyaz bir daire
   seçilip oran 1,0 çıkıyordu).
5. **İzleme listesi kalıcı.** `?watch=` okurun belgesine yazılıyor; sonraki sayfada akışı değiştirip
   başka adayın hedefini gizledi. Çekim sırası: izleme listesiz çekimler önce.
6. **Tek bakış dışı çağrı** (iki `Read`) 1/324; puanlamadan çıkarıldı (önceden "sayılır" yazıyordu).

## Sonuç tablosu (düzeltilmiş kuralla; `dryrun/scores.md`)

| tasarım | tek bakış: kısma (birincil) | okuma: tam boy | gri | kanıt | BT [%95] |
|---|---|---|---|---|---|
| `fotolay` | 0,372 [0,316, 0,434] | 0,857 [0,807, 0,903] | 0,896 | 1,000 | +1,32 [0,78, 2,03] |
| `vadrel` | 0,278 [0,240, 0,316] | 0,893 [0,856, 0,930] | 0,938 | 0,944 | +1,09 [0,55, 1,74] |
| `kestim` (bozuk) | **0,257** [0,212, 0,302] | 0,882 [0,840, 0,926] | 0,917 | 0,889 | **−2,41** [−2,66, −2,20] |

Kısmada `t_first` ("ilk ne görüyorsun") işaret cevabı: `vadrel` 19/32, `fotolay` 20/32, bozuk
**10/32** (bozukta cümle 21/32). Konum çelişkisi: `fotolay|vadrel` 2/8, bozuklu çiftlerde 0/8.
Uzunluk yanlılığı: kesin sonuçlarda daha çok metinli olanın kazanma payı 0,15 (n = 13) —
eleştirmen bu koşuda metni kayırmadı. Haiku ↔ sonnet okuyucu uyumu 0,944 (n = 36 cevap).

**Bozuk tasarım sınaması (`dryrun/broken-check.json`): GEÇTİ** — birincilde son (0,257 < 0,278 <
0,372; aralığı en iyinin aralığının altında), BT'de son (aralıklar ayrık). Pay ince (kısma
`vadrel` ile aralıklar örtüşüyor); tam koşuda (3 tekrar, 4 dil/tema) yeniden sınanır.
Bilgi: tam boy okumada bozuk son **değil** (bulgu 2).

Seçim kuralı (yalnız gösterim): sert eşiklerin beşi de **hiçbir adayca** geçilemedi → uygulanmadı
(E6 kuralı): kabuk kaynaklı (sekme yüksekliği 32,9 px < 44; "Sunucu doğruladı" 10–11 px; kanıt
ekranında açık temada turuncu metin 1,65:1) ve alfabe kaynaklı (kapanmış işaret 1,29:1;
"gerçekleşti" ile "aşağı, geçerli" deutan benzetiminde ΔE00 0,71). Birincil: `fotolay` ile `vadrel`
aralıkları 0,316'da değiyor → **ayırt edilemedi**; varsayılan için bağ çözme `fotolay`.

## Bozuk tasarımın durumu

Mekanizma: T3'ün `?look=` kaydı henüz yok; bozuk tasarım düzenekte bir Canvas 2D süzgeciyle
kuruldu ve koşuldu (yukarıda). Sınır: süzgeç sahne tarifini değiştirmez, bu yüzden `compute`
bozuk tasarım için `vadrel`'in değerlerini verir (bozuk zaten seçime girmez). T3 birleşince aynı
tarif bir görünüş kaydı olarak kurulmalı ve sınama o kayıtla yeniden koşulmalı (§G8).
