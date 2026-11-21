#!/usr/bin/env bash
# Disk hijyeni (K56, CLAUDE.md "Disk hijyeni").
#
#   scripts/clean.sh [--dry-run] [--force] [--node-modules] [--deep]
#
# Ne yapar:
#   1. `git worktree prune` (klasörü kaybolmuş kayıtlar).
#   2. Ana ağaç dışındaki worktree'leri listeler. Depo içindeki (`.claude/`) worktree'leri
#      kaldırır; commit edilmemiş değişikliği olanları yalnız `--force` ile kaldırır
#      (yoksa atlar ve söyler). Depo DIŞINDAKİ worktree'lere dokunmaz, yalnız listeler.
#      Dallar kalır; yalnız klasör gider. Kilitli worktree: kilidi tutan süreç yaşıyorsa
#      hiçbir koşulda kaldırılmaz; değilse yalnız --force ile (Claude Code alt ajan kilitleri
#      oturum sürecinin pid'ini taşır: biten ajanınkini `git worktree unlock` ile açın).
#   3. Ortak `target/` içinde: kullanılmayan eski profil/hedef klasörleri ve artımlı
#      önbellek (`incremental/`). `cargo clean` yapılmaz; güncel profiller kalır.
#      `target/wt/<görev>`: worktree başına alt klasör (eşzamanlı worktree'ler aynı
#      klasörde birbirinin çıktısını ezdiği için; K56 notu). Worktree'si kalmamış olan
#      alt klasörler silinir; `--force` ile hepsi.
#   4. Worktree'lerdeki `node_modules`; ana ağacınkiler yalnız `--node-modules` ile
#      (gösterim ve doğrulama onlara bağlı; yeniden kurulum ağ ister).
#   5. Depo içindeki başsız tarayıcı profilleri (`target/tmp`, `*.chrome-profile`).
#   6. (`--deep`, K68) `target/` bütçeyi (6 GB) aşıyorsa `cargo clean` (bütün `target/`;
#      bir sonraki `scripts/verify.sh` yeniden derler). Bütçe altındaysa hiçbir şey yapmaz.
#
# Dokunmadığı yerler: depo dışı, `fixtures/local/`, `.env.local`, izlenen dosyalar.
# Önce/sonra boyutları basar: `target/`, `.claude/worktrees/`, `.claude/wt/`, depo toplamı.
set -euo pipefail

dry=0
force=0
nm=0
deep=0
for a in "$@"; do
  case "$a" in
    --dry-run) dry=1 ;;
    --force) force=1 ;;
    --node-modules) nm=1 ;;
    --deep) deep=1 ;;
    -h|--help) sed -n '2,25p' "$0"; exit 0 ;;
    *) echo "clean: bilinmeyen seçenek: $a" >&2; exit 2 ;;
  esac
done

# Ana ağaç: worktree listesinin ilk satırı (betik bir worktree'den çağrılsa bile).
root="$(git worktree list --porcelain | awk '/^worktree /{print substr($0,10); exit}')"
cd "$root"

say() { printf '%s\n' "$*"; }
size() { if [ -e "$1" ]; then du -sh "$1" 2>/dev/null | cut -f1; else echo "-"; fi; }

