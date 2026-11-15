// RP2/T8 (docs/PROTOCOL.md §19.6, VERIFY 12): replays a replica tape written by
// `qp replica-tape` (native authority + native replica) in the WASM replica under Node and
// checks the document, view and display digests after every step against the native ones;
// with lag 0 also against the authority's.
// Usage: node replica_tape.mjs <wasm-pack nodejs pkg dir> <tape.json>...
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const require = createRequire(import.meta.url);
const [pkgDir, ...tapes] = process.argv.slice(2);
if (!pkgDir || tapes.length === 0) {
  console.error('usage: node replica_tape.mjs <pkg dir> <tape.json>...');
  process.exit(2);
}
const wasm = require(resolve(pkgDir, 'qp_wasm.js'));
const VIEW = ['v', 800, 400];
let failed = 0;
for (const file of tapes) {
  const tape = JSON.parse(readFileSync(file, 'utf8'));
  const r = new wasm.Replica(tape.principal, JSON.stringify(tape.provider));
  for (const [id, cond] of tape.watches) r.watch(id, JSON.stringify(cond));
  let bad = 0, sameAsAuthority = 0, i = 0;
  for (const step of tape.steps) {
    for (const input of step.inputs) {
      if (input.msg !== undefined) r.on_message(JSON.stringify(input.msg));
      else { try { r.data_event(JSON.stringify(input.data)); } catch (_) { /* as native */ } }
    }
    r.take_outbox();
    const got = { doc: r.document_digest(), view: r.view_digest(...VIEW), display: r.display_digest() };
    const e = step.expect;
    if (got.doc !== e.doc || got.view !== e.view || got.display !== e.display) {
      if (bad < 5) console.error(`${file} step ${i}: wasm ${JSON.stringify(got)} != native ${JSON.stringify(e)}`);
      bad++;
    }
    if (got.doc === e.authority_doc && got.view === e.authority_view) sameAsAuthority++;
    if (tape.lag === 0 && (got.doc !== e.authority_doc || got.view !== e.authority_view)) {
      if (bad < 5) console.error(`${file} step ${i}: wasm replica != authority`);
      bad++;
    }
    i++;
  }
  const last = tape.steps[tape.steps.length - 1].expect;
  console.log(`${file}: lag ${tape.lag}, ${i} steps, wasm == native (doc, view, display) at ${i - bad}/${i}; wasm == authority (doc, view) at ${sameAsAuthority}/${i}; last doc ${last.doc} view ${last.view} display ${last.display}`);
  if (bad) failed++;
}
process.exit(failed ? 1 : 0);
