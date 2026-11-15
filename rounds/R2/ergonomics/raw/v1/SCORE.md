| Variant | First try | First try w/o document ids | Final | Recovered (failed first → final) | First try had an error → final | Attempts | Msgs (first / all) | Tokens | Output tokens | Agent wall time (s) |
|---|---|---|---|---|---|---|---|---|---|---|
| base | 7/20 | 2/20 | 20/20 | 13/13 | 2/2 | 34 | 66 / 107 | 3,189,676 | 47,124 | 387.3 |
| A | 19/20 | 19/20 | 19/20 | 0/1 | 0/0 | 20 | 55 / 55 | 2,506,378 | 36,482 | 292.8 |
| B | 17/20 | 10/20 | 20/20 | 3/3 | 1/1 | 25 | 53 / 67 | 2,642,064 | 36,264 | 301.7 |

| Task | base first / final (att) | A first / final (att) | B first / final (att) |
|---|---|---|---|
| R01 | ✗ / ✓ (3) | ✓ / ✓ (1) | ✓ / ✓ (1) |
| R02 | ✗ / ✓ (2) | ✗ / ✗ (1) | ✗ / ✓ (2) |
| R03 | ✗ / ✓ (2) | ✓ / ✓ (1) | ✓ / ✓ (1) |
| R04 | ✗ / ✓ (2) | ✓ / ✓ (1) | ✓ / ✓ (1) |
| R05 | ✗ / ✓ (2) | ✓ / ✓ (1) | ✗ / ✓ (2) |
| R06 | ✓ / ✓ (1) | ✓ / ✓ (1) | ✓ / ✓ (1) |
| R07 | ✗ / ✓ (2) | ✓ / ✓ (1) | ✓ / ✓ (1) |
| R08 | ✗ / ✓ (2) | ✓ / ✓ (1) | ✓ / ✓ (2) |
| R09 | ✗ / ✓ (2) | ✓ / ✓ (1) | ✓ / ✓ (1) |
| R10 | ✓ / ✓ (1) | ✓ / ✓ (1) | ✓ / ✓ (1) |
| R11 | ✓ / ✓ (1) | ✓ / ✓ (1) | ✓ / ✓ (1) |
| R12 | ✗ / ✓ (2) | ✓ / ✓ (1) | ✓ / ✓ (1) |
| R13 | ✓ / ✓ (1) | ✓ / ✓ (1) | ✓ / ✓ (1) |
| R14 | ✓ / ✓ (1) | ✓ / ✓ (1) | ✓ / ✓ (1) |
| R15 | ✓ / ✓ (1) | ✓ / ✓ (1) | ✓ / ✓ (1) |
| R16 | ✓ / ✓ (1) | ✓ / ✓ (1) | ✓ / ✓ (1) |
| R17 | ✗ / ✓ (2) | ✓ / ✓ (1) | ✗ / ✓ (2) |
| R18 | ✗ / ✓ (2) | ✓ / ✓ (1) | ✓ / ✓ (2) |
| R19 | ✗ / ✓ (2) | ✓ / ✓ (1) | ✓ / ✓ (1) |
| R20 | ✗ / ✓ (2) | ✓ / ✓ (1) | ✓ / ✓ (1) |

### base

