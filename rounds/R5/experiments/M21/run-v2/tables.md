# M21 yeniden ölçüm — tablolar (üretildi: `tables_v2.py`)

## Geçerlilik (Ek G8-1): son kareyle `m_from`, şans 0,25, tek yönlü

| tasarım | uyaran | n | doğruluk [Wilson %95] | geçerli (alt sınır ≤ 0,25) | iki yönlü (0,25 aralıkta) |
|---|---|---|---|---|---|
| `surek` | motion2_final | 384 | 0.000 [0.000, 0.010] | evet | hayır |
| `nabiz` | motion2_final | 383 | 0.000 [0.000, 0.010] | evet | hayır |
| `r4` | motion2_final | 384 | 0.000 [0.000, 0.010] | evet | hayır |
| `still` | motion2 | 388 | 0.003 [0.000, 0.014] | evet | hayır |
| `tirvan` | motion2_final | 384 | 0.000 [0.000, 0.010] | evet | hayır |

Soru bütün tasarımlar için geçerli (`still` sınaması, G8-1 ii): **evet**

## Süreklilik `m_from` (birincil; F2:H1, %95 önyükleme)

| tasarım | birincil | `motion2` hepsi | şerit 1 / 2 / 3 / 4 (ham pay) | "olduğu yerde beliriyor" payı | son kare (`motion2_final`) |
|---|---|---|---|---|---|
| `surek` | 0.573 [0.524, 0.625] | 0.573 [0.522, 0.628] | 0.917 / 0.429 / 0.316 / 0.615 | 0.013 | 0.000 [0.000, 0.000] |
| `nabiz` | 0.698 [0.651, 0.744] | 0.698 [0.650, 0.743] | 0.729 / 0.571 / 0.510 / 0.938 | 0.088 | 0.000 [0.000, 0.000] |
| `r4` | 0.373 [0.324, 0.424] | 0.373 [0.323, 0.424] | 0.750 / 0.184 / 0.163 / 0.365 | 0.031 | 0.000 [0.000, 0.000] |
| `still` | — | 0.002 [0.000, 0.005] | 0.010 / 0.000 / 0.000 / 0.000 | 0.103 | — |
| `tirvan` | 0.020 [0.007, 0.038] | 0.020 [0.007, 0.038] | 0.052 / 0.000 / 0.000 / 0.021 | 0.000 | 0.000 [0.000, 0.000] |

## Fark edilme (Ek G8-3): isabet − yanlış alarm (F2:H1, %95 önyükleme)

| tasarım | isabet − YA (mid + after) | yalnız mid | yalnız after | isabet F / H | YA F / H | isabet: hedef canlı / kapanmış | YA: canlılı / canlısız denetim |
|---|---|---|---|---|---|---|---|
| `surek` | -0.019 [-0.073, 0.031] | -0.049 [-0.118, 0.021] | 0.010 [-0.066, 0.087] | 0.141 / 0.115 | 0.172 / 0.109 | 0.104 / 0.151 | 0.177 / 0.104 |
| `nabiz` | 0.113 [0.047, 0.181] | 0.076 [-0.017, 0.163] | 0.149 [0.059, 0.240] | 0.526 / 0.359 | 0.396 / 0.281 | 0.641 / 0.245 | 0.552 / 0.125 |
| `r4` | -0.024 [-0.080, 0.031] | -0.024 [-0.101, 0.052] | -0.024 [-0.100, 0.049] | 0.167 / 0.100 | 0.203 / 0.099 | 0.109 / 0.157 | 0.172 / 0.130 |
| `still` | -0.002 [-0.057, 0.054] | -0.028 [-0.104, 0.052] | 0.024 [-0.045, 0.097] | 0.193 / 0.083 | 0.188 / 0.099 | 0.104 / 0.172 | 0.151 / 0.135 |
| `tirvan` | -0.071 [-0.120, -0.021] | -0.122 [-0.177, -0.066] | -0.021 [-0.097, 0.056] | 0.104 / 0.042 | 0.188 / 0.089 | 0.057 / 0.089 | 0.151 / 0.125 |

Durum başına doğru cevap payı (değişimde doğru şerit, denetimde "hayır"; mid + after):

