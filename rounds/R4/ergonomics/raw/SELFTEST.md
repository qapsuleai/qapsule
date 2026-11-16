| Run | First try | First try id-free | Final | Final id-free | First try had an error | Attempts | Unnumbered runs | Msgs (first / all) | Tokens | Output tokens | Agent time (s) | Cost |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| selftest-ref | 20/20 | 10/20 | 20/20 | 10/20 (lenient final 20) | 0 | 20 | 20 | 39 / 39 | 0 | 0 | 0.0 | 0.00 $ |
| selftest-empty | 0/20 | 0/20 | 0/20 | 0/20 (lenient final 0) | 0 | 20 | 0 | 20 / 20 | 0 | 0 | 0.0 | 0.00 $ |

| Task | selftest-ref first / final (att) id-free | selftest-empty first / final (att) id-free |
|---|---|---|
| C01 | ✓ / ✓ (1) id-free | ✗ / ✗ (1)  |
| C02 | ✓ / ✓ (1) ids | ✗ / ✗ (1)  |
| C03 | ✓ / ✓ (1) ids | ✗ / ✗ (1)  |
| C04 | ✓ / ✓ (1) ids | ✗ / ✗ (1)  |
| C05 | ✓ / ✓ (1) id-free | ✗ / ✗ (1)  |
| C06 | ✓ / ✓ (1) id-free | ✗ / ✗ (1)  |
| C07 | ✓ / ✓ (1) id-free | ✗ / ✗ (1)  |
| C08 | ✓ / ✓ (1) id-free | ✗ / ✗ (1)  |
| C09 | ✓ / ✓ (1) ids | ✗ / ✗ (1)  |
| C10 | ✓ / ✓ (1) ids | ✗ / ✗ (1)  |
| C11 | ✓ / ✓ (1) id-free | ✗ / ✗ (1)  |
| C12 | ✓ / ✓ (1) id-free | ✗ / ✗ (1)  |
| C13 | ✓ / ✓ (1) ids | ✗ / ✗ (1)  |
| C14 | ✓ / ✓ (1) ids | ✗ / ✗ (1)  |
| C15 | ✓ / ✓ (1) ids | ✗ / ✗ (1)  |
| C16 | ✓ / ✓ (1) ids | ✗ / ✗ (1)  |
| C17 | ✓ / ✓ (1) id-free | ✗ / ✗ (1)  |
| C18 | ✓ / ✓ (1) id-free | ✗ / ✗ (1)  |
| C19 | ✓ / ✓ (1) id-free | ✗ / ✗ (1)  |
| C20 | ✓ / ✓ (1) ids | ✗ / ✗ (1)  |

### selftest-ref

| Task | Agent | Att | First | Final | Ids used (first; last) | Errors (codes, per attempt) | Why (first attempt) | Why (last attempt) |
|---|---|---|---|---|---|---|---|---|
| C01 | A | 1 | ✓ | ✓ | —; — | — | — | — |
| C02 | B | 1 | ✓ | ✓ | cap.THYAO.breakout.down.313; cap.THYAO.breakout.down.313 | — | — | — |
| C03 | C | 1 | ✓ | ✓ | cap.ASELS.breakout.up.313; cap.ASELS.breakout.up.313 | — | — | — |
| C04 | D | 1 | ✓ | ✓ | fnd.THYAO.breakout.down.310; fnd.THYAO.breakout.down.310 | — | — | — |
| C05 | E | 1 | ✓ | ✓ | —; — | — | — | — |
| C06 | A | 1 | ✓ | ✓ | —; — | — | — | — |
| C07 | B | 1 | ✓ | ✓ | —; — | — | — | — |
| C08 | C | 1 | ✓ | ✓ | —; — | — | — | — |
| C09 | D | 1 | ✓ | ✓ | fnd.THYAO.extreme.down.308; fnd.THYAO.extreme.down.308 | — | — | — |
| C10 | E | 1 | ✓ | ✓ | fnd.THYAO.breakout.down.310; fnd.THYAO.breakout.down.310 | — | — | — |
| C11 | A | 1 | ✓ | ✓ | —; — | — | — | — |
| C12 | B | 1 | ✓ | ✓ | —; — | — | — | — |
| C13 | C | 1 | ✓ | ✓ | cap.THYAO.breakout.down.310; cap.THYAO.breakout.down.310 | — | — | — |
| C14 | D | 1 | ✓ | ✓ | fnd.THYAO.breakout.down.310; fnd.THYAO.breakout.down.310 | — | — | — |
| C15 | E | 1 | ✓ | ✓ | cap.THYAO.breakout.down.310, fnd.THYAO.breakout.down.310; cap.THYAO.breakout.down.310, fnd.THYAO.breakout.down.310 | — | — | — |
| C16 | A | 1 | ✓ | ✓ | cap.THYAO.squeeze.neutral.138; cap.THYAO.squeeze.neutral.138 | — | — | — |
| C17 | B | 1 | ✓ | ✓ | —; — | — | — | — |
| C18 | C | 1 | ✓ | ✓ | —; — | — | — | — |
| C19 | D | 1 | ✓ | ✓ | —; — | — | — | — |
| C20 | E | 1 | ✓ | ✓ | fnd.THYAO.breakout.down.310; fnd.THYAO.breakout.down.310 | — | — | — |

