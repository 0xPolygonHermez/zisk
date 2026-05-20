use std::assert_eq;

use ziskos::zisklib::{add_agtb, add_short, div_long, div_short, mul_long, mul_short, U256};

use super::constants::*;

pub fn array_arith_tests() {
    // Add short
    // ============================
    // inA = inB and len(inA) = len(inB) = 1
    let a = [U256::MAX];
    let b = U256::MAX;
    let mut res = vec![U256::ZERO; 2];
    let len_res = add_short(&a, &b, &mut res);
    assert_eq!(len_res, 2);
    assert_eq!(res, [U256_MAX_MINUS_ONE, U256::ONE]);

    // inA < inB and len(inA) = len(inB) = 1
    let a = [U256_MAX_MINUS_ONE];
    let b = U256::MAX;
    let mut res = vec![U256::ZERO; 2];
    let len_res = add_short(&a, &b, &mut res);
    assert_eq!(len_res, 2);
    assert_eq!(res, [U256_MAX_MINUS_TWO, U256::ONE]);

    // [1, 0, 1] + [MAX]
    let a = [U256::ONE, U256::ZERO, U256::ONE];
    let b = U256::MAX;
    let mut res = vec![U256::ZERO; 3];
    let len_res = add_short(&a, &b, &mut res);
    assert_eq!(len_res, 3);
    assert_eq!(res, [U256::ZERO, U256::ONE, U256::ONE]);

    // [MAX, MAX, MAX] + [MAX]
    let a = [U256::MAX, U256::MAX, U256::MAX];
    let b = U256::MAX;
    let mut res = vec![U256::ZERO; 4];
    let len_res = add_short(&a, &b, &mut res);
    assert_eq!(len_res, 4);
    assert_eq!(res, [U256_MAX_MINUS_ONE, U256::ZERO, U256::ZERO, U256::ONE]);

    // [MAX, MAX, MAX, MAX, MAX] + [1]
    let a = [U256::MAX, U256::MAX, U256::MAX, U256::MAX, U256::MAX];
    let b = U256::ONE;
    let mut res = vec![U256::ZERO; 6];
    let len_res = add_short(&a, &b, &mut res);
    assert_eq!(len_res, 6);
    assert_eq!(res, [U256::ZERO, U256::ZERO, U256::ZERO, U256::ZERO, U256::ZERO, U256::ONE]);

    // random
    let a = [
        U256::from_u64s(&[
            15262553996936329612,
            15924648878889788388,
            8492970576554176395,
            3597563832107918239,
        ]),
        U256::from_u64s(&[
            3342927987972609282,
            10396703647066598664,
            8519928914533603853,
            3839014248966290641,
        ]),
    ];
    let b = U256::from_u64s(&[
        6761209321478583405,
        17504581311336448750,
        5122917048051454884,
        5590305811533986117,
    ]);
    let mut res = vec![U256::ZERO; 2];
    let len_res = add_short(&a, &b, &mut res);
    let expected = [
        U256::from_u64s(&[
            3577019244705361401,
            14982486116516685523,
            13615887624605631280,
            9187869643641904356,
        ]),
        U256::from_u64s(&[
            3342927987972609282,
            10396703647066598664,
            8519928914533603853,
            3839014248966290641,
        ]),
    ];
    assert_eq!(len_res, 2);
    assert_eq!(res, expected);
    // ============================

    // Add long
    // ============================
    // inA = inB and len(inA) = len(inB) = 1
    let a = [U256::MAX];
    let b = [U256::MAX];
    let mut res = vec![U256::ZERO; 2];
    let len_res = add_agtb(&a, &b, &mut res);
    assert_eq!(len_res, 2);
    assert_eq!(res, [U256_MAX_MINUS_ONE, U256::ONE]);

    // inA < inB and len(inA) = len(inB) = 1
    let a = [U256_MAX_MINUS_ONE];
    let b = [U256::MAX];
    let mut res = vec![U256::ZERO; 2];
    let len_res = add_agtb(&a, &b, &mut res);
    assert_eq!(len_res, 2);
    assert_eq!(res, [U256_MAX_MINUS_TWO, U256::ONE]);

    // [MAX, MAX, MAX] + [MAX, MAX]
    let a = [U256::MAX, U256::MAX, U256::MAX];
    let b = [U256::MAX, U256::MAX];
    let mut res = vec![U256::ZERO; 4];
    let len_res = add_agtb(&a, &b, &mut res);
    assert_eq!(len_res, 4);
    assert_eq!(res, [U256_MAX_MINUS_ONE, U256::MAX, U256::ZERO, U256::ONE]);

    // [MAX, MAX, MAX, MAX] + [0, 1]
    let a = [U256::MAX, U256::MAX, U256::MAX, U256::MAX];
    let b = [U256::ZERO, U256::ONE];
    let mut res = vec![U256::ZERO; 5];
    let len_res = add_agtb(&a, &b, &mut res);
    assert_eq!(len_res, 5);
    assert_eq!(res, [U256::MAX, U256::ZERO, U256::ZERO, U256::ZERO, U256::ONE]);

    // [MAX, MAX, MAX-1, MAX-1] + [1]
    let a = [U256::MAX, U256::MAX, U256_MAX_MINUS_ONE, U256_MAX_MINUS_ONE];
    let b = [U256::ONE];
    let mut res = vec![U256::ZERO; 5];
    let len_res = add_agtb(&a, &b, &mut res);
    assert_eq!(len_res, 4);
    assert_eq!(&res[..len_res], [U256::ZERO, U256::ZERO, U256::MAX, U256_MAX_MINUS_ONE]);
    // ============================

    // Mul short
    // ============================
    // [1] * 0
    let a = [U256::ONE];
    let b = U256::ZERO;
    let mut res = vec![U256::ZERO; 2];
    let len_res = mul_short(&a, &b, &mut res);
    assert_eq!(len_res, 1);
    assert_eq!(&res[..len_res], [U256::ZERO]);

    // [MAX] * MAX
    let a = [U256::MAX];
    let b = U256::MAX;
    let mut res = vec![U256::ZERO; 2];
    let len_res = mul_short(&a, &b, &mut res);
    assert_eq!(len_res, 2);
    assert_eq!(&res[..len_res], [U256::ONE, U256_MAX_MINUS_ONE]);

    // [MAX-1] * MAX
    let a = [U256_MAX_MINUS_ONE];
    let b = U256::MAX;
    let mut res = vec![U256::ZERO; 2];
    let len_res = mul_short(&a, &b, &mut res);
    assert_eq!(len_res, 2);
    assert_eq!(&res[..len_res], [U256::TWO, U256_MAX_MINUS_TWO]);

    // [MAX, MAX, MAX] * MAX
    let a = [U256::MAX, U256::MAX, U256::MAX];
    let b = U256::MAX;
    let mut res = vec![U256::ZERO; 4];
    let len_res = mul_short(&a, &b, &mut res);
    assert_eq!(len_res, 4);
    assert_eq!(&res[..len_res], [U256::ONE, U256::MAX, U256::MAX, U256_MAX_MINUS_ONE]);

    // [MAX, 100, MAX, 6] * 400
    let a = [U256::MAX, U256::from_u64(100), U256::MAX, U256::from_u64(6)];
    let b = U256::from_u64(400);
    let mut res = vec![U256::ZERO; 4];
    let len_res = mul_short(&a, &b, &mut res);
    assert_eq!(len_res, 4);
    assert_eq!(
        &res[..len_res],
        [
            U256::from_u64s(&[
                18446744073709551216,
                18446744073709551615,
                18446744073709551615,
                18446744073709551615,
            ]),
            U256::from_u64(40399),
            U256::from_u64s(&[
                18446744073709551216,
                18446744073709551615,
                18446744073709551615,
                18446744073709551615,
            ]),
            U256::from_u64(2799),
        ]
    );
    // ============================

    // Mul long
    // ============================
    // len(inA) = len(inB) and inA = inB
    let a = [U256::from_u64(5), U256::from_u64(6)];
    let b = [U256::from_u64(5), U256::from_u64(6)];
    let mut res = vec![U256::ZERO; 4];
    let len_res = mul_long(&a, &b, &mut res);
    assert_eq!(len_res, 3);
    assert_eq!(&res[..len_res], [U256::from_u64(25), U256::from_u64(60), U256::from_u64(36)]);

    // len(inA) = len(inB) and inA < inB
    let a = [U256::from_u64(5), U256::from_u64(5)];
    let b = [U256::from_u64(5), U256::from_u64(6)];
    let mut res = vec![U256::ZERO; 4];
    let len_res = mul_long(&a, &b, &mut res);
    assert_eq!(len_res, 3);
    assert_eq!(&res[..len_res], [U256::from_u64(25), U256::from_u64(55), U256::from_u64(30)]);

    // len(inA) = len(inB) and inA > inB
    let a = [U256::from_u64(5), U256::from_u64(6), U256::from_u64(7)];
    let b = [U256::from_u64(2), U256::from_u64(3), U256::from_u64(4)];
    let mut res = vec![U256::ZERO; 6];
    let len_res = mul_long(&a, &b, &mut res);
    assert_eq!(len_res, 5);
    assert_eq!(
        &res[..len_res],
        [
            U256::from_u64(10),
            U256::from_u64(27),
            U256::from_u64(52),
            U256::from_u64(45),
            U256::from_u64(28)
        ]
    );

    // len(inA) < len(inB)
    let a = [U256::from_u64(5), U256::from_u64(6)];
    let b = [U256::from_u64(11), U256::from_u64(21), U256::from_u64(16)];
    let mut res = vec![U256::ZERO; 5];
    let len_res = mul_long(&a, &b, &mut res);
    assert_eq!(len_res, 4);
    assert_eq!(
        &res[..len_res],
        [U256::from_u64(55), U256::from_u64(171), U256::from_u64(206), U256::from_u64(96)]
    );

    // [MAX, MAX, MAX] * [MAX, MAX]
    let a = [U256::MAX, U256::MAX, U256::MAX];
    let b = [U256::MAX, U256::MAX];
    let mut res = vec![U256::ZERO; 5];
    let len_res = mul_long(&a, &b, &mut res);
    assert_eq!(len_res, 5);
    assert_eq!(&res[..len_res], [U256::ONE, U256::ZERO, U256::MAX, U256_MAX_MINUS_ONE, U256::MAX]);
    // ============================

    // Div short
    // ============================
    // inA == 0, inB != 0
    let a = [U256::ZERO];
    let b = U256::from_u64(8);
    let (q, r) = div_short(&a, &b);
    assert_eq!(q.len(), 1);
    assert_eq!(q, [U256::ZERO]);
    assert_eq!(r, U256::ZERO);

    // inA < inB
    let a = [U256::from_u64(10)];
    let b = U256::from_u64(11);
    let (q, r) = div_short(&a, &b);
    assert_eq!(q.len(), 1);
    assert_eq!(q, [U256::ZERO]);
    assert_eq!(r, a[0]);

    // inA == inB
    let a = [U256::from_u64(10)];
    let b = U256::from_u64(10);
    let (q, r) = div_short(&a, &b);
    assert_eq!(q.len(), 1);
    assert_eq!(q, [U256::ONE]);
    assert_eq!(r, U256::ZERO);

    // inA = k·inB
    let a = [U256::from_u64(8)];
    let b = U256::from_u64(2);
    let (q, r) = div_short(&a, &b);
    assert_eq!(q.len(), 1);
    assert_eq!(q, [U256::from_u64(4)]);
    assert_eq!(r, U256::ZERO);

    // [9n, 8n, 7n, 6n] / 8n
    let a = [U256::from_u64(9), U256::from_u64(8), U256::from_u64(7), U256::from_u64(6)];
    let b = U256::from_u64(8);
    let (q, r) = div_short(&a, &b);
    assert_eq!(q.len(), 3);
    assert_eq!(
        q,
        [
            U256::ONE,
            U256::from_u64s(&[1, 0, 0, 16140901064495857664]),
            U256::from_u64s(&[0, 0, 0, 13835058055282163712])
        ]
    );
    assert_eq!(r, U256::ONE);

    // [MAX, 7n, MAX, 12n, MAX, 20n, MAX, 80n] / MAX
    let a = [
        U256::MAX,
        U256::from_u64(7),
        U256::MAX,
        U256::from_u64(12),
        U256::MAX,
        U256::from_u64(20),
        U256::MAX,
        U256::from_u64(80),
    ];
    let b = U256::MAX;
    let (q, r) = div_short(&a, &b);
    assert_eq!(q.len(), 7);
    assert_eq!(
        q,
        [
            U256::from_u64(120),
            U256::from_u64(112),
            U256::from_u64(113),
            U256::from_u64(100),
            U256::from_u64(101),
            U256::from_u64(80),
            U256::from_u64(81),
        ]
    );
    assert_eq!(r, U256::from_u64(119));

    // [U256_MAX_QUARTER, 1n] / 2
    let a = [U256_MAX_QUARTER, U256::ONE];
    let b = U256::from_u64(2);
    let (q, r) = div_short(&a, &b);
    assert_eq!(q.len(), 1);
    assert_eq!(q, [U256::from_u64s(&[0, 0, 0, 11529215046068469760])]);
    assert_eq!(r, U256::ZERO);

    // [prev_res] / 2
    let a = [U256::from_u64s(&[0, 0, 0, 11529215046068469760])];
    let b = U256::from_u64(2);
    let (q, r) = div_short(&a, &b);
    assert_eq!(q.len(), 1);
    assert_eq!(q, [U256::from_u64s(&[0, 0, 0, 5764607523034234880])]);
    assert_eq!(r, U256::ZERO);

    // [MAX-1]*10 / [2]
    let a = vec![U256_MAX_MINUS_ONE; 10];
    let b = U256::from_u64(2);
    let (q, r) = div_short(&a, &b);
    assert_eq!(q.len(), 10);
    assert_eq!(q, vec![U256_MAX_HALF_MINUS_ONE; 10]);
    assert_eq!(r, U256::ZERO);

    // [1]*10 / [2]
    let a = vec![U256::ONE; 10];
    let b = U256::from_u64(2);
    let (q, r) = div_short(&a, &b);
    assert_eq!(q.len(), 9);
    assert_eq!(q, vec![U256_MAX_HALF; 9]);
    assert_eq!(r, U256::ONE);

    // MAX || [MAX-1]*9 / [2]
    let mut a = vec![U256_MAX_MINUS_ONE; 10];
    a[0] = U256::MAX;
    let b = U256::from_u64(2);
    let (q, r) = div_short(&a, &b);
    assert_eq!(q.len(), 10);
    assert_eq!(q, vec![U256_MAX_HALF_MINUS_ONE; 10]);
    assert_eq!(r, U256::ONE);
    // ============================

    // Div long
    // ============================
    // inA == 0, inB != 0, len(inB) > 1
    let a = [U256::ZERO];
    let b = [U256::from_u64(8), U256::ONE];
    let (q, r) = div_long(&a, &b);
    assert_eq!(q.len(), 1);
    assert_eq!(q, [U256::ZERO]);
    assert_eq!(r.len(), 1);
    assert_eq!(r, [U256::ZERO]);

    // inA < inB, 1 == len(inA) < len(inB)
    let a = [U256::from_u64(7)];
    let b = [
        U256::from_u64s(&[
            1229782938247303441,
            1229782938247303441,
            1229782938247303441,
            1229782938247303441,
        ]),
        U256::from_u64(17),
    ];
    let (q, r) = div_long(&a, &b);
    assert_eq!(q.len(), 1);
    assert_eq!(q, [U256::ZERO]);
    assert_eq!(r.len(), 1);
    assert_eq!(r, a);

    // inA < inB, 1 < len(inA) < len(inB)
    let a = [U256::from_u64(10), U256::from_u64(30)];
    let b = [U256::from_u64(6), U256::from_u64(7), U256::from_u64(8)];
    let (q, r) = div_long(&a, &b);
    assert_eq!(q.len(), 1);
    assert_eq!(q, [U256::ZERO]);
    assert_eq!(r.len(), 2);
    assert_eq!(r, a);

    // inA == inB, len(inA),len(inB) > 1
    let a = [U256::from_u64(10), U256::from_u64(30)];
    let b = [U256::from_u64(10), U256::from_u64(30)];
    let (q, r) = div_long(&a, &b);
    assert_eq!(q.len(), 1);
    assert_eq!(q, [U256::ONE]);
    assert_eq!(r.len(), 1);
    assert_eq!(r, [U256::ZERO]);

    // inA == k·inB, len(inA),len(inB) > 1
    let a = [U256::from_u64(8), U256::from_u64(8)];
    let b = [U256::from_u64(2), U256::from_u64(2)];
    let (q, r) = div_long(&a, &b);
    assert_eq!(q.len(), 1);
    assert_eq!(q, [U256::from_u64(4)]);
    assert_eq!(r.len(), 1);
    assert_eq!(r, [U256::ZERO]);

    // inA > inB, len(inA) > len(inB) > 1
    let a = [U256::from_u64(9), U256::from_u64(8), U256::from_u64(7), U256::from_u64(6)];
    let b = [U256::from_u64(8), U256::from_u64(1)];
    let (q, r) = div_long(&a, &b);
    assert_eq!(q.len(), 3);
    assert_eq!(
        q,
        [
            U256::from_u64(335),
            U256::from_u64s(&[
                18446744073709551575,
                18446744073709551615,
                18446744073709551615,
                18446744073709551615,
            ]),
            U256::from_u64(5)
        ]
    );
    assert_eq!(r.len(), 1);
    assert_eq!(
        r,
        [U256::from_u64s(&[
            18446744073709548945,
            18446744073709551615,
            18446744073709551615,
            18446744073709551615,
        ])]
    );

    // [MAX, 7n, MAX, 12n, MAX, 20n, MAX, 80n] / [MAX, MAX, MAX, MAX, 100n]
    let a = [
        U256::MAX,
        U256::from_u64(7),
        U256::MAX,
        U256::from_u64(12),
        U256::MAX,
        U256::from_u64(20),
        U256::MAX,
        U256::from_u64(80),
    ];
    let b = [U256::MAX, U256::MAX, U256::MAX, U256::MAX, U256::from_u64(100)];
    let (q, r) = div_long(&a, &b);
    assert_eq!(q.len(), 3);
    assert_eq!(
        q,
        [
            U256::from_u64s(&[
                11871666978129909455,
                3652820608655356755,
                8218846369474552700,
                13880718312890355671
            ]),
            U256::from_u64s(&[
                8766769460772856213,
                1278487213029374864,
                7488282247743481349,
                12236949038995445131
            ]),
            U256::from_u64s(&[
                9497333582503927564,
                2922256486924285404,
                6575077095579642160,
                14793923465054194860,
            ]),
        ]
    );
    assert_eq!(r.len(), 5);
    assert_eq!(
        r,
        [
            U256::from_u64s(&[
                11871666978129909454,
                3652820608655356755,
                8218846369474552700,
                13880718312890355671,
            ]),
            U256::from_u64s(&[
                8766769460772856221,
                1278487213029374864,
                7488282247743481349,
                12236949038995445131,
            ]),
            U256::from_u64s(&[
                9497333582503927563,
                2922256486924285404,
                6575077095579642160,
                14793923465054194860,
            ]),
            U256::from_u64(13),
            U256::from_u64(84)
        ]
    );

    // [82987931714326364316120253427931880709278140571418487333162713377057429160720n,4257238595720679277571917967782652353394431698489248379634099239588181418140n,15209178211456919413336795740141505754388379695813905932093982440742677791802n,88987534839350135473536361176867192550264928852523682165693061442019881855583n,14n] / [4n, 6n, 7n]
    let a = [
        U256::from_u64s(&[
            305311352864539408,
            5121716100336988427,
            13890375398671508018,
            13220740273570563432,
        ]),
        U256::from_u64s(&[
            10128952842933608604,
            16153313847886807076,
            15464981750310544624,
            678217237060349431,
        ]),
        U256::from_u64s(&[
            14322988078933886010,
            15646911644215079500,
            15147024421881662240,
            2422961878364395612,
        ]),
        U256::from_u64s(&[
            4563939620935117407,
            3421005181133378521,
            11628240122499389652,
            14176532194418598693,
        ]),
        U256::from_u64(14),
    ];
    let b = [U256::from_u64(4), U256::from_u64(6), U256::from_u64(7)];
    let (q, r) = div_long(&a, &b);
    assert_eq!(q.len(), 3);
    assert_eq!(
        q,
        [
            U256::from_u64s(&[
                15792929665875295082,
                5581016146118383704,
                15045632755492617534,
                14421730430160007040,
            ]),
            U256::from_u64s(&[
                8557738834580538891,
                8394462486037433338,
                9566924620518292071,
                2025218884916942670,
            ]),
            U256::from_u64(2),
        ]
    );
    assert_eq!(r.len(), 3);
    assert_eq!(
        r,
        [
            U256::from_u64s(&[
                10920568984201565544,
                1244395589573005223,
                9048076597829692729,
                10874050774059190117,
            ]),
            U256::from_u64s(&[
                10267628025326543857,
                4429599248155426341,
                16050695251248532445,
                16727423558689846200,
            ]),
            U256::from_u64(4),
        ]
    );

    // [0n,0n,0n,0n,87552057494100699607633960453116574392480272162273084008350826812719088235449n,29405388739667337424543497575767709934732594998639086405406332616399343873602n,370491411790392985199n], [0n, 0n, 8238129386n, 23102318237n]
    let a = [
        U256::ZERO,
        U256::ZERO,
        U256::ZERO,
        U256::ZERO,
        U256::from_u64s(&[
            17085908631624315833,
            9999507225620644265,
            14793599250044772163,
            13947847459685522413,
        ]),
        U256::from_u64s(&[
            5459001374359711298,
            17851732715271037845,
            13717288955952459357,
            4684548694486933077,
        ]),
        U256::from_u64s(&[1556530316201952879, 20, 0, 0]),
    ];
    let b = [U256::ZERO, U256::ZERO, U256::from_u64(8238129386), U256::from_u64(23102318237)];
    let (q, r) = div_long(&a, &b);
    assert_eq!(q.len(), 4);
    assert_eq!(
        q,
        [
            U256::from_u64s(&[
                15421798497135261849,
                11065096246185916255,
                6807084294288706043,
                1692642783571174715,
            ]),
            U256::from_u64s(&[
                14513279990233740390,
                11902279128586214763,
                8717992811341171740,
                2023105637456124205,
            ]),
            U256::from_u64s(&[
                11381495444223541657,
                10000920975836796190,
                7365357682358776824,
                10032328693147815539,
            ]),
            U256::from_u64(16036979838),
        ]
    );
    assert_eq!(r.len(), 4);
    assert_eq!(
        r,
        [
            U256::ZERO,
            U256::ZERO,
            U256::from_u64s(&[
                16564039407399658534,
                272324349106987740,
                8204470348635441003,
                6979613961251619239,
            ]),
            U256::from_u64(6019321230),
        ]
    );

    // [9,12,16,2,0,MAX-3] / [MAX,MAX]
    let a = [
        U256::from_u64(9),
        U256::from_u64(12),
        U256::from_u64(16),
        U256::from_u64(2),
        U256::ZERO,
        U256_MAX_MINUS_THREE,
    ];
    let b = [U256::MAX, U256::MAX];
    let (q, r) = div_long(&a, &b);
    assert_eq!(q.len(), 4);
    assert_eq!(q, [U256::from_u64(17), U256_MAX_MINUS_ONE, U256::ZERO, U256_MAX_MINUS_THREE,]);
    assert_eq!(r.len(), 2);
    assert_eq!(r, [U256::from_u64(26), U256::from_u64(10),]);

    // [MAX]*10 / [1,1]
    let a = vec![U256::MAX; 10];
    let b = [U256::ONE, U256::ONE];
    let (q, r) = div_long(&a, &b);
    assert_eq!(q.len(), 9);
    let mut q_expected = vec![U256::ZERO; 9];
    for i in (0..9).rev() {
        if i % 2 == 0 {
            q_expected[i] = U256::MAX;
        }
    }
    assert_eq!(q, q_expected);
    assert_eq!(r.len(), 1);
    assert_eq!(r, [U256::ZERO]);

    // [MAX]*10 / [1]*5
    let a = vec![U256::MAX; 10];
    let b = vec![U256::ONE; 5];
    let (q, r) = div_long(&a, &b);
    assert_eq!(q.len(), 6);
    let mut q_expected = vec![U256::ZERO; 6];
    for i in (0..6).rev() {
        if i == 5 || i == 0 {
            q_expected[i] = U256::MAX;
        }
    }
    assert_eq!(q, q_expected);
    assert_eq!(r.len(), 1);
    assert_eq!(r, [U256::ZERO]);

    // [MAX] || [MAX-1]*9 / [1]*5
    let mut a = vec![U256_MAX_MINUS_ONE; 10];
    a[0] = U256::MAX;
    let b = vec![U256::ONE; 5];
    let (q, r) = div_long(&a, &b);
    assert_eq!(q.len(), 6);
    let mut q_expected = vec![U256::ZERO; 6];
    for i in (0..6).rev() {
        if i == 5 || i == 0 {
            q_expected[i] = U256_MAX_MINUS_ONE;
        }
    }
    assert_eq!(q, q_expected);
    assert_eq!(r.len(), 1);
    assert_eq!(r, [U256::ONE]);

    // [MAX, MAX] || [MAX-1]*8 / [2]*5
    let mut a = vec![U256_MAX_MINUS_ONE; 10];
    a[0] = U256::MAX;
    a[1] = U256::MAX;
    let b = vec![U256::from_u64(2); 5];
    let (q, r) = div_long(&a, &b);
    assert_eq!(q.len(), 6);
    let mut q_expected = vec![U256::ZERO; 6];
    for i in (0..6).rev() {
        if i == 5 || i == 0 {
            q_expected[i] = U256_MAX_HALF_MINUS_ONE;
        }
    }
    assert_eq!(q, q_expected);
    assert_eq!(r.len(), 2);
    assert_eq!(r, [U256::ONE, U256::ONE]);
    // ============================

    println!("Array Arith tests passed");
}
