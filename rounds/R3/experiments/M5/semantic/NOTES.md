# M5 aday "semantic" — Türün anlamı

Crate: `rounds/R3/experiments/M5/semantic/` (`m5_semantic`), giriş `outcome()` (ad
`semantic`) ve `perturbations()`. Düzenekte kayıtlı: `harness-sim m5 run --alt semantic`.

## Öncül

"Gerçekleşti" her tür için o türün kendi iddiasıdır (K30, `docs/CAPSULE.md` §4): kırılım
sınırın ötesinde tutunup uzar, dönüş önceki hareketi gerçekten tersine çevirir, sıkışma
genişlemeyle çözülür, aşırılık ortalamaya döner, devam sürer, akışı fiyat izler. Sonuç
kuralı fiyat tahmini değildir (NORTH_STAR): her hikâyenin kendi söylediği şeyin olup
olmadığını, açık bir geçersizlik koşuluyla, yalnız `bars[..=now]` ile ölçer; karne bu
oranı "geçmişte N seferin K'sında" diye gösterir.

İddia her zaman hikâyenin **yönüdür**; kanıt (seviye, bant, salınım noktaları) yalnız
çapa verir. Böylece yönü çevrilmiş hikâye karşıt iddia olarak yargılanır.

## Tanımlar

Gösterim: `d` = +1 (`up`), −1 (`down`); `c` = `now` kapanışı; `c0` = doğum barı kapanışı;
`atr` = doğum barında biten 14 barın ortalama gerçek aralığı (seri kısaysa en az 5 bar;
bar 0'ın TR'si `h − l`). Eşikler `atr` katıdır. Aynı barda önce geçersizlik denetlenir.
Kural durumsuzdur (önbellek yok); yalnız `s.facts[0]` (doğuran olgu), `s.story` ve
`bars[..=now]` okunur, daha uzun dilim verilse bile `now`'dan sonrası okunmaz.

| Tür (dedektör) | Gerçekleşti | Geçersiz | Ufuk |
|---|---|---|---|
| Kırılım (`breakout.range`; `level`) | `d·(c − c0) ≥ 1,0·atr`: sınırın ötesinde uzadı | `d·(c − level) < −0,25·atr`: kırılan aralığın içine geri kapandı (başarısız kırılım) | 10 |
| Dönüş (`reversal.rsi_divergence`; `p1_bar`, `p2_bar`, `p2_price`) | Kapanış "boyun çizgisini" geçti: `down` → `p1..=p2` barlarının en düşük dibi altı, `up` → en yüksek tepesi üstü (önceki hareketin son salınımı kırıldı) | `d·(c − p2_price) < −0,25·atr`: uyumsuzluğun ikinci tepesinin/dibinin ötesinde yeni uç (eski trend sürdü) | 20 |
| Sıkışma (`squeeze.bandwidth`; `upper`, `lower`) | Kapanış doğumdaki bandın `0,25·atr` dışında **ve** son 5 barın ortalama TR'si `≥ 1,0·atr` (oynaklık genişledi) | yok: genişlemeden sönen sıkışmanın süresi dolar | 30 |
| Aşırılık (`extreme.rsi`) | `d·(c − sma20(now)) ≤ 0,25·atr`: ortalamaya döndü | `d·(c − c0) ≥ 1,5·atr`: daha da uzadı | 15 |
| Devam (`continuation.ma_stack`) | `d·(c − c0) ≥ 1,5·atr`: hareket sürdü | `d·(c − sma50(now)) < −0,25·atr`: trend yapısı kırıldı | 20 |
| Akış (`flow.*`) | `d·(c − c0) ≥ 1,0·atr`: fiyat akışın yönünü izledi | `d·(c − çapa) < −0,25·atr`, çapa = doğum barının dibi (`up`) / tepesi (`down`): akış barı tümüyle geri alındı | 10 |
| Nötr yön (sıkışma dışı) ya da kanıt eksik | 1·atr barajı (nötr: `\|c − c0\| ≥ 1·atr`) | ters 1·atr (yalnız yönlü) | türün |

Kenar durumları: dönüşte boyun çizgisi doğumda zaten geçilmişse hedef `c0`'dan `1·atr`
ötesidir (yoksa iddia tespit anında bedavaya gerçekleşirdi). Ufuk zaman türüne değil türe
bağlıdır (dedektörlerin hepsi aynı türde tek zaman türü üretiyor).

