# M18 `pavka` — hikâye omurgası

Yön ilkesi: alışılmışın dışı (K18). Grafik derinliğinde soru "bu hissede son üç ayda ne
oldu, hangileri tuttu, şu an canlı olan hangisi"dir; fiyat bu sorunun cevabı değil,
bağlamıdır. Bu yüzden `pavka` çerçeveyi **hikâyelere** verir, fiyatı üstte ince bir
**bağlam şeridine** indirir.

Seçim: `?story=pavka`, Ayarlar → Hikâye çizelgesi (`style` hareketi `story`,
`ViewState.story`). Varsayılan `r4` (hiçbir şey eklenmez; var olan altınlar ve dökümler
değişmez). Kapanmışları gizle: sahnedeki onay kutusu ya da Ayarlar → Kapanmış hikâyeleri
gizle (`style.hide_closed`, `ViewState.hide_closed`).

## Ekran (yukarıdan aşağı)

1. **Fiyat bağlam şeridi** (çerçeve yüksekliğinin %38'i, en az 150 px): açık (seçili)
   kapsülün grafiği olduğu gibi — mumlar, seçili hikâyenin kanıt geometrisi (seviye, bant,
   noktalar; K50) ve işareti. Zaman ekseni şeridin altında; şeritler aynı ekseni paylaşır.
2. **Özet satırı** (tek bakış): `205 hikâye ▶ 1 canlı ● 83 tuttu ✕ 121 tutmadı ‒ 3 yazı
   tura`. Her sayının önünde şeritte çizildiği biçim durur: özet aynı zamanda lejanttır.
   Sağ kenarda **kapanmış** onay kutusu (K54).
3. **Odak satırları**: seçili hikâye sözle — `aşağı kırılım · 8 Mart 2023'te` /
   `tutmadı · 41'de 24 tuttu` (+ `yazı tura`). Metinler satırın `face` yapı taşlarından.
4. **Şeritler**: görünen aralıktaki her hikâye, doğum barından kapanış barına (açıksa son
   bara) bir yatay şerit.
   - Önce **canlılar**, her biri kendi şeridinde, son bara uzanır ve ok başıyla biter;
     sağ kenarda `canlı`. Sonra ince bir çizgi; altında **kapanmışlar** soluk (`closed`
     rolü, ≥ 3:1), en son kapananlar üstte, birbirine değmeyenler aynı şeridi paylaşır.
   - Doğum: ▲ yukarı, ▼ aşağı, ◆ yönsüz. Sonuç (şeridin ucu): ● gerçekleşti, ✕ geçersiz,
     | süresi doldu, ○ geri çekildi. Yazı tura (K53, `hidden: coin_flip` olanlar dahil):
     kesik şerit. Artık geçerli değil (`no_longer`): ince şerit, içi boş ok.
   - **Seçili**: odak rengi, kalın şerit, renkli satır zemini ve doğum barından fiyat
     şeridinin tepesine çıkan ince bir kılavuz (hikâye ↔ fiyat bağı).
   - Renk tek başına anlam taşımaz: yön, sonuç, canlılık ve geçerlilik biçimdir.
5. **Kümeler** (gerektiğinde): şeritler yetmezse önce incelir (20 → 14 px; az şeritte
   28 px'e kadar büyür: daha büyük dokunma hedefi), yine yetmezse sığmayan kapanmışlar
   alttaki satırda zaman kovalarına toplanır (`+21` çipleri). Seçili her zaman, canlılar
   kapanmışlardan önce şerit alır.

## Etkileşim (çekirdekte)

- Başka bir hikâyenin şeridine dokunmak **odağı taşır**: o kapsül açık kapsül olur
  (`open` geçişi, grafik derinliğinde), görünen aralık korunur; liste yeniden istenmez
  (aynı enstrüman). Otoritenin `tap_at`'i (`qp_grammar::stories::pavka::tap`) karar verir.
- Seçili şeride dokunmak kanıtı açar (var olan açılma kuralı, `tap_opens`).
- Küme çipine dokunmak o kümenin barlarını gösterir (en az 20 bar).
- Onay kutusu kapanmışları gizler/gösterir; seçili hikâye gizlenmez.
- Sürükle/kıstır var olan kurallarla; uzaklaşınca şeritler incelir, sonra kümelenir.

## Mimari

- `crates/qp-grammar/src/stories/{mod.rs,pavka.rs}`: kayıt (`STORIES`, `names`,
  `by_name`) ve tasarım. Saf işlev: derlenmiş grafik sahnesi + satırlar + odak + görünüş →
  genişletilmiş sahne (`stories` paneli). `depth.rs`'te tek bağlantı: `story_design`.
- Her hikâye bir katman: `story:<kapsül>`, veri `Finding{finding}` (nokta sorgusu onu
  adlandırır), `label` olay ve geçerlilik. Küme `story-cluster:<i>`: sabitleri her
  hikâyenin ilk olgusuna köken verir (`/stories/<kapsül>`), değerleri `from_bar`, `to_bar`,
  `stories`. Onay kutusu `story-toggle`.
- Otorite (`qp-session/interact.rs`: `capsule_scene`, `tap_at`, `story_tap`) ve cihaz
  (`qp-session/viewer.rs::open_scene_with`, `qp-wasm/shell.rs::scene`) aynı sahneyi kurar:
  grafik `price_height(h)`'de derlenir, sonra `extend`. Otorite satırları kendi dünyasında
  `Session::story_rows` ile, cihaz §23'ün listesinden alır (aynı kural, aynı sıra).
- Kabuk: yalnız `?story=` okuma, Ayarlar'da seçim ve onay kutusu, ölçüm için
  `window.qp.stories` (mimari testi 0 ihlal).

## Ölçümler

Testler: `crates/qp-grammar/tests/story_pavka.rs` (5 / 50 / 200 hikâye; her görünen hikâye
bir şeritte ya da bir kümede, iki kez değil; canlı ve seçili şeritte; seçili en son çizilir;
her şeridin merkezinde `hit` o hikâyeyi adlandırır; kapanmışları gizle; odak/küme/onay
dokunuşu; sonuç biçimleri; kapanmış işaret karşıtlığı iki temada ≥ 3:1; çerçeve içinde;
belirlenimci) ve `crates/qp-wasm/tests/story_pavka.rs` (gösterim uçtan uca: cihazın
pikseline dokunuş otoritede tam o hikâyeye odak taşıyor, aralık korunuyor, liste yeniden
istenmiyor; onay kutusu; seçili şeritten kanıt; `r4` ile sahne değişmiyor; bilinmeyen ad
reddediliyor).

**K52** (`k52.mjs`, `PHONE` = WebKit 26.5 JIT kapalı DPR 3, `canvas` yolu; `qp demo
--history 1000 --days 1200`, adım 200, ilk kapsül `SYN26`, 10 sn oynatma + ±40 px sürükleme;
makine yükü ~13, başka ajanlar eşzamanlı çalışıyordu). p50 / p95 ms:

| görünen hikâye (aralık) | tasarım | sahne boyama | sahne kurma | tik |
|---|---|---|---|---|
| 12 (44 bar) | r4 | 2 / 3 | 1 / 2 | 0 / 1 |
| 12 | pavka | 6 / 6 | 3 / 4 | 0 / 1 |
| 52 (264 bar) | r4 | 7 / 8 | 3 / 3 | 0 / 1 |
| 52 | pavka | 12 / **13** | 6 / 6 | 0 / 1 |
| 202–207 (~1.010 bar) | r4 | 8 / 8 | 3 / 4 | 0 / 1 |
| 207 | pavka | 16 / **17** | 9 / 9 | 0 / 1 |

(`sahne` = sayfanın boyama aralığı; çekirdeğin değişen sahneyi kurması da onun içinde
olduğundan `sahne kurma` ayrıca verildi.) **Bütçe (sahne p95 ≤ 8 ms) 5 hikâyede tutuyor;
50 ve 200 hikâyede tutmuyor** (13 ve 17 ms). Başvuru `r4` aynı aralıkta 8 ms'de, yani bu
veri setinde 200 hikâye ~1.000 bar demek ve mumlar tek başına bütçenin sınırında. Fark
çoğunlukla çekirdekte (`extend` yerelde ~70 µs; JIT'siz WASM'da birkaç ms). Denenenler:
metin ya da kenar yumuşatmalı biçimleri kaldırmak boyamayı ölçülebilir değiştirmedi; tek
geçişli yerleşim, olgu dizininin bir kez kurulması, satırların kopyalanmaması, kesik çizgi
sınırı küçük kazanç verdi. Sonraki adım önerisi: aralık değişmedikçe şerit yerleşimini
önbelleğe almak (yalnız odak/gizle değişince yeniden kurmak) ve 200 hikâyede kümelemeyi
daha erken başlatmak.

Sonuçlar: `results/k52-pavka.json`, `results/k52-r4.json`.

## Kayda alınması gereken değerler (`look.rs`'e)

`pavka.rs` sabitleri, `r4`'ten türetilmiş: `PRICE_SHARE` 0,38, `PRICE_MIN_H` 150,
`STORY_MIN_H` 120, `LANE_H` 20, `LANE_MIN` 14, `LANE_MAX` 28, `CLUSTER_W` 36, `CLUSTER_H`
24, `HIT_MIN` 24, `FOCUS_ROW_ALPHA` 40, `GUIDE_ALPHA` 110, `BAR_FOCUS` 6, `BAR_LIVE` 4,
`BAR_CLOSED` 2, `MARK` 5, `DASH` (5, 3), `DASH_MAX` 12. Renkler var olan rollerden:
`focus`, `valid`, `invalid`, `closed`, `text`, `muted`, `line`, `bg`.

## Sınırlar

- Dokunma hedefi: şerit yüksekliği 14–28 px (44 pt değil); şeritler genişlikçe en az 24 px.
- Tam yansı kipinde (`?mode=replica`) cihaz listeyi kendi yansısından kurar; barındırıcının
  yazı tura kuralı yansıda yoksa yazı tura işaretleri orada çıkmaz.
- Liste gelmeden önceki kısa anda cihaz boş şerit alanı çizer (otorite dolu sahneyle
  dokunuşu çözer).
