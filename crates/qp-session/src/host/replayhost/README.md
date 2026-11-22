# host/replayhost/

`replayhost.rs`'in parçaları: barındırıcının kurulması (`setup`: beslemeler, genişletilmiş
init, pencere, canlı), mesajlar ve oynatma denetimi (`control`) ve teslim edilen veri
olaylarının kaydı (`recorded`).
Kaynak sağlayıcı değildir; beslemeler `qp-source`'tan gelir.
