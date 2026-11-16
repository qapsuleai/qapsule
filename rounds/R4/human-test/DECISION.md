# Karar kuralı (önceden; hiçbir katılımcıdan önce yazıldı)

Bu kural `score.py` `decide()` içinde aynen uygulanır ve raporun "Karar" bölümünü üretir.
Sonuç görüldükten sonra değiştirilirse değişiklik ve nedeni sonuç notunda açıkça yazılır.

**Sınır cümlesi: persona ajanı insan değildir.** M8–M11 persona ölçümleri ön elemeydi. İnsan
sonucu ajan sonucundan farklıysa **insan sonucu geçerlidir** ve ajan seçimi geçersiz sayılır.
İnsan sonucu "eşit" çıkarsa ajan seçimi yerinde kalır ama "insanca onaylandı" denmez
("onaylanmadı, değiştirilmedi").

## Ne kapanır

| Karşılaştırma | A (ön seçim) | B | Kapanan |
|---|---|---|---|
| G5 alfabe | `siluet` (Ö32) | `rozet` | **A21** — her durumda kapanır (eşitlikte `siluet`) |
| G1 yüz | `Y1` (M8 birincisi) | `Y2` (M8 ikincisi) | **yüz seçimi** — her durumda kapanır (eşitlikte `Y1`) |
| G2 akış | `elikov` (M9) | `vakiro` | M9 seçimi onaylanır ya da değişir |
| G3 geçiş | `zumlak` (M10) | `tutamir` | M10 seçimi onaylanır ya da değişir |
| G4 sonuç işareti | `tg5` (M11) | `r3` | M11 seçimi onaylanır ya da değişir |

Yüz adları `plan.json` `faces`'tadır (`Y1`, `Y2`).

## Kural (her karşılaştırma için ayrı)

0. **Yeterlilik.** İki tasarımı da cevaplamış en az **5** katılımcı ve her grupta (alışkın, genç)
   en az **2** kişi yoksa karar yok: test sürer, ön seçim geçici kalır.
1. **Doğruluk.** Birincil ölçüde (`SCORING.md`) katılımcı başına eşli fark (A − B), ağırlıklı
   ortalama (genç 2, alışkın 1). Kazanan şu üçü birden sağlanırsa belli olur:
   - ağırlıklı fark mutlak değerce **≥ 0,10**,
   - aynı yönde daha iyi olan katılımcı sayısı öteki yöndekinden **en az 2 fazla**,
   - iki grup ters yönde ayrışmıyor (iki grubun ortalama farkı ikisi de ≥ 0,10 ve ters işaretli
     değil). Ayrışırsa kural 1 uygulanmaz, kural 2'ye geçilir ve rapor bunu yazar.
2. **Tercih.** Kural 1 karar vermezse: tercih belirtenlerin (fark yok hariç) **en az ⅔'ü** ve en az
   **3** kişi aynı tasarımı seçtiyse o.
3. **Süre** (yalnız G3; doğruluğun tavanda olması beklenir, M10 gözlemi): kural 1 ve 2 karar
   vermezse ve katılımcı medyan süre farklarının medyanı **≥ 2 sn** ise hızlı olan.
4. Hiçbiri karar vermezse **eşit**: A kalır (A21 ve yüz seçimi böylece de kapanır; kayıt
   "insan testinde ayırt edilmedi" diye düşülür).

Kural 1'i geçen bir sonuç, persona ölçümünün tersi olsa da geçerlidir. Kural 2–3 ile verilen
karar "zayıf" diye işaretlenir; sonraki round'da daha çok katılımcıyla yeniden bakılması
önerilir.

## Neden bu eşikler

- 0,10: R3 insan testi kuralıyla aynı (`rounds/R3/human-test/SCORING.md`); G1'de 3 kapsüllük bir
  setin bir kapsülü = 0,33, G4/G5'te bir işaret = 0,33; yani eşik "en az bir maddelik fark"tan
  küçük değildir ama tek bir katılımcının tek maddesine dayanmasın diye işaret sayısı şartı eklenir.
- "En az 2 fazla": n = 5'te 4–1 ya da 3–1 (+1 eşit); n = 10'da ör. 6–4. İşaret testi p değeri
  raporlanır ama karar için kullanılmaz (güç yetmez; R3'te de aynı uyarı).
- Genç ağırlığı 2: NORTH_STAR'ın birincil kullanıcısı "geleceğin yatırımcısı"; persona
  ölçümlerinde de F 2, H 1.

## Kayıt

Karar `DECISIONS.md`'ye orkestratör/ana akış eliyle yazılır: A21 "KAPANDI" (kanıt: `results.md`
yolu, n, ağırlıklı fark, işaret sayısı, kuralın hangi adımı); yüz seçimi, M9/M10/M11 onay ya da
değişiklik aynı biçimde. Varsayılan değişikliği (ör. `DEFAULT_FEED`) ayrı bir ürün işidir.
