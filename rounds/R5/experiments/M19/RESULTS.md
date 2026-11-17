# M19 — Akış önceliği ve gizleme eşiği: sonuçlar

Yazan: R5 `m19-feed` alt ajanı, 2026-11-18. Ölçüm düzeneği ve önceden yazılı seçim kuralı:
`rounds/R5/experiments/harness/feed/SPEC.md` (§4), M19 ürün kodundan önce commit edilmiş.
Yalnız toplu istatistik; gerçek veri `fixtures/local/` (31 enstrüman günlük, 2220 gün; ham çıktı yok).

## 1. Ne koşuldu

| Koşu | Komut | Çıktı |
|---|---|---|
| Bütün ızgara (29 kural × 3 sıralama × K42 muaf/muafsız; sentetik 3 tohum × 30 × 500 gün + gerçek) | `feedh run --out rounds/R5/experiments/M19/harness-run --real` (README; ≈ 2 dk) | `harness-run/RESULTS.md` — kuru koşuyla **bayt bayt aynı** (gövde; belirleyici) |
| Ürün kuralı ↔ düzenek | `m19check --real` (`tool/`; düzenek kitaplığını ve `qp_capsule::feedrule`'u çağırır) | `check-run.md` |
| Ürün yolu testleri | `cargo test -p qp-session --test feed_m19`, `-p qp-wasm --features replica --test feed_m19`, `qp-grammar` `feeds`, `qp-capsule` `feedrule` | §6 |

Testler (düzenek): `no_look_ahead`, `hidden_still_count_in_ledger`, `rules_judge` — geçti.

## 2. Sonuç tablosu (gerçek veri; sıralama `LiveNew`, K42 muafiyetsiz; seçim burada)

| kural | akışta kalan/gün | gizli/gün | boş gün payı | kalanların sonradan tutma oranı | gizlenenlerinki | kalan Brier becerisi [önyükleme %95] | T2 görünür (muafsız) |
|---|---:|---:|---:|---:|---:|---:|---:|
| `none` (başvuru) | 4,95 | 0,00 | 0,014 | 0,556 (778/1399) | — | 0,019 [0,003; 0,036] | 1,000 |
| **`rate τ=0,55 N=20`** | 3,18 | 1,76 | 0,138 | **0,583** (522/896) | 0,509 (256/503) | 0,024 [−0,000; 0,047] | 0,422 |
| `rate τ=0,55 N=10` | 3,17 | 1,78 | 0,139 | 0,582 | 0,509 | 0,020 [−0,004; 0,042] | 0,415 |
| `delta δ=0,05 N=20` | 3,55 | 1,40 | 0,096 | 0,576 | 0,513 | 0,022 [−0,002; 0,042] | 0,666 |
| `wilson N=30` | 3,25 | 1,70 | 0,180 | 0,570 | 0,519 | 0,021 [−0,001; 0,043] | 0,643 |
| `rate τ=0,60 N=20` | 0,80 | 4,14 | 0,606 | 0,616 | 0,551 | 0,024 [−0,093; 0,136] | 0,044 |
| `rate τ=0,70 N=20` | 0,20 | 4,74 | 0,884 | 0,591 | 0,556 | −0,085 | 0,018 |

Sentetikte (bilgi; SPEC: sentetik karneler yazı turaya yakın, kural farkı beklenmez) `none`
8,85/gün, kalan tutma 0,473; `rate τ=0,55 N=20` 1,45/gün, boş gün 0,493, kalan tutma 0,487 —
hiçbir kural `none`'dan ayrılmıyor, çoğu kapıdan geçmiyor. Bütün satırlar: `harness-run/RESULTS.md`.

**N'ye duyarlılık** (gerçek; kalan/gün · kalan tutma · beceri): `rate τ=0,55` N=5 3,15 · 0,581 ·
0,021; N=10 3,17 · 0,582 · 0,020; N=20 3,18 · 0,583 · 0,024; N=30 3,22 · 0,579 · 0,020 — N'ye
neredeyse duyarsız (karne anahtarlarının çoğu ısınmadan sonra n ≥ 30). τ'ya çok duyarlı: 0,60'ta
akışın %61'i boş gün.

## 3. Ürün karnesiyle (doğum karnesi) aynı kural

Ürün kararı **doğum karnesine** bakar (`Story.ledger_at_birth`; kararı doğumda verir, zaman
ilerleyince değişmez, yüzün gösterdiği karne odur). Düzenek her günü o günün sonundaki karneyle
yargılar. `m19check` aynı kabul edilen öğeler üzerinde ikisini karşılaştırır:

- Kararlar: her karne `(n, r)`, `0 ≤ r ≤ n ≤ 400` için ürünün `rate55` ve `none` kuralı düzenekteki
  `rate τ=0,55 N=20` ve `none` ile **aynı** (80 601 karnede 0 ayrışma).
- Gerçek veride ayrışan karar payı (doğum ↔ gün sonu karnesi): `rate55` 0,0146 (157/10 731).
- Doğum karnesiyle akış (gerçek, `rate55`): kalan/gün 3,19 (düzenek 3,18), boş gün 0,138
  (0,138), kalan tutma 0,581 (0,583), gizlenen 0,512 (0,509), T2 görünür muafsız 0,432, **muaf
  1,000**, muaf biçimde kalan/gün 3,74 ve boş gün 0,054.

## 4. Seçim (SPEC §4, önceden yazılı kural)

1. **Kapı** (boş gün ≤ 0,25, kalan/gün ≥ 1): 12 / 28 kural geçti.
2. **Birincil** (kalan Brier becerisi): `rate τ=0,55 N=20` 0,024 [−0,000; 0,047] ↔ `delta δ=0,05
   N=20` 0,022 [−0,002; 0,042] → **ayırt edilemedi** (aralıklar örtüşüyor). Başvuru `none`
   0,019 [0,003; 0,036] ile de örtüşüyor: **gizleme kuralları "hiç gizleme"den ayırt edilemedi.**
3. Eşitlik bozucu (kalan tutma oranı): **`rate τ=0,55 N=20`** (0,583), ikinci `rate τ=0,55 N=10`
   (0,582).
4. **K42:** muafiyetsiz T2 görünür 0,422 < 1 → **muafiyet önerilir** (muaf: 1,000).
5. **Sıralama** (seçilen kural, muaf; T1 ve T2 ilk 5 ortalaması): `Grouped` 0,881, `Trust` 0,760,
   `LiveNew` 0,727; fark ≥ 0,02 → **`Grouped`**.

| sıralama | K42 muaf | satır/gün | T1 görünür | T1 ilk 5 | T2 görünür | T2 ilk 5 | ortalama |
|---|---|---:|---:|---:|---:|---:|---:|
| LiveNew | evet | 3,51 | 0,767 | 0,648 | 1,000 | 0,806 | 0,727 |
| Trust | evet | 3,51 | 0,767 | 0,699 | 1,000 | 0,821 | 0,760 |
| **Grouped** | evet | 3,04 | 0,767 | 0,769 | 1,000 | 0,994 | **0,881** |
| LiveNew | hayır | 3,18 | 0,605 | 0,508 | 0,422 | 0,310 | 0,409 |
| Grouped | hayır | 2,79 | 0,605 | 0,607 | 0,422 | 0,416 | 0,512 |
| (`none`) Grouped | — | 4,13 | 1,000 | 1,000 | 1,000 | 0,993 | 0,996 |

**İlk iki aday (ürüne giren):**

- Gizleme: `rate55` (= `rate τ=0,55 N=20`, seçilen) ve `none` (başvuru; ondan ayırt edilemedi).
  Düzeneğin ikincisi `rate τ=0,55 N=10` ürüne **alınmadı**: N=10, ürünün "az örnek" eşiğinin (20,
  `GLYPH_MIN_LEDGER_N`) altında; 10–19 örnekli kapsülün yüzü "az örnek" derken gizlenirdi (K53 kural
  3'e aykırı). Döküm üretimi sırasında görüldü (520 barlık kurguda gizlenen 2 satırın ikisi de
  `few_cases`).
- Sıralama: `grouped` (seçilen) ve `live_new` (kabul 7'nin "canlı ve yeni önce"si; sayıyla ikinci
  `Trust`'tır, 0,760 — ürüne alınmadı, rapora yazıldı).

## 5. Ürüne giriş

- **Kural:** `qp_capsule::feedrule` (`HideRule::Rate {min_n: 20, tau: 0.55}` = `rate55`, `Never` =
  `none`; `FeedRule.exempt_negative`, K42). Otorite: `SessionInit.feed_rule` ya da
  `Session::set_coin_flip_rule`; gösterim için okuyucu başına `set_principal_coin_flip_rule`
  (`ReplayHost` geri sarmada korur). Kural yokken hiçbir şey işaretlenmez: var olan 63 döküm ve
  altınlar değişmedi.
- **Az örnekli gizlenmez:** `n < 20` → `FewCases` (yüz "az örnek" der).
- **Gizlenen karnede:** kural yalnız akıştaki yeri belirler (test: karne cevabı kuralla ve kuralsız
  bayt eşit).
- **Görünürlük:** `important`/`balanced`'ta akış dışı (`hidden: coin_flip`, `coin_flip_hidden`
  sayısı); `all`'da akışta `coin_flip: true` ve çekirdeğin "karnesi yazı tura" işareti; hikâye
  listesinde (grafik derinliği) her seviyede işaretli.
- **Boş akış:** çekirdek yazar (`qp_grammar::feeds::empty_rows`): "Şu an anlatacak bir şey yok." +
  "Karnesi yazı turaya yakın N kapsül gizlendi; en açık hassasiyette (Hepsi) işaretli görünür." ya
  da "İzlediklerinde akışa girecek bir kapsül yok." (EN karşılıkları); kabuk başlık satırı olarak
  boyar.
- **Varsayılanlar:** protokol: kural yok, düzen `r3` (değişmedi). Gösterim (`qp demo`,
  `demo_session_init`): kural `rate55` (muaf); akış düzeni **`elikov` kaldı** — `grouped`
  varsayılan yapılsaydı var olan demo altınları (`feed-tr-dark-r4`, ilk üç yüz) ve `settings.mjs`
  varsayılan denetimi değişirdi; Grouped bir akış tasarım kararıdır (K46), Arkin'e.
- **Adres parametreleri:** `?feed_order=grouped|live_new|layout` (çekirdeğin gösterim ayarı;
  protokol `feed` adı değil — döküm 56 düzen adlarını listeler, değişmedi), `?feed_rule=rate55|none`
  (barındırıcıya bağlantıda gider; o okuyucu için kural). Ayarlar'da "Akış sırası (M19)" ve "Yazı
  tura eşiği (M19)" seçicileri (adlar çekirdeğin `choices()`'ından). `qp demo --feed-rule`.

## 6. Doğrulama

- `qp-session` `tests/feed_m19.rs`: zaman ileri alınınca geçmiş kararlar değişmiyor (aynı oturumda
  ve sonraki barları hiç görmemiş oturumla); gizlenenler karnede; az örnekli gizlenmiyor; akışın
  gizlenen sayısı listeninkine eşit; `all`'da işaretli; K42 muafiyeti parametre; bilinmeyen kural
  reddi; hiçbir kuralın N'si az örnek eşiğinin altında değil.
- `qp-wasm` `tests/feed_m19.rs`: gösterim barındırıcısının kuralı, seçenekler, `feed_order`
  gösterim ayarı ve belirleyici sıra, boş akış metni ve `all`'da işaret.
- `qp-grammar`: `live_new`/`grouped` belirleyici (girdi sırasından bağımsız), boş akış metinleri
  TR/EN.
- Döküm `fixtures/protocol/conformance/64-feed-rule.json` (kural açık: liste 4 satır, 3 yazı tura,
  1 az örnekli; akışta 27 gizli; `all`'da 2 işaretli öğe; pozisyonla 2 olumsuz aşama muaf; `story`
  olayı işaretli; karne) — `qp conform` süreç içi, `qp serve`, MCP: geçti. İki üretim aynı bayt.

## 7. Okuma ve sınırlar

- Gizleme güveni "yazı turayı göstermemek" yoluyla korur; karnenin öngörü gücünü artırmaz (beceri
  `none`'dan ayrılmıyor). RESEARCH F16'nın 0,70 hedefi bu dedektörlerle ulaşılmıyor.
- K42 muafiyeti olmadan kural, elde tutulan hissedeki olumsuz geçişlerin yarısından fazlasını
  saklıyor.
- Kitle sentetik ve küçük (11 enstrüman evren); gerçek veride lisanslı akış yok.
