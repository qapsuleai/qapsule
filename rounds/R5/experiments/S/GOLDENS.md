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
sahnenin kendi ilkelidir ve altın görüntüye girer (deterministik). Grafik derinliğinin fiyat
şeridi R4'ün grafik görünümüyle kurulur (`depth::chart_view`, LOOK_RECIPE "kayıt dışında
kalanlar"); `pavka`'nın başlık satırındaki "Kapanmışları gizle" yazısı `oltan` boylarında sağ
kenarda kesiliyor (bilinen, rapor "kalan sorunlar").

Doğrulama: v1 16/16 altınla bayt eşit (`?look=r4&voice=r4&face=vadrel&story=r4&open=r4&motion=r4`),
v2 28/28 iki koşuda bayt eşit (bu ajanın koşusu, 2026-11-18).

## Değişen sınama kayıtları

- `crates/qp-wasm/tests/data/pavka_scene_digests.txt`: bkz. rapor (pavka performans eşitliği).
