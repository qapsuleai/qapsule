# v1 — alt ajan kayıtları

Beş taze alt ajan (model: sonnet, `general-purpose`), her biri dört görev. Hepsine aynı
yönerge verildi (aşağıda); yalnızca `packet/` (o andaki `docs/PROTOCOL.md` kopyası ve
üretilen şemalar) ve `packet/run.sh` kullanmalarına izin verildi. Görev listesi her ajanın
`TASKS.txt` dosyasında (yalnızca kim/yüzey/doğal dil isteği; başarı ölçütü verilmedi).

| Ajan | Görevler | Toplam belirteç (alt ajan) | Süre | Araç çağrısı |
|------|----------|---------------------------:|-----:|-------------:|
| A | G01 G06 G11 G16 | 74 279 | 34 s | 6 |
| B | G02 G07 G12 G17 | 78 862 | 54 s | 8 |
| C | G03 G08 G13 G18 | 71 747 | 30 s | 5 |
| D | G04 G09 G14 G19 | 74 152 | 42 s | 7 |
| E | G05 G10 G15 G20 | 73 953 | 38 s | 8 |

Görev metinlerinde G11, G12, G17'de "kullanıcı"nın kim olduğu ("(arkin)") eklendi; diğer
metinler `tasks/TASKS.md` ile aynı.
