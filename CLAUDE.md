# Qapsule — Claude Code Kuralları

Bu repoda çalışan her oturum ve her alt ajan önce şunları okur:
`NORTH_STAR.md`, `docs/CONCEPTS.md`, `docs/ARCHITECTURE.md`, `docs/ENGINEERING.md`,
`STATUS.md` ve aktif round'un `rounds/Rn/` klasörü. Ana akış oturumu `docs/HANDOFF.md`'den başlar.

## Değişmezler

- `NORTH_STAR.md` değiştirilmez. Çelişki görürsen dur ve rapora yaz.
- Hesap kodunda LLM, ağ çağrısı veya rastgelelik olmaz. Aynı girdi her zaman aynı çıktıyı verir.
- Fiyat ve hesap değerleri `f64`.
- Şemanın tek kaynağı `qp-schema` içindeki Rust tipleridir. JSON Schema ve diğer
  dillerdeki tipler üretilir, elle yazılmaz.
- Dokümanı değiştiren her şey komut yolundan geçer.
- Çizim, etkileşim ve veri mantığı çekirdekte (Rust) yaşar. Kabuklar incedir.
- Kavramlar `docs/CONCEPTS.md` içindeki adlarla kullanılır. Yeni kavram gerekiyorsa
  önce sözlüğe eklenir.

## Çalışma biçimi

- Yapı da kabul ölçütüdür (K80): `docs/ENGINEERING.md` kuralları her round'da geçerlidir;
  bir değişiklik dosyayı ya da modülü bütçenin üstüne itecekse önce bölünür.

- Round'un kapsamı dışına çıkma. Fark ettiğin kapsam dışı işleri `REPORT.md` içinde
  "Öneriler" başlığına yaz, yapma.
- Alt görevleri alt ajanlara ver; her alt ajan yalnızca kendi crate'ine veya klasörüne yazar.
- Bir alt görev, `VERIFY.md` içindeki ilgili adımlar geçmeden bitmiş sayılmaz.
- Doğrulamayı işi yapan ajan değil, ayrı bir denetçi alt ajan çalıştırır.
- Testleri kırık bir dal `main`'e birleştirilmez. Birleştirmeden önce o dalda `scripts/verify.sh`
  (en azından fmt, clippy, test adımları) yeşil olmalıdır.
- Alt ajan başlatılamıyorsa bunu round'un başında `REPORT.md`'ye yaz ve Arkin'e bildir;
  işe tek başına devam edebilirsin, ama denetim yine bağımsız bir alt ajanla yapılmalıdır.
- Yeni altın görsel yalnızca yeni bir görünüm için eklenir; var olan altın görsel
  kendiliğinden güncellenmez.
- Mimari bir seçim yaptığında `DECISIONS.md` içine satır ekle; "Açık" bir kararı
  kapatıyorsan kanıtını (ölçüm, deneme sonucu) yaz.
- Emin olmadığın kütüphane davranışını varsayma; dokümantasyonuna bak ve küçük bir
  denemeyle doğrula.

## Güvenlik ve hijyen

- `.env`, token, anahtar hiçbir zaman commit edilmez. `../Services/**/.env` okunmaz, kopyalanmaz.
- Depo kökündeki `.env.local` Arkin'in girdiği belirteç ve anahtarları tutar (`docs/MODELS.md`,
  `docs/DATA_ACCESS.md`). Bu dosyayı açma, içeriğini yazdırma, değerlerini komut satırına ya da
  günlüğe koyma. Betikler değeri ortamdan okur. Eksikse hangi değişkenin gerektiğini söyle ve bekle.
- Gerçek piyasa verisi yalnızca `fixtures/local/` altında durur; içeriği, türevleri ve ondan
  üretilen görseller commit edilmez, rapora yalnızca toplu istatistik girer.
- `../Docs`, `../Services` ve `../refs` (araştırma kanalı; dizin `docs/research/README.md`) salt
  okunurdur; oradan yalnızca okunur, oraya yazılmaz. `../refs/experiments` ve `../refs/src`
  altındaki kod bu depoya kopyalanmaz (K83).
- Commit'ler küçük ve anlamlı olur; her alt görev kendi dalında veya worktree'sinde çalışır.

## GitHub hesabı

Bu makinede birden fazla GitHub hesabı var ve etkin `gh` hesabı başka klasörlerdeki
işler tarafından değiştirilebilir. Bu yüzden:

- `gh` doğrudan çağrılmaz; her zaman `scripts/qgh` kullanılır. `qgh`, etkin hesaba
  dokunmadan yalnızca o çağrı için doğru hesabı ve depoyu kullanır.
- `gh auth switch`, `gh auth login`, `gh auth logout` ve `gh auth setup-git` çalıştırılmaz.
- Her `git push`, uzak dal birleştirme, PR, sürüm ve CI işleminden hemen önce
  `scripts/preflight.sh` çalıştırılır. Başarısızsa işlem yapılmaz, durum rapora yazılır
  ve Arkin'e sorulur; sorunu kendi başına hesap değiştirerek çözmeye çalışma.
- `origin` adresi `scripts/qapsule-gh.env` içindekiyle aynı kalır; HTTPS adresine çevrilmez.
- Başka bir depoya, çatala ya da hesaba gönderim yapılmaz.

## Ana akış değişiklikleri

