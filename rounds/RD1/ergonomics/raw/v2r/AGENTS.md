# v2r — alt ajan kayıtları (yeniden koşu)

Beş taze alt ajan (sonnet, `general-purpose`); yönerge v1/v2 ile birebir aynı (klasör yolu
`ergo/v2r/`). `packet/` = v2 şartnamesi; `run.sh` `QP_ERGO_VARIANT=renamed` ile kimlikleri
yeniden adlandırılmış dokümanı (`harness/renamed/doc.json`) kullanır. İlk v2r koşusu
geçersiz sayıldı (`../v2r-void/VOID.md`).

| Ajan | Görevler | Toplam belirteç | Süre | Araç çağrısı |
|------|----------|----------------:|-----:|-------------:|
| A | G01 G06 G11 G16 | 73 725 | 34 s | 6 |
| B | G02 G07 G12 G17 | 77 818 | 43 s | 6 |
| C | G03 G08 G13 G18 | 77 352 | 52 s | 8 |
| D | G04 G09 G14 G19 | 75 943 | 43 s | 8 |
| E | G05 G10 G15 G20 | 76 127 | 45 s | 9 |
