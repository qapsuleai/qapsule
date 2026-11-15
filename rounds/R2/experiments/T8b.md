# R2/T8b — Anlamla başvuru: metin adları

Dal: `r2/t8b-names` (`r2/int` üstünde). Şartname: `docs/PROTOCOL.md` §17 (seçilmiş gibi
yazıldı). Kod: `crates/qp-session/src/names.rs` (dil, eşleşme, sıralama, kanonik adlar,
alan gezgini); bağlantılar `session.rs` (`handle_value`), `expand.rs` (`expand_ops`: işlem
işlem çözüm), `summary.rs` (`refs`), `query.rs` (`screen` paneli `ref`). Testler:
`crates/qp-session/tests/names.rs` (8 test), döküm
`crates/qp-session/tests/transcripts/t-names.json` (üç taşımada geçiyor).

## 1. Fikir

RD1 v2r: kimlikler kopyalanamayınca ilk deneme 14/20 → 6/20; ajan ilk denemesini kimlik
keşfine harcadı. Bu tasarımda ajan kimlik yerine **dokümanın içeriğinden türeyen kısa bir
ad** yazar. Ad, ajanın zaten kullandığı kelimelerle kurulur (hesap adı ve parametresi,
"fiyat", "kapanış", panel, işaret, "en son kırılım"):

```
"@rsi(14)"  "@price"  "@close"  "@volume"  "@bollinger(20,2).upper"  "@sma50"
"@view"  "@panel:price"  "@panel:rsi(14)"  "@panel:macd"
"@line:rsi(14)"  "@candles"  "@rule:70"  "@bar:macd.histogram"
"@panel:price/line:bollinger.upper"
"@finding:latest(breakout)"  "@fact:oldest(breakout)"
```

Otorite her öğenin **kanonik adını** `welcome.summary.refs`'te verir; ajan ilk mesajında ya
listeden kopyalar ya da dilbilgisinden kendisi kurar (listeyi görmeden de: `@rsi(14)`,
`@panel:macd`, `@finding:latest(breakout)` belge örneklerinden değil dokümanın
içeriğinden türer, yeniden adlandırılmış dokümanda da aynıdır).

## 2. Dilbilgisi (özet; tamamı §17.2)

```
ad      = "@" [ "panel:" panelsel "/" ] öğe
öğe     = "view" | "panel:" panelsel | ("layer"|işaret) ":" veri | işaret
        | ("fact"|"finding") [":" ("latest"|"oldest")] ["(" tür ")"]
        | seri
seri    = "price" ["." alan] | open|high|low|close|volume
        | hesap ["(" sayı {"," sayı} ")"] ["." çıktı] | hesap rakamlar ["." çıktı]
hesap   = sma|ema|rsi|macd|bollinger|bb
veri    = seri | olgu/bulgu seçimi | sayı | eşik-adı   (sayı/eşik yalnız rule:)
panelsel= panel adı (sözcük) | veri
```

Büyük/küçük harf ve boşluk önemsiz. Parametreler konumsal ve önek: `@macd(12)` yalnız
`fast`'ı sabitler. Sözdizimi hatası `schema` (+ `expected` biçim listesi).

**Ad ile kimlik ayrımı:** `@` ile başlayan dizge addır (kimlik `@` içeremez). `@`'siz bir
dizge yalnız kimlik biçiminde **değilse** ve ad olarak ayrışıyorsa ad sayılır (`rsi(14)`).
Kimlik biçimindeki her dizge kimliktir — belgeye bağlı bir öncelik kuralı yoktur.

## 3. Türetme ve eşleşme

- **Seri:** hesap adı + verilen parametreler (konumsal önek, `f64` eşitlik); `price` = hesap
  ve nokta olmayan seri.
- **Katman:** işaret süzgeci + katmanın çizdiği alanlar (`y`, `y2` alanları ve kendi serisine
  başvurular; `y` yoksa mum `o h l c`, çok çıktılı çizgi çizim çıktıları, diğerleri ana alan).
  `@price` hacim çubuklarına uymaz, `@volume` uyar. `rule:` sabit ya da eşik kökeniyle.
- **Panel:** önce panel adı (tam eşitlik); hiçbir panelin adı uymuyorsa içerik (veriye uyan
  bir katmanı olan paneller). Kapsam mesajın `view`'u (o da ad olabilir); görünümsüz
  hedefte bütün görünümler.
- **Olgu/bulgu:** tür (`kind`) eşitliği; bulgu için olgularından biri; `latest`/`oldest` bara
  göre süzer (bulgu barı = olgularının en büyüğü).
