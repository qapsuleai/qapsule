# RD1/T7 — Ajan yazabilirlik ölçümü

Yapay zekâ için tasarlanan protokolü yapay zekâ sınadı: uygulamayı, testleri ve bu klasördeki
notları görmemiş **taze alt ajanlar** yalnızca `docs/PROTOCOL.md` ve üretilen şemayla, yirmi
doğal dil görevi için mesaj dizileri yazdı; diziler referans oturumda (`qp serve`) çalıştı;
sonuç, tasarımdan **önce** yazılmış görev ölçütlerine (`tasks/TASKS.md`) göre bir denetleyiciyle
(`harness/ergo.py check`) puanlandı. Ajanların kendi "başardım" beyanı puana girmedi.

## Yöntem

- **Görevler:** `tasks/TASKS.md` G01–G20 (tasarımdan önce, `492f79a`). Ajanlara yalnızca
  "kim, hangi yüzey, doğal dil isteği" verildi; başarı ölçütü verilmedi. G11, G12, G17'de
  "kullanıcı"nın kim olduğu parantezle eklendi ("(arkin)").
- **Ajanlar:** her turda 5 taze alt ajan (`general-purpose`, model `sonnet`), her biri 4
  görev; görev dağılımı her turda aynı. Yönerge her turda birebir aynı (yalnızca klasör yolu
  farklı); kayıtlarda `raw/<tur>/AGENTS.md` ve `packet/`.
- **Görülen:** `packet/PROTOCOL.md` (o anki şartname kopyası), `packet/schema/*.schema.json`
  (9 protokol şeması), `packet/run.sh`. Başka dosya okumaları yasaklandı.
- **Çalıştırma:** her deneme tam bir mesaj dosyası; her çalıştırma aynı taze durumdan
  başlar (gizli kurulum: 4 eski günlük girdisi farklı zamanlarda, bekleyen bir öneri
  `prop-3`, kullanıcının masaüstü bağlantısı; "şimdi" = son işlem + 10 dk). Ajan yalnızca
  kendi bağlantılarına giden mesajları görür. En çok 3 deneme.
- **Ölçüler:** ilk denemede başarı; sonunda (≤ 3 deneme) başarı; hata/sonuç gördükten sonra
  düzeltme oranı; deneme, mesaj ve belirteç sayısı (deneme dosyasının baytı / 4; ayrıca alt
  ajan başına toplam belirteç `raw/*/AGENTS.md`).
- **Bir tek-atış sınırı:** ajan bir çalıştırma içinde sorgu cevabını görüp ona göre yazamaz
  (dosya tek seferde gönderilir). Gerçek etkileşimli bir ajan bunu bir tur içinde yapardı.
  Bu yüzden "ilk deneme", "önce keşif, sonra yazma" stratejisini cezalandırır; ikisi ayrı
  okunmalıdır.

## Turlar

| Tur | Şartname | Doküman | Ne ölçer |
|-----|----------|---------|----------|
| **v1** | `qp/0` ilk hali (`raw/v1/packet/PROTOCOL.md`) | temel (`px`, `rsi14`, `fnd-overbought`…) | tasarımın ilk hali |
| **v2** | tasarım değişikliği sonrası (`raw/v2/packet/PROTOCOL.md`) | temel | önce/sonra (v1 ile aynı koşul) |
| **v2r** | v2 | **kimlikleri yeniden adlandırılmış** aynı doküman (`thy-d`, `osc-a`, `fnd-9e1`…) | şartnamedeki örnek kimliklerinin sızıntısı |

v2r'nin nedeni: v1 ajan notları, ajanların kimlikleri (`rsi14`, `px`, `fnd-overbought`)
**şartnamenin örneklerinden** tahmin ettiğini gösterdi; örnekler test dokümanının
kimliklerini kullanıyor. Bu, v1/v2 ilk deneme başarısını yapay olarak yükseltir. v2r aynı
dokümanı farklı kimliklerle verir; ajan kimlikleri protokolle keşfetmek zorundadır.

Bir v2r çalıştırması **geçersiz** sayıldı (`raw/v2r-void/VOID.md`): yeniden adlandırma
hesap adını da (`macd`) değiştirmişti. Düzeltilip taze ajanlarla yeniden koşuldu.

## Sonuçlar

