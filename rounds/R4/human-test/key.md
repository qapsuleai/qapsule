# Cevap anahtarı (yürütücü; katılımcı görmez)

`node prepare.mjs key` üretti (sentetik pazar, `qp demo --synthetic`). Kurallar `SCORING.md`. Satır = akışın yukarıdan kaçıncı kapsülü (0 = en üstteki; başlık satırları sayılmaz).

## P0 — adım 1, satır 0 (alıştırma, puanlanmaz)

SYN28 · SYN28 1 bardır kısa ortalamanın (95,70) altında; şu an 93,42. Düşüş eğilimi sürüyor. 1 bar önce hacim de teyit etti; 2 bağımsız kanıt ailesi destekliyor. Geçmişte bu sinyal 54 seferin 30'unda beklendiği gibi sonuçlandı.

| S1 yön | S1b ne | S2 ne zaman | S3 geçerli mi | S4 güven | S5 |
|---|---|---|---|---|---|
| aşağı | hareket sürüyor | birkaç gün önce | evet, sürüyor | yarı yarıya (30/54) | hiçbir şey, yalnız bilgi veriyor |

## G5-1 — adım 1

| sıra | satır | kapsül | sonuç (G4) | yön | ne | geçerli mi |
|---|---:|---|---|---|---|---|
| 1 | 37 | SYN25 (continuation.down, 8/17) | tutmadı | aşağı | hareket sürüyor | kapandı: tutmadı |
| 2 | 38 | SYN18 (continuation.up, 10/16) | tutmadı | yukarı | hareket sürüyor | kapandı: tutmadı |
| 3 | 39 | SYN21 (continuation.down, 17/28) | gerçekleşti | aşağı | hareket sürüyor | kapandı: gerçekleşti |

## G5-2 — adım 201

| sıra | satır | kapsül | sonuç (G4) | yön | ne | geçerli mi |
|---|---:|---|---|---|---|---|
| 1 | 37 | SYN29 (breakout.up, 21/58) | gerçekleşti | yukarı | fiyat önemli bir seviyeyi geçti | kapandı: gerçekleşti |
| 2 | 38 | SYN11 (breakout.up, 50/122) | hâlâ açık | yukarı | fiyat önemli bir seviyeyi geçti | evet, sürüyor |
| 3 | 39 | SYN21 (breakout.up, 6/23) | tutmadı | yukarı | fiyat önemli bir seviyeyi geçti | kapandı: tutmadı |

## G1-1a — adım 1, satır 1

SYN25 · SYN25 93 bar önce 178,70'teki desteği aşağı kırdı (kapanış 177,45): aşağı kırılım. Sinyal 90 bar önce geçersiz oldu; artık geçerli değil. Karnesi henüz az örnekli: 5 seferin 4'ünde tuttu.

| S1 yön | S1b ne | S2 ne zaman | S3 geçerli mi | S4 güven | S5 |
|---|---|---|---|---|---|
| aşağı | fiyat önemli bir seviyeyi geçti | bir haftadan önce | kapandı: tutmadı | az örnek var, bilinmiyor (4/5) | hiçbir şey, yalnız bilgi veriyor |

_S8 #6 ile aynı_

## G1-1b — adım 46, satır 2

SYN01 · SYN01 bu barda 4,83'teki desteği aşağı kırdı (kapanış 4,62): aşağı kırılım. Bu barda hacim de teyit etti; 2 bağımsız kanıt ailesi destekliyor. Hâlâ geçerli. Geçmişte bu sinyal 198 seferin 106'sında beklendiği gibi sonuçlandı.

| S1 yön | S1b ne | S2 ne zaman | S3 geçerli mi | S4 güven | S5 |
|---|---|---|---|---|---|
| aşağı | fiyat önemli bir seviyeyi geçti | birkaç gün önce | evet, sürüyor | yarı yarıya (106/198) | hiçbir şey, yalnız bilgi veriyor |

## G1-1c — adım 64, satır 2

SYN19 · SYN19 2 bar önce 29,21'deki desteği aşağı kırdı (kapanış 28,69): aşağı kırılım. Beklenen sonuç bu barda gerçekleşti. Geçmişte bu sinyal 198 seferin 106'sında beklendiği gibi sonuçlandı.

