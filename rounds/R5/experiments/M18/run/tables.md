### Deterministik kapı (§G7; açık sahnenin boyama listesi + DOM)

| tasarım | metin karşıtlığı en kötü / eşik (öğe) | metin dışı en kötü (rol) | en küçük yazı px | dokunma < 44 (DOM) | renk ΔE00 en kötü çift (koşul) | sahne metni ort. karakter | geçti |
|---|---|---|---|---:|---|---:|---|
| `korda` | 3.57 / 4.5 (`36.32`, 11 px) | 1.1 (focus/neutral, line) | 11 | 60 | up ↔ closed/invalid: 0.13 / 15.0 (gray) | 110.3 | False |
| `selen` | 1.88 / 4.5 (`5`, 11 px) | 1.02 (closed/invalid, circle) | 11 | 60 | up ↔ closed/invalid: 0.13 / 15.0 (gray) | 113.8 | False |
| `pavka` | 3.57 / 4.5 (`36.32`, 11 px) | 1.08 (focus/neutral, fill) | 11 | 60 | up ↔ closed/invalid: 0.13 / 15.0 (gray) | 179.8 | False |
| `r4` | 3.57 / 4.5 (`C 36.32`, 16 px) | 2.8 (up, rect) | 11 | 60 | up ↔ down: 0.63 / 15.0 (gray) | 49.3 | False |
| `kestim` | 1.53 / 4.5 (`36.32`, 12 px) | 1.27 (up, rect) | 11 | 60 | up ↔ down: 1.79 / 15.0 (gray) | 47.7 | False |

Uygulanmayan eşikler (hiçbir aday, `r4` dahil, geçemedi; §G9.1): ['text_contrast', 'nontext_contrast', 'touch', 'colour']

### K52 (k52chart.mjs; 3 tekrarın ortancası; ms)

| tasarım | kip | görünen hikâye (5/50/200) | tik p95 | sahne boyama p95 | sürüklemede kare p99 | sürüklemede uzun kare payı | PHONE_BIAS ile sahne p95 | geçti |
|---|---|---|---|---|---|---|---|---|
| `korda` | `phone-jit-est` | 6 / 52 / 219 | 1 / 1 / 1 | 1 / 2 / 2 | 19 / 19 / 19 | 0 / 0 / 0 | — | True |
| `korda` | `phone` | 6 / 52 / 222 | 1 / 1 / 1 | 5 / 13 / 13 | 19 / 23 / 22 | 0 / 0 / 0 | 9.2 / 24.1 / 24.1 | True |
| `selen` | `phone-jit-est` | 6 / 52 / 219 | 1 / 1 / 1 | 1 / 2 / 2 | 19 / 18 / 19 | 0 / 0 / 0 | — | True |
| `selen` | `phone` | 6 / 52 / 219 | 1 / 1 / 1 | 5 / 13 / 20 | 19 / 24 / 33 | 0 / 0.003 / 0.133 | 9.2 / 24.1 / 37 | False |
| `pavka` | `phone-jit-est` | 6 / 52 / 219 | 1 / 1 / 1 | 1 / 3 / 2 | 19 / 19 / 18 | 0 / 0 / 0 | — | True |
| `pavka` | `phone` | 6 / 52 / 222 | 1 / 1 / 1 | 11 / 18 / 23 | 20 / 32 / 38 | 0 / 0.218 / 0.14 | 20.4 / 33.3 / 42.6 | False |
| `r4` | `phone-jit-est` | 6 / 52 / 219 | 1 / 1 / 1 | 1 / 2 / 1 | 19 / 19 / 19 | 0 / 0 / 0 | — | True |
| `r4` | `phone` | 6 / 52 / 222 | 1 / 1 / 1 | 4 / 9 / 10 | 18 / 19 / 20 | 0 / 0 / 0 | 7.4 / 16.7 / 18.5 | True |

JIT kapalı yeniden koşu (`phone`, 5 tekrar, model çağrısı yokken; seçimde bu kullanılır):

