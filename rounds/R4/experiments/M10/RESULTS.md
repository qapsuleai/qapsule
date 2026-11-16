# R4/M10 — Açılma geçişi (cümle → kapsül → grafik): sonuç

> Persona ajanı insan değildir; model küçük pikseli ve kısa bakışı insan gibi deneyimlemez.
> Bu ölçüm **ön elemedir**; son söz insan testi v2'dedir (SPEC §0.5).

## Düzen ve sıra (VERIFY 14)

| Adım | Commit | Dal |
|---|---|---|
| Şartname (SPEC §4, seçim kuralı) | `bad3d67`, ekler E1–E5 `5b327e5` | `r4/int` |
| Düzenek + `TransitionDesign` arayüzü + başvuru `r3` ölçüsü | `6b4b581` | `r4/h-m8910` → `r4/int` |
| Alternatif A: `tutamir` ("cümle yerinde kalır, işaret olgunun yerine uçar") | `f48559b` | `r4/m10-a` |
| Alternatif B: `zumlak` ("açılma bir yakınlaşma: ortak bar × fiyat penceresi") | `da0da52` | `r4/m10-b` |
| Değerlendirme dalı (`r4/int` `7d7cae1` + iki birleştirme) | `389e169`, `b3280cf` | `r4/eval-m10` |
| Okuyucu koşuları, puanlama, bu not (+ düzenekte `M8910_PORT`) | bu commit | `r4/eval-m10` |

İki alternatif ayrı alt ajanlarca, ayrı worktree'lerde yazıldı; bu değerlendirme onların
kodunu değiştirmedi. Düzenekteki tek değişiklik: `m10` gösterim portu `M8910_PORT` ortam
değişkeniyle seçilebilir (varsayılan 8472 aynı; ölçülere, seçim kuralına dokunmaz; README).
Bu koşuda ekranlar `M8910_PORT=8574` ile alındı. `r3/computed.json` yeniden üretildi ve
commit edilmiş hâliyle bayt eşit çıktı.

## Körleme

Ekranlar tohumlu karma kodla adlandırıldı (`r3` = `2760beb1`, `tutamir` = `f75a83ba`,
`zumlak` = `06489420`; eşleme yalnız `raw/key.json` ve `raw/keys/`'te, okuyucu dizinine
kopyalanmadı). Okuyucu: taze `claude -p --model sonnet --tools Read --allowedTools Read
--restricted --strict-mcp-config --permission-mode dontAsk --no-session-persistence
--output-format json`, çalışma dizininde yalnız o okuyucunun 16 PNG'si. Her istem
gönderilmeden önce `m8910 check-prompt` ile yasak sözcük denetiminden geçti (18/18 geçti).
Dizilerin sırası okuyucu başına tohumlu (`meta.json` `order`). Kanıt olarak bir istem:
`rounds/R4/experiments/M10/raw/readers/F_f75a83ba_r1/prompt.txt` (tasarım adı, alternatif
sayısı, önceki sonuç geçmez).

## Durumlar (SPEC §4, §8 E3)

| sıra (dosya `i`) | S8 | kapsül | anahtar G1 / G2 / G3 |
|---|---|---|---|
| 1 | #6 kapandı: tutmadı | `cap.SYN25.breakout.down.26` | olgu barı × seviye / evet / kapandı: tutmadı |
| 2 | **#101** (#1'in yedeği) | `cap.SYN06.breakout.up.118` | … / evet / evet, sürüyor |
| 3 | **#112** (#12'nin yedeği) | `cap.SYN28.reversal.down.120` | … / evet / evet, sürüyor |
| 4 | #3 artık geçerli değil | `cap.SYN06.extreme.up.123` | … / evet / hayır, artık geçerli değil |

**E3:** SPEC #1 (bugün, yukarı kırılım) ve #12 (teyitli aşağı dönüş, dün) yalnız düzeneğin
saatlik dünyasında bulunur; canlı gösterim onları gösteremez. M10 gösterim yedeklerini
kullandı: #101 = yukarı kırılım, açık, **dün** (bugün değil), adım 10; #112 = aşağı dönüş,
açık, dün, **teyitsiz**, adım 10 (`s8.json` `substitutes`). Tür ve yön aynı; "bugün" ve
"teyit" farkı bütün tasarımlar için aynı.

## Ölçüler

**(a) Hesap** (kapsül → grafik; `raw/<ad>/computed.json`; dört durumun ortalaması):

