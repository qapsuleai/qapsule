# R5/T2 — bozuk tasarım sınaması, görünüş kaydıyla (2026-11-18)

> Ajan okuyucu insan değildir; bu bir düzenek sınamasıdır (SPEC §G0, §G8). Tasarım kararı değildir.

**Sonuç: GEÇTİ.** Bozuk tasarım (`kestim`, gizli ad) birincil ölçüde ve eleştirmen BT'de **kesin son**.
Birincil aralığı en iyinin aralığının altında. Eşleştirilmiş farkla her iki adaydan ayrık (önyükleme
%95 alt sınırı > 0). Ayrıca tam boy okumada da son.

## Kurulum (SPEC §G8 Ek G2)

- Bozuk tasarım artık gerçek bir görünüş kaydı (`broken/look.patch` → `look.rs` `KESTIM`):
  - yazı rolleri ters: başlık cümlesi 11 px; üst satır 24 px, karne 22 px;
  - iki temada metin, işaret, anlam ve kabuk renkleri %65 zemine çekili;
  - işaret yüzde 12 px.
- **Ürüne girmedi.** `broken/build.sh` yamayı HEAD'in (`main` `c8fce42`) `git archive` kopyasına
  uyguladı ve `qp` ile WASM paketini orada derledi (≈ 5 dk; `CARGO_TARGET_DIR=target/wt/t2-broken`).
  Çalışma ağacında ve dalda `look.rs` değişmedi; yalnız yama dosyası commit edildi.
- Üç aday da **aynı derlemeyle** çekildi: `vadrel` (yüz `vadrel`, görünüş `r4`; başvuru), `fotolay`
  ve `kestim` (`?look=kestim`, yüz `vadrel`). `compute` bozuk değerleri sahne tarifinden okudu:
  - metin karşıtlığı 1,62;
  - işaret 1,07;
  - roller `headline` 11 px / 1,66 ve `title` 24 px / 2,13 (`broken/run/computed.md`).
- Kapsam:
  - 8 yüz durumu (§G2) + akış ekranı;
  - TR + EN, açık + koyu → 108 görüntü;
  - 396 okuyucu uyaranı ve 48 eleştirmen çifti türetildi.
- Durum kümesi kuru koşuyla aynı (S* = adım 1).

## Ölçümler (`broken/run/scores.md`, `broken-check.json`)

| tasarım | tek bakış: kısma kopyası (birincil, F2:H1) | okuma: tam boy (kapı) | BT [%95] |
|---|---|---|---|
| `vadrel` | 0,314 [0,297, 0,332] | 0,899 [0,872, 0,925] | +1,47 [1,22, 1,73] |
| `fotolay` | 0,261 [0,243, 0,279] | 0,880 [0,853, 0,906] | +1,63 [1,37, 1,92] |
| `kestim` (bozuk) | **0,236** [0,222, 0,252] | **0,816** [0,745, 0,882] | **−3,10** [−3,21, −2,98] |

- **Örneklem:**
  - Kısma: 3 aday × 4 dil/tema × 8 durum × 2 persona × **6 tekrar** = 1152 haiku çağrısı; aday başına ≈ 380 birim.
  - Okuma: 1 tekrar, 192 çağrı.
  - Eleştirmen: 8 durum × 3 çift × 2 sıra × 3 tekrar = 144 sonnet çağrısı, TR/açık.
- **Eşleştirilmiş fark** (aynı durum/dil/tema/persona/tekrar; F2:H1; önyükleme 2000, tohumlu):
  - `fotolay − kestim` = +0,025 [+0,002, +0,049];
  - `vadrel − kestim` = +0,078 [+0,057, +0,098].
- **Tekrar sayısı:** 3 tekrarda `fotolay − kestim` +0,021 [−0,012, +0,053] idi, yani ayrık değildi. 6 tekrarda ayrıldı. Bu yüzden **birincil ölçüde 6 tekrar** öneriyorum (README maliyet tablosu 3 tekrarla hesaplı; 6 tekrarla haiku kalemi ≈ iki katı).
- **"İlk ne görüyorsun"** (`t_first`), işaret diyenler:
  - `vadrel` 153/381;
  - `fotolay` 156/383 (+ grafik 71);
  - **bozuk 0/379** (cümle 356, sayı 22).
