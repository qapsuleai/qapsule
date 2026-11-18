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

(R3 çekimleri bunun üstüne `face=r3&feed=r3&transition=r3&outcome=r3`, `-fotolay` çekimi
`face=fotolay`.) `motion` M21 birleşene kadar sayfada okunmaz; zararsız. Protokol varsayılanları
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

Toplam 16 çekim, 56 sahne PNG'si (sayfa ekran görüntüleri `*.page.png` altın değildir; iki koşu
`pagecmp.mjs` toleransıyla karşılaştırılır).

## Değişen sınama kayıtları

- `crates/qp-wasm/tests/data/pavka_scene_digests.txt`: bkz. rapor (pavka performans eşitliği).
