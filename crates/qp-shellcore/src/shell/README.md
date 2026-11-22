# shell/

Kabuk çekirdeğinin (`ShellCore`, `shell.rs`) konu dosyaları: gelen iletiler (`inbound`),
okurun hareketleri (`input`), yüzler ve sahneler (`scene`), bakış (`look`), akış (`feed`),
başlatma ve demo oturumu (`start`), ölçümler (`metrics`), boya listeleri (`paint*`), hareket
(`motion`, `live`), ajan (`agent`), sıklık (`freq`) ve ölçüm koşusu (`bench`).
Hepsi aynı `impl ShellCore`'un parçalarıdır; `wasm_bindgen` bağlaması değildir (o `qp-wasm`'da).
