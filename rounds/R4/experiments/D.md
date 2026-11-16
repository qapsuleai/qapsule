# R4/D — Cihaz ölçümü: çıktı notu

Dal `r4/d` (`r4/int`'ten). Sonuç belgesi: `rounds/R4/DEVICE.md` (tablolar, benzetimin sınırı,
telefon yönergesi, K17/K45 önerileri). Bu not: ne yapıldı, yöntem, ham çıktılar, açık sorular.

## Ne yapıldı

| Parça | Yer |
|-------|-----|
| Ölçüm çekirdeği: `zoom` kipi kabukta (tekerlek, kıstırma), p99, uzun kare oranı (`long_16_7`, `long_33_3`), 1 sn'ye kadar aralık sayılır (önce 250 ms üstü atılıyordu) ve daha uzunu `gaps_1s`, cihaz bilgisi (`device`, çekirdeğin `scale`'i dahil), bellek (`memory_bytes.js_heap`, `memory_bytes.wasm` — WASM belleğini çekirdek `memory_size` ile okur), özetin `look`'u, `metric_reset` | `crates/qp-wasm/src/shell.rs`, `lib.rs` |
| Kabuk: ham değerleri verir; **Sıfırla** düğmesi; tek satır yönerge; Ölçüm sekmesi açıkken saniyede bir yenileme; saniyede bir bellek örneği | `shells/web/main.js`, `index.html` |
| Mimari testi izin listesi (`metric_device`, `metric_memory`, `metric_reset`) | `shells/web/arch-check.mjs`, `ARCH.md` |
| Birim testi | `crates/qp-wasm/tests/metrics_d.rs` |
| Ölçüm betiği (belirleyici sıra, 3 tekrar, medyan, kopyalama denetimi) | `shells/web/tests/device.mjs` |
| README (Ölçüm satırı, telefon yönergesine bağlantı, betik) | `shells/web/README.md` |
| Sentetik ölçüm medyanları | `rounds/R4/experiments/D/device-synthetic.json` |

Değişen döküm, altın görsel, görünüm altını, hesap özeti yok; ölçüm alanları yalnız kabuk
çekirdeğinin özet JSON'unda (protokol değil). Yeni bağımlılık yok.

## Yöntem notları

- **250 ms kesiminin düzeltilmesi.** İlk tam koşuda `r3` DPR3 4×/6× satırları "iyi"
  görünüyordu (p95 küçük) ama evrede yalnız 3–240 kare vardı: ana iş parçacığı doyunca kare
  aralıkları 250 ms'yi aşıyor ve T5'in çekirdeği bunları atıyordu. Kesim 1 sn'ye çıkarıldı,
  üstü `gaps_1s`; `device.mjs` her evre için kare/sn verir. Bütün matris yeniden koşuldu
  (DEVICE.md'deki sayılar ikinci koşudan).
- **Playwright tıklaması.** Grafik katmanı açıkken `#c_play`'e `page.click` bir koşuda
  "kararlı" denetiminde 30 sn bekledi; düğme DOM'dan tıklanıyor (aynı işleyici). Koşu düşerse
  yeniden denenir, sayısı `failed_runs` (son koşuda 0).
- Süre: bir veri kaynağı için 12 yapılandırma × 3 koşu ≈ 16 dk.
- Sentetik ve gerçek koşular arka arkaya; yük ortalaması 7,0 → 4,9 → 5,0.

## Ham çıktılar

Sentetik: `D/device-synthetic.json` (yapılandırma başına medyanlar: açılış, dört evrenin kare
istatistikleri ve kare/sn, tik/yüz/sahne süreleri, bellek, cihaz, kullanılan görünüm,
kopyalama denetimi). Ham koşular `--raw` ile alınabilir; depoya konmadı (boyut).
Gerçek veri: çıktı depoda değil (yalnız toplu süreler `DEVICE.md` §2.2–2.3'te).

Tekrar: `cd shells/web && node tests/device.mjs ../../target/release/qp` (önce
`scripts/demo.sh`'ın paket derlemesi; `QP_DEMO_REBUILD=1 scripts/demo.sh` sonra durdurmak ya da
yalnız wasm-pack adımı). Gerçek veri: `... -- --data <bars> --from 2025-06-02`.

## Önerilen kararlar

- **ÖK-D-1 (K17):** telefon sınıfı hedefi iki basamak, geçici (DEVICE.md §5): orta sınıf
  (≈ 4× DPR3) ve taban (≈ 6× DPR3); telefon ölçümü gelince telefon sayılarıyla yazılır.
- **ÖK-D-2 (K45):** GPU ertelenmeye devam; tetikleyici orta sınıf telefonda grafik kare p95
  > 33 ms ya da sahne boyama p95 > 16,7 ms (DEVICE.md §6).
- **ÖK-D-3:** ölçüm ekranında kare aralığı 1 sn'ye kadar sayılır; ölçüm özeti cihaz ve
  bellek taşır.

## Açık sorular

1. Kısıt oranı ↔ telefon sınıfı eşlemesi telefonda doğrulanmadı (Arkin'in ölçümü).
2. Gerçek dokunma (momentum kaydırma, iki parmak kıstırma) başsız düzenekte yok.
3. Tik p95'in kaynağı ayrıntılı bölünmedi (akış yeniden kurulumu mu, yüz boyama mı); ölçüm
   ekranında tik ve yüz ayrı, ama tik içindeki yüzler tik süresinin içinde.

## Öneriler (kapsam dışı)

- **Tikte yüz boyamayı hafifletmek:** yeni kapsül gelince yalnız değişen yüzleri boyamak;
  görünmeyenleri ertelemek; boyamayı tikler arasına yaymak (orta sınıfta tik p95 35,9 ms).
- **Hareket sırasında düşük ölçek:** DPR 3'te sürükleme/kıstırma sürerken 2× boyamak, bitince
  3×; çekirdekte bir bakış ayarı.
- **Barındırıcıda sıkıştırma:** ham 4,4 MB WASM yerine önceden sıkıştırılmış `.wasm.gz`
  (1,29 MB) — telefonda açılışın en büyük kalemi olabilir.
- `r3` yüzü her tikte değişiyor; insan testinde seçenek olarak kalırsa DPR 3'te yavaş
  telefonlarda akış donar (DEVICE.md §2.3).
- `device.mjs`'nin kısa bir kipi (1 yapılandırma, 1 koşu, 2 sn evre; ≈15 sn) verify `demo`
  adımına girebilir: Ölçüm ekranının kopyalama yolu ve alanları her koşuda sınanır.
