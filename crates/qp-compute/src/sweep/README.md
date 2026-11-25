# sweep/

Tarama (`sweep.rs`) içindeki paylaşılan alt hesaplar: `share.rs` aynı parametre önekine sahip
varyantların ortak kısmını (macd çizgisi, bollinger orta/sapma) bir kez koşturur; çıktılar tek
koşuyla bit eşit kalır. Tarama sürücüsü ve özetleri `sweep.rs`'tedir.
`rl1.rs` RL1 hesap dili çekirdeklerinin (`stdev`, `percentrank`) ayrı özetidir
(`rl1_studies_digest`, `fixtures/compute/digest-rl1.txt`); eski özetler değişmez.
