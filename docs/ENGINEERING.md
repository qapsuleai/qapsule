# Geliştirme Kuralları — yapı, kod, parametre ve profil yönetimi

Durum: **kabul (K80, 2026-11-22).** Uygulaması RG1 (yönetişim turu) ile başlar; o tura kadar yeni kod için bağlayıcı, mevcut kod için borç listesi (§11).

## 0. Neden (ölçülmüş durum, 2026-11-21)

Qapsule bir haftada ≈ 130 bin satır Rust'a ulaştı. Round'lar yalnız davranışı ve ölçümü
denetledi; yapıyı denetleyen kural yoktu:

| Gözlem | Değer |
|---|---|
| En büyük dosyalar | `qp-schema/protocol.rs` 4.916 satır; `qp-session/session.rs` 3.713; `qp-session/capsule.rs` 3.425; `qp-wasm/shell.rs` 3.378; `qp-cli/livehost.rs` 3.067 |
| En büyük crate | `qp-session` 32 bin satır, tek düz klasörde 37 dosya; testleri tek klasörde 40 dosya |
| Kabuk "ince" değil | `qp-cli` 14 bin, `qp-wasm` 11 bin satır (kural: kabuklar incedir) |
| Tek dosyalık karar kaydı | `DECISIONS.md` 90 KB |

Kural konmazsa her round bu eğilimi büyütür; dışarıdan uyarlanan yapılar (OpenChart
kalıpları gibi) da aynı düzensizliğin içine düşer.

## 1. İlkeler

1. **Yapı da kabul ölçütüdür.** Bir round, davranış testleri yeşil olsa bile yapı bütçesini
   bozuyorsa bitmiş sayılmaz.
2. **Önce yer aç.** Bir değişiklik dosyayı ya da modülü bütçenin üstüne itecekse önce bölünür,
   sonra eklenir.
3. **Kural makineyle denetlenir.** Bu belgedeki her sayısal kural bir betikle ölçülür
   (`scripts/arch-check`), `verify.sh`'ın parçasıdır. Denetlenemeyen kural öneridir.
4. **Borç görünürdür ve büyümez.** Bugün bütçeyi aşan her şey adıyla bir borç listesindedir;
   listedeki sayı yalnız azalabilir (mandal).
5. **Tek doğruluk kaynağı.** Şema, parametre, hata kodu, kavram adı tek yerde tanımlanır;
   gerisi üretilir.

## 2. Katmanlar ve bağımlılık yönü

Bağımlılık yalnız aşağı doğrudur; yukarı ya da yana bağımlılık denetimde hata verir.

```
kabuklar        qp-cli, qp-wasm, shells/*            (yalnız bağlama; alan mantığı yok)
uyarlayıcılar   qp-mcp, qp-live, qp-matriks, qp-source (dış dünya: protokol, ağ, dosya)
oturum          qp-session                           (otorite, komut yolu, günlük)
alan            qp-capsule, qp-grammar, qp-facts, (yeni) hesap dili, sicil, ajan sözleşmesi
çekirdek        qp-compute, qp-calendar, qp-render
şema            qp-schema                            (tipler; hiçbir şeye bağımlı değil)
```

- Kabuk kuralı: kabuk crate'inde alan kararı veren kod bulunmaz. Kabuk büyüyorsa mantık
  yanlış katmandadır. Bütçe: kabuk crate'i en çok 4 bin satır.
- Uyarlayıcı kuralı: dış biçim (proto, MCP, REST) alan tiplerine uyarlayıcıda çevrilir;
  alan katmanı dış biçimi görmez.
- Hesap kodunda model, ağ, saat, rastgelelik yoktur (mevcut değişmez); saat ve rastgelelik
  gerektiğinde dışarıdan verilir.

## 3. Boyut bütçeleri

| Ölçü | Bütçe | Aşılırsa |
|---|---|---|
| Dosya (test hariç) | 600 satır; kesin sınır 800 | Konuya göre alt modüle böl |
| Fonksiyon | 80 satır | Adlandırılmış adımlara böl |
| Bir klasördeki kaynak dosya | 15 | Konu klasörlerine ayır |
| Crate | 12 bin satır | Alan sınırından ikiye böl |
| Kabuk crate'i | 4 bin satır | Mantığı aşağı katmana taşı |
| Test dosyası | 800 satır | Konuya göre böl; testler de konu klasörlerinde |
| Belge dosyası | 1.500 satır ya da 60 KB | Konuya göre böl, dizin dosyası bırak |
| Fonksiyon parametresi | 7 (mevcut clippy eşiği) | Yapı tipiyle grupla |

