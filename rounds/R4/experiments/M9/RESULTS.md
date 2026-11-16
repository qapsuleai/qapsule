# R4/M9 — Akışın düzeni: sonuç

> Persona ajanı insan değildir; model küçük pikseli ve kısa bakışı insan gibi deneyimlemez.
> Bu ölçüm **ön elemedir**; son söz insan testi v2'dedir (SPEC §0.5).

## Düzen ve sıra (VERIFY 14)

| Adım | Commit | Dal |
|---|---|---|
| Şartname (SPEC §3, seçim kuralı) | `bad3d67` (12:04) | `r4/int` |
| SPEC ekleri E1–E5 | `5b327e5` (15:00) | `r4/int` |
| Düzenek arayüzü (`feeds.rs` `LAYOUTS`, bakış ayarı `feed`/`seen`) | `464dba4` (14:51) | `r4/h-m8910` → `r4/int` |
| Düzenek (`m9`, senaryo, `run_readers.sh`, `score`) + başvuru `r3` | `6b4b581` (14:51) | `r4/h-m8910` → `r4/int` |
| Alternatif A: `elikov` ("kişiye göre öncelik", `rules` 5) | `5f155af` (15:46) | `r4/m9-a` |
| Alternatif B: `vakiro` ("zamana göre bölümlü akış", `rules` 4) | `fdc933d` (15:35) | `r4/m9-b` |
| Değerlendirme birleştirmesi (`r4/int` `b02113a` + A + B) | `f1137c8`, `5f43125` | `r4/eval-m9` |
| Okuyucu koşuları, puanlama, bu not | bu commit | `r4/eval-m9` |

Düzenek commit'i (`6b4b581`) iki alternatifin de atasıdır (`git merge-base --is-ancestor`
doğrulandı). İki alternatif yalnız `crates/qp-grammar/src/feeds.rs` (tek satır kayıt +
`mod`), kendi `feeds/<ad>.rs` dosyası ve kendi `DESIGN.md`'sine dokundu; düzeneğe ve kabuğa
dokunmadı. Okuyucu ve puanlama koşusu bu dalda, düzenek değiştirilmeden yapıldı.

## Körleme

- Okuyucunun gördüğü yalnız görüntülerdir: her tasarımın 6 ekranı, tohumlu karma kodla
  adlandırılmış (`raw/screens/<kod>-<k>.png`; `r3` = `1ffa7657`, `elikov` = `5e000c35`,
  `vakiro` = `c6640e72`). Eşleme `raw/key.json` ve `raw/keys/`'te; okuyucu dizinine
  kopyalanmadı (`run_readers.sh` yalnız `images.json`'da listelenen PNG'leri yeni geçici
  dizine kopyalar).
- Okuyucu: taze `claude -p --model sonnet --tools Read --allowedTools Read --restricted
  --strict-mcp-config --permission-mode dontAsk --no-session-persistence --output-format json`;
  hepsinde model `claude-sonnet-5-5` (cevapların `modelUsage` alanı).
- Her istem gönderilmeden önce `m8910 check-prompt` ile yasak sözcük denetiminden geçti
  (tasarım adları, dal adları, `alt-`, `variant`, `feed` …); altı istem şablonu ayrıca elle de
  denetlendi (hepsi geçti). Okuyucu tasarım sayısını, adını, kimin yaptığını bilmez; her
  okuyucu tek bir tasarımın ekranlarını görür.
- Kanıt: gönderilen birebir istem, ör.
  `rounds/R4/experiments/M9/raw/readers/F_c6640e72_r1/prompt.txt` (şablon:
  `raw/prompts/F_c6640e72.txt`). Görüntü yolları geçici dizindir; istemde tasarım adı yok.
