use ziskos::zisklib::{
    add_fp2_bls12_381, conjugate_fp2_bls12_381, dbl_fp2_bls12_381, inv_fp2_bls12_381,
    mul_fp2_bls12_381, neg_fp2_bls12_381, scalar_mul_fp2_bls12_381, square_fp2_bls12_381,
    sub_fp2_bls12_381,
};

pub fn fp2_tests() {
    // Addition
    let a = [
        0xAFA88AEF27D60DD6,
        0x7DF7E8F434638498,
        0xBFCC93D3CB0F2D90,
        0xCEEFD04D135ED114,
        0x25EECFFB828CBBAA,
        0x826D79D7EF86923,
        0x19DF0F755A9FD769,
        0xDDE2BD7949239D18,
        0x5DCD501C21D1CF8A,
        0xB9E1A075672BC771,
        0xF9FDD8CF5A3A15D,
        0x75B601CA1ED5DB1,
    ];
    let b = [
        0x153035649C9482F5,
        0xD99EEB90C799310E,
        0x2CC49B67CDAC1004,
        0xC5F920DDF69C4088,
        0x49C78E029C987112,
        0x149F1B82229784D0,
        0xE13EBE1C70860D10,
        0x62010F7746D634AF,
        0xFCC9F42A71F80D3D,
        0xEC34EE9238AFB18C,
        0x78A58747CEDF5CB6,
        0x16610361B4D3413E,
    ];
    let res = add_fp2_bls12_381(&a, &b);
    let res_exp = [
        0xAD9C053C46AE620,
        0x38EAD4864AA8B5A7,
        0x85605C9AA20A4771,
        0x3071A5A61675FEDD,
        0x249AB647DBD97FE6,
        0x2C4E13568100759,
        0x411ECD91CB2639CE,
        0x2137CCF1DEA5D1C8,
        0xF36671A59D18E6A4,
        0x419F4382AC56663E,
        0x3D29BD1E8137513D,
        0x3BB51941D40B855,
    ];
    assert_eq!(res, res_exp);

    // Doubling
    let a = [
        0x54CA7CC417BE036A,
        0x3C8CDB2B78322588,
        0x7EBDA60A56060EC5,
        0x5A15FBA91BFDD70C,
        0x3FA4B708872526BB,
        0x154F4AA8F9CBE6E8,
        0xA7CE14631DB2689B,
        0xCE9A4FD34567F576,
        0x3140D5D4E6BBA545,
        0x8F48D36771B43124,
        0x43EEC61A8B03CF17,
        0x22795D8136799C8,
    ];
    let res = dbl_fp2_bls12_381(&a);
    let res_exp = [
        0xEF95F9882F7C5C29,
        0x5A6DB6583F104B10,
        0x964A7973B55B2766,
        0x4FB4ABCD44769B59,
        0x342DC65ACAFEA09F,
        0x109D8367BA17E736,
        0x4F9C28C63B64D136,
        0x9D349FA68ACFEAED,
        0x6281ABA9CD774A8B,
        0x1E91A6CEE3686248,
        0x87DD8C3516079E2F,
        0x44F2BB026CF3390,
    ];
    assert_eq!(res, res_exp);

    // Negation
    let a = [0; 12];
    let res = neg_fp2_bls12_381(&a);
    let res_exp = a;
    assert_eq!(res, res_exp);

    let a = [
        0x22C4C806BDBD9DC7,
        0x85706331F6823ACA,
        0x27C983D89463A601,
        0x8E32665B7095DA5E,
        0xABC70942D3827502,
        0x1172E4DE5792FEFF,
        0xF1FB493CCAF73D29,
        0x4086A10C6009EFFF,
        0x75E97CB0A00AB8FB,
        0x388E461FB0D93B65,
        0xA668BAC025D80F70,
        0x140547D7A0700866,
    ];
    let res = neg_fp2_bls12_381(&a);
    let res_exp = [
        0x973A37F942420CE4,
        0x993B9CCCBAD1C535,
        0x3F674EC8624D5022,
        0xD644E52982EF3861,
        0x9F549E736FC937D4,
        0x88E2D0BE1ECE79A,
        0xC803B6C335086D82,
        0xDE255EF2514A0FFF,
        0xF14755F056A63D28,
        0x2BE9056542ABD759,
        0xA4B2ECF61D739D67,
        0x5FBCA12990FDE33,
    ];
    assert_eq!(res, res_exp);

    // Subtraction
    let a = [
        0x9AFF594C7F19E070,
        0xB3C37CFBE27744A1,
        0xED12448E470A8852,
        0xC5FA714FFDCD3ABE,
        0x39BE20D8D249FE90,
        0x5CC365173E40F0F,
        0x5BE06FCF8A9E85F0,
        0x28F2CC429A463BDC,
        0xE4173B19D50D32F9,
        0xF569F38199E0E753,
        0x84252257554CF268,
        0x1835A31226C66696,
    ];
    let b = [
        0x14FF37520499AE4C,
        0xE5A3BF617512C586,
        0xE598F424166A788C,
        0x55A6F5B00D8E1EA3,
        0x3912F02071296F29,
        0x1253708C7AE7FC9D,
        0x55EC5EAF9113E200,
        0x1F1B37D1F7D7A561,
        0xE099205D9488DEBD,
        0x80DF97B042CD2145,
        0x927AAD9DB56AC60A,
        0x11F52058FF9BB8F8,
    ];
    let res = sub_fp2_bls12_381(&a, &b);
    let res_exp = [
        0x3FFF21FA7A7FDCCF,
        0xECCBBD991EB87F1B,
        0x6EAA230B275105E9,
        0xD4CAC724E3C42EDA,
        0x4BC6D86EA46C3C3E,
        0xD79D7AF327BF90C,
        0x5F4111FF98AA3F0,
        0x9D79470A26E967B,
        0x37E1ABC4084543C,
        0x748A5BD15713C60E,
        0xF1AA74B99FE22C5E,
        0x64082B9272AAD9D,
    ];
    assert_eq!(res, res_exp);

    // Multiplication
    let a = [
        0x9AFF594C7F19E070,
        0xB3C37CFBE27744A1,
        0xED12448E470A8852,
        0xC5FA714FFDCD3ABE,
        0x39BE20D8D249FE90,
        0x5CC365173E40F0F,
        0x5BE06FCF8A9E85F0,
        0x28F2CC429A463BDC,
        0xE4173B19D50D32F9,
        0xF569F38199E0E753,
        0x84252257554CF268,
        0x1835A31226C66696,
    ];
    let b = [
        0x14FF37520499AE4C,
        0xE5A3BF617512C586,
        0xE598F424166A788C,
        0x55A6F5B00D8E1EA3,
        0x3912F02071296F29,
        0x1253708C7AE7FC9D,
        0x55EC5EAF9113E200,
        0x1F1B37D1F7D7A561,
        0xE099205D9488DEBD,
        0x80DF97B042CD2145,
        0x927AAD9DB56AC60A,
        0x11F52058FF9BB8F8,
    ];
    let res = mul_fp2_bls12_381(&a, &b);
    let res_exp = [
        0x7394DA288049DF2C,
        0x115ECE6E1D13F8B9,
        0x5745086B20BEEE4B,
        0xF4FC52147B6605E5,
        0x34BB88DBC7251500,
        0x32AA465640A2B33,
        0xF9E64D2C9B34FD9E,
        0x688FB6A3BB75CAFA,
        0x445861C60CEDDE7D,
        0x5B87CF128EBCE9B6,
        0x88962B7D8863F32,
        0x6EB10C931C954B0,
    ];
    assert_eq!(res, res_exp);

    // Scalar multiplication
    let a = [
        0x860D7DBA85FEC994,
        0xD3CDC823C1857543,
        0x891A654FAA65754,
        0xB94D01AF3D80BA5C,
        0x6F1685595D40A15E,
        0x146B0679EB7C3AB,
        0xF87041078CB7C5A4,
        0x55E354B9E2143B55,
        0x72DDBA619E3A4A71,
        0xBA20B37B78A3A5DB,
        0x176725B899AC37A6,
        0x79C0D3E566A67D9,
    ];
    let b = [
        0x8098C06781B3EBAE,
        0x175979EF02778FF1,
        0x19911DA77B6CE291,
        0x46B8778C757772B6,
        0x1543764FD010DE7D,
        0x11F070E7AEB68671,
    ];
    let res = scalar_mul_fp2_bls12_381(&a, &b);
    let res_exp = [
        0x6697F06E852B002C,
        0xA340D4AA36936037,
        0xDF1B766E87EACF69,
        0x8DDC4CC8E0A329F6,
        0xEE898AD58742075A,
        0x29768F883A57DF9,
        0xA2E18496E0E55456,
        0x48C1423B009D7E8,
        0xCB4668C5BF50EF7C,
        0x429E9AFC97605415,
        0xD511B0F48CE9580F,
        0xF197441660DD204,
    ];
    assert_eq!(res, res_exp);

    // Squaring
    let a = [
        0xE839AF30641641E9,
        0xA94593BF4B55BD33,
        0x145C3CA1BACF893E,
        0xB3EB7171E24C9135,
        0x685E275CFF697B5E,
        0xCD76FCAEC30AD1D,
        0x19B4F27C2E26D135,
        0x979643E755355FF1,
        0xFC560E9D9BDBD9E7,
        0x59D1E4F17C9EAE20,
        0x327A7DB732741C43,
        0x2599860F5822B1D,
    ];
    let res = square_fp2_bls12_381(&a);
    let res_exp = [
        0xD27E32B93DCF89CF,
        0xBDB222068C752CF0,
        0xF7539E5B185FE144,
        0xDCD15D99632E91BE,
        0x8CD05760A1C98EFC,
        0x15D6509B042F58AF,
        0x322F8772DA6BB4DD,
        0x5A95AED52CA4973C,
        0x89F17478471F15DA,
        0xBBB8A0A6C3293075,
        0x94385657156E2842,
        0x15F96975916502A6,
    ];
    assert_eq!(res, res_exp);

    // Inversion
    let a = [0; 12];
    let res = inv_fp2_bls12_381(&a);
    let res_exp = a;
    assert_eq!(res, res_exp);

    let a = [
        0x98C81A73F4E49B06,
        0xE2ADC65A7A440EF2,
        0x90E03613EB9201A1,
        0x8F7A37886C4F453,
        0xCE14C9E634E57986,
        0xDD6F73411C9CBF6,
        0x9A4B0DB9E0725DD1,
        0x31B0E537E5CC8C39,
        0x6923B68528B6AA75,
        0x41858BC4FE068DD3,
        0x49257C491B1C7418,
        0xD76CB528782C322,
    ];
    let res = inv_fp2_bls12_381(&a);
    let res_exp = [
        0xDC85405AB2A52B0D,
        0x137356630644BFAA,
        0x1D0542BB2C383006,
        0x1147BF719E36C782,
        0xA0D9E646E2DCB8B0,
        0x133A2F37FC662D0A,
        0xEBC82BC86E7A549C,
        0x31BCA02C8E3D7020,
        0xFD66801177589146,
        0xF96D7857C0F3A6EF,
        0x6F2D5FE63A698F07,
        0x49BFB72AAE1980,
    ];
    assert_eq!(res, res_exp);

    // Conjugate
    let a = [0; 12];
    let res = conjugate_fp2_bls12_381(&a);
    let res_exp = a;
    assert_eq!(res, res_exp);

    let a = [
        0x9576BA03C6217197,
        0x32F4672815838154,
        0x13411066FC88DD13,
        0x439894C349A0F1FA,
        0x6CB0CFA46B5ED635,
        0x668A30B168C7952,
        0x74361CB16E8FD610,
        0x85D2A3D5E4981FC3,
        0xA7B48F873EAFC30B,
        0xC071057CFAA288B,
        0x7504D3E0422A9ABA,
        0x10AA1605D1279203,
    ];
    let res = conjugate_fp2_bls12_381(&a);
    let res_exp = [
        0x9576BA03C6217197,
        0x32F4672815838154,
        0x13411066FC88DD13,
        0x439894C349A0F1FA,
        0x6CB0CFA46B5ED635,
        0x668A30B168C7952,
        0x45C8E34E916FD49B,
        0x98D95C28CCBBE03C,
        0xBF7C4319B8013318,
        0x58703B2D23DAEA33,
        0xD616D3D60121121D,
        0x956FBE468585496,
    ];
    assert_eq!(res, res_exp);
}