| tasarım | derinlik | görünen (5 koşu) | sürüklemede uzun kare payı (ortanca; 5 koşu) | sahne boyama p95 (5 koşu) | kare p99 | geçti (≤ %10) |
|---|---|---|---|---|---|---|
| `korda` | 5 | [6, 5, 6, 7, 5] | 0 ([0, 0, 0, 0, 0]) | [5, 5, 5, 5, 5] | 19 | True |
| `korda` | 50 | [52, 54, 55, 56, 56] | 0 ([0, 0, 0, 0.003, 0]) | [12, 14, 12, 13, 13] | 22 | True |
| `korda` | 200 | [234, 237, 201, 206, 208] | 0 ([0.003, 0, 0, 0.006, 0]) | [14, 14, 12, 13, 13] | 22 | True |
| `selen` | 5 | [6, 5, 6, 6, 5] | 0 ([0, 0, 0, 0, 0]) | [6, 5, 6, 6, 6] | 19 | True |
| `selen` | 50 | [52, 54, 55, 56, 56] | 0 ([0, 0, 0, 0, 0]) | [12, 12, 13, 13, 13] | 24 | True |
| `selen` | 200 | [234, 237, 201, 206, 208] | 0.086 ([0.086, 0.096, 0.086, 0.063, 0.066]) | [18, 18, 16, 16, 16] | 31 | True |
| `pavka` | 5 | [6, 5, 6, 6, 5] | 0 ([0, 0, 0, 0, 0.003]) | [10, 11, 11, 11, 10] | 19 | True |
| `pavka` | 50 | [52, 54, 55, 56, 56] | 0.135 ([0.175, 0.15, 0.135, 0.121, 0.1]) | [18, 18, 18, 18, 18] | 32 | False |
| `pavka` | 200 | [234, 237, 201, 206, 208] | 0.086 ([0.106, 0.101, 0.066, 0.086, 0.075]) | [23, 23, 22, 21, 22] | 38 | True |
| `r4` | 5 | [6, 5, 6, 6, 5] | 0 ([0, 0, 0, 0, 0]) | [4, 4, 4, 4, 4] | 19 | True |
| `r4` | 50 | [52, 54, 55, 56, 56] | 0 ([0, 0, 0, 0, 0]) | [9, 9, 9, 9, 9] | 19 | True |
| `r4` | 200 | [234, 237, 201, 203, 208] | 0 ([0, 0, 0, 0, 0]) | [10, 10, 9, 10, 10] | 19 | True |

### Birincil: `chart` kümesi doğruluğu (F 2 : H 1, birim = görüntü × persona × tekrar × model — seçim kuralının kendi girdisi: haiku 6 tekrar + sonnet alt kümesi; %95 önyükleme)

| tasarım | hepsi | 5 hikâye | 50 hikâye | 200 hikâye | F | H | birim (F/H) |
|---|---|---|---|---|---|---|---|
| `korda` | 0.928 [0.907, 0.948] | 0.987 [0.967, 1.000] | 0.863 [0.820, 0.903] | 0.933 [0.903, 0.960] | 0.953 | 0.877 | 75/75 |
| `selen` | 0.868 [0.842, 0.892] | 0.877 [0.840, 0.913] | 0.890 [0.847, 0.927] | 0.837 [0.787, 0.883] | 0.883 | 0.837 | 75/75 |
| `pavka` | 0.987 [0.977, 0.996] | 0.993 [0.980, 1.000] | 0.983 [0.960, 1.000] | 0.983 [0.960, 1.000] | 0.987 | 0.987 | 75/75 |
| `r4` | 0.411 [0.384, 0.438] | 0.540 [0.500, 0.577] | 0.340 [0.303, 0.373] | 0.353 [0.310, 0.403] | 0.437 | 0.360 | 75/75 |
| `kestim` | 0.406 [0.377, 0.436] | 0.573 [0.530, 0.617] | 0.310 [0.280, 0.343] | 0.333 [0.300, 0.370] | 0.410 | 0.397 | 75/75 |

Şans düzeyi (soru başına 1/k): c_recent 0,20 · c_held 0,25 · c_live 0,20 · c_evid 0,167 → birim ≈ 0,204.

### Soru başına doğruluk (haiku, F+H birlikte; Wilson %95)

