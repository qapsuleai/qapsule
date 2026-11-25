# I1: yeniden kaydedilen qp-session sınamalarının kanıtı

Yöntem: `main` (ae5ad278) ile `rl1/i1-schema` iki ayrı worktree ve iki ayrı derleme klasöründe
koşuldu. `charac/snap.rs`'e geçici, commit edilmeyen bir döküm kancası eklendi (her olgunun
giden JSON satırları). İki döküm alan alan karşılaştırıldı. Fark yalnız aşağıdadır ve
tamamen eklemedir. Eski listeler yeni listenin öneki olarak duruyor; silinen ya da değişen alan yok.

## charac (570 olgu, olgu sırası aynı; 8 olgu değişti)

```
CASE transcripts-core 03-errors-self-correction.03 bytes 776 -> 828 delta 52
  LIST /msg/error/expected added ['add_program', 'set_program_params', 'remove_program'] removed [] old list kept as prefix: True
CASE transcripts-core 16-screen-describe-presenter.04 bytes 1864 -> 1964 delta 100
  LIST /msg/data/description/ops added ['add_program', 'set_program_params', 'remove_program'] removed [] old list kept as prefix: True
  LIST /msg/data/description/queries added ['check_program', 'try_program', 'explain_program'] removed [] old list kept as prefix: True
CASE worlds w.70.q_describe_o bytes 1961 -> 2061 delta 100
  LIST /msg/data/description/ops added ['add_program', 'set_program_params', 'remove_program'] removed [] old list kept as prefix: True
  LIST /msg/data/description/queries added ['check_program', 'try_program', 'explain_program'] removed [] old list kept as prefix: True
CASE worlds w.80.q_describe_a bytes 1966 -> 2066 delta 100
  LIST /msg/data/description/ops added ['add_program', 'set_program_params', 'remove_program'] removed [] old list kept as prefix: True
  LIST /msg/data/description/queries added ['check_program', 'try_program', 'explain_program'] removed [] old list kept as prefix: True
CASE worlds w.90.q_describe_b bytes 1966 -> 2066 delta 100
  LIST /msg/data/description/ops added ['add_program', 'set_program_params', 'remove_program'] removed [] old list kept as prefix: True
  LIST /msg/data/description/queries added ['check_program', 'try_program', 'explain_program'] removed [] old list kept as prefix: True
CASE worlds w.100.q_describe_c bytes 1966 -> 2066 delta 100
  LIST /msg/data/description/ops added ['add_program', 'set_program_params', 'remove_program'] removed [] old list kept as prefix: True
  LIST /msg/data/description/queries added ['check_program', 'try_program', 'explain_program'] removed [] old list kept as prefix: True
CASE worlds w.110.q_describe_A bytes 1966 -> 2066 delta 100
  LIST /msg/data/description/ops added ['add_program', 'set_program_params', 'remove_program'] removed [] old list kept as prefix: True
  LIST /msg/data/description/queries added ['check_program', 'try_program', 'explain_program'] removed [] old list kept as prefix: True
CASE worlds w.120.q_describe_C bytes 1966 -> 2066 delta 100
  LIST /msg/data/description/ops added ['add_program', 'set_program_params', 'remove_program'] removed [] old list kept as prefix: True
  LIST /msg/data/description/queries added ['check_program', 'try_program', 'explain_program'] removed [] old list kept as prefix: True
```

`03`: bilinmeyen işlem hatasının `expected` listesi (+52 bayt). Öbür 7 olgu: `describe`
cevabı (+100 bayt = `,"add_program","set_program_params","remove_program"` 52 +
`,"check_program","try_program","explain_program"` 48).

## capsule_perf_equality (bayt toplamları)

| Senaryo | Eski | Yeni | Fark | Açıklama |
|---|---|---|---|---|
| bound, toplam | 21665331 | 21666331 | +1000 | 10 describe cevabı × 100 |
| bound, a / b / c / d | 4940270 / 4221191 / 3772143 / 4520902 | +200 her biri | +200 | 2 cevap × 100 |
| bound, e / g | 1963982 / 2147333 | +100 her biri | +100 | 1 cevap × 100 |
| bound, f / h / i | 33828 / 32308 / 33374 | aynı | 0 | describe yok |
| inline, toplam | 13110350 | 13111350 | +1000 | 10 × 100 |
| inline, a–d / e, g / f, h, i | | +200 / +100 / 0 | | aynı kural |

Satır sayıları, günlük (`log`) ve doküman (`doc`) özetleri iki senaryoda da değişmedi.
fixtures/protocol, altın görseller, views ve hesap özetleri dokunulmadı.
