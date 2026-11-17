# R5 — Alt Görevler

| # | Görev | Yazdığı yer | Bağımlılık |
|---|-------|-------------|------------|
| T0 | Hazırlık: ana akış değişikliklerini kaydet; `scripts/clean.sh` (K56) ve ortak `CARGO_TARGET_DIR` düzeni; eski worktree'lerin temizliği (`git worktree prune`, kalanların listesi); `cpu` yolunun kaldırılması; ölçüm ekranı bölme hatası | `scripts/`, `shells/web/`, `crates/qp-wasm/` | — |
| T1 | Araştırma: yeni nesil arayüz, tek bakış okunabilirliği, anlam kodlama, erişilebilirlik, hareket; güncel kaynaklardan | `rounds/R5/RESEARCH.md` | — |
| T2 | Ölçüm düzeneği: tek bakış okuması, kısma sınaması, ikili kör karşılaştırma ve sıralama, hesapla ölçülenler (karşıtlık, ΔE, yoğunluk, yasak ifade taraması), bozuk tasarım sınaması, K52 bütçe ölçümü. **Alternatiflerden önce commit** | `rounds/R5/experiments/harness/` | T1 (ölçüt girdisi) |
| T3 | Tasarım kayıtlarının genişletilmesi: yazı ailesi ve ölçek, renk rolleri, boşluk, yüzey, hareket süreleri çekirdekte değiştirilebilir kayıtlar; `?look=` ile seçim; `v1` görünümü aynen üretebilme | `crates/qp-schema/`, `crates/qp-grammar/`, `crates/qp-render/` | — |
| M17a–c | Kapsül dili: üç cümle yapısı, ayrı alt ajanlarla; sözlük | dal başına; `crates/qp-capsule/` (şablonlar), `docs/LANGUAGE.md` | T2 |
| M16a–d | Görsel dil: dört yön, ayrı alt ajanlarla, birbirini görmeden; biri bilinçli olarak alışılmışın dışı | dal başına; tasarım kayıtları, yazı tipi dosyaları ve lisansları | T2, T3; M17 seçimi (aynı metinle yarışsınlar) |
| M18a–c | Hikâye çizelgesi: üç tasarım; protokol eki (enstrüman hikâye listesi sorgusu) ortak ve önce | `crates/qp-session/`, `crates/qp-grammar/`, `docs/PROTOCOL.md`, `docs/GRAMMAR.md` | T2, T3 |
| M19 | Akış önceliği ve gizleme eşiği: geriye dönük benzetim; sıralama | `crates/qp-capsule/`, `crates/qp-session/`, `rounds/R5/experiments/M19/` | T2 |
| M20a–b | Açılmış kapsül ve kanıt ekranı; yetkisiz yüz (A32) | dal başına | M16 ve M17 seçimi |
| M21a–b | Hareket | dal başına | M16 seçimi |
| D | Yoğunluk sınaması: seçilen dil altı görünümde | `rounds/R5/experiments/D/` | M16 seçimi |
| G | Galeri sayfası ve gösterimde aday seçimi | `shells/web/`, `crates/qp-cli/` | finalistler |
| S | Bütünleştirme: seçilenler birlikte; K52 ölçümü; `v2` altınlar; ekran testleri | bütünleştirme dalı | hepsi |
| H | İnsan testi v3 paketi | `rounds/R5/human-test/` | S |
| A | Denetim: bağımsız alt ajan. Düzeneğin alternatiflerden önce yazıldığını commit sırasından doğrular; ölçümleri yeniden üretir; karşıtlık ve yasak ifade taramasını kendi koduyla tekrarlar; lisans girişimsizliğini sınar; yazı tipi lisanslarını denetler | `rounds/R5/AUDIT.md` | hepsi |

## Notlar

- **Sıra:** dil (M17) görsel dilden (M16) önce seçilir; görsel yönler aynı metinle
  yarışır. Aksi halde iyi cümle kötü tasarımı, kötü cümle iyi tasarımı gölgeler.
- **Alternatif üreten ajanlara verilecek tarif** yalnızca sorun, sınırlar ve değişmezlerdir;
  bugünkü tasarımın ekran görüntüsü "başvuru" olarak verilir, "iyileştir" denmez. Bir yön
  bilinçli olarak sıfırdan düşünür (bugünkü tasarımı görmez).
- **Alışılmışın dışı yön** için serbestlik: kart olmak zorunda değil; liste olmak
  zorunda değil. Kapsülün yaşı, gücü, güveni nasıl hissedilir? Zaman nasıl görünür?
  İşaret nasıl yaşar? Ölçülebildiği sürece her şey denenebilir.
- **Yazı tipi:** aday aileler güncel belgelerden lisansıyla doğrulanır; değişken yazı
  tipi tek dosyada birden çok ağırlık verir ama boyutu ölçülür. Çekirdek çizici (fontdue)
  ve `canvas` yolu aynı dosyayı kullanır; iki yol arasındaki metin farkı ölçülür.
- **Hikâye çizelgesi verisi:** cihaz listeyi sunucudan ister (K48); liste kapsülün
  kimliği, türü, doğum ve kapanış barı, sonucu ve yüz özetidir; bar taşımaz (K23).
  Yetkisiz okuyucuya lisanslı kapsülün yalnızca rakamsız biçimi gider (K33).
- **Gizleme eşiği (K53):** eşik karnenin o andaki haline bakar (geleceğe bakmaz); örnek
  sayısı azsa "yazı tura" denmez, "az örnekli" denir. İki durumun sınırı ölçümle seçilir.
  Elindeki hissede olumsuz geçiş (K42) gizlemeden muaf mı: iki biçimi de ölç, öneri yaz;
  karar Arkin'in.
- **Altınlar:** `v1` altınlar `?look=r4` (ya da eşdeğeri) ile bayt eşit üretilmeye devam
  eder; bu, tasarım kayıtlarının doğru soyutlandığının kanıtıdır.
- **Disk (K56):** her alt ajan ortak `CARGO_TARGET_DIR` ile çalışır; elenen alternatifin
  worktree'si ölçüm biter bitmez kaldırılır. Ekran görüntüsü yığınları depo dışında
  tutulur; kanıt olarak yalnız özet tablolar ve seçilmiş küçük görseller girer.
- **Gerçek veri:** `fixtures/local/` varsa M19 benzetiminde kullanılır; görseller ve
  türevleri depoya girmez; rapora yalnız toplu istatistik. Galeri sentetik veriyle çalışır.
- **GitHub ve ana akış:** `CLAUDE.md` kuralları geçerli.