| Ölçü | v1 | v2 | v2r |
|------|---:|---:|----:|
| İlk denemede başarı | **15/20** | **14/20** | **6/20** |
| Sonunda başarı (≤ 3 deneme) | 17/20 | 18/20 | **19/20** |
| İlk denemesi başarısız olup düzelen | 2/5 | 4/6 | **13/14** |
| İlk denemede hata cevabı alan görev | 0 | 0 | 12 |
| Toplam deneme | 23 | 26 | 35 |
| İlk deneme ortalama mesaj | 3,2 | 2,8 | 3,0 |
| İlk deneme ortalama belirteç (~bayt/4) | 91 | 77 | 82 |
| Tüm denemeler: mesaj / belirteç | 75 / 2 155 | 72 / 2 118 | 106 / 3 216 |
| Alt ajan toplam belirteç (5 ajan) | 373 k | 371 k | 380 k |

### Görev görev

| Görev | v1 ilk | v1 son | v1 den. | v2 ilk | v2 son | v2 den. | v2r ilk | v2r son | v2r den. |
|---|---|---|---|---|---|---|---|---|---|
| G01 | ✓ | ✓ | 1 | ✓ | ✓ | 1 | ✗ | ✓ | 2 |
| G02 | ✗ | ✗ | 1 | ✓ | ✓ | 1 | ✓ | ✓ | 1 |
| G03 | ✓ | ✓ | 1 | ✓ | ✓ | 1 | ✗ | ✓ | 2 |
| G04 | ✓ | ✓ | 1 | ✓ | ✓ | 1 | ✗ | ✓ | 2 |
| G05 | ✓ | ✓ | 1 | ✓ | ✓ | 1 | ✗ | ✓ | 2 |
| G06 | ✗ | ✗ | 1 | ✗ | ✗ | 1 | ✗ | ✗ | 2 |
| G07 | ✓ | ✓ | 1 | ✗ | ✓ | 2 | ✗ | ✓ | 2 |
| G08 | ✓ | ✓ | 1 | ✓ | ✓ | 1 | ✗ | ✓ | 2 |
| G09 | ✗ | ✓ | 2 | ✗ | ✗ | 2 | ✗ | ✓ | 2 |
| G10 | ✓ | ✓ | 1 | ✓ | ✓ | 1 | ✓ | ✓ | 1 |
| G11 | ✓ | ✓ | 1 | ✓ | ✓ | 1 | ✓ | ✓ | 1 |
| G12 | ✗ | ✓ | 3 | ✗ | ✓ | 3 | ✗ | ✓ | 2 |
| G13 | ✓ | ✓ | 1 | ✗ | ✓ | 2 | ✗ | ✓ | 3 |
| G14 | ✓ | ✓ | 1 | ✓ | ✓ | 1 | ✗ | ✓ | 2 |
| G15 | ✓ | ✓ | 1 | ✓ | ✓ | 1 | ✓ | ✓ | 1 |
| G16 | ✓ | ✓ | 1 | ✓ | ✓ | 1 | ✗ | ✓ | 2 |
| G17 | ✓ | ✓ | 1 | ✓ | ✓ | 1 | ✓ | ✓ | 1 |
| G18 | ✓ | ✓ | 1 | ✓ | ✓ | 1 | ✗ | ✓ | 2 |
| G19 | ✗ | ✗ | 1 | ✗ | ✓ | 2 | ✗ | ✓ | 2 |
| G20 | ✓ | ✓ | 1 | ✓ | ✓ | 1 | ✓ | ✓ | 1 |

Puanlama tabloları ve başarısızlık nedenleri: `raw/<tur>/SCORE.md`.

## v1 başarısızlıkları ve tasarım değişikliği (v2)

| v1 bulgusu | Neden | v2 değişikliği | v2'de |
|------------|-------|----------------|-------|
| G02 "son yarım saat": ajan "şimdi"yi bilemedi; son girdiyi şimdi saydı | Protokolde otoritenin saatini öğrenmenin yolu yoktu | `welcome.now`, `result.now`; `log` sorgusuna `within_ms` | ✓ (v2 ve v2r ilk denemede) |
| G12: seri listesi bütün barları döktü (ajan notu: "çok uzun") | `list` serileri veriyle döndürüyordu | `list` serileri varsayılan **verisiz**; `full: true` ile veri | belirteç düştü; G12 yine keşif turu harcadı |
| G19: "100'ün altına inerse" → ajan `cross_below` seçti | Şartname durum ile kesişim ayrımını anlatmıyordu | §5.1'e "durum mu kesişim mi" tablosu (doğal dil → koşul) | v2'de 2. denemede, v2r'de 2. denemede ✓ (ajan tabloyu andı) |
| G12: "hacim"i hedefleyemedi | `Target`'ta çıktı/alan yoktu | `Target.output` (`"v"`) | v2/v2r ajanları `output: "v"` kullandı |
| G10: ack neyin geri alındığını söylemiyordu | — | `ack.undoes` | v2/v2r ajanları ack'ten okudu |

