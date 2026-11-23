# RF1 — Karar noktası

**Tarih:** 2026-11-23.
**Karar veren:** Arkin.
**Plan:** `PLAN.md` v2 (`aaa207a5`).
**İnceleme:** ana akış oturumu (`MAINFLOW-REVIEW.md`).

Round oturumu Arkin'e sordu: "Bunu senin onayın olarak DECISION.md'ye aynen alıp gramer ve
oturum işine başlayayım mı?" Arkin'in cevabı: **"Evet, onayımdır."** İncelemenin metni aşağıda
aynen yer alıyor.

---

2026-11-23. Bu belge Arkin'in karar noktasındaki cevabıdır; `DECISION.md`'ye aynen alınabilir.

## Hüküm: plan onaylandı; aşağıdaki dört düzeltmeyle ilerle

v2 dürüst ve uygulanabilir: v1'in döngülü sınırlarını ölçümle geri aldı, çekirdekte kalanı
açıkça söylüyor, her dalganın geri dönüşü var.

## Üç sorunun cevabı

1. **Çekirdek istisnası: kabul.** `qp-authority` için tek adlı istisna; tavan ölçülen değer,
   13.000'i geçemez. Okuma ya da yansı dalgası çekirdekte kalmak zorunda kalırsa (tavan aşılır)
   yeniden sor; o durumda hangi dalganın neden tutmadığını ve çekirdeğin yeni boyutunu getir.
2. **Kapsül yaşamı, iddia/itiraz/yorum ve canlı yolun çekirdekte kalması: kabul.** Bayt eşitlik
   önce gelir. Koşulu: çekirdek donuktur. `qp-authority`'nin tavanı mandallıdır; sonraki round'lar
   çekirdeğe kayıt satırı dışında dokunmaz. Kalan ayrım `docs/DEBT.md`'ye adıyla yazılır
   (aday: RS1'den sonra, sicil kapsül karnesini devraldığında).
3. **Crate adları: kabul.** Tek not, bu round'un işi değil ama plana yazılsın: §5'teki
   `qp-drawing` (alan) ve `qp-drawings` (oturum) çifti yalnız çoğul ekiyle ayrılıyor; R7 bu
   ikisine ayırt edilir adlar verir.

## Dört düzeltme

**D1. Sicilin (RS1) kapısı çekirdekte açılmalı.** Kapsül yaşamı ve iddialar çekirdekte kalınca
RS1'in en çok ihtiyaç duyduğu iki şey çekirdeğin içinde kalıyor: yeni iddia türü (çizim
izlemesi, sinyal; eleştiri 2, yük tablosu) ve bir iddianın sonucunun kesinleştiği an. Fazlara
bir **sonuç kesinleşti** fazı ekle (bugün sonucun yazıldığı satırda çağrılır) ve iddia türü için
kayıt kolu aç. İkisi de `Adjust` gibi taslak kanca olabilir: tip, çağrı noktası, birer sınama.
Bayt eşitliği tutmuyorsa yapma; o zaman raporda "RS1 çekirdeğin şu dosyalarına dokunacak" diye
adıyla yaz, ki RS1 paketi buna izin versin.

**D2. Yeni doküman nesnesi yolu yazılı ve yer açık olmalı.** RL1 (program) ve R7 (çizim)
dokümana yeni nesne türü ekleyecek; bu yeni `ChangeOp` kolu ve `qp-ops`'taki kapalı `match`'ler
(uygulama, ters çevirme, kanonik biçim, özet, açıklama) demek. Bu round'da `ChangeOp` için kayıt
kurma; ama: (a) `docs/ARCHITECTURE.md`'ye "Yeni doküman nesnesi nasıl eklenir" bölümü yaz:
dokunulacak dosyaların tam listesi, hedef "otorite 0 dosya"; (b) `qp-ops` 10,2 bin ile doğuyor,
pay 1,8 bin. Payı en az 2,5 bine çıkar (veri düzlemi ve sunucu ayrı crate'e ayrılabilir) ya da
neden yetmeyeceğini ölçümle göster.

**D3. Program çıktısı bir değer kaynağıdır.** RL1'in programları yalnız koşul değil, grafikte
çizilen ve başka koşulların okuduğu seri de üretecek. `Condition` kaydına ek olarak veri
düzleminde **değer kaynağı** için de kayıt kolu olsun (taslak kanca yeter): bir alt sistem
"bu adresin değerini ben veririm" diyebilsin. Yoksa RL1 veri sağlayıcının çekirdekteki
uygulamasına dokunmak zorunda kalır.

**D4. Ek noktası kanıtı riskli dalgalardan önce.** Sıra: S3.0 → S3.1 (`qp-watch`) → **E** →
S3.2 → S3.3. Okuma ve yansı dalgaları tutmasa bile sözleşme ve kanıtı elde olur. `Tally`
sınaması bir fazı, bir sorguyu ve bir konuyu da kullansın; yalnız işlem kaydını değil.

## Küçük notlar (karar gerektirmez)

- `Session(Authority)` + `Deref`: çağrı yerlerini korumak için kabul; yeni kod `Deref`'e
  yaslanmasın, `sub::<T>()` kullansın. README'ye yaz.
- Faz çağrıları tik yolunda (`AfterData`): K52 kapısı her dalgada ölçülüyor; boş kayıtla
  maliyetin sıfıra yakın olduğunu S3.0 sonunda bir kez göster.
- CONCEPTS'e eklenecek adlar (alt sistem, faz, bileşim, işlem kaydı, tasarım kaydı, canlı yol)
  kod yazılmadan önce girsin; plandaki gibi.
- Geri dönüş kuralı (tutmayan alt sistem çekirdekte kalır) doğru. Geri dönen her dalga raporda
  nedeniyle ve bir sonraki denemenin neye ihtiyaç duyacağıyla yazılsın.

---

## Uygulama notları (round oturumu)

- **G2a, üretilmiş dosyanın yeri.** `PLAN.md` §3.1 ve §9 (B15–B19: `advances` taşıması
  `config-diff`'te `DECISION.md`'yi anar) gereği `crates/qp-grammar/src/advances.rs`,
  `look` ile birlikte saf taşımayla `crates/qp-look/src/advances.rs` oldu. Üreticisi aynı
  (`QP_BLESS=1 cargo test -p qp-render --test fonts`), yalnız yazdığı yol değişti;
  `arch-config.json` `generated_files` girdisi yeni yolu gösterir. Bu bir gevşetme değil,
  aynı istisnanın yer değiştirmesidir.
