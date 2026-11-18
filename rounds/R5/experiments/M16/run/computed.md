# Hesapla ölçülenler (sert eşikler)

| tasarım | metin karşıtlığı (en kötü / eşik) | işaret karşıtlığı (≥ 3) | en küçük yazı (≥ 11 px) | dokunma < 44 | renk ΔE00 en küçük / eşik | yüz karakter ort./en çok | ekran karakter ort. | K52 | geçti |
|---|---|---|---|---:|---|---|---|---|---|
| `bertan` | 6.39 / 4.5 | 3.17 | 13 | 0 | 7.28 / 10.0 (light: up ↔ invalid) | 131.4 / 140 | 651.7 | True | False |
| `kestim` | 1.62 / 4.5 | 1.26 | 10 | 108 | 0.31 / 15.0 (light: down ↔ invalid) | 127.4 / 137 | 677.8 | — | False |
| `kivra` | 6.17 / 4.5 | 3.17 | 12 | 0 | 11.45 / 10.0 (dark: down ↔ invalid) | 132.7 / 143 | 900.2 | True | True |
| `oltan` | 6.33 / 4.5 | 3.46 | 13 | 0 | 10.53 / 10.0 (dark: up ↔ valid) | 130.4 / 139 | 665.7 | True | True |
| `r4` | 2.75 / 4.5 | 2.06 | 10 | 108 | 0.5 / 10.0 (dark: down ↔ invalid) | 131.0 / 139 | 686.8 | True | False |
| `zemre` | 6.61 / 4.5 | 3.11 | 12 | 0 | 10.43 / 10.0 (light: down ↔ neutral) | 170.2 / 182 | 601.1 | True | True |

Renk kapısı Ek G5 (görünüş kaydının rolleri). Bilerek paylaşılan roller (sayılmaz; gri soruyla sınanır):

- `bertan`: dark: neutral = valid, dark: neutral = text, dark: valid = text, dark: invalid = closed, light: neutral = valid, light: neutral = text, light: valid = text, light: invalid = closed
- `kestim`: dark: neutral = accent, dark: neutral = focus, dark: valid = text, dark: invalid = closed, dark: invalid = muted, dark: closed = muted, dark: accent = focus, light: neutral = accent, light: neutral = focus, light: valid = text, light: invalid = closed, light: invalid = muted, light: closed = muted, light: accent = focus
- `kivra`: dark: neutral = valid, dark: neutral = text, dark: valid = text, dark: invalid = closed, light: neutral = valid, light: neutral = text, light: valid = text, light: invalid = closed
- `oltan`: dark: neutral = invalid, dark: neutral = closed, dark: valid = text, dark: invalid = closed, light: neutral = invalid, light: neutral = closed, light: valid = text, light: invalid = closed
- `r4`: dark: neutral = accent, dark: neutral = focus, dark: valid = text, dark: invalid = closed, dark: invalid = muted, dark: closed = muted, dark: accent = focus, light: neutral = accent, light: neutral = focus, light: valid = text, light: invalid = closed, light: invalid = muted, light: closed = muted, light: accent = focus
- `zemre`: dark: neutral = valid, dark: neutral = text, dark: valid = text, dark: invalid = closed, light: neutral = valid, light: neutral = text, light: valid = text, light: invalid = closed
