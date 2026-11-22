# RG1 dal denetçisi — kurallar

Sen bir taşıma dalını **bağımsız** denetlersin; kodu sen yazmadın, düzeltmezsin. Hiçbir
dosyayı değiştirme, commit etme, birleştirme. Yalnız ölç ve hüküm ver.

Önce oku: `rounds/RG1/PLAN.md` §2 ve §13 (özellikle §13.1 kapı), `rounds/RG1/briefs/MOVER.md`.

## Ortam
- Denetlediğin worktree'ye `cd` et. **Kendi derleme klasörünü kullan** (ortak klasör
  worktree'ler arasında güvenilir değil):
  `export CARGO_TARGET_DIR=/Users/arkns/Works/Mchart/qapsule/target/wt/check-<görev>`.
  İş bitince o klasörü **sil** (`rm -rf`).
- Taban: dalın `main` ile ortak atası (`git merge-base main HEAD`).

## Kapı (hepsi o dalda, o klasörle)
1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace` — özet: geçen/kalan; test sayısı tutanakla aynı mı
   (`rounds/RG1/experiments/baseline/text/tests.count`, 1164; ad listesi
   `text/tests.names` — `cargo test --workspace -q -- --list 2>/dev/null | grep ': test$' | sed -E 's/: test$//; s/.*:://' | LC_ALL=C sort | diff - rounds/RG1/experiments/baseline/text/tests.names`).
4. `scripts/rg1-compare.sh` → "225 items byte-identical".
5. `python3 /Users/arkns/Works/Mchart/qapsule/scripts/rg1-moved.py $(git merge-base main HEAD) HEAD --list` (ana ağaçtaki **güncel** sürüm; cwd denetlenen worktree) → CHANGED 0; PATH,
   ALLOW ve izinli CHANGED kalemlerinin (planın §13'ünde ve görev metninde adıyla izin verilenler)
   her birine tek tek bak: gerçekten yalnız yol mu, izinli mi?
6. `scripts/arch-check --json` → görevin crate(ler)inde `file.hard` (istisna isteği olarak
   bildirilen tek fonksiyonlu dosyalar dışında), `root.logic`, `name.forbidden`, `path.outside`,
   `layer.up` 0; `dir.files` `src/` altında 0.
7. Elle örneklem: rastgele 5 taşınan öğeyi `git log --follow`/`git show` ile eski hâliyle
   karşılaştır; ayrıca `git diff $(git merge-base main HEAD) HEAD --stat` ile görev alanı dışında
   dosya değişmiş mi bak (yalnız izinli crate'ler, `Cargo.lock`).

## Hüküm (son cevabın, Türkçe, kısa)
"GEÇTİ" ya da "KALDI"; her adımın sonucu bir satır; KALDI ise neden ve hangi dosya/öğe;
şüpheli bulduğun ama kapıyı düşürmeyen şeyler.