Üretilmiş dosyalar (proto çıktısı, üretilmiş şema) ve veri tabloları bütçe dışıdır; başlarında
"üretilmiştir" yazar.

## 4. Modül düzeni

- **Konuya göre klasör, türe göre değil.** `models/ utils/ helpers/` yok; `capsule/story/`,
  `session/proposal/`, `watch/match/` var.
- **Bir kavram, bir modül.** Kavram adları `docs/CONCEPTS.md`'den; yeni kavram önce sözlüğe.
- `lib.rs` ve `mod.rs` yalnız modül bildirir ve dışa açar; mantık taşımaz.
- Her crate'in kökünde kısa `README.md`: ne işe yarar, hangi katman, nelere bağımlı, dışa
  açtığı tipler, ne **değildir**.
- Dışa açık yüzey bilinçlidir: `pub` varsayılan değildir; crate dışına açılan her tip
  `lib.rs`'te listelenir.
- "Çeşitli" çöplüğü yok: `util.rs`, `common.rs`, `misc.rs` adlı dosya açılmaz.

## 5. Kod kuralları (Rust)

- `unsafe` yasak (mevcut). Kütüphane kodunda `unwrap`, `expect`, `panic!`, `todo!`,
  `unimplemented!` yasak (test hariç); hata tipli ve kodludur (`ErrorCode`).
- Hata iletisi makine okunur: kod + yer + beklenen + "bunu mu demek istedin"; ajan da insan da
  aynı hatayı okur.
- Gizli küresel durum yok; durum sahibinden parametreyle geçer.
- Sihirli sayı yok: davranışı değiştiren her sayı ya adlandırılmış sabittir (değişmezse) ya da
  parametre kayıt defterindedir (§6).
- Dışa açık her tip ve fonksiyonun bir satırlık belgesi vardır.
- Yeni bağımlılık (crate) eklemek karar satırı ister: neden, lisansı, alternatifi.
- Yorumlar ne yaptığını değil **neden**ini söyler; karar numarasına (K…) başvurur.
- Biçim ve lint: `rustfmt`, `clippy` uyarısız; ek olarak `too_many_lines`,
  `cognitive_complexity`, `unwrap_used`, `expect_used`, `panic` lint'leri açılır.

## 6. Parametre yönetimi

**Kural: davranışı, sınırı ya da varsayılanı belirleyen her değer tek bir parametre kayıt
defterinde tanımlıdır.** Kodun içine dağılmış varsayılan olmaz.

Her parametrenin tanımı:

| Alan | Örnek |
|---|---|
| Anahtar | `watch.hold_bars` |
| Tip, birim, aralık | tamsayı, bar, 1–50 |
| Varsayılan | 3 |
| Kapsam | sistem / plan / kullanıcı / enstrüman / nesne (hangi düzeylerde değiştirilebilir) |
| Kim değiştirir | kullanıcı (profilden), yönetici, yalnız sürüm |
| Hak mı? | satılabilir kullanım hakkı mı (soru adedi, yapay zekâ kullanımı, kontör, hesap bütçesi) |
| Açıklama, eklendiği sürüm | kullanıcıya gösterilecek metin |

- **Çözümleme sırası:** nesne > kullanıcı > plan > sistem. Etkin değer her zaman kaynağıyla
  birlikte sorulabilir ("neden 3?" → "sistem varsayılanı").
- **Türler:** davranış varsayılanları (tutunma barı, kırılımda kapanış / dokunmada fitil,
  ara periyot seçimi), sınırlar ve haklar (günlük soru, model kullanımı, etkin görev sayısı,
  hesap maliyeti üst sınırı), bildirim süzgeci, gösterim.
- Kayıt defterinin kaynağı `qp-schema`'daki Rust tipleridir; belge ve arayüz ondan üretilir.
- Parametre değişikliği günlüğe yazılır (kim, ne zaman, eski → yeni). Parametre kaldırmak ya
  da anlamını değiştirmek sürüm işidir.
- Ajan parametre okuyabilir; kullanıcının parametresini ancak öneriyle değiştirir.
- Program parametreleri (X30/9): hesap programının girdileri (dönem, eşik, izlemede çizim
  seviyesi) aynı tanım biçimini kullanır; tarama, yeniden kurma ve paylaşım bu tanımdan çalışır.

## 7. Profil yönetimi

Profil tek bir "kullanıcı ayarları" torbası değildir; ayrı sorumlulukları olan bölümlerdir ve
karıştırılmaz:

