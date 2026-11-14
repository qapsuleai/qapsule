# v2r-void — geçersiz sayılan çalıştırma

Bu çalıştırma **sonuçlara katılmadı**. Yeniden adlandırılmış doküman (`harness/renamed/`)
üretilirken kimlik dönüştürücüsü, `macd` dizgesini yalnızca seri kimliğinde değil hesap adı
alanında da (`source.study`) değiştirdi: MACD serisi `{"study": "trend-m"}` oldu, yani
geçersiz bir hesap. Bu, özellikle G14 (MACD parametreleri) ve G15 (bardaki değerler) görevlerini
bozdu; ajan D G14'te `set_params` yerine `put_series` ile seriyi yeniden kurmak zorunda kaldı.
Hata ajan D'nin raporundan fark edildi, düzeltildi (`study` alanı dönüştürülmüyor) ve v2r
taze ajanlarla yeniden koşuldu (`raw/v2r/`). Kayıtlar şeffaflık için burada duruyor;
`SCORE.md` bilgi amaçlıdır.
