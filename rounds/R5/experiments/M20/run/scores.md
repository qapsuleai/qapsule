# Tek bakış düzeneği — puanlar

> Ajan okuyucu insan değildir; bu ölçüm ön elemedir (SPEC §G0).

| tasarım | birincil: cap_evidence,cap_evidence_visitor (F2:H1) | okuma: tam boy | okuma: yarım boy | gri | akış | kanıt | hareket | hareket (yalnız son kare) | BT [%95] | kesilen |
|---|---|---|---|---|---|---|---|---|---|---:|
| `kestim` | 0.726 [0.673, 0.777] (F 50, H 50) | 0.972 [0.947, 0.993] (F 24, H 24) | — | — | — | 0.972 [0.949, 0.993] (F 24, H 24) | — | — | -3.406 [-3.803, -2.801] | 4 |
| `nobra` | 0.848 [0.812, 0.884] (F 50, H 49) | 0.975 [0.954, 0.993] (F 24, H 24) | — | — | — | 0.975 [0.954, 0.993] (F 24, H 24) | — | — | 2.413 [1.803, 3.351] | 12 |
| `r4` | 0.732 [0.679, 0.784] (F 50, H 50) | 0.972 [0.954, 0.991] (F 24, H 24) | — | — | — | 0.972 [0.954, 0.991] (F 24, H 24) | — | — | -0.817 [-1.229, -0.456] | 4 |
| `tesir` | 0.968 [0.949, 0.987] (F 50, H 50) | 0.952 [0.915, 0.981] (F 24, H 23) | — | — | — | 0.952 [0.914, 0.981] (F 24, H 23) | — | — | 1.810 [1.195, 2.463] | 5 |

## Konum çelişkisi (U1)

- `kestim|nobra`: 0/6 = 0.0
- `kestim|r4`: 0/6 = 0.0
- `kestim|tesir`: 0/6 = 0.0
- `nobra|r4`: 0/6 = 0.0
- `nobra|tesir`: 2/6 = 0.333 → ayırt edilemedi
- `r4|tesir`: 0/6 = 0.0

Uzunluk yanlılığı (U2): {'spearman_bt_vs_chars': 1.0, 'decisive_won_by_more_text': {'share': 0.971, 'n': 34}}

Okuyucu modelleri arası uyum: {'n': 40, 'agree': 0.925, 'models': ['claude-haiku-5-5', 'claude-sonnet-5-5']}

Tek bakış dışı çağrılar (birden çok Read; puanlamadan çıkarıldı): ['1870d6485e.throttle/H', '22d506fe73.throttle/H', '68a32d61b3.throttle/H', '83b3ac49fc.evidence/H', 'a4c14b530e.throttle/F', 'bd333e8e32.capsule/H']

## Seçim kuralının uygulanışı

- {'step': 1, 'rule': 'hard gates', 'eligible': ['nobra', 'r4', 'tesir'], 'waived': ['text_contrast', 'nontext_contrast', 'touch', 'colour'], 'eliminated': {}}
- {'step': '1b', 'rule': 'reading non-inferiority vs reference', 'reference': 'r4', 'eliminated': [], 'eligible': ['nobra', 'r4', 'tesir']}
- {'step': 2, 'rule': 'primary: cap_evidence,cap_evidence_visitor accuracy (F2:H1)', 'order': ['tesir', 'nobra', 'r4'], 'top_group': ['tesir']}

**Karar:** {'winner': 'tesir', 'told_apart': True, 'not_told_apart': [], 'default': 'tesir', 'top_two': ['tesir', 'nobra'], 'ranking_point_estimates': ['tesir', 'nobra', 'r4']}
