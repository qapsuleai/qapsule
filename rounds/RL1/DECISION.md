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

## D3 — `restricted_calls` yapılandırma anahtarı (2026-11-26)

GOAL 0.2 (izleyici tipi, A71) `qp-authority/src`'ye dokunmadan makine kuralıyla kapatıldı: yeni
sıkılaştırıcı kural `calls.restricted`, yapılandırması `scripts/arch-config.json` `restricted_calls`
(izleyici yöntemlerini yalnız `crates/qp-replica/src/replica/follower.rs` ve `qp-authority`
çağırır). Gevşetme değil, yeni kural; `config` adımı yeni anahtar için bu satırı ister
(`rule_changes` `restricted_calls`). Arkin'in GOAL'ü (ön iş 2) bu kuralın kaynağıdır.
## D2 — `unreachable` lint'i ve `qp-authority` (2026-11-26)

**Soru:** Ön iş 3 (K93) `clippy::unimplemented` ve `clippy::unreachable`'ı açar; mevcutlar öğe başına
izinle. `qp-authority/src`'de 2 `unreachable!()` var; Cargo `[lints] workspace = true` ile crate
başına geçersiz kılmayı reddediyor, lint tablosunu crate'e kopyalamak yeni istisna olur.

**Karar (Arkin): 2 özniteliği değiştir.** İki yerde var olan `#[allow(clippy::too_many_lines)]`
özniteliğine `clippy::unreachable` eklenir. Yeni satır yok, `qp-authority` izin sayısı 25'te kalır.
VERIFY 4'te `qp-authority/src` farkı yalnız bu iki öznitelik değişikliğidir (+2/−2).

**Mandal gevşetmeleri (A72, K93):** `lint.allow` `qp-compute` 27 → 30, `qp-ops` 20 → 23,
`qp-plane` 1 → 2 (var olan `unreachable!` yerlerine öğe başına izin; sayı bundan sonra yalnız düşer).

## D4 — İ8 tavanı sıkılaştırıldı (ön iş 1, 2026-11-26)

GOAL 0.1: "kazanç varsa uygulanır ve İ8 tavanı yeni değere göre sıkılaştırılır". Kabuk paketinde
`wasm-opt` (`-O1 --duplicate-function-elimination --reorder-functions --reorder-locals`) açıldı
(`rounds/RL1/experiments/o1-wasm/RESULT.md`, denetim KABUL). Kabuk paketi (`--features replica`,
verify adım 18): ham 9.004.332 → 6.598.168 B, gzip 2.485.078 → 1.939.823 B. **Yeni İ8 tavanı**
(aynı kural, yeni değer + %2,5): **ham 6.763.122 B, gzip 1.988.319 B**. Varsayılan modül
(2.354.868 / 773.639) ±%2 kuralında kalır. RL1'in sonraki işleri (qp-lang'in qp-plane'e
bağlanması) bu tavana göre ölçülür (D1 şart 1). İ8 makine kuralı değildir (RF1 denetimi);
VERIFY 6 bu satıra göre elle ölçülür.

## D5 — `describe` cevabı ve üç karakterizasyon sınaması (yürüten oturum, 2026-11-26)

Yeni işlemler (`add_program`, `set_program_params`, `remove_program`) ve sorgular (`check_program`,
`try_program`, `explain_program`) `describe` cevabına ve "bilinmeyen işlem" hatasının `expected`
listesine kendiliğinden girer. `qp-session` testlerindeki üç karakterizasyon kaydı (`charac`
transcripts/worlds, `capsule_perf_equality`) bu yüzden değişir. Bunlar `fixtures/protocol`
dökümleri değildir; R5B aynı nedenle yeniden kaydetmişti. Program işlemlerini `describe`'dan
gizlemek ajanın keşfini bozacağı için seçilmedi. Karar: **yalnız ekleme olduğu kanıtlanınca
yeniden kayıt.** Kanıt `rounds/RL1/experiments/i1/describe-diff.md`: 570 durumdan 8'i değişti,
hepsi listelerin sonuna yeni adların eklenmesi (eski liste değişmemiş önek); bayt toplamları
yalnız bu eklemelerle açıklanıyor. `fixtures/` değişmedi. PROGRAMS §17'deki "describe değişmez"
varsayımı bu kadarıyla düzeltilir (ekleme).
