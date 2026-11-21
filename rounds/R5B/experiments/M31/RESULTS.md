# M31 — Nöbetçinin bildirim süzgeci: sonuç

Düzenek: `rounds/R5B/experiments/harness/SPEC.md` §2, §10 (seçim kuralı önceden yazılı,
değiştirilmedi). Süzgeç ürün kodu `crates/qp-capsule/src/sentinel.rs` (deterministik, dil
modelsiz, K3); koşu `crates/qp-session/examples/m31_sentinel.rs`.

## Koşu

```text
cargo run -p qp-session --release --example m31_sentinel -- rounds/R5B/experiments/M31
python3 rounds/R5B/experiments/harness/score.py m31 rounds/R5B/experiments/M31/<süzgeç>/notifications.jsonl
```

- Oturum `session/init.r5b.json` üzerinde `ReplayHost`; sabit konum (`position: null`).
- `claude` (ajan, `on_behalf: arkin`, işaretli bağlantı) `m31/watches.json`'daki üç
  izlemeyi `add_watch` ile kurdu (hepsi `ack`); `intents` ve `capsules {all: true}`
  konularına abone; ikinci ajan bağlantısı `capsules` (kitle süzgeçli) — denetim için.
- Pencere (10 gün) **barındırıcı** tarafından ilerletildi: `ReplayHost::host_control(step 1)`
  × 10 (ajan değil).
- Çıktılar: `events.jsonl` (ham olaylar), `<süzgeç>/notifications.jsonl`,
  `<süzgeç>/score.json`, `check.json`.

## Sonuç

| Süzgeç | Bildirim | Gün başına | Kaçırılan zorunlu | Gereksiz | Gereksiz oranı | Önemli yakalama | Tekrar | Uydurma |
|--------|---------:|-----------:|------------------:|---------:|---------------:|----------------:|-------:|--------:|
| `every` | 9 | 0,9 | 0/7 | 1 | 0,111 | 0,80 | 0 | 0 |
| `tier_sens` | 9 | 0,9 | 0/7 | 1 | 0,111 | 0,80 | 0 | 0 |
| `tier_repeat_hold` | 8 | 0,8 | 0/7 | 0 | **0,000** | 0,80 | 0 | 0 |

**Seçim (kural §10):** (1) üçü de zorunluyu geçer (kaçırılan 0, uydurma 0); (2) gereksiz
oranı en düşük `tier_repeat_hold` (0). **Seçilen: `tier_repeat_hold`.**

Gereksiz bildirim (`every`, `tier_sens`): 3. gün `cap.SYN04.continuation.up.159 weakened`.
SYN04'ün etkin izlemesi olduğu için motor bu kapsülü `relevant` sayar ve olumsuz geçişi K42
gereği akışa girer; ama izleme "SYN04'te yukarı kırılım doğarsa" der, geçiş ona uymaz ve
SYN04'te pozisyon yok. `tier_repeat_hold`'un "elindekiler önceliği" bunu düşürür.

**Belirlenimcilik:** iki ayrı koşu, bütün çıktıların sha256'sı eşit:

| Dosya | sha256 |
|-------|--------|
| `events.jsonl` | `e215b9555c4cd38caa2a21be5ff9689e35328ad2619cde0c7285e990d56f2c88` |
| `every/notifications.jsonl` | `2272d5f47177df29da9d2df49aefa845bbf1b2f270740005f386438103ad8b23` |
| `tier_sens/notifications.jsonl` | `396b175e0269eaa6243967deec4107919bf941eaffa4178c420ac463cb6a09b9` |
| `tier_repeat_hold/notifications.jsonl` | `26647abfb638bea808b492a0fe019f8019c3fb99ea8d687b46eb12386d61662d` |
| `check.json` | `40992fa56899f73c189563c3dae40e5f44b28c4b2149686d50320d16c111614b` |

## Motorun olayları ↔ `m31/events.json`

- 72 geçiş, aynı `(gün, kapsül, aşama)` kümesi ve aynı sıra; `kind`, `direction`, `tier`,
  `coin_flip` hepsi eşit.
- **`seq`** her olayda +3: üç `add_watch` komutu günlüğe üç kayıt ekler (düzenek izlemesiz
  üretti).
- **`relevant`** 3 olayda `false → true`: `cap.SYN04.continuation.up.159` (born, weakened,
  strengthened). Neden: SYN04 izlemesi etkin niyet; `relevant` = "pozisyon ya da etkin
  niyetle ilgili" (§20.4, §25.5). Düzenek izlemeler kurulmadan üretildiği için `false`.
  Puanlama zorunluyu `events.json`'daki `relevant`'tan alır (yalnız pozisyonlar) — zorunlu
  küme değişmez (7).
- İzleme eşleşmeleri (8): "elimdekilerde olumsuz" 7 kez (zorunlu yedi olayın tamamı),
  "herhangi bir sıkışma doğumu" 1 kez (`cap.SYN11.squeeze.neutral.168`, 10. gün), "SYN04
  yukarı kırılım doğumu" 0 (pencerede böyle doğum yok; doğru).
- Süzgecin `in_feed` kuralı motorun kitle süzgeçli `capsules` aboneliğiyle birebir aynı
  (`check.json`: 9/9, fark yok).

Motorda hata bulunmadı; `watch.rs` değişmedi.

## Gözlemler ve açık noktalar

1. **Önemli yakalama 0,80, üç süzgeçte de:** kaçan iki önemli olay izleme listesinde kademe
   1 doğumlar — `cap.SYN02.breakout.down.162` (4. gün) ve `cap.SYN03.breakout.up.168` (10.
   gün). İkisi de yazı tura (`coin_flip: true`); süzgeçler doğumu akışa girme kuralıyla
   (K53, `balanced` seviyesinde yazı tura akışa girmez) değerlendirir. Seçim kuralı yakalamayı
   ölçmüyor; bildirim kanalının akıştan geniş olmaması bilinçli seçim. Yazı tura kademe 1
   doğumun bildirilip bildirilmeyeceği ürün kararıdır (ana akışa soru).
2. `every` ile `tier_sens` bu pencerede aynı çıktıyı verdi: `balanced` seviyesinde kişinin
   akışı zaten dar (10 günde 9 geçiş). Fark daha geniş seviyede (`all`) ya da daha uzun
   pencerede görülür; pencere küçük (10 gün, 10 önemli olay) — ayrım gücü sınırlı.
3. `tier_repeat_hold`'un tekrar bastırması bu pencerede hiçbir bildirimi düşürmedi
   (`SYN02.breakout.down.162` doğum+teyit aynı gün, ama ikisi de yazı tura yüzünden zaten
   dışarıda); farkı yalnız elindekiler önceliği yaptı.
4. K42 güvencesi yapıda: zorunlu (pozisyondaki hissede olumsuz geçiş) süzgece sorulmadan
   karar verilir; `tests/sentinel.rs` her süzgeç × seviye × aşama × kademe × ilgili ×
   yazı tura × önceki bildirim birleşiminde düşmediğini sınar.
