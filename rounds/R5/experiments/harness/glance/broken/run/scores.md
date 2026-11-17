# Tek bakış düzeneği — puanlar

> Ajan okuyucu insan değildir; bu ölçüm ön elemedir (SPEC §G0).

| tasarım | tek bakış: kısma kopyası (birincil, F2:H1) | okuma: tam boy | okuma: yarım boy | gri | akış | kanıt | hareket | hareket (yalnız son kare) | BT [%95] | kesilen |
|---|---|---|---|---|---|---|---|---|---|---:|
| `fotolay` | 0.261 [0.243, 0.279] (F 192, H 191) | 0.880 [0.853, 0.906] (F 31, H 32) | — | — | — | — | — | — | 1.630 [1.374, 1.921] | 0 |
| `kestim` | 0.236 [0.222, 0.252] (F 187, H 192) | 0.816 [0.745, 0.882] (F 32, H 32) | — | — | — | — | — | — | -3.097 [-3.212, -2.978] | 12 |
| `vadrel` | 0.314 [0.297, 0.332] (F 190, H 191) | 0.899 [0.872, 0.925] (F 32, H 32) | — | — | — | — | — | — | 1.467 [1.222, 1.728] | 6 |

## Konum çelişkisi (U1)

- `fotolay|kestim`: 0/24 = 0.0
- `fotolay|vadrel`: 11/24 = 0.458 → ayırt edilemedi
- `kestim|vadrel`: 0/24 = 0.0

Uzunluk yanlılığı (U2): {'spearman_bt_vs_chars': -1.0, 'decisive_won_by_more_text': {'share': 0.069, 'n': 58}}

Okuyucu modelleri arası uyum: None

Tek bakış dışı çağrılar (birden çok Read; puanlamadan çıkarıldı): ['062cf016a4.throttle/H', '305e4a2607.throttle/F', '3c21a9f416.throttle/F', '688ff6be23.throttle/F', '8b0cd7a59f.face/F', '9c8811049a.throttle/H', 'c05a7deec7.throttle/F', 'd4a1f31d58.throttle/F', 'e143931476.throttle/F', 'f6f287bc1a.throttle/F']

## Seçim kuralının uygulanışı

- {'step': 1, 'rule': 'hard gates', 'eligible': ['fotolay', 'vadrel'], 'waived': ['nontext_contrast', 'min_font', 'touch', 'colour'], 'eliminated': {}}
- {'step': '1b', 'rule': 'reading non-inferiority vs reference', 'reference': 'vadrel', 'eliminated': [], 'eligible': ['fotolay', 'vadrel']}
- {'step': 2, 'rule': 'primary: glance (throttle copy) accuracy (F2:H1)', 'order': ['vadrel', 'fotolay'], 'top_group': ['vadrel']}

**Karar:** {'winner': 'vadrel', 'told_apart': True, 'not_told_apart': [], 'default': 'vadrel', 'top_two': ['vadrel', 'fotolay'], 'ranking_point_estimates': ['vadrel', 'fotolay']}