| Task | Agent | Att | First | Final | Msgs first/all | Tokens | Wall (s) | Errors (codes, per attempt) | Why (first attempt) | Why (last attempt) |
|---|---|---|---|---|---|---|---|---|---|---|
| R01 | A | 3 | ✗ | ✓ | 5/12 | 201,584 | 23.7 | —; —; — | [re-run after editing; first run scored, last run counts for final] an RSI(7) series on the price series; an RSI(7) line layer in a NEW panel; that panel is directly below the price panel (2nd in panel order) | [re-run after editing; first run scored, last run counts for final]  |
| R02 | B | 2 | ✗ | ✓ | 4/7 | 170,969 | 17.4 | —; — | a layer drawing Bollinger upper in red in the price panel | — |
| R03 | C | 2 | ✗ | ✓ | 6/9 | 120,473 | 18.1 | —; — | [identical file run from R13/runs/01-attempt1.jsonl] a persistent emphasis targeting the new breakout (bar 185) | — |
| R04 | D | 2 | ✗ | ✓ | 3/5 | 128,867 | 18.7 | —; — | a sentence-depth presentation of the new breakout finding; answer contains the presentation text (capsule headline) | — |
| R05 | E | 2 | ✗ | ✓ | 4/7 | 175,526 | 19.0 | not_found; — | price panel y scale is log | — |
| R06 | A | 1 | ✓ | ✓ | 3/3 | 201,584 | 23.7 | — | — | — |
| R07 | B | 2 | ✗ | ✓ | 0/3 | 170,969 | 17.4 | —; — | [never run] never run | — |
| R08 | C | 2 | ✗ | ✓ | 6/9 | 120,473 | 18.1 | —; — | [identical file run from R13/runs/01-attempt1.jsonl] a bar layer drawing volume (v) of the price series; in a panel other than the price panel; that panel is directly below the price panel | — |
| R09 | D | 2 | ✗ | ✓ | 3/6 | 128,867 | 18.7 | —; — | a band/area layer filling Bollinger upper..lower with opacity < 1 | — |
| R10 | E | 1 | ✓ | ✓ | 3/3 | 175,526 | 19.0 | — | — | — |
| R11 | A | 1 | ✓ | ✓ | 3/3 | 201,584 | 23.7 | — | — | — |
| R12 | B | 2 | ✗ | ✓ | 0/3 | 170,969 | 17.4 | —; — | [never run] never run | — |
| R13 | C | 1 | ✓ | ✓ | 6/6 | 120,473 | 18.1 | — | — | — |
| R14 | D | 1 | ✓ | ✓ | 2/2 | 128,867 | 18.7 | — | — | — |
| R15 | E | 1 | ✓ | ✓ | 3/3 | 175,526 | 19.0 | — | — | — |
| R16 | A | 1 | ✓ | ✓ | 3/3 | 201,584 | 23.7 | — | — | — |
| R17 | B | 2 | ✗ | ✓ | 0/3 | 170,969 | 17.4 | —; — | [never run] never run | — |
| R18 | C | 2 | ✗ | ✓ | 6/9 | 120,473 | 18.1 | —; — | [identical file run from R13/runs/01-attempt1.jsonl] panel order: price, MACD, RSI | — |
| R19 | D | 2 | ✗ | ✓ | 3/5 | 128,867 | 18.7 | —; — | a capsule-depth presentation of the new breakout finding | — |
| R20 | E | 2 | ✗ | ✓ | 3/6 | 175,526 | 19.0 | schema; — | a bandwidth line layer in a new panel (not the price panel) | — |

### A

| Task | Agent | Att | First | Final | Msgs first/all | Tokens | Wall (s) | Errors (codes, per attempt) | Why (first attempt) | Why (last attempt) |
|---|---|---|---|---|---|---|---|---|---|---|
| R01 | A | 1 | ✓ | ✓ | 4/4 | 121,962 | 14.7 | — | — | — |
| R02 | B | 1 | ✗ | ✗ | 4/4 | 122,374 | 15.7 | — | Bollinger middle still drawn and not red; Bollinger lower still drawn and not red | Bollinger middle still drawn and not red; Bollinger lower still drawn and not red |
| R03 | C | 1 | ✓ | ✓ | 2/2 | 118,424 | 13.1 | — | — | — |
| R04 | D | 1 | ✓ | ✓ | 2/2 | 120,160 | 14.6 | — | — | — |
| R05 | E | 1 | ✓ | ✓ | 2/2 | 143,675 | 15.1 | — | — | — |
| R06 | A | 1 | ✓ | ✓ | 4/4 | 121,962 | 14.7 | — | — | — |
| R07 | B | 1 | ✓ | ✓ | 3/3 | 122,374 | 15.7 | — | — | — |
| R08 | C | 1 | ✓ | ✓ | 2/2 | 118,424 | 13.1 | — | — | — |
| R09 | D | 1 | ✓ | ✓ | 3/3 | 120,160 | 14.6 | — | — | — |
| R10 | E | 1 | ✓ | ✓ | 2/2 | 143,675 | 15.1 | — | — | — |
| R11 | A | 1 | ✓ | ✓ | 4/4 | 121,962 | 14.7 | — | — | — |
| R12 | B | 1 | ✓ | ✓ | 4/4 | 122,374 | 15.7 | — | — | — |
| R13 | C | 1 | ✓ | ✓ | 2/2 | 118,424 | 13.1 | — | — | — |
| R14 | D | 1 | ✓ | ✓ | 2/2 | 120,160 | 14.6 | — | — | — |
| R15 | E | 1 | ✓ | ✓ | 2/2 | 143,675 | 15.1 | — | — | — |
| R16 | A | 1 | ✓ | ✓ | 4/4 | 121,962 | 14.7 | — | — | — |
| R17 | B | 1 | ✓ | ✓ | 3/3 | 122,374 | 15.7 | — | — | — |
| R18 | C | 1 | ✓ | ✓ | 2/2 | 118,424 | 13.1 | — | — | — |
| R19 | D | 1 | ✓ | ✓ | 2/2 | 120,160 | 14.6 | — | — | — |
| R20 | E | 1 | ✓ | ✓ | 2/2 | 143,675 | 15.1 | — | — | — |