| tasarım | ortak öğe | alan payı önce | alan payı sonra | kayma ort. (px) | dokunma (akış → grafik) |
|---|---:|---:|---:|---:|---:|
| `r3` (başvuru) | 3,5 | 0,2803 | 0,6690 | 124,86 | 2 |
| `tutamir` | 3,5 | 0,2803 | 0,6690 | 129,71 | 2 |
| `zumlak` | 3,5 | 0,1544 | 0,5696 | 135,14 | 2 |

`zumlak`'ın alan payı düşüktür çünkü bütün fiyat paneli (`price`) yerine yalnız iki sahnede
de görünen pencereyi (`span`) ortak sayar (DESIGN.md); tanım farkıdır, okuyucuya görünen bir
eksik değildir. Durum başına değerler `computed.json`'da.

**(b) Persona** (persona × 3 okuyucu × 4 dizi = persona başına 12 cevap; ağırlık F 2, H 1):

| tasarım | F G1 | F G2 | F G3 | H G1 | H G2 | H G3 | ağırlıklı G1 | ağırlıklı G2+G3 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `r3` | 0,9167 | 1,00 | 0,3333 | 0,9167 | 1,00 | 0,4167 | 0,9167 | 0,6806 |
| `tutamir` | 1,00 | 1,00 | 0,3333 | 1,00 | 1,00 | 0,3333 | 1,0000 | 0,6667 |
| `zumlak` | 1,00 | 1,00 | 0,3333 | 1,00 | 1,00 | 0,5000 | 1,0000 | 0,6944 |

Şans düzeyi: G1 0,25, G2 0,33, G3 0,17. Teknik düşme yok (18/18 okuyucu geçerli JSON
döndürdü; yeniden koşu gerekmedi). Ham kayıtlar `raw/readers/`, puanlar
`raw/scores.{json,md}`; `m8910 score --exp m10` ham dosyalardan bayt eşit yeniden üretti.

## Seçim kuralının uygulanışı (SPEC §4, önceden yazılı; `r3` sıralanır, seçilmez)

1. **Birincil — ağırlıklı G1:** `tutamir` 1,0000, `zumlak` 1,0000; fark 0,0000 < 0,05 →
   eşitlik bozucuya.
2. **G2+G3 (yüksek):** `zumlak` 0,6944, `tutamir` 0,6667 → **`zumlak`**. Kural burada karar
   verdiği için sonraki adımlar (ortak öğe alan payı: `tutamir` 0,669 > `zumlak` 0,570;
   dokunma: ikisi 2) uygulanmadı.
3. Sıralama: `zumlak`, `tutamir`, `r3` (başvuru; G1 0,9167).

**Seçim (öneri): `zumlak`** — kuralın harfiyle. Ama aşağıdaki gözlemler kararın persona
ölçüsünce **ayırt edilmediğini** gösteriyor: insan testi v2'de iki alternatif birlikte
denenmeli; bu sonuç `tutamir`'i elemek için yeterli değildir.

## Gözlemler

