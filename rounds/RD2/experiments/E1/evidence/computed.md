# Hesapla ölçülenler (sert eşikler)

| tasarım | metin karşıtlığı (en kötü / eşik) | işaret karşıtlığı (≥ 3) | en küçük yazı (≥ 11 px) | dokunma < 44 | renk ΔE00 en küçük / eşik | yüz karakter ort./en çok | ekran karakter ort. | K52 | geçti |
|---|---|---|---|---:|---|---|---|---|---|
| `oltan` | 5.24 / 4.5 | 3.46 | 13 | 0 | 10.53 / 10.0 (dark: up ↔ valid) | 128.6 / 140 | 539.5 | — | True |
| `tavek` | 5.08 / 4.5 | 3.07 | 11 | 180 | 11.45 / 10.0 (light: up ↔ neutral) | 176.4 / 194 | 681.5 | — | False |
| `tavek-tel` | 5.08 / 4.5 | 3.07 | 13.0 | 0 | 11.45 / 10.0 (light: up ↔ neutral) | 176.0 / 193 | 500.7 | — | True |

Renk kapısı Ek G5 (görünüş kaydının rolleri). Bilerek paylaşılan roller (sayılmaz; gri soruyla sınanır):

- `oltan`: dark: neutral = invalid, dark: neutral = closed, dark: valid = text, dark: invalid = closed, light: neutral = invalid, light: neutral = closed, light: valid = text, light: invalid = closed
- `tavek`: dark: neutral = invalid, dark: neutral = closed, dark: valid = text, dark: invalid = closed, light: neutral = invalid, light: neutral = closed, light: valid = text, light: invalid = closed
- `tavek-tel`: dark: neutral = invalid, dark: neutral = closed, dark: valid = text, dark: invalid = closed, light: neutral = invalid, light: neutral = closed, light: valid = text, light: invalid = closed
