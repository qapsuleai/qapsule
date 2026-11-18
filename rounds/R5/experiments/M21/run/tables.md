## Okuyucu ölçüleri (haiku, F2:H1, %95 önyükleme)

| tasarım | süreklilik (birincil; geçerli sorular) | geçerli sorular | `motion` hepsi | `motion_final` | fark edilme (`notice`, mid+after) | yalnız mid | yalnız after | çift (önce|mid) |
|---|---|---|---|---|---|---|---|---|
| `surek` | — | yok | 1.000 [1.000, 1.000] | 0.986 [0.965, 1.000] | 0.297 [0.259, 0.336] | 0.339 [0.278, 0.397] | 0.256 [0.200, 0.307] | 0.888 [0.832, 0.936] |
| `nabiz` | — | yok | 0.860 [0.787, 0.927] | 1.000 [1.000, 1.000] | 0.515 [0.471, 0.559] | 0.565 [0.505, 0.623] | 0.465 [0.407, 0.523] | 0.917 [0.866, 0.958] |
| `r4` | — | yok | 1.000 [1.000, 1.000] | 0.986 [0.965, 1.000] | 0.265 [0.226, 0.304] | 0.262 [0.208, 0.317] | 0.269 [0.215, 0.326] | 0.738 [0.662, 0.810] |
| `still` | 0.000 [0.000, 0.000] | m_same | 0.000 [0.000, 0.000] | 0.000 [0.000, 0.000] | 0.265 [0.228, 0.302] | 0.271 [0.215, 0.324] | 0.259 [0.208, 0.312] | 0.729 [0.650, 0.804] |
| `kestim` | — | yok | 0.907 [0.847, 0.960] | 0.944 [0.895, 0.979] | 0.310 [0.271, 0.350] | 0.331 [0.275, 0.389] | 0.289 [0.234, 0.347] | 0.824 [0.760, 0.881] |
| `surek-az` | — | yok | — | — | 0.575 [0.527, 0.615] | 0.580 [0.520, 0.640] | 0.569 [0.512, 0.627] | 0.881 [0.826, 0.933] |
| `nabiz-az` | — | yok | — | — | 0.440 [0.397, 0.484] | 0.442 [0.384, 0.505] | 0.438 [0.377, 0.500] | 0.893 [0.834, 0.944] |

## Geçerlilik sınaması (son kareyle; Wilson %95)

| tasarım | soru | n | son kareyle doğruluk [aralık] | şans | tek yönlü geçerli | lafzî (aralık şansı içerir) |
|---|---|---|---|---|---|---|
| `surek` | `m_same` | 96 | 0.979 [0.927, 0.994] | 0.5 | hayır | hayır |
| `surek` | `m_where` | 0 | anahtar yok (puanlanmadı) | | | |
| `nabiz` | `m_same` | 96 | 1.000 [0.962, 1.000] | 0.5 | hayır | hayır |
| `nabiz` | `m_where` | 0 | anahtar yok (puanlanmadı) | | | |
| `r4` | `m_same` | 96 | 0.979 [0.927, 0.994] | 0.5 | hayır | hayır |
| `r4` | `m_where` | 0 | anahtar yok (puanlanmadı) | | | |
| `still` | `m_same` | 96 | 0.000 [0.000, 0.038] | 0.5 | evet | hayır |
| `still` | `m_where` | 0 | anahtar yok (puanlanmadı) | | | |
| `kestim` | `m_same` | 95 | 0.926 [0.856, 0.964] | 0.5 | hayır | hayır |
| `kestim` | `m_where` | 0 | anahtar yok (puanlanmadı) | | | |
| `surek-az` | `m_same` | 0 | anahtar yok (puanlanmadı) | | | |
| `surek-az` | `m_where` | 0 | anahtar yok (puanlanmadı) | | | |
| `nabiz-az` | `m_same` | 0 | anahtar yok (puanlanmadı) | | | |
| `nabiz-az` | `m_where` | 0 | anahtar yok (puanlanmadı) | | | |

## Payları

