# Puanlama

```sh
node prepare.mjs key                                    # anahtar (oturumlardan önce): key.json, key.md, links.html
python3 -I score.py answers.csv > results.md            # puan + karar (anahtar key.json, plan plan.json)
python3 -I score.py answers.example.csv                 # örnek (uydurma) → answers.example.scored.md
python3 -I score.py --sequence 10                       # sıra tablosu
python3 -I make_example.py                              # örnek CSV'yi yeniden üret (tohumlu)
```

`score.py` yalnız Python standart kitaplığıdır; aynı girdi → bayt düzeyinde aynı rapor (denendi).
`prepare.mjs` aynı plan + aynı gösterim için bayt eşit `key.json` verir (iki koşu `cmp` ile denendi).

## Anahtar: gösterimin hangi anında hangi kapsül

Anahtar motorun durumundan kurulur, tasarımdan değil. `plan.json` her madde için gösterimin
**adımını** (oynatma çubuğuna yazılan sayı, sentetik pazar, tohum 1, 30 enstrüman) ve akıştaki
**satırı** (yukarıdan sıra, 0 tabanlı; eksi = sondan) ve beklenen kapsül kimliğini verir.
`prepare.mjs key` gösterimi (`qp demo --synthetic --paused`) başsız Chromium'da telefon
boyutunda açar, oturumu yürütücünün sırasıyla oynar (URL ayarları, Git, G2'de Ayarlar'dan
kitle ve `qp.seen()`), satırları çekirdekten (`window.qp.feed()`) ve kapsül durumunu kabuğun
aldığı protokol iletilerinden okur. Satırda beklenen kapsül yoksa betik durur.

Akış sırası yüze, alfabeye, geçişe ve sonuç işaretine bağlı değildir (yalnız akış düzenine ve
kitleye); bu yüzden tek anahtar bütün katılımcılara geçer. G1/G3/G4/G5'te akış düzeni sabit
`r3`'tür (M8 gibi "akış düzeni bu görevde sınanmıyor"); G2'de her iki düzen ayrı ayrı oynanır ve
anahtarın iki düzende aynı olduğu denetlenir.

Kurallar (R4 düzeneğinin S8 kuralları, SPEC §2 ve E1 ile aynı; `prepare.mjs` başı):

| Soru | Doğru |
|---|---|
| S1 yön | kapsülün yönü (yukarı / aşağı / belirsiz) |
| S1b ne | türün sade karşılığı (kırılım → "fiyat önemli bir seviyeyi geçti", dönüş → "yön değişiyor", devam → "hareket sürüyor", sıkışma → "fiyat sıkıştı, hareketsiz", aşırılık → "aşırı gitti", akış → "büyük alım ya da satım var") |
| S2 ne zaman | son geçişin barının yerel günü ile oynatma saatinin yerel günü arasındaki takvim günü (UTC+3): 0 bugün, 1 dün, 2–6 birkaç gün önce, ≥ 7 bir haftadan önce |
| S3 hâlâ geçerli | geçerli → "evet, sürüyor"; artık değil → "hayır, artık geçerli değil"; kapandı + gerçekleşti / tutmadı / diğer (süresi doldu, geri çekildi) |
| S4 güven | doğuştaki karne: n < 20 → "az örnek var, bilinmiyor" (ürünün eşiği, E1); yoksa oran ≥ %60, %40–60, < %40 |
| S5 | her zaman "hiçbir şey, yalnız bilgi veriyor" |
| G4 sonuç | kapandı + gerçekleşti → gerçekleşti; kapandı + tutmadı → tutmadı; açık → hâlâ açık |
| G3 yer | grafikte vurgulu sütun (olgunun barı) ± 1 bar; yürütücü oturumda işaretler |
| G3 aynı şey | evet |
| G2a ne değişti | son bakış (t0) ile bakış (t1) arasında kitledeki bir enstrümanın kapsülünde aşama, geçerlilik ya da son bar değişti **ve** kapsül t1'de akışta görünüyor; "enstrüman yön" çiftleri |
| G2b olumsuz | elde tutulan enstrümanda t0–t1 arası zayıfladı / tutmadı / geri çekildi; hangisi ve ne |
| G2c en güvenilir | akıştaki kapsüllerden n ≥ 20 olanların en yüksek oran kovasındakiler (hepsi doğru) |

