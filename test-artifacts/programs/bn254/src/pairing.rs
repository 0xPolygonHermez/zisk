use ziskos::zisklib::{exp_fp12_bn254, neg_bn254, pairing_bn254, pairing_check_bn254};

use crate::constants::{IDENTITY_G1, IDENTITY_G2, P};

pub fn pairing_check_tests() {
    // Test 1: Empty pairing (should return true)
    let result = pairing_check_bn254(&[], &[]).expect("Empty pairing should succeed");
    assert!(result, "Empty pairing should return true");

    // Test 2: Identity points (e(0,Q) = 1, should return true after filtering)
    let p = IDENTITY_G1;
    let q = [
        0x9990C4F783E78FC5,
        0x47B71082B0D94CED,
        0xDFF850E44C211262,
        0x099ECE5FA385A639,
        0x9FA008B0854738C5,
        0xADA5D1130460685D,
        0x9546DBB3D53487DF,
        0x0486687755AD3E80,
        0x311BB2C2C86690CF,
        0xCC56CB84BC137759,
        0x86897C3C692D952C,
        0x00B6884E5D02665B,
        0x2702AB556C5EFF3E,
        0x059E40D295EB0F4E,
        0x51704116523CBD21,
        0x1FA69C987C6371FF,
    ];
    let result = pairing_check_bn254(&[p], &[q]).expect("Identity G1 should succeed");
    assert!(result, "e(0,Q) = 1");

    // Test 3: e(P,0) = 1
    let p = [
        0xD3C208C16D87CFD3,
        0xD97816A916871CA8,
        0x9B85045B68181585,
        0x030644E72E131A02,
        0xFF3EBF7A5A18A2C4,
        0x68A6A449E3538FC7,
        0xE7845F96B2AE9C0A,
        0x15ED738C0E0A7C92,
    ];
    let q = IDENTITY_G2;
    let result = pairing_check_bn254(&[p], &[q]).expect("Identity G2 should succeed");
    assert!(result, "e(P,0) = 1");

    // e(0,0) = 1
    let p = IDENTITY_G1;
    let q = IDENTITY_G2;
    let result = pairing_check_bn254(&[p], &[q]).expect("Identity G1 and G2 should succeed");
    assert!(result, "e(0,0) = 1");

    // Test 4: Successful pairing check - e(P,Q) * e(-P,Q) = 1
    let p = [
        0x0000000000000001,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000002,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
    ];
    let neg_p = neg_bn254(&p);
    let q = [
        0x46DEBD5CD992F6ED,
        0x674322D4F75EDADD,
        0x426A00665E5C4479,
        0x1800DEEF121F1E76,
        0x97E485B7AEF312C2,
        0xF1AA493335A9E712,
        0x7260BFB731FB5D25,
        0x198E9393920D483A,
        0x4CE6CC0166FA7DAA,
        0xE3D1E7690C43D37B,
        0x4AAB71808DCB408F,
        0x12C85EA5DB8C6DEB,
        0x55ACDADCD122975B,
        0xBC4B313370B38EF3,
        0xEC9E99AD690C3395,
        0x090689D0585FF075,
    ];
    let result =
        pairing_check_bn254(&[p, neg_p], &[q, q]).expect("Bilinearity test should succeed");
    assert!(result, "e(P,Q) * e(-P,Q) should equal 1");

    // Test 5: Failed pairing check - e(P,Q) * e(P,Q) != 1
    let result = pairing_check_bn254(&[p, p], &[q, q]).expect("Should compute but return false");
    assert!(!result, "e(P,Q) * e(P,Q) should not equal 1");

    // Test 6: G1 point not on curve (0, 1)
    let bad_p = [
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000001,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
    ];
    let result = pairing_check_bn254(&[bad_p], &[q]);
    assert!(result.is_err(), "G1 point (0,1) not on curve should fail");
    assert_eq!(result.unwrap_err(), 3, "Should return G1_NOT_ON_CURVE error");

    // Test 7: G1 field element >= P (x coordinate out of range)
    let bad_p = [
        P[0],
        P[1],
        P[2],
        P[3],
        0x0000000000000001,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
    ];
    let result = pairing_check_bn254(&[bad_p], &[q]);
    assert!(result.is_err(), "G1.x >= P should fail");
    assert_eq!(result.unwrap_err(), 2, "Should return G1_INVALID error");

    // Test 8: G1 field element >= P (y coordinate out of range)
    let bad_p = [
        0x0000000000000001,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        P[0],
        P[1],
        P[2],
        P[3],
    ];
    let result = pairing_check_bn254(&[bad_p], &[q]);
    assert!(result.is_err(), "G1.y >= P should fail");
    assert_eq!(result.unwrap_err(), 2, "Should return G1_INVALID error");

    // Test 9: G2 point not on curve (1, 2, 3, 3)
    let bad_q = [
        0x0000000000000001,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000002,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000003,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000003,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
    ];
    let result = pairing_check_bn254(&[p], &[bad_q]);
    assert!(result.is_err(), "G2 point not on curve should fail");
    assert_eq!(result.unwrap_err(), 5, "Should return G2_NOT_ON_CURVE error");

    // Test 10: G2 field element >= P (x1 coordinate)
    let bad_q = [
        P[0],
        P[1],
        P[2],
        P[3],
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
    ];
    let result = pairing_check_bn254(&[p], &[bad_q]);
    assert!(result.is_err(), "G2.x1 >= P should fail");
    assert_eq!(result.unwrap_err(), 4, "Should return G2_INVALID error");

    // Test 11: G2 field element >= P (x2 coordinate)
    let bad_q = [
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        P[0],
        P[1],
        P[2],
        P[3],
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
    ];
    let result = pairing_check_bn254(&[p], &[bad_q]);
    assert!(result.is_err(), "G2.x2 >= P should fail");
    assert_eq!(result.unwrap_err(), 4, "Should return G2_INVALID error");

    // Test 12: G2 field element >= P (y1 coordinate)
    let bad_q = [
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        P[0],
        P[1],
        P[2],
        P[3],
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
    ];
    let result = pairing_check_bn254(&[p], &[bad_q]);
    assert!(result.is_err(), "G2.y1 >= P should fail");
    assert_eq!(result.unwrap_err(), 4, "Should return G2_INVALID error");

    // Test 13: G2 field element >= P (y2 coordinate)
    let bad_q = [
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        P[0],
        P[1],
        P[2],
        P[3],
    ];
    let result = pairing_check_bn254(&[p], &[bad_q]);
    assert!(result.is_err(), "G2.y2 >= P should fail");
    assert_eq!(result.unwrap_err(), 4, "Should return G2_INVALID error");

    // Test 14: G2 not in subgroup (on curve but not in G2)
    let bad_q = [
        0xE642D1780FA77460,
        0x940C7100CC3B163F,
        0x9E35DB46F7250AFC,
        0x1ED91A62C98A6383,
        0x9E87C23424EE0063,
        0x12859810070565E9,
        0x03CE49ABC798B83F,
        0x181BAE8231C1F263,
        0x6B283DE32DD4D366,
        0xFE4EED8EF8036477,
        0x49D7F9C268537017,
        0x1B2D40EFDB1326CC,
        0xE0A118CFA3E0D8D7,
        0x9A3C9ECF0C83F1D7,
        0x28D70CD532462E40,
        0x07D105D0066EC703,
    ];
    let result = pairing_check_bn254(&[p], &[bad_q]);
    assert!(result.is_err(), "G2 not in subgroup should fail");
    assert_eq!(result.unwrap_err(), 6, "Should return G2_NOT_IN_SUBGROUP error");

    // Test 15: Multiple pairs with mixed valid/identity points
    let p1 = p;
    let p2 = IDENTITY_G1;
    let p3 = neg_p;
    let q1 = q;
    let q2 = q;
    let q3 = q;
    let result = pairing_check_bn254(&[p1, p2, p3], &[q1, q2, q3])
        .expect("Mixed valid/identity should succeed");
    assert!(result, "e(P,Q) * e(0,Q) * e(-P,Q) = 1");
}

