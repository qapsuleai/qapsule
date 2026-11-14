#!/usr/bin/env bash
# R0/T3 web spike entry point for orchestrators/verifiers.
#   1. builds qp-wasm (profile `wasm`) into shells/web-spike/pkg with wasm-pack
#   2. prints wasm sizes: raw, gzip -9, and after `wasm-opt -Oz` (binaryen from npm)
#   3. runs compare.mjs in headless Chromium; exits non-zero if the canvas pixels differ
#      from fixtures/golden/sample.png in more than QP_WEB_MAX_DIFF_PIXELS pixels (default 0)
# Requirements: rustup target wasm32-unknown-unknown, wasm-pack, node >= 20, and
# Playwright's Chromium (npx playwright install chromium if missing).
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"
cd "$root"

wasm_pack="$(command -v wasm-pack || echo "$HOME/.cargo/bin/wasm-pack")"
echo "== build qp-wasm (wasm-pack, profile wasm)"
"$wasm_pack" build crates/qp-wasm --target web --profile wasm --no-opt \
  --out-dir ../../shells/web-spike/pkg

cd "$here"
if [ ! -d node_modules ]; then
  echo "== npm ci"
  npm ci --no-audit --no-fund
fi
if ! node -e "const fs=require('fs');process.exit(fs.existsSync(require('playwright').chromium.executablePath())?0:1)"; then
  echo "== installing Playwright chromium"
  npx playwright install chromium
fi

echo "== wasm size"
node --input-type=module -e "
import fs from 'node:fs'; import zlib from 'node:zlib'; import { execFileSync } from 'node:child_process';
const f = 'pkg/qp_wasm_bg.wasm';
const raw = fs.readFileSync(f);
fs.mkdirSync('../../target/web-spike', { recursive: true });
const opt = '../../target/web-spike/qp_wasm_bg.opt.wasm';
execFileSync('node_modules/.bin/wasm-opt', ['-Oz', '--enable-bulk-memory', '--enable-nontrapping-float-to-int', '--enable-sign-ext', '--enable-mutable-globals', '--strip-debug', '--strip-producers', f, '-o', opt]);
const o = fs.readFileSync(opt);
const gz = (b) => zlib.gzipSync(b, { level: 9 }).length;
console.log(JSON.stringify({ wasmRawBytes: raw.length, wasmGzipBytes: gz(raw), wasmOptOzBytes: o.length, wasmOptOzGzipBytes: gz(o) }));
"

echo "== browser compare"
node compare.mjs
