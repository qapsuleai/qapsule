# R4 — `qp replay` geri seek'ten sonra susuyordu (FIX-REPLAY, denetim Ö3)

Dal `r4/fix-replay` (taban `r4/int` 39fcaf0). Dokunulan: `crates/qp-cli/src/main.rs`
(`replay_cmd`), yeni `crates/qp-cli/tests/replay.rs`, `scripts/verify.sh` adım 16.
`qp-session` (`ReplayHost`) değişmedi.

## Ne, neden

Geri `seek` yeni otoritedir (PROTOCOL §21.3: `reset: true`, `epoch + 1`, bütün bağlantılar
kapanır, istemci yeniden `hello` der). Gösterim kabuğu bunu yapıyor, `qp replay` yapmıyordu:
aracın izleyen bağlantısı (`arkin`, `capsules` + `changes` aboneliği) kapandıktan sonra sonraki
denetimler `handshake` hatasıyla döndü, bildirim satırı da gelmedi. verify adım 16'nın
denetim listesinin son öğesi geri seek olduğundan akışın o noktadan sonrası boştu. "İki koşu
bayt eşit" iddiası da o kısım için hiçbir şey sınamıyordu.

Düzeltme: bağlantıyı kuran `hello` ve iki abonelik tek bir `connect` kapanışında toplandı.
Bir denetimin cevabındaki `ReplayState.reset` `true` ise araç aynı `hello`'yu (aynı ilke ve
yüzeyle) ve aynı abonelikleri hemen yeniden gönderiyor. Yeniden kurulma satırları
(`welcome`, iki `ack`) cevabın hemen ardından akışa yazılıyor. Duvar saati ya da rastgelelik
yok, bu yüzden araç belirleyici kalıyor.

## Test

`crates/qp-cli/tests/replay.rs::backward_seek_reconnects_and_the_stream_goes_on`: sentetik pazar
(seed 7, 6 enstrüman, 200 gün, 160 adım). Denetimler: ileri `step`, geri `seek`, `status`,
geri `seek`. Test şunları istiyor: iki koşu bayt eşit, hiç `error` satırı yok, iki `reset`,
son reset'te `epoch` 2, ardından `welcome` ve `caps`/`ch` için `ack`, son geri seek'ten sonra
en az bir `notify`, iki geri seek arasındaki `status` bir `result` döndürüyor.
Eski kodla düşüyor (`error line: {"code":"handshake", …}`), yeni kodla geçiyor.

## Önce / sonra (satır sayıları)

| Koşu | Önce | Sonra |
|------|------|-------|
| verify adım 16 (`--steps 300`, 3 denetim; son denetim geri seek) | 693 satır, son geri seek'ten sonra 0 | 1816 satır, son geri seek'ten sonra 1120 `notify` |
| denetim yeniden üretimi (`--seed 7 --instruments 12 --days 300 --steps 400`, `r4/audit` replay-controls.json) | 146 satır, 4 `handshake` hatası | 1628 satır, 0 hata, son geri seek'ten sonra 450 `notify` |

İki koşu her iki durumda da bayt eşit.

## verify

Adım 16 artık şunları da denetliyor: hiç `error` satırı yok, akışta en az bir `reset: true`
var, son reset'ten sonra en az bir `welcome` ve en az bir `notify` geliyor. Not metni bu
sayıyı basıyor. Eski koşunun çıktısı bu denetimden geçmiyor (son geri seek'ten sonra 0
`welcome` ve 0 `notify`).

`QP_VERIFY_BROWSER=1 scripts/verify.sh`: 18/18 PASS, `verify: OK` (467 sn). Adım 16 notu:
"two runs byte-identical (1816 lines; 1120 notifications after the last backward seek)".
Dökümler (`fixtures/protocol/conformance/*`), altınlar ve hesap özeti (`7b1e99d40889f83c`)
değişmedi; çalışma ağacında başka değişiklik yok.
