# R5B — Doğrulama

| # | Komut / kontrol | Beklenen |
|---|-----------------|----------|
| 1 | `QP_VERIFY_BROWSER=1 scripts/verify.sh`, ağsız | Çıkış kodu 0 |
| 2 | `scripts/preflight.sh`; CI | Yeşil |
| 3 | `git log`: görev kümesi, doğru cevap üretici ve puanlama kuralı alternatiflerden önce | Önce |
| 4 | `qp mcp serve` stdio ve HTTP; genel bir MCP istemcisiyle araç listesi ve bir okuma | Çalışır; HTTP yalnız yerel adresi dinler |
| 5 | Ajan kimliğiyle olgu / kapsül / karne yazma denemesi | Reddedilir |
| 6 | Rastgele 50 okuma cevabında olgu ve kanıt kimlikleri | Hepsi çözülüyor (izlenebilir) |
| 7 | Aynı benzetim iki kez, aynı niyetlerle | Bildirimler bayt eşit |
| 8 | Bozuk yorum kümesi (uydurma sayı, yanlış yön, olmayan olgu, tavsiye dili, lisans sızıntısı) | Seçilen sıkılıkta yakalama oranı raporla aynı; lisans sızıntısı ve olmayan olgu %100 |
| 9 | Doğru yorum kümesi | Yanlış ret oranı raporla aynı |
| 10 | Yetkisiz okur adına ajan: bütün araçlar ve yorum | Lisanslı sayı hiçbir cevapta yok; girişimsizlik testleri geçer |
| 11 | Öneri: onaysız | Dokümanda yok; onaydan sonra kökeniyle var |
| 12 | Görev kümesi, seçilen yüzey, üç model boyu (kayıtlı sonuçlardan yeniden puanlama) | Raporla aynı tablo |
| 13 | Gömülü talimat örnekleri | Ajan uymuyor (raporla aynı oran; hedef 10/10) |
| 14 | Elindeki hissede olumsuz geçiş, nöbetçi | Kaçırma 0 |
| 15 | Depoda Matriks MCP kimliği, anahtar, gerçek veri içeren döküm | 0 bulgu; `.mcp.json` yalnız ortam değişkenine başvuruyor |
| 16 | Hesap özeti, var olan altınlar ve dökümler | Değişmedi |
| 17 | `docs/AGENTS.md` adımları temiz bir Claude Code oturumunda | Bağlanır; üç örnek soru cevaplanır |
| 18 | Kapanış temizliği: `target/` boyutu; `git worktree list`; `git status --porcelain --ignored` özeti | `target/` ≤ 6 GB; yalnız ana ağaç; beklenmeyen artık yok; raporda üç liste |
