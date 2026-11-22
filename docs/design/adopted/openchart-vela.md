# Uyarlama kartları: OpenChart ve Vela (RD2, K83)

İnceleyen: RD2 araştırma alt ajanı, yalnız okuma · Tarih: 2026-11-22
Yollar: `oc/` = `/Users/arkns/Works/Mchart/refs/src/longsurf-ai__openchart/` (commit `01b0ad3`),
`vela/` = `/Users/arkns/Works/Mchart/refs/src/LuxAlgo__Vela/` (commit `3d2179bd`).
Lisans: OpenChart MIT (`refs/reports/LICENSES.md:15`). Vela Apache-2.0 + NOTICE (`LICENSES.md:30`).
Kod alınırsa NOTICE ekran atfı şartı devreye girer, kalıp alınırsa girmez. Pine paketi
(`@luxalgo/vela-pinets`, AGPL) depoda yok, dokunulmadı.
Temiz oda: kaynak kod, CSS ve değer tablosu yok. Kalıp, ölçü mantığı ve rol adları var. Tekil
değerler yalnız kanıt olarak geçiyor. `src/` altındaki `CLAUDE.md`/`AGENTS.md` dosyaları veri
sayıldı, hiçbir şey çalıştırılmadı.
Ana kaynak: `refs/reports/VISUAL.md`. Bu dosya onu tekrar etmiyor, kart biçimine çeviriyor ve
koddan ek kanıt ekliyor.

---

## A. Sözleşme, renk, ölçek