- **Kanonik ad:** sabit bir merdivenin dokümanda tam o öğeye çözülen ilk basamağı (§17.8).
  Merdiven `resolve`'un kendisiyle sınanır, bu yüzden listedeki her ad geri çözülür (test:
  `welcome_lists_a_resolvable_name_for_every_item`).

## 4. Sıralama kuralı (`ambiguous`)

| Tür | Anahtar (küçük önce; eşitlikte kimlik) |
|-----|----------------------------------------|
| seri | çiziliyor önce → varsayılan parametreler önce → doküman sırası |
| görünüm | doküman sırası |
| panel | görünüm sırası → yığın sırası |
| katman | görünüm sırası → çizim sırası |
| olgu, bulgu | en son bar önce → doküman sırası |

Neden bu: belirsiz bir adda ajanın büyük olasılıkla kastettiği ekranda görünen, varsayılan
ayarlı, en yeni öğedir; ama kural yalnızca **sıralar**, seçmez. `about` adayın kanonik adıyla
başlar: düzeltme bir kopyalamadır.

## 5. Protokol ve şema değişiklikleri (hepsi ek)

| Değişiklik | Neden |
|------------|-------|
| `PanelRef` (şema tipi: tamsayı **ya da** dizge); panel alanlarına `#[schemars(with)]`: `Show.panel`, `Target.panel`, `move_layer.panel`, `insert_panel/move_panel.after/before`, `delete_panel/move_panel/set_panel.panel`, `tap.panel`, `Layer.panel` | Panel kimliği sayı; ad dizge. Rust tipi `u32` kalır: otorite adları çözümden **önce** JSON üstünde değiştirir. Doküman, günlük, öneri yalnız sayı görür; diğer crate'ler etkilenmez. |
| `ServerMsg::Result.resolved` (isteğe bağlı) | Sorgu/girdi cevabında da `{gösterici → kimlik}`. |
| `DocSummary.refs: Vec<RefSummary {ref, kind, id, view?}>` | İlk mesajda neye başvurulabileceği. Var olan özet nesnelerine alan eklenmedi (T5'in tam eşitlik testleri değişmeden geçsin). |
| `PanelInfo.ref` (`screen`) | Ekrandaki panelin kanonik adı. |
| `Op`/`Query` belgelerine bir cümle | MCP araç açıklamaları şemadan üretildiği için ajan adları araç tanımında görür. |

`ack.resolved`, `ErrorCode::Ambiguous`, `Candidate` T5'te zaten vardı; artık dolduruluyor.

## 6. Çözüm akışı

1. `handle_raw` → JSON → `Walk(Detect)`: mesajda ad var mı (yalnız kimlik alanlarına bakar).
   Yoksa eski yol, değişmeden (RD1'in 17 dökümü bu yüzden aynen geçiyor).
2. Komut: panel alanlarındaki adlar `0` ile maskelenmiş bir kopya şemaya göre çözülür
   (şema hataları her zamanki gibi); ham işlemler `expand_ops`'a gider. Her işlem, önceki
   işlemlerin etkisini taşıyan geçici dokümana karşı `Walk(Resolve)` ile çözülür, sonra
   tipli `Op`'a çevrilir. Günlük `request` ve `Proposal.ops` çözülmüş işlemleri saklar.
3. Diğer mesajlar (sorgu, abonelik, girdi, dikkat): o anki dokümana karşı çözülür;
   `resolved` cevaba eklenir.
4. Tipli `Session::handle` da aynı yoldan geçer (mesaj JSON'a çevrilip ad aranır).
5. Okuma yetkisi olmayan bağlantıda ad çözülmez.

## 7. Elenen seçenekler

| Seçenek | Neden elendi |
|---------|--------------|
| `@`'siz çıplak sözcükler (`rsi`, `price`), "tam kimlik kazanır" önceliğiyle | RD1 dökümü 03 `"with":"rsi"` için `not_found` bekliyor; çıplak ad onu `rsi14`'e çözerdi. Ayrıca ad/kimlik sınırı dokümana bağlı olurdu: sonradan `price` kimlikli bir öğe eklenince aynı mesajın anlamı sessizce değişirdi. |
| Panel alanlarını Rust'ta `PanelRef`/dizge yapmak | `Layer.panel` dokümanda saklanır; tip değişimi dokümanı, çiziciyi ve bütün `l.panel == id` kodunu etkilerdi; günlükte ad kalma riski. Şema-yalnız tip + JSON üstünde değiştirme seçildi. |
| Bütün adları komutun başında, ilk dokümana karşı çözmek | R01'deki doğal yazımı ("RSI(7) ekle, onu yeni panele koy": `"series":"@rsi(7)"`) reddederdi. İşlem işlem çözüm seçildi. |
| Bulanık eşleşme (en yakın adı seçmek) | Belirlenimci ama tahmin; yanlış öğeye yazmak `not_found`'dan pahalı. Yakın adlar yalnız `did_you_mean`'de. |
| Kullanıcının verdiği kalıcı takma adlar (`alias` alanı) | Yeni doküman alanı ve işlem ister; içerikten türeyen ad hiçbir şey saklamaz. Kalıcılık gerekiyorsa kimlik zaten var. |
| Her özet nesnesine `ref` alanı eklemek | T5 testleri özet nesnelerini tam eşitlikle sınıyor; ayrı `refs` listesi hem ek hem tek bakışta "ne yazabilirim" tablosu. |
| Sembol niteleyicisi (`@price(thyao)`), bar/zaman seçicileri (`@fact:bar(185)`) | Dil küçük kalsın; R2 görevleri gerektirmiyor. Öneri olarak bırakıldı. |
| Nesne biçimli seçiciler (`{"ref":{"study":"rsi","params":{…}}}`, RD1 v3 önerisi) | Bu tasarım ailesinin dışında (metin adları); alanların tipini dizgeden nesneye çevirmek her kimlik alanında şemayı genişletir. |

## 8. R2 görevleri bu tasarımla (TASKS.md R01–R20)

Gizli kurulum: fiyat, RSI, MACD, Bollinger, SMA(50) (çizilmemiş), iki `breakout` olgusu ve
bulgusu. Kimlikler bilinmiyor; mesajlar yalnız ad kullanıyor. ✓ = `tests/names.rs`'te aynı
biçimiyle sınandı.

| # | Mesaj(lar) |
|---|-----------|
| R01 ✓ | `{"op":"add_study","study":"rsi","inputs":["@price"],"params":{"period":7}}`, `{"op":"insert_panel","view":"@view","panel":{"y":{"kind":"linear"}},"after":"@panel:price","layers":[{"mark":"line","series":"@rsi(7)"}]}` (tek komut) |
| R02 ✓ | `set_encoding {"view":"@view","layer":"@line:bollinger(20,2)","encoding":{"y":"middle"}}`, `insert_layer {"view":"@view","after":"@line:bollinger(20,2).middle","layer":{"mark":"line","series":"@bollinger(20,2).lower","panel":"@panel:price"}}`, `insert_layer {…"series":"@bollinger(20,2).upper",…,"encoding":{"color":"down"}}` |
| R03 ✓ | `{"op":"set_emphasis","view":"@view","emphasis":[{"target":{"item":"@finding:latest(breakout)"},"kind":"highlight"}]}` |
| R04 ✓ | `{"kind":"present","finding":"@finding:latest(breakout)","surface":"watch"}` |
| R05 ✓ | `{"op":"set_scale","view":"@view","scale":"@panel:price","kind":"log"}` |
| R06 ✓ | `{"op":"insert_layer","view":"@view","layer":{"mark":"rule","panel":"@panel:rsi(14)","encoding":{"y":30}}}` (köken `rsi.oversold` otomatik) |
| R07 ✓ | `{"op":"delete_panel","view":"@view","panel":"@panel:macd"}` |
| R08 ✓ | `{"op":"insert_panel","view":"@view","panel":{"y":{"kind":"linear"}},"after":"@panel:price","layers":[{"mark":"bar","series":"@volume"}]}` (→ `encoding.y:"v"`) |
| R09 ✓ | `{"op":"insert_layer","view":"@view","before":"@candles","layer":{"mark":"band","series":"@bb","panel":"@panel:price","encoding":{"y":"upper","y2":"lower","opacity":0.2}}}` |
| R10 ✓ | `{"op":"insert_layer","view":"@view","layer":{"mark":"bar","series":"@macd.histogram","panel":"@panel:macd","encoding":{"color":{"when":{"field":"histogram","op":"ge","than":0},"then":"up","else":"down"}}}}` |
| R11 ✓ | `{"op":"insert_layer","view":"@view","layer":{"mark":"line","series":"@close","panel":"@panel:price","encoding":{"color":{"when":{"field":"c","op":"ge","than":{"series":"@sma(50)","field":"sma"}},"then":"up","else":"down"}}}}` |
| R12 ✓ | `{"op":"set_view_range","view":"@view","from_bar":140,"to_bar":199}` (bar sayısı `summary.series[].len`'den) |
| R13 | `{"kind":"screen"}` → panellerin `ref`'i (`@panel:rsi(14)`) ve `series`/`marks` |
| R14 | `{"kind":"hit","view":"@view","x":900,"y":200,"width":1200,"height":600}` |
| R15 ✓ | `{"op":"set_encoding","view":"@view","layer":"@line:rsi(14)","encoding":{"width":2}}` |
| R16 ✓ | `{"op":"insert_layer","view":"@view","layer":{"mark":"point","fact":"@fact:oldest(breakout)","panel":"@panel:price"}}` |
| R17 ✓ | `{"op":"add_fact","kind":"level","series":"@price","bar":199,"text":"…","evidence":{"level":123.45}}`, `{"op":"insert_layer","view":"@view","layer":{"mark":"rule","panel":"@panel:price","encoding":{"y":123.45},"origin":{"/encoding/y":{"kind":"fact","fact":"@fact(level)","field":"level"}}}}` (olgu aynı komutta adla) |
| R18 ✓ | `{"op":"move_panel","view":"@view","panel":"@panel:macd","before":"@panel:rsi(14)"}` |
| R19 | `{"kind":"present","finding":"@finding:latest(breakout)","surface":"phone"}` → `depth: capsule`, `spark` 60 bar |
| R20 ✓ | `{"op":"insert_panel","view":"@view","panel":{"y":{"kind":"linear"}},"layers":[{"mark":"line","series":"@bollinger.bandwidth"}]}` |

R06, R15, R18 gizli dokümanda RSI(14) varsayar; parametresiz `@rsi`/`@panel:rsi` gizli dokümanda tek RSI varsa çözülür; R01'den
sonra ikinci RSI eklenmişse `ambiguous` olur ve adaylar `@rsi(14)` / `@rsi(7)`'yi önerir.

## 9. Bilinen zayıflıklar

1. **Adlar kalıcı değil:** içerikten türer; yeni bir RSI `@rsi`'yi, yeni bir kırılım
   `@finding:latest(breakout)`'u değiştirir. Komut içinde tutarlı, günlük kimlik saklar;
   ama bir ajanın iki mesaj arasında "aynı ad = aynı öğe" varsayması yanlış olabilir.
2. **Alan gezgini elle bakılan bir aynadır:** hangi alanın hangi tür kimlik aldığı
   `Walk::op` içinde yazılı. Şemaya yeni bir kimlik alanı eklenip gezgine eklenmezse o alanda
   ad sessizce kimlik gibi denetlenir (`not_found`). Bir test ya da şemada bir işaret
   (`x-qp-ref`) ile otomatikleştirilebilir (öneri).
3. **Panel alanlarında tip ikiliği:** şema `PanelRef` diyor, Rust `u32`. Mesajı tipli çözen
   istemciler (ör. `Transcript.send`) panel adı taşıyamaz; dökümde bu adımlar `send_raw`.
4. Ajan dilbilgisini öğrenmeli; yanlış yazım `schema` hatası (biçim listesiyle) — bir tur.
5. Tek enstrüman varsayımı (`@price`); nokta serileri, niyetler, öneriler, kapsüller,
   anlatımlar adsız; olgu türü serbest metin (yalnız sözcük biçimli türlerin kanonik adı var).
6. Panel adının içerikten önce gelmesi şaşırtabilir (adı `rsi` olan bir MACD paneli
   `@panel:rsi`'yi alır).
7. Çıktı seçen ad isteği yalnız kimlik değiştirmenin ötesinde değiştirir (kardeş alana
   çıktıyı yazar); günlükte görünür ama "yalnız kimlik değişir" kuralına bir istisnadır.
8. `put_view` içinde panel adı çözülmez (paneller aynı mesajda tanımlanıyor).
9. Belirsiz katman adlarında sıralama çizim sırasıdır, anlamsal değildir.
10. Ölçülmedi: ilk deneme başarısına etkisi T9'un işi (iki tasarım aynı düzenekle).

## 10. Doğrulama (bu dalda)

- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`: yeşil.
- `qp conform fixtures/protocol/conformance crates/qp-session/tests/transcripts --transport
  inproc|serve|mcp`: 21/21 (RD1'in 17'si + 3 eski + `t-names`).
- `scripts/verify.sh` (`QP_VERIFY_BROWSER=0`): OK (şema 23 dosya, belirlenimci; protokol
  örnekleri ve dökümler şemaya uygun; MCP araçları üretiliyor).