| tasarım | ara karede "aynı nesne taşınıyor" | son karede "aynı nesne taşınıyor" | değişimde "değişen yok" deme | denetimde yanlış alarm |
|---|---|---|---|---|
| `surek` | 1.0 | 0.979 | 0.257 | 0.786 |
| `nabiz` | 0.85 | 1.0 | 0.078 | 0.771 |
| `r4` | 1.0 | 0.979 | 0.294 | 0.792 |
| `still` | 0.98 | 0.979 | 0.263 | 0.771 |
| `kestim` | 0.89 | 0.926 | 0.37 | 0.635 |
| `surek-az` | None | None | 0.078 | 0.724 |
| `nabiz-az` | None | None | 0.109 | 0.766 |

## Soru başına (Wilson)

| tasarım | küme/soru/persona | p [aralık] | n |
|---|---|---|---|
| `surek` | motion/m_same/F | 1.0 [0.929, 1.0] | 50 |
| `surek` | motion/m_same/H | 1.0 [0.929, 1.0] | 50 |
| `surek` | motion_final/m_same/F | 1.0 [0.926, 1.0] | 48 |
| `surek` | motion_final/m_same/H | 0.958 [0.86, 0.988] | 48 |
| `surek` | notice/n_where/F | 0.296 [0.246, 0.351] | 287 |
| `surek` | notice/n_where/H | 0.3 [0.25, 0.355] | 287 |
| `surek` | notice_pair/n_what/F | 0.917 [0.83, 0.961] | 72 |
| `surek` | notice_pair/n_what/H | 0.833 [0.731, 0.902] | 72 |
| `surek` | notice_pair/n_where2/F | 0.917 [0.83, 0.961] | 72 |
| `surek` | notice_pair/n_where2/H | 0.819 [0.715, 0.891] | 72 |
| `nabiz` | motion/m_same/F | 0.88 [0.762, 0.944] | 50 |
| `nabiz` | motion/m_same/H | 0.82 [0.692, 0.902] | 50 |
| `nabiz` | motion_final/m_same/F | 1.0 [0.926, 1.0] | 48 |
| `nabiz` | motion_final/m_same/H | 1.0 [0.926, 1.0] | 48 |
| `nabiz` | notice/n_where/F | 0.524 [0.467, 0.581] | 288 |
| `nabiz` | notice/n_where/H | 0.497 [0.439, 0.554] | 288 |
| `nabiz` | notice_pair/n_what/F | 0.931 [0.848, 0.97] | 72 |
| `nabiz` | notice_pair/n_what/H | 0.889 [0.796, 0.943] | 72 |
| `nabiz` | notice_pair/n_where2/F | 0.931 [0.848, 0.97] | 72 |
| `nabiz` | notice_pair/n_where2/H | 0.889 [0.796, 0.943] | 72 |
| `r4` | motion/m_same/F | 1.0 [0.929, 1.0] | 50 |
| `r4` | motion/m_same/H | 1.0 [0.929, 1.0] | 50 |
| `r4` | motion_final/m_same/F | 1.0 [0.926, 1.0] | 48 |
| `r4` | motion_final/m_same/H | 0.958 [0.86, 0.988] | 48 |
| `r4` | notice/n_where/F | 0.257 [0.21, 0.31] | 288 |
| `r4` | notice/n_where/H | 0.281 [0.232, 0.336] | 288 |
| `r4` | notice_pair/n_what/F | 0.75 [0.639, 0.836] | 72 |
| `r4` | notice_pair/n_what/H | 0.667 [0.552, 0.765] | 72 |
| `r4` | notice_pair/n_where2/F | 0.778 [0.669, 0.858] | 72 |
| `r4` | notice_pair/n_where2/H | 0.708 [0.595, 0.801] | 72 |
| `still` | motion/m_same/F | 0.0 [0.0, 0.071] | 50 |
| `still` | motion/m_same/H | 0.0 [0.0, 0.071] | 50 |
| `still` | motion_final/m_same/F | 0.0 [0.0, 0.074] | 48 |
| `still` | motion_final/m_same/H | 0.0 [0.0, 0.074] | 48 |
| `still` | notice/n_where/F | 0.271 [0.223, 0.325] | 288 |
| `still` | notice/n_where/H | 0.253 [0.207, 0.307] | 288 |
| `still` | notice_pair/n_what/F | 0.732 [0.619, 0.821] | 71 |
| `still` | notice_pair/n_what/H | 0.722 [0.61, 0.812] | 72 |
| `still` | notice_pair/n_where2/F | 0.732 [0.619, 0.821] | 71 |
| `still` | notice_pair/n_where2/H | 0.722 [0.61, 0.812] | 72 |
| `kestim` | motion/m_same/F | 0.94 [0.838, 0.979] | 50 |
| `kestim` | motion/m_same/H | 0.84 [0.715, 0.917] | 50 |
| `kestim` | motion_final/m_same/F | 0.979 [0.889, 0.996] | 47 |
| `kestim` | motion_final/m_same/H | 0.875 [0.753, 0.941] | 48 |
| `kestim` | notice/n_where/F | 0.319 [0.268, 0.375] | 288 |
| `kestim` | notice/n_where/H | 0.292 [0.242, 0.347] | 288 |
| `kestim` | notice_pair/n_what/F | 0.819 [0.715, 0.891] | 72 |
| `kestim` | notice_pair/n_what/H | 0.792 [0.684, 0.869] | 72 |
| `kestim` | notice_pair/n_where2/F | 0.833 [0.731, 0.902] | 72 |
| `kestim` | notice_pair/n_where2/H | 0.819 [0.715, 0.891] | 72 |
| `surek-az` | notice/n_where/F | 0.583 [0.526, 0.639] | 288 |
| `surek-az` | notice/n_where/H | 0.557 [0.5, 0.614] | 287 |
| `surek-az` | notice_pair/n_what/F | 0.861 [0.763, 0.923] | 72 |
| `surek-az` | notice_pair/n_what/H | 0.847 [0.747, 0.912] | 72 |
| `surek-az` | notice_pair/n_where2/F | 0.903 [0.813, 0.952] | 72 |
| `surek-az` | notice_pair/n_where2/H | 0.903 [0.813, 0.952] | 72 |
| `nabiz-az` | notice/n_where/F | 0.458 [0.402, 0.516] | 288 |
| `nabiz-az` | notice/n_where/H | 0.403 [0.348, 0.46] | 288 |
| `nabiz-az` | notice_pair/n_what/F | 0.875 [0.779, 0.933] | 72 |
| `nabiz-az` | notice_pair/n_what/H | 0.903 [0.813, 0.952] | 72 |
| `nabiz-az` | notice_pair/n_where2/F | 0.889 [0.796, 0.943] | 72 |
| `nabiz-az` | notice_pair/n_where2/H | 0.917 [0.83, 0.961] | 72 |

