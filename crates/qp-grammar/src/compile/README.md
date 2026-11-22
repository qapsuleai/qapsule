# compile

Derleyicinin içi: görünüm + veri sağlayıcı → sahne betimi (`docs/GRAMMAR.md` §14).
`compile.rs` seçenekleri ve küçük yardımcıları tutar; `scene.rs` `compile` işlevinin kendisi.
Burada: paneller, seriler, eksen, vurgu, çerçeve (`frame`), katman hazırlama (`layer/`),
piksel işi (`draw/`), ölçek eşlemeleri (`scale`), sözlük terimleri (`terms`) ve grafik
alfabesi (`alphabet/`: eksenler, son değer etiketi, imleç, OHLC lejantı, hacim; RD2/C2a).
Ne değildir: tasarım kayıtları (`look`, yüz, hikâye tasarımları) burada değil.