### selftest-empty

| Task | Agent | Att | First | Final | Ids used (first; last) | Errors (codes, per attempt) | Why (first attempt) | Why (last attempt) |
|---|---|---|---|---|---|---|---|---|
| C01 | A | 1 | ✗ | ✗ | —; — | — | a capsules query whose answer holds every open THYAO capsule; answer names every open THYAO capsule (missing: ['cap.THYAO.extreme.down.308', 'cap.THYAO.breakout.down.310', 'cap.THYAO.continuation.down.309']) | a capsules query whose answer holds every open THYAO capsule; answer names every open THYAO capsule (missing: ['cap.THYAO.extreme.down.308', 'cap.THYAO.breakout.down.310', 'cap.THYAO.continuation.down.309']) |
| C02 | B | 1 | ✗ | ✗ | —; — | — | capsule query on the open THYAO breakout with evidence values returned | capsule query on the open THYAO breakout with evidence values returned |
| C03 | C | 1 | ✗ | ✗ | —; — | — | capsule query on cap.ASELS.breakout.up.311; answer names the invalidation; answer gives the reason (None) | capsule query on cap.ASELS.breakout.up.311; answer names the invalidation; answer gives the reason (None) |
| C04 | D | 1 | ✗ | ✗ | —; — | — | reject_finding on the open THYAO breakout's finding, in the log (applied or proposed) | reject_finding on the open THYAO breakout's finding, in the log (applied or proposed) |
| C05 | E | 1 | ✗ | ✗ | —; — | — | a capsules query; answer names the three most important capsules in order (all stories or open ones) | a capsules query; answer names the three most important capsules in order (all stories or open ones) |
| C06 | A | 1 | ✗ | ✗ | —; — | — | a ledger query; answer gives n=73 and realized=42 | a ledger query; answer gives n=73 and realized=42 |
| C07 | B | 1 | ✗ | ✗ | —; — | — | editor's sensitivity important (got None) | editor's sensitivity important (got None) |
| C08 | C | 1 | ✗ | ✗ | —; — | — | ASELS in editor's watch list (got None) | ASELS in editor's watch list (got None) |
| C09 | D | 1 | ✗ | ✗ | —; — | — | present of the open THYAO extreme capsule at capsule depth | present of the open THYAO extreme capsule at capsule depth |
| C10 | E | 1 | ✗ | ✗ | —; — | — | present of the open THYAO breakout at chart depth; answer says the broken level is not drawn | present of the open THYAO breakout at chart depth; answer says the broken level is not drawn |
| C11 | A | 1 | ✗ | ✗ | —; — | — | a capsule / capsules / verify_evidence query as viewer; answer names the entitlement it needs (flow/…) | a capsule / capsules / verify_evidence query as viewer; answer names the entitlement it needs (flow/…) |
| C12 | B | 1 | ✗ | ✗ | —; — | — | subscribe to the capsules topic, instrument THYAO (acked) | subscribe to the capsules topic, instrument THYAO (acked) |
| C13 | C | 1 | ✗ | ✗ | —; — | — | capsule query on cap.THYAO.breakout.down.308 | capsule query on cap.THYAO.breakout.down.308 |
| C14 | D | 1 | ✗ | ✗ | —; — | — | verify_evidence on the open THYAO breakout's fact (or its finding) | verify_evidence on the open THYAO breakout's fact (or its finding) |
| C15 | E | 1 | ✗ | ✗ | —; — | — | dispute on the open THYAO breakout capsule, decided (rejected/upheld/pending) | dispute on the open THYAO breakout capsule, decided (rejected/upheld/pending) |
| C16 | A | 1 | ✗ | ✗ | —; — | — | a capsule/capsules query that returned the THYAO squeeze capsule; answer says no longer valid | a capsule/capsules query that returned the THYAO squeeze capsule; answer says no longer valid |
| C17 | B | 1 | ✗ | ✗ | —; — | — | a capsules query; answer counts per kind = {'breakout': 4, 'flow': 1, 'extreme': 1, 'continuation': 1} (or {'breakout': 5, 'flow': 1, 'extreme': 1, 'continuation': 1}) | a capsules query; answer counts per kind = {'breakout': 4, 'flow': 1, 'extreme': 1, 'continuation': 1} (or {'breakout': 5, 'flow': 1, 'extreme': 1, 'continuation': 1}) |
| C18 | C | 1 | ✗ | ✗ | —; — | — | editor's position THYAO = 100 (got None) | editor's position THYAO = 100 (got None) |
| C19 | D | 1 | ✗ | ✗ | —; — | — | capsules query with stages [realized] | capsules query with stages [realized] |
| C20 | E | 1 | ✗ | ✗ | —; — | — | a PNG was drawn (run.sh --png) | a PNG was drawn (run.sh --png) |

