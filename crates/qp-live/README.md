# qp-live

Canlı Matriks bağlantısının çekirdeği (R6/T2). Yalnız sunucu tarafında kullanılır; WASM
paketine girmez. Ağ yalnız bu crate'tedir; hesap çekirdeği ağ bilmez.

## Nasıl çalışır

1. `LiveConfig::from_env()` değerleri yalnız ortamdan okur (dosya okumaz).
2. `AuthClient` belirteç servisine `POST` + HTTP Basic + `{"typeCode","hardwareId"}` gönderir;
   belirteç bellekte durur, ömrü JWT `exp`'ten (imza doğrulanmadan) okunur.
3. `LiveConnection::open(cfg, endpoint, auth, opts, sink)` uç nokta kilitlerini alır ve bağlantı
   başına bir iş parçacığı başlatır (tokio yok): TCP → TLS (rustls + ring, webpki kökleri,
   sertifika denetimi açık) → WebSocket (`Sec-WebSocket-Protocol`, varsayılan `mqttv3.1`) →
   MQTT CONNECT (varsayılan 3.1 `MQIsdp`; `JWT` / belirteç; temiz oturum; keepalive 30 sn) →
   konular tek tek (ya da `subscribe_batch` partileriyle) → PINGREQ.
4. Gelen her PUBLISH `LiveSink::on_message`'a (konu, yük, yerel sıra no, oturum no, varış
   zamanı UTC µs), bağlantı olayları `LiveSink::on_event`'e gider. Crate kendisi hiçbir şey
   yazdırmaz.
5. MQTT kodlayıcı kendi kodumuz (`mqtt.rs`): yeniden bağlanma tümüyle bizim denetimimizde.

## Koruma kuralları nerede

| Kural | Kod | Test |
|---|---|---|
| Uç nokta başına süreç içi tek bağlantı | `lock::claim_process_slot`, `LiveConnection::open` | `tests/live.rs::second_open_in_same_process_is_refused`, `lock::tests` |
| Makinede tek süreç (`fixtures/local/live-<uç>.lock`, işletim sistemi kilidi, PID dosyada) | `lock::acquire_machine_lock`, `lock::default_lock_dir` (worktree'den de ana ağacın klasörü) | `tests/two_process.rs` (VERIFY 3) |
| Aynı uç noktaya eşzamanlı iki soket asla | `Worker::close_ws` (DISCONNECT + WebSocket kapanış el sıkışması) her yeni bağlantıdan önce | `max_concurrent == 1` denetimleri: `token_renewal_closes_old_connection_before_opening_new`, `drop_backs_off_…`, `connack_auth_refusal_refreshes_token_once` |
| "Çoklu bağlantı" = dur, yeniden deneme yok | `Notice::MultiConnect` → `SessionEnd::Halt` → `Halted(MultiConnect)`; süreç içi yasak + `live-<uç>.halted` işaret dosyası; yalnız `reset_multiconnect_halt(dir, key, not)` açar | `multiconnect_notice_halts_without_retry_until_reset` (VERIFY 4) |
| Ölçülü yeniden bağlanma | `backoff.rs` (2 sn, ×2, tavan 60 sn, jitter yok, 6 ardışık hatada dur; 60 sn sağlam kalan oturum sayacı sıfırlar) | `consecutive_failures_halt_with_deterministic_backoff`, `backoff::tests` |
| Yeniden bağlanmada yeni belirteç yok (yalnız süre dolunca ya da CONNACK 4/5'te bir kez) | `Worker::run`, `AuthClient::token` | `drop_backs_off_…`, `reconnect_storm_does_not_hammer_the_token_service`, `connack_auth_refusal_*` |
| Belirteç isteği sıklığı sınırlı (en az 30 sn; hatada ×2, tavan 15 dk; 5 ardışık hatada dur) | `AuthClient::fetch` | `valid_token_is_reused_and_requests_are_rate_limited`, `failures_back_off_exponentially_and_stop` |
| Kimlik `qapsule-<makine adı>`, belirleyici, rastgele yok; önek yoksa uyarı bayrağı | `config::derive_hardware_id`, `LiveConfig::hardware_id_warning` | `config::tests` |
| Gizli bilgi maskeleme | `redact::Redactor` (tek yol); bütün hata/olay metinleri kurulurken maskelenir (`error::Masked`); `Debug` elle | `tests/auth_and_redaction.rs::no_secret_reaches_any_log_or_error_text` (VERIFY 5) |
| Sertifika denetimi açık; düz `ws://` yalnız loopback | `transport::tls_config`, `config::Endpoint::parse` | `transport::tests::tls_rejects_self_signed_certificate` (`--ignored`, `openssl` ister), `config::tests` |
| Yönlendirme izlenmez, vekil kullanılmaz (Basic bilgisi başka adrese gitmez) | `AuthClient::new` (`max_redirects(0)`, `proxy(None)`) | — |

Açık bağlantıda belirteç yenileme `ConnectOptions::renewal` ile seçilir: varsayılan
`OnReconnectOnly` (açık bağlantı sürer); `ReconnectBeforeExpiry` önce yeni belirteci alır,
eskisinden farklıysa eski bağlantıyı kapatır, sonra yenisini açar.

## Ortam değişkenleri (adlar)

Gerekli: `MATRIKS_TOKEN_SERVICE_URL`, `MATRIKS_API_USERNAME`, `MATRIKS_API_PASSWORD`,
`MATRIKS_SERVICE_TYPE_CODE`, `MATRIKS_MQTT_MODE` (`delayed` | `realtime`) ve kipe göre
`MATRIKS_MQTT_DL_HOST` ya da `MATRIKS_MQTT_RT_HOST` (tam adres `wss://host/market` ya da
yalnız ana makine; şema yoksa `wss://`, yol yoksa `/market` eklenir).

İsteğe bağlı: `MATRIKS_HARDWARE_ID` (boşsa `qapsule-<makine adı>`), `MATRIKS_MQTT_STATS_HOST`,
`MATRIKS_MQTT_NEWS_HOST`, `MATRIKS_TOKEN` (yalnız maskelenir), `QP_LIVE_MQTT_PROTOCOL`
(`3.1` | `3.1.1`), `QP_LIVE_WS_SUBPROTOCOL` (`none` = başlık yok), `QP_LIVE_MQTT_KEEPALIVE_SECS`,
`QP_LIVE_LOCK_DIR`.

Eksik değişkende hata yalnız adı söyler.

## Test

```sh
cargo test -p qp-live                                   # yalnız 127.0.0.1; dış ağ yok
cargo test -p qp-live -- --ignored tls_rejects          # openssl ile TLS doğrulama denemesi
```

Sahte sunucu (`mock` özelliği): `MockBroker` (düz `ws://` MQTT; CONNACK kodu, konu başına
SUBACK, ilk durum iletileri, yayın, çoklu bağlantı bildirimi, bağlantı koparma, el sıkışma
reddi; istatistik: en büyük eşzamanlı bağlantı sayısı), `MockTokenService` (`exp`'li sahte JWT,
istek sayımı, 500 / `success:false`), `RecordingSink`, `test_classifier`, `mock_config`.
Başka crate'lerin testleri: `qp-live = { path = "../qp-live", features = ["mock"] }`.