pub fn pairing_tests() {
    let mut one = [0; 48];
    one[0] = 1;

    // Degenerate tests: e(0,Q) = e(P,0) = e(0,0) = 1
    let p = IDENTITY_G1;
    let q = [
        0x9990C4F783E78FC5,
        0x47B71082B0D94CED,
        0xDFF850E44C211262,
        0x099ECE5FA385A639,
        0x9FA008B0854738C5,
        0xADA5D1130460685D,
        0x9546DBB3D53487DF,
        0x0486687755AD3E80,
        0x311BB2C2C86690CF,
        0xCC56CB84BC137759,
        0x86897C3C692D952C,
        0x00B6884E5D02665B,
        0x2702AB556C5EFF3E,
        0x059E40D295EB0F4E,
        0x51704116523CBD21,
        0x1FA69C987C6371FF,
    ];
    let res = pairing_bn254(&p, &q);
    assert_eq!(res, one);

    let p = [
        0xD3C208C16D87CFD3,
        0xD97816A916871CA8,
        0x9B85045B68181585,
        0x030644E72E131A02,
        0xFF3EBF7A5A18A2C4,
        0x68A6A449E3538FC7,
        0xE7845F96B2AE9C0A,
        0x15ED738C0E0A7C92,
    ];
    let q = IDENTITY_G2;
    let res = pairing_bn254(&p, &q);
    assert_eq!(res, one);

    let p = IDENTITY_G1;
    let q = IDENTITY_G2;
    let res = pairing_bn254(&p, &q);
    assert_eq!(res, one);

    // Bilinearity test
    let answer: [u64; 48] = [
        0x5D5DD924F5E2CC1A,
        0xD9BE2B2BE1EF692B,
        0x3865D61958739177,
        0x1DA7C92C292F43D1,
        0x8A752BBBB0F9B536,
        0x6741F920F0CD4BB8,
        0xE2F321E12CA506D4,
        0x07D8501D116DF568,
        0xEE9E4D1E8907DCE0,
        0xDA4EB0A7C20004AF,
        0x675A65DAB1718FBC,
        0x18E4229543B82DCF,
        0xF8CEF9933D08FDA2,
        0x2140CA4FDB0BEF37,
        0xA1C502B047ECE884,
        0x22B197B6680F372F,
        0x5A55732D2481148D,
        0xDB451F4EBE3D8C9C,
        0xFD5A3CC19CBF6DCA,
        0x1FAF3E63129CCDF3,
        0xFDE97926FD5FA206,
        0x24285D861684B4D9,
        0xD016AAF0159A44DE,
        0x0CE32134EE65CC90,
        0x7E0E5760E2BE5A88,
        0x4EB14EB4A51E718C,
        0x6E8CEF133A54832A,
        0x2236FBBBC1DBFED2,
        0x6330F092DCAE2552,
        0x76428A91AD58F5CB,
        0xADFEE57BA632ED1A,
        0x0B452BBE55E7C82F,
        0x0A7FCB3B78D6277F,
        0x0D73B3036661B68C,
        0x6D4E555BEB971225,
        0x195181B3E0234B49,
        0xC3B601BF8490DE2E,
        0x2EF6AB5057D511B8,
        0x2F916AB9443D335C,
        0x16956AE54BCB34FC,
        0x262CFF2987FA5F4D,
        0x21F64E0BC8D867CF,
        0x2F9C5E7FE5E8479C,
        0x109CA139E10CFE6C,
        0x677C4F1E301569BC,
        0x9BF029327DCF2651,
        0x6E2BE3343B78A1FD,
        0x035DE8307BAF5E4E,
    ];

    // e(2P,12Q)
    let p = [
        0xD3C208C16D87CFD3,
        0xD97816A916871CA8,
        0x9B85045B68181585,
        0x030644E72E131A02,
        0xFF3EBF7A5A18A2C4,
        0x68A6A449E3538FC7,
        0xE7845F96B2AE9C0A,
        0x15ED738C0E0A7C92,
    ];
    let q = [
        0x9990C4F783E78FC5,
        0x47B71082B0D94CED,
        0xDFF850E44C211262,
        0x099ECE5FA385A639,
        0x9FA008B0854738C5,
        0xADA5D1130460685D,
        0x9546DBB3D53487DF,
        0x0486687755AD3E80,
        0x311BB2C2C86690CF,
        0xCC56CB84BC137759,
        0x86897C3C692D952C,
        0x00B6884E5D02665B,
        0x2702AB556C5EFF3E,
        0x059E40D295EB0F4E,
        0x51704116523CBD21,
        0x1FA69C987C6371FF,
    ];
    let e1 = pairing_bn254(&p, &q);
    assert_eq!(e1, answer);

    // e(P,12Q)²
    let p = [
        0x0000000000000001,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000002,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
    ];
    let q = [
        0x9990C4F783E78FC5,
        0x47B71082B0D94CED,
        0xDFF850E44C211262,
        0x099ECE5FA385A639,
        0x9FA008B0854738C5,
        0xADA5D1130460685D,
        0x9546DBB3D53487DF,
        0x0486687755AD3E80,
        0x311BB2C2C86690CF,
        0xCC56CB84BC137759,
        0x86897C3C692D952C,
        0x00B6884E5D02665B,
        0x2702AB556C5EFF3E,
        0x059E40D295EB0F4E,
        0x51704116523CBD21,
        0x1FA69C987C6371FF,
    ];
    let e2 = pairing_bn254(&p, &q);

    // e(2P,Q)¹²
    let p = [
        0xD3C208C16D87CFD3,
        0xD97816A916871CA8,
        0x9B85045B68181585,
        0x030644E72E131A02,
        0xFF3EBF7A5A18A2C4,
        0x68A6A449E3538FC7,
        0xE7845F96B2AE9C0A,
        0x15ED738C0E0A7C92,
    ];
    let q = [
        0x46DEBD5CD992F6ED,
        0x674322D4F75EDADD,
        0x426A00665E5C4479,
        0x1800DEEF121F1E76,
        0x97E485B7AEF312C2,
        0xF1AA493335A9E712,
        0x7260BFB731FB5D25,
        0x198E9393920D483A,
        0x4CE6CC0166FA7DAA,
        0xE3D1E7690C43D37B,
        0x4AAB71808DCB408F,
        0x12C85EA5DB8C6DEB,
        0x55ACDADCD122975B,
        0xBC4B313370B38EF3,
        0xEC9E99AD690C3395,
        0x090689D0585FF075,
    ];
    let e3 = pairing_bn254(&p, &q);

    // e(P,Q)²⁴
    let p = [
        0x0000000000000001,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000002,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
    ];
    let q = [
        0x46DEBD5CD992F6ED,
        0x674322D4F75EDADD,
        0x426A00665E5C4479,
        0x1800DEEF121F1E76,
        0x97E485B7AEF312C2,
        0xF1AA493335A9E712,
        0x7260BFB731FB5D25,
        0x198E9393920D483A,
        0x4CE6CC0166FA7DAA,
        0xE3D1E7690C43D37B,
        0x4AAB71808DCB408F,
        0x12C85EA5DB8C6DEB,
        0x55ACDADCD122975B,
        0xBC4B313370B38EF3,
        0xEC9E99AD690C3395,
        0x090689D0585FF075,
    ];
    let e4 = pairing_bn254(&p, &q);

    // e(12P,2Q)
    let p = [
        0x9FC0B7D1002BC851,
        0xF75AD9E4F36526B0,
        0x9AC9B4118D040166,
        0x25D32C471C8CD1AB,
        0xAE5BF9095585B69C,
        0x7FF42B63CB1C200B,
        0xDF3404069078F036,
        0x2DB09AE9BC0CB9AD,
    ];
    let q = [
        0x49F8130962B4B3B9,
        0x9D5CD3CFA9A62AEE,
        0xC36C59277C3E6F14,
        0x27DC7234FD11D3E8,
        0x9957ED8C3928AD79,
        0x6DB86431C6D83584,
        0xB60121B83A733370,
        0x203E205DB4F19B37,
        0x6E2A6DAD122B5D2E,
        0x44A59B4FE6B1C046,
        0xA0BC372742C48309,
        0x04BB53B8977E5F92,
        0x98E185F0509DE152,
        0x3505566B4EDF48D4,
        0x722B8C153931579D,
        0x195E8AA5B7827463,
    ];
    let e5 = pairing_bn254(&p, &q);

    // e(2P,12Q) = e(P,12Q)² = e(2P,Q)¹² = e(P,Q)²⁴ = e(12P,2Q)
    assert_eq!(e1, exp_fp12_bn254(2, &e2));
    assert_eq!(e1, exp_fp12_bn254(12, &e3));
    assert_eq!(e1, exp_fp12_bn254(24, &e4));
    assert_eq!(e1, e5);
}
