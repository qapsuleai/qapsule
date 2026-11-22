# realize/

R3 cümle üretici (şablon dilbilgisi, TR + EN; `qp_capsule::realize::Template`). `plan.rs`
girdiyi dilden bağımsız çerçeveye okur, `tr.rs` / `en.rs` çerçeveyi yazar, `template.rs`
üreticiyi ve rakamsız okur korumasını taşır; `morph.rs` Türkçe ekler, `num.rs` sayı biçimi,
`elapsed.rs` geçen süre, `parts.rs` R4 yüz parçaları, `stamp.rs` satır zaman damgası.
Sesler (`voice/`) bu parçaları kullanır; burada ses yok.
