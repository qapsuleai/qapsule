# M16 — `oltan`: işaret önde / saat yüzeyi

Dal `r5/m16-oltan`. Açılış: `?look=oltan&face=oltan&voice=marla` (koyu varsayılan; `&theme=light`,
`&lang=en`). Kayıt `crates/qp-grammar/src/look.rs` (`OLTAN`), yüz `crates/qp-grammar/src/faces/oltan.rs`
(`faces.rs` `FACES` satırı), yazı tipi `crates/qp-render/assets/fonts/AtkinsonHyperlegibleNext-*.ttf`.
Metin değişmedi (`marla`); kabukta biçem kararı yok (`style.css`, `arch-check.mjs` dokunulmadı).

## İlke

Anlam işareti kahramandır: yüzün en büyük ve en yüksek karşıtlıklı öğesi. Saat kadranındaki
komplikasyon gibi, göz önce işarete düşer ve yönü, türü, geçerliliği oradan okur (K7); cümle
ikincildir ama rahat okunur. Bulanık ve yarım boy kopyada (düzenek §G3, σ 4,5) yüz bir renkli
işaret + gri metin bloğuna indirgenir: işaret tek başına ayırt edilir (kendi denemem, scratchpad).

```
┌──────────────────────────────────────────────┐   yüz 390 × 226 CSS px (r4/vadrel/marla: 259)
│ ┌────────────┐ SYN20 son 20 günün en          │
│ │            │ yüksek fiyatını (227,08)       │  başlık: 19 px yarı kalın, ≤ 5 satır,
│ │   işaret   │ aştı · bugün, hâlâ geçerli     │  176 px sütunda (1 062 marla başlığının
│ │  120 px    │                                │  en kötüsü 5 satır, ölçüldü)
│ └────────────┘                                │
│ Benzer 20 durumun 11'inde fiyat 10 işlem      │  karne: 16 px normal, muted, 2 satır
│ günü içinde yükseldi.                         │  (328 karnenin hepsi ≤ 2 satır); sözlük burada
└──────────────────────────────────────────────┘
```

Tek bakış (K43): bir cümle, bir işaret, bir ek bilgi (karne). Başlık satırı boş (enstrüman
başlığın ilk sözcüğü). Güven işareti ("Sunucu doğruladı") kabukta sağ altta, rozet sağ üstte.

## Kararlar ve gerekçe

1. **İşaret 120 px** (r4: 32). Başlık bloğu (5 × 24 px) ile aynı yükseklikte; alanı ve renk
   doygunluğu metnin gri dokusundan ayrılır. RESEARCH F9 (tek vurgu), M3 Expressive (tek anahtar
   öğede kazanç), Live Activities (büyük, ağır öğe). Daha büyük (128) başlık sütununu 168 px'e
   indirip 6 satıra taşıyordu (ölçüldü) → 120.
2. **Renk = parlaklık merdiveni.** Gri tonlamada ve üç renk körlüğü benzetiminde ayrım için anlam
   renkleri farklı ışıklık basamaklarına konur (CIE L\*):
   - koyu: gri (yönsüz / artık geçerli değil / tutmadı) 47 < aşağı 59 < yukarı 80 < mürekkep
     (gerçekleşti, `tg5` plakası) 97;
   - açık: gri 57 > yukarı 43 > aşağı 28 > mürekkep 6.
   Açık temada kırmızı koyu (bordo) tutuldu: protanopide kırmızı kararır; aşağı rengi açık
   olsaydı protan benzetiminde koyu yeşille çakışıyordu (ΔE 7,8, denendi).
3. **Gri = yön yok.** `neutral` = `invalid`. Renk yalnız yön taşır; yönsüz ve artık geçerli
   olmayan işaret aynı gridir, rozet (köşede halka+çizgi) ve sözcük ayırır. Gerekçe: dört
   basamaktan fazlası koyu temada (L\* 40–100) ve açıkta (L\* 0–58) ΔE00 ≥ 10 ile sığmıyor.
4. **`invalid` = `tg5`'in solmuş `muted`'ı.** `tg5` tutmayan işareti `muted` × 0,55 ile çizer;
   `invalid` tam bu bileşimin rengi (fark ≤ 2/255; sınamada). Böylece "tutmadı" ile "artık geçerli
   değil" aynı renk (renkle kodlanmamış çift, ✗ ↔ halka biçimle ayrılır) ve ayrı bir basamak yemez.
5. **Başlık geçerli değilse `muted`** (vadrel `invalid` kullanır; burada `invalid` 3,75:1 ile
   metin eşiğinin altında kalırdı). Geçerlilik üç yoldan: işaret rengi/rozeti, sözcük, cümle rengi.