| S1 yön | S1b ne | S2 ne zaman | S3 geçerli mi | S4 güven | S5 |
|---|---|---|---|---|---|
| aşağı | fiyat önemli bir seviyeyi geçti | dün | kapandı: gerçekleşti | yarı yarıya (106/198) | hiçbir şey, yalnız bilgi veriyor |

## G1-2a — adım 100, satır 7

SYN12 · SYN12 1 bar önce 20,25'teki desteği aşağı kırdı (kapanış 19,78): aşağı kırılım. 1 bar önce hacim de teyit etti; 2 bağımsız kanıt ailesi destekliyor. Artık geçerli değil. Geçmişte bu sinyal 203 seferin 110'unda beklendiği gibi sonuçlandı.

| S1 yön | S1b ne | S2 ne zaman | S3 geçerli mi | S4 güven | S5 |
|---|---|---|---|---|---|
| aşağı | fiyat önemli bir seviyeyi geçti | birkaç gün önce | hayır, artık geçerli değil | yarı yarıya (110/203) | hiçbir şey, yalnız bilgi veriyor |

## G1-2b — adım 163, satır 1

SYN02 · SYN02 10 bar önce 134,14'teki desteği aşağı kırdı (kapanış 122,15): aşağı kırılım. Sinyalin 10 barlık ömrü bu barda sonuçsuz doldu; artık geçerli değil. Geçmişte bu sinyal 201 seferin 108'inde beklendiği gibi sonuçlandı.

| S1 yön | S1b ne | S2 ne zaman | S3 geçerli mi | S4 güven | S5 |
|---|---|---|---|---|---|
| aşağı | fiyat önemli bir seviyeyi geçti | dün | kapandı: süresi doldu ya da geri çekildi | yarı yarıya (108/201) | hiçbir şey, yalnız bilgi veriyor |

## G1-2c — adım 334, satır 4

SYN20 · SYN20 2 bar önce 201,61'deki desteği aşağı kırdı (kapanış 192,61): aşağı kırılım. Sinyal bu barda geçersiz oldu; artık geçerli değil. Geçmişte bu sinyal 218 seferin 115'inde beklendiği gibi sonuçlandı.

| S1 yön | S1b ne | S2 ne zaman | S3 geçerli mi | S4 güven | S5 |
|---|---|---|---|---|---|
| aşağı | fiyat önemli bir seviyeyi geçti | dün | kapandı: tutmadı | yarı yarıya (115/218) | hiçbir şey, yalnız bilgi veriyor |

## G3-1a — adım 334, satır 5

SYN25 · SYN25 63 bar önce 173,47'deki desteği aşağı kırdı (kapanış 160,80): aşağı kırılım. Beklenen sonuç 61 bar önce gerçekleşti. Geçmişte bu sinyal 104 seferin 54'ünde beklendiği gibi sonuçlandı.

İşaretli yer: grafikteki **vurgulu sütun** (olgunun barı) ve üstündeki işaret; cümlede seviye varsa o seviye. Aynı şey mi: **evet**. Hâlâ geçerli mi: **kapandı: gerçekleşti**.

## G3-1b — adım 334, satır 7

SYN23 · SYN23 50 bar önce 7,18'deki desteği aşağı kırdı (kapanış 7,16): aşağı kırılım. Sinyal 41 bar önce geçersiz oldu; artık geçerli değil. Geçmişte bu sinyal 104 seferin 51'inde beklendiği gibi sonuçlandı.

İşaretli yer: grafikteki **vurgulu sütun** (olgunun barı) ve üstündeki işaret; cümlede seviye varsa o seviye. Aynı şey mi: **evet**. Hâlâ geçerli mi: **kapandı: tutmadı**.

## G3-2a — adım 469, satır 7

SYN13 · SYN13 bu barda 127,97'deki desteği aşağı kırdı (kapanış 116,97): aşağı kırılım. Bu barda hacim de teyit etti; 2 bağımsız kanıt ailesi destekliyor. Hâlâ geçerli. Geçmişte bu sinyal 236 seferin 124'ünde beklendiği gibi sonuçlandı.

İşaretli yer: grafikteki **vurgulu sütun** (olgunun barı) ve üstündeki işaret; cümlede seviye varsa o seviye. Aynı şey mi: **evet**. Hâlâ geçerli mi: **evet, sürüyor**.

