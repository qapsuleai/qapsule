# Tek bakış düzeneği — puanlar

> Ajan okuyucu insan değildir; bu ölçüm ön elemedir (SPEC §G0).

| tasarım | birincil: chart (F2:H1) | okuma: tam boy | okuma: yarım boy | gri | akış | kanıt | hareket | hareket (yalnız son kare) | BT [%95] | kesilen |
|---|---|---|---|---|---|---|---|---|---|---:|
| `kestim` | 0.406 [0.377, 0.436] (F 75, H 75) | — | — | — | — | — | — | — | -4.195 [-4.697, -3.821] | 3 |
| `korda` | 0.928 [0.907, 0.948] (F 75, H 75) | — | — | — | — | — | — | — | 1.368 [0.899, 2.014] | 77 |
| `pavka` | 0.987 [0.977, 0.996] (F 75, H 75) | — | — | — | — | — | — | — | 3.822 [3.179, 4.625] | 76 |
| `r4` | 0.411 [0.384, 0.438] (F 75, H 75) | — | — | — | — | — | — | — | -0.248 [-0.907, 0.356] | 2 |
| `selen` | 0.868 [0.842, 0.892] (F 75, H 75) | — | — | — | — | — | — | — | -0.747 [-1.453, -0.218] | 46 |

## Konum çelişkisi (U1)

- `kestim|korda`: 0/9 = 0.0
- `kestim|pavka`: 0/9 = 0.0
- `kestim|r4`: 0/9 = 0.0
- `kestim|selen`: 0/9 = 0.0
- `korda|pavka`: 1/9 = 0.111
- `korda|r4`: 3/9 = 0.333 → ayırt edilemedi
- `korda|selen`: 1/9 = 0.111
- `pavka|r4`: 0/9 = 0.0
- `pavka|selen`: 0/9 = 0.0
- `r4|selen`: 1/9 = 0.111

Uzunluk yanlılığı (U2): {'spearman_bt_vs_chars': 0.7, 'decisive_won_by_more_text': {'share': 0.845, 'n': 84}}

Okuyucu modelleri arası uyum: {'n': 120, 'agree': 0.875, 'models': ['claude-haiku-5-5', 'claude-sonnet-5-5']}

Tek bakış dışı çağrılar (birden çok Read; puanlamadan çıkarıldı): []

## Seçim kuralının uygulanışı

- {'step': 1, 'rule': 'hard gates', 'eligible': ['korda', 'r4', 'selen'], 'waived': ['text_contrast', 'nontext_contrast', 'touch', 'colour'], 'eliminated': {'pavka': ['k52']}}
- {'step': 2, 'rule': 'primary: chart accuracy (F2:H1)', 'order': ['korda', 'selen', 'r4'], 'top_group': ['korda']}

**Karar:** {'winner': 'korda', 'told_apart': True, 'not_told_apart': [], 'default': 'korda', 'top_two': ['korda', 'selen'], 'ranking_point_estimates': ['korda', 'selen', 'r4']}