Round koşarken ana akış oturumu aynı çalışma ağacında doküman yazabilir (`NORTH_STAR.md`,
`DECISIONS.md`, `STATUS.md`, `ROADMAP.md`, `CLAUDE.md`, `docs/`, `rounds/`). Bunlar commit
edilmemiş olarak durur.

- Birleştirmeden önce ana ağaçta senin yapmadığın, commit edilmemiş doküman değişikliği
  varsa bunları değiştirmeden tek bir commit olarak kaydet (ileti: "ana akış: ..."), sonra
  birleştir. Çakışmada iki tarafın satırlarını da koru.
- `NORTH_STAR.md` değişikliği yalnızca `DECISIONS.md`'de onu açıkça anan bir karar satırı
  varsa bu yolla commit edilir. Yoksa dur ve Arkin'e sor.
- Doküman dışındaki (kod, betik, test verisi) sahipsiz değişiklikleri commit etme; sor.

## Disk hijyeni (K56)

Worktree başına ayrı `target/` klasörü diski hızla doldurur (bir kez 400 GB elle temizlendi).

- Bütün worktree'ler tek bir ortak derleme klasörü kullanır: alt ajanı başlatırken
  `CARGO_TARGET_DIR` ana ağacın `target/` klasörünü göstersin. Worktree içinde ayrı
  `target/` oluşturma. Eşzamanlı derlemeler kilit bekler; bu kabul edilir. Kesin hükmü veren
  derleme (denetçi, birleştirme öncesi karşılaştırma) iş başına ayrı klasörde koşar
  (`target/wt/<iş>`) ve iş bitince silinir (K87).
- Bir alt görev birleştirildiğinde ya da elendiğinde worktree'sini hemen kaldır
  (`git worktree remove`), dalı raporda adıyla bırak. Elenen alternatifin kodu dalda
  kalır, klasörü kalmaz.
- Geçici dosyalar (`/tmp`, scratchpad, başsız tarayıcı profilleri, ekran görüntüsü
  yığınları) iş bitince silinir. Kanıt olarak gerekenler `rounds/Rn/experiments/` altına
  küçük boyutta taşınır.
- Round kapanışında `scripts/clean.sh` çalıştırılır (yoksa yaz: `git worktree prune`,
  kalan worktree'lerin listesi ve kaldırılması, `cargo clean` yerine yalnız eski
  profillerin ve artımlı önbelleğin silinmesi, `node_modules` ve tarayıcı önbelleği,
  önce/sonra boyut). `--dry-run` ne sileceğini gösterir. Betik depo dışına ve
  `fixtures/local/`, `.env.local` dosyalarına dokunmaz.
- **Kapanış temizliği zorunludur (K68).** Round'un ürettiği geçici her şey silinir:
  worktree'ler ve `target/wt/*`; scratchpad ve `/tmp` altındaki oturum klasörleri; başsız
  tarayıcı profilleri, ekran görüntüsü ve video yığınları; ara ölçüm dökümleri ve günlükler;
  yarım indirmeler; kullanılmayan derleme profilleri ve artımlı önbellek. Kanıt olarak
  gereken, küçültülerek `rounds/Rn/experiments/` altına taşınır (tek dosya 1 MB'ı, klasör
  20 MB'ı geçmez; geçiyorsa özet tablo bırakılır).
- **`target/` bütçesi:** round sonunda en çok 6 GB (ana ağacın güncel derlemeleri).
  Aşıyorsa `scripts/clean.sh --deep` (gerekirse `cargo clean` + yalnız `verify.sh`'ın
  gerektirdiği yeniden derleme). `clean.sh` bu seçenekleri taşımıyorsa ekle.
- **`fixtures/local/` saklama:** ham canlı kayıtlar 30 gün ya da toplam 2 GB; geçmiş veri
  önbelleği toplam 5 GB; aşan en eskiler silinir. Kanıt olan kayıtlar raporda adıyla sayılır.
- **Rapor "Disk" bölümü üç liste verir:** silinenler (ne, kaç GB), bilerek bırakılanlar
  (neden), kalan boyutlar (`target/`, `.claude/worktrees/`, `fixtures/local/`, depo toplamı,
  depo dışı oturum klasörleri). `git worktree list` yalnız ana ağaç;
  `git status --porcelain --ignored` özetinde beklenmeyen artık dosya yok.
- Temizlik depo dışına (oturumun kendi geçici klasörleri hariç), `.env.local`'a ve başka
  projelere dokunmaz. Emin olmadığın dosyayı silme; raporda sor.

## Yapı (K80, K89)

- Her round `scripts/arch-check` yeşil ve borç taban çizgisi büyümeden kapanır.
- RF1'e kadar `qp-session`'a net satır eklenmez; yeni alt sistem yeni crate'te doğar.
- Yeni istisna ya da taban çizgisi gevşetmesi karar satırı ister; kendin ekleme, sor.

## Round kapanışı

1. `scripts/verify.sh` yeşil.
2. `rounds/Rn/REPORT.md` yazıldı: ne yapıldı, ne yapılamadı, ölçümler, açık sorular, öneriler.
3. `STATUS.md` ve `ROADMAP.md` durum sütunu güncellendi.
4. Disk hijyeni: worktree'ler kaldırıldı, `scripts/clean.sh` çalıştı, kalan boyut raporda.
5. Sonraki round'u kendin başlatma; round tasarımı ana akış oturumunda yapılır.
