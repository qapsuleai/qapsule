# M20 `nobra` — soru kartları

Açılış: `?open=nobra` (Ayarlar → "Açılmış kapsül (M20)"). Örnek: `?open=nobra&voice=marla`,
`?open=nobra&voice=marla&look=ornek&theme=light`, yetkisiz okuyucu `?open=nobra&principal=visitor`.
Varsayılan değişmedi (`r4`: bugünkü kart, her dokunuş grafiğe iner).

## İlke

Açılmış kapsül, okurun ona sorabileceği sorular etrafında örgütlenir. Her soru bir kart:
soru satırı (dokunulabilir, `+`/`−`), **tek satırlık cevap** (dokunmadan görünür) ve dokununca
açılan **ayrıntı** (aynı anda bir tane). Başlık (sesin başlık cümlesi) kabuğun başlık satırında
kalır; sahnede tekrar etmez (K49).

| Sıra | Soru | Kısa cevap | Ayrıntı |
|---|---|---|---|
| 1 | Ne oldu? (işaret solda) | geçerlilik/kapanış: "Hâlâ geçerli; ikinci bir kanıt ailesi teyit etti", "Kapandı: beklenen oldu", "Artık geçerli değil…" | sesin ayrıntı cümlesi (A33) |
| 2 | Kim doğruladı? | "Sunucu doğruladı." / "…ve bu cihazda da sınandı." / "…bu cihazda sınanıyor." / "Kaynaklar farklı…" (`warn`) | lisanslı veri: cihaz yeniden hesaplayamaz; açık veri: cihaz aynı kuralı yalnız bu hissenin kendi verisinde sınar (K48) |
| 3 | Daha önce kaç kez? | kartın `ledger` alanı (sesin karne cümlesi; geçiş onu ortak öğe olarak taşır) | nasıl sayıldığı, n ve gerçekleşen, "az örnek" (n < 20), "karne geleceği söylemez", yetkiden bağımsız |
| 4 | Neden rakam göremiyorum? | yalnız kanıt değerleri gizli okuyucuda: "Kanıtın değerleri veri yetkisi gerektiriyor." | kanıt var, neye dayandığı, gereken yetki (`flow/…`), yorum ve karne herkese açık (K34, A32) |
| 5 | Neye dayanıyor? | kanıt aileleri ("Fiyat ve hacim verisine dayanıyor"; lisanslıysa "sunucuda hesaplandı") | işaretlerin anlamı (seviye/bant/dönüş noktaları) ve "grafiğe dokun" |
| — | (5'in altında) kanıt grafiği | mumlar + kanıt geometrisi (origin ile bağlı, K33) | dokunuş → grafik derinliği |

"Neye dayanıyor?" en altta: kanıt geometrisi bu cevabın içinde, grafiğe geçiş kapısı olarak
çerçevenin kalanını kaplar (ay etiketleri yalnız en alttaki panelin altına çizildiği için de).

## Yetkisiz okuyucu (A32)

- **Değerleri gizli ama barları açık** okuyucu (lisanslı sunucu olgusu): sahnede "Neden rakam
  göremiyorum?" kartı; geometri katmanı yok; soru kartlarının hiçbir metninde rakam yok (karne
  sayıları da yazılmaz; ses zaten rakamsız). Sınama: `qp-grammar/tests/capsule_depth.rs`
  `nobra_question_cards_over_entitled_and_withheld_evidence`.
- **Hiç verisi olmayan** okuyucu (gösterimdeki `visitor`): bugün olduğu gibi sahne yok. Kabuk,
  çekirdeğin `open().asks` satırlarını (`[{key,q,a,d}]`, `qp_grammar::open::text_rows`) aynı
  sorularla listeler; ayrıntı yerel `<details>` ile açılır. Metin besleme akış öğesidir (ses,
  geçerlilik, güven yolu, karne); aile adları öğede olmadığı için "tek bir / iki bağımsız kanıt
  ailesi" (yazıyla). Rakam: yalnız enstrüman etiketi. Sınama: `qp-session/tests/open_nobra.rs`.

## Mimari