## Parametreler ve gerekçe

- **Ölçek ATR, doğumda donar:** eşikler enstrümandan ve dönemden bağımsız olsun; doğumdan
  sonra değişmez, yani hüküm yalnız fiyatın yoluna bağlıdır. Wilder yerine basit ortalama:
  durumsuz ve O(14); çekirdekteki `Atr` ile bit eşitliği burada gerekmiyor.
- **Geçersizlik tamponu 0,25·atr:** yapısal çizginin (seviye, tepe, SMA50, akış barı ucu)
  tek bir gürültülü kapanışla delinmesini geçersiz saymamak için küçük, sabit tampon.
- **Uzama 1,0 (kırılım, akış) / 1,5 (devam):** devam iddiası zaten süren bir trende
  dairdir, "sürdü" demek için olağan bir barın ötesinde hareket gerekir.
- **Aşırılık 0,25 yakınlık / 1,5 uzama:** ortalamaya "dönmek" SMA20'ye bir tampon kadar
  yaklaşmaktır; RSI yerine fiyat, çünkü iddia fiyatın gerilmesine dairdir ve RSI'ı
  `bars[..=now]` üzerinden yeniden hesaplamak her yargıda O(n) olurdu.
- **Ufuklar:** olaylar kısa (10), dönüş ve devam yapı ister (20), sıkışma uzun sürer
  (dedektör 120 barlık en düşüğe bakar; 30), aşırılık arada (15).
- **Çeşitler (`perturbations()`):** `h-20%` / `h+20%` bütün ufuklar ×0,8 / ×1,2
  (yuvarlanır); `k-10%` / `k+10%` bütün eşikler ×0,9 / ×1,1.
