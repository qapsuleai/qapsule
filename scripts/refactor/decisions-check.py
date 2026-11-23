#!/usr/bin/env python3
"""RG1/D1: DECISIONS.md bölünmesinin denetimi.

  scripts/refactor/decisions-check.py <bölünmeden-önceki-commit> [<bölünme-commit'i>]

Bölünme commit'i verilirse (1) o commit'teki dosyalarla eşitlik olarak, ayrıca HEAD'de
"özgün satırların hiçbiri kaybolmadı" (alt küme) olarak sınanır; sonradan eklenen karar
satırları ve dosyaları serbesttir.

1. Özgün DECISIONS.md'nin boş olmayan satırları (başlık `# Karar Günlüğü` hariç) çoklu
   kümesi == docs/decisions/**/*.md dosyalarının satırları (her dosyanın eklenen üç satırlık
   başı hariç) çoklu kümesi.
2. Her K/Ö/A satırı (numara + metin) aynen bir dosyada.
3. Dizin (DECISIONS.md) her bölüm başlığını ve dosyasını listeliyor, dosyalar var.
4. Depo içindeki `DECISIONS.md` başvurularında anılan bölüm adları ("Önerilen (R5)" gibi) ve
   K/Ö/A numaraları bölünmüş kayıtta bulunuyor (kırık başvuru 0).
"""
import collections, glob, re, subprocess, sys

def read_at(rev, path):
    return subprocess.run(["git", "show", f"{rev}:{path}"], capture_output=True, text=True,
                          check=True).stdout


def main(base, split=None):
    old = subprocess.run(["git", "show", f"{base}:DECISIONS.md"], capture_output=True,
                         text=True, check=True).stdout.split("\n")
    old_c = collections.Counter(l for l in old if l.strip() and l != "# Karar Günlüğü")
    new_c = collections.Counter()
    head_c = collections.Counter()
    if split:
        files = sorted(subprocess.run(["git", "ls-tree", "-r", "--name-only", split, "docs/decisions"],
                                      capture_output=True, text=True, check=True).stdout.split())
    else:
        files = sorted(glob.glob("docs/decisions/**/*.md", recursive=True))
    for f in sorted(glob.glob("docs/decisions/**/*.md", recursive=True)):
        head_c.update(l for l in open(f, encoding="utf-8").read().split("\n") if l.strip())
    for f in files:
        lines = (read_at(split, f) if split else open(f, encoding="utf-8").read()).split("\n")
        assert lines[0].startswith("# Karar Günlüğü — ") and lines[2].startswith("<!-- dizin"), f
        new_c.update(l for l in lines[4:] if l.strip())
    ok = True
    if old_c != new_c:
        ok = False
        print("satır kümesi FARKLI")
        for l, n in (old_c - new_c).items(): print("  eksik:", l[:100], n)
        for l, n in (new_c - old_c).items(): print("  fazla:", l[:100], n)
    else:
        print(f"satır kümesi eşit: {sum(old_c.values())} satır, {len(files)} dosya")
    lost = old_c - head_c
    if lost:
        ok = False
        for l, n in lost.items(): print("HEAD'de kaybolan satır:", l[:100], n)
    else:
        print("HEAD: özgün satırların hepsi duruyor")
    idx = open("DECISIONS.md", encoding="utf-8").read()
    heads = [l[3:] for l in old if l.startswith("## ")]
    for h in heads:
        if h not in idx:
            ok = False; print("dizinde yok:", h)
    for f in files:
        if f not in idx:
            ok = False; print("dizinde dosya yok:", f)
    ids = set(re.findall(r"^\| ([KÖA]\d+) ", "\n".join(new_c), re.M))
    print(f"başlık {len(heads)}/{len(heads)} dizinde; {len(ids)} farklı numara")
    refs = subprocess.run(["git", "grep", "-n", "DECISIONS.md", "--", "*.md", "*.rs", "*.mjs", "*.sh"],
                          capture_output=True, text=True).stdout.splitlines()
    old_idx = "\n".join(old)
    broken = pre = 0
    def bad(msg, key_ok_before):
        nonlocal broken, pre
        if key_ok_before:
            broken += 1; print("YENİ kırık", msg)
        else:
            pre += 1; print("önceden de kırık:", msg)
    old_ids = set(re.findall(r"^\| ([KÖA]\d+) ", old_idx, re.M))
    for r in refs:
        if r.startswith(("DECISIONS.md:", "docs/decisions/")):
            continue
        for n in re.findall(r"`DECISIONS\.md`[^.;\n]{0,60}?\b([KÖA]\d+)", r):
            if n not in ids:
                bad(f"numara {n} ← {r[:110]}", n in old_ids)
        for s in re.findall(r'"((?:Önerilen|R\d\w*) \(?[A-Z0-9]+\)?[^"]*)"', r):
            key = s.split(" —")[0]
            if key not in idx and not any(h.startswith(key) for h in heads):
                bad(f"bölüm {key} ← {r[:110]}", key in old_idx)
    print(f"depo içi başvuru: {len(refs)} satır, yeni kırık {broken}, önceden de kırık {pre}")
    return 0 if ok and broken == 0 else 1

if __name__ == "__main__":
    sys.exit(main(sys.argv[1], sys.argv[2] if len(sys.argv) > 2 else None))