Denetim: G1-1a (`cap.SYN25.breakout.down.26`, adım 1) R4 düzeneğinin S8 #6 anahtarıyla aynı
çıkar (key.json `s8_check`); G2-1 (adım 600–609, M9 senaryosunun kitlesi) M9 anahtarını verir
(G1 SYN09 yukarı, G2 SYN09 tutmadı; G3 E1 eşiğiyle SYN22 20/33).

## Puanlar

- Boş, tanınmayan (`?`) ya da ipucuyla verilen cevap yanlış.
- **G1:** yüz başına tam doğru (S1–S5 hepsi). Alan başına ve "S2+S3" (zaman ve geçerlilik) ayrıca.
- **G5:** işaret başına tam doğru (yön + ne + geçerli mi).
- **G4:** işaret başına sonuç doğru.
- **G3:** kapsül başına (yer, aynı şey, hâlâ geçerli) ortalaması; süre (sn) ve "kayboldum" ayrıca.
- **G2:** pencere başına (G2a F1, G2b hangisi + ne, G2c) ortalaması; G2a yalnız enstrüman
  düzeyinde F1 de raporlanır (M9 öneri 1: yön yorumu tasarımı ölçmeyebilir).
- Katılımcı başına tasarım puanı = o tasarımın maddelerinin ortalaması; **eşli fark** A − B.
  Ağırlık genç 2, alışkın 1 (persona F 2, H 1 ile aynı). İşaret testi (kesin binom, eşitler
  atılır) bilgi içindir: 5–10 kişide güç çok düşüktür (n = 6'da yalnız 6/6 p < 0,05).
- Tercih: "1/2" katılımcının sırasından tasarıma çevrilir.
- Karar: `DECISION.md`, raporun "Karar" bölümünde otomatik uygulanır.

## Sınırlar (bilinerek)

- **Sentetik akış tekdüze ve durağan.** Taban akışın (`r3`) üst satırları adım 217'den sonra
  yüzlerce adım değişmez; kapsüllerin çoğu aşağı kırılımdır. G1'de S1 (hep aşağı) ve S1b (hep
  kırılım) ayırt etmez; G1'i asıl S2, S3, S4, S5 ayırır. G5'te türler devam ve kırılım, yönler
  ikisi de var.
- **Set ve sıra karışık.** 1. set hep önce gelir (geri sarma kitleyi sildiği için ileri giden
  oturum); hangi tasarımın 1. sette olduğu dengelenir. Set zorluğu ile sıra etkisi ayrılamaz,
  ikisi de tasarımlar arasında dengelenir; rapor sıra etkisini ayrıca verir.
- **G1 metin içerir.** Kapalı yüz cümle taşır; G4/G5 bu yüzden metin örtüsüyle yapılır (işaret
  tek başına, gerçek boyutta). Örtü elle tutulduğu için yürütücü notu önemlidir.
- **S2 takvim günüdür.** "Bu barda" diyen bir kapsül pazartesi sabahı cuma barını anlatır; anahtar
  "birkaç gün önce" der (S8 #2 ile aynı kural). Katılımcının "dün"ü raporda yanlış görünür;
  yüzün süre ifadesi takvime göre yazılıyorsa (GOAL sınırı) bu tutarlıdır.
- **G2c iki pencerede aynı** (SYN22 20/33, akışın üstünde durağan). İkinci pencerede öğrenilmiş
  olabilir; sıra dengesi bunu tasarımlar arasında eşitler, rapordaki sıra etkisine bakın.
- **G2'de olumsuz haber akışta görünmeyebilir.** Arama sırasında pek çok pencerede elde tutulan
  hissenin yeni doğup aynı anda geçersizleşen kapsülü 40 satırlık okuyucu akışına girmedi; plan
  bu yüzden ikinci pencere için kitleyi değiştirir (README "Açık sorular").
