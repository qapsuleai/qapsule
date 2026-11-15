# Puanlama

```sh
python3 -I score.py answers.csv > results.md          # anahtar: aynı klasördeki key.json
python3 -I score.py answers.example.csv               # örnek (uydurma veri) → answers.example.scored.md
```

Yalnız Python standart kitaplığı. Aynı girdi → bayt düzeyinde aynı rapor. Girdi biçimi
`FORM.md`'de. Anahtar `key.json` (üretici yazar; katılımcı görmez): alfabe başına 20 madde
(`condition`: `nolegend` / `legend` / `learn`, doğru `kind`, `direction`, `strength`, `phase`
ve çizilen tam `GlyphSpec` kimliği), 5 örnek, 8 cümle (metin, durum kimliği, doğru cevaplar),
kitapçık sıraları.

## Doğru cevap kuralları

- **İşaret:** `kind`, `direction` (`up`/`down`/`neutral`), `strength` (güç < 0,5 → `low`;
  sayfalarda yalnız 0,25 ve 0,9 kullanıldı), `phase` (5 aşama). **Tam doğru** = dört alan
  birden doğru. Güven (1–5) puan değil, ayrıca raporlanır.
- **Cümle** (M2 düzeneğinin doğrusu, `harness/src/m2/situations.rs`): **ne** = tür + yön;
  **ne zaman** = `past` (olay ya da bitmiş durum), `now` (süren durum), `conditional`
  (beklenti); gerçek "0 bar önce" olan geçmiş olayda `now` da doğru. **Ne zaman (kesin)** =
  geçmişte ise yazılan bar sayısı da eşit (rakamsız cümlelerde yazılamaz; bilgi amaçlı).
  **Hâlâ geçerli** = `yes`/`no`/`unknown` (olay cümleleri çoğunlukla `unknown`: cümle
  geçerliliğin bilinmediğini söyler). **Tam** = ne + ne zaman + hâlâ geçerli.
- Boş ya da tanınmayan cevap yanlıştır; tanınmayanlar raporun sonunda "Uyarılar"da listelenir.

## Rapordaki tablolar

| Tablo | Ne için |
|---|---|
| Katılımcılar | grup × kitapçık dengesi |
| Alan başına doğruluk (alfabe × koşul) | ana sonuç; açıklamasız / açıklamalı / öğrenme ayrı |
| Çoğunluk tabanı | "hep en sık cevabı söylemek" ne kazandırırdı (şans düzeyinin yanında) |
| Gruba göre | alışkın kullanıcı ve genç aynı alfabeyi aynı okuyor mu |
| Sıra etkisi | alfabe 1. bölümde mi 2. bölümde mi görüldü (A/B dengesi bunu dengeler, bu tablo ne kadar olduğunu gösterir) |
| Güven ve doğruluk | tür doğruyken ve yanlışken ortalama güven: alfabe yanıltıcı mı (yanlışken de emin) |
| Tür karışıklık matrisi | hangi tür hangisiyle karışıyor (alfabe başına) |
| Madde başına | zor işaretler: tasarım düzeltmesi buradan çıkar |
| Eşli karşılaştırma | katılımcı başına siluet − rozet ve işaret testi |
| Cümleler | alan başına, gruba göre, cümle başına (K26 işaretli) |
| Tercih | T1 sayımı, T2/T3 aynen |

## Şans düzeyleri

| Alan | Şans |
|---|---:|
| tür | 1/6 = 0,17 |
| yön | 1/3 = 0,33 |
| güç | 1/2 = 0,50 |
| aşama | 1/5 = 0,20 |
| tam doğru (işaret) | 1/180 ≈ 0,006 |
| cümle: ne zaman, hâlâ geçerli | 1/3 = 0,33 |

Anahtardaki dağılım: her alfabede 20 maddenin türleri 3–4'er (sıkışma 2), yön 9 yukarı /
7 aşağı / 4 yönsüz, güç 10 zayıf / 10 güçlü, aşama 5'e yakın dengeli (çoğunluk tabanı
aşamada 0,25, güçte 0,50). AI ön elemesindeki "aşama hep `final`" kusuru burada yok: her
yarıda (A–B, C–D) beş aşamanın hepsi var.

## Eşli karşılaştırma ve istatistik gücü

Her katılımcı iki alfabeyi de aynı 20 anlamla (aynı `GlyphSpec` kümesi, alfabeye göre farklı
tohumlu sırada) görür; bu yüzden karşılaştırma eşlidir. Katılımcı başına iki özet: **alan
doğruluğu** (80 alan cevabının doğru oranı) ve **tam doğru oranı**. Fark = siluet − rozet;
**işaret testi**: farkı pozitif ve negatif olanlar sayılır, eşitler atılır, iki yönlü kesin
binom p değeri.

Uyarı: 5–10 kişiyle güç çok düşüktür. n = 6'da p < 0,05 yalnız 6/6 aynı yönde olursa
(p = 0,031); n = 10'da 9/10 (p = 0,021) gerekir, 8/10 p = 0,11'dir. Anlamlı çıkmayan fark
"eşit" demek değildir. Bu test bir **karar destek**idir: sonuç, karışıklık matrisi, madde
tablosu, gruplar ve nitel cevaplarla birlikte yorumlanır. Önceden yazılmış karar kuralı
(K39 için önerilen, sonuç görülmeden):

1. Bir alfabe açıklamalı koşulda tür doğruluğunda ≥ 0,10 ve tam doğruda ≥ 0,10 öndeyse ve
   katılımcıların çoğunda (işaret testinde pozitif > negatif) öndeyse o seçilir.
2. Fark bundan küçükse M1 ön elemesinin önerisi (`siluet`) kalır; zayıf maddeler
   (madde tablosunda tür < 0,5) tasarım düzeltmesi olarak yazılır.
3. Bir grup (özellikle genç) bir alfabede açıklamasız tür doğruluğunda şansa yakınsa
   (< 0,35), o alfabe için bu bir bulgu olarak ayrıca raporlanır.

## Örnek

`answers.example.csv` **uydurma veridir** (`make_example.py`, tohumlu; 6 hayali katılımcı,
gerçek kişi yok). Puanlanmış hâli `answers.example.scored.md`. Biçimi ve raporu göstermek
içindir; içindeki sayılar hiçbir şey ölçmez.
