# M1 adayı "rozet" — tasarım notları

Öncül: her işaret aynı yuvarlak rozettir. Dış çevre (halka) her boyutta ve her durumda
aynıdır; bir satır işaret, bir satır eşit madeni para gibi okunur, değişen yalnız içtir.
Gerçekleme: `src/lib.rs` (`m1_rozet::Alphabet`, ad `rozet`; `LEGEND` 115 sözcük).

## Kanal başına gerekçe

| Kanal | Kodlama | Neden |
|---|---|---|
| Tür | İç desen: **devam** üst üste iki ok başı; **dönüş** solda küçük aşağı + sağda büyük yukarı ok; **kırılım** ortası açık çizgiyi delip geçen ok; **sıkışma** uçları birbirine dönük iki üçgen (▶◀); **aşırılık** çizginin ötesindeki iri nokta; **akış** damla | Her desen kütlesini rozetin farklı bölgesine koyar (orta sütun, sağ sütun, yanlar, üst, alt-orta); 16 px'te en çok karışan çiftleri ölçüyle ayırdım. İlk akış deseni "büyükten küçüğe üç nokta" kırılımın gövdesiyle aynı sütuna düştü (en zayıf çift 1,05); damla ile 1,59'a, hale ile 2,55'e çıktı. |
| Yön | Desenin döndürülmesi: uç yukarı = yukarı, 180° = aşağı, 90° (uç sağa) = yönsüz. Renk tekrar eder: `up`, `down`, yönsüz `text` | Döndürme renk körlüğünden etkilenmez; renk yalnız pekiştirir. Sıkışmanın doğal yönü yönsüzdür (döndürülmez); yukarı/aşağı sıkışma üste/alta küçük bir üçgen ekler. Trigonometri yok: döndürmeler koordinat değiş tokuşu. |
| Güç | Desenin rozet içindeki boyu (0,56 + 0,44·güç) | Dış çevre sabit kalır; içteki desen büyür/küçülür. |
| Güven | Alttan yükselen gri dolgu (`muted`, %42) + üstünde ince "su çizgisi"; güven yoksa dolgu yok | "Doluluk"un en düz okunuşu. Su çizgisi `yok` ile `0,3` arasını 1,12'den 1,49'a çıkardı. |
| Aşama | aday: içi boş desen + ince gri halka; kesin: dolu desen; gerçekleşti: kalın çerçeve; gerçekleşmedi: tek eğik çizgi (⊘); geri çekildi: her şey gri + çarpı (desenin altında) | Halka teyidi taşıdığı için "kesik çizgi" halkaya konamadı; kesikli desen 32 px'te bile okunmadı (denendi, atıldı). "Kalem" halkası + boş desen 16 px'te de ayrışıyor. Çarpı desenin altında kalır ki geri çekilmiş işaretin türü hâlâ okunsun. |
| Tazelik | Bütün glifin opaklığı (0,22 + 0,78·tazelik) | Solukluk. |
| Teyit | Halka 1–5 eşit parçaya bölünür (ilk boşluk tepede) | Saymak kolay; dış çevre değişmez. Boşluk ≥ 2,8 px. |
| Sunucu | Sağ üst köşede, zemin renkli halesiyle küçük elmas çentik | Dairenin köşelerde bıraktığı boş alanı kullanır. |

Ortak: motifin çevresinde zemin renginde ince hale (dolgudan ve çarpıdan ayırır; ana ölçüyü
1,44'ten 2,46'ya çıkardı). Tüm renkler temadan; açık ve koyu temada aynı kural. Sinüs/kosinüs
yalnız temel IEEE işlemleriyle (çeyrek indirgeme + Taylor; `sincos_deg`), `std` trigonometrisi
yok. Bütün ilkeller kutunun içinde (test: 16–64 px, iki tema, 450 spec).

## Ayırt edilebilirlik (son koşu, `harness m1 measure --alt rozet`)

- **Ana ölçü (en kötü koşulda `K`'nin en zayıf çifti): 2,55** — `continuation.down` ↔
  `breakout.down`, 16 px / açık / gri. (Kontrol: 0,00.)
- `K` ortalamasının en düşüğü: 4,06 (16 px açık gri). 16 px açık normalde `K` min 3,74, ort. 7,88.
- Kutu dışı ilkel: 0.

| Kanal | En zayıf çift ΔE | Koşul |
|---|---:|---|
| yön | 2,57 | 24 / açık / gri (kırılım ↑↓) |
| aşama | 2,02 | 16 / açık / gri (akış↓ kesin ↔ gerçekleşmedi) |
| güç | 2,27 | 24 / koyu / gri (aşırılık↓ 0,2 ↔ 0,9) |
| teyit | 2,17 | 24 / açık / protan (1 ↔ 2) |
| güven | 1,49 | 16 / koyu / gri (yok ↔ 0,3) |
| tazelik | 10,50 | 16 / açık / gri |
| sunucu | 1,65 | 16 / açık / protan |

## Bilinen zayıflıklar (dürüstçe)

- **Güven `yok` ↔ `0,3`** en zayıf kanal: alttaki küçük bir daire parçası. Ayrıca `Some(0)` en az
  %10 dolgu ile çizilir ki `yok`tan ayrılsın; 0 ile 0,1 aynı görünür.
- **Sunucu elması küçük** (16 px'te ~4 px); kalabalık bir sayfada gözden kaçabilir.
- **Gerçekleşmedi eğik çizgisi** deseni keser; 16 px'te türü okumayı zorlaştırır.
- **Gerçekleşti kalın çerçevesi** iç alanı daraltır; desen biraz küçülür ve düşük güçle
  karışabilir (güç ile aşamanın etkileşimi).
- **Yönsüz = yana dönük** kuralı öğrenilmesi gereken bir uzlaşımdır; açıklamasız okuyucu
  sağa bakan damlayı/oku "yukarı/aşağı değil" diye okumayabilir.
- **Dönüş** (biri küçük aşağı, biri büyük yukarı iki ok) 16 px'te küçük oku yitirebilir; o
  zaman tek yukarı ok gibi görünür ve kırılımla karışabilir.
- Güven dolgusu gri; gri desenli **geri çekildi** işaretinde desenle dolgu arasındaki karşıtlık
  düşer.
- Renk tek başına hiçbir kanalı taşımaz; ama renkli okumada yön ayrıca renkle pekiştirildiği
  için gri koşul en kötü koşuldur (beklenen).
- Okuma testleri bu ajan tarafından çalıştırılmadı (görev gereği); yukarıdaki sayılar yalnız
  hesapla ölçülen ayırt edilebilirliktir.