| tasarım | c_recent | c_held | c_live | c_evid |
|---|---|---|---|---|
| `korda` | 0.97 [0.92, 0.99] n=144 | 0.74 [0.67, 0.81] n=144 | 0.96 [0.91, 0.98] n=144 | 0.98 [0.94, 0.99] n=144 |
| `selen` | 0.91 [0.85, 0.95] n=144 | 1.00 [0.97, 1.00] n=144 | 0.52 [0.44, 0.60] n=144 | 0.99 [0.95, 1.00] n=144 |
| `pavka` | 0.99 [0.96, 1.00] n=144 | 0.96 [0.91, 0.98] n=144 | 1.00 [0.97, 1.00] n=144 | 1.00 [0.97, 1.00] n=144 |
| `r4` | 0.31 [0.24, 0.39] n=144 | 0.29 [0.22, 0.37] n=144 | 0.01 [0.00, 0.04] n=144 | 1.00 [0.97, 1.00] n=144 |
| `kestim` | 0.28 [0.22, 0.36] n=144 | 0.34 [0.27, 0.42] n=144 | 0.00 [0.00, 0.03] n=144 | 0.99 [0.96, 1.00] n=144 |

### Soru × derinlik (haiku; oran, n = 24 her hücre)

| tasarım | c_recent 5 | c_recent 50 | c_recent 200 | c_held 5 | c_held 50 | c_held 200 | c_live 5 | c_live 50 | c_live 200 | c_evid 5 | c_evid 50 | c_evid 200 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `korda` | 1.00 | 1.00 | 0.90 | 0.98 | 0.48 | 0.77 | 0.98 | 0.94 | 0.96 | 1.00 | 0.94 | 1.00 |
| `selen` | 1.00 | 0.98 | 0.75 | 1.00 | 1.00 | 1.00 | 0.48 | 0.56 | 0.52 | 1.00 | 0.96 | 1.00 |
| `pavka` | 1.00 | 0.98 | 1.00 | 1.00 | 0.94 | 0.94 | 1.00 | 1.00 | 1.00 | 1.00 | 1.00 | 1.00 |
| `r4` | 0.85 | 0.00 | 0.06 | 0.29 | 0.33 | 0.25 | 0.00 | 0.00 | 0.02 | 1.00 | 1.00 | 1.00 |
| `kestim` | 0.85 | 0.00 | 0.00 | 0.48 | 0.23 | 0.31 | 0.00 | 0.00 | 0.00 | 0.98 | 1.00 | 1.00 |

### Kapanmışlar görünürken canlının bulunması (`c_live`, haiku; Wilson %95)

| tasarım | 5 | 50 | 200 | cevap dağılımı (hepsi) |
|---|---|---|---|---|
| `korda` | 0.98 [0.89, 1.00] | 0.94 [0.83, 0.98] | 0.96 [0.86, 0.99] | {'A': 10, 'B': 31, 'C': 6, 'D': 91, 'unk': 6} |
| `selen` | 0.48 [0.34, 0.62] | 0.56 [0.42, 0.69] | 0.52 [0.38, 0.66] | {'A': 15, 'B': 2, 'C': 7, 'D': 54, 'unk': 66} |
| `pavka` | 1.00 [0.93, 1.00] | 1.00 [0.93, 1.00] | 1.00 [0.93, 1.00] | {'A': 15, 'B': 2, 'C': 31, 'D': 96} |
| `r4` | 0.00 [0.00, 0.07] | 0.00 [0.00, 0.07] | 0.02 [0.00, 0.11] | {'D': 1, 'none': 141, 'unk': 2} |
| `kestim` | 0.00 [0.00, 0.07] | 0.00 [0.00, 0.07] | 0.00 [0.00, 0.07] | {'none': 141, 'unk': 3} |

### İkinci boy (sonnet; tr/açık, 1 tekrar) ve boylar arası uyum

| tasarım | sonnet doğruluk (F2:H1) | haiku aynı alt küme (tr/açık, tekrar 0) |
|---|---|---|
| `korda` | 1.000 [1.000, 1.000] | 0.972 [0.917, 1.000] |
| `selen` | 1.000 [1.000, 1.000] | 0.917 [0.806, 1.000] |
| `pavka` | 0.944 [0.833, 1.000] | 1.000 [1.000, 1.000] |
| `r4` | 0.333 [0.250, 0.444] | 0.500 [0.417, 0.611] |
| `kestim` | 0.361 [0.250, 0.472] | 0.389 [0.278, 0.472] |

