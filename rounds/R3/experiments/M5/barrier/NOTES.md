# M5 aday `barrier` — Oynaklık bariyeri

Crate: `rounds/R3/experiments/M5/barrier/` (`m5_barrier`). Giriş: `outcome()` (ad `barrier`),
`perturbations()`. Düzenekte kayıtlı (`harness-sim m5 run --alt barrier`). Ham ve özet:
`rounds/R3/experiments/M5/raw/barrier/` (gerçek veride yalnız toplu özet).

## Tanım

Her hikâye için doğum barında iki sabit alınır, sonra yalnız kapanışlara bakılır:

- **Başvuru fiyatı** `P0` = doğum barının kapanışı.
- **Ölçü** `A` = doğum barındaki Wilder ATR, `qp_compute::Atr` ile **yalnız `bars[..=doğum]`**
  üzerinden. Yönlü hikâyede ATR(14), nötr hikâyede ATR(100).
- **Yönlü hikâye** (`up`/`down`): `d = ±(kapanış − P0)` (hikâye yönünde artı). `(doğum, şimdi]`
  aralığında ilk `d ≥ k·A` → **gerçekleşti** (`barrier.target`); ilk `d ≤ −k·A` →
  **geçersiz** (`barrier.stop`). Hiçbiri ufka kadar olmazsa motor **süresi doldu** der
  (`horizon`). Bariyer simetriktir (hedef = durak).
- **Nötr hikâye** (sıkışma; olası nötr akış): iddia "bir hareket geliyor, yönü belirsiz".
  İlk `|kapanış − P0| ≥ k_n·A100` → **gerçekleşti** (`barrier.expansion`); ufka kadar olmazsa
  **geçersiz** (`barrier.no_expansion`): bu iddia için koridorun içinde kalmak sonucun
  kendisidir, sonuçsuzluk değil. (Kural ufuk barında motorun `expired` adımından önce
  çalışır.)
- Hüküm `bars[..=şimdi]` ve hikâyenin (tür, yön, doğum barı) saf fonksiyonudur: kural her
  çağrıda `(doğum, şimdi]`'yi baştan tarar (ilk geçiş), her barda çağrılmasına dayanmaz.
  Doğum ATR'si önbellekte tutulur; anahtar doğum barının kendi değerlerini (t, h, l, c)
  içerir.
- Doğum barında hüküm yok; ATR ısınmamışsa (ATR(14) için bar < 13, ATR(100) için < 99) hüküm
  yok → süresi dolar.

### Tür başına parametreler

Bariyer, türün ufkundan difüzyon ölçeklemesiyle gelir: yönlü `k = 0,3·√h`, nötr
`k_n = 0,65·√h` (ATR birimi). Ufuk çeşitlemesi bariyeri **taşımaz** (bariyer temel ufuktan
hesaplanır); iki parametre bağımsız sarsılır.

| Tür | Zaman türü (dedektör) | Ufuk h (bar) | Yönlü k (× ATR14) | Nötr k_n (× ATR100) | Neden |
|---|---|---|---|---|---|
| Devam | durum (`ma_stack`) | 20 | 1,34 | 2,91 | Trend iddiası yavaş gelişir; ortalama dizilimi haftalarca sürer. |
| Dönüş | olay (`rsi_divergence`) | 15 | 1,16 | 2,52 | Uyumsuzluk 2 bar gecikmeyle teyit edilir, dönüş kırılımdan yavaş oturur. |
| Kırılım | olay (`range`) | 10 | 0,95 | 2,06 | Sınır aşımı ya hemen devam eder ya geri döner; uzun ufuk iddiayı sulandırır. |
| Sıkışma | durum, nötr (`bandwidth`) | 20 | (1,34) | 2,91 | Sıkışma çözülmesi gecikebilir; ölçü "olağan" oynaklık (ATR100), çünkü ATR14 sıkışmada zaten bastırılmıştır. |
| Aşırılık | durum (`rsi`) | 10 | 0,95 | 2,06 | RSI bölgeleri kısa ömürlü. |
| Akış | olay (`volume_surge`, `institutional`) | 10 | 0,95 | 2,06 | Para akışının fiyata yansıması kısa vadeli bir iddia. |

