#!/usr/bin/env python3
"""RG1 snapshot helper: replays each conformance transcript's inputs through `qp serve`
(stdio, `--batch-end`, `--host-frames`) and writes the authority's actual reply stream
(every output line), so the baseline holds real replies, not only PASS lines.

  serve_dump.py <qp> <out-dir> <transcript>...
"""
import json, os, subprocess, sys, tempfile

def main(qp, out, files):
    os.makedirs(out, exist_ok=True)
    root = os.getcwd()
    for f in files:
        t = json.load(open(f, encoding="utf-8"))
        init = t["init"]
        tmp = None
        if not isinstance(init, str):
            tmp = tempfile.NamedTemporaryFile("w", suffix=".json", delete=False)
            json.dump(init, tmp); tmp.close()
            init = tmp.name
        lines = []
        for s in t["steps"]:
            if "send" in s:
                lines.append(json.dumps(s["send"], ensure_ascii=False))
            elif "send_raw" in s:
                lines.append('{"conn":%s,"msg":%s}' % (json.dumps(s["send_raw"]["conn"]), s["send_raw"]["line"]))
            elif "data" in s:
                lines.append(json.dumps({"host": {"data": s["data"]}}, ensure_ascii=False))
        p = subprocess.run([qp, "serve", "--init", init, "--root", root, "--batch-end",
                            "--host-frames", "--no-local-data"],
                           input="\n".join(lines) + "\n", capture_output=True, text=True)
        if tmp:
            os.unlink(tmp.name)
        name = os.path.splitext(os.path.basename(f))[0]
        with open(os.path.join(out, name + ".jsonl"), "w", encoding="utf-8") as fh:
            fh.write(p.stdout)
            fh.write(f"# exit {p.returncode}\n")

if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2], sys.argv[3:])
