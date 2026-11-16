| Run | First try | First try id-free | Final | Final id-free | First try had an error | Attempts | Unnumbered runs | Msgs (first / all) | Tokens | Output tokens | Agent time (s) | Cost |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1 | 14/20 | 12/20 | 15/20 | 13/20 (lenient final 16) | 0 | 29 | 0 | 45 / 67 | 7,087,901 | 49,685 | 433.7 | 3.56 $ |
| r2 | 17/20 | 15/20 | 16/20 | 13/20 (lenient final 17) | 1 | 26 | 0 | 46 / 62 | 9,216,913 | 50,197 | 453.4 | 5.06 $ |

| Task | r1 first / final (att) id-free | r2 first / final (att) id-free |
|---|---|---|
| C01 | ✗ / ✗ (3)  | ✓ / ✗ (3)  |
| C02 | ✓ / ✗ (2)  | ✓ / ✗ (2)  |
| C03 | ✓ / ✓ (1) id-free | ✓ / ✓ (1) id-free |
| C04 | ✓ / ✓ (1) id-free | ✓ / ✓ (1) id-free |
| C05 | ✓ / ✓ (2) id-free | ✓ / ✓ (1) id-free |
| C06 | ✓ / ✓ (1) id-free | ✓ / ✓ (1) id-free |
| C07 | ✓ / ✓ (1) id-free | ✓ / ✓ (1) id-free |
| C08 | ✗ / ✓ (2) id-free | ✓ / ✓ (1) id-free |
| C09 | ✓ / ✓ (1) id-free | ✓ / ✓ (1) id-free |
| C10 | ✗ / ✗ (2)  | ✗ / ✓ (2) ids |
| C11 | ✗ / ✗ (2)  | ✗ / ✗ (1)  |
| C12 | ✓ / ✓ (1) id-free | ✓ / ✓ (1) id-free |
| C13 | ✗ / ✗ (1)  | ✗ / ✗ (2)  |
| C14 | ✓ / ✓ (1) id-free | ✓ / ✓ (1) id-free |
| C15 | ✓ / ✓ (1) ids | ✓ / ✓ (1) ids |
| C16 | ✓ / ✓ (1) id-free | ✓ / ✓ (1) id-free |
| C17 | ✓ / ✓ (2) id-free | ✓ / ✓ (2) id-free |
| C18 | ✗ / ✓ (2) id-free | ✓ / ✓ (1) id-free |
| C19 | ✓ / ✓ (1) id-free | ✓ / ✓ (1) id-free |
| C20 | ✓ / ✓ (1) ids | ✓ / ✓ (1) ids |

### r1