- Not: ekran görüntüsünde düzenin kendi başlıkları ("Elindeki hisselerde yeni", "Son
  baktığından beri · dün 15:00") ve işaretleri ("yeni · tutmadı", "pozisyonun") görünür —
  bunlar tasarımın kendisidir, körlemeyi bozmaz.

## Senaryo ve SPEC §8 ekleri

- Senaryo (`raw/scenario.json`, düzenek commit'inde hazır; yeniden üretilmedi): gösterimin
  sentetik oynatması, `t0` = adım 600 (son bakış), `t1` = adım 609 (2023-09-20 15:00, bir
  işlem seansı sonra). Pozisyon `SYN09`, `SYN12` (olumsuz geçişli: `SYN09`), izleme listesi 8
  (`SYN02 SYN03 SYN04 SYN05 SYN22 SYN25 SYN26 SYN28`), `balanced`, kitle `habituated`.
- Anahtar (üç tasarımda aynı, motordan): G1 = `SYN09 yukarı` (tek geçiş: `SYN09` para girişi
  kapsülü doğdu ve aynı barda geçersiz oldu; yön kapsülün yönü, README sapma 6); G2 =
  `SYN09` / "tutmadı, geçersiz oldu"; G3 = en yüksek güven kovası (≥ %60, n ≥ 5):
  `cap.SYN25.breakout.down.26` (4/5) ve `cap.SYN22.continuation.down.108` (20/33).
- **Yüz sabit:** başvuru `r3` yüzü (gösterimin R3 kapsül kartı: cümle + karne + küçük grafik).
  M8 henüz seçilmedi; SPEC §3 "M8 seçilmeden önce koşulursa düzeneğin başvuru yüzü".
- Ekler: E1 (S4 "az örnek" kovası `n < 20`) M8 içindir; M9'un G3 anahtarı düzenekte hâlâ
  `n ≥ 5` kullanır (bkz. Zayıflıklar 2). E2–E5 M9'u etkilemez (E3: M9 yalnız gösterim
  dünyasında).
- Ekranlar: canlı gösterim (`qp demo --synthetic`), 390×844 CSS px, DPR 2, açık tema; en çok
  6 ekran (üç tasarımda da sınıra ulaşıldı: akışta 40–46 kapsül, 6 ekranda 17'si görünür).

## Ölçüler

Persona × tasarım × 3 okuyucu = 18 koşu; hepsi ilk denemede başarılı (`is_error: false`, boş
`stderr`); teknik nedenle yeniden koşulan yok. Ham kayıtlar `raw/readers/`, puanlar
`raw/scores.{json,md}` (`m8910 score --exp m9`, ham cevaptan belirleyici).

| tasarım | persona | G1 F1 | G2 | G3 | doğruluk | kaydırma (doğru cevaplarda) | dokunma |
|---|---|---:|---:|---:|---:|---:|---:|
| `r3` (başvuru) | F | 0,00 | 0,00 | 0,00 | 0,000 | — | 0 |
| | H | 0,00 | 0,00 | 0,00 | 0,000 | — | 0 |
| `elikov` | F | 0,00 | 1,00 | 1,00 | 0,667 | 0,83 | 0 |
| | H | 0,00 | 1,00 | 1,00 | 0,667 | 1,67 | 0 |
| `vakiro` | F | 0,33 | 1,00 | 0,00 | 0,444 | 0,00 | 0 |
| | H | 0,67 | 1,00 | 0,00 | 0,556 | 0,20 | 0 |

| tasarım | ağırlıklı doğruluk (F2, H1) | ağırlıklı kaydırma + dokunma | `rules` | anahtardan en az kaydırma (G1/G2/G3) |
|---|---:|---:|---:|---|
| `elikov` | **0,667** | 1,11 | 5 | 0 / 0 / 0 |
| `vakiro` | 0,481 | 0,07 | 4 | 0 / 0 / 1 |
| `r3` | 0,000 | — | 0 | yok / yok / 0 |

"yok": gerekli kapsül ilk 6 ekranda değil. Dokunma: hiçbir okuyucu "açmam gerekirdi" demedi;
düzenlerin kendi gerektirdiği dokunma 0 (grup açma yok).

## Seçim kuralının uygulanışı (SPEC §3, önceden yazılı; değiştirilmedi)

1. Başvuru `r3` sıralanır, seçilmez → aday: `elikov`, `vakiro`.
2. Birincil: üç görevin ortalama doğruluğu, `F` 2, `H` 1. `elikov` 0,667 − `vakiro` 0,481 =
   **0,185 ≥ 0,05** → birincil ölçü karar verir; eşitlik bozucular (kaydırma + dokunma,
   `rules`) uygulanmaz.
3. Sıra: `elikov`, `vakiro`, (`r3`). `top2` = `elikov`, `vakiro`.

**Seçim (öneri): `elikov`.** İki alternatif de `feed` ayarıyla seçilebilir kalır; insan testi
v2'ye ilk iki (`elikov`, `vakiro`) gider. Varsayılan değişikliği bütünleştirmede yapılır.

**Duyarlılık (kayıt için, seçimi değiştirmez):** G1 bu senaryoda tasarımı değil, para girişi
kartının yönünün yorumunu ölçtü (aşağıda 1). G1 yalnız enstrümana göre puanlansaydı
`elikov` 1,000, `vakiro` 0,667, `r3` 0,000 olurdu; G3 E1'e uygun anahtarla (`n ≥ 20`)
puanlansaydı sonuç aynı kalırdı (`elikov`'un okuyucularının seçtiği 20/33 kart doğru,
`vakiro`'da o kart 6 ekranın dışında). Seçim iki okumada da `elikov`.

## Gözlemler ve zayıflıklar

1. **G1 "yön" tasarımları ayırmıyor.** İki alternatifin 12 okuyucusunun 12'si de değişen
   enstrümanı doğru buldu (yalnız `SYN09`; kesinlik ve duyarlılık enstrüman düzeyinde 1). Fark
   yalnız yönde: anahtar kapsülün yönü (`yukarı`); okuyucular "hacim artışı yön söylemez"
   diye çoğunlukla `yönsüz` dedi (`elikov` 6/6 `yönsüz`, `vakiro` 3/6 `yukarı`). Kart "para
   girişi" der ve sinyal aynı barda geçersiz olmuştur; `yönsüz` savunulabilir bir okuma.
   `vakiro`'nun G1 üstünlüğü düzenden değil, bu yorum gürültüsünden. Düzenek için öneri:
   M9 G1'i enstrüman düzeyinde (ya da yön seçeneğine "yön belirtilmemiş" eşdeğerliğiyle)
   puanlamak.
2. **G3 anahtarı ürünün eşiğiyle çelişiyor.** Anahtar `n ≥ 5` ister; 4/5'lik kart doğru
   sayılır. Ürünün cümlesi o kartta "Karnesi henüz az örnekli" der (E1'in gerekçesi). `vakiro`
   okuyucuları 4/5 kartı (ekran 2) bu yüzden geçip en büyük örnekli kartı (110/203, %54)
   seçti; 20/33'lük kart `vakiro`'da 6 ekranın dışında kaldı. `vakiro` iki anahtarla da 0
   alır, ama "en az kaydırma 1" (DESIGN.md) yalnız `n ≥ 5` anahtarıyla doğrudur.
3. **`elikov` DESIGN.md zayıflıkları:**
   - "Pozisyonun eski haberleri üçüncü gruba karışır; okuyucu G2'de bunları da sayabilir" —
     **görülmedi**: 6/6 okuyucu yalnız `SYN09` dedi; üç H okuyucusu `SYN12`'nin kartlarını
     açıkça "eski, zaten sonuçlanmış" diye ayırdı. (Aynı tuzak `r3`'te görüldü: F okuyucularının
     2/3'ü eski geçersiz kartlara bakıp "ikisinde de" dedi.)
   - "Az örnekli yüksek oranlı kapsül aşağıda kalır" — G3'ü etkilemedi; 6/6 okuyucu üçüncü
     grubun başındaki 20/33 kartı seçti, üçü "örneklem küçük" çekincesi yazdı.
   - Niyet/pozisyon başlığı darlığı, kalabalık grup 1: senaryoda sınanmadı (pozisyon dışı yeni
     geçiş yok, grup 2 boş).
   - Kaydırma 1,11: okuyucuların bir kısmı G3 için "bütün ekranlara baktım" diye `ekran` 1–6
     yazdı (anahtardan en az kaydırma 0). Ölçü okuyucunun tarama alışkanlığını ölçüyor;
     birincil karar verdiği için seçimi etkilemedi.
4. **`vakiro` DESIGN.md zayıflıkları:**
   - "G3 bir kaydırma ister" — gerçekte G3'ün `n ≥ 20` kartı 6 ekranın hiç birinde yok; 5/6
     okuyucu ekran 2'deki 110/203 kartını, 1'i 24/41 kartını seçti (G3 0/6).
   - "'Daha önce' 38 kapsüllük tek uzun bölüm; zaman bilgisi ayırt etmez" — okuyucu
     notlarında bölüm adı anılmadı; G3'teki başarısızlık bununla tutarlı (bölüm içi otorite
     sırası güvene göre değil).
   - "'pozisyonun' işareti niyette tam doğru değil" — senaryoda niyet yok; işaret G2'de
     yardımcı oldu (6/6 doğru), bir H okuyucusu işareti andı.
   - "Son baktığından beri · dün 15:00" başlığı G1'i iyi taşıdı (6/6 enstrüman doğru, ekran 1).
5. **Başvuru `r3`:** son bakış işareti yok, olumsuz geçişli `SYN09` kartı ilk 6 ekranda yok;
   6/6 okuyucu "yeni bir şey yok" dedi (en yeni kart "8 bar önce"). Bu, alternatiflerin
   çözmeye çalıştığı sorunun düzenekte doğrulanması: düzen olmadan son bakıştan beri değişen
   pozisyon haberi görünmüyor.
6. **Tavan/şans:** G2 her iki alternatifte tavan (1,00); ayrımı G3 yaptı (`elikov` 6/6,
   `vakiro` 0/6). G3 tek senaryoda iki kapsüle bağlı; sonuç senaryoya duyarlıdır.
7. Yüz cümlesindeki iç terim ("3 bar önce", "bar") okuyucu notlarında sık geçti; bu yüzün
   (M8) işidir, düzenin değil.

## Düzenekte rastlanan sorunlar

- README "Komutlar" `wasm-pack` sonrasında `shells/web/pkg/sizes.json`'ı anmıyor; elle
  `scripts/verify.sh` biçiminde yazıldı (`{"wasm_raw":…,"wasm_gzip":…}`; bu koşuda
  5.468.792 / 1.472.481 bayt). README'ye eklenmeli.
- G1 yön anahtarı ve G3 kovası (yukarıda 1, 2).
- `raw/readers/*/images.json` ve `prompt.txt` yerel mutlak yolları taşır (geçici dizin,
  worktree yolu); içerik sentetik, gizli bilgi yok.

## Maliyet

18 okuyucu, toplam **≈ 1,03 $** (`total_cost_usd` toplamı; `r3` 0,36, `elikov` 0,34,
`vakiro` 0,33), toplam süre ≈ 281 sn (okuyucu başına 11–28 sn, 7–8 tur). Üç akış ekranı
koşusu ve derlemeler yerel.
