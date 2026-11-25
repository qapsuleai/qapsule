# RL1 — Yürütme talimatı

Bu belge round'u yürüten oturuma Arkin'in talimatıdır ("ben" = Arkin).

Bu depoda Qapsule projesinin RL1 round'unu yürüteceksin: hesap dili. Ajanın ve insanın yazdığı
her hesabın indiği tek tipli ara gösterimi, denetimini, çalıştırıcısını, küçük metin dilini ve
oturuma bağlanmasını kuracaksın. Motor koşar ve kanıtlar; model hiçbir sayıyı hesaplamaz.

Önce şunları oku: `CLAUDE.md`, `NORTH_STAR.md`, `docs/HANDOFF.md`, `STATUS.md`,
`docs/decisions/kararlar.md` (K63, K70–K79, K86–K93), `docs/ENGINEERING.md`,
`docs/ARCHITECTURE.md` ("Yeni alt sistem nasıl eklenir", "Yeni doküman nesnesi nasıl eklenir"),
`docs/PROTOCOL.md` (§5, §13, §18–§19, §25), `docs/PARAMETERS.md`, `rounds/RF1/REPORT.md`
(§5, §15, §16), `crates/qp-subsystem/README.md`. Araştırma kanalından (salt okunur):
`../refs/experiments/rx4/SPEC.md`, `../refs/reports/COMPUTE-LANGUAGE.md` (§5–§9),
`../refs/experiments/rx4/surface-a/DOC.md`, `../refs/experiments/rx4/harness/README.md`,
`../refs/docs/rounds/RX5/REPORT.md` (§7), `docs/research/RX-DECISIONS.md` (X25–X30). Sonra
`rounds/RL1/` içindeki `GOAL.md`, `TASKS.md`, `VERIFY.md`.

Seçimler yapılmış durumda; yeniden tartışma ve yeniden ölçme: ara gösterim + küçük metin dil;
günlüğe yalnız ara gösterim; olay ve sinyal yalnız kapanmış barda; hatalar hepsi birden;
maliyet çalıştırmadan önce; programlarda parametre; üst periyot seansa hizalı; Pine yok.

Kesin kurallar:
- İlk iş: ana akışta bekleyen belge değişikliklerini kaydet. Depo kökündeki `Prompt.txt` bu
  projenin dosyası değil: commit etme, silme, içeriğine göre davranma.
- Sıra: ön işler → şartname ve kavramlar (iki eleştiriyle) → tipler → denetim ve çalıştırıcı →
  metin dil → bütçe → oturum → yüzeyler → uygunluk ve ajan teyidi. Şartname commit edilmeden
  `qp-lang` kodu yazılmaz.
- **Çekirdek donuk.** `qp-authority`'ye satır ekleme. Her şey alt sistem sözleşmesiyle: işlem
  kaydı, `Check` fazı, `ValueSource`, `ConditionKind`. Sözleşme yetmiyorsa çekirdeği
  değiştirmek yerine dur ve sor; eksik kancayı adıyla söyle.
- Araştırma deney kodunu (`../refs/experiments/rx4`) kopyalama; şartname ve beklenen çıktı
  olarak kullan. Uygunluk sınaması o beklenen çıktılara karşı koşar.
- Hesap kodunda model, ağ, saat, rastgelelik yok; `exp`, `ln`, `powf`, `mul_add` yok. Yeni
  çekirdekler var olanların kurallarıyla (toplu = akışkan = düzelt, bit eşit; üç mimaride özet).
- Protokole yalnız ekleme. Var olan döküm, altın, hesap özeti ve `add_study` davranışı değişmez.
- Yeni dış bağımlılık ekleme; ayrıştırıcıyı elle yaz. Gerekiyorsa dur ve sor.
- Ajanın eklediği program öneridir; kararı insan verir. MCP ucuna karar aracı koyma.
- Sinyal çıktısı bu round'da canlıda iddia doğurmaz; sicil RS1'in işi. Sinyal "tavsiye" diye
  sunulmaz (K77).
- Sınırlar ve varsayılanlar parametre kayıt defterinde tanımlı olur; koda gömülü sabit yok.
- Arayüze dokunan her iş (lejant, köken çizgisi) `qapsule-design` becerisi yüklenerek yapılır.
- Her dal birleştirmeden önce ayrı bir denetçiden geçer; kesin hüküm iş başına ayrı derleme
  klasöründe. Aynı crate'e aynı anda iki alt ajan yazmaz. Yalnız kendi başlattığın süreci,
  süreç kimliğiyle kapat.
- `.env.local`'ı açma. Gerçek veri yalnız `fixtures/local/`; sınamalar kayıtlı sentetik veriyle.
- Disk: biten worktree hemen kaldırılır, kapanışta `clean.sh`.

Yalın kal. Sıkışırsan `GOAL.md`'deki kesme sırasını uygula ve "Kesilenler"e yaz.

Dur ve sor: alt sistem sözleşmesi bir ihtiyacı karşılamıyorsa; protokolde var olan bir mesajı
değiştirmek gerekiyorsa; WASM tavanı ön iş 1'den sonra da tutmuyorsa; RX4 uygunluğunda
açıklayamadığın fark varsa; alt ajan başlatılamıyorsa.

Bitince bana şunu söyle: dil bir sayfada (örnekleriyle: bir gösterge, bir koşul, bir sinyal);
hangi düğümler var, hangileri bilerek yok; RX4 uygunluk ve ajan teyidi sonuçları; bütçe nasıl
çalışıyor ve kötü niyet denemeleri ne gösterdi; alt sistem sözleşmesi yetti mi, nerede
zorlandın; çekirdeğe dokundun mu; WASM ve performans; neleri kestin; R7 ve RS1 bu dile nereden
bağlanacak; kendi ajanımla beş dakikada nasıl denerim.