6. **Yazı: Atkinson Hyperlegible Next** (Braille Institute; düşük görüş için tasarlandı: açık
   biçimler, 0/O, 1/l/I ayrımı, eğik çizgili sıfır). Başlık 600, geri kalanı 400. RESEARCH §3.2
   (hümanist, açık iç boşluk bakışta okunur), 40+ persona. En küçük yazı 13 px (sahne ve kabuk).
7. **Yüzey: kadran karoları.** Koyu: karo zemini #121317 neredeyse siyah sayfada (#060608), köşe
   14 px, gölge yok; açık: beyaz karo sıcak gri sayfada. Kabuk 48 px sekme, 44 px denetim.
8. **Hareket:** süre × 0,85, `cubic-bezier(0.2, 0, 0, 1)` (kısa, kesin; HIG Motion), kabuk 140 ms.
9. Grafik derinliği oturumda `r4` görünümle kurulur (bilinen sınır); renk rolleri ve yazı
   boyları bu kayıttan gelir. Yeni akış eklenmedi.

## Yazı tipi ve lisans

| | |
|---|---|
| Aile | Atkinson Hyperlegible Next 2.001, `google/fonts` `ofl/atkinsonhyperlegiblenext/` (2026-11-18 indirildi) |
| Lisans | SIL OFL 1.1, **RFN yok** (`OFL-AtkinsonHyperlegibleNext.txt`, depoda); değiştirilmiş sürüm aynı adla dağıtılabilir |
| Değişiklik | fontTools 4.57 `instancer` ile wght 400 ve 600 statik örnek; Latin + Latin-1 + Latin Ext-A + birkaç işaret alt kümesi (`kern`, `locl`, `tnum`, `liga` korunur); 0–9 cmap'te `tnum` gliflerine yeniden eşlendi (fontdue `tnum` uygulamaz) → 10/10 rakam 632 birim |
| Türkçe | ğĞşŞıİçÇöÖüÜâÂîÎûÛ tam; `₺` yok (marla metni `₺` kullanmıyor) |
| Dosyalar | Regular 36 768 B, SemiBold 36 804 B (toplam 73 572 B) |
| Ölçüler | `ascent` 0,86 em (İ/Ü tepesi 861 birim), `advance` 0,46 / 0,47 (yalnız yüz yüksekliği tahmini için; derleyici `advances.rs` tablosuyla ölçer, `QP_BLESS` ile üretildi) |

Betik (scratchpad, depoya girmedi): değişken dosya → `instantiateVariableFont({"wght": w})` →
`Subsetter` → GSUB `tnum` eşlemesiyle cmap 0x30–0x39 yeniden yazımı.

## Sert eşikler (kendi hesabım; düzeneğin `gl/color.py`'siyle, tazelik 1)

| Eşik | Koyu | Açık | Sonuç |
|---|---:|---:|---|
| Metin `text` / zemin | 17,1 | 18,7 | ✓ ≥ 4,5 |
| Metin `muted` / zemin (karne, geçersiz başlık) | 9,8 | 14,4 | ✓ |
| Kabuk fg / muted / kart / accent üstü / not / uyarı | 18,7 / 9,0 / 8,3 / 8,9 / 10,6 / 11,0 | 16,1 / 6,3 / 7,4 / 6,4 / 7,1 / 11,0 | ✓ |
| Kanıt notu `warn` / kabuk `req` | 12,1 / 12,3 | 5,6 / 5,2 | ✓ ≥ 4,5 |
| İşaret yukarı / aşağı / gri / mürekkep | 11,0 / 5,7 / 3,8 / 17,1 | 5,7 / 10,2 / 3,5 / 18,7 | ✓ ≥ 3 |
| Kapanmış işaret tabanı (`closed` @ `closed_alpha`, `r3` sonuç biçimi) | 4,3 (α 170) | 3,6 (α 200) | ✓ ≥ 3 |
| En küçük yazı (sahne rolleri, kabuk) | 13 px | 13 px | ✓ ≥ 11 (öneri 13) |
| Dokunma: sekme / denetim / alt çubuk | 48 / 44 / 60 | | ✓ ≥ 44; bar ≥ denetim + 12 |
| Anlam renkleri ΔE00 (normal, gri, protan, deutan, tritan; CVD'de Viénot/Brettel ile Machado'nun küçüğü) | en küçük 10,5 (yukarı↔mürekkep, gri) | en küçük 11,7 (yukarı↔aşağı, deutan) | ✓ ≥ 10 (işaret 120 px, küçük-işaret eşiği uygulanmaz) |

ΔE00 tablosu (tazelik 1; "aynı" = renkle kodlanmamış çift, biçimle taşınır):