- Parametreler veriye uydurulmadı. Ölçümden sonra yapılan tek değişiklik: ilk koşuda
  `born < 14` olan hikâyeler hiç yargılanmıyor, süresi dolup ıska yazılıyordu
  (`flow.institutional` bar 5'ten doğar); bu karnenin ilk satırlarını bozuyordu
  (p(25) 0,12–0,16). ATR penceresi kısa seride en az 5 bara indirildi; eşikler değişmedi.

## Sonuçlar (başvuru tema + sıralayıcı)

Tam tablolar: `rounds/R3/experiments/M5/raw/semantic/summary-{synthetic,real}.md`.

| Veri | Kapanan | İsabet | Süresi dolan | maks \|p(100)−p(son)\| | maks \|p(200)−p(son)\| | Gerçek − yalancı | Hüküm uyuşması (4 çeşit) | Kapsama | İleriye bakma |
|---|---|---|---|---|---|---|---|---|---|
| sentetik s1 | 5.045 | 0,480 | 0,103 | 0,090 | 0,045 | 0,172 | 0,95–0,97 | 0,97–0,99 | geçti (0 fark, 0 ihlal) |
| sentetik s2 | 4.990 | 0,471 | 0,111 | 0,053 | 0,078 | 0,165 | 0,95–0,97 | 0,97–0,99 | geçti |
| sentetik s3 | 4.915 | 0,466 | 0,115 | 0,103 | 0,061 | 0,156 | 0,95–0,97 | 0,97–0,99 | geçti |
| gerçek günlük | 2.718 | 0,500 | 0,088 | 0,107 | 0,062 | 0,223 | 0,96–0,98 | 0,97–1,00 | geçti |
| gerçek saatlik | 4.966 | 0,482 | 0,134 | 0,067 | 0,066 | 0,180 | 0,93–0,97 | 0,97–0,99 | geçti |

Karşılaştırma (README): başvuru `atr_barrier` sentetik gerçek − yalancı 0,006–0,019,
gerçek 0,106 (günlük) / 0,033 (saatlik); süresi dolan 0,08–0,09.

Anahtar başına (gerçek günlük isabet / yalancı eş isabeti): kırılım 0,49 / 0,20; devam
0,53 / 0,04; dönüş 0,44 / 0,08; hacim akışı 0,52 / 0,35; aşırılık 0,42 / **0,97**;
sıkışma 0,95 / — (nötr, yalancı eşte aynı). Sentetikte yalancı eş oranları neredeyse
aynı: kırılım 0,21–0,22, devam 0,03–0,05, dönüş 0,05–0,11, akış 0,32–0,40, aşırılık
0,93–0,95.

## Zayıflıklar (dürüstçe)

1. **Gerçek − yalancı farkının büyük kısmı yapısaldır, öngörü değildir.** Rastgele yürüyüşte
   (sentetik) gerçek bir anlam farkı ~0 olmalı; başvuru öyle (0,01), bu kural 0,16 veriyor.
   Neden: yalancı eş yalnız yönü çevirir, kanıtı (seviye, tepe) çevirmez. "Aşağı kırılım"
   etiketli bir yukarı kırılımın seviyesi fiyatın altındadır; kural onu ilk barda
   "aralığın içine döndü" diye geçersiz sayar (devam ve dönüşte de aynı: SMA50 ve ikinci tepe
   yanlış taraftadır). Bu tutarlı bir yargıdır (iddia kanıtıyla çelişiyor) ama anlamlılık
   ölçüsünü şişirir. Kaba düzeltme: gerçek veride fark sentetik yapısal farkın ~0,06
   (günlük) / ~0,02 (saatlik) üstünde; başvurunun karşılığı ~0,09 / ~0,02. Yani bu
   ölçüyle başvurudan **daha anlamlı olduğu gösterilmedi**.
2. **Aşırılıkta yalancı eş %93–97 tutar:** ters iddia "aşırı satımdan ortalamaya dön"
   der, fiyat zaten ortalamanın üstündedir, ilk barda gerçekleşir. Bu türün iddiası yönün
   *tersine* olduğu için yön çevirme testi burada anlamsızlaşır; aşırılık bu ölçüde farkı
   eksiye çeker.
3. **Sıkışma çok kolay gerçekleşir (0,77–0,95):** geçersizlik yok; oynaklığın 120 barın
   en düşüğünden ortalamaya dönmesi neredeyse kesin. Karne doğru ama az bilgi taşır;
   yön iddiası olmadığı için anlamlılık testine girmez. Sertleştirmek (ör. genişlemenin
   bant yönünde 1·atr sürmesi) yeni bir eşik demek; denenmedi.
4. **Dönüşte boyun çizgisi uzak olabilir** (40 barlık uyumsuzlukta birkaç atr): süresi
   dolan payı orada yüksek; 20 bar ufuk bir seçimdir.
5. **Akışta çapa barın aralığıdır:** hacim sıçramasının yönü `c − o` olduğu için kapanış
   barın yön tarafına yakındır; ters iddianın çapası kapanışa yakın düşer, bu da yalancı eşi
   biraz cezalandırır (1. maddenin hafif biçimi).
6. **Kararlılık:** p(25) sapması 0,16–0,23 (küçük örneklem, özellikle seyrek anahtarlar);
   p(100) sonrası 0,05–0,11, başvuruyla aynı düzeyde. Süresi dolan payı (0,09–0,13)
   başvurudan biraz yüksek; saatlikte en yüksek.
7. **Hikâyeye sonradan katılan olgular yok sayılır** (yalnız doğuran olgu çapadır);
   `state_ended` gibi geçişler hükmü etkilemez. Tema kuralı değişirse (M4) davranış aynı
   kalır, bu bilinçli.
8. Tanımlar dedektör kanıt anahtarlarına (`level`, `p1_bar`, `p2_bar`, `p2_price`, `upper`,
   `lower`) bağlıdır; anahtar yoksa genel 1·atr barajına düşer. Yeni dedektör eklenince
   türün tanımı kanıt sözleşmesiyle birlikte gözden geçirilmeli.

## Testler

`cargo test` (9): tür başına elle kurulmuş seriler (gerçekleşti / geçersiz / henüz değil
ve kod), çeşit adları ve ufukları, yön çevrilince karşıt hüküm, ATR penceresi yetmezse hüküm
yok, **ileriye bakma yok**: her `now` için `bars[..=now]` ile tam seri aynı hükmü verir;
ilk 40 barı ortak iki seride `now < 40` hükümleri eşit; çağrılar iç içe geçince değişmez
(durumsuz). Düzenekte: koruma ihlali 0, üç kesimde fark 0 (sentetik ve gerçek).