| Task | Agent | Att | First | Final | Ids used (first; last) | Errors (codes, per attempt) | Why (first attempt) | Why (last attempt) |
|---|---|---|---|---|---|---|---|---|
| C01 | A | 3 | ✗ | ✗ | —; — | —; —; — | answer names every open THYAO capsule (missing: ['cap.THYAO.extreme.down.313', 'cap.THYAO.continuation.down.314']) | a capsules query whose answer holds every open THYAO capsule; answer names every open THYAO capsule (missing: ['cap.THYAO.extreme.down.313', 'cap.THYAO.continuation.down.314']) |
| C02 | B | 2 | ✓ | ✗ | —; cap.THYAO.breakout.down.311 | —; — | — | capsule query on the open THYAO breakout with evidence values returned |
| C03 | C | 1 | ✓ | ✓ | —; — | — | — | — |
| C04 | D | 1 | ✓ | ✓ | —; — | — | — | — |
| C05 | E | 2 | ✓ | ✓ | —; — | —; — | — | — |
| C06 | A | 1 | ✓ | ✓ | —; — | — | — | — |
| C07 | B | 1 | ✓ | ✓ | —; — | — | — | — |
| C08 | C | 2 | ✗ | ✓ | —; — | —; — | ASELS in editor's watch list (got None) | — |
| C09 | D | 1 | ✓ | ✓ | —; — | — | — | — |
| C10 | E | 2 | ✗ | ✗ | —; cap.THYAO.breakout.down.311 | —; — | present of the open THYAO breakout at chart depth; answer says the broken level is not drawn | present of the open THYAO breakout at chart depth; answer says the broken level is not drawn |
| C11 | A | 2 | ✗ | ✗ | —; cap.THYAO.flow.down.298 | —; — | answer names the entitlement it needs (flow/…) | a capsule / capsules / verify_evidence query as viewer; answer names the entitlement it needs (flow/…) |
| C12 | B | 1 | ✓ | ✓ | —; — | — | — | — |
| C13 | C | 1 | ✗ | ✗ | —; — | — | capsule query on cap.THYAO.breakout.down.309 | capsule query on cap.THYAO.breakout.down.309 |
| C14 | D | 1 | ✓ | ✓ | —; — | — | — | — |
| C15 | E | 1 | ✓ | ✓ | cap.THYAO.breakout.down.311, fnd.THYAO.breakout.down.311; cap.THYAO.breakout.down.311, fnd.THYAO.breakout.down.311 | — | — | — |
| C16 | A | 1 | ✓ | ✓ | —; — | — | — | — |
| C17 | B | 2 | ✓ | ✓ | —; — | —; — | — | — |
| C18 | C | 2 | ✗ | ✓ | —; — | —; — | editor's position THYAO = 100 (got None) | — |
| C19 | D | 1 | ✓ | ✓ | —; — | — | — | — |
| C20 | E | 1 | ✓ | ✓ | fnd.THYAO.breakout.down.311; fnd.THYAO.breakout.down.311 | — | — | — |

### r2

| Task | Agent | Att | First | Final | Ids used (first; last) | Errors (codes, per attempt) | Why (first attempt) | Why (last attempt) |
|---|---|---|---|---|---|---|---|---|
| C01 | A | 3 | ✓ | ✗ | —; — | —; —; — | — | a capsules query whose answer holds every open THYAO capsule |
| C02 | B | 2 | ✓ | ✗ | —; cap.THYAO.breakout.down.312 | schema; — | — | capsule query on the open THYAO breakout with evidence values returned |
| C03 | C | 1 | ✓ | ✓ | —; — | — | — | — |
| C04 | D | 1 | ✓ | ✓ | —; — | — | — | — |
| C05 | E | 1 | ✓ | ✓ | —; — | — | — | — |
| C06 | A | 1 | ✓ | ✓ | —; — | — | — | — |
| C07 | B | 1 | ✓ | ✓ | —; — | — | — | — |
| C08 | C | 1 | ✓ | ✓ | —; — | — | — | — |
| C09 | D | 1 | ✓ | ✓ | —; — | — | — | — |
| C10 | E | 2 | ✗ | ✓ | —; cap.THYAO.breakout.down.314, fnd.THYAO.breakout.down.314 | —; — | present of the open THYAO breakout at chart depth; answer says the broken level is not drawn | — |
| C11 | A | 1 | ✗ | ✗ | —; — | — | answer names the entitlement it needs (flow/…) | answer names the entitlement it needs (flow/…) |
| C12 | B | 1 | ✓ | ✓ | —; — | — | — | — |
| C13 | C | 2 | ✗ | ✗ | —; — | —; — | capsule query on cap.THYAO.breakout.down.307 | capsule query on cap.THYAO.breakout.down.307 |
| C14 | D | 1 | ✓ | ✓ | —; — | — | — | — |
| C15 | E | 1 | ✓ | ✓ | cap.THYAO.breakout.down.314, ex437, fnd.THYAO.breakout.down.314; cap.THYAO.breakout.down.314, ex437, fnd.THYAO.breakout.down.314 | — | — | — |
| C16 | A | 1 | ✓ | ✓ | —; — | — | — | — |
| C17 | B | 2 | ✓ | ✓ | —; — | —; — | — | — |
| C18 | C | 1 | ✓ | ✓ | —; — | — | — | — |
| C19 | D | 1 | ✓ | ✓ | —; — | — | — | — |
| C20 | E | 1 | ✓ | ✓ | fnd.THYAO.breakout.down.314; fnd.THYAO.breakout.down.314 | — | — | — |
