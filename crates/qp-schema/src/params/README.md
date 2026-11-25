# params/

Parametre kayıt defteri (K78, `docs/ENGINEERING.md` §6). Tipler ve çözümleme
(`resolve`: nesne > kullanıcı > plan > sistem) üst dosya `params.rs`'tedir. `system.rs`
tanımlı her anahtarın `ParamDef`'ini ve `REGISTRY` listesini tutar; sahibi crate'lerdeki
adlı sabitler her anahtarın sahibin tipindeki `*_DEFAULT` sabitinden okur. `program.rs`
RL1 hesap dili satırlarını (`program.*`) tutar; sahibi sabiti yoktur, `qp_lang::Limits`
satırları doğrudan okur. `doc.rs`
`docs/PARAMETERS.generated.md`'yi üretir (`cargo run -p qp-schema --example params_doc`).
JSON şemasına (`qp schema export`) girmez. Envanter: `docs/PARAMETERS.md`.
