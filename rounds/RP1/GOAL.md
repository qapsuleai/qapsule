# RP1 — Araştırma: Hesap Çekirdeği Performansı

Bu bir araştırma round'udur (K18). Amaç bilinen bir çözümü uygulamak değil, hesap
çekirdeğinin "ajanın düşünme hızı"nı belirleyen sınırlarını bulmak ve ötesine geçmenin
yollarını denemektir.

## Soru

Bir ajan "hangi indikatör, hangi periyot, hangi parametre bu hareketi en iyi açıklıyor?"
diye sorduğunda, cevabın insanın fark etmeyeceği kadar hızlı gelmesi için çekirdek nasıl
kurulmalı?

## Değişmezler

- Sonuç deterministiktir ve tek tek çalıştırmayla **bit-eşittir**; aarch64, x86_64 ve
  wasm32 arasında da.
- Hesapta LLM, ağ, saat, rastgelelik yok. Tüm değerler `f64`.
- Bu değişmezleri bozan bir yaklaşım çok hızlıysa uygulanmaz; ölçümü ve bedeli rapora
  ayrı bir bölüm olarak yazılır, karar ana akışa bırakılır.

## Kapsam

1. Ölçüm tabanı: R1 `BENCH.md` düzeneği genişletilir. Yük profilleri: (a) tek seri,
   çok parametre (bugünkü 10k × 1.000); (b) çok seri, tek hesap (ör. 500 sembol × 5
   periyot); (c) zincirli hesaplar; (d) canlı güncelleme (tek barda yüzlerce hesabın
   son değeri).
2. Yaklaşım araştırması: literatür ve var olan motorlar incelenir, sonra kendi
   yaklaşımların geliştirilir. Başlangıç için yönler (liste sınır değildir):
   ortak alt hesapların paylaşımı ve hesap grafiği; bellek yerleşimi ve önbellek
   davranışı; veri paralelliği (SIMD, şeritler) ve bit-eşitlikle ilişkisi; iş
   parçacıkları; artımlı ve tembel hesap (yalnızca görünen aralık, yalnızca istenen
   çıktı); parametre uzayında yapı (bir periyodun sonucundan komşusuna geçiş); sonuçların
   içerik adresli önbelleği; GPU hesap.
3. Her yaklaşım için: hipotez, küçük deneme, ölçüm, karar (al / alma / sonra).
4. Kazananların çekirdeğe alınması, testlerle.

## Kapsam dışı

- Yeni indikatör. Çizim. Şema değişikliği (gerekiyorsa öneri olarak yazılır).

## Kabul ölçütleri

| # | Ölçüt |
|---|-------|
| 1 | `rounds/RP1/RESEARCH.md`: denenen her yaklaşım hipotezi, ölçümü ve kararıyla kayıtlı; işe yaramayanlar dahil. En az altı farklı yön denenmiş. |
| 2 | Dizüstü sınıfı hedefleri (K17) beş indikatörün hepsinde GitHub Linux koşucusunda da tutuyor; tutmuyorsa neden tutmadığı ve neyin gerektiği ölçümle gösterilmiş. |
| 3 | Dört yük profilinin her biri için önce/sonra tablosu: referans makine, CI koşucusu, WASM. |
| 4 | Bit-eşitlik testleri (tarama = tek çalıştırma; mimariler arası özet) değişmeden geçiyor. |
| 5 | Değişmezleri bozan ama umut vadeden yaklaşımlar ayrı bölümde, bedeliyle birlikte, karar için ana akışa bırakılmış. |
| 6 | Bir sonraki sınırın ne olduğu yazılmış: bugün en yavaş olan ne, ve onu aşmak için hangi mimari değişiklik gerekir. |
| 7 | `REPORT.md`, `AUDIT.md`; `STATUS.md`, `ROADMAP.md` güncel. |
