# M9 — `elikov`: kişiye göre öncelik

Kod: `crates/qp-grammar/src/feeds/elikov.rs` (kayıt: `feeds.rs` `LAYOUTS`, `rules: 5`).
Seçim: `?feed=elikov` ya da Ayarlar → Akış düzeni. Okuyucuya gitmez.

## Amaç

Akışı okuyucunun **kendisi için** ne kadar önemli olduğuna göre dizmek; SPEC §3'ün üç
sorusunu ilk ekranda cevaplatmak: G2 (elindeki hissede olumsuz bir şey var mı) en üstte,
G1 (son bakıştan beri ne değişti) hemen altında, G3 (en güvenilir kapsül) geri kalanın
başında. Kaydırma ve dokunma az, kural az.

## Ne gösterir, ne göstermez

- Hiçbir kapsülü gizlemez, yinelemez; yalnız sıralar, grup başlığı ve kısa işaret ekler.
- Kendi önem puanı yoktur: grup içinde otoritenin sırası (önem, son geçiş, kimlik) korunur;
  yalnız üçüncü grup karneye göre dizilir.
- Kitle rolü gerekmez: `CapsuleItem.relevant` (otoritenin kitle kuralı: okuyucunun pozisyonu
  ya da etkin niyetiyle ilgili) yeter; `FeedCtx` genişletilmedi.
- Eski olumsuz haberi (son bakıştan önce bilinen) işaretlemez ve öne almaz: haber değildir;
  geri kalanın içinde karnesine göre yerini alır.

## Yerleşim (yukarıdan aşağı)

| Grup | Kim | İç sıra | Başlık TR / EN |
|---|---|---|---|
| 1 | `relevant` ve son bakıştan beri yeni (son bakış yoksa bütün `relevant`) | olumsuz aşama (`weakened`, `invalidated`, `retracted`) önce | "Elindeki hisselerde yeni" / "New in what you hold" (son bakış yoksa "Elindeki hisseler" / "What you hold") |
| 2 | öteki yeni kapsüller | otoritenin sırası | "Son bakışından beri" / "Since your last look" |
| 3 | geri kalan her şey (pozisyonun eski haberleri dahil) | karne `n ≥ 20` (`GLYPH_MIN_LEDGER_N`) olanlar isabet oranına göre azalan (tam sayı çapraz çarpımla, kayan nokta yok), sonra az örnekli ve karnesizler otoritenin sırasıyla | "Diğerleri, geçmişte en çok tutan önce" / "The rest, best track record first" |

Boş grup ve başlığı yazılmaz. Kurallar (5): `relevant`, `fresh`, olumsuz aşama, karne sırası,
işaret kuralı.

## İşaret metinleri

| Durum | TR | EN |
|---|---|---|
| son bakıştan beri yeni | `yeni` | `new` |
| grup 1, olumsuz aşama, yeni | `yeni · zayıfladı` / `yeni · tutmadı` / `yeni · geri çekildi` | `new · weakened` / `new · did not hold` / `new · withdrawn` |
| grup 1, olumsuz aşama, son bakış yok | `zayıfladı` / `tutmadı` / `geri çekildi` | `weakened` / `did not hold` / `withdrawn` |
| diğer | — | — |

## Düzenek koşusu (2026-11-16, sentetik, `DRY_RUN=1`)

- Kod `5e000c35`; 6 ekran (düzeneğin üst sınırı); akışta 46 kapsül, 6 ekranda 17'si görünür.
- Satırlar: başlık "Elindeki hisselerde yeni" → `SYN09` para girişi (`yeni · tutmadı`);
  başlık "Diğerleri, geçmişte en çok tutan önce" → `SYN22` (20/33), `SYN22` (13/22), …
  Senaryoda pozisyon dışı yeni geçiş yok, grup 2 boş.
- Anahtardan en az kaydırma: G1 0, G2 0, G3 0 (gerekli etiketlerin hepsi ekran 1'de).
  Karşılaştırma için başvuru `r3` (`1ffa7657`): G3 0, G1/G2'nin gerektirdiği kapsül ilk
  6 ekranda yok (en az kaydırma tanımsız).
- `DRY_RUN=1 run_readers.sh m9 elikov {F,H} - 3`: 6 istem, yasak sözcük denetimi geçti.
  Gerçek okuyucu koşulmadı.

## Bilinen zayıflıklar

- `relevant` pozisyon **ve** etkin niyeti kapsar; başlık "Elindeki hisseler" der. Niyetli
  okuyucuda başlık biraz dar kalır.
- Pozisyonun eski haberleri (ör. 50 işlem günü önce tutmamış sinyaller) üçüncü gruba karışır;
  yüz cümlesi "geçersiz oldu" dediği için okuyucu G2'de bunları da sayabilir (işaret yok,
  ama cümle var).
- Üçüncü grubun "en çok tutan önce" sırası karneye dayanır; az örnekli (`n < 20`) yüksek
  oranlı bir kapsül (ör. 4/5) aşağıda kalır. Bu ürünün dürüstlük eşiğiyle tutarlıdır;
  düzeneğin G3 kovası (`n ≥ 5`) ikisini de doğru sayar.
- Grup 1 kalabalıksa (çok pozisyon, çok yeni geçiş) grup 2 aşağı iner; senaryoda sınanmadı.
- Yüz `r3` kartıdır (M8 seçilmeden); kart cümlesinde iç terim ("bar") düzenin dışındadır.
