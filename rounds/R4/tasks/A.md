# R4/A — Bağımsız son denetim

Sen R4'ün işini yapmadın; bağımsız denetçisin. Yapanın notlarına güvenme, kendin dene.
Yazdığın tek yer `rounds/R4/AUDIT.md` (ve gerekiyorsa `rounds/R4/audit/` altında yalnız sentetik
ekran görüntüleri ve betikler). Ürün koduna, dökümlere, altınlara dokunma; bulgu yaz.

Önce: `CLAUDE.md`, `NORTH_STAR.md`, `rounds/R4/GOAL.md` (15 ölçüt), `rounds/R4/VERIFY.md`
(20 adım — her birini sen uygula), `rounds/R4/TASKS.md`, `docs/CONCEPTS.md`, `docs/FACE.md`,
`DECISIONS.md` "Önerilen (R4)".

Özellikle (VERIFY'a göre):
- **Gösterimi kendin aç** (`scripts/demo.sh`, yerel veri olmadan temiz bir kopyada — ör. `git worktree add` ile yeni bir ağaç; sentetik olduğunu ekranda söylüyor mu) ve başsız tarayıcıda (`shells/web/node_modules` Playwright, 390×844) **bir kullanıcı gibi kullan**: akış, kapsüle dokunma, grafiğe inme, kaydırma, yakınlaştırma, kanıta açılma (yetkili/yetkisiz), karne, hassasiyet seviyesi, izleme listesi ve pozisyon işaretleme, son bakış, Türkçe/İngilizce, açık/koyu tema, alfabe ve yüz (ilk iki) değiştirme, oynatma denetimleri. Ekran görüntülerini (yalnız sentetik) `rounds/R4/audit/`'e koy, bulguları yaz.
- **Ağ:** sayfanın bütün isteklerini kaydet; yalnız yerel adres mi.
- **Kabuğun inceliği kendi yönteminle:** mimari testinin yanında kendi taraman (kabuk kodunda hesap, cümle, sıralama, yerleşim var mı); ve aynı oturumu yalnız protokol mesajlarıyla (kabuksuz; `qp drive` ya da kendi istemcin) sür — doküman ve sahne özetleri kabuklu koşuyla aynı mı.
- **Yeniden oynatma** iki kez aynı girdilerle: mesaj akışı bayt eşit mi.
- **M8 persona koşularından üçünü kendi taze ajanlarınla tekrarla** (düzenek `rounds/R4/experiments/harness/m8910/`, `run_readers.sh` yöntemiyle aynı kısıtlı `claude -p --model sonnet --restricted ...`; aynı istem ve görüntüler; sonuçları `rounds/R4/audit/m8-repeat/`'e). Sıralama raporla tutarlı mı; farkları yaz.
- **M8–M11 kayıtları:** düzenek/ölçüt commit'lerinin alternatiflerden önce olduğu (`git log` sırası), okuyucu istemlerinde tasarım adlarının geçmediği (kendi taraman).
- **K42:** geçerli olmayan kapsülde başlık ve işaret aynı şeyi mi söylüyor (iki dil); süre ifadesi günlük ve 60 dk periyotta hafta sonu/tatil aşan örnekte takvime göre doğru mu (kendi örneklerin).
- **Renk karşıtlığı ve gri kopya:** eşikler; bilinen açık (kapanmış işaret 2,06) dahil.
- **Ölçüm ekranı:** masaüstü ve 4× kısıtta sayılar `rounds/R4/DEVICE.md` ile ±%20 uyumlu mu.
- **Bağlanma süresi** 30 enstrüman < 100 ms mı (T6 düzeneği).
- **Hassasiyet benzetimi** belirleyici mi; `important`'ta pozisyon/niyet doğum payı.
- **Hijyen:** `git ls-files | grep fixtures/local` boş; depodaki ekran görüntülerinde gerçek sembol/veri yok; gizli bilgi taraması (anahtar/belirteç kalıpları); `.env.local` hiçbir commit'te yok; eski round klasörleri (`rounds/R0..R3`, `RD1`, `RP*`) değişmemiş (`git diff main -- rounds/R3` vb.); var olan altınlar ve hesap özeti `main`'e göre değişmemiş (değişenler K42 listesinde mi).
- `QP_VERIFY_BROWSER=1 scripts/verify.sh` kendin koş.

`AUDIT.md`: her VERIFY adımı için PASS/FAIL/KISMİ + kanıt (komut, sayı, dosya); GOAL'un 15
ölçütü için senin hükmün; bulgular önem sırasıyla (engelleyici / önemli / küçük); "persona ajanı
insan değildir" sınırı. Commit iletileri şu satırla bitsin:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
