# Qapsule — Claude Code Kuralları

Bu repoda çalışan her oturum ve her alt ajan önce şunları okur:
`NORTH_STAR.md`, `docs/CONCEPTS.md`, `docs/ARCHITECTURE.md`, `STATUS.md` ve aktif
round'un `rounds/Rn/` klasörü.

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
- `../Docs` ve `../Services` salt okunurdur; oradan yalnızca okunur, oraya yazılmaz.
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

## Round kapanışı

1. `scripts/verify.sh` yeşil.
2. `rounds/Rn/REPORT.md` yazıldı: ne yapıldı, ne yapılamadı, ölçümler, açık sorular, öneriler.
3. `STATUS.md` ve `ROADMAP.md` durum sütunu güncellendi.
4. Sonraki round'u kendin başlatma; round tasarımı ana akış oturumunda yapılır.
