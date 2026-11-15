| Run | First try | First try id-free | Final | Final id-free | First try had an error | Attempts | Unnumbered runs | Msgs (first / all) | Tokens | Output tokens | Agent time (s) | Cost |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1 | 12/20 | 10/20 | 19/20 | 10/20 (lenient final 20) | 5 | 33 | 0 | 46 / 81 | 7,167,389 | 57,692 | 573.3 | 3.86 $ |
| r2 | 11/20 | 9/20 | 20/20 | 11/20 (lenient final 20) | 4 | 36 | 0 | 53 / 100 | 8,754,979 | 64,306 | 612.7 | 4.36 $ |

| Task | r1 first / final (att) id-free | r2 first / final (att) id-free |
|---|---|---|
| C01 | ✓ / ✓ (2) id-free | ✓ / ✓ (2) id-free |
| C02 | ✗ / ✓ (2) ids | ✗ / ✓ (2) ids |
| C03 | ✗ / ✓ (3) ids | ✗ / ✓ (2) ids |
| C04 | ✗ / ✓ (2) ids | ✗ / ✓ (2) ids |
| C05 | ✓ / ✓ (1) id-free | ✓ / ✓ (1) id-free |
| C06 | ✓ / ✓ (1) id-free | ✓ / ✓ (1) id-free |
| C07 | ✓ / ✓ (1) id-free | ✓ / ✓ (1) id-free |
| C08 | ✓ / ✓ (2) id-free | ✗ / ✓ (2) id-free |
| C09 | ✗ / ✓ (2) ids | ✗ / ✓ (2) ids |
| C10 | ✗ / ✓ (2) ids | ✗ / ✓ (2) ids |
| C11 | ✓ / ✓ (2) ids | ✓ / ✓ (3) id-free |
| C12 | ✓ / ✓ (1) id-free | ✓ / ✓ (1) id-free |
| C13 | ✗ / ✗ (2)  | ✗ / ✓ (3) ids |
| C14 | ✗ / ✓ (2) ids | ✗ / ✓ (2) ids |
| C15 | ✓ / ✓ (1) ids | ✓ / ✓ (2) ids |
| C16 | ✓ / ✓ (1) id-free | ✓ / ✓ (2) id-free |
| C17 | ✓ / ✓ (2) id-free | ✓ / ✓ (2) id-free |
| C18 | ✗ / ✓ (2) id-free | ✗ / ✓ (2) id-free |
| C19 | ✓ / ✓ (1) id-free | ✓ / ✓ (1) id-free |
| C20 | ✓ / ✓ (1) ids | ✓ / ✓ (1) ids |

### r1

| Task | Agent | Att | First | Final | Ids used (first; last) | Errors (codes, per attempt) | Why (first attempt) | Why (last attempt) |
|---|---|---|---|---|---|---|---|---|
| C01 | A | 2 | ✓ | ✓ | —; — | —; — | — | — |
| C02 | B | 2 | ✗ | ✓ | —; cap.THYAO.breakout.down.311 | —; — | capsule query on the open THYAO breakout with evidence values returned | — |
| C03 | C | 3 | ✗ | ✓ | —; cap.ASELS.breakout.up.313 | —; —; — | capsule query on cap.ASELS.breakout.up.313; answer gives the reason (None) | — |
| C04 | D | 2 | ✗ | ✓ | —; fnd.THYAO.breakout.down.314 | not_found; — | reject_finding on the open THYAO breakout's finding, in the log (applied or proposed) | — |
| C05 | E | 1 | ✓ | ✓ | —; — | — | — | — |
| C06 | A | 1 | ✓ | ✓ | —; — | — | — | — |
| C07 | B | 1 | ✓ | ✓ | —; — | — | — | — |
| C08 | C | 2 | ✓ | ✓ | —; — | —; — | — | — |
| C09 | D | 2 | ✗ | ✓ | —; fnd.THYAO.extreme.down.312 | not_found; — | present of the open THYAO extreme capsule at capsule depth | — |
| C10 | E | 2 | ✗ | ✓ | —; cap.THYAO.breakout.down.314, fnd.THYAO.breakout.down.314 | —; — | present of the open THYAO breakout at chart depth; answer says the broken level is not drawn | — |
| C11 | A | 2 | ✓ | ✓ | —; cap.THYAO.flow.down.301 | schema; not_found | — | — |
| C12 | B | 1 | ✓ | ✓ | —; — | — | — | — |
| C13 | C | 2 | ✗ | ✗ | —; cap.THYAO.breakout.down.314 | —; — | capsule query on cap.THYAO.breakout.down.310 | capsule query on cap.THYAO.breakout.down.310 |
| C14 | D | 2 | ✗ | ✓ | —; fnd.THYAO.breakout.down.314 | schema; — | verify_evidence on the open THYAO breakout's fact (or its finding) | — |
| C15 | E | 1 | ✓ | ✓ | cap.THYAO.breakout.down.314, fnd.THYAO.breakout.down.314; cap.THYAO.breakout.down.314, fnd.THYAO.breakout.down.314 | — | — | — |
| C16 | A | 1 | ✓ | ✓ | —; — | — | — | — |
| C17 | B | 2 | ✓ | ✓ | —; — | —; — | — | — |
| C18 | C | 2 | ✗ | ✓ | —; — | schema; — | editor's position THYAO = 100 (got None) | — |
| C19 | D | 1 | ✓ | ✓ | —; — | — | — | — |
| C20 | E | 1 | ✓ | ✓ | fnd.THYAO.breakout.down.314; fnd.THYAO.breakout.down.314 | — | — | — |

