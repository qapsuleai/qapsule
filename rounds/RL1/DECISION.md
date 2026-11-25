# RL1 — Arkin'in kararları

## D1 — Alt sistem sözleşmesi yetmiyor: program çıktısı hesap serisi olarak durur (2026-11-25)

**Soru (yürüten oturum, derleme denemesiyle ölçülmüş):** Alt sistem sözleşmesi iki yerde yetmiyor.

1. `ConditionKind`: yeni koşul türü `Condition`'a kol ister; bu kol `qp-authority`'de iki kapalı
   eşlemeyi kırar (`data/live.rs:257 eval_slots`, `capsule/claims.rs:222 holds_at`) ve canlı yolun
   belleği türsüz koşulu bir daha değerlendirmez (`cond_ids` boş → "etkilenmedi").
2. `ValueSource` yalnız koşul değerlendirmesinde sorulur; çizim (gramer), değer önbelleği,
   bağımlılık kapanışı ve veri yetkisi (feeds) onu görmez — yetki sızıntısı riski.

**Karar: öneriyle ilerle.** Çekirdeğe satır yok. Program çıktıları dokümanda hesap serisi olarak
durur (`SeriesSource::Study`'ye isteğe bağlı `program` alanı; `qp-plane` değeri `qp-lang` ile
hesaplar, `qp-compute` hesapları gibi). Çizim, önbellek, bağımlılık, yetki ve kanıt var olan
yoldan çalışır. Olay çıktısı yalnız kapanmış barlardan oluşan bir seridir (olay barında 1, öbür
kapanmış barlarda 0; canlı barda değer yok); canlı yolun koşulu var olan `threshold` koşuludur
(`p1.kirilim ge 1`). `ConditionKind` ve `ValueSource` bu round'da kullanılmaz.

**Üç şart:**

1. **WASM:** `qp-plane` kabuk paketinde; `qp-lang` oraya bağlanınca paket büyür. Bağlar bağlamaz
   kabuk paketi ölçülür. İ8 tavanını ön iş 1'den sonra da aşıyorsa **dur ve sor**; çalıştırıcıyı
   pakete sokmadan çözmenin yolu (özellik bayrağı, izleyicide hesap yok) önerilir.
2. **Olay birleşmesi sessiz kalmaz:** olay serisi her olay barını kaydeder (kanıt ve "dene"
   çıktısı bar bar doğru); yalnız bildirim birleşir. `check` ya da "dene", art arda barlarda doğan
   olay varsa bunu söyler. Sınır `docs/PROGRAMS.md`'de yazılı.
3. **Eksik kancalar çözümüyle yazılır:** `ConditionKind` ve `ValueSource`'un gerçekten çalışması
   için çekirdekte tam olarak hangi satırların değişmesi gerektiği (iki kapalı eşleme, canlı yol
   belleği, çizim/önbellek/bağımlılık/yetki yolları) ve bunun bayt eşit yapılıp yapılamayacağı
   raporda yazılır. R7'nin izleme görevleri bu kancalara dayanacaktı; R7'den önce karar verilecek.

**Ek:** Şema değişikliği yalnız ekleme: `program` alanı yokken var olan dökümler bayt eşit kalır.
