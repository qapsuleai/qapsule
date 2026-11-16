# R3/M2a — Kalıp dilbilgisi cümlecisi (`template`)

Dal: `r3/m2a-template`. Crate: `rounds/R3/experiments/M2/template/` (`m2_template`, çalışma
alanı üyesi değil; `qp-schema`, `qp-capsule`'e yol bağımlılığı). Düzenekte kayıtlı:
`harness m2 measure --alt template`. Dil modeli yok, ağ yok, saat yok, rastgelelik yok:
cümle `SentenceInput`'un saf fonksiyonudur.

Bu not yalnız bu adayı anlatır; öbür aday görülmedi.

## 1. Tasarım

İki adım:

1. **Çerçeve (`Plan`, dilden bağımsız).** Girdiden şunlar çıkarılır:
   - **zaman sınıfı** (K26, K27): `story.time` olay → *olay*; beklenti → *beklenti*; durum →
     `still_holds = true` ise *süren durum*, `false` ise ya da aşama
     `invalidated/expired/retracted` ise *biten durum* (`still_holds = None` olan durum,
     olay gibi geçmiş + "bilinmiyor" ile söylenir);
   - baş olgunun ve son geçişin **kaç bar önce** olduğu (seri günlükse ve bir önceki takvim
     günüyse "dün"); biten durumda çıkış (`edge = exit`) olgusunun barı;
   - **koşul** (`subject`, yön, eşik anahtarı, `within_bars`) ve **güncel değer**;
   - **aşama notu** (güçlendi, teyit edildi + hangi aile, zayıfladı, gerçekleşti, geçersiz,
     süresi doldu, geri çekildi, aday);
   - **geçerlilik kuyruğu** (olay/beklenti için: evet / hayır / bilinmiyor);
   - **karne**, **sunucu işareti**, **lisans**.

   Çerçevenin sayı erişimcileri (`val`, `thr`, `current`, `families`, `horizon`, `within`,
   `ledger`) `redacted` iken **hiç sayı döndürmez**; rakamsız aile bu yüzden "unutulamaz".
2. **Kalıplar (`tr.rs`, `en.rs`).** Her zaman sınıfı için sabit bir kalıp; ardından aşama
   notu, geçerlilik kuyruğu, karne, sunucu işareti ayrı kısa cümleler olarak eklenir.

| Sınıf | TR kalıbı | EN kalıbı |
|---|---|---|
| Olay | `{özne} {ne zaman} {tür yüklemi, -DI}: {tür adı}.` | `{When}, {subject} {past clause}: {kind noun}.` |
| Süren durum | `{özne} {süre -DIr} {eşik-İN} üstünde; şu an {v}. {Tür adı} sürüyor.` | `{Subject} has been above {thr} for {n} bars and is now {v}. {Noun} is still in place.` |
| Biten durum | `{özne} {ne zaman} {eşik-İN} üstüne çıktı, {ne zaman} yeniden altına indi; şu an {v}, artık {eşik-İN} üstünde değil. {Tür adı} sona erdi.` | `{When}, {subject} crossed above {thr}; it fell back below {when} and is now {v}, no longer above {thr}. {Noun} has ended.` |
| Beklenti | `{özne} {n} bar içinde {eşik-İN} üstünde kapanırsa / üstüne çıkarsa / altına inerse {tür adı} teyit olur.` | `If {subject} closes above / rises above / falls below {thr} within {n} bars, {noun} would be confirmed.` |

Olay yüklemi türe (ve akışta kanıt anahtarlarına) göredir: kırılım "148,20'deki direnci
yukarı kırdı (kapanış 150,13)", aşırılık "RSI 70'in üstüne çıktı (71,4)", devam "hareketli
ortalamalar yükseliş yönünde dizildi", dönüş "fiyat yeni dip yaparken RSI daha yüksek dip
yaptı", sıkışma "Bollinger bantları daraldı", akış "kurumsal net alış son günlerin
ortalamasını belirgin biçimde aştı" / "hacim ortalamasının belirgin üstüne çıktı". Kanıt
anahtarları hem gerçek dedektörlerin (`sma20/sma50`, `p2_price/p2_rsi`, `bw`, `vol_avg`)
hem düzeneğin (`fast/slow`, `price/rsi`, `bandwidth/threshold`) adlarıyla okunur; anahtar
yoksa değer parantezi düşer, cümle bozulmaz.

Kısaltmalar:
- Ne zaman (an): `0` → "bugün" (günlük) / "bu barda"; `1` → "dün" (günlük ve bir önceki
  takvim günü) / "1 bar önce"; `n` → "n bar önce". EN: "today / on this bar", "yesterday /
  1 bar ago", "n bars ago".
- Süre (süren durum): "bugün itibarıyla", "dünden beri", "5 bardır"; EN "as of today",
  "since yesterday", "for 5 bars". Süren durumda "n bar önce" yerine süre kullanıldı ki
  okuyucu "şimdi" desin, "geçmiş" demesin.
- Beklentide doğum barı söylenmez (koşullu okunması için); yalnız `within_bars`.

### Karne kuralı

- `n = 0` → "Henüz karnesi yok." / "No track record yet."
- `1 ≤ n < 20` → "Karnesi henüz az örnekli: 8 seferin 3'ünde tuttu." / "Its record is still
  thin: it played out in 3 of 8 cases." (sayı verilir ama uyarıyla)
- `n ≥ 20` → "Geçmişte bu sinyal 140 seferin 87'sinde beklendiği gibi sonuçlandı." /
  "Historically, this signal played out in 87 of 140 cases." (CAPSULE §5'teki biçim)
- Yüzde ve ufuk yazılmaz (ufuk hikâyeye göre değişebiliyor; yüzde ikinci bir sayı ekleyip
  okumayı ağırlaştırıyor).
- Eşik `LEDGER_MIN_N = 20` sabittir (`lib.rs`); seçimi gerekçesiz bir başlangıç değeri.

### Sunucu olgusu ve rakamsız aile (K33, K38)

- `trust = server` → "Sunucu doğruladı." / "Server-verified."
- `redacted` → ayrıca "rakamlar kurumsal akış lisansı gerektiriyor" (yetki `flow/…` ise;
  değilse "veri lisansı").
- `redacted` iken hiçbir sayı yazılmaz; karne dahil (§9.5: lisanslı olgunun cümlesi hiç
  rakam içermez):
  - bar-önce: 0 → "bugün", 1 → "dün" / "bir bar önce" ("bir" sayı sözlüğünde değil, belirteç);
    günlük seride 2–3 "birkaç gün önce", 4–7 "yaklaşık bir hafta önce", 8–17 "birkaç hafta
    önce", 18–26 "yaklaşık bir ay önce", üstü "bir aydan uzun süre önce"; günlük olmayan
    seride "birkaç bar önce / bir süre önce / uzun süre önce". Süre biçimleri aynı kovalarla
    (-DIr ekiyle: "birkaç gündür", "yaklaşık bir haftadır").
  - eşik adla: "aşırı alım eşiği", "kırılım seviyesi", "uzun ortalama", "son günlerin
    ortalaması", "eşik"; `within_bars` → "birkaç bar içinde".
  - karne niteliksel: oran ≥ 0,6 "çoğunlukla beklendiği gibi sonuçlandı", 0,4–0,6 "kabaca
    yarı yarıya tuttu", < 0,4 "çoğunlukla tutmadı"; `n < 20` "Karnesi henüz az örnekli."
  - sayı sözcüğü de yok ("iki", "beş", "yüzde", "half", "twice"…): kalıplar düzenek
    sözlüğündeki sözcüklerden kaçınacak biçimde yazıldı ve testle sınanıyor.
- Son güvenlik ağı: `redacted` cümlede yine de rakam çıkarsa (ör. adında rakam olan bir
  sembol) cümle genel bir rakamsız cümleyle değiştirilir. Testteki 35 640 rakamsız
  birleşimin hiçbirinde ağ devreye girmez (test ağı atlayarak `render`'ı doğrudan sınar).

## 2. Türkçe biçimbilim (`morph.rs`)

Ekler sözcüğün **okunuşuna** göre seçilir, **yazılışına** eklenir; sayı ve özel adlarda
kesme işaretiyle.

- **Sayının okunuşu:** son okunan sözcük bulunur: `70` → yetmiş → `70'in`; `30` → otuz →
  `30'un`; `148,20` → "… virgül yirmi" → `148,20'nin`, `148,20'deki`; `0,06` → altı →
  `0,06'nın`; `71,4` → dört → `71,4'te`; `100` → yüz → `100'ün`; `2.340.000` → bin →
  `2.340.000'in`; `1.000.000` → milyon → `'un`.
- **Ünlü uyumu:** iki yönlü A (a/e) ve dört yönlü I (ı/i/u/ü), son ünlüye göre.
- **Ünsüz benzeşmesi:** sert ünsüzden (f s t k ç ş h p) sonra D → t: `çıktı`, `aştı`,
  `ASELS'te`, `AKBNK'ta`, `71,4'te`, `5 bardır`
- **Kaynaştırma:** ünlüden sonra tamlayan `n` (`yirmi'nin`), yönelme/belirtme `y`, iyelik
  `s` (`87'sinde`); iyelikli adlardan sonra `n` (`son günlerin ortalamasını`,
  `kırılım seviyesinin`).
- **İyelik + bulunma:** "100 seferin 62'sinde", `40'ında`, `3'ünde`, `6'sında`.
- **Yumuşama:** yazılı, çok heceli adlarda p/ç/t/k → b/c/d/ğ (`eşik` → `eşiğin`, `direnç` →
  `direnci`, `destek` → `desteği`); kesme işaretinden sonra hiç; istisna listesi (`fiyat`…).
- **Fiil:** geçmiş -DI (`kırdı`, `çıktı`, `indi`, `döndü`, `oldu`, `zayıfladı`); geniş zaman
  (tek heceli -Ar, istisna listesi -Ir: `al, bil, bul, dur, gel, gör, kal, ol, öl, san, var,
  ver, vur`; çok heceli -Ir; ünlüden sonra -r) + koşul -sA: `çıkarsa`, `inerse`,
  `kapanırsa`, `kalırsa`, `olursa`, `yükselirse`; ek-fiil -DIr: `5 bardır`, `birkaç gündür`.
- **Semboller** sözcük gibi okunur (BIST alışkanlığı): `THYAO'da`, `THYAO'nun`, `GARAN'da`,
  `KCHOL'da`, `SISE'de`, `PGSUS'ta`, `BIMAS'ta` (ASCII `I` noktalı okunur: BİMAŞ). Ünlüsüz
  sembol harf harf okunur (ünsüz harf adları `-e` ile biter).
- Sayılar düzeneğin yuvarlama kuralıyla (`(v·10^k).round()/10^k`) yazılır: fiyat 2 ondalık
  (`148,20`), RSI en çok 1 (`70`, `71,4`), oran en çok 4 (`0,0588`), akış/hacim tamsayı ve
  binlik ayraçlı (`2.340.000`; EN `2,340,000`). Eksi işaret yazılmaz, sözcükle söylenir
  ("net satış").

## 3. Örnek cümleler

**K26** (RSI 3 bar önce 70'i yukarı kesti, bugün 55; `still_holds = false`):

- TR: "THYAO'da RSI 3 bar önce 70'in üstüne çıktı, bugün yeniden altına indi; şu an 55,
  artık 70'in üstünde değil. Aşırı alım sona erdi. Geçmişte bu sinyal 140 seferin 87'sinde
  beklendiği gibi sonuçlandı."
- EN: "3 bars ago, THYAO's RSI crossed above 70; it fell back below today and is now 55, no
  longer above 70. The overbought state has ended. Historically, this signal played out in
  87 of 140 cases."
- Aynı durum lisanslı ve yetkisiz okuyucuya (test): "THYAO'da RSI birkaç gün önce aşırı alım
  eşiğinin üstüne çıktı, bugün yeniden altına indi; artık aşırı alım eşiğinin üstünde değil.
  Aşırı alım sona erdi. Geçmişte bu tür sinyal çoğunlukla beklendiği gibi sonuçlandı."

Diğerleri (düzenek durumları ve testler):

- Olay: "THYAO 5 bar önce 148,20'deki direnci yukarı kırdı (kapanış 150,13): yukarı
  kırılım. Hâlâ geçerli olup olmadığı bilinmiyor. Geçmişte bu sinyal 140 seferin 62'sinde
  beklendiği gibi sonuçlandı." / "5 bars ago, THYAO broke above resistance at 148.20 (close
  150.13): an upside breakout. Whether it still holds is unknown. Historically, this signal
  played out in 62 of 140 cases."
- Süren durum: "THYAO'da RSI 5 bardır 70'in üstünde; şu an 71,4. Aşırı alım sürüyor.
  Karnesi henüz az örnekli: 8 seferin 3'ünde tuttu." / "THYAO's RSI has been above 70 for 5
  bars and is now 71.4. The overbought state is still in place. Its record is still thin: it
  played out in 3 of 8 cases."
- Beklenti: "THYAO 3 bar içinde 148,20'nin üstünde kapanırsa yukarı kırılım teyit olur.
  Koşul hâlâ açık. …" / "If THYAO closes above 148.20 within 3 bars, the upside breakout
  would be confirmed. The setup is still open. …"
- Biten durum + geçersiz (s26): "PGSUS dün uzun ortalamanın (29,19) altına indi; şu an
  30,07, artık uzun ortalamanın altında değil. Düşüş eğilimi sona erdi. Sinyal bugün geçersiz
  oldu. Karnesi henüz az örnekli: 8 seferin 2'sinde tuttu."
- Akış, sunucu olgusu, rakamsız (teyitli): "THYAO'da dün kurumsal net alış son günlerin
  ortalamasını belirgin biçimde aştı: para girişi. Bugün hacim de teyit etti. Hâlâ geçerli
  olup olmadığı bilinmiyor. Geçmişte bu tür sinyal çoğunlukla beklendiği gibi sonuçlandı.
  Sunucu doğruladı; rakamlar kurumsal akış lisansı gerektiriyor." / "Yesterday, net
  institutional buying in THYAO clearly exceeded the recent average: money inflow. Volume
  confirmed it today. Whether it still holds is unknown. Historically, this kind of signal
  mostly played out. Server-verified; the figures require an institutional flow data
  license."
- Gerçekleşen beklenti (s66): "THYAO'da RSI 3 bar içinde 70'in üstüne çıkarsa aşırı alım
  teyit olur. Beklenti bugün gerçekleşti. Koşulun bugünkü durumu bilinmiyor. …"

Tam liste: `rounds/R3/experiments/M2/raw/template/auto.json` (73 durum × 2 dil).

## 4. Otomatik ölçüler (`harness m2 measure --alt template`)

| ölçü | tr | en |
|---|---:|---:|
| zaman doğru (73) | **1,00** | **1,00** |
| — olay / süren / biten / beklenti | 1,00 / 1,00 / 1,00 / 1,00 | 1,00 / 1,00 / 1,00 / 1,00 |
| K26 zaman doğru | **evet** | **evet** |
| tam zaman başvurusu (olay + biten durum, 35) | 0,83 | 0,83 |
| — yalnız açık (rakamlı) durumlarda | 1,00 (29/29) | 1,00 (29/29) |
| belirleyicilik (bayt eşit) | 1,00 | 1,00 |
| sızıntı / redacted | **0/18** | **0/18** |
| uydurma sayılı cümle / açık | **0/55** | **0/55** |
| boş cümle | 0 | 0 |
| ns/cümle (saat ölçüsü, belirleyici değil; release, bu makine) | ~1 600–2 100 | ~1 000 |

Tam zaman başvurusu kaçan 6 durumun 6'sı da rakamsız (lisanslı) ve bar-önce ≥ 2: "birkaç gün
önce", "yaklaşık bir hafta önce" bilerek tam sayı vermez. Rakamsız kuralının bedelidir.

Crate testleri (`cargo test --release`, 14 test): K26 iki dilde bayt bayt; K26'nın rakamsız
biçimi; tür × yön × zaman türü × 9 aşama × `still_holds` (3) × bar-önce (0, 1, 2, 5, 12,
22, 30, 100) × karne (5) = 35 640 rakamsız birleşimde rakam yok, düzenek sözlüğündeki sayı
sözcüğü yok, lisanslı değerin yuvarlanmış biçimi yok; aynı kümede sınıf başına zaman
işaretleri; belirleyicilik; açık birleşimlerde her sayının bir motor yuvasına eşitliği;
biçimbilim tabloları (sayı, sembol, ad, fiil).

Okuyucu testi (anlama) bu dalda **koşulmadı**; görev gereği orkestratör koşacak.

## 5. Zayıflıklar (dürüstçe)

- **Uzunluk ve tekdüzelik.** Cümle 3–5 kısa cümleden oluşur (ana yüklem + aşama + geçerlilik
  + karne + sunucu). Bilgi eksiksiz ama ses mekanik; her olayda "Hâlâ geçerli olup olmadığı
  bilinmiyor" tekrar eder. Düzenek doğrusu bunu istiyor (olayda `still_holds = None` →
  `unknown`), ama gerçek kullanıcıya bu kuyruk gereksiz gelebilir. Çeşitleme yok (bilerek:
  belirleyicilik ve sınanabilirlik için).
- **"Gerçekleşti" + "bilinmiyor".** Gerçekleşen olay/beklentide hem "beklenen sonuç
  gerçekleşti" hem "hâlâ geçerli olup olmadığı bilinmiyor" denir; doğru ama okuyucuya ters
  gelebilir.
- **Rakamsız zaman bulanık.** Lisanslı olgularda "birkaç gün önce" gibi kovalar tam bar
  sayısını vermez (tam zaman başvurusu 0,83). Bar-önce lisanslı bir değer değil; rakamsız
  kuralı gevşetilirse (ör. yalnız zaman için rakam) bu kayıp kalkar — karar ana akışın.
- **Kalıp kapsamı elle büyür.** Yeni bir dedektör ya da kanıt anahtarı için olay yüklemi ve
  eşik adı tablosuna satır gerekir; bilinmeyen anahtar genel adla ("eşik") söylenir, kötü
  okunmaz ama zayıflar. Altı tür × iki dil için bugünkü tablo ~60 kalıp parçası.
- **Birim yok.** Akış/hacim değerleri birimsiz yazılır (kanıtta birim yok: lot mu TL mi
  bilinmiyor).
- **Sıkışma adı** "sıkışma (yön belirsiz)" parantezi her yerde taşınır; doğal değil ama yön
  bilgisini kaybetmemek için.
- **"Dün" takvimi UTC gününe bakar.** Bar zaman damgası İstanbul gece yarısına yakınsa yanlış
  gün çıkabilir; hafta sonu boşluğunda doğru biçimde "1 bar önce"ye düşer. Gün içi seride
  "bugün" yerine "bu barda" denir.
- **Sembolde rakam** (ör. `A1CAP`) rakamsız kipte güvenlik ağını tetikler: cümle genel ve
  zamansız bir cümleye iner. Düzenek kümesinde böyle sembol yok.
- **Karne cümlesi** ufku ve yönü söylemez ("beklendiği gibi sonuçlandı"); CAPSULE §5
  örneğindeki "on bar içinde yukarı" kısmı yok, çünkü ufuk hikâyeye göre değişebiliyor ve
  karne anahtarının ufku girdide yok. Hukuki dil (A4) denetlenmedi.
- **Niteliksel karne eşikleri** (0,6 / 0,4) ve `LEDGER_MIN_N = 20` ölçülmeden seçildi.
- **Düzenek eserleri:** düzenekte lisans her türe `flow/…` yetkisi verdiği için RSI olgusunda
  da "kurumsal akış lisansı" yazılır; gerçek veride yetki anahtarı doğru olunca düzelir.
- EN tarafında `'s` iyeliği sembolde mekaniktir ("PGSUS's"); doğru ama göze takılabilir.
