# M8 alternatifi `fotolay` — olayın resmi

Kod: `crates/qp-grammar/src/faces/fotolay.rs` (kayıt `faces.rs`'te tek giriş). Yükseklik
228 CSS px (390 genişlikte). Okuyucuya gitmez.

## Amaç

Kapalı yüz, olayın **küçük bir resmidir**: fiyatın olay çevresinde ne yaptığı, olayın anı ve
seviyesi işaretli; işaret o anın tam üstünde. Teknik analiz bilmeyen biri "burada fiyat şunu
yaptı" diye görebilmeli; kısa başlık resmin ne gösterdiğini söyler. Geçerliliğini yitirmiş ya
da kapanmış öykünün resmi de solar.

## Ne gösterir

- **Başlık satırı (düzen):** enstrüman etiketi (`View.title`, ör. `SYN03`).
- **Başlık cümlesi** (`head` paneli, 15 px, en çok üç satır): ne oldu, en son ne zaman
  hareket etti (`moved`, anahtarın S2'si), hâlâ geçerli mi; ardından benzerlerinin karnesi
  (kısaltılmış, "az örnek" korunur).
- **İşaret** (`mark` paneli, 28 px): olayın barının üstünde, kendi şeridinde (çizgiyle
  çakışmaz). `siluet` ve `rozet` ile çalışır (birim test).
- **Resim** (`pic` paneli): bar başına bir noktadan fiyat izi (kapanış), olayın anında ince
  dikey çizgi, olayın kanıt geometrisi (seviye çizgisi / bant / dönüş noktaları; olgu fiyat
  serisindeyse), olay anındaki fiyatta bir nokta. Pencere: olaydan önce 60 bar, en az 80 bar,
  sağ kenar en yeni bar ("şimdi"); eski olaylarda pencere olayı kapsayacak kadar uzar.
- **Geçerlilik resimde:** geçerliyse iz `text`, an/seviye/nokta `accent`; artık geçerli
  değilse ya da kapandıysa hepsi `muted` (solmuş resim). Hangisi olduğunu işaret ve cümle
  söyler (K42: üçü aynı şeyi söyler).

Ek bilgi birimi (SPEC §2(b)): 1 (resim paneli). Karne ayrı bir metin değil, başlığın ikinci
cümlesi; böylece İngilizce okuyucu da onu kendi dilinde görür (`ledger` metin alanı yalnız
Türkçe "Karne a/b").

## Ne göstermez

- Mum, hacim, indikatör adı, "bar" ya da aile/teyit gibi iç terim.
- Fiyat iz için lejant ve son değer etiketi yok: iz bir `point` işaretidir (çizgi işareti
  derleyicide lejant "fiyat 197.46" ve son değer etiketini kendiliğinden ekliyor, lejant
  izin üstüne biniyordu). Bkz. zayıflıklar: eksen, ızgara, değer ve ay etiketleri yine var.
- Tavsiye (al/sat/bekle) yok.

## Metin şablonu

`{Olay}, {moved}: {geçerlilik}. {karne}` — karne cümlesi 80 karaktere sığan en uzun
biçimiyle (sırayla): `Benzerleri X.` / `Before: X.` → `Geçmişte X.` / `Past: X.` → "az
örnek"/"few cases" → "az"/"few" → EN'de `a of b` → `a/b`. Karne `n = 0`: "Henüz kapanmış
benzeri yok." / "No similar case has closed yet." Yetkisiz okuyucuda parçalar rakamsız,
şablon rakam eklemez (test).

Örnekler (S8, düzeneğin hesabından):

| # | TR | EN |
|---|---|---|
| 1 | Yukarı kırılım, 4 saat önce: hâlâ geçerli. Benzerleri 74'te 36 tuttu. | Upside breakout, 4 hours ago: still holds. Before: 36 of 74 played out. |
| 3 | Aşırı alım, cuma günü: artık geçerli değil. Benzerleri 41'de 16 tuttu. | Overbought, on Friday: no longer holds. Before: 16 of 41 played out. |
| 4 | Sıkışma (yön belirsiz), dün: hâlâ geçerli. Benzerleri 7'de 2 tuttu (az örnek). | Squeeze (direction open), yesterday: still holds. Past: 2 of 7 played out (few). |
| 6 | Aşağı kırılım, 10 Şubat'ta: tutmadı. Benzerleri 5'te 4 tuttu (az örnek). | Downside breakout, on 10 February: did not play out. Past: 4/5 played out (few). |
| 10 | Olağandışı para akışı, dün: hâlâ geçerli. Henüz kapanmış benzeri yok. | Unusual money flow, yesterday: still holds. No similar case has closed yet. |

## Yerleşim (390 × 228 CSS px)

| bölge | yükseklik | içerik |
|---|---:|---|
| başlık satırı | 34 | enstrüman (16 px) |
| `head` | 60 | başlık cümlesi, 15 px, 3 satır |
| `mark` | 30 | işaret, olayın barının üstünde |
| `pic` | 64 | fiyat izi, an, seviye, nokta (+ sağda değer etiketleri) |
| alt boşluk | 24 | ay etiketleri (derleyicinin) |

## Düzenek hesabı (`m8 --face fotolay`, 2026-11-16)

| eşik | sınır | en kötü | not |
|---|---:|---:|---|
| başlık (TR / EN), şablonlu yüzler | 80 | 78 / 80 (ort. 72,2 / 75,8) | geçer |
| başlık, #9 dahil | 80 | 104 | #9 yetkisiz okuyucu: kabuk cümleyi düz metin gösterir (bütün tasarımlarda aynı, `r3` de 104) |
| metin karşıtlığı açık / koyu | 4,5 | 5,78 / 5,27 | geçer |
| işaret karşıtlığı açık / koyu | 3 | 2,06 / 2,36 | yalnız kapanmış #5, #6, #7; açık durumlarda ≥ 5,16 / 5,29. Değerler `r3` ile aynı: alfabe kapanmış işareti gri ve tazeliğe göre saydam çizer, yüz bunu değiştiremez |
| en küçük yazı | 11 | 11 | derleyicinin eksen etiketleri |
| ek bilgi | 2 | 1 | |

Gri kopya: yön işaretin biçiminde ve izin eğiminde; geçerlilik işaretin biçiminde (siluet:
gri silüet + köşe rozeti) ve resmin açık/koyu tonunda.

## Bilinen zayıflıklar

1. **Eksen, ızgara, değer ve ay etiketleri:** veri paneli derleyicide her zaman sağda değer
   etiketleri (ör. "200.00"), ızgara, eksen çizgileri ve altta ay etiketleri alır; "sayısız,
   eksensiz resim" bu arayüzle kurulamıyor. Gramere "çıplak panel" seçeneği (derleyicide
   panelin ölçek süslerini kapatan bir bayrak) eklemek `compile.rs` değişikliği ister,
   alternatifin dosyası dışında; yapılmadı (öneri). Soldaki ay etiketi panelin solundan
   taşıp kesilebiliyor ("023-03").
2. **Nokta izi** çizgiden zayıf okunur (lejant ve son değer etiketinden kaçmak için).
   Çıplak panel olsaydı çizgi işareti tercih edilirdi.
3. **Kapanmış öykülerin işaret karşıtlığı** (yukarıda) eşik altında; alfabenin özelliği.
4. Taze olayda an resmin sağ kenarına düşer (olaydan sonrası yok); işaret panel içinde
   kenara sıkıştırılır.
5. `moved` ile `when` farklı olduğunda (#3, #11) resimdeki an doğuşun anı, cümledeki zaman
   son geçişin anıdır.
6. Koyu temada `muted` ile `text` ton farkı açık temadakinden az; solma daha az belirgin.
