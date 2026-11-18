# Tek bakış düzeneği — puanlar

> Ajan okuyucu insan değildir; bu ölçüm ön elemedir (SPEC §G0).

| tasarım | birincil: throttle (F2:H1) | okuma: tam boy | okuma: yarım boy | gri | akış | kanıt | hareket | hareket (yalnız son kare) | BT [%95] | kesilen |
|---|---|---|---|---|---|---|---|---|---|---:|
| `bertan` | 0.345 [0.319, 0.370] (F 191, H 191) | 0.855 [0.835, 0.874] (F 103, H 104) | 0.842 [0.822, 0.861] (F 96, H 96) | 0.997 [0.990, 1.000] (F 96, H 96) | 0.931 [0.861, 0.986] (F 12, H 12) | — | — | — | 1.303 [0.992, 1.650] | 2 |
| `kestim` | 0.183 [0.171, 0.197] (F 191, H 192) | 0.837 [0.817, 0.855] (F 104, H 104) | 0.754 [0.722, 0.782] (F 95, H 96) | 1.000 [1.000, 1.000] (F 96, H 96) | 0.764 [0.653, 0.875] (F 12, H 12) | — | — | — | -5.291 [-5.738, -4.937] | 21 |
| `kivra` | 0.243 [0.225, 0.260] (F 192, H 192) | 0.775 [0.740, 0.809] (F 104, H 104) | 0.768 [0.732, 0.801] (F 95, H 95) | 0.873 [0.825, 0.922] (F 96, H 96) | 1.000 [1.000, 1.000] (F 12, H 12) | — | — | — | 1.791 [1.466, 2.191] | 5 |
| `oltan` | 0.471 [0.445, 0.497] (F 190, H 192) | 0.865 [0.847, 0.882] (F 104, H 103) | 0.810 [0.786, 0.833] (F 95, H 95) | 1.000 [1.000, 1.000] (F 96, H 96) | 0.840 [0.735, 0.931] (F 11, H 12) | — | — | — | 2.153 [1.822, 2.551] | 11 |
| `r4` | 0.318 [0.295, 0.343] (F 192, H 192) | 0.866 [0.847, 0.884] (F 104, H 104) | 0.849 [0.829, 0.869] (F 96, H 96) | 1.000 [1.000, 1.000] (F 96, H 96) | 0.958 [0.889, 1.000] (F 12, H 12) | — | — | — | -1.198 [-1.639, -0.847] | 8 |
| `zemre` | 0.279 [0.257, 0.301] (F 191, H 191) | 0.862 [0.844, 0.880] (F 104, H 103) | 0.806 [0.786, 0.828] (F 96, H 95) | 1.000 [1.000, 1.000] (F 95, H 96) | 0.889 [0.792, 0.972] (F 12, H 12) | — | — | — | 1.242 [0.953, 1.536] | 3 |

## Konum çelişkisi (U1)

- `bertan|kestim`: 0/24 = 0.0
- `bertan|kivra`: 10/24 = 0.417 → ayırt edilemedi
- `bertan|oltan`: 0/24 = 0.0
- `bertan|r4`: 0/24 = 0.0
- `bertan|zemre`: 6/24 = 0.25
- `kestim|kivra`: 0/24 = 0.0
- `kestim|oltan`: 0/24 = 0.0
- `kestim|r4`: 0/24 = 0.0
- `kestim|zemre`: 0/24 = 0.0
- `kivra|oltan`: 4/24 = 0.167
- `kivra|r4`: 1/24 = 0.042
- `kivra|zemre`: 9/24 = 0.375 → ayırt edilemedi
- `oltan|r4`: 3/24 = 0.125
- `oltan|zemre`: 8/24 = 0.333 → ayırt edilemedi
- `r4|zemre`: 0/24 = 0.0

Uzunluk yanlılığı (U2): {'spearman_bt_vs_chars': -0.02857142857142857, 'decisive_won_by_more_text': {'share': 0.445, 'n': 308}}

Okuyucu modelleri arası uyum: {'n': 576, 'agree': 0.861, 'models': ['claude-haiku-5-5', 'claude-sonnet-5-5']}

Tek bakış dışı çağrılar (birden çok Read; puanlamadan çıkarıldı): ['0005ab853d.half/F', '1d9de2d77a.throttle/F', '33ca08b99e.gray/F', '4b910504b4.face/F', '625bf2ecd2.throttle/F', '6e0bda714e.half/H', '91fa548f94.throttle/H', '92f5745b1c.half/F', '9a7ab29754.half/H', 'a34b6e0cf7.throttle/F', 'b16c0ac953.face/H', 'b216a0c909.half/H', 'b216a0c909.throttle/F', 'c2adca0db4.throttle/H', 'd7239a85b1.feed/F', 'e143931476.half/F', 'eb26bd7331.throttle/F', 'f866ab1a6f.face/H']

## Seçim kuralının uygulanışı

- {'step': 1, 'rule': 'hard gates', 'eligible': ['kivra', 'oltan', 'zemre'], 'waived': [], 'eliminated': {'bertan': ['colour'], 'r4': ['text_contrast', 'nontext_contrast', 'min_font', 'touch', 'colour']}}
- {'step': '1b', 'rule': 'reading non-inferiority vs reference', 'reference': 'r4', 'eliminated': ['kivra'], 'eligible': ['oltan', 'zemre']}
- {'step': 2, 'rule': 'primary: throttle accuracy (F2:H1)', 'order': ['oltan', 'zemre'], 'top_group': ['oltan']}

**Karar:** {'winner': 'oltan', 'told_apart': True, 'not_told_apart': [], 'default': 'oltan', 'top_two': ['oltan', 'zemre'], 'ranking_point_estimates': ['oltan', 'zemre']}
