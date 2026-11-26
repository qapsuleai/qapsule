# RL1 — Alt ajan ortak kuralları (yürüten oturumun talimatı)

Repo: /Users/arkns/Works/Mchart/qapsule. Round RL1 ("hesap dili"). Belgeler ve raporlar Türkçe;
kod yorumları kod tabanı gibi İngilizce.

Önce oku: CLAUDE.md, docs/ENGINEERING.md, rounds/RL1/BRIEF.md, rounds/RL1/GOAL.md,
rounds/RL1/DECISION.md, docs/PROGRAMS.md (normatif şartname qp-ir/0), docs/PROGRAMS-SESSION.md,
docs/CONCEPTS.md ("Hesap dili (RL1)"), docs/PARAMETERS.md (RL1 anahtarları) ve dokunduğun her
crate'in README.md'si.

Kesin kurallar:
- YALNIZ kendi worktree'nde çalış: depo kökünden `git worktree add .claude/worktrees/<ad> -b rl1/<ad> main`.
  Yalnız görevin adlandırdığı crate/klasörlere yaz.
- Derleme: `export CARGO_TARGET_DIR=/Users/arkns/Works/Mchart/qapsule/target/wt/<ad>` (iş başına bir
  klasör, K87; worktree içinde target/ yok). Bu round'da ölçüldü: worktree'ler arasında paylaşılan
  klasör aynı crate'in farklı ağaçlardaki eserlerini karıştırıp yanlış derleme sonucu veriyor.
  Bitince yalnız kendi klasörünü `rm -rf` et. Scratchpad'deki başkalarının dosyalarını silme.
- `crates/qp-authority/src` altında SIFIR değişiklik. `crates/qp-session/src`'ye net satır yok;
  şartnamenin adlandırdığı kayıt satırları (`compose.rs` LIST / FOLLOWER_LIST) hariç.
- Yeni dış bağımlılık yok. Gerekiyorsa dur ve raporda söyle.
- Hesap kodu: model, ağ, saat, rastgelelik yok; yalnız IEEE temel işlemler ve sqrt (exp, ln, powf,
  mul_add yok). Aynı girdi → bayt eşit çıktı.
- Protokol yalnız ekleme. Var olan dökümler (fixtures/protocol), altınlar (fixtures/golden,
  fixtures/views), hesap özetleri (fixtures/compute) ve `add_study` davranışı bayt eşit kalır. Var
  olan fixture'ı yeniden üretme ya da değiştirme; yeni fixture yalnız yeni dosya.
- Sınırlar/varsayılanlar parametre kayıt defterinden (`qp_schema::params::REGISTRY`), koda gömülü değil.
- Yapı kabul ölçütüdür (K80): `scripts/arch-check` yeşil, yeni istisna yok, taban çizgisi büyümez;
  bütçeyi aşmadan önce böl. Yeni crate ⇒ README.md, `[lints] workspace = true`, arch-config katman sırası.
- `.env.local`'ı açma. Gerçek piyasa verisi yalnız fixtures/local/ (sınamalarda kullanma).
  `../refs` salt okunur; RX4 deney kodu kopyalanmaz.
- Yalnız kendi başlattığın süreci PID ile kapat (pkill/killall yok).
- Arayüze dokunan iş (lejant, köken çizgisi) `qapsule-design` becerisi yüklenerek yapılır.
- Bitirmeden: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  dokunulan crate'lerin (ve API değiştiyse bağımlılarının) testleri, `scripts/arch-check` ve
  `scripts/arch-check --config-diff`. Küçük anlamlı commit'ler, önek "RL1/<görev>:", son satır
  "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>". main'e birleştirme, push yok.
  Worktree'yi denetçi için bırak.
- Şartname yanlış ya da belirsizse GOAL/DECISION ile tutarlı en yalın okumayı seç, raporda
  "Şartname notları" altında yaz.
- Son rapor < 50 satır: dal, commit'ler, yapılan (genel API adları), eklenen sınamalar (sayı),
  denetim sonuçları, şartname notları, açık konular.
