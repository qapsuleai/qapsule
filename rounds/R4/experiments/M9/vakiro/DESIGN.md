# M9 alternatifi `vakiro` — zamana göre bölümlü akış

Kod: `crates/qp-grammar/src/feeds/vakiro.rs` (kayıt: `feeds.rs` `LAYOUTS`, tek satır).
`rules = 4`. Okuyucuya gitmez.

## Amaç

Okuyucu akışı açtığında ilk ekranda **nerede kaldığını** ve **neyin yeni olduğunu** görsün
(SPEC §3 G1, G2): akış en yeniden eskiye, gündelik zaman bölümleriyle ayrılır. Bölüm başlığı
"ne zaman"ı söylediği için bölümün içinde sıra yeniden zaman değil, otoritenin önem sırasıdır
(G3: en önemli/güvenilir kapsül eski bölümün başına gelir).

## Ne gösterir / ne göstermez

- Gösterir: son bakıştan beri bilinir olan kapsüller en üstte, son bakışın ne zaman olduğu
  başlıkta ("dün 15:00"); kalan kapsüller takvim gününe göre bölümlerde; okuyucunun
  pozisyonu/niyetiyle ilgili kapsülde kısa işaret.
- Göstermez: yeni bir sıralama ölçüsü (önem, güven) icat etmez; kapsülü süzmez, gizlemez;
  grup açma/kapama yok (gereken dokunma 0); "yeni" işareti yok (başlık söyler).

## Yerleşim (yukarıdan aşağı)

1. **Son bakış bölümü** (yalnız son bakış varsa): başlık + `fresh` olan kapsüller (çekirdeğin
   `known_at` kuralı), otorite sırasıyla. Yeni bir şey yoksa başlık bunu söyler ve tek başına
   durur.
2. **Takvim bölümleri** (kalanlar; son geçişin barının zamanı `CapsuleItem.t` — yüzün süre
   ifadesinin okuduğu zaman — ile şimdinin yerel günü arasındaki fark): `0` Bugün, `1` Dün,
   `2–7` Son 7 gün, `> 7` Daha önce. Saat yoksa (`now` yok) hepsi "Daha önce".
3. Boş bölümün başlığı yazılmaz. Bölüm içi sıra: `capsules` sırası (önem, son geçiş, kimlik).
4. `relevant` kapsülün işareti: "pozisyonun" / "your position".

Yerel gün: borsanın dilimi `Europe/Istanbul`, 2016'dan beri sabit UTC+03:00 (yaz saati yok);
kabuk çekirdeğinin gösterim dilimiyle aynı (`qp_wasm::shell::DISPLAY_TZ`). `qp-grammar`
`qp-calendar`'a bağlı olmadığı için (bağımlılık eklemek başka dosyaya dokunmak olurdu) ofset
sabit olarak yazıldı; tamsayı aritmetiği.

Kurallar (4): son bakış bölümü; takvim bölümleri; pozisyon işareti; dilim ofseti.

## Metin şablonları

| Öğe | TR | EN |
|---|---|---|
| Son bakış (yeni var) | Son baktığından beri · dün 15:00 | Since your last look · yesterday 15:00 |
| Son bakış (yeni yok) | Son baktığından beri yeni bir şey yok · bugün 10:30 | Nothing new since your last look · today 10:30 |
| Son bakış zamanı | bugün HH:MM / dün HH:MM / salı HH:MM (2–6 gün) / 9 gün önce | today / yesterday / Tuesday HH:MM / 9 days ago |
| Bölümler | Bugün · Dün · Son 7 gün · Daha önce | Today · Yesterday · Last 7 days · Earlier |
| İşaret | pozisyonun | your position |

## Düzenekte görülen (2026-11-16, M9 senaryosu t0 = adım 600, t1 = adım 609)

- `m9 --feed vakiro`: 6 ekran (sınır), 40 kapsül, 3 başlık ("Son baktığından beri · dün
  15:00" 1 kapsül, "Son 7 gün" 1, "Daha önce" 38). `DRY_RUN=1 run_readers.sh` F ve H: 6'şar
  istem, 6 görüntü.
- Anahtardan en az kaydırma: **G1 0, G2 0** (olumsuz geçişli SYN09 ekran 1'de, ilk kapsül,
  "pozisyonun" işaretli), **G3 1** (iki doğru kapsülden SYN25 ekran 2'de, "Daha önce"nin ilki).
- Karşılaştırma (aynı düzenek, `r3`, ayrı kökte): G1 ve G2'nin kapsülü 6 ekranın hiçbirinde
  yok (en az kaydırma tanımsız), G3 0.
- İlk sürüm bölüm içinde de "en yeni önce" sıralıyordu (5 kural): G3'ün kapsülleri 6 ekranın
  dışına düştü; bölüm içi otorite sırasına dönüldü (4 kural).

## Bilinen zayıflıklar

- G3 bir kaydırma ister: üstteki iki bölüm başlığı ve iki kapsül ilk ekranı doldurur.
- "Daha önce" bu senaryoda 38 kapsüllük tek uzun bölüm; zaman bilgisi orada ayırt etmez.
- `relevant` niyet kapsüllerini de kapsar; "pozisyonun" işareti niyette tam doğru değil.
- Bölüm geçişin **barının** zamanına göredir: günlük seride dünkü barın bugün bilinen geçişi
  (son bakıştan önce bilinmişse) "Dün"e düşer — yüzün süre ifadesiyle tutarlı, bilinme anıyla
  değil.
- Yerel gün sabit +03:00; başka dilimli borsa ya da gelecekte yaz saati gelirse yanlış olur.
- "Son 7 gün" kayan penceredir (takvim haftası değil): pazartesi cuma kapsülleri "Dün"e değil
  "Son 7 gün"e düşer.
- Yüz metni (ör. "3 bar önce") düzenin değil yüzün işidir (M8).
