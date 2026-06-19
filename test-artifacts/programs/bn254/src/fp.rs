use ziskos::zisklib::{add_fp_bn254, inv_fp_bn254, mul_fp_bn254, neg_fp_bn254, square_fp_bn254};

pub fn fp_tests() {
    // Addition
    let a = [0x1964DD5556D03AD5, 0x02EFF78E60548D50, 0x2F3DEF28C53CF773, 0x284B1398BBF95716];
    let b = [0x64C4D9D238CF0BDA, 0xCD49320A6CB17F4D, 0xF8E0ECA71FC477E3, 0x0B058DC67FDA5B16];
    let res = add_fp_bn254(&a, &b);
    let res_exp = [0x42092B10B7224968, 0x38B7BF0764944210, 0x6FCE9619638016F9, 0x02EC52EC5AA21203];
    assert_eq!(res, res_exp);

    // Negation
    let a = [0; 4];
    let res = neg_fp_bn254(&a);
    let res_exp = a;
    assert_eq!(res, res_exp);

    let a = [0x1964DD5556D03AD5, 0x02EFF78E60548D50, 0x2F3DEF28C53CF773, 0x284B1398BBF95716];
    let res = neg_fp_bn254(&a);
    let res_exp = [0x22BBAEC181ACC272, 0x94917303081D3D3D, 0x8912568DBC4460EA, 0x08193ADA25384913];
    assert_eq!(res, res_exp);

    // Multiplication
    let a = [0x1964DD5556D03AD5, 0x02EFF78E60548D50, 0x2F3DEF28C53CF773, 0x284B1398BBF95716];
    let b = [0x64C4D9D238CF0BDA, 0xCD49320A6CB17F4D, 0xF8E0ECA71FC477E3, 0x0B058DC67FDA5B16];
    let res = mul_fp_bn254(&a, &b);
    let res_exp = [0xC00947FE313A2480, 0xBE80FDFAE6E08BB9, 0x175ECDAB4308BAA5, 0x1A96DF0446C04798];
    assert_eq!(res, res_exp);

    // Squaring
    let a = [0x1964DD5556D03AD5, 0x02EFF78E60548D50, 0x2F3DEF28C53CF773, 0x284B1398BBF95716];
    let res = square_fp_bn254(&a);
    let res_exp = [0x0EC76FD26434AC4D, 0xC784D561F76AA585, 0x2AA8ADE1397E6C28, 0x251611454585F58C];
    assert_eq!(res, res_exp);

    // Inversion
    let a = [0; 4];
    let res = inv_fp_bn254(&a);
    let res_exp = a;
    assert_eq!(res, res_exp);

    let a = [0x1964DD5556D03AD5, 0x02EFF78E60548D50, 0x2F3DEF28C53CF773, 0x284B1398BBF95716];
    let res = inv_fp_bn254(&a);
    let res_exp = [0x2D4D5DD1EE777B62, 0x5173FF6B2FDAAC4F, 0xE6E26CD89F1C4C93, 0x0A834F673D3675BE];
    assert_eq!(res, res_exp);
}
