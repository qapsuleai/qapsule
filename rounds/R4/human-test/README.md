# R4 — İnsan testi v2 paketi (GOAL ölçütü 12)

Gösterim (`scripts/demo.sh`) üzerinden, telefon boyutunda ve gerçek hızda beş karşılaştırma:
alfabe `siluet`/`rozet` (A21), kapsülün yüzü (M8'in ilk iki sırası), akış `elikov`/`vakiro`
(M9), geçiş `zumlak`/`tutamir` (M10), sonuç işareti `tg5`/`r3` (M11). Persona ajanları ön
elemeydi; **persona ajanı insan değildir** — son söz bu testtedir.

**Yüz adları tek yerde:** `plan.json` → `"faces": {"Y1": "vadrel", "Y2": "fotolay"}` (M8 ilk iki, `rounds/R4/experiments/M8/RESULTS.md`;
yer tutucular 2026-11-16 dolduruldu). `node prepare.mjs key` yüzler gösterimde kayıtlıyken yeniden koşulur (anahtar
değişmez, `links.html` dolar). Başka hiçbir dosyada yüz adı yazmaz; belgeler `Y1`/`Y2` der.

## İçindekiler

| Dosya | Ne |
|---|---|
| `GUIDE.md` | Yürütücü yönergesi: gösterimi açma (telefondan), hazırlık, ~20 dakikalık akış, aynen söylenecekler, sıra tablosu, söylenmeyecekler, onam/gizlilik, kayıt |
| `FORM.md`, `form.html` | Gözlem formu ve yanıt kartı (basılır); CSV biçimi |
| `plan.json` | Oturumun tek tanımı: yüz adları, taban ayarlar, karşılaştırmalar, görev sırası, her maddenin adımı/satırı/kapsülü, G2 kitleleri ve pencereleri |
| `prepare.mjs` | Oturum provası ve anahtar çıkarma (`key`), aday arama (`scan`, `scan-g2`) |
| `key.json`, `key.md` | Cevap anahtarı (sentetik pazar; yürütücü, katılımcı görmez) |
| `links.html` | Yürütücünün telefonda açtığı tarafsız kodlu bağlantılar (U0–U10) ve "son bakış" yer imi |
| `score.py`, `SCORING.md` | Puanlayıcı (CSV → özet + karar; belirleyici) ve kuralları |
| `DECISION.md` | Önceden yazılmış karar kuralı: A21 ve yüz nasıl kapanır, M9–M11 nasıl onaylanır/değişir |
| `answers.example.csv`, `answers.example.scored.md`, `make_example.py` | Uydurma örnek ve puanlanmış hâli (karar kuralının dallarını gösterir; hiçbir şey ölçmez) |

## Gösterimde denenen ayarlar (2026-11-16, `r4/int` 0ca4ce8, `qp demo --synthetic`, başsız Chromium 390×844)

| Ayar | URL | Ayarlar sayfası | Sonuç |
|---|---|---|---|
| alfabe `siluet` / `rozet` | ✓ (`look.alphabet`) | ✓ | seçilebilir |
| yüz `r3` | ✓ | ✓ (tek seçenek) | bugün kayıtlı tek yüz `r3`; M8 alternatifleri `r4/int`'te yok (`choices().face = ["r3"]`). Bilinmeyen ad (`?face=nope`) sessizce yok sayılır, varsayılan yüz çizilir |
| akış `elikov` / `vakiro` | ✓ | ✓ | seçilebilir (grup başlıkları ve işaretler görünür) |
| geçiş `zumlak` / `tutamir` | ✓ | ✓ | seçilebilir; kapsül → grafik açılması ikisinde de çalıştı |
| sonuç işareti `r3` / `tg5` | ✓ | ✓ | seçilebilir |
| tema `light`/`dark`, dil `en`/`tr` | ✓ | ✓ | seçilebilir |
| hepsi birden (`?face=r3&alphabet=rozet&feed=vakiro&transition=tutamir&outcome=tg5&theme=light`) | ✓ | — | `look` hepsini taşıyor |

Kalıcılık: URL ile verilen ayarlar ileri/geri sarmada ve sayfa yenilemede kalır; oynatma
konumu barındırıcıda durur (yenilemede kalır); kitle (izleme listesi, pozisyon) sayfa yenilemede
kalır, geri sarmada silinir. **Ayarlar sayfasında yapılan seçim geri sarınca kaybolur** (bağlantı
yeniden kurulur, URL değerine döner). Adıma gitme 0,3–0,7 sn (adım 442: 0,74 sn). Sayfa hatası yok.

## Açık sorular (orkestratöre)

1. **Son bakış arayüzde yok.** M9'un "son baktığından beri" görevi için gereken son bakış yalnız
   `window.qp.seen()` (düzenek API'si) ile konuyor; kullanıcı arayüzünde düğme ya da URL
   parametresi yok. Paket telefon yer imi (`javascript:qp.seen()`) ile ve masaüstü yedeğiyle
   çalışıyor; sağlamı ürüne küçük bir ek (Ayarlar'da "son bakış: şimdi" ya da `?seen=`), ayrıca
   kitle için URL parametresi (`?watch=&long=`). İnsan testinden önce önerilir.
2. **Pozisyondaki olumsuz haber akışa girmeyebilir.** `scan-g2` ile taranan pencerelerin
   çoğunda elde tutulan hissenin yeni doğup aynı barda geçersizleşen kapsülü 40 satırlık okuyucu
   akışında yoktu (iki düzende de; ör. adım 762–771 SYN12, 906–915 SYN12 "zayıfladı", 801–810
   SYN01). K42 "olumsuz geçişler hassasiyetten bağımsız" ile çelişiyor olabilir; nedeni
   (`capsules` sorgusunun sayfa sınırı mı, süzgeç mi) incelenmeli. Plan bu yüzden 2. pencere için
   kitleyi değiştirdi (SYN19, SYN01; adım 882–891).
3. **Sentetik akış durağan ve tekdüze** (`SCORING.md` "Sınırlar"): taban akışın üstü yüzlerce adım
   aynı, neredeyse hepsi aşağı kırılım. G1'de yön/tür soruları ayırt etmez. Yerel gerçek veriyle
   daha çeşitli bir plan kurulabilir (`prepare.mjs scan`), ama anahtar ve plan o zaman commit
   edilemez.
4. **M8 yüzleri gelince** satır numaraları değişmez (akış sırası yüze bağlı değil), ama metin
   örtüsü (G4/G5) yeni yüzün işaret yerine göre kesilmeli; çok uzun yüzde "son üç" tek ekrana
   sığmayabilir (yürütücü kaydırır).
5. G1'in "ilk bakış" sorusu yüz başına 5 saniyedir; katılımcı alt kapsülleri de görür (gerçek
   kullanım gibi). Önceki kapsülü bir kez daha görmek mümkün (aynı ekranda sonraki madde).

## Nasıl yeniden üretilir

```sh
cargo build --release -p qp-cli && scripts/demo.sh --synthetic --paused   # bir kez (pkg/ için), sonra durdurun
(cd shells/web && npm ci)
cd rounds/R4/human-test
node prepare.mjs key            # ~2 dk; key.json depodakiyle bayt eşit olmalı
python3 -I make_example.py && python3 -I score.py answers.example.csv > answers.example.scored.md
```

Planı kurmak için: `node prepare.mjs scan --from 1 --to 600 --stride 9 --rows 8` (adım başına üst
satırların durumu), `node prepare.mjs scan-g2 --from 600 --to 1000` (G2 penceresi; `--plan` ile başka
kitle). Seçim gerekçesi: her sette bir açık ve iki kapanmış kapsül (G1), her ekranda karışık
sonuç (G4: 1 tutmadı/2 gerçekleşti, sonra 1/2 tersi), G5'te yön ve tür karışık ve bir açık kapsül,
G2'de olumsuz haber iki düzenin akışında da görünür. Gerçek veri kullanılmadı; ekran görüntüsü
commit edilmedi.