## Eleştirmen (sonnet, üç kare, tr/açık)

| tasarım | BT [%95] |
|---|---|
| `nabiz` | 1.72 [1.41, 2.12] |
| `still` | 1.14 [0.85, 1.47] |
| `surek` | 0.57 [0.28, 0.89] |
| `r4` | 0.34 [0.03, 0.65] |
| `kestim` | -3.76 [-4.42, -3.13] |

| çift | bölünme / toplam | oran | ayırt edilemedi |
|---|---|---|---|
| kestim|nabiz | 0/24 | 0.0 |  |
| kestim|r4 | 1/24 | 0.042 |  |
| kestim|still | 0/24 | 0.0 |  |
| kestim|surek | 0/24 | 0.0 |  |
| nabiz|r4 | 1/24 | 0.042 |  |
| nabiz|still | 1/24 | 0.042 |  |
| nabiz|surek | 4/24 | 0.167 |  |
| r4|still | 0/24 | 0.0 |  |
| r4|surek | 2/23 | 0.087 |  |
| still|surek | 3/24 | 0.125 |  |

## Hareketi azalt bilgi kaybı

| tasarım | azaltılmış | (a) durağan içerik kaybı | (b) metin kuralı (öğe) | (b) herhangi ilke (ana) | (b) çıplak dizgi (ilk uygulama) | hareketli yüz sayısı |
|---|---|---|---|---|---|---|
| `nabiz` | `nabiz-az` | 0 | 4 | 0 | 6 | 16 |
| `r4` | `still` | 0 | 0 | 0 | 0 | 0 |
| `surek` | `surek-az` | 0 | 0 | 0 | 0 | 12 |

## Kapılar ve seçim adımları (ana: `any_prim`)

