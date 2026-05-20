use ziskos::zisklib::{
    add_fp2_bn254, conjugate_fp2_bn254, dbl_fp2_bn254, inv_fp2_bn254, mul_fp2_bn254, neg_fp2_bn254,
    scalar_mul_fp2_bn254, square_fp2_bn254, sub_fp2_bn254,
};

pub fn fp2_tests() {
    // Addition
    let a = [
        0xD49C1D68AFF6B445,
        0xAC991FC7702BEFC3,
        0x061D4D434BE1ABD7,
        0x2674110820EE8DC8,
        0x7DC138B505EDCCB3,
        0x4C8FCCE97AF7D3F6,
        0x6E247A9876D82D51,
        0x1413CBE73851D8D2,
    ];
    let b = [
        0x71FE61CB3ED75FC1,
        0x168F81023BB57E98,
        0x9664418158828792,
        0x11DC32424D62A6F6,
        0x8254ED32E0BF1F82,
        0xA470A3A7A755443A,
        0x4F68189AF21A6B35,
        0x01239AEC3CA5514A,
    ];
    let res = add_fp2_bn254(&a, &b);
    let res_exp = [
        0x0A79F31D165116BF,
        0x2BA73638436FA3CF,
        0xE431490E22E2DB0C,
        0x07EBF4D78D1F9494,
        0x001625E7E6ACEC35,
        0xF1007091224D1831,
        0xBD8C933368F29886,
        0x153766D374F72A1C,
    ];
    assert_eq!(res, res_exp);

    // Doubling
    let a = [
        0xD49C1D68AFF6B445,
        0xAC991FC7702BEFC3,
        0x061D4D434BE1ABD7,
        0x2674110820EE8DC8,
        0x7DC138B505EDCCB3,
        0x4C8FCCE97AF7D3F6,
        0x6E247A9876D82D51,
        0x1413CBE73851D8D2,
    ];
    let res = dbl_fp2_bn254(&a);
    let res_exp = [
        0x6D17AEBA87706B43,
        0xC1B0D4FD77E614FA,
        0x53EA54D01641FF51,
        0x1C83D39D60AB7B66,
        0xFB82716A0BDB9966,
        0x991F99D2F5EFA7EC,
        0xDC48F530EDB05AA2,
        0x282797CE70A3B1A4,
    ];
    assert_eq!(res, res_exp);

    // Negation
    let a = [0; 8];
    let res = neg_fp2_bn254(&a);
    let res_exp = a;
    assert_eq!(res, res_exp);

    let a = [
        0xD49C1D68AFF6B445,
        0xAC991FC7702BEFC3,
        0x061D4D434BE1ABD7,
        0x2674110820EE8DC8,
        0x7DC138B505EDCCB3,
        0x4C8FCCE97AF7D3F6,
        0x6E247A9876D82D51,
        0x1413CBE73851D8D2,
    ];
    let res = neg_fp2_bn254(&a);
    let res_exp = [
        0x67846EAE28864902,
        0xEAE84AC9F845DAC9,
        0xB232F873359FAC85,
        0x09F03D6AC0431261,
        0xBE5F5361D28F3094,
        0x4AF19DA7ED79F696,
        0x4A2BCB1E0AA92B0C,
        0x1C50828BA8DFC757,
    ];
    assert_eq!(res, res_exp);

    // Subtraction
    let a = [
        0xD49C1D68AFF6B445,
        0xAC991FC7702BEFC3,
        0x061D4D434BE1ABD7,
        0x2674110820EE8DC8,
        0x7DC138B505EDCCB3,
        0x4C8FCCE97AF7D3F6,
        0x6E247A9876D82D51,
        0x1413CBE73851D8D2,
    ];
    let b = [
        0x71FE61CB3ED75FC1,
        0x168F81023BB57E98,
        0x9664418158828792,
        0x11DC32424D62A6F6,
        0x8254ED32E0BF1F82,
        0xA470A3A7A755443A,
        0x4F68189AF21A6B35,
        0x01239AEC3CA5514A,
    ];
    let res = sub_fp2_bn254(&a, &b);
    let res_exp = [
        0x629DBB9D711F5484,
        0x96099EC53476712B,
        0x6FB90BC1F35F2445,
        0x1497DEC5D38BE6D1,
        0xFB6C4B82252EAD31,
        0xA81F2941D3A28FBB,
        0x1EBC61FD84BDC21B,
        0x12F030FAFBAC8788,
    ];
    assert_eq!(res, res_exp);

    // Multiplication
    let a = [
        0xD49C1D68AFF6B445,
        0xAC991FC7702BEFC3,
        0x061D4D434BE1ABD7,
        0x2674110820EE8DC8,
        0x7DC138B505EDCCB3,
        0x4C8FCCE97AF7D3F6,
        0x6E247A9876D82D51,
        0x1413CBE73851D8D2,
    ];
    let b = [
        0x71FE61CB3ED75FC1,
        0x168F81023BB57E98,
        0x9664418158828792,
        0x11DC32424D62A6F6,
        0x8254ED32E0BF1F82,
        0xA470A3A7A755443A,
        0x4F68189AF21A6B35,
        0x01239AEC3CA5514A,
    ];
    let res = mul_fp2_bn254(&a, &b);
    let res_exp = [
        0x4B3D18D42DB65B1A,
        0x2707194A77A4063A,
        0xE81DCED2EC31B22E,
        0x1418E999CE8F2AC3,
        0x77C705C5915BBC51,
        0xAE3483B9771360DA,
        0x0328234986F39F02,
        0x25F3C9A0D02DBF67,
    ];
    assert_eq!(res, res_exp);

    // Scalar multiplication
    let a = [
        0xD49C1D68AFF6B445,
        0xAC991FC7702BEFC3,
        0x061D4D434BE1ABD7,
        0x2674110820EE8DC8,
        0x7DC138B505EDCCB3,
        0x4C8FCCE97AF7D3F6,
        0x6E247A9876D82D51,
        0x1413CBE73851D8D2,
    ];
    let b = [0x71FE61CB3ED75FC1, 0x168F81023BB57E98, 0x9664418158828792, 0x11DC32424D62A6F6];
    let res = scalar_mul_fp2_bn254(&a, &b);
    let res_exp = [
        0x2E30A97BBC4E640E,
        0x6DC3535B359F225A,
        0x2E8595980DD38011,
        0x2BB079D81A8DA535,
        0xA10771DE25208013,
        0x0686E78577AFCEEE,
        0xEAC7CD6C7BE9F011,
        0x11400258CD46D0BB,
    ];
    assert_eq!(res, res_exp);

    // Squaring
    let a = [
        0xD49C1D68AFF6B445,
        0xAC991FC7702BEFC3,
        0x061D4D434BE1ABD7,
        0x2674110820EE8DC8,
        0x7DC138B505EDCCB3,
        0x4C8FCCE97AF7D3F6,
        0x6E247A9876D82D51,
        0x1413CBE73851D8D2,
    ];
    let res = square_fp2_bn254(&a);
    let res_exp = [
        0x2EF08DCE6DD1E067,
        0xB270A8D576E3DE7C,
        0x30FF3C5DEF4D32B4,
        0x1A131BBA0A14E33A,
        0x612851335B831058,
        0xC3D1E96F945BCD05,
        0xC0074B79FB7953EE,
        0x297414CF9435B47B,
    ];
    assert_eq!(res, res_exp);

    // Inversion
    let a = [0; 8];
    let res = inv_fp2_bn254(&a);
    let res_exp = a;
    assert_eq!(res, res_exp);

    let a = [
        0xD49C1D68AFF6B445,
        0xAC991FC7702BEFC3,
        0x061D4D434BE1ABD7,
        0x2674110820EE8DC8,
        0x7DC138B505EDCCB3,
        0x4C8FCCE97AF7D3F6,
        0x6E247A9876D82D51,
        0x1413CBE73851D8D2,
    ];
    let res = inv_fp2_bn254(&a);
    let res_exp = [
        0x56746946512BA596,
        0xC90E5AA79CC0459F,
        0xB081DA5EF22BC384,
        0x0A5498A557C2C30A,
        0xC3564EE435C9B533,
        0x926A463E3A7FFE1E,
        0xEBC7C39636019AC6,
        0x1A8CF1FD16B0E8E3,
    ];
    assert_eq!(res, res_exp);

    // Conjugate
    let a = [0; 8];
    let res = conjugate_fp2_bn254(&a);
    let res_exp = a;
    assert_eq!(res, res_exp);

    let a = [
        0xD49C1D68AFF6B445,
        0xAC991FC7702BEFC3,
        0x061D4D434BE1ABD7,
        0x2674110820EE8DC8,
        0x7DC138B505EDCCB3,
        0x4C8FCCE97AF7D3F6,
        0x6E247A9876D82D51,
        0x1413CBE73851D8D2,
    ];
    let res = conjugate_fp2_bn254(&a);
    let res_exp = [
        0xD49C1D68AFF6B445,
        0xAC991FC7702BEFC3,
        0x061D4D434BE1ABD7,
        0x2674110820EE8DC8,
        0xBE5F5361D28F3094,
        0x4AF19DA7ED79F696,
        0x4A2BCB1E0AA92B0C,
        0x1C50828BA8DFC757,
    ];
    assert_eq!(res, res_exp);
}
