# R5/S — altın görseller ve dökümler

Yazan: R5 `s-int` (bütünleştirme) alt ajanı, 2026-11-18.

## v1 (değişmedi)

`fixtures/golden/demo/*.png` (16 dosya) ve `fixtures/golden/` altındaki bütün sahne/görünüm
altınları, `fixtures/transcripts/` dökümleri ve hesap özeti **değişmedi** (dosyalara dokunulmadı;
`verify.sh` `golden`, `views`, `conform`, `replay`, `digest` ve `demo` adımları bayt eşit sınar).

Gösterimin varsayılanları R5'in seçimlerine geçtiği için (görünüş `oltan` + yüz `oltan`, dil
`marla`, hikâye çizelgesi `pavka`, açılmış kapsül `tesir`) `v1` demo çekimleri artık adreste
R5 öncesi tasarımları açıkça adlandırır (`shells/web/screens.mjs`, `V1`):

    ?look=r4&voice=r4&face=vadrel&story=r4&open=r4&motion=r4

(M21 birleştikten sonra `motion=r4` açılış geçişini R4'ün `zumlak`'ına döndürür.) (R3 çekimleri bunun üstüne `face=r3&feed=r3&transition=r3&outcome=r3`, `-fotolay` çekimi
`face=fotolay`.) Protokol varsayılanları
değişmediği için (Ö51) çıplak bağlantılar ve var olan dökümler bayt eşittir.

## v2 (yeni)

`fixtures/golden/demo/v2/` — gösterimin bugünkü varsayılanıyla (adreste tasarım parametresi
yok), `?paint=ref`, sentetik veri, adım 120 (`screens.mjs` `V2`; `verify.sh` `demo` adımı iki
koşuda bayt eşit ve altınlara karşı sınar):

| çekim | tuval | ne |
|-------|-------|----|
| `feed-{tr,en}-{dark,light}` | `face-0..2` | akışın ilk üç yüzü (`oltan` yüzü, `marla` dili) |
| `closed-{tr,en}-{dark,light}` | `closed-0..1` | kapanmış hikâyeli ilk iki yüz (sonuç işareti `tg5`, `oltan` tabanları) |
| `capsule-{tr,en}-{dark,light}` | `scene` | açılmış kapsül (`tesir` sütunu, tasarımın yüksekliği) |
| `chart-{tr,en}-{dark,light}` | `scene` | grafik derinliği + hikâye çizelgesi (`pavka`) |

Toplam 16 çekim, 28 sahne PNG'si (772 KB) (sayfa ekran görüntüleri `*.page.png` altın değildir; iki koşu
`pagecmp.mjs` toleransıyla karşılaştırılır).

Çekimler `?anim=0` ile alınır (= hareketi azalt, `?reduced_motion=1`'in eski adı): `v2`'de
varsayılan hareket `surek`, hareketi azalt'ta yüz değişimini kalıcı etiketle söyler; etiket
sahnenin kendi ilkelidir ve altın görüntüye girer (deterministik). Grafik derinliği: cihaz
(`viewer::open_view`) görünümü zaten bağlantının görünüşüyle kuruyordu
(`depth::chart_view_in(look_of(look), …)`); `r4` ile kurulan yalnız otoritenin `tap_at`/`hit`
görünümüydü (sunucunun `Presentation.view`'ı, `depth::chart_view`). R5/S son düzeltme:
otorite de `r4` dışındaki görünüşte belgede saklı olmayan sunucu görünümünü
`chart_view_in(look, …)` ile yeniden kurar (`interact.rs` `depth_view`; aynı kural, aynı kimlik);
`r4`'te ve saklı görünümde eskisi gibi. Bu yüzden `chart-*` v2 altınları yeniden üretildiğinde
bayt eşit çıktı (değişen cihazın çizdiği değil, otoritenin dokunuşu çözdüğü sahne); v1 16/16
bayt eşit. (Önceki raporumdaki "fiyat şeridi `r4` ile kuruluyor" ifadesi cihaz için yanlıştı.)
`pavka`'nın özet satırındaki kapanmışları gizle kutusunun yazısı ("kapanmış"/"closed")
`oltan` boylarında sağ kenarda kesiliyordu: `pavka` sözcükleri ortalama ilerlemeyle ölçüyordu.
Düzeltme (`stories/pavka.rs` `text_w`/`fit`): derleyicinin ölçüsü (`FontAsset::width`, orantılı
ailede ilerleme tablosu) ve kenar boşluğuna sığmayan sözcükte kutu sola kayar; `r4` bayt eşit
(`story_pavka_equal`, r4); dört `chart-*` v2 altını bu düzeltmeyle yeniden üretildi. 28 v2
çekiminin hepsine bakıldı: başka kesilen ya da taşan metin görülmedi.

Doğrulama: v1 16/16 altınla bayt eşit (`?look=r4&voice=r4&face=vadrel&story=r4&open=r4&motion=r4`),
v2 28/28 iki koşuda bayt eşit (bu ajanın koşusu, 2026-11-18).

## R5/fix47 — v1 adresine `feed_order=layout`, v2 yeniden üretildi

Gösterim artık akışını `live_new` ile sıralıyor (kabul 7; `feeds::PRESENTED_FEED_ORDER`), bu
yüzden `v1` çekimleri adreste bağlantının düzenini açıkça adlandırır (`screens.mjs` `V1`):

    ?look=r4&voice=r4&face=vadrel&story=r4&open=r4&motion=r4&feed_order=layout

v1 16/16 altınla **bayt eşit** (dosyalara dokunulmadı). v2'nin **28/28** sahnesi değişti; iki koşu
bayt eşit, yenileri `fixtures/golden/demo/v2/`'ye yazıldı (700 KB):

| çekim | değişiklik | gerekçe |
|---|---|---|
| `feed-*` `face-0..2` | akışın ilk üç yüzü başka kapsüller (canlı ve en yeni önce); gri işaretlerin plakası daha açık | kabul 7 (`live_new`); A46 `grey_plate_alpha_max` |
| `closed-*` `closed-0..1` | kapanmışların sırası `live_new`'e göre (en yeni önce); "tutmadı" izinin gri plakası 61 → 49 (koyu) / 29 (açık) | kabul 7; A46 (gri çizgiler plakada 2,93 / 2,70 → 3,09 / 3,08) |
| `capsule-*` `scene` | açılan kapsül akışın yeni ilk satırı (canlı SYN12; eskiden kapanmış SYN26); küçük grafiğin eksen çizgisi | kabul 7; A46 `axis` 2,95 / 2,88 → 3,23 / 3,19 |
| `chart-*` `scene` | aynı kapsülün grafiği ve hikâye çizelgesi; eksen; seçili satır bandı alfa 40 → 20 | kabul 7; A46 (bant üstündeki kapanmış şeritler 2,85 / 2,73 → 3,33 / 3,07) |

Açılmış kapsülün geçersiz başlığı (`invalid` → `muted`) v2 `capsule-*` çekiminde artık görünmüyor
(ilk satır canlı); kural `look::tests::oltan_meets_the_hard_thresholds` ve
`experiments/S/CONTRAST.md` ölçümüyle sınanıyor. Sayfa ekran görüntülerinde kabuk denetimlerinin
sınırı `--control` (altın değil). Gözle bakılanlar: `capsule-tr-dark` (sayfa), `closed-tr-light`
(sayfa), `chart-tr-light` (sahne); kesilen ya da taşan metin görülmedi.

## Değişen sınama kayıtları

- `crates/qp-wasm/tests/data/pavka_scene_digests.txt`: bkz. rapor (pavka performans eşitliği).