# Korunan yollar: hiçbir rm bunlara ulaşmamalı.
guard() {
  local p
  p="$(cd "$(dirname "$1")" 2>/dev/null && pwd)/$(basename "$1")"
  case "$p" in
    "$root"/*) ;;
    *) say "clean: depo dışı yol reddedildi: $p" >&2; return 1 ;;
  esac
  case "$p" in
    "$root"/fixtures/local|"$root"/fixtures/local/*|"$root"/.env.local|"$root"/.git|"$root"/.git/*)
      say "clean: korunan yol reddedildi: $p" >&2; return 1 ;;
  esac
  return 0
}

del() {
  guard "$1" || return 0
  if [ "$dry" = 1 ]; then
    say "  [dry-run] silinecek: ${1#"$root"/} ($(size "$1"))"
  else
    say "  siliniyor: ${1#"$root"/} ($(size "$1"))"
    rm -rf -- "$1"
  fi
}

report() {
  say "  target/              $(size target)"
  say "  .claude/worktrees/   $(size .claude/worktrees)"
  say "  .claude/wt/          $(size .claude/wt)"
  say "  depo toplamı         $(size .)"
}

say "clean: kök $root$([ "$dry" = 1 ] && echo '  (dry-run)')"
say "önce:"
report

# 1. prune
say "1. git worktree prune"
if [ "$dry" = 1 ]; then git worktree prune --dry-run -v || true; else git worktree prune -v || true; fi

# 2. worktree'ler
say "2. worktree'ler"
git worktree list --porcelain | awk '/^worktree /{print substr($0,10)}' | tail -n +2 | while IFS= read -r w; do
  case "$w" in
    "$root"/*) ;;
    *) say "  depo dışı, dokunulmadı: $w"; continue ;;
  esac
  lock="$(git worktree list --porcelain | awk -v w="$w" '$0=="worktree "w{f=1;next} /^worktree /{f=0} f&&/^locked/{sub(/^locked ?/,"");print;exit}')"
  if [ -n "$lock" ] || git worktree list --porcelain | awk -v w="$w" '$0=="worktree "w{f=1;next} /^worktree /{f=0} f&&/^locked/{e=1} END{exit !e}'; then
    pid="$(printf '%s' "$lock" | sed -n 's/.*pid \([0-9][0-9]*\).*/\1/p')"
    if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
      say "  ATLANDI (kilitli, sahibi çalışıyor: pid $pid): ${w#"$root"/}"
      continue
    fi
    if [ "$force" != 1 ]; then
      say "  ATLANDI (kilitli; --force ile kaldırılır): ${w#"$root"/}"
      continue
    fi
  fi
  dirty="$(git -C "$w" status --porcelain --untracked-files=no 2>/dev/null | wc -l | tr -d ' ')"
  if [ "$dirty" != 0 ] && [ "$force" != 1 ]; then
    say "  ATLANDI (commit edilmemiş $dirty değişiklik; --force ile kaldırılır): ${w#"$root"/}"
    continue
  fi
  br="$(git -C "$w" rev-parse --abbrev-ref HEAD 2>/dev/null || echo '?')"
  if [ "$dry" = 1 ]; then
    say "  [dry-run] kaldırılacak: ${w#"$root"/} (dal $br kalır, $(size "$w"))"
  else
    say "  kaldırılıyor: ${w#"$root"/} (dal $br kalır, $(size "$w"))"
    git worktree remove -f -f "$w" || say "  UYARI: kaldırılamadı: $w"
  fi
done
# Kayıtsız kalmış boş worktree klasörleri
registered="$(git worktree list --porcelain | awk '/^worktree /{print substr($0,10)}')"
for d in .claude/worktrees/* .claude/wt/*; do
  [ -d "$d" ] || continue
  if ! printf '%s\n' "$registered" | grep -qxF "$root/$d"; then
    del "$root/$d"
  fi
done

# 3. target: güncel profiller kalır; geri kalanı eski.
say "3. target/ (eski profiller ve artımlı önbellek)"
keep=" release debug wasm wasm32-unknown-unknown x86_64-apple-darwin wasm-bench web-spike wt CACHEDIR.TAG .rustc_info.json "
if [ -d target/wt ]; then
  live="$(git worktree list --porcelain | awk '/^worktree /{print substr($0,10)}' | tail -n +2 | wc -l | tr -d ' ')"
  for d in target/wt/*; do
    [ -d "$d" ] || continue
    if [ "$force" = 1 ] || [ "$live" = 0 ]; then del "$root/$d"; else say "  kalır (worktree'ler hâlâ var; --force ile silinir): $d ($(size "$d"))"; fi
  done
fi
if [ -d target ]; then
  for d in target/* target/.[!.]*; do
    [ -e "$d" ] || continue
    b="$(basename "$d")"
    case "$keep" in *" $b "*) ;; *) del "$root/$d" ;; esac
  done
  find target -type d -name incremental -prune 2>/dev/null | while IFS= read -r d; do
    if [ -n "$(ls -A "$d" 2>/dev/null)" ]; then del "$root/$d"; fi
  done || true
fi

# 4. node_modules
say "4. node_modules"
find .claude -type d -name node_modules -prune 2>/dev/null | while IFS= read -r d; do del "$root/$d"; done || true
if [ "$nm" = 1 ]; then
  find shells tools -type d -name node_modules -prune 2>/dev/null | while IFS= read -r d; do del "$root/$d"; done || true
else
  say "  ana ağaç node_modules kalır (--node-modules ile silinir)"
fi

# 5. başsız tarayıcı profilleri
say "5. tarayıcı profilleri"
find . -path ./.git -prune -o -path ./fixtures/local -prune -o -type d \( -name '*.chrome-profile' -o -name 'chrome-profile*' \) -print 2>/dev/null \
  | while IFS= read -r d; do del "$root/${d#./}"; done || true

# 6. --deep: target/ bütçesi (K68)
if [ "$deep" = 1 ]; then
  say "6. --deep: target/ bütçesi 6 GB"
  kb=$(du -sk target 2>/dev/null | cut -f1 || echo 0)
  if [ "${kb:-0}" -gt $((6 * 1024 * 1024)) ]; then
    if [ "$dry" = 1 ]; then
      say "  [dry-run] cargo clean (target/ $(size "$root/target"))"
    else
      CARGO_TARGET_DIR="$root/target" cargo clean
      say "  cargo clean yapıldı"
    fi
  else
    say "  target/ bütçe altında ($(size "$root/target")); dokunulmadı"
  fi
fi

say "sonra:"
report
say "git worktree list:"
git worktree list
