# Tek bakış düzeneği — puanlar

> Ajan okuyucu insan değildir; bu ölçüm ön elemedir (SPEC §G0).

| tasarım | tek bakış: kısma kopyası (birincil, F2:H1) | okuma: tam boy | okuma: yarım boy | gri | akış | kanıt | hareket | hareket (yalnız son kare) | BT [%95] | kesilen |
|---|---|---|---|---|---|---|---|---|---|---:|
| `fotolay` | 0.372 [0.316, 0.434] (F 16, H 16) | 0.857 [0.807, 0.903] (F 14, H 12) | 0.852 [0.796, 0.907] (F 12, H 12) | 0.896 [0.771, 1.000] (F 8, H 8) | 1.000 [1.000, 1.000] (F 1, H 1) | 1.000 [1.000, 1.000] (F 2, H 2) | 1.000 [1.000, 1.000] (F 2, H 2) | — | 1.322 [0.775, 2.031] | 0 |
| `kestim` | 0.257 [0.212, 0.302] (F 16, H 16) | 0.882 [0.840, 0.926] (F 14, H 12) | 0.898 [0.852, 0.940] (F 12, H 12) | 0.917 [0.812, 1.000] (F 8, H 8) | 1.000 [1.000, 1.000] (F 1, H 1) | 0.889 [0.778, 1.000] (F 2, H 2) | 1.000 [1.000, 1.000] (F 2, H 2) | — | -2.413 [-2.661, -2.203] | 2 |
| `vadrel` | 0.278 [0.240, 0.316] (F 16, H 16) | 0.893 [0.856, 0.930] (F 14, H 11) | 0.894 [0.843, 0.940] (F 12, H 12) | 0.938 [0.833, 1.000] (F 8, H 8) | 1.000 [1.000, 1.000] (F 1, H 1) | 0.944 [0.889, 1.000] (F 2, H 2) | 1.000 [1.000, 1.000] (F 2, H 2) | — | 1.091 [0.551, 1.737] | 3 |

## Konum çelişkisi (U1)

- `fotolay|kestim`: 0/8 = 0.0
- `fotolay|vadrel`: 2/8 = 0.25
- `kestim|vadrel`: 0/8 = 0.0

Uzunluk yanlılığı (U2): {'spearman_bt_vs_chars': -0.8660254037844387, 'decisive_won_by_more_text': {'share': 0.154, 'n': 13}}

Okuyucu modelleri arası uyum: {'n': 36, 'agree': 0.944, 'models': ['claude-haiku-5-5', 'claude-sonnet-5-5']}

Tek bakış dışı çağrılar (birden çok Read; puanlamadan çıkarıldı): ['4e7b09e9dd.face/H']

## Seçim kuralının uygulanışı

- {'step': 1, 'rule': 'hard gates', 'eligible': ['fotolay', 'vadrel'], 'waived': ['text_contrast', 'nontext_contrast', 'min_font', 'touch', 'colour'], 'eliminated': {}}
- {'step': '1b', 'rule': 'reading non-inferiority vs reference', 'reference': 'vadrel', 'eliminated': [], 'eligible': ['fotolay', 'vadrel']}
- {'step': 2, 'rule': 'primary: glance (throttle copy) accuracy (F2:H1)', 'order': ['fotolay', 'vadrel'], 'top_group': ['fotolay', 'vadrel']}
- {'step': 3, 'rule': 'secondary: critic Bradley-Terry', 'order': ['fotolay', 'vadrel'], 'top_group': ['fotolay', 'vadrel']}
- {'step': 4, 'rule': 'full-size reading accuracy', 'order': ['vadrel', 'fotolay'], 'top_group': ['vadrel', 'fotolay']}

**Karar:** {'winner': None, 'told_apart': False, 'not_told_apart': ['vadrel', 'fotolay'], 'default': 'fotolay', 'top_two': ['fotolay', 'vadrel'], 'ranking_point_estimates': ['fotolay', 'vadrel']}
