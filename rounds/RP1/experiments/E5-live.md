# E5 — Canlı güncelleme (profil D) ve hesabı geçmişe bağlama (attach)

Dal: `rp1/e5-live` (taban `caab6e3`). Yürüten: orkestratör.

## Önce ölçüm: sınır nerede?

T1 taban ölçümü (M4 Max): 500 hesabın hepsinde bir tik (`update_last`) medyan **1,83 µs**,
yeni bar (`push`) **2,75 µs**; WASM 4,3 / 5,3 µs; CI x86_64 3,5 / 5,2 µs. 16 ms'lik bir ekran
karesine ≈ 4,4 milyon hesap-tiki, 1 ms'ye ≈ 270.000 sığıyor. "500 sembol × 10 hesap, hepsi aynı
anda tik" = 5.000 güncelleme ≈ 18 µs. **Tik yolu sınır değil.**

Sınır "attach": ajan canlı grafiğe 500 hesap eklediğinde her hesap 10.000 barlık geçmişi
koşmalı: **26,9 ms** yerel, 54,7 ms WASM, 51,8 ms CI. Bu, insanın fark edebileceği mertebe.

## Hipotez (denemeden önce yazıldı)

**H5.1 — Geçmiş için yalnızca `commit`.** Attach'ta geçmişin çıktısı istenmiyor, yalnızca
taahhüt edilmiş durum. Bugünkü yol her bar için `push` = önceki bekleyeni `commit` + `peek`
(çıktı, `Option`, `dyn` çağrısı) yapıyor. Geçmişi `Core::commit` ile tek biçimli bir döngüde
koşup son barı bekleyen olarak bırakmak, `push` dizisinin bıraktığı durumun **aynısını**
bırakır (aynı `commit`ler aynı sırada) ve `peek`'in (iş yarısı) ve sanal çağrıların maliyetini
kaldırır. Beklenen: attach ≈ 2 kat.

## Deney 5.1 — `Study::extend` (yalnızca `commit` ile geçmişe bağlama)

**Yapılan.** `Study` arayüzüne varsayılan gövdeli `extend(&mut self, columns)` eklendi
(kaynak uyumlu: varsayılan bar bar `push`). `Streaming<C>` bunu tek biçimli bir döngüde
yalnızca `Core::commit` ile, son barı bekleyen bırakarak uyguluyor. `profiles::Bank::warm`
bunu kullanıyor; D profilinin özeti (`d_digest`) artık geçmişi `warm` ile kuruyor ve T1'deki
sabit değeri (`4db86a7246f733bf`) veriyor — yani `extend` = bar bar `push`, bit bit.

**Ölçüm** (500 hesap kur + 10.000 barlık geçmiş; medyan ms):

| Hedef | push döngüsü | extend | Hız |
|-------|------------:|-------:|----:|
| aarch64 | 28,8 | **14,9** | 1,9× |
| wasm32 (Node 24) | 60,8 | **19,0** | 3,2× |

**Bit-eşitlik kanıtı.** `tests/extend.rs`: beş hesaptan 52 parametre kümesi + bir zincir;
20 tur boyunca rastgele uzunlukta bloklar ya `extend` ya `push` ile eklenir, her bloktan
sonra son bar iki kez düzeltilir (`update_last`); her seferinde çıktılar yalnızca `push`
kullanan eşine bit bit eşit. Ayrıca `tests/profiles.rs` (D özeti) değişmeden geçiyor.

**Karar: al.** Tik yolu (1,8 µs / 500 hesap) sınır olmadığı için tek biçimli SoA "banka"
(H5'teki diğer fikir) denenmedi: kazanç mikro saniyenin altında kalırdı; ölçüm gerekçesi
yukarıda. Attach için daha büyük kazanç: E8'in kanıtlı geç başlangıcı (EMA/RSI/MACD durumu
geçmişin tamamı koşulmadan bit-eşit kurulabilir) — sonraki adım.
