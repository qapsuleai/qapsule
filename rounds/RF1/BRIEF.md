# RF1 — Yürütme talimatı

Bu belge round'u yürüten oturuma Arkin'in talimatıdır ("ben" = Arkin).

Bu depoda Qapsule projesinin RF1 round'unu yürüteceksin: oturumun ve gramerin bölünmesi. Yeni
özellik yok. Amaç, bütçenin üstündeki üç crate'i (`qp-session`, `qp-grammar`, `qp-capsule`)
bölmek ve sonraki dört round'un (RL1, R7, RS1, RA1) kendi crate'lerinde doğabileceği sınırları
kurmak.

Önce şunları oku: `CLAUDE.md`, `NORTH_STAR.md`, `docs/HANDOFF.md`, `STATUS.md`,
`docs/decisions/kararlar.md` (K78–K92), `docs/ENGINEERING.md`, `docs/ARCHITECTURE.md`,
`docs/DEBT.md`, `rounds/RG1/REPORT.md` ve `PLAN.md` (§3, §4), `rounds/RG1/AUDIT.md`,
`rounds/RD2/REPORT.md` (§12, §16), `docs/design/SURFACES.md` (R7'nin kuracakları için göz at) ve
`rounds/RF1/` içindeki `GOAL.md`, `TASKS.md`, `VERIFY.md`.

**Tek değişmez: davranış değişmez.** RG1'den farklı olarak fonksiyon gövdelerini ve tip
sınırlarını değiştirebilirsin; ama hiçbir altın, döküm, özet, şema ya da araç tanımı değişmez.
Bir değişiklik bunlardan birini oynatıyorsa değişiklik yanlıştır: geri al. Fark ettiğin hatayı
düzeltme; raporda "Fark edilenler"e yaz.

Bir yerde durup bana soracaksın:

- **Bölme planı hazır olunca.** `PLAN.md` ve iki eleştiriye cevapların yazılınca dur. Bana
  planın bir sayfalık özetini ver: hedef crate'ler ve tahmini boyutları, alt sistem sınırları,
  otorite arayüzü, RL1 / R7 / RS1 / RA1'in nereye oturacağı, en riskli üç adım. Ben planı ana
  akış oturumuna gösterip cevabı getireceğim; cevabı `DECISION.md`'ye yaz. Onaydan önce yalnız
  tutanak, ön işler, araçlar ve `qp-voice` taşıması yapılabilir.

Kesin kurallar:
- İlk iş: ana akışta bekleyen belge değişikliklerini kaydet (bu sefer
  `.claude/skills/qapsule-design/SKILL.md` de aralarında; o da ana akışın). Sonra başlangıç
  tutanağı; tutanak commit edilmeden kod değişmez.
- Sıra: tutanak → araçlar → plan → karar noktası → gramer → oturum. Gramer önce: `qp-grammar`
  tavana 36 satır uzakta.
- Gövdesi değişecek her yerde önce karakterizasyon sınaması yaz (bugünkü çıktıyı kilitleyen
  ince taneli test), sonra değiştir. Sınama değişiklikten önce commit edilir.
- Taşınabileni taşı; yeniden yazdığın her öğeyi say ve raporda listele.
- Alt sistemler otoriteyi tanımaz. "Yeni alt sistem nasıl eklenir" sorusunun cevabı otorite
  crate'ine dokunmadan verilebilmeli; örnek sınamayla kanıtla.
- Protokol (`qp/0`), şema, komut satırı ve MCP yüzeyi değişmez. Rust'ın dışa açık yolları
  değişebilir; listele.
- Yeni istisna ya da taban çizgisi gevşetmesi ekleme; gerekiyorsa dur ve sor. Kapanan istisnayı
  yapılandırmadan sil.
- Yeni kavram adı gerekiyorsa önce `docs/CONCEPTS.md`. Crate adları kısa ve işini söyleyen
  adlar olsun; `util`, `common`, `core2` gibi adlar yok.
- Her dal birleştirmeden önce ayrı bir denetçiden geçer; kesin hükmü veren derleme iş başına
  ayrı klasörde. Aynı crate'e aynı anda iki alt ajan yazmaz.
- Yalnız kendi başlattığın süreci, süreç kimliğiyle kapat; `pkill` / `killall` yok (K92).
- Depo dışında yalnız GOAL'de adı geçen üç eski Playwright klasörünü sil; önce listele,
  tarihlerini doğrula.
- `.env.local`'ı açma. `NORTH_STAR.md`'ye dokunma. Disk: biten worktree hemen kaldırılır,
  kapanışta `clean.sh`.

Yalın kal: hedef üç istisnayı kapatmak ve yer açmak; uzun fonksiyon borcunun tamamını
temizlemek değil. Sıkışırsan `GOAL.md`'deki kesme sırasını uygula ve "Kesilenler"e yaz.

Dur ve sor (karar noktası dışında): tutanak karşılaştırması bir değişiklikten sonra farklı
çıkıyor ve nedenini bulamıyorsan; bir sınır protokolü ya da şemayı değiştirmeden çizilemiyorsa;
12 bin hedefi tek adlı istisnayla bile tutmuyorsa; performans ya da WASM boyutu sınırı
aşıyorsa; alt ajan başlatılamıyorsa.

Bitince bana şunu söyle: tutanak karşılaştırmasının sonucu; önce ve sonra crate tablosu;
oturum hangi alt sistemlere ayrıldı ve otorite ne kadar kaldı; gramerde ne ayrıldı ve R7 için
ne kadar pay var; kapanan ve kalan istisnalar; kaç öğeyi yeniden yazdın; ek noktası sınaması
ne gösterdi; RL1, R7, RS1 ve RA1 nereden başlayacak; neleri kestin; fark ettiğin ama
dokunmadığın şeyler.