### B

| Task | Agent | Att | First | Final | Msgs first/all | Tokens | Wall (s) | Errors (codes, per attempt) | Why (first attempt) | Why (last attempt) |
|---|---|---|---|---|---|---|---|---|---|---|
| R01 | A | 1 | ✓ | ✓ | 3/3 | 164,018 | 15.6 | — | — | — |
| R02 | B | 2 | ✗ | ✓ | 3/6 | 122,656 | 16.6 | —; — | Bollinger middle still drawn and not red; Bollinger lower still drawn and not red | — |
| R03 | C | 1 | ✓ | ✓ | 3/3 | 157,967 | 17.0 | — | — | — |
| R04 | D | 1 | ✓ | ✓ | 2/2 | 103,996 | 13.1 | — | — | — |
| R05 | E | 2 | ✗ | ✓ | 2/4 | 111,879 | 13.1 | —; — | price panel y scale is log | — |
| R06 | A | 1 | ✓ | ✓ | 3/3 | 164,018 | 15.6 | — | — | — |
| R07 | B | 1 | ✓ | ✓ | 3/3 | 122,656 | 16.6 | — | — | — |
| R08 | C | 2 | ✓ | ✓ | 3/6 | 157,967 | 17.0 | —; — | — | — |
| R09 | D | 1 | ✓ | ✓ | 3/3 | 103,996 | 13.1 | — | — | — |
| R10 | E | 1 | ✓ | ✓ | 2/2 | 111,879 | 13.1 | — | — | — |
| R11 | A | 1 | ✓ | ✓ | 3/3 | 164,018 | 15.6 | — | — | — |
| R12 | B | 1 | ✓ | ✓ | 4/4 | 122,656 | 16.6 | — | — | — |
| R13 | C | 1 | ✓ | ✓ | 2/2 | 157,967 | 17.0 | — | — | — |
| R14 | D | 1 | ✓ | ✓ | 2/2 | 103,996 | 13.1 | — | — | — |
| R15 | E | 1 | ✓ | ✓ | 2/2 | 111,879 | 13.1 | — | — | — |
| R16 | A | 1 | ✓ | ✓ | 3/3 | 164,018 | 15.6 | — | — | — |
| R17 | B | 2 | ✗ | ✓ | 3/6 | 122,656 | 16.6 | no_origin; — | a rule bound to a fact with level 123.45, or the user is told why not and the option | — |
| R18 | C | 2 | ✓ | ✓ | 3/6 | 157,967 | 17.0 | —; — | — | — |
| R19 | D | 1 | ✓ | ✓ | 2/2 | 103,996 | 13.1 | — | — | — |
| R20 | E | 1 | ✓ | ✓ | 2/2 | 111,879 | 13.1 | — | — | — |