```json
{
 "gates": {
  "nabiz": {
   "k52_jit": true,
   "k52_motion_jit": true,
   "k52_motion_nojit": true,
   "reduced_motion_loss": true
  },
  "r4": {
   "k52_jit": true,
   "k52_motion_jit": true,
   "k52_motion_nojit": true,
   "reduced_motion_loss": true
  },
  "still": {
   "k52_jit": true,
   "k52_motion_jit": true,
   "k52_motion_nojit": true,
   "reduced_motion_loss": true
  },
  "surek": {
   "k52_jit": true,
   "k52_motion_jit": true,
   "k52_motion_nojit": true,
   "reduced_motion_loss": true
  }
 },
 "waived": [],
 "steps": [
  {
   "eligible": [
    "r4",
    "still",
    "surek",
    "nabiz"
   ],
   "eliminated": {},
   "rule": "hard gates (K52 JIT on, K52 during motion JIT on / off, reduced-motion loss)",
   "step": 1,
   "waived": []
  },
  {
   "no_primary": [
    "r4",
    "surek",
    "nabiz"
   ],
   "order": [
    "still"
   ],
   "rule": "continuity (motion, valid questions only, F2:H1)",
   "step": 2,
   "top_group": [
    "still"
   ]
  }
 ],
 "verdict": {
  "default": "still",
  "not_told_apart": [],
  "ranking": [
   "still",
   "nabiz",
   "surek",
   "r4"
  ],
  "told_apart": true,
  "top_two": [
   "still",
   "nabiz"
  ],
  "winner": "still"
 },
 "chars": {
  "kestim": 565.6,
  "nabiz": 580.5,
  "r4": 576.4,
  "still": 576.4,
  "surek": 576.4
 }
}
```

## Duyarlılık (G6-5 lafzı: yalnız metin)
```json
{
 "gates": {
  "nabiz": {
   "k52_jit": true,
   "k52_motion_jit": true,
   "k52_motion_nojit": true,
   "reduced_motion_loss": false
  },
  "r4": {
   "k52_jit": true,
   "k52_motion_jit": true,
   "k52_motion_nojit": true,
   "reduced_motion_loss": true
  },
  "still": {
   "k52_jit": true,
   "k52_motion_jit": true,
   "k52_motion_nojit": true,
   "reduced_motion_loss": true
  },
  "surek": {
   "k52_jit": true,
   "k52_motion_jit": true,
   "k52_motion_nojit": true,
   "reduced_motion_loss": true
  }
 },
 "steps": [
  {
   "eligible": [
    "r4",
    "still",
    "surek"
   ],
   "eliminated": {
    "nabiz": [
     "reduced_motion_loss"
    ]
   },
   "rule": "hard gates (K52 JIT on, K52 during motion JIT on / off, reduced-motion loss)",
   "step": 1,
   "waived": []
  },
  {
   "no_primary": [
    "r4",
    "surek"
   ],
   "order": [
    "still"
   ],
   "rule": "continuity (motion, valid questions only, F2:H1)",
   "step": 2,
   "top_group": [
    "still"
   ]
  }
 ],
 "verdict": {
  "default": "still",
  "not_told_apart": [],
  "ranking": [
   "still",
   "surek",
   "r4"
  ],
  "told_apart": true,
  "top_two": [
   "still",
   "surek"
  ],
  "winner": "still"
 }
}
```

## Bozuk tasarım
```json
{
 "broken_check": {
  "bt": -3.7642118009893863,
  "bt_strict_last": true,
  "motion_all_strict_last": false,
  "notice": 0.3101851851851852,
  "notice_strict_last": false,
  "pass": false,
  "primary": null,
  "primary_strict_last": false
 },
 "paired_vs_broken": {
  "nabiz": {
   "diff": -0.0467,
   "hi": 0.0261,
   "lo": -0.1224,
   "n": 100,
   "separated": false
  },
  "r4": {
   "diff": 0.0933,
   "hi": 0.1523,
   "lo": 0.0411,
   "n": 100,
   "separated": true
  },
  "still": {
   "diff": -0.9067,
   "hi": -0.8477,
   "lo": -0.9589,
   "n": 100,
   "separated": false
  },
  "surek": {
   "diff": 0.0933,
   "hi": 0.1523,
   "lo": 0.0411,
   "n": 100,
   "separated": true
  }
 }
}
```

Boylar arası uyum (haiku–sonnet, `motion` tr/açık, ilk tekrar): {'agree': 0.85, 'models': ['claude-haiku-5-5', 'claude-sonnet-5-5'], 'n': 20}

Tek bakış dışı (birden çok Read) çıkarılan: 5

Maliyet (puanlanan kayıtlar): {'critic_calls': 479, 'critics': 8.0318, 'reader_calls': 6015, 'readers': 5.3599}