Zaman türü ufku değiştirmez: durum türleri tabloda zaten uzun ufuk aldı; bugün beklenti
üreten dedektör yok, gelirse türünün satırını kullanır.

### Neden böyle

1. **Tek taban oran.** Simetrik bariyerde sürüklenmesiz bir piyasa, karar verilen
   durumların %50'sinde "gerçekleşti" verir — her tür için. Karne cümlesi ("geçmişte 100
   benzer durumun 62'sinde fiyat, aynı mesafe kadar geri gitmeden önce 1,3 ATR yükseldi, en
   çok 20 bar içinde") tek bir tabana karşı okunur. Asimetrik bariyer (ör. hedef 2, durak 1)
   tabanı türe göre %33'e çeker ve "62" ile "38"in hangisinin iyi olduğu okuyana kalır.
2. **Ölçek oynaklığa bağlı, fiyata değil.** Yüzde eşik aynı hikâyeyi sakin hissede zor,
   oynak hissede kolay kılar; ATR enstrümanlar ve dönemler arası karşılaştırılabilir yapar.
3. **Bariyer ufka bağlı (`∝ √h`).** Bariyer ufka göre küçükse hüküm 1–2 barlık gürültüyle
   verilir (sinyal yok); büyükse çoğu hikâye süresi dolar (karne ıskalarla dolar). `√h`
   ölçeklemesi her türde süresi dolan payını yaklaşık sabit tutar.
4. **Kapanışla.** Kesin barın kapanışı otoritenin kesinleştirdiği değerdir (K28); fitil
   gürültüsüne ve aynı barda iki bariyere birden değme belirsizliğine kapalıdır.
5. **Nötr iddia da çürütülebilir.** Başvuru kuralında sıkışma "|hareket| ≥ 1 ATR14" ile
   %95–100 gerçekleşiyordu (sıkışmada ATR14 küçük olduğu için neredeyse kesin): karnede
   bilgi yok. Burada ölçü olağan oynaklık (ATR100) ve `k_n` öyle seçildi ki rastgele bir
   barda genişleme olasılığı ~%50 olsun; sıkışmanın karnesi ancak gerçekten genişleme
   getiriyorsa tabanın üstüne çıkar.

### Sabitlerin seçimi (`examples/calibrate.rs`)

Sabitler **yalnız sentetik pazarda**, hikâyeden ve dedektörden bağımsız seçildi: her bar
(≥ 120) bir "doğum" sayıldı, `c ∈ {0,25 … 0,4}` (yönlü) ve `{0,5 … 0,8}` (nötr) tarandı.
Ölçüt: yönlüde süresi dolan payı ~%10, nötrde genişleme ~%50.

| Sentetik, koşulsuz | h = 10 | h = 15 | h = 20 |
|---|---|---|---|
| Yönlü c = 0,3: süresi dolan | 0,109 | 0,097 | 0,093 |
| Nötr c = 0,6 / 0,7: genişleme | 0,507 / 0,431 | — | 0,558 / 0,479 |

