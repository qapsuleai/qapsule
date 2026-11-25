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
- **S4, `expand_op` dağıtıcısı (Arkin'in kararı, 2026-11-24).** `expand_op` 903 satırdan
  23 kol fonksiyonuna bölündü (her biri ≤ 80). Kalan dağıtıcı `match`, alanları açılmış 23 kolla
  `rustfmt` biçiminde 101 satır. Round oturumu üç seçenek sundu: (a) taban çizgisi kalemi,
  (b) `rustfmt::skip`, (c) ölü kollu iki katmanlı dağıtım. Arkin'in seçimi: **(a) taban
  çizgisine al.** `arch-baseline.json`'a `crates/qp-ops/src/expand/op.rs::expand_op` 101 kalemi
  ve `lint.allow` `qp-ops` 21 girer (dağıtıcının `too_many_lines` izni). `main`'e göre borç
  küçülüyor: `expand_op` 903 → 101, `lint.allow` toplamı azalıyor. Kalem mandallıdır.
- **S3.0, İ3'ün yeri.** `qp-session`'ın içeriği saf taşımayla `qp-authority` oldu (otorite
  çekirdeği, `PLAN.md` §3.3 satır 5); `qp-session` artık ince bileşim (satır 8). Arkin'in
  karar noktasındaki 1. cevabı (`qp-authority` için tek adlı istisna; tavan ölçülen değer,
  13.000'i geçemez, mandallı) gereği `crate.lines` istisnası İ3 aynı içerikle
  `qp-authority`'ye taşındı. Tavan gevşemedi: ölçülen değer 20.437 (S2 sonu), tavan aynı.
  13.000 sınırı S3.2/S3.3 dalgaları (okuma, yansı, görüntüleyici, barındırma) çıkınca
  uygulanır; o zamana kadar tavan yalnız aşağı iner. Bu bir gevşetme değil, aynı istisnanın
  crate adı değişen öğeye taşınmasıdır.
- **S3.0, araç izni (Arkin'in kararı, 2026-11-24).** Round oturumu sordu:
  (A) taban çizgisi yazımı (`conform.rs` kalemi `crates/qp-session/src/conform.rs`'e taşınır;
  `committed`, `build_with_keys` ve eski `conform.rs` kalemleri sıkılaşır; yeni borç yok);
  (B) İ3 istisnasının `qp-session` → `qp-authority` yer değiştirmesi (aynı kural, aynı tavan
  20.437). Arkin'in cevabı: **"İkisine de onay."** (B) için `config-diff`'e "yeni crate'e taşınan
  istisna" tanıması eklenir: aynı kimlik ve kural, tavan yükselmez, eski istisna kalemi artık yok (eski crate
  başka bir rolle sürebilir: S3.0'da `qp-session` bileşim oldu), yeni kalemin crate'i bu
  değişiklikte yapılandırmaya girmiş; bir yeni kaleme tek eski istisna taşınır ve aynı kural ve
  kalem iki kez yazılamaz (denetim `audit-s30.md` B7). Öz-sınamasıyla gelir. Bu round'un kalan taşımalarında
  (S3.1–S3.3) aynı türden taban çizgisi yazımları yapılabilir: yalnız taşıma ve sıkılaşma, yeni
  borç yok.
- **S3.2, izleyici otoritelerin listesi (round oturumunun uygulama kararı, 2026-11-24).**
  İzleyici otoriteler — yansı (`qp_session::Replica`) ve görüntüleyicinin açık tarafı
  (`qp_session::viewer::OpenSide`) — `FOLLOWER` = `STANDARD` eksi okuma (`qp-read`) ile
  kurulur; bağlantı konuşan otoriteler (oturum, barındırıcı) `STANDARD` ile. Gerekçe ölçüm:
  okuma alt sisteminin kaydı, sorgu ve girdi işleyicilerinin tablosu üstünden bütün okuma
  kodunu ve `schemars` şemalarını WASM kabuk paketine çekiyordu (+1,38 MB; paket +%17,6,
  sınır +%2); `FOLLOWER` ve ayrı `ReadView` ile +%1,92. Davranış aynı: izleyiciler sorgu ya
  da girdi almaz (`handle_raw` çağrısı yok; denetim `experiments/audit-s32.md` adım 2).
  `PLAN.md` §3.3 satır 6'dan ("yansı kendi otoritesini `qp-read` + `qp-watch` kaydıyla
  kurar") bu ölçüde sapılır. Kayıt şeridi (`tape`) de izleyicileri `FOLLOWER` ile kurar
  (ürün yoluyla aynı; audit-s32 B2). Arkin itiraz ederse geri alınır (o zaman WASM kapısı
  yeniden konuşulur).
- **İ8 (RF1), kabuk WASM paketi (Arkin'in kararı, 2026-11-25: "WASM istisnası kabul").**
  Round oturumu S3.3'te durup sordu; seçenekler: (1) istisna; (2) yansı ve görüntüleyici
  çekirdekte kalır (çekirdek 15.449, 13.000 tavanı tutmaz); (3) bir tur daha küçültme.
  Ölçüm (S3.3 sonu, `--features replica`, `--no-opt`): kabuk paketi ham 9.004.524 B (+%2,45;
  taban 8.789.414), gzip 2.485.151 B (+%2,17; taban 2.432.388); varsayılan `qp_wasm.wasm`
  +%0,39 (±%2 içinde). Neden: taşınan kodun kendisi değil (`qp_authority` −35,7 KB, `qp_replica`
  +27,6 KB, `qp_viewer` +10,7 KB), `--no-opt` pakette crate'ler arası jenerik örnek kopyaları
  (`core`/`alloc` BTreeMap arama ve itme, `drop_in_place`, `clone`, serde ziyaretçileri, bir
  kararlı sıralama; isim bölümü +17,5 KB). Tavan: kabuk paketi en çok ham +%2,5 / gzip +%2,5
  (RG1 İ7 emsali, K85); varsayılan paket ±%2 içinde kalır. Kalıcı çözüm önerisi: kabuk
  paketinde `wasm-opt` ya da derleme ayarı (ayrı round; `REPORT.md` Öneriler).
- **Güncelleme (S3.3, kapanış denetimi notu).** S3.3'ten sonra `FOLLOWER` = `STANDARD` eksi
  okuma eksi görüntüleyici (izleyicilerin görüntüleyici bağlantısı yok; `audit-s33`). S4
  notundaki `qp-ops` `lint.allow` 21 sonraki dalgalarda 20'ye sıkıştı (taban çizgisi).
