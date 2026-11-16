# R4/K42-FEED — pozisyondaki olumsuz geçiş okuyucunun akışında yok

Dal `r4/k42-feed`. Kaynak: `rounds/R4/human-test/README.md` açık soru 2.

## Gözlem

Sentetik gösterimde (`qp demo --synthetic`; kabuksuz eşi `qp_wasm::drive::parity_host`)
kitle insan testi planındaki gibiydi: izleme listesi 8 hisse, 2 pozisyon, `balanced`. Elde
tutulan hissenin bu seansta geçersizleşen ya da zayıflayan kapsülü okuyucunun 40 satırlık
akışında (`capsules`, `audience: true`, `limit: FEED_LIMIT` = 40) yoktu; M9 senaryosunda
(adım 600–609, SYN09) vardı.

## Yeniden üretim (kabuksuz, protokol istemcisi)

Adım `t0`'a git, `set_audience`, `t0 + 9`'a git (bir seans); `capsules` `audience: true`
sınırsız ve `limit: 40` ile sorulur. Kapsül, yaşam döngüsü, sınırsız cevaptaki sırası:

| Pencere | Kapsül | Doğum → son geçiş (bar) | Aşama | Kademe | Önem | Kitlede sıra | 40'luk akışta (önce → sonra) |
|---|---|---|---|---:|---:|---:|---|
| 762–771 | `cap.SYN12.breakout.up.203` | 203 → 204 | invalidated | 2 | 1,595 | 87 / 139 | yok → 38 |
| 762–771 | `cap.SYN12.extreme.up.203` | 203 → 204 | invalidated | 3 | 1,480 | 116 / 139 | yok → 39 |
| 801–810 | `cap.SYN01.reversal.down.205` | 205 → 208 | invalidated | 3 | 1,528 | 117 / 143 | yok → 39 |
| 906–915 | `cap.SYN12.squeeze.neutral.215` | 215 → 220 | weakened | 3 | 1,152 | 136 / 146 | yok → 39 |
| 600–609 (M9) | `cap.SYN09.flow.up.183` | 183 → 186 | invalidated | 1 | 1,714 | 29 / 129 | 29 → 29 |

(801–810 penceresinin kitlesi planın 2. G2 penceresininki: pozisyonlar SYN19, SYN01.)

## Kök neden

Elenen adayların her biri ölçüldü:

- **Hassasiyet süzgeci değil.** Kapsüllerin hepsi `audience: true` sınırsız cevabında var
  (`relevant: true`, aşama olumsuz; K42/A22 geçidi çalışıyor). Konu bildirimleri de geçişin
  kendi aşamasıyla süzülüyor, doğru.
- **Kitle rolü eşleşmesi değil.** İzleme listesi pozisyonu kapsıyor, `relevant` doğru.
- **"Aynı barda doğup kapanma" özel durumu değil.** Hiçbirinin doğumu ile kapanışı aynı
  barda değil (203 → 204, 205 → 208, 215 → 220); doğum önceki seansta biliniyordu, olumsuz
  geçiş bu seansta.
- **Neden: `limit` + sıralama.** `capsules` cevabı önem azalan, sonra son geçiş zamanı
  azalan sıralanıp `limit`'e kesiliyordu. Kitlenin kapsül sayısı 129–146; bunun büyük kısmı
  pozisyon hisselerinin **geçmişteki** olumsuz geçişleri (şimdiki aşaması `invalidated` olan
  hikâye K42 gereği her seviyede geçer ve kalır). 40. satırın önemi ~1,67–1,70; bu seansın
  kademe 2–3 olumsuz geçişleri (önem 1,15–1,60) altta kalıp kesiliyordu. M9 penceresinde
  geçiş kademe 1 ve önemi 1,714 olduğu için ilk 40'a giriyordu; M9 düzeneğinin
  `scenario()`'su zaten "olumsuz geçiş akışta" olan ilk pencereyi seçtiği için sorunu
  görmedi. İki akış düzeni de (`elikov`, `vakiro`) yalnız gelen öğeleri dizer, bu yüzden
  ikisinde de yoktu.

Sonuç: sayfa sınırı, K42'nin "olumsuz geçiş seviyeden bağımsız gelir" kuralını fiilen geri
alıyordu (akışın ilk 40'ı aylar önceki yüksek önemli kapsüllerle dolu: 762 penceresinde 40.
satır ~5 ay öncesinden).

## Karar (uygulanan)