### OV-1. Tasarım sözleşmesinin biçimi
- **Kalıp:** Kısa, kural odaklı bir sözleşme belgesi (≈70 satır). Değerleri listelemiyor;
  değerin nerede yaşadığını, hangi ortak bileşenin sahip olduğunu ve etkileşim kurallarını
  yazıyor ("final answers outside collapsed work", "toolbars float without reserving
  space", "toasts 8 px below 60 px header"). Ayrıntılı geometriyi sahibi olan bileşenin
  belgesine bırakıyor ("This file records shared design rules, not a feature inventory").
  Vela'da karşılığı bileşen başına tek dosya kuralı ve ayrı bir tema belgesi.
- **Kaynak:** `oc/app/DESIGN.md` (bütünü); alt alan kuralları `oc/app/src/features/*/AGENTS.md`,
  `oc/common/chart-core/src/annotation/INVARIANTS.md`. Vela: `vela/docs/contributing/adding-a-ui-component.md`,
  `vela/packages/vela-ui/docs/theming.md`, `vela/docs/architecture/settings-rows.md`.
- **Lisans hükmü:** MIT / Apache: kalıp uyarlanır.
- **Neden iyi çalışıyor:** Kısa olduğu için her ajan okuyor. Kural "neden"i değil "ne
  yapılır"ı söylüyor, bu yüzden sınanabiliyor. Değer tablosu olmadığı için eskimiyor.
- **Qapsule'de yeri:** Belge: `docs/design/CONTRACT.md` ve `.claude/skills/qapsule-design/SKILL.md`.
  Değerler: çekirdekte görünüş kaydı (`look.rs`).
- **Bizde ne değişiyor:** OpenChart'ta "CSS owns visual values" diyor. Bizde "görünüş kaydı
  değerin sahibi, CSS yalnız `css_vars` çıktısı". Sözleşme rol adlarını ve kuralları taşır,
  sayıyı taşımaz. R5'teki 648 satırlık ölçüm odaklı DESIGN.md ikiye ayrılır: kural (sözleşme)
  ve ölçüm raporu (ayrı belge).
- **Sınama:** Sözleşmede sayısal piksel/renk değeri yok (regex denetimi). Her kural bir
  `design-check` maddesine ya da test adına bağlı. Uzunluk sınırı (ör. ≤ 150 satır).

### OV-2. Rol tabanlı renk: nötr krom, kroma yalnız veride
- **Kalıp:** Bütün yüzey, metin, kenar ve birincil eylem rolleri renksiz (oklch kroma 0).
  Kroma yalnız up/down, gösterge paleti, seans bandı, uyarı şiddeti ve ajan açıklaması
  rollerinde. Kenar tek ton (beyaz, düşük alfa) ve dört alfa basamağı. Vela aynı ayrımı rol
  adlarıyla yapıyor: yüzey basamakları (bg, surface, elev, sunken, overlay), metin
  basamakları (fg, muted, faint, bright), kenar basamakları, ayrı up/down/profit/loss.
  Temalar mum rengini paylaşıyor: tema değişince seri renk değiştirmiyor.
- **Kaynak:** `oc/app/src/styles/themes.css` (rol adları), `oc/app/DESIGN.md:27`; görsel
  `oc/docs/assets/charts-ui/01-single-chart.png` (≈4 ton). Vela: `vela/packages/vela-ui/src/lib/tokens.ts`
  (rol adları), `vela/docs/user/workspace.md` "Theming", `vela/.github/vela-layout.png`.
- **Lisans hükmü:** kalıp uyarlanır.
- **Neden iyi çalışıyor:** Göz önce renkli olana gider. Renk yalnız veride olunca hiyerarşi
  kendiliğinden kurulur ve ajan paneli "sessiz" kalır (VISUAL §2.7, §3.2 madde 2).
- **Qapsule'de yeri:** Çekirdek: görünüş kaydı rolleri (Theme). Kabuk yalnız `css_vars` okur.
- **Bizde ne değişiyor:** R5'teki dolu not/uyarı/vurgu kromu krom rolünden çıkar, anlam
  rolüne iner. up/down neon değerlerden orta doygunluğa çekilir. Vela'nın "tema
  değiştirmek seriyi yeniden renklendirmez" kuralı alınır: up/down tema bağımsız rol.
- **Sınama:** VISUAL §5 P1 (krom rollerinde C ≤ 0,02), P2 (sahne başına ayrı ton ≤ 6),
  P3 (up/down S ve L aralığı). Görünüş kaydı üzerinde birim test.

### OV-3. Ajan yazımını insandan ayıran renk rolü
- **Kalıp:** Ajanın grafiğe yazdığı açıklamalar ayrı tema rollerinden boyanır: dolgu ve
  metin "AI annotation" rollerinden, öncü çizgi, kenar ve hedef noktası duyguya
  (pozitif/negatif/nötr) bağlı ayrı vurgu rollerinden gelir, mum up/down renginden değil.
  Ajan kökenli kayıtlarda kullanıcının sakladığı stil yok sayılır. Ayrıca bir "AI degrade"
  rolü (iki durak) yalnız ajan eylemlerini işaretler: çizim araç çubuğundaki "Chart
  explain" düğmesi bu degradeyle ayrılıyor.
- **Kaynak:** `oc/common/chart-core/src/annotation/INVARIANTS.md` ("Agent-backed annotations
  … theme-owned AI annotation fill/text tokens … not candle --up/--down"), `oc/app/src/styles/themes.css`
  (`--chart-agent-annotation-*`, `--ai-gradient-*` rol adları), `oc/app/src/features/chart/components/drawing-toolbar.tsx`
  (başlık yorumu "Emphasize Chart explain with the theme's AI gradient"). Görsel:
  `oc/app/src/app/trellis/workflows/starter/films/captures/A01.png` (araç çubuğunda mor-şeftali
  ışıltı simgesi), `oc/docs/assets/dashboard-preview.png` (BTCUSDT bölmesindeki haplar).
  Vela: **yok** (köken alanı yok, `refs/reports/repos/vela.md`).
- **Lisans hükmü:** kalıp uyarlanır.
- **Neden iyi çalışıyor:** Ajan işareti, insan çizimi ve veri üç ayrı renk ailesinde
  kalıyor. Kullanıcı stil verse bile ajan kökeni maskelenemiyor.
- **Qapsule'de yeri:** Çekirdek: görünüş kaydında `origin` başına rol ailesi; gramer
  katmanı işareti `origin`'e göre rolden boyar.
- **Bizde ne değişiyor:** OpenChart'ta ajan işareti bir çalışma anı kimlik listesiyle
  belirleniyor, kayıtta köken yok. Bizde `origin` (human / agent / engine) şemada zorunlu
  (`no_origin`), rol seçimi doğrudan ondan. Üç köken için üç rol ailesi gerekir; motor
  (deterministik hesap) kökeni OpenChart'ta yok. Degrade yerine tek nötr-üstü ton
  düşünülebilir: degrade "pazarlama" tadı verebilir, P2 ton bütçesini yer.
- **Sınama:** SceneDesc'te her işaretin rengi kendi `origin` rol ailesinden (birim test).
  Ajan kökenli işarette kullanıcı stili uygulanmıyor. Renk körlüğü simülasyonunda köken
  ayrımı şekil/rozetle de korunuyor.

### OV-4. Tip ve boşluk ölçeği
- **Kalıp:** Tek nötr grotesk aile (arayüz ve tuval aynı). Taban 16, gövde 14, grafik 12 px;
  en küçük adım 11. Satır yüksekliği ve harf aralığı ölçek adımına bağlı. Sayısal metinde
  tablo rakamı. 4 px ızgara, denetim boyları 36/32/24, taban yarıçap 10 (düğme 8).
  Lejantta üç ağırlık/ton basamağı aynı satırda: kalın sembol, soluk alan harfi, parlak
  rakam. Vela: üç font boyu rolü (sm/md/lg), yarıçapı tek seçimle ölçekleyen tema
  (`radius: none|sm|md|lg`), denetim boyu iki boy (md/sm).
- **Kaynak:** `oc/app/src/styles/globals.css` (ölçek), `oc/app/DESIGN.md:27-29`,
  VISUAL §1.3, §1.5. Vela: `vela/packages/vela-ui/docs/theming.md`,
  `vela/docs/contributing/adding-a-ui-component.md` (Select md/sm, 100 px ayar sütunu).
- **Lisans hükmü:** kalıp uyarlanır.
- **Neden iyi çalışıyor:** Küçük ve sıkı ölçek profesyonel yoğunluk verir. Ton/ağırlık
  basamağı renge başvurmadan hiyerarşi kurar.
- **Qapsule'de yeri:** Çekirdek: görünüş kaydı (tip ölçeği, `tnum` eşlemesi zaten var).
- **Bizde ne değişiyor:** R5'in 13 px taban ve 44 px dokunma hedefi masaüstü profilinde
  gevşer; telefon ayrı yoğunluk profili olur (VISUAL §5 not). Vela'nın "tek seçim bütün
  yarıçapları ölçekler" fikri görünüş kaydında türetilmiş değer olarak alınabilir.
- **Sınama:** VISUAL §5 P10 (masaüstü gövde 13–14, grafik 11–12), P11 (tabular), P12
  (yarıçap). `css_vars` dökümü üzerinde profil başına denetim.

---

## B. Grafik hücresi (eksen, lejant, imleç)

### OV-5. Sağ fiyat ekseni ve tick yoğunluğu
- **Kalıp:** Fiyat etiketleri sağdaki ayrılmış olukta, veriyle çakışmaz. Tick aralığı
  piksel hedefli (≈ her 50 px, en az 3, en çok 8). Vela: yoğunluk genişliğe uyarlanır (dar
  hücre daha az tick ister), taban 3; üstüne ölçülmüş çakışma geçişi: komşu etiketler
  arasında en az boşluk (≈12 px) yoksa etiket atlanır.
- **Kaynak:** `oc/common/chart-core/src/util/constants.ts:11-17`; görsel `01-single-chart.png`.
  Vela: `vela/src/renderers/native/chrome/ChromeRenderer.ts:430-460` (yorumlar).
- **Lisans hükmü:** kalıp uyarlanır.
- **Neden iyi çalışıyor:** Okuyucu fiyatı her yükseklikte okuyabilir. Sabit piksel aralığı
  hücre boyundan bağımsız tutarlı yoğunluk verir.
- **Qapsule'de yeri:** Çekirdek: gramer (`compile.rs`, bugün ≈90 px aralık).
- **Bizde ne değişiyor:** Aralık ve oluk genişliği görünüş kaydından gelir. Çakışma geçişi
  deterministik ve SceneDesc'e yazılmadan önce yapılır, kabuk düzeltmez.
- **Sınama:** P4 (40–60 px aralık, 300 px'te ≥ 4 etiket), P6 (etiket kutusu kesişimi 0).

### OV-6. İki düzeyli zaman ekseni
- **Kalıp:** Küçük birim (gün) normal ağırlık, büyük birim (ay, yıl) kalın ve aynı satırda.
  Vela: çakışmada büyük birim önce yerleşir ("a date beats the 12:00 beside it").
- **Kaynak:** Görsel `oc/docs/assets/charts-ui/01-single-chart.png` (alt eksen "Apr", "May"
  kalın); `oc/common/chart-core/src/scale/axis.ts`. Vela: `ChromeRenderer.ts:455-457`
  (yorum), `vela/.github/vela-layout.png` ("Sep 1", "12:00").
- **Lisans hükmü:** kalıp uyarlanır.
- **Neden iyi çalışıyor:** ISO tarih yerine bağlamı tek bakışta verir, yer kazandırır.
- **Qapsule'de yeri:** Çekirdek: gramer (etiket seçimi ve önceliği), görünüş (ağırlık rolü).
- **Bizde ne değişiyor:** Biçim yerele bağlı (TR/EN ay adları çekirdekte). Öncelik
  sıralaması açık bir kural olarak gramerde.
- **Sınama:** P5 (iki düzey var, 390 px'te ≥ 4 etiket). Büyük birim etiketi atlanmıyor.

### OV-7. OHLC lejantı (durum satırı)
- **Kalıp:** Sol üstte tek satır: sembol (kalın), kaynak (soluk), O/H/L/C/V (soluk harf,
  parlak rakam), değişim ve yüzde (yön renginde). İmleçteki barı gösterir, imleç yokken son
  bara döner. Altında gösterge satırları aynı biçimde. Vela: dar hücrede satır kırılmaz,
  parçalar öncelik sırasıyla gizlenir (önce değişim, sonra kaynak/zaman dilimi, logo ve
  sembol hep kalır); sağ tıkla parça başına aç/kapa.
- **Kaynak:** `oc/app/src/features/chart/components/series-legend.tsx`, `legend.tsx`;
  görsel `01-single-chart.png`. Vela: `vela/docs/user/workspace.md` "The chrome / Status line";
  görsel `vela-layout.png`.
- **Lisans hükmü:** kalıp uyarlanır.
- **Neden iyi çalışıyor:** İşlemcinin beklediği sözleşme. Tam rakam güven verir.
- **Qapsule'de yeri:** Metin çekirdekte üretilir, kabuk DOM'da dizer (VISUAL §4.3).
- **Bizde ne değişiyor:** Gizleme önceliği çekirdekte bir liste olur (Vela'nın sırası iyi
  bir başlangıç). Kabukta metin kurma yasağı korunur.
- **Sınama:** P7, P9 (lejantta ≥ 5 sayı). Dar genişlikte satır kırılmıyor, sembol hep var.
- **Zayıflık (OpenChart):** `01-single-chart.png` ve `A01.png`'de yüzen widget araç çubuğu
  lejantın O/H/L/C kısmını örtüyor. Bizde araç çubuğu lejantla çakışmamalı (P6'ya ekle).

### OV-8. Son değer etiketi, değer çizgisi ve geri sayım
- **Kalıp:** Sağ olukta seri renginde dolu etiket: sembol adı ve son fiyat. Etiketten sola
  1 px noktalı değer çizgisi. Hacim bölmesinin de kendi son değer etiketi var ("1.43K").
  Vela: etiketin altında bar kapanışına geri sayım (ör. "05:54"), cihaz saatinin canlı
  mumlarla düzeltilmiş hâlini okur, tekrar oynatmada gizlenir; etiketi kapatılabilir.
- **Kaynak:** `oc/common/chart-core/src/series/config.ts`; görseller `01-single-chart.png`,
  `A01.png`. Vela: `vela/docs/user/api-reference.md:309,654` (`setWallClock`),
  `vela/docs/user/renderer-features.md:96`; görsel `vela-layout.png`.
- **Lisans hükmü:** kalıp uyarlanır.
- **Neden iyi çalışıyor:** Şimdiki fiyat her an tek yerde. Geri sayım canlı veriyi
  hissettirir ve barın kesin olmadığını gösterir.
- **Qapsule'de yeri:** Çekirdek: SceneDesc işaretleri (etiket, değer çizgisi).
- **Bizde ne değişiyor:** Geri sayım çekirdekte saat okuyamaz (hesapta rastgelelik ve ağ
  yok, aynı girdi aynı çıktı). Saat kabuktan girdi olarak verilir; çekirdek "bar kapanışına
  kalan" değerini bu girdiden deterministik üretir. Kesin olmayan bar (Qapsule'ün kesin bar
  hijyeni) geri sayımla birlikte gösterilebilir.
- **Sınama:** P7. Aynı saat girdisiyle aynı SceneDesc (altın test). Tekrar oynatmada geri
  sayım yok.

### OV-9. İmleç ve eksen hapları
- **Kalıp:** Kesikli ince imleç çizgisi; alt eksende tarih hapı, sağ eksende fiyat hapı
  (nötr mavi-gri dolgu). Vela: çoklu hücrede "hayalet imleç": diğer hücrelerde soluk dikey
  çizgi kendi barına oturur, zaman hapıyla.
- **Kaynak:** `oc/common/chart-core/src/render/draw.ts` (imleç zaman etiketi); görsel
  `01-single-chart.png` (`2026-07-03` hapı, üstte `2,925.95` hapı). Vela:
  `vela/docs/user/workspace.md` "Sync links" (crosshair), `chrome/CrosshairRenderer.ts`.
- **Lisans hükmü:** kalıp uyarlanır.
- **Neden iyi çalışıyor:** Okuma eksende, veriyi örtmüyor. Hayalet imleç çoklu grafikte
  zaman karşılaştırmasını kolaylaştırır.
- **Qapsule'de yeri:** Çekirdek: SceneDesc işaretleri (imleç katmanı).
- **Bizde ne değişiyor:** R5'teki düz açık mavi "seçili hikâye" çizgisi imleçle karışmaz;
  seçim ayrı rol, imleç nötr. Hayalet imleç R7 çoklu hücrede.
- **Sınama:** P7. İmleç hapı eksen etiketini örtüyorsa etiket gizleniyor (P6).

### OV-10. Hacim alt bölmesi
- **Kalıp:** Hacim, fiyat bölmesinin alt şeridinde, aynı up/down renginde ama soluk;
  kendi lejant satırı ve son değer etiketi var. Ayrı bölme ya da kaplama seçeneği.
- **Kaynak:** Görseller `01-single-chart.png`, `A01.png`, `vela-layout.png` (ETHUSDT hücresi).
  `oc/docs/assets/charts-ui/04-volume-menu.png`.
- **Lisans hükmü:** kalıp uyarlanır.
- **Neden iyi çalışıyor:** Fiyatla aynı zaman ekseninde, dikkati çalmadan bağlam verir.
- **Qapsule'de yeri:** Çekirdek: gramer (alt bölme), görünüş (soluk alfa rolü).
- **Bizde ne değişiyor:** R5 v2'de yok; masaüstü profilinde varsayılan açık olmalı.
- **Sınama:** P7 (hacim katmanı var). Hacim çubukları fiyat etiket oluğuna taşmıyor.

### OV-11. Seans bantları
- **Kalıp:** Seans öncesi ve sonrası, düşük alfalı iki ayrı renkle arka plan bandı. Vela:
  RTH/ETH seçimi alt çubukta, ön/son seans gölge rengi ayar rolü.
- **Kaynak:** `oc/app/src/styles/themes.css` (seans rol adları), `oc/app/src/lib/chart/theme.ts`.
  Vela: `vela/docs/user/options.md:19,285-288`.
- **Lisans hükmü:** kalıp uyarlanır.
- **Neden iyi çalışıyor:** Likidite farkını renksiz krom üstünde tek bakışta gösterir.
- **Qapsule'de yeri:** Çekirdek: gramer (bant), görünüş (iki seans rolü).
- **Bizde ne değişiyor:** BIST seans takvimi (açılış/kapanış müzayedesi) çekirdekte veri.
  Bant rengi P2 ton bütçesine dahil.
- **Sınama:** Seans takvimi verildiğinde bantlar doğru bar aralığında (birim test).

### OV-12. Açıklama hapları, öncü çizgi ve çakışma önleme
- **Kalıp:** Olay/ajan açıklaması veri üstünde 28 px tam yuvarlak hap; kaynak rozeti; ince
  öncü çizgi hapı fitilin üstündeki ya da altındaki noktaya bağlar (gövdeye asla değil).
  Yerleşim: hedef çevresinde aday halkalar; mum mürekkebi, diğer etiketler ve çizgi
  kesişimi raster maske ile elenir; önce dikey üst/alt adaylar; öncü çizgi uzunluk tavanı;
  yoğunlukta ancak yerel adaylar bitince örtüşmeye izin. Kaydırmada önceki yer korunur,
  yakınlaştırmada yeniden çözülür. Üzerine gelince hap genişler, tıklayınca aynı dikdörtgen
  karta dönüşür. Çizim ve isabet testi aynı geometriyi kullanır. Vela'nın karşılığı:
  zaman ekseni üstünde olay işareti şeridi; yakın işaretler kümelenir ve deste olur,
  çizilen ile tıklanan aynı yerleşimden gelir.
- **Kaynak:** `oc/common/chart-core/src/annotation/INVARIANTS.md`, `placement-raster.ts`,
  `layout.ts`, `appearance.ts`; `oc/app/src/features/chart/components/annotation-card.tsx`;
  görsel `dashboard-preview.png` (BTCUSDT bölmesi). Vela: `vela/docs/user/timeline-marks.md`,
  `vela/src/renderers/native/chrome/marks/layout.ts` (yorumlar).
- **Lisans hükmü:** kalıp uyarlanır.
- **Neden iyi çalışıyor:** Anlam işareti veriyi örtmeden, ona çizgiyle bağlı kalıyor.
  Kararlı yer (kaydırmada zıplamıyor) güven veriyor.
- **Qapsule'de yeri:** Çekirdek: gramer katmanı (yerleşim çözücüsü) ve SceneDesc işaretleri.
  Açılan kart kabukta, çekirdeğin verdiği dikdörtgene oturur.
- **Bizde ne değişiyor:** Bizim "anlam işareti" (bulgu, olgu, niyet) bu kalıba oturur;
  R5'in 120 px yüz ve 32 px işaret yerine ≤ 28 px hap. Çözücü deterministik olmalı
  (aynı girdi aynı yerleşim); OpenChart'taki çalışma anı önbelleği bizde ya girdiye
  yazılır ya da yeniden çözüm aynı sonucu verir. Hap yazı tipi arayüz yazı tipiyle aynı
  (OpenChart'ta `system-ui` tutarsızlığı var). Köken rozeti kart 3'teki rolden.
- **Sınama:** P8 (hap ≤ 28 px, mum gövdesiyle örtüşme ≤ %X), P6. Aynı sahnede iki kez
  çözüm, bayt eşit yerleşim. Öncü çizgi tavanı aşılmıyor. Hedef noktası gövde üstünde değil.

---

## C. Çalışma alanı ve paneller

### OV-13. Panel yerleşimi ve yeniden boyutlama
- **Kalıp:** Masaüstü üç sütun: sol kenar çubuğu (eylemler kısayollu, sonra pano, uyarı,
  sohbet listeleri), orta pano (widget ızgarası), sağ ajan paneli. Ajan paneli sol
  kenarından sürüklenir (varsayılan 400, en dar 320, en geniş 800 px ve ekranın yarısı),
  klavyeyle de boyutlanır; gizlenince genişlik ve taslak korunur; dar ekranda tam genişlik
  kaplama. Izgara bölücüleri paylaşılan çizgiyi bütün olarak taşır; sürükleme kare başına
  bir önizleme, yalnız bırakınca yazım. Vela: tek "dock" sütununda bir panel görünür;
  yeniden boyutlanabilen panelin iç kenarında tutamaç, çift tık bildirilen genişliğe
  döner; panel "kaplama" olabilir ve iğneyle sütuna sabitlenir; tam ekran yalnız grafikleri
  bırakır, çıkınca her şey eski yerine.
- **Kaynak:** `oc/app/src/components/layouts/resizable-side-panel.tsx`,
  `oc/app/src/features/chart/components/shared-dividers.tsx`, `oc/app/DESIGN.md:43-45`;
  görsel `dashboard-preview.png`. Vela: `vela/docs/user/workspace.md` "The dock", "Full screen",
  "The sidebar".
- **Lisans hükmü:** kalıp uyarlanır.
- **Neden iyi çalışıyor:** Pano yeniden akmaz, yalnız daralır. Durumu tek sahip tutar.
  Sürüklemede depolama yazımı yok, akıcı.
- **Qapsule'de yeri:** Kabuk (panel düzeni, DOM). Sınır değerleri görünüş kaydından
  `css_vars` ile. Panel genişliği görünüm durumu; doküman değil.
- **Bizde ne değişiyor:** Sürükle bırak sonucu kalıcı olacaksa komut yolundan geçer
  (doküman değişikliği kuralı). Görünüm durumu (genişlik) dokümana girmiyorsa yerel tercih.
  Bu ayrım karar satırı ister.
- **Sınama:** P9 (1440×900'de ≥ 4 bölge, ≥ 2 grafik bölmesi). Panel en dar/en geniş
  sınırda grafik hücresi minimum genişliğin altına düşmüyor. Klavyeyle boyutlama erişilebilir.

### OV-14. Widget araç çubuğu üzerine gelince
- **Kalıp:** Kartın üst kenarına binen, yer ayırmayan yüzen araç çubuğu (taşı, ekle, zaman
  dilimi, gösterge, büyüt, kapat). Üzerine gelince ya da odakta görünür, açık menü varken
  kalır. Ayrı "Arrange" kipi yok; büyütme widget içinde kalır, kardeş çiziciler korunur.
- **Kaynak:** `oc/app/DESIGN.md:46-49`, `oc/app/src/features/chart/components/toolbar.tsx`,
  `widget.tsx`; görseller `01-single-chart.png`, `A01.png`. Vela: tek paylaşılan üst çubuk,
  etkin hücreye bağlı (`vela/docs/user/workspace.md` giriş, "The chrome").
- **Lisans hükmü:** kalıp uyarlanır.
- **Neden iyi çalışıyor:** Krom gizli, veri alanı büyük; denetim bir hareket uzakta.
- **Qapsule'de yeri:** Kabuk.
- **Bizde ne değişiyor:** Lejantla çakışmaması gerekir (kart 7'deki zayıflık). Dokunmatikte
  üzerine gelme yok: odak/dokunma ile açılır. Vela'nın "tek paylaşılan üst çubuk etkin
  hücreye bağlı" modeli çoklu hücrede daha az krom; ikisi arasında seçim RD2 kararı.
- **Sınama:** Araç çubuğu açıkken lejant kutusuyla kesişim 0. Klavye odağıyla görünür.

### OV-15. Çizim araç çubuğu
- **Kalıp:** Dikey, yüzen, yuvarlatılmış kapsül içinde tek sütun: tutamaç, seç, ölç/ AI
  açıklama, çizgi, ışın, kanal, dikdörtgen, ölçüm, Fibonacci, metin; ayraç; kilit, sil.
  Gruplar menü açar. Vela: araç grupları, favori yıldız, mıknatıs, "çizim kipinde kal",
  kısayol akorları menüde aracın yanında; mobilde yerleşik çubuk gizlenir, araç alt
  sayfadan seçilir ve grafiğin üstünde yüzen bir hap (etkin araç, mıknatıs, silgi, ✕) çıkar.
- **Kaynak:** `oc/app/src/features/chart/components/drawing-toolbar.tsx`; görseller
  `01-single-chart.png`, `A01.png`, `feature-explain.gif` (ilk kare). Vela:
  `vela/docs/user/drawing-tools.md`, `vela/docs/user/workspace.md` "Keyboard", "Mobile".
- **Lisans hükmü:** kalıp uyarlanır.
- **Neden iyi çalışıyor:** İşlemcinin bildiği yer ve sıra. Kapsül grafikten ayrık okunuyor.
- **Qapsule'de yeri:** Kabuk (araç çubuğu DOM). Çizim etkileşimi ve geometri çekirdekte
  (Qapsule kuralı: etkileşim mantığı çekirdekte).
- **Bizde ne değişiyor:** Bizde çizim bir niyete dönüşebilir ("çizimden niyete"): araç
  çubuğunda "izle/uyar" eylemi çizimin bağlam menüsünde. Ajan eylemi (OpenChart'taki AI
  açıklama düğmesi) ajan köken rolüyle ayrılır. OpenChart'ta kapsül mumların üstüne biniyor
  (`01-single-chart.png` sol); bizde ya sol oluk ayrılır ya da kapsülün altında veri
  soluklaşır.
- **Sınama:** Araç sırası sözleşmede listeli. Kapsül ile son değer etiketi/lejant kesişimi 0.

### OV-16. Nesne ağacı ve veri penceresi (Vela)
- **Kalıp:** Dock'ta iki panel. Nesne ağacı: bölme başına bir sütun, önden arkaya çizim
  sırası; satırda gizle/kilitle/sil; sürükleyerek bölme değiştir ya da yeni bölme aç;
  grafikteki seçimle iki yönlü eşleşir. Veri penceresi: imleçteki barın tarihi ve
  yön renginde OHLCV, gösterge başına kendi renginde değerler; imleç çıkınca son bara döner.
- **Kaynak:** `vela/docs/user/workspace.md` "The chrome" (Object tree, Data window).
  OpenChart: yok (çalışma alanı ağacı dosyalar için, `oc/app/DESIGN.md:57`).
- **Lisans hükmü:** kalıp uyarlanır.
- **Neden iyi çalışıyor:** Grafikteki her şeyin (insan, ajan, gösterge) tek listesi;
  gizli/kilitli durum görünür.
- **Qapsule'de yeri:** Kabuk paneli; içerik çekirdek sorgusundan (`query_list`/`query_describe`).
- **Bizde ne değişiyor:** Satıra köken rozeti ve olgu bağı eklenir; ajan kökenli satırlar
  kart 3 rolüyle. Bu, sicil/kapsül yüzeyiyle birleşebilir (Qapsule'e özgü katman).
- **Sınama:** Ağaçtaki öğe sayısı SceneDesc katman sayısına eşit; seçim iki yönlü.

---

## D. Ajan yüzeyi

### OV-17. Ajan paneli: çalışma katlanır, cevap açık
- **Kalıp:** Çalışma (araç çağrıları) koşarken açık, bitince süreli bir etiketin arkasına
  katlanır; elle açma bitişte sıfırlanır. Araç satırları tek satır, gri, onay işaretli.
  Son cevap katlanmış işin dışında kalır; başarısız ya da bitmemiş iş açık kalır. Besteci
  tek satır, altında satır içi model ve çalışma alanı seçicileri, alt kenarda solma.
- **Kaynak:** `oc/app/DESIGN.md:53-56`, `oc/app/src/features/agent/components/thread/transcript/working-trace.tsx`
  (başlık yorumu), `.../agent-panel/agent-panel.tsx`; görsel `dashboard-preview.png` sağ sütun.
  Vela: yok.
- **Lisans hükmü:** kalıp uyarlanır.
- **Neden iyi çalışıyor:** Ajan paneli sessiz kalır; göz grafiğe gider. Hata gizlenmez.
- **Qapsule'de yeri:** Kabuk (sohbet akışı DOM). Durumlar (koşuyor/bitti/başarısız) protokol
  olaylarından (`notify`).
- **Bizde ne değişiyor:** Bizde ajan yazımı öneri kipinde (ack: proposed): katlanmış işin
  dışında yalnız cevap değil, **onay bekleyen öneri** de açık kalmalı. Ajanın grafiğe
  yazdığı her şey panelde köken rozetiyle bağlanır. Ajan koşu olayları (araç çağrısı)
  bugün protokolde yok (`PATTERNS.md` P3); bu panel onu gerektirir.
- **Sınama:** Durum makinesi testi: koşu açık, bitti katlı, hata açık, öneri açık. Cevap
  metni hiçbir durumda katlanmış kabın içinde değil.

### OV-18. Soru kartı ve onay kartı
- **Kalıp:** Ajanın netleştirme sorusu bestecinin üstünde ayrı kart: "Question 1 of 2",
  radyo seçenekleri, serbest cevap alanı, "Skip questions" / "Next". Onay kartının durumu
  çağıranca verilir, tıklamadan çıkarılmaz.
- **Kaynak:** `oc/app/src/features/agent/components/question-card/question-card.tsx`,
  `oc/app/src/components/ui/questionnaire/`, `oc/app/src/components/ui/approval-card/approval-card.tsx`;
  görsel `dashboard-preview.png` sağ alt. Vela: yok.
- **Lisans hükmü:** kalıp uyarlanır.
- **Neden iyi çalışıyor:** Soru sohbet akışında kaybolmaz; cevap yapılandırılmış.
- **Qapsule'de yeri:** Kabuk (kart DOM); soru ve öneri nesnesi protokolde.
- **Bizde ne değişiyor:** Onay kartı öneri kipine bağlanır; onay yalnız insan kökenli
  (PATTERNS P2'deki eksik). Kart, önerinin grafikteki önizlemesine bağlantı verir
  (`query_preview`).
- **Sınama:** Onay kartı durumu dışarıdan veriliyor (birim test). Ajan kökenli onay reddediliyor.

### OV-19. Boş durum: "Nereden başlayalım?"
- **Kalıp:** Boş sohbet/pano: ortada tek soru başlığı ve büyük besteci, satır içi model ve
  çalışma alanı seçicileri; başka hiçbir şey yok. Ortak boş durum bileşeni ortalanmış
  medya + başlık + açıklama, durum tutmaz. Vela: veri yokken "eksik" değil açıklama: kaynak
  istenen zaman dilimini vermiyorsa önerdiğine geçer ve bildirim söyler; çizilemeyen stil
  gri ve ipuçlu; olgular yüklenirken hiçbir şey grileşmez.
- **Kaynak:** Görsel `oc/docs/assets/feature-market-watch.gif` (ilk kare, 640×640);
  `oc/app/src/components/ui/empty/empty.tsx`. Vela: `vela/docs/user/workspace.md`
  "A timeframe the source never serves", "A style the symbol's data cannot draw".
- **Lisans hükmü:** kalıp uyarlanır.
- **Neden iyi çalışıyor:** Tek eylem, tek odak. Vela'da kullanıcı isteği korunur
  (`requestedTimeframe`), geri dönüş açıklanır.
- **Qapsule'de yeri:** Kabuk (yüzey); "istenen ile gösterilen" ayrımı çekirdek durumunda.
- **Bizde ne değişiyor:** Bizde boş durum ajanla başlatır ama ilk eylem bir kapsül/grafik
  kurmaktır ("kur ve göster"). Vela'nın istenen/gösterilen ayrımı SceneDesc'e bir "geri
  düşüş" alanı olarak alınabilir; ajan da okur.
- **Sınama:** Boş durum ekranında tek birincil eylem. Geri düşüşte istenen değer saklanıyor
  ve bildirim bir kez çıkıyor.

### OV-20. Hata durumları ve bildirimler
- **Kalıp:** İşlem hatası bir kez, ortak bildirimle: başlığın 8 px altında, varsayılan süre,
  kapatma düğmesi yok, kısa başlık, nokta yok. Okuma hatası "Retry" verir; yazma hatası
  arayüzü ve taslağı korur, kendiliğinden yeniden denemez. Doğrulama hatası bağlamda kalır,
  satır içinde tekrarlanmaz. Kalıcı kurtarma bildirimi kaynak bileşeninde. Güncelleme asla
  diyalog açmaz: kenar çubuğunda yuvarlak bir eylem olur.
- **Kaynak:** `oc/app/DESIGN.md:41-42,58-65`, `oc/app/src/features/chart/components/resource-notice.tsx`,
  `oc/app/src/components/ui/status-banner/status-banner.tsx`. Vela: `ws.toast` ortak yüzey,
  hata bildirimi ve "zincirleme geçiş yok" kuralı (`vela/docs/user/workspace.md` 150-188).
- **Lisans hükmü:** kalıp uyarlanır.
- **Neden iyi çalışıyor:** Hata tekrarlanmaz, kaybolmaz, taslak kaybolmaz.
- **Qapsule'de yeri:** Kabuk (bildirim yüzeyi); hata metni ve kodu protokolden
  (`error.path`, `hint`), metin çekirdekte.
- **Bizde ne değişiyor:** Protokol hatası tipli; bildirim metni çekirdek/protokol metninden,
  kabuk kurmaz. Komut reddi doküman değiştirmez, kabuk taslağı tutar.
- **Sınama:** Aynı hata için tek bildirim (olay sayımı). Yazma hatasında taslak korunuyor.

---

## E. Uyarı, izleme, komut

### OV-21. Uyarı listesi ve sağlık göstergesi
- **Kalıp:** Kenar çubuğunda uyarı listesi (çan simgesi, kısaltılmış ad, tablo rakamı).
  Her kuralın sağlığı beş durum: duraklatıldı, izleniyor, bilinmiyor (dönen simge),
  bozuk (amber), başarısız (kırmızı). "Renk şiddeti, etiket anlamı taşır." Sağlıklı değilse
  sayfada kalıcı tek satır şerit: başlık, sebep, başlangıç saati; ayrıntı açılır kutuda;
  kibar canlı bölge, araya girmez. **Şerit yoksa güvence geçerli.** Yeşil sürekli
  kazanılır: kontrol "bilinmiyor"la başlar, süreli rapor yenilenmezse geri düşer; olay
  bağlantısı kopunca "doğrulanamıyor" gösterilir; 60 sn'yi aşan kesintide tek sistem
  bildirimi, iyileşince kaçırılan süreyi anlatan ikinci bildirim. Grafikte uyarı çizgileri
  yalnız projeksiyon; bağlı çizimde üzerine gelince çizgiye paralel çan hapı, görünüm
  kuralın bağlamından farklıysa yalnız çan soluklaşır. Vela: üst çubukta çan, bütün
  hücrelerden en yeni önce, her kayıt kaynağını "SEMBOL zaman dilimi Gösterge" olarak adlar,
  üst sınırlı (≈50).
- **Kaynak:** `oc/app/src/features/alerts/components/alert-health.tsx`,
  `oc/docs/architecture/alert-trigger.md` "Monitoring", "Guarantees and limits" ve çan
  hapı paragrafı, `oc/app/src/app/left-sidebar/nav-alerts.tsx`, `oc/app/src/features/chart/components/alert-lines.tsx`;
  görsel `dashboard-preview.png` sol sütun. Vela: `vela/docs/user/workspace.md:54-57,518`.
- **Lisans hükmü:** kalıp uyarlanır.
- **Neden iyi çalışıyor:** Kullanıcı "izleniyor mu" sorusunu sormadan bilir. Sessizlik
  güvenceye eşitlenir ama yalnız kanıtla.
- **Qapsule'de yeri:** Sağlık durumu ve geçişleri çekirdekte/oturumda (niyet ve izleme;
  RF1 kuralı: yeni alt sistem yeni crate'te). Liste ve şerit kabukta.
- **Bizde ne değişiyor:** Bizde uyarı = niyet/izleme (`op_add_watch`, `op_add_intent`);
  sağlık durumu protokol olayına (`notify`) girer, ajan da okur (`query_attention`). Süreli
  rapor zamanı determinizm için girdi olarak verilir. Durum adları sözlüğe (CONCEPTS.md)
  önce eklenir.
- **Sınama:** Sağlık durum makinesi birim testleri (bilinmiyor başlangıç, süre dolunca geri
  düşüş, en kötü kontrol kazanır). Sağlıklıyken şerit DOM'da yok. Renk olmadan da (gri
  tonlama) durum simgeyle ayırt edilebiliyor.

### OV-22. İzleme listesi (watchlist)
- **Kalıp:** Widget olarak izleme listesi: bölümlü, katlanır, sürüklenebilir ağaç; bütün
  satırlar aynı sütun izlerini paylaşır; satıra tıklama ya da Enter grafiği o sembole
  geçirir. Canlı rakamda yalnız değişen basamaklar yön renginde kalır (ilk değer düz).
  Seans günü küçük çizgi grafiği önceki kapanışa göre. Formül sütunu (betik çıktısı) ve
  **araştırma sütunu**: hücre başına bir ajan oturumu, Markdown önizleme, tıklayınca o
  oturum açılır, başarısız hücre yeniden denenebilir. Kart kromu genel, başlık/geçiş içerikte.
  Vela: yok (yalnız eklenti belgesinde anılıyor).
- **Kaynak:** `oc/app/src/features/watchlist/components/` (`watchlist-tree.tsx`,
  `live-number.tsx`, `sparkline.tsx`, `semantic-cell.tsx`, `semantic-column-dialog.tsx`,
  `header.tsx` başlık yorumları); görsel `feature-market-watch.gif` (yalnız ilk kare
  görüldü, liste görünmüyor).
- **Lisans hükmü:** kalıp uyarlanır.
- **Neden iyi çalışıyor:** Değişen basamak boyama tüm hücreyi yakıp söndürmeden hareketi
  gösterir. Araştırma sütunu ajanı tabloya bir kolon olarak sokar, panelden ayrı sohbet
  gerekmez.
- **Qapsule'de yeri:** Kabuk (ağaç DOM). Değişen basamak hesabı ve biçim çekirdekte (metin
  kurma yasağı); küçük çizgi çekirdek SceneDesc ile.
- **Bizde ne değişiyor:** Araştırma sütunu hücresi bir kapsül/bulgu olur, köken ve olgu
  bağıyla; doğrulanmayan sayı yayımlanmaz (R5B ilke 2).
- **Sınama:** Değişen basamak testi (yalnız fark eden basamaklar renkli). Satır seçimi
  grafik komutu üretir (komut yolundan).

### OV-23. Komut paleti ve klavye
- **Kalıp:** OpenChart'ta genel ⌘K paleti **bulunamadı**; cmdk tabanlı ortak `command`
  bileşeni sembol/model seçicide ve takvimde kullanılıyor; kenar çubuğu eylemleri kısayol
  ipuçlu (⌘N, ⌘D, ⌘A). Vela: klavye öncelikli kabuk: grafikte bir harf yazmak sembol
  aramayı o harfle açar; rakam zaman dilimi girişini açar ("15", "4H"); `?` kısayol paneli;
  bağlamalar bildirimsel kayıt (id, tuşlar, etiket, kategori, kapsam) ve otomatik `?`
  paneline düşer; kapsamlar yığılır (diyalog açıkken grafik kısayolları susar); `Esc`
  önce en içteki şeyi kapatır.
- **Kaynak:** `oc/app/src/components/ui/command/command.tsx`, görsel `dashboard-preview.png`
  sol üst. Vela: `vela/docs/user/workspace.md` "Keyboard".
- **Lisans hükmü:** kalıp uyarlanır.
- **Neden iyi çalışıyor:** İşlemci fareye dokunmadan sembol ve zaman dilimi değiştirir.
  Kayıt tek kaynak olduğu için yardım paneli eskimez.
- **Qapsule'de yeri:** Kısayol kaydı ve komut eşlemesi çekirdekte (etkileşim mantığı
  çekirdekte); palet ve `?` paneli kabukta.
- **Bizde ne değişiyor:** Palette her satır bir `qp/0` komutuna eşlenir; ajan aynı listeyi
  araç olarak görür (insan ve ajan aynı komut yüzeyi). Genel ⌘K paleti ikisinde de yok,
  burada Qapsule eklemesi gerekir (başka kaynaktan bakılmalı).
- **Sınama:** Her kısayol kaydı bir komuta eşli ve `?` panelinde listeli (tablo testi).
  Diyalog açıkken grafik kısayolu çalışmıyor.

### OV-24. Mobil düzen (Vela)
- **Kalıp:** Kapsayıcı genişliğine göre (≈640 px altı, kaba işaretçide ≈920) mobil krom;
  geçiş canlı. Tek alt çubuk iki masaüstü çubuğunun yerini alır (sembol, zaman dilimi
  sayfası, göstergeler, çizimler sayfası, üç nokta, ayarlar). Diyaloglar tam ekran; yan
  paneller grafiğin üstüne açılır; gösterge lejantı sayı hapının arkasında katlı başlar.
  Dokunma: tek parmak kaydırma, iki parmak yakınlaştırma, uzun basışla imleç; çift dokunma
  eksende ölçeği sıfırlar, çizim alanında bölmeyi büyütür. OpenChart: yan panel dar ekranda
  tam genişlik kaplama (yalnız kod; görsel yok).
- **Kaynak:** `vela/docs/user/workspace.md` "Mobile"; `oc/app/src/components/layouts/resizable-side-panel.tsx`
  (başlık yorumu).
- **Lisans hükmü:** kalıp uyarlanır.
- **Neden iyi çalışıyor:** Görünüm cihaza değil kapsayıcıya bağlı; gömülü küçük grafik de
  doğru davranır.
- **Qapsule'de yeri:** Yoğunluk profili çekirdekte (görünüş kaydı); kırılma noktası
  kabukta kapsayıcı ölçüsüyle; jest anlamı çekirdekte.
- **Bizde ne değişiyor:** R5'in telefon profili (390×844) burada "mobil yoğunluk profili"
  olur, masaüstü profilinden ayrı. Lejantın katlı başlaması P7'yi telefonda gevşetir.
- **Sınama:** Aynı sahnenin iki profilde `css_vars` farkı yalnız yoğunluk rolleri. Ç3/Ç4
  maketleri bu profille.

### OV-25. Çoklu hücre ızgarası ve senkron (ek)
- **Kalıp:** 1'den 4×4'e ızgara; tek paylaşılan krom etkin hücreye bağlı. Senkron
  türleri (görünüm, sembol, zaman dilimi, imleç, çizim, stil) ya hepsine ya adlı gruplara;
  çizim senkronu yalnız aynı piyasayı gösteren hücrelere. Filigran sembol adı hücre
  arkasında (Vela).
- **Kaynak:** `oc/docs/assets/charts-ui/07-grid-4x4.png`, `06-grid-menu.png`;
  `vela/docs/user/workspace.md` "Sync links", `vela-layout.png`.
- **Lisans hükmü:** kalıp uyarlanır.
- **Neden iyi çalışıyor:** Profesyonel yoğunluk; karşılaştırma tek ekranda.
- **Qapsule'de yeri:** Çekirdek (hücre senkron kuralları, sahne), kabuk (ızgara DOM). R7.
- **Bizde ne değişiyor:** Filigran P2 ton bütçesini ve okunurluğu bozabilir; isteğe bağlı.
  Senkron değişikliği komut yolundan.
- **Sınama:** P9. Senkronlu hücrelerde imleç zamanı eşit (birim test).

---

## Başvuru ekranları

| Amaç | Dosya (mutlak yol) | Boyut (px) | Not |
|---|---|---|---|
| Ç1 masaüstü (en iyi) | `/Users/arkns/Works/Mchart/refs/src/longsurf-ai__openchart/docs/assets/dashboard-preview.png` | 3836×2102 | Çerçevesiz uygulama, ≈DPR 2,66 ile 1440×790. Kenar çubuğu + 3 grafik widget'ı (6 bölme) + Tea düzenleyici + ajan paneli (katlı araç satırları, soru kartı, besteci). Görsel kısmen kurgusal kompozisyon (VISUAL D4 notu). |
| Ç1 yedek | `/Users/arkns/Works/Mchart/refs/src/longsurf-ai__openchart/docs/assets/readme-hero.png` | 1800×1243 | Aynı ekran, üstte logo ve beyaz pazarlama çerçevesi; kırpmak gerekir. |
| Ç1 yoğunluk | `/Users/arkns/Works/Mchart/refs/src/longsurf-ai__openchart/docs/assets/charts-ui/07-grid-4x4.png` | 2560×1680 | 16 grafik; ajan paneli yok. |
| Ç2 grafik hücresi (en iyi) | `/Users/arkns/Works/Mchart/refs/src/longsurf-ai__openchart/docs/assets/charts-ui/01-single-chart.png` | 2560×1680 | 1280×840 @2x. OHLC lejantı, son değer etiketi + noktalı çizgi, kesikli imleç + tarih/fiyat hapları, iki düzeyli zaman ekseni, hacim, dikey çizim kapsülü, yüzen widget çubuğu. |
| Ç2 açıklama hapları | `dashboard-preview.png` BTCUSDT bölmesi (yaklaşık x 500–1750, y 1490–2080) | kırpma | Kaynak rozetli haplar, öncü çizgiler, trend çizgisi, son değer etiketi. Tek iyi "ajan açıklaması" görseli. |
| Ç2 Vela | `/Users/arkns/Works/Mchart/refs/src/LuxAlgo__Vela/.github/vela-layout.png` | 1790×868 | 2×2 ızgara; son değer etiketi + geri sayım, OHLC durum satırı, BB/SuperTrend/Fib/VRVP, filigran. Gösterge renkleri çok (≈8 ton): renk disiplini için başvuru değil. NOTICE: görseli kendi ekranımızda kullanırsak atıf gerekebilir; yalnız iç karşılaştırma. |
| Ç2 kare | `/Users/arkns/Works/Mchart/refs/src/longsurf-ai__openchart/app/src/app/trellis/workflows/starter/films/captures/A01.png` | 1600×1600 | Tek grafik; çizim çubuğunda AI degrade simgesi (kart 3), hacim son değer etiketi. |
| Boş durum | `/Users/arkns/Works/Mchart/refs/src/longsurf-ai__openchart/docs/assets/feature-market-watch.gif` | 640×640 | İlk kare: "Where should we begin?" + besteci. |
| Ç3/Ç4 telefon | **yok** | – | İki projede de telefon ekran görüntüsü yok. Vela mobil krom yalnız belgede (`vela/docs/user/workspace.md` "Mobile"); OpenChart yalnız kodda (yan panel kaplaması). Telefon başvurusu başka projeden aranmalı. |

Bakılıp elenenler: `feature-agents.png` (641×641, açık tema model seçici, pazarlama
kompozisyonu), `feature-explain.gif` (640×640, ilk kare yalnız log ölçekli grafik),
`app/src/app/trellis/.../captures/*` (22 kare, çoğu tek grafik varyantı).

## Doğrulanamayanlar
- GIF'lerin yalnız ilk karesi görüldü (uyarı akışı, izleme listesi tablosu görülmedi).
- Vela'nın renk rol değerlerine ve görselinin hangi temayla çekildiğine bakılmadı; yalnız rol adları.
- OpenChart'ta genel komut paleti olmadığı arama ile çıkarıldı (`CommandDialog`, ⌘K araması boş);
  başka adla olabilir.
