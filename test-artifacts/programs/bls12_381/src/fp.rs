use ziskos::zisklib::{
    add_fp_bls12_381, inv_fp_bls12_381, mul_fp_bls12_381, neg_fp_bls12_381, square_fp_bls12_381,
};

pub fn fp_tests() {
    // Addition
    let a = [
        0x9EE55D8A853B75A1,
        0xE0D271DE0EDF0B6E,
        0xABC21C78615AE13,
        0xFDA41D97F44531D2,
        0xCA22793961B78A73,
        0x11BF69B7BEDE0658,
    ];
    let b = [
        0x6837321513C65F63,
        0x1924A70E855889F,
        0xE33271566DD2B4F3,
        0xEBF91EA1FE42CE46,
        0xD349C1060223F555,
        0xA11A0ACC66DE7EB,
    ];
    let res = add_fp_bls12_381(&a, &b);
    let res_exp = [
        0x4D1D8F9F99022A59,
        0xC3B8BC5045E0940E,
        0x86BDC07CFD376CE2,
        0x8525F0B4FF02ED59,
        0x52509289208FD2F2,
        0x1CFF87A4BCC07AA,
    ];
    assert_eq!(res, res_exp);

    // Negation
    let a = [0; 6];
    let res = neg_fp_bls12_381(&a);
    let res_exp = a;
    assert_eq!(res, res_exp);

    let a = [
        0x9A59EB34AA137E1,
        0x5222DAF086D6A0A8,
        0xF1371C121E97DBFF,
        0x84BA31A8BDDDB847,
        0xB2466459FFBDCF61,
        0xF975A82009249E5,
    ];
    let res = neg_fp_bls12_381(&a);
    let res_exp = [
        0xB059614CB55E72CA,
        0xCC89250E2A7D5F57,
        0x75F9B68ED8191A24,
        0xDFBD19DC35A75A77,
        0x98D5435C438DDD75,
        0xA69B76838ED9CB4,
    ];
    assert_eq!(res, res_exp);

    // Multiplication
    let a = [
        0xB794A4D7E29A8043,
        0x75B7D3620592E10,
        0xEB0E92EE5A053EA0,
        0x9EB508299ABB0BA1,
        0xD29A8398F4F0F9B4,
        0x93691EB351BDCE0,
    ];
    let b = [
        0x7BA5D77A7ECEB005,
        0x2210118885B7D0DB,
        0xA74DC95BD1D43A6D,
        0xFC228367BF4B8C10,
        0xE22E01F6FE61A2E5,
        0x1229C1DE296EF3A5,
    ];
    let res = mul_fp_bls12_381(&a, &b);
    let res_exp = [
        0xAE6B8D4A24330787,
        0xDEB52B2B74C4A083,
        0xBB17822644533B97,
        0x816EC1FB52C84088,
        0xA622FF9487F863C2,
        0x1326D7932E4A97EB,
    ];
    assert_eq!(res, res_exp);

    // Squaring
    let a = [
        0x7241B6F150346B29,
        0x4F114F1A0A90D60D,
        0xE104161D4818DC8E,
        0xE0CC5CF6C6925B2D,
        0x85EACC7879747234,
        0x9C0122D8467CEDA,
    ];
    let res = square_fp_bls12_381(&a);
    let res_exp = [
        0xDCD35DE1854944B1,
        0x9783A8B51E1F0FF9,
        0xA9C1DEC644A425AB,
        0x61DE249C203E2EF8,
        0x7014A5D14ACFB859,
        0x1243D7248472E006,
    ];
    assert_eq!(res, res_exp);

    // Inversion
    let a = [0; 6];
    let res = inv_fp_bls12_381(&a);
    let res_exp = a;
    assert_eq!(res, res_exp);

    let a = [
        0xF918184E13DCC9C5,
        0xCB5711ADD8ABB9EA,
        0xE5B13BD70BE8C0D4,
        0xC9174B30720CF52A,
        0x3515A681B1D93EEE,
        0xB49D856D0652100,
    ];
    let res = inv_fp_bls12_381(&a);
    let res_exp = [
        0x2E01077169F50AA2,
        0x6937B3AF9615B7BE,
        0xAEC58E3A24DBF910,
        0x2E323E6AC1F8B1D8,
        0x87F430E4387E34DD,
        0xBBE44AC961A7606,
    ];
    assert_eq!(res, res_exp);
}