İlk deneme (sentetikte ATR ≈ 1,14 × kapanıştan kapanışa σ ölçülüp saf rastgele yürüyüş
varsayımıyla `k = 0,5·√h`, `k_n = √h`) süresi dolan payını %27'ye çıkardı: sentetik ve gerçek
fiyat h bar ölçeğinde `√h` yürüyüşünden ~%25 yavaş yayılıyor (ortalamaya dönüş; gün içi
aralık ATR'yi şişiriyor). Bu yüzden sabitler varsayımdan değil ölçümden alındı. Aynı koşulsuz
oranlar gerçek veride (yalnız denetim, seçimde kullanılmadı): yönlü c = 0,3 süresi dolan
0,045–0,075 (günlük), 0,058–0,101 (saatlik); nötr h = 20, c = 0,65 genişleme ≈ 0,66 (günlük),
≈ 0,57 (saatlik).

## Ölçümler (düzenek `harness-sim m5`, başvuru tema + sıralayıcı)

| Veri | Kapanan | İsabet | Süresi dolan payı | maks \|p(100)−p(son)\| | maks \|p(200)−p(son)\| | Yön (gerçek) | Yalancı eş | Fark | İleriye bakma |
|---|---|---|---|---|---|---|---|---|---|
| sentetik s1 | 5181 | 0,469 | 0,080 | 0,146 | 0,096 | 0,469 | 0,450 | 0,020 | geçti |
| sentetik s2 | 5130 | 0,461 | 0,085 | 0,140 | 0,035 | 0,462 | 0,452 | 0,011 | geçti |
| sentetik s3 | 5034 | 0,454 | 0,086 | 0,059 | 0,064 | 0,455 | 0,457 | −0,002 | geçti |
| gerçek günlük | 2802 | 0,526 | 0,052 | 0,057 | 0,045 | 0,520 | 0,426 | **0,094** | geçti |
| gerçek saatlik | 5126 | 0,478 | 0,089 | 0,148 | 0,072 | 0,474 | 0,434 | **0,041** | geçti |

Duyarlılık (kendi çeşitlerim: ufuk ±%20, bariyer ±%10):

| Veri | Genel oran değişimi (h−20 / h+20 / k−10 / k+10) | Hüküm uyuşması (en düşük–en yüksek) | Kapsama (en düşük) | Anahtar başına en büyük \|değişim\| |
|---|---|---|---|---|
| sentetik (3 tohum) | −0,023…−0,018 / +0,012…+0,018 / +0,010…+0,017 / −0,012…−0,009 | 0,954–0,980 | 0,956 | 0,130 (s2, h+20, küçük anahtar) |
| gerçek günlük | −0,016 / +0,016 / +0,011 / −0,010 | 0,955–0,976 | 0,959 | 0,113 (k+10) |
| gerçek saatlik | −0,032 / +0,015 / +0,009 / −0,017 | 0,945–0,972 | 0,969 | 0,104 (h−20) |

İleriye bakma: 3 kesimde (T = %25/50/75) farklı olay 0, farklı hüküm 0, karnesi farklı anahtar
0, koruma ihlali 0 — sentetik ve gerçek. Birim testleri: el hesabı hedef/durak/genişleme/
genişlemesizlik, ısınma, ilk geçiş (çağrı atlansa da), çeşit ölçekleri, `judge`'ın `now`
sonrası barlardan bağımsızlığı, motor düzeyinde gelecek karıştırılınca kesim öncesi karne ve
olayların bayt eşitliği (`cargo test`, 9 test).

Anahtar başına (gerçek, toplu): günlükte bütün yönlü anahtarlar 0,50–0,54 (kırılım 0,519,
devam 0,537, akış/hacim 0,532, uyumsuzluk 0,508, aşırılık 0,500); saatlikte 0,46–0,49.
Sıkışma 0,680 (günlük, n = 97; koşulsuz taban ≈ 0,66) ve 0,572 (saatlik, n = 166; taban
≈ 0,57). Sentetikte sıkışma 0,35–0,46 (koşulsuz taban ~0,52): sentetik sıkışmalar genişleme getirmiyor.

### Başvuru kuralıyla karşılaştırma (aynı düzenek)

| | barrier | başvuru `atr_barrier` |
|---|---|---|
| Gerçek − yalancı (günlük / saatlik) | 0,094 / 0,041 | 0,106 / 0,033 |
| Süresi dolan payı (günlük / saatlik) | 0,052 / 0,089 | 0,045 / 0,082 |
| Hüküm uyuşması (gerçek) | 0,945–0,976 | 0,952–0,979 |
| Sıkışma isabeti (günlük / saatlik) | 0,680 / 0,572 (taban ≈ 0,66 / 0,57) | 1,000 / 0,988 (bilgi yok) |

Yönlü hikâyelerde iki kural yakın (fark ölçümün gürültüsü içinde: n ≈ 2.700 için oran
farkının standart hatası ~0,02). Ayrışma nötr hikâyede ve okunabilirlikte: barrier'da her
türün sayısı tek bir %50 tabanına karşı okunur, sıkışmanın karnesi bilgi taşır, ufuk türün
iddiasına göre değişir.

## Zayıflıklar (dürüstçe)

- **Anlamlılık sınırlı ve tanıma değil veriye bağlı.** Gerçek − yalancı farkı günlükte ~0,09,
  saatlikte ~0,04; sentetikte ~0 (beklenen: rastgele yürüyüşe yakın veri). Bu, tanımın
  yönü ödüllendirdiğini gösterir ama dedektörlerin piyasada güçlü bir kenarı olduğunu
  göstermez; yön oranları 0,50–0,54 bandında. Karne "çoğu zaman haklı" demeyecek; bu
  ürünün "fiyat tahmini değil" ilkesiyle uyumlu ama kullanıcıya düz bir tablo gösterir.
- **Fark, isabet tanımına gömülü süresi dolan payıyla seyrelir:** süresi dolan ıska
  sayıldığından gerçek isabetler ~%5–9 aşağı çekilir.
- **Kararlılık örneklem gürültüsüyle sınırlı:** küçük anahtarlarda (sıkışma, hacim
  sıçraması; n < 100) p(25) ile p(son) arası 0,1–0,35'e çıkıyor; bu tanımın değil n'in
  sorunu, ama karne erken dönemde oynak görünecek. Öneri: karne cümlesi n < 50'de sayı
  söylememeli (M2/M3 işi).
- **Sabitler sentetikte ayarlandı, gerçekte değil;** gerçek saatlikte koşulsuz süresi dolan
  payı (~%10) sentetikle aynı, günlükte daha düşük (~%5–7). Rejim değişen enstrümanlarda
  doğum ATR'si sonraki oynaklığı temsil etmeyebilir (oynak dönemden sakin döneme geçişte
  süresi dolan artar).