- Kayıt `crates/qp-grammar/src/open.rs` (`OPENS`: `r4`, `nobra`; `view`, `tap`, `height`),
  tasarım `open/nobra.rs`. Görünüm otoritede (`interact::depth_view`, `tap_at`'ın sahnesi) ve
  cihazda (`viewer::open_scene_in`) aynı fonksiyonla kurulur; `ref`/`canvas` aynı sahneyi boyar.
- Metinler görünümde taşınmaz: metin alanı `ask/<dil>/<soru>/<parça>/<denetim>` derlemede
  sağlayıcıdan çözülür (`layer.rs`'te tek kanca, `open::field_text`).
- Seçim: `style {open}` → `ViewState.open`; açık soru `ViewState.ask` (dokunuşla, otoritede).
  Kapsül kapanınca/başkası açılınca silinir. Protokol notu `docs/PROTOCOL.md` §4.6.2.
- Cihazın denetimi (`Viewer::trust`) yalnız kapsül derinliğinde ve tasarım seçiliyken sorulur
  (`r4`'te denetim sayısı değişmez). "Kim doğruladı?" kartının yüksekliği dört denetim durumunun
  en uzununa göre ayrılır: otoritenin (denetimi bilmez) ve cihazın kart kutuları aynıdır (sınandı).
- Yerleşim çerçeveden piksel olarak (panel ağırlığı = piksel): kart başına bir panel (taşan metin
  panelin sonunda "…" ile kesilir), kalan yükseklik kanıt grafiğine (en az ¼).
- Tasarım kendi çerçeve yüksekliğini bildirir (`open::frame_height`): `r4`'te 560 px, başka
  görünüşte `types.body.px` oranıyla (ör. `ornek` 700). Otorite `capsule` çerçevesinde, kabuk
  çekirdeği `open_capsule`/`open_to` isteğinde onu kullanır.
- Kabuk: Ayarlar seçimi, `?open=`, `open.hint` (varsa çekirdeğin ipucu), sahnesiz okuyucu için
  `#asks` listesi. `arch-check` 0 ihlal.

## Görünüşten okunan değerler

Yazı rolleri `label` (soru, ayrıntı), `body` (cevap); `glyph.card_px`, `space.glyph_gap`,
`space.scale[0]` (iç aralık), `space.scale[1]` (kart altı), `space.panel_gap`, `space.plot`;
renk rolleri `accent` (soru), `text`, `warn` (kaynaklar farklı), `look.evidence.*` (geometri).

**Kayda alınmalı** (bugün `r4`'ten türeyen sabit): kapsül derinliği çerçeve yüksekliği 560 px
(`nobra::height`); kanıt grafiğinin en küçük payı (kalan yüksekliğin ¼'ü, en az 48 px); cevabın
en çok iki satırı; açma/kapama işaretleri `+`/`−` (yazı tipi kapsamı sınandı: Plex Mono'da
U+2212 var, ▸/▾ yok).

## Bilinen sınırlar

- "Kim doğruladı?" cevabı iki satırlık yer ayırır (en uzun denetim durumu); kısa durumda bir
  satır boşluk kalır (kutu kararlılığı için bilinçli).
- Sahneli okuyucuda "Neden rakam göremiyorum?" yalnız lisanslı (sunucu) olgu varken çıkar;
  sentetik gösterim dünyasında böyle olgu yok, bu durum birim sınamasıyla gösterildi.
- `qp drive` (kabuksuz sürücü) Türkçe ve denetimsiz kurar; `open` seçiliyken sürücü/kabuk
  özetleri karşılaştırılmadı (varsayılan `r4`'te değişiklik yok).

## K52 (`k52/`, `harness/glance/k52.mjs` + `open=` parametresi, `voice=marla`, 3 tekrar)

Kalibrasyon senaryosu kapsülü açıp sahnenin (195, 150) noktasına dokunur: `r4`'te bu grafiği
açar, `nobra`'da bir soru kartının ayrıntısını açar. Yani `nobra` satırları **kapsül derinliğini
tik akarken** ölçer (390×560, DPR 3 → 1170×1680 px), `r4` satırları grafiği (1170×900 px).

| Yapılandırma | Tasarım | tik p95 | yüz p95 | sahne p95 | ilk kapsül | kaydırma kare p99 | Sonuç |
|---|---|---|---|---|---|---|---|
| phone-jit-est (tahmin) | nobra | 1 ms | 1 ms | 1 ms | 197 ms | 18 ms | geçti |
| phone-jit-est (tahmin) | r4 | 1 ms | 1 ms | 1 ms | 181 ms | 19 ms | geçti |
| phone (kalibre, JIT yok) | nobra | 1,6 ms | 1 ms | 18,5 ms | 214 ms | 19 ms | geçti (kullanılabilir uzun kare) |
| phone (kalibre, JIT yok) | r4 | 1,6 ms | 2 ms | 5,55 ms | 213 ms | 20 ms | geçti |

JIT'siz sahne p95'i `nobra`'da daha yüksek (daha büyük tuval, her tikte kapsül derinliği yeniden
boyanıyor); K52 JIT'siz durumda yalnız "kullanılabilir" ister ve bu geçti. Grafik derinliği
`r4` ile aynı görünümdür.
