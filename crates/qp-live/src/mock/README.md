# mock/

Yalnız 127.0.0.1 üstündeki test ikizlerinin parçaları (`mock.rs` kök modülü aracıyı,
ayarları ve kayıt alıcısını taşır): `issuer.rs` HTTP belirteç hizmeti (`MockTokenService`),
`serve.rs` aracının tek bağlantı sunucu tarafı. Gerçek bir sunucuya hiç gitmez;
dışa açık yollar `qp_live::mock::…` olarak kalır.