- **Kısmanın öteki soruları:** `t_dir` 0,05–0,18 ve `t_valid` 0,01–0,22. Taban düzeyine yakınlar; birincil ölçünün ayrımını büyük ölçüde `t_first` taşıyor (aşağıda risk 1).
- **Eleştirmen:**
  - bozuklu çiftlerde konum çelişkisi 0/24 ve 0/24;
  - `fotolay|vadrel` 11/24 = 0,46 → bu çift "ayırt edilemedi" (> %30);
  - uzunluk yanlılığı: kesin sonuçlarda daha çok metinli ekranın kazanma payı 0,07 (n = 58); eleştirmen metni kayırmadı.
- **Körlük:**
  - 1488 kaydın hiçbirinin isteminde tasarım adı geçmiyor;
  - 10 çağrı birden çok `Read` yaptı ve puanlamadan çıkarıldı.
- **Maliyet:** haiku 1344 çağrı 0,965 $ + sonnet 144 çağrı 1,675 $ = **2,64 $**. Kuru koşuyla birlikte T2 toplamı ≈ 3,8 $.

Not: Seçim kuralı bu küçük örnekte `vadrel`'i `fotolay`'dan ayırdı (kısma aralıkları ayrık). Bu
yalnız düzeneğin çalıştığını gösterir; M16 kararı değildir, `fotolay`/`vadrel` R4'te seçilmiş
yüzlerdir.

## Düzenekte bu işle değişenler

- `broken/look.patch` ve `broken/build.sh` (yeni).
- `glance.py`:
  - `--web` / `QP_WEB` (bozuk derlemenin kabuk dizini);
  - `shoot` her tasarımın görünüş kaydını `qp look show` ile `looks/<ad>.json` olarak saklar;
  - `k52 --web`.
- `gl/compute.py`: her yüz metninin yazı rolü, görünüş kaydından boyla eşlenir (T3 `mark_role`). Sonuç `roles` alanında. Gösterim sahnesi rol taşımıyor; `qp scene --roles` yalnız oturum görünümlerinde çalışıyor. Kabuk renkleri `core.css()` ile DOM'a yazılıyor ve hesaplanmış biçemden okunuyor.
- `gl/select.py`: bozuk tasarıma karşı eşleştirilmiş fark (`paired_vs_broken`, bilgi; kabul kuralı değişmedi). Birim testi eklendi (30/30 geçti).
- `SPEC.md` §G7.8 ve §G8: Ek G2 (görünüş kaydı yolu). README güncellendi.

## Açık riskler

1. **Kısma bileşimi:** ayrımı çoğunlukla `t_first` sağlıyor. `t_valid` ikili bir soru ve yanıt yanlılığı gösteriyor (hep "bitmiş" diyen biri kapanmış durum payı kadar puan alır). `t_dir` bulanıkta tabana yakın. M16 tam koşusundan önce `t_valid` durumlarının dengelenmesini (açık/kapanmış eşit) öneririm. Bu kural değişikliği alternatifler gelmeden yapılmalı.
2. **Pay:** `fotolay` ile bozuk arasındaki fark küçük (+0,025). Tam koşu 6 tekrar ve 4 dil/tema ile yapılmalı. Bozuk tasarım her M koşusunda yeniden yarışmalı.
3. **Sert eşikler:** `vadrel` ve `fotolay` dört kapıyı geçemiyor (işaret 1,27; en küçük yazı 10 px; dokunma; renk), bu yüzden uygulanmadı (E6 kuralı). Metin karşıtlığı kapısı artık geçiliyor: T3 kabuk renkleriyle 5,27.
4. **Model ailesi:** tek aile (haiku/sonnet); ikinci aile yok.