| durum | `surek` | `nabiz` | `r4` | `still` | `tirvan` |
|---|---|---|---|---|---|
| `c_born` | 0.125 | 0.667 | 0.146 | 0.115 | 0.094 |
| `c_failed` | 0.198 | 0.323 | 0.221 | 0.250 | 0.083 |
| `c_realized` | 0.104 | 0.167 | 0.094 | 0.094 | 0.094 |
| `c_strengthened` | 0.083 | 0.615 | 0.073 | 0.094 | 0.021 |
| `n_live1` | 0.375 | 0.010 | 0.333 | 0.354 | 0.333 |
| `n_live2` | 0.250 | 0.010 | 0.333 | 0.375 | 0.312 |
| `n_still1` | 0.219 | 0.208 | 0.167 | 0.219 | 0.198 |
| `n_still2` | 0.510 | 0.531 | 0.500 | 0.500 | 0.510 |

## Eleştirmen BT (%95)

| tasarım | BT |
|---|---|
| `nabiz` | 1.45 [1.08, 1.92] |
| `still` | 0.70 [0.41, 1.04] |
| `surek` | -0.03 [-0.27, 0.21] |
| `r4` | -0.36 [-0.67, -0.06] |
| `tirvan` | -1.76 [-2.31, -1.38] |

Çelişki oranları: nabiz|r4 2/18; nabiz|still 5/18; nabiz|surek 3/18; nabiz|tirvan 1/18; r4|still 0/18; r4|surek 0/18; r4|tirvan 1/18; still|surek 0/18; still|tirvan 0/18; surek|tirvan 0/18

## Bozuk hareketin yeri (Ek G8-2)

```
{
 "bt": -1.7640041259510362,
 "bt_strict_last": true,
 "notice": -0.07118055555555555,
 "notice_strict_last": true,
 "pass": true,
 "primary": 0.020497204117248993,
 "primary_strict_last": true
}
```

## Kapılar

```
{
 "gates": {
  "nabiz": {
   "k52_jit": true,
   "k52_motion_jit": false,
   "k52_motion_nojit": true,
   "reduced_motion_loss": false
  },
  "r4": {
   "k52_jit": true,
   "k52_motion_jit": false,
   "k52_motion_nojit": true,
   "reduced_motion_loss": true
  },
  "still": {
   "k52_jit": true,
   "k52_motion_jit": false,
   "k52_motion_nojit": true,
   "reduced_motion_loss": true
  },
  "surek": {
   "k52_jit": true,
   "k52_motion_jit": false,
   "k52_motion_nojit": true,
   "reduced_motion_loss": true
  }
 },
 "waived": [
  "k52_motion_jit"
 ]
}
```

## Bilgi kaybı (Ek G8-6)

| tasarım → azaltılmış | (a) durağan | (b) metin kuralı | (b) herhangi bir ilke (duyarlılık) | (b) çıplak dizgi | hareketli yüz |
|---|---|---|---|---|---|
| `nabiz` → `nabiz-az` | 4 | 0 | 0 | 2 | 24 |
| `r4` → `still` | 0 | 0 | 0 | 0 | 0 |
| `surek` → `surek-az` | 0 | 0 | 0 | 0 | 12 |

## Seçim adımları

```
[
 {
  "eligible": [
   "surek",
   "r4",
   "still"
  ],
  "eliminated": {
   "nabiz": [
    "reduced_motion_loss"
   ]
  },
  "rule": "hard gates (K52; reduced-motion loss, text rule)",
  "step": 1,
  "waived": [
   "k52_motion_jit"
  ]
 },
 {
  "no_primary": [
   "still"
  ],
  "order": [
   "surek",
   "r4"
  ],
  "rule": "continuity m_from (valid designs only, F2:H1)",
  "step": 2,
  "top_group": [
   "surek"
  ]
 }
]
```

## Karar

```
{
 "default": "surek",
 "not_told_apart": [],
 "ranking": [
  "surek",
  "r4",
  "still"
 ],
 "told_apart": true,
 "top_two": [
  "surek",
  "r4"
 ],
 "usable": true,
 "winner": "surek"
}
```

Boylar arası uyum: {"agree": 0.25, "models": ["claude-haiku-5-5", "claude-sonnet-5-5"], "n": 20}

Maliyet: {"critic_calls": 360, "critics": 6.1067, "reader_calls": 7313, "readers": 6.3472}; tek bakış dışı okuyucu birimi 3, eleştirmen 0