| çift | koyu: norm / gri / prot / deut / trit | açık: norm / gri / prot / deut / trit |
|---|---|---|
| yukarı – aşağı | 71,8 / 16,5 / 26,3 / 14,7 / 63,8 | 60,4 / 13,2 / 20,5 / 11,7 / 50,0 |
| yukarı – gri | 39,8 / 27,6 / 37,5 / 32,4 / 33,1 | 29,0 / 14,0 / 23,0 / 22,0 / 20,6 |
| yukarı – gerçekleşti | 27,5 / 10,5 / 19,7 / 17,8 / 25,1 | 37,8 / 27,6 / 35,3 / 30,8 / 31,3 |
| aşağı – gri | 31,8 / 11,5 / 21,8 / 31,3 / 27,5 | 38,3 / 27,3 / 36,0 / 33,1 / 36,9 |
| aşağı – gerçekleşti | 39,2 / 26,7 / 37,7 / 31,3 / 37,9 | 30,3 / 14,7 / 18,1 / 28,5 / 28,6 |
| gri – gerçekleşti | 37,8 / 37,4 / 37,6 / 37,9 / 37,5 | 40,9 / 40,9 / 40,9 / 40,8 / 40,9 |
| yönsüz – artık geçerli değil – tutmadı | aynı (0,0–0,3) | aynı (0,0–0,3) |

Kayıttan sınanan alt küme: `look::tests::oltan_meets_the_hard_thresholds` (boylar, dokunma,
bütün kabuk metin çiftleri, kapanmış taban, merdivenin her basamağı ≥ 3:1 ve gri ΔE00 ≥ 10,
`invalid` = `tg5` solgunu). Yüz geometrisi: `faces::oltan::tests`.

### Görünüşün çözemediği (bütün yönler için geçerli; öneri)

- **Tazelik saydamlığı.** Alfabe işareti tazeliğe göre saydam çizer (`0.3 + 0.7·freshness`);
  `closed_alpha` tabanı yalnız `siluet`'in kendi kapanmış çiziminde uygulanır. Gösterimin sonuç
  biçimi **`tg5`** bu tabanı kullanmaz: gerçekleşti plakası `text`, tutmadı `muted × 0,55`, ikisi de
  tazelik saydamlığıyla. Gösterimdeki kapanmış kapsüllerin tazeliği 0,06–0,29 (ölçtüm) → α 0,34–0,51:
  koyu temada gerçekleşti plakası ≈ 2,9–5,1:1, tutmadı ≈ 1,5–1,9:1; açıkta benzer. Açık işaretler de
  eskidikçe solar (tazelik 0,3'te aşağı 2,3:1 koyu). Hiçbir görünüş kaydı bunu ≥ 3:1'e taşıyamaz.
  Öneri: `tg5` plakası ve solgun izi de `closed_alpha` tabanına uysun (alfabe/sonuç biçimi işi,
  M16 kapsamı dışı; altınları değiştirmez çünkü `r4` tabanı 0).
- Aynı nedenle tazeliği farklı iki kapsülün bileşik renkleri merdivenden kayabilir; ΔE tablosu
  tazelik 1 içindir.
- `?look=oltan` + varsayılan yüz `vadrel` birlikte seçilirse vadrel geçersiz başlığı `invalid`
  ile yazar (3,75:1, metin eşiği altı). Bu görünüş `face=oltan` ile birlikte tasarlandı.

## K52 (`k52.mjs`, 3 tekrar ortancası, aynı derleme, ms)

| | tik p95 | yüz p95 | sahne p95 | ilk kapsül | kaydırma kare p99 | geçti |
|---|---:|---:|---:|---:|---:|---|
| önce `r4`/`vadrel`, phone-jit-est | 1 | 1 | 2 | 181 | 19 | ✓ |
| sonra `oltan`/`oltan`, phone-jit-est | 1 | 1 | 2 | 180 | 19 | ✓ |
| önce `r4`/`vadrel`, phone (JIT kapalı) | 1,6 | 2 | 5,55 | 212 | 19 | ✓ |
| sonra `oltan`/`oltan`, phone (JIT kapalı) | 1,6 | 1 | 5,55 | 189 | 19 | ✓ |

## Paket boyu (`scripts/demo.sh` `pkg/sizes.json`)

| | wasm ham | gzip | brotli |
|---|---:|---:|---:|
| önce (main 71c2aec) | 5 261 189 | 1 547 432 | 1 026 203 |
| sonra | 5 419 120 | 1 621 004 | 1 055 567 |
| fark | +157 931 | +73 572 | +29 364 |

Farkın ≈ 73,6 KB'ı iki yazı tipi dosyası (gömülü); kalanı ilerleme tabloları (`advances.rs`), kayıt ve
yüz kodu (ayrıca ayrıştırılmadı).
Not: `canvas` yolu açılışta gömülü **bütün** yazı tiplerini `FontFace` ile yükler (görünüş ne
olursa olsun); her yeni görünüşün yazı tipi herkesin açılışına eklenir (öneri: yalnız seçili
görünüşün dosyaları).
