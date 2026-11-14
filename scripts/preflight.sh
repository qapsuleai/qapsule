#!/usr/bin/env bash
# preflight — git push/merge ve gh işlemlerinden ÖNCE çalıştırılır.
# Yanlış hesaba ya da yanlış depoya gönderimi engeller. Hiçbir şeyi değiştirmez.
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
. "$here/qapsule-gh.env"
fail=0
ok()  { printf 'PASS  %s\n' "$1"; }
bad() { printf 'FAIL  %s\n' "$1"; fail=1; }

# 1. origin doğru mu
url="$(git -C "$here/.." remote get-url origin 2>/dev/null || true)"
if [ "$url" = "$QAPSULE_REMOTE_URL" ]; then ok "origin = $url"
else bad "origin beklenen '$QAPSULE_REMOTE_URL', bulunan '${url:-yok}'"; fi

# 2. SSH takma adı doğru hesabı selamlıyor mu (git push bu yolu kullanır)
greet="$(ssh -T -o BatchMode=yes -o ConnectTimeout=10 "git@$QAPSULE_SSH_ALIAS" 2>&1 || true)"
case "$greet" in
  *"Hi $QAPSULE_GH_USER!"*) ok "ssh $QAPSULE_SSH_ALIAS -> $QAPSULE_GH_USER" ;;
  *"Hi "*) bad "ssh $QAPSULE_SSH_ALIAS başka bir hesabı selamlıyor: $(printf '%s' "$greet" | head -1)" ;;
  *) bad "ssh $QAPSULE_SSH_ALIAS çalışmıyor: $(printf '%s' "$greet" | head -1)" ;;
esac

# 3. gh: doğru hesap ve depoya erişim (etkin hesaptan bağımsız, qgh üzerinden)
who="$("$here/qgh" api user -q .login 2>&1)"
if [ "$who" = "$QAPSULE_GH_USER" ]; then ok "gh hesabı = $who (qgh)"
else bad "gh: $who"; fi
perm="$("$here/qgh" api "repos/$QAPSULE_GH_REPO" -q '.permissions.push' 2>&1)"
if [ "$perm" = "true" ]; then ok "depoya yazma izni var: $QAPSULE_GH_REPO"
else bad "depoya yazma izni doğrulanamadı: $perm"; fi

# 4. bilgi: makinedeki etkin gh hesabı (değiştirilmez)
active="$(env -u GH_TOKEN gh api user -q .login 2>/dev/null || echo '?')"
printf 'INFO  makinedeki etkin gh hesabı: %s (dokunulmadı)\n' "$active"

# 5. commit e-postası gerçek bir adres mi
mail="$(git -C "$here/.." config user.email 2>/dev/null || true)"
case "$mail" in *@*.*) ok "commit e-postası: $mail" ;; *) bad "git user.email geçerli bir adres değil: '${mail}'" ;; esac

if [ "$fail" -ne 0 ]; then echo "preflight: BAŞARISIZ — gönderme, birleştirme ve gh işlemi yapma."; exit 1; fi
echo "preflight: TAMAM"