Uyum (aynı uyaran, persona, soru; ilk tekrar): {'agree': 0.875, 'models': ['claude-haiku-5-5', 'claude-sonnet-5-5'], 'n': 120}

### Eleştirmen (sonnet; 10 çift × 3 derinlik × 2 sıra × 3 tekrar; tr/açık)

| tasarım | BT gücü [%95 önyükleme] |
|---|---|
| `korda` | 1.368 [0.899, 2.014] |
| `selen` | -0.747 [-1.453, -0.218] |
| `pavka` | 3.822 [3.179, 4.625] |
| `r4` | -0.248 [-0.907, 0.356] |
| `kestim` | -4.195 [-4.697, -3.821] |

| çift | bölünme / toplam | çelişki oranı | ayırt edilemedi (> %30) |
|---|---|---|---|
| `kestim|korda` | 0/9 | 0.0 | hayır |
| `kestim|pavka` | 0/9 | 0.0 | hayır |
| `kestim|r4` | 0/9 | 0.0 | hayır |
| `kestim|selen` | 0/9 | 0.0 | hayır |
| `korda|pavka` | 1/9 | 0.111 | hayır |
| `korda|r4` | 3/9 | 0.333 | evet |
| `korda|selen` | 1/9 | 0.111 | hayır |
| `pavka|r4` | 0/9 | 0.0 | hayır |
| `pavka|selen` | 0/9 | 0.0 | hayır |
| `r4|selen` | 1/9 | 0.111 | hayır |

Uzunluk yanlılığı (U2): BT ~ ekran karakteri Spearman = 0.7; kesin sonuçlarda daha çok metinli olanın kazanma payı = {'n': 84, 'share': 0.845}

| tasarım | ham kararlar: kazandı / ayırt edemiyorum / kaybetti |
|---|---|
| `korda` | 51 / 0 / 21 |
| `selen` | 26 / 0 / 46 |
| `pavka` | 71 / 0 / 1 |
| `r4` | 32 / 0 / 40 |
| `kestim` | 0 / 0 / 72 |

Tek bakış dışı (birden çok Read) çıkarılan: 0; kesilen cevap: {'korda': 77, 'selen': 46, 'pavka': 76, 'r4': 2, 'kestim': 3}

### Bozuk tasarım sınaması (§G8)

```json
{
 "broken": "kestim",
 "checks": {
  "last_on_bt": true,
  "last_on_primary": true,
  "paired_separated_from_all_info": false,
  "primary_below_best_interval": true
 },
 "paired_primary_info": {
  "korda": {
   "diff": 0.5222,
   "hi": 0.5536,
   "lo": 0.4899,
   "n": 150,
   "separated": true
  },
  "pavka": {
   "diff": 0.5811,
   "hi": 0.6114,
   "lo": 0.5504,
   "n": 150,
   "separated": true
  },
  "r4": {
   "diff": 0.0056,
   "hi": 0.0391,
   "lo": -0.0283,
   "n": 150,
   "separated": false
  },
  "selen": {
   "diff": 0.4622,
   "hi": 0.5,
   "lo": 0.4243,
   "n": 150,
   "separated": true
  }
 },
 "required": [
  "last_on_primary",
  "last_on_bt"
 ],
 "valid": true
}
```

### Seçim kuralı adımları (gl/select.py çıktısı)

- {"eligible": ["korda", "r4", "selen"], "eliminated": {"pavka": ["k52"]}, "rule": "hard gates", "step": 1, "waived": ["text_contrast", "nontext_contrast", "touch", "colour"]}
- {"order": ["korda", "selen", "r4"], "rule": "primary: chart accuracy (F2:H1)", "step": 2, "top_group": ["korda"]}

Karar: `{"default": "korda", "not_told_apart": [], "ranking_point_estimates": ["korda", "selen", "r4"], "told_apart": true, "top_two": ["korda", "selen"], "winner": "korda"}`

Maliyet: okuyucu 750 çağrı 1.23 $, eleştirmen 180 çağrı 2.60 $, toplam 3.84 $.