- **Tavan etkisi (G1, G2).** G1'de iki alternatif de 1,00; G2 her tasarımda 1,00. Grafik
  ekranı bütün tasarımlarda aynıdır (`depth::chart_view`): olgunun barı vurgulu sütunla,
  seviyesi turuncu çizgiyle çizili ve doğru etiket tam bu kesişimdedir; üstte de kapsülün
  cümlesi durur. Okuyucu geçişe bakmadan son ekrandan doğru cevabı bulabiliyor. `r3`'ün iki
  G1 hatası (#101, F ve H birer) geçişten değil etiket yerleşiminden: `r3`'ün tohumlu
  çeldiricisi C kırılan seviye çizgisinin üstüne düştü ("B kırılım çubuğu, C kırılan
  seviyenin ortası, arada kaldım"). Çeldirici yerleri tasarım koduna göre tohumlandığı için
  tasarımlar arasında aynı değil; G1 farkı bu yüzden geçişe bağlanamaz.
- **Ara kareleri hiç açmayan okuyucular.** 18 okuyucudan 4'ü (F persona; `r3` 2, `tutamir`
  1, `zumlak` 1) yalnız kart ve grafik ekranlarını okudu (`num_turns` 9; ikisi "ara kareleri
  açmadım" diye yazdı). F tarifinin "birkaç saniye, uzun uzun inceleme" yönergesiyle tutarlı;
  ama geçiş bu okuyucuların cevabına hiç girmedi.
- **Kararı veren fark gürültü düzeyinde ve geçişle ilgisiz.** G2+G3 farkı (0,028) yalnız H
  personasının #112'deki G3 cevaplarından geliyor (`zumlak` 3/3, `tutamir` 1/3, `r3` 2/3).
  Kartın cümlesi burada "Hâlâ geçerli olup olmadığı bilinmiyor" der; anahtar "evet, sürüyor".
  Okuyucular cümleye uyunca "bilmiyorum" diyor (F 8/9). G3 metinden okunuyor, geçişten değil.
- **G3 anahtarı ile ürün cümlesinin çatışması (bütün tasarımlarda aynı):**
  #6'da cümle "Sinyal 90 bar önce geçersiz oldu; artık geçerli değil" der, anahtar
  "kapandı: tutmadı": 18 okuyucunun hiçbiri anahtarı seçmedi (15 "hayır, artık geçerli
  değil", 3 "süresi doldu ya da geri çekildi"). #3'te cümle "Aşırı alım sona erdi" der,
  anahtar "artık geçerli değil": H 9/9 "evet, sürüyor". #112 yukarıda. G3 bu turda geçişleri
  ayıramaz; düzenek anahtarı ile kapsül cümlesinin dili ayrıca hizalanmalı (Öneriler).
- **RSI paneli yok (bütün tasarımlarda aynı).** #3 (RSI aşırılığı) grafik ekranında RSI alt
  paneli görünmüyor; H okuyucularının çoğu bunu yazdı ve doğru etiketi yalnız vurgulu
  sütundan buldu (README Açık sorular 3: grafik işaretlemesi geçişe açık değil).
- **`tutamir` bilinen zayıflığı — ara karede iki işaret.** `p = 2/3` karesinde uçan işaret
  (olgu noktası) ile grafiğin kendi işareti (sütun tepesi) birlikte görünüyor (ör.
  `raw/screens/f75a83ba-2-3.png`). Hiçbir okuyucu bundan söz etmedi ve G1'de hata yok; bir F
  okuyucusu son ekranda "kartın simgesinin bir kopyası"nı vurgulu sütunun dibinde gördüğünü
  yazdı (doğru cevaba yardım etti). Bu ölçüm zayıflığı ne doğruladı ne çürüttü.
- **`zumlak` bilinen zayıflığı — olgu kapsülün aralığı dışında (#6).** Bu durumda yakınlaşma
  tersine döner (uzaklaşma), işaretli an ilk ara karede ekranda yok. Okuyucuların hepsi #6'da
  G1'i doğru verdi (F 3/3, H 3/3); kimse ara kareden söz etmedi. Son ekran yeterli olduğu
  için bu zayıflık da ölçülemedi.
- **Sonuç:** geçişlerin farkı okuyucu cevaplarına yansımıyor; okuma testi (bu soru
  kümesiyle) geçişi ölçmüyor, son ekranı ölçüyor. İki alternatif de `r3`'ten kötü değil;
  hesap ölçülerinde `tutamir` `r3` ile aynı alan payında, `zumlak` tanımı gereği düşük.
  Geçişin değeri (süreklilik, "aynı şeye bakıyorum" hissi) insanda zamanlı izlemeyle
  (canlı geçiş, gerçek hızda) ölçülmeli.

## Maliyet

18 okuyucu (3 tasarım × 2 persona × 3), model `sonnet`: toplam ≈ 1,86 $ (`r3` 0,60, `tutamir`
0,63, `zumlak` 0,62; `answer.json` `total_cost_usd`). Eşzamanlılık en çok 3.

## Öneriler (kapsam dışı)

- M10 G3 anahtarını kapsülün kendi geçerlilik cümlesiyle hizala (#6 "geçersiz oldu; artık
  geçerli değil" ↔ "kapandı: tutmadı"; #112 "bilinmiyor" ↔ "sürüyor"; #3 "sona erdi"), ya da
  seçeneklere ürünün sözcüklerini koy. Aynı çatışma M8 S3'te de olabilir.
- Çeldirici yerlerini tasarımdan bağımsız tohumla (durum başına aynı A–D yerleşimi): şimdi
  G1 farkı kısmen etiket yerleşiminden geliyor.
- Geçişi ayırt edecek bir soru: ara kareden sonra son ekran **olmadan** "işaret nereye
  gitti?" (ör. `p = 2/3` karesinde A–D), ya da okuyucuya ara kareleri açmayı zorunlu kılan
  sıralı gösterim.
- Düzenekte sabit portlar (`m9::PORT` 8471 da) için boş port seçimi; `shells/web/pkg/sizes.json`
  yokken `demo_shots.mjs` 404 ile durur — elle wasm-pack derlemesinden sonra yazılmalı
  (bu koşuda `scripts/demo.sh`'in biçimiyle elle yazıldı).
