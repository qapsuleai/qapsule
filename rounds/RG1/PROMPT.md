# RG1 — Claude Code Goal Metni

```sh
cd /Users/arkns/Works/Mchart/qapsule
scripts/preflight.sh
claude --add-dir /Users/arkns/Works/Mchart/refs
```

"preflight: TAMAM" görmeden başlatma. Aynı çalışma ağacında iki round aynı anda koşmaz.

Oturum açılınca:

```
/goal rounds/RG1/GOAL.md içindeki 15 kabul ölçütünün hepsi karşılandı (kesilenler ve istisna istekleri raporda gerekçesiyle), rounds/RG1/VERIFY.md'deki 15 adım geçti, başlangıç tutanağındaki her kalem bayt eşit, bağımsız denetçi AUDIT.md'yi yazdı ve STATUS.md güncellendi.
```

Ardından şu metni ver:

---

Bu depoda Qapsule projesinin RG1 round'unu yürüteceksin: yönetişim turu. Yeni özellik yok.
Geliştirme kurallarını ve otomatik denetimi depoya koyacak, mevcut kodu bu kurallara göre
yeniden düzenleyecek ve parametre kayıt defterinin iskeletini açacaksın.

Önce şunları oku: `CLAUDE.md`, `NORTH_STAR.md`, `docs/HANDOFF.md`, `docs/ENGINEERING.md`,
`docs/ARCHITECTURE.md`, `DECISIONS.md` (özellikle K69–K83), `STATUS.md`,
`rounds/R5B/REPORT.md` (§6–§8) ve `rounds/RG1/` içindeki `GOAL.md`, `TASKS.md`, `VERIFY.md`.

**Tek değişmez: davranış değişmez.** Hiçbir altın görsel, döküm, özet, şema ya da araç tanımı
güncellenmez. Bir düzenleme bunlardan birini değiştiriyorsa düzenleme yanlıştır: geri al.
"Küçük bir iyileştirme" de yapma; fark ettiğin hatayı düzeltme, raporda "Fark edilenler"e yaz.

Kesin kurallar:
- İlk iş: ana akışta bekleyen belge değişikliklerini `CLAUDE.md` "Ana akış değişiklikleri"
  kuralına göre kaydet; sonra **başlangıç tutanağını** al ve commit et. Tutanak commit
  edilmeden kod taşıma.
- Sıra: tutanak → `arch-check` → bölme planı (`rounds/RG1/PLAN.md`; iki bağımsız alt ajan
  eleştirir) → taşımalar. Plan yazılmadan kod taşınmaz.
- Taşı, yeniden yazma: `git mv` + yalnız `mod` / `use` / görünürlük / yeniden dışa aktarma
  satırları. Fonksiyon gövdesi, ad, mantık değişmez. Uzun fonksiyonları bu round'da bölme;
  taban çizgisine yaz.
- Protokol (`qp/0`), şema, komut satırı ve MCP yüzeyi değişmez.
- Her taşıma kendi dalında, küçük commit'lerle; birleştirmeden önce o dalda `verify.sh`
  (fmt, clippy, test) ve `scripts/rg1-compare.sh` yeşil. Aynı crate'e aynı anda iki alt ajan
  yazmaz.
- `docs/ENGINEERING.md` metnini değiştirme; uygulanamaz bulduğun kuralı "Kural önerileri"ne
  yaz. `NORTH_STAR.md`'ye dokunma.
- `DECISIONS.md` bölünürken K, Ö, A numaraları ve metinleri değişmez.
- `recovered/worktree-agent/*` dallarını silme; özetle ve bana karar listesi ver.
- `.env.local`'ı açma. `../refs` salt okunur; oradan kod kopyalama.
- Disk: ortak `CARGO_TARGET_DIR`, biten worktree hemen kaldırılır, kapanışta `clean.sh`.

Yalın kal: hedef borcu sıfırlamak değil, kesin sınırları tutturmak ve kalanını görünür
kılmak. Sıkışırsan `GOAL.md`'deki kesme sırasını uygula ve "Kesilenler"e yaz. Bir bütçe
tutturulamıyorsa zorlamak yerine gerekçeli istisna isteği yaz; kararı ben veririm.

Dur ve sor: tutanak karşılaştırması bir taşımadan sonra farklı çıkıyor ve nedenini
bulamıyorsan; bir bölme protokolü ya da şemayı değiştirmeden yapılamıyorsa; alt ajan
başlatılamıyorsa.

Bitince bana şunu söyle: tutanak karşılaştırmasının sonucu; önce ve sonra crate ve en büyük
dosya tablosu; `qp-session` ve kabuklar nasıl bölündü; hangi istisnaları istiyorsun; borç
taban çizgisinde ne kaldı; kayıt defterine bağlanan sabitler; `recovered/*` dalları için
önerin; neleri kestin; fark ettiğin ama dokunmadığın şeyler.