| Bölüm | İçerik | Kural |
|---|---|---|
| Kimlik | hesap, dil, bölge, saat dilimi | bölgeye özgü davranış buradan parametreye bağlanır, koda gömülmez (X11) |
| Plan ve haklar | üyelik, ek paketler, kalan kullanım (kontör) | her model çağrısı ve ücretli hesap bir kullanım kaydına yazılır; kalan hak sorgulanabilir |
| Veri lisansları | hangi veriyi görmeye yetkili (K33) | gösterim kararı buradan; ajan da aynı yetkiyle okur |
| Tercihler | kullanıcı kapsamındaki parametreler (§6) | yalnız kayıt defterinde tanımlı anahtarlar |
| Kazanılmış varsayılanlar | "bu kalıpta artık sormuyorum" sayaçları (X10) | tercihlerden ayrı; görünür, tek tek sıfırlanır; deterministik sayaç, model belleği değil |
| Rızalar | toplu istatistiğe katılım, paylaşım (X28) | açık, geri alınabilir; varsayılan kapalı |
| Kişisel nesneler | çizimler, izlemeler, üyelikler, kendi sicili | pozisyon bilgisi hiçbir zaman paylaşılmaz |

- Profilin her bölümü şemalıdır; dışa aktarılabilir ve silinebilir.
- Profil bilgisi modele ancak görevin gerektirdiği kadar verilir.
- Kullanım kaydı ürün ölçümünün de kaynağıdır (hangi iş, hangi model boyu, kaç belirteç).

## 8. Test ve doğrulama

- Birim testi kodun yanında; bütünleşik test konu klasöründe.
- Belirlenimcilik, toplu = akışkan, geleceğe bakmama, köken gibi değişmezler özellik
  testleriyle; her yeni çekirdek bu kalıbı taşır.
- Test verisi küçük ve sentetik; gerçek veri depoya girmez (mevcut kural).
- Ölçüm düzenekleri alternatiflerden önce yazılır ve değişmez (mevcut kural, K39).

## 9. Dışarıdan uyarlama (OpenChart ve benzerleri → Rust / WASM)

- **Temiz oda:** kod kopyalanmaz, kaynak yapının klasör düzeni ve adları da aynalanmaz.
  Davranış kendi şartnamemize yazılır, sonra bu belgenin katman ve bütçe kurallarına göre
  kurulur.
- Her uyarlama için bir **uyarlama kartı**: kaynak (proje, dosya), lisans hükmü, aldığımız
  şey (fikir / kalıp), bizim tasarımımız, hangi katmana oturduğu, hangi testle sınandığı.
- Uyarlanan kalıp var olan bir kavramla örtüşüyorsa yeni kavram açılmaz; var olan genişletilir.
- Testler kaynak projenin testlerinden çevrilmez; davranış şartnamesinden yazılır.

## 10. Round disiplini

- Her round'un `GOAL.md`'sinde bir **yapı ölçütü** bulunur: `arch-check` yeşil; borç listesi
  büyümedi; yeni crate/modül README'li; yeni parametreler kayıt defterinde.
- Bağımsız denetçi yapı kurallarını da denetler (yalnız davranışı değil).
- Borç eşiği aşılırsa araya **düzenleme round'u** (RF) girer: davranış değişmez, altınlar ve
  özetler bayt eşit kalır, yalnız yapı düzelir.
- Bu klasördeki (`refs/experiments/`) deney kodu atılabilir koddur: bütçeler bağlamaz, ama
  her deneyin `SPEC.md`/`README.md`'si olur ve **deney kodu Qapsule'e kopyalanmaz**; Qapsule
  round'u onu şartname olarak alır ve bu kurallarla yeniden kurar.

## 11. Bugünkü borç (öneri: R5B'den sonra RF1)

| Borç | Bugün | Hedef |
|---|---|---|
| `qp-schema/protocol.rs` | 4.916 satır | mesaj ailesine göre modüller |
| `qp-session` | 32 bin satır, düz 37 dosya | konu klasörleri; gerekirse iki crate |
| `qp-session/session.rs`, `capsule.rs`, `expand.rs`, `select.rs` | 1,9–3,7 bin | ≤ 800 |
| `qp-cli` (`livehost.rs`, `live.rs`, `demo.rs`) | 14 bin | canlı barındırma mantığı uyarlayıcı katmanına; kabuk ≤ 4 bin |
| `qp-wasm/shell.rs` | 3.378 | kabuk mantığı alan katmanına |
| `qp-capsule/voice/*`, `qp-grammar/look.rs`, `layer.rs` | 1,7–2,2 bin | veri tabloları koddan ayrılır |
| `DECISIONS.md` | 90 KB | alan dosyaları + dizin |
| Parametre kayıt defteri | yok | §6 |
| `arch-check` | yok (yalnız `shells/web/arch-check.mjs`) | depo geneli |