### r2

| Task | Agent | Att | First | Final | Ids used (first; last) | Errors (codes, per attempt) | Why (first attempt) | Why (last attempt) |
|---|---|---|---|---|---|---|---|---|
| C01 | A | 2 | ✓ | ✓ | —; — | —; — | — | — |
| C02 | B | 2 | ✗ | ✓ | —; cap.THYAO.breakout.down.314 | —; — | capsule query on the open THYAO breakout with evidence values returned | — |
| C03 | C | 2 | ✗ | ✓ | —; cap.ASELS.breakout.up.314 | —; — | capsule query on cap.ASELS.breakout.up.314; answer gives the reason (None) | — |
| C04 | D | 2 | ✗ | ✓ | —; fnd.THYAO.breakout.down.310 | not_found; — | reject_finding on the open THYAO breakout's finding, in the log (applied or proposed) | — |
| C05 | E | 1 | ✓ | ✓ | —; — | — | — | — |
| C06 | A | 1 | ✓ | ✓ | —; — | — | — | — |
| C07 | B | 1 | ✓ | ✓ | —; — | — | — | — |
| C08 | C | 2 | ✗ | ✓ | —; — | schema; not_found,not_found | ASELS in editor's watch list (got None) | — |
| C09 | D | 2 | ✗ | ✓ | —; fnd.THYAO.extreme.down.308 | —; — | present of the open THYAO extreme capsule at capsule depth | — |
| C10 | E | 2 | ✗ | ✓ | —; fnd.THYAO.breakout.down.312 | not_found; — | present of the open THYAO breakout at chart depth; answer says the broken level is not drawn | — |
| C11 | A | 3 | ✓ | ✓ | —; — | —; schema,not_found; — | — | — |
| C12 | B | 1 | ✓ | ✓ | —; — | — | — | — |
| C13 | C | 3 | ✗ | ✓ | —; cap.THYAO.breakout.down.311 | —; —; — | capsule query on cap.THYAO.breakout.down.311 | — |
| C14 | D | 2 | ✗ | ✓ | —; fnd.THYAO.breakout.down.310 | schema; — | verify_evidence on the open THYAO breakout's fact (or its finding) | — |
| C15 | E | 2 | ✓ | ✓ | cap.THYAO.breakout.down.312, fnd.THYAO.breakout.down.312; cap.THYAO.breakout.down.312, fnd.THYAO.breakout.down.312 | —; — | — | — |
| C16 | A | 2 | ✓ | ✓ | —; — | —; — | — | — |
| C17 | B | 2 | ✓ | ✓ | —; — | —; — | — | — |
| C18 | C | 2 | ✗ | ✓ | —; — | —; not_found,not_found | never run | — |
| C19 | D | 1 | ✓ | ✓ | —; — | — | — | — |
| C20 | E | 1 | ✓ | ✓ | fnd.THYAO.breakout.down.312; fnd.THYAO.breakout.down.312 | — | — | — |
