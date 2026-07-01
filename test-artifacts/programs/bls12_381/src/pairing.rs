use ziskos::zisklib::{
    exp_fp12_bls12_381, pairing_bls12_381, scalar_mul_bls12_381, scalar_mul_twist_bls12_381,
};

use crate::constants::{G1, G2, IDENTITY_G1, IDENTITY_G2};

pub fn pairing_valid_tests() {
    let mut one = [0; 72];
    one[0] = 1;

    // Degenerate tests: e(0,0) = e(P,0) = e(0,Q) = 1
    let p = IDENTITY_G1;
    let q = IDENTITY_G2;
    let res = pairing_bls12_381(&p, &q);
    assert_eq!(res, one);

    let p = G1;
    let q = IDENTITY_G2;
    let res = pairing_bls12_381(&p, &q);
    assert_eq!(res, one);

    let p = IDENTITY_G1;
    let q = G2;
    let res = pairing_bls12_381(&p, &q);
    assert_eq!(res, one);

    // Bilinearity test
    // 2P, 12P, 2Q, 12Q
    let p_two = scalar_mul_bls12_381(&G1, &[2, 0, 0, 0]);
    let p_twelve = scalar_mul_bls12_381(&G1, &[12, 0, 0, 0]);
    let q_two = scalar_mul_twist_bls12_381(&G2, &[2, 0, 0, 0]);
    let q_twelve = scalar_mul_twist_bls12_381(&G2, &[12, 0, 0, 0]);

    // e(2P,12Q)
    let p = p_two;
    let q = q_twelve;
    let e1 = pairing_bls12_381(&p, &q);

    // e(P,12Q)²
    let p = G1;
    let q = q_twelve;
    let e2 = pairing_bls12_381(&p, &q);

    // e(2P,Q)¹²
    let p = p_two;
    let q = G2;
    let e3 = pairing_bls12_381(&p, &q);

    // e(P,Q)²⁴
    let p = G1;
    let q = G2;
    let e4 = pairing_bls12_381(&p, &q);

    // e(12P,2Q)
    let p = p_twelve;
    let q = q_two;
    let e5 = pairing_bls12_381(&p, &q);

    // e(2P,12Q) = e(P,12Q)² = e(2P,Q)¹² = e(P,Q)²⁴ = e(12P,2Q)
    assert_eq!(e1, exp_fp12_bls12_381(2, &e2));
    assert_eq!(e1, exp_fp12_bls12_381(12, &e3));
    assert_eq!(e1, exp_fp12_bls12_381(24, &e4));
    assert_eq!(e1, e5);
}

// pub fn pairing_invalid_tests() {
//     // P not in range
//     let p = [P[0], P[1], P[2], P[3], P[4], P[5], 0, 0, 0, 0, 0, 0];
//     let q = IDENTITY_G2;
//     let (_, error_code) = pairing_bls12_381(&p, &q);
//     assert_eq!(error_code, 1);

//     let p = [0, 0, 0, 0, 0, 0, P[0], P[1], P[2], P[3], P[4], P[5]];
//     let q = IDENTITY_G2;
//     let (_, error_code) = pairing_bls12_381(&p, &q);
//     assert_eq!(error_code, 2);

//     // Q not in range
//     let p = IDENTITY_G1;
//     let q =
//         [P[0], P[1], P[2], P[3], P[4], P[5], 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
//     let (_, error_code) = pairing_bls12_381(&p, &q);
//     assert_eq!(error_code, 3);

//     let p = IDENTITY_G1;
//     let q =
//         [0, 0, 0, 0, 0, 0, P[0], P[1], P[2], P[3], P[4], P[5], 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
//     let (_, error_code) = pairing_bls12_381(&p, &q);
//     assert_eq!(error_code, 4);

//     let p = IDENTITY_G1;
//     let q =
//         [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, P[0], P[1], P[2], P[3], P[4], P[5], 0, 0, 0, 0, 0, 0];
//     let (_, error_code) = pairing_bls12_381(&p, &q);
//     assert_eq!(error_code, 5);

//     let p = IDENTITY_G1;
//     let q =
//         [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, P[0], P[1], P[2], P[3], P[4], P[5]];
//     let (_, error_code) = pairing_bls12_381(&p, &q);
//     assert_eq!(error_code, 6);

//     // P not in E
//     let p = [1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0];
//     let q = IDENTITY_G2;
//     let (_, error_code) = pairing_bls12_381(&p, &q);
//     assert_eq!(error_code, 7);

//     // P not in G1 but in E
//     let p = [
//         12449451058704328707,
//         8161700913679311761,
//         1816652880343017915,
//         8253839500538953423,
//         8462536224480843085,
//         256213116093571628,
//         15517235739590233588,
//         16514315301706036196,
//         16264892575127720520,
//         2949797484004982713,
//         17348781962738390123,
//         859117203576425396,
//     ];
//     let q = IDENTITY_G2;
//     let (_, error_code) = pairing_bls12_381(&p, &q);
//     assert_eq!(error_code, 8);

//     // Q not in E'
//     let p = IDENTITY_G1;
//     let q = [1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0];
//     let (_, error_code) = pairing_bls12_381(&p, &q);
//     assert_eq!(error_code, 9);

//     // Q not in G2
//     let p = IDENTITY_G1;
//     let q = [
//         2068375159528909513,
//         2594615493905690519,
//         16619858116915266764,
//         18136688891674078724,
//         6677626148617948320,
//         273146053115976580,
//         795269139261085354,
//         14247204170329808436,
//         9583579405675251987,
//         17705226805394537645,
//         3554842714094901011,
//         1849578438592794556,
//         5720587250323258479,
//         1554226903006696113,
//         5808766992236362198,
//         15248276575265979326,
//         3524134702068163927,
//         1457436382215323138,
//         6853678559606647523,
//         4584480908755827212,
//         5261245535210933263,
//         7274037165591842448,
//         1286267742145149970,
//         1544690384496288947,
//     ];
//     let (_, error_code) = pairing_bls12_381(&p, &q);
//     assert_eq!(error_code, 10);
// }