`capsules` sorgusunda `audience: true` iken `limit`, soranın pozisyon/niyet kapsülünün
**şimdiki** olumsuz geçişini kesmez:

- korunan: `relevant`, şimdiki aşama `weakened`/`invalidated`/`retracted`, son geçişin barı
  dizinin **son kesin barı** (günlük dizide: bu seansta bilinir olan; `ShellCore::known_at`
  ile aynı ölçü);
- korunanlar önce alınır, kalan yer önem sırasıyla dolar; cevap yine sıralı (önem, zaman,
  kimlik) ve en çok `limit` satır (korunan öğe sıradaki yerinde, çoğunlukla sonda; öne
  çıkarma akış düzeninin işi, ör. `elikov` olumsuzları öne alır);
- bir sonraki kesin barda (ertesi seans) öğe yeniden önemle yarışır; geçmişteki olumsuz
  geçişler sınırı aşmaz (yoksa 100+ eski geçersizleşme akışı doldururdu).

Belirleyici; bakış durumuna (son bakış) bağlı değil; `audience` verilmeyen ya da sınırın
kesmediği cevaplar bayt bayt aynı.

Kod: `crates/qp-session/src/capsule.rs` (`capsules_query`, `Session::is_current`). Belge:
`docs/PROTOCOL.md` §20.2 `limit` satırı.

## Regresyon testi

`crates/qp-wasm/tests/k42_feed.rs` (gösterim barındırıcısı, insan testi kitlesi):

1. 762, 801, 906 pencerelerinde yukarıdaki kapsüller sınırsız sırada 40'ın altında ve
   40'luk akışta; akış sıralı, en çok 40; geri kalanı sıralamanın tepesi (yalnız korunanlar
   yer değiştiriyor).
2. 600–609 (M9 senaryosu ve insan testinin 1. G2 penceresi) ve 882–891 (2. G2 penceresi):
   akış düzeltmeden önceki gibi (sıralamanın ilk 40'ı).
3. Kabuk çekirdeği (`ShellCore`) 762–771'de: kapsül `feed()`'de ve her kayıtlı akış
   düzeninin satırlarında (`feed_json`).

Düzeltme olmadan 1 ve 3 kırılıyor, 2 geçiyor (denendi).

## Değişen dökümler, altınlar, özetler

**Hiçbiri.** Dökümler (`fixtures/protocol/conformance/*`) `capsules`'ı `limit`'le sormuyor;
gösterim altınları (`fixtures/golden/demo`) ve eşlik dökümleri kitlesiz bağlantıyla
(pozisyon yok → korunan öğe yok). M9 ham anahtarları (600–609) ve insan testi anahtarı
(`rounds/R4/human-test/key.json`; pencereler 600–609 ve 882–891) değişmez (test 2).
`QP_VERIFY_BROWSER=1 scripts/verify.sh` yeşil (18 adım, 474 sn; demo altınları ve dökümler aynı).

Ayrı commit: `cargo fmt` — `r4/int`'te M11 dosyalarında (`crates/qp-grammar/src/glyph/outcome.rs`,
`outcome/muhrak.rs`) kalmış biçim farkı `cargo fmt --check`'i kırıyordu; yalnız biçim,
davranış değişmez.

## Önerilen karar

"K42/A22'nin akıştaki karşılığı: kitle sorgusunda sayfa sınırı, pozisyon/niyet kapsülünün
şimdiki (son kesin bardaki) olumsuz geçişini kesmez." (DECISIONS'a orkestratör derler.)

## Öneriler (kapsam dışı)

- **Akışın zaman ufku.** `capsules` önem sırası tüm geçmiş üzerinde; akışın 40 satırının
  çoğu aylar önceki kapsüller. Ürün akışı için bir zaman ufku (ör. son N seans ya da son
  bakıştan beri, `since_t`) ya da önemin yaşla azalması M9/insan testi sonrası düşünülmeli.
  O kurulunca bu korumanın "son kesin bar" ölçüsü "son bakıştan beri" ile değiştirilebilir.
- İnsan testi README açık soru 2 bu notla cevaplandı; plan isterse 762–771 / 906–915
  pencerelerine geri dönebilir (korunan satır sıranın sonunda; `r3`'te kaydırma gerekir,
  `elikov` olumsuzları öne alır, öteki düzenler kendi kuralıyla dizer).
- M9 düzeneğinin `scenario()`'su "akışta görünen" pencereyi seçtiği için bu tür hatayı
  saklar; seçimden bağımsız bir "pozisyonun olumsuz geçişi akışta" denetimi eklenebilir.