- **Aşırılık yönü tartışmalı.** Aşırı alım `up` taşıdığı için "gerçekleşti" = germe sürdü
  (devam); oysa okuyucu aşırılığı sıklıkla "dönecek" diye anlar. Simetrik bariyer sayesinde
  karne iki okumayı da verir (0,50 = %50 sürdü, %50 döndü), ama cümle hangi yönün sayıldığını
  açıkça söylemeli.
- **Durum bitişi yok sayılır.** `bull_end`/`overbought_end` hikâyeyi zayıflatır ama hükmü
  değiştirmez; sonuç yalnız fiyatla ölçülür. Durum türü için "koşul bozulana kadar" ömür
  (K27) bilinçli olarak bar ufkuyla değiştirildi (karşılaştırılabilir karne için).
- **Nötr hikâyede süresi dolma yok** (`no_expansion` geçersiz sayılır). Bu, "süresi dolan
  payı" ölçüsünü biraz iyileştirir; sıkışma hikâyesi sayısı az (gerçekte ~%3) olduğu için
  etkisi küçük, ama bilinmeli. Isınmamış ATR(100) ile doğan nötr hikâye (bar < 99) hükümsüz
  kalır ve süresi dolar.
- **Kırılımda seviye kullanılmıyor:** durak doğum kapanışından ölçülür, kırılan seviyeden
  değil; "kırılım geri alındı" iddiası doğrudan sınanmıyor.
- **Bariyer fitili görmez:** yalnız kapanış; gün içi dokunup dönen hareketler sayılmaz.
- Önbellek kural örneği boyunca büyür (kapanan hikâyeyi bildiren çağrı yok); düzenek
  ölçeğinde önemsiz, canlı otoritede sınır gerekir.