v1 → v2: ilk deneme 15 → 14, sonunda 17 → 18. Değişikliklerin hedeflediği görevler (G02, G19)
düzeldi; ilk deneme oranı yükselmedi, çünkü v2'deki yeni ilk deneme kayıpları (G07, G13)
ajanların keşif turu ya da kendi JSONL biçim hatasıyla (G13: çok satırlı JSON) ilgili, tasarımla
değil. Örneklem küçük (20 görev, tek koşu); ±1–2 görevlik fark gürültü içinde.

## v2r: asıl bulgu

Kimlikler kopyalanamayınca ilk deneme 6/20'ye düştü, ama **sonunda başarı en yüksek (19/20)
ve hata cevabından sonra düzeltme 13/14**. Yani:

1. v1/v2'deki yüksek ilk deneme başarısının büyük kısmı şartnamedeki örnek kimliklerinin
   sızıntısıydı. Gerçek kullanımda ajan kimlikleri **bilmeden** başlar.
2. **Hata modeli işe yarıyor:** ilk denemede 12 görev hata cevabı aldı (`not_found`,
   `schema`); ajanlar hata iletisi ve bir listeleme sorgusuyla ikinci denemede düzeltti.
3. Kalan maliyet bir tur: "önce kimlikleri öğren, sonra yaz". Tek atışlık düzenekte bu ilk
   denemeyi yakar; etkileşimli kullanımda bir gidiş-dönüş eder.

## Kalan sorunlar ve sonraki tasarım önerisi (v3, uygulanmadı)

| # | Gözlem | Öneri |
|---|--------|-------|
| 1 | Her görev kimlik keşfiyle başlıyor (v2r) | **Anlamsal referans:** işlemler ve sorgular kimlik yerine anlamla da başvurabilsin: `{"ref": {"study": "rsi", "params": {"period": 14}, "on": "@bars"}}`, `"@main_view"`, `"@top_finding"`, `"@first_pending"`. Otorite tek eşleşmeyi kimliğe çözer, belirsizse `ambiguous` hatası ve adaylar döner. Böylece "RSI(14)'ün son 50 bardaki en yüksek değeri" tek atışta yazılabilir. Ayrıca `welcome`'a kısa bir doküman özeti (kimlik + tür + etiket). |
| 2 | G06: cevap yuvarlanıyor (78,12); kesin değer protokolde doğru döndü | Ölçüt bilerek katı tutuldu (bit düzeyinde eşitlik); bu bir sunum sorunu, protokol sorunu değil. |
| 3 | G07: "yakınlaştır" kalıcı mı geçici mi belirsiz | Şartname: kullanıcı jesti → `input`; "görünümü şuna ayarla" → `set_view_range`. Örnek eklenmeli. |
| 4 | G12: RSI 70'i birçok barda kesiyor; ajanlar olgu (`fact`) üzerinden 150'yi buldu, biri 189'u seçti | "Bulgunun dayandığı olgu" dikkat hedefi olarak doğrudan verilebilsin: `target.item = <olgu>` → otorite barı çözer. |
| 5 | "150. bar" 0 tabanlı mı sıra mı | Ajanlar şartnameye uyup 150 kullandı; ölçüt ikisini de kabul ediyordu. Doğal dil sıra sayısı için şartnameye not. |
| 6 | Şartname örnekleri test dokümanının kimliklerini kullanıyor | Örnekleri ayrı, açıkça "örnek" kimlikli bir dokümana taşımak; ölçüm düzeneği her zaman yeniden adlandırılmış dokümanla koşmalı. |

## Kayıtlar

- `raw/v1/`, `raw/v2/`, `raw/v2r/`: her ajanın `TASKS.txt`, her görevin deneme dosyaları
  (`attemptN.jsonl`), otoritenin tam çıktısı (`attemptN.jsonl.full.jsonl`), `answer.txt`,
  `notes.txt`; o turun `packet/` kopyası (ajanların gördüğü tek şey); `SCORE.md`, `score.json`.
- `raw/v2r-void/`: geçersiz çalıştırma ve nedeni.
- Düzenek: `harness/ergo.py` (çalıştır, denetle), `harness/score.py`, `harness/renamed/`.
  Yeniden puanlama: `python3 rounds/RD1/ergonomics/harness/score.py rounds/RD1/ergonomics/raw/v2`
  (v2r için `QP_ERGO_VARIANT=renamed`).
