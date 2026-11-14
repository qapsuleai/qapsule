# Durum

Bu dosya, Claude Code oturumları ile ana akış oturumu arasındaki köprüdür.
Her round sonunda güncellenir.

## Şu an

- **Aktif round:** R1 (koşulmayı bekliyor)
- **Son güncelleme:** 2026-11-14
- **Sıradaki adım:** Arkin GitHub deposunu açar; `rounds/R1/PROMPT.md` içeriği Claude Code'a verilir.

## Round geçmişi

| Round | Sonuç | Rapor |
|-------|-------|-------|
| — | Temel dokümanlar yazıldı | — |
| R0 | Tamamlandı: 8/8 kabul ölçütü, VERIFY 13/13. Şema v0, `qp` CLI, deterministik referans çizici (tiny-skia + fontdue), wasm + tarayıcı 0 px fark, A1 → K12. | `rounds/R0/REPORT.md`, `rounds/R0/AUDIT.md` |

## Bilinen durumlar

- Eski prototip `../Services/quantex-charts-core` ve `../Docs/q-charts` altında, yalnızca
  referans olarak duruyor. Testleri bu çalışma sırasında çalıştırılıp doğrulanmadı.
- Eski repoda `ts/.env` içinde bir Matriks token'ı var ve eski `.gitignore` bunu
  dışlamıyor. Yeni repoya hiçbir gizli bilgi taşınmayacak.
- Eski klasörde git geçmişi yok.

- Depoda git remote yok; CI (`.github/workflows/ci.yml`) henüz hiç çalışmadı. Gerçek x86_64/Linux üzerinde altın görsel ilk push'ta sınanacak.
- `scripts/verify.sh` tek komutluk doğrulama girişi. Tarayıcı adımı `QP_VERIFY_BROWSER=1` ile zorunlu, `=0` ile kapalı.

## Arkin'den beklenenler

- R1 öncesi: depo açıldı (https://github.com/qapsuleai/qapsule). Bu makinede SSH takma adı `github-qapsule` ve `gh` oturumu kurulacak.
- K14 (bar hizası borsaya göre, gösterim kullanıcıya göre) onayı.
- A2: Qapsule adı için marka ön taraması ve alan adı kararı.
- A4 (hukuk) ve A5 (ajan ortamı) kararları.
- R6 öncesi: NATS akışlarından birkaç örnek mesaj (konu adı ve içerik biçimi).
- Deponun konacağı uzak sunucu (CI'ın ilk çalışması için).
- Eski `ts/.env` içindeki token'ın iptali ve yenilenmesi (eski `.gitignore` onu dışlamıyor).
- R6 öncesi: Matriks protobuf (`.proto`) tanımlarının kaynağı.