| Run | First try | First try id-free | Final | Final id-free | First try had an error | Attempts | Unnumbered runs | Msgs (first / all) | Tokens | Output tokens | Agent time (s) | Cost |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| selftest-sel | 20/20 | 18/20 | 20/20 | 18/20 (lenient final 20) | 0 | 20 | 20 | 39 / 39 | 0 | 0 | 0.0 | 0.00 $ |

| Task | selftest-sel first / final (att) id-free |
|---|---|
| C01 | ✓ / ✓ (1) id-free |
| C02 | ✓ / ✓ (1) id-free |
| C03 | ✓ / ✓ (1) id-free |
| C04 | ✓ / ✓ (1) id-free |
| C05 | ✓ / ✓ (1) id-free |
| C06 | ✓ / ✓ (1) id-free |
| C07 | ✓ / ✓ (1) id-free |
| C08 | ✓ / ✓ (1) id-free |
| C09 | ✓ / ✓ (1) id-free |
| C10 | ✓ / ✓ (1) id-free |
| C11 | ✓ / ✓ (1) id-free |
| C12 | ✓ / ✓ (1) id-free |
| C13 | ✓ / ✓ (1) id-free |
| C14 | ✓ / ✓ (1) id-free |
| C15 | ✓ / ✓ (1) ids |
| C16 | ✓ / ✓ (1) id-free |
| C17 | ✓ / ✓ (1) id-free |
| C18 | ✓ / ✓ (1) id-free |
| C19 | ✓ / ✓ (1) id-free |
| C20 | ✓ / ✓ (1) ids |

### selftest-sel

| Task | Agent | Att | First | Final | Ids used (first; last) | Errors (codes, per attempt) | Why (first attempt) | Why (last attempt) |
|---|---|---|---|---|---|---|---|---|
| C01 | A | 1 | ✓ | ✓ | —; — | — | — | — |
| C02 | B | 1 | ✓ | ✓ | —; — | — | — | — |
| C03 | C | 1 | ✓ | ✓ | —; — | — | — | — |
| C04 | D | 1 | ✓ | ✓ | —; — | — | — | — |
| C05 | E | 1 | ✓ | ✓ | —; — | — | — | — |
| C06 | A | 1 | ✓ | ✓ | —; — | — | — | — |
| C07 | B | 1 | ✓ | ✓ | —; — | — | — | — |
| C08 | C | 1 | ✓ | ✓ | —; — | — | — | — |
| C09 | D | 1 | ✓ | ✓ | —; — | — | — | — |
| C10 | E | 1 | ✓ | ✓ | —; — | — | — | — |
| C11 | A | 1 | ✓ | ✓ | —; — | — | — | — |
| C12 | B | 1 | ✓ | ✓ | —; — | — | — | — |
| C13 | C | 1 | ✓ | ✓ | —; — | — | — | — |
| C14 | D | 1 | ✓ | ✓ | —; — | — | — | — |
| C15 | E | 1 | ✓ | ✓ | cap.THYAO.breakout.down.313, fnd.THYAO.breakout.down.313; cap.THYAO.breakout.down.313, fnd.THYAO.breakout.down.313 | — | — | — |
| C16 | A | 1 | ✓ | ✓ | —; — | — | — | — |
| C17 | B | 1 | ✓ | ✓ | —; — | — | — | — |
| C18 | C | 1 | ✓ | ✓ | —; — | — | — | — |
| C19 | D | 1 | ✓ | ✓ | —; — | — | — | — |
| C20 | E | 1 | ✓ | ✓ | fnd.THYAO.breakout.down.313; fnd.THYAO.breakout.down.313 | — | — | — |