## G3-2b — adım 469, satır 6

SYN02 · SYN02 45 bar önce 148,30'daki desteği aşağı kırdı (kapanış 134,97): aşağı kırılım. Beklenen sonuç 44 bar önce gerçekleşti. Geçmişte bu sinyal 199 seferin 107'sinde beklendiği gibi sonuçlandı.

İşaretli yer: grafikteki **vurgulu sütun** (olgunun barı) ve üstündeki işaret; cümlede seviye varsa o seviye. Aynı şey mi: **evet**. Hâlâ geçerli mi: **kapandı: gerçekleşti**.

## G4-1 — adım 76

| sıra | satır | kapsül | sonuç (G4) | yön | ne | geçerli mi |
|---|---:|---|---|---|---|---|
| 1 | 37 | SYN21 (breakout.up, 6/23) | tutmadı | yukarı | fiyat önemli bir seviyeyi geçti | kapandı: tutmadı |
| 2 | 38 | SYN23 (breakout.up, 23/72) | gerçekleşti | yukarı | fiyat önemli bir seviyeyi geçti | kapandı: gerçekleşti |
| 3 | 39 | SYN08 (breakout.up, 32/91) | gerçekleşti | yukarı | fiyat önemli bir seviyeyi geçti | kapandı: gerçekleşti |

## G4-2 — adım 351

| sıra | satır | kapsül | sonuç (G4) | yön | ne | geçerli mi |
|---|---:|---|---|---|---|---|
| 1 | 37 | SYN14 (breakout.down, 30/58) | gerçekleşti | aşağı | fiyat önemli bir seviyeyi geçti | kapandı: gerçekleşti |
| 2 | 38 | SYN05 (breakout.up, 42/103) | tutmadı | yukarı | fiyat önemli bir seviyeyi geçti | kapandı: tutmadı |
| 3 | 39 | SYN18 (breakout.up, 18/54) | tutmadı | yukarı | fiyat önemli bir seviyeyi geçti | kapandı: tutmadı |

## G2-1 — son bakış adım 600, bakış adım 609; elde: SYN09, SYN12

- **G2a ne değişti:** SYN09 yukarı
- **G2b olumsuz:** SYN09 — tutmadı, geçersiz oldu
- **G2c en güvenilir** (n ≥ 20, kova ≥ %60; 1 kapsül doğru): SYN22 20/33
- akışın ilk satırları (`elikov`): # Elindeki hisselerde yeni · cap.SYN09.flow.up.183 [yeni · tutmadı] · # Diğerleri, geçmişte en çok tutan önce · cap.SYN22.continuation.down.108 · cap.SYN22.continuation.down.90 · cap.SYN26.breakout.down.47 · cap.SYN26.breakout.down.46 · cap.SYN28.continuation.down.118
- akışın ilk satırları (`vakiro`): # Son baktığından beri · dün 15:00 · cap.SYN09.flow.up.183 [pozisyonun] · # Son 7 gün · cap.SYN02.squeeze.neutral.184 · # Daha önce · cap.SYN25.breakout.down.26 · cap.SYN02.breakout.down.127 · cap.SYN12.breakout.down.129 [pozisyonun]

## G2-2 — son bakış adım 882, bakış adım 891; elde: SYN19, SYN01

- **G2a ne değişti:** SYN01 yukarı
- **G2b olumsuz:** SYN01 — tutmadı, geçersiz oldu
- **G2c en güvenilir** (n ≥ 20, kova ≥ %60; 1 kapsül doğru): SYN22 20/33
- akışın ilk satırları (`elikov`): # Elindeki hisselerde yeni · cap.SYN01.continuation.up.208 [yeni · tutmadı] · # Diğerleri, geçmişte en çok tutan önce · cap.SYN22.continuation.down.108 · cap.SYN22.continuation.down.90 · cap.SYN26.breakout.down.47 · cap.SYN26.breakout.down.46 · cap.SYN28.continuation.down.118
- akışın ilk satırları (`vakiro`): # Son baktığından beri · dün 18:00 · cap.SYN01.continuation.up.208 [pozisyonun] · # Son 7 gün · cap.SYN05.breakout.down.211 · # Daha önce · cap.SYN25.breakout.down.26 · cap.SYN02.breakout.down.127 · cap.SYN19.breakout.down.124 [pozisyonun]

