#!/bin/sh
# RD2 insan testi v4: galeri çıktısını nötr kodlu, depo dışı bir klasöre kopyalar (SCREENS.md).
#   rounds/RD2/human-test/screens.sh [GALERİ_ÇIKTISI] [HEDEF]
# Varsayılan: rounds/RD2/experiments/gallery/out → ~/qp-ht-v4. Başvuru görselleri ../refs'ten
# hedefe kopyalanır (depoya değil). Kod eşlemesi GUIDE.md "Ekranlar" ile aynıdır.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../../.." && pwd)
main=$(dirname "$(git -C "$root" rev-parse --path-format=absolute --git-common-dir)")
refs=${QP_REFS:-$(cd "$main/.." && pwd)/refs}
src=${1:-$root/rounds/RD2/experiments/gallery/out}/shots
dst=${2:-$HOME/qp-ht-v4}
case "$dst" in "$root"*) echo "screens.sh: hedef depo içinde olamaz: $dst" >&2; exit 2 ;; esac

mkdir -p "$dst/screens"
cp "$here/blind.html" "$dst/"
put() { # kod kaynak
  [ -f "$2" ] || { echo "screens.sh: yok: $2" >&2; exit 1; }
  cp "$2" "$dst/screens/$1.png"
}
for th in dark:"" light:a; do
  t=${th%%:*}; s=${th#*:}
  [ -f "$src/c1-tavek-$t.png" ] || continue
  put "K3$s" "$src/c1-tavek-$t.png";     put "K1$s" "$src/c1-oltan-$t.png"
  put "K2$s" "$src/c2-tavek-$t.png";     put "K4$s" "$src/c2-oltan-$t.png"
  put "K6$s" "$src/c3-tavek-tel-$t.png"; put "K7$s" "$src/c3-oltan-$t.png"
  put "K8$s" "$src/c4-tavek-tel-$t.png"; put "K5$s" "$src/c4-oltan-$t.png"
done
put B1 "$refs/src/longsurf-ai__openchart/docs/assets/dashboard-preview.png"
put B2 "$refs/src/longsurf-ai__openchart/docs/assets/charts-ui/01-single-chart.png"
put B3 "$refs/src/Hitheshkaranth__OpenTerminalUI/assets/readme/research-mobile.png"
put B4 "$refs/src/Hitheshkaranth__OpenTerminalUI/output/playwright/wave2-data-quality-mobile.png"
echo "hazır: $dst/blind.html?o=1  (telefona: $dst/screens/K5..K8, B3, B4)"
