//! Big-endian twins of the ArithEq / ArithEq384 syscalls (`syscall_*_be`).
//!
//! Each twin must give, on operands stored as big-endian integers, exactly the result its
//! little-endian syscall gives on the same values: so every check here runs both on the same
//! inputs (the big-endian one on the converted operands) and compares, after converting the
//! big-endian result back. The little-endian results themselves are pinned against constants in
//! the sibling diagnostics.

use ziskos::syscalls::*;

/// A 256-bit integer, given as little-endian limbs, laid out as the big-endian integer the `*_be`
/// syscalls take: words reversed, bytes of every word swapped.
fn be4(v: &[u64; 4]) -> [u64; 4] {
    let mut out = *v;
    swap_endianness(&mut out);
    out
}

/// A 384-bit integer, as [`be4`].
fn be6(v: &[u64; 6]) -> [u64; 6] {
    let mut out = *v;
    swap_endianness(&mut out);
    out
}

fn be_point256(p: &SyscallPoint256) -> SyscallPoint256 {
    SyscallPoint256 { x: be4(&p.x), y: be4(&p.y) }
}

fn be_point384(p: &SyscallPoint384) -> SyscallPoint384 {
    SyscallPoint384 { x: be6(&p.x), y: be6(&p.y) }
}

fn be_complex256(f: &SyscallComplex256) -> SyscallComplex256 {
    SyscallComplex256 { x: be4(&f.x), y: be4(&f.y) }
}

fn be_complex384(f: &SyscallComplex384) -> SyscallComplex384 {
    SyscallComplex384 { x: be6(&f.x), y: be6(&f.y) }
}

pub fn diagnostic_big_endian() {
    // The conversion itself: 0x0102..1f20 as a big-endian number occupies the bytes 1..32 in
    // memory order, and converting twice is the identity.
    let bytes: [u8; 32] = core::array::from_fn(|i| i as u8 + 1);
    let memory: [u64; 4] =
        core::array::from_fn(|i| u64::from_le_bytes(bytes[8 * i..8 * i + 8].try_into().unwrap()));
    let limbs = be4(&memory);
    assert_eq!(limbs[0], 0x191A1B1C1D1E1F20);
    assert_eq!(limbs[3], 0x0102030405060708);
    assert_eq!(be4(&limbs), memory);

    //////////////
    // Arith256
    //////////////

    let a = [13970229013151504741, 8476296752562947313, 11810450538887363942, 511990551865481398];
    let b = [11990850244716481796, 14558188671963395327, 9424388055416098482, 1459171711273467932];
    let c = [16528603495754341937, 8893271371239080203, 9406449307822347647, 250213327518958686];
    let (mut dl, mut dh) = ([0u64; 4], [0u64; 4]);
    syscall_arith256(&mut SyscallArith256Params { a: &a, b: &b, c: &c, dl: &mut dl, dh: &mut dh });
    let (a_be, b_be, c_be) = (be4(&a), be4(&b), be4(&c));
    let (mut dl_be, mut dh_be) = ([0u64; 4], [0u64; 4]);
    syscall_arith256_be(&mut SyscallArith256Params {
        a: &a_be,
        b: &b_be,
        c: &c_be,
        dl: &mut dl_be,
        dh: &mut dh_be,
    });
    assert_eq!(be4(&dl_be), dl);
    assert_eq!(be4(&dh_be), dh);
    assert_eq!(dl_be, be4(&dl), "the big-endian result is the big-endian image of the result");

    //////////////
    // Arith256Mod
    //////////////

    let module =
        [4332616871279656262, 10917124144477883021, 13281191951274694749, 3486998266802970665];
    let mut d = [0u64; 4];
    syscall_arith256_mod(&mut SyscallArith256ModParams {
        a: &a,
        b: &b,
        c: &c,
        module: &module,
        d: &mut d,
    });
    let module_be = be4(&module);
    let mut d_be = [0u64; 4];
    syscall_arith256_mod_be(&mut SyscallArith256ModParams {
        a: &a_be,
        b: &b_be,
        c: &c_be,
        module: &module_be,
        d: &mut d_be,
    });
    assert_eq!(be4(&d_be), d);

    //////////////
    // Secp256k1 add / dbl
    //////////////

    let k1_p1 = SyscallPoint256 {
        x: [545887436851369351, 6828787688065214038, 3784847408168804653, 5801960250918850699],
        y: [7960650550461533833, 6752854485708698976, 7033117147444881223, 3794673124853157169],
    };
    let k1_p2 = SyscallPoint256 {
        x: [545887435628795271, 6828787688065214038, 3784847408168804653, 5801960250918850699],
        y: [17120705537189757149, 10411841749873587876, 3281661694091576154, 5012240766734575141],
    };
    let mut p1 = k1_p1;
    syscall_secp256k1_add(&mut SyscallSecp256k1AddParams { p1: &mut p1, p2: &k1_p2 });
    let mut p1_be = be_point256(&k1_p1);
    let p2_be = be_point256(&k1_p2);
    syscall_secp256k1_add_be(&mut SyscallSecp256k1AddParams { p1: &mut p1_be, p2: &p2_be });
    assert_eq!(be4(&p1_be.x), p1.x);
    assert_eq!(be4(&p1_be.y), p1.y);

    let mut p1 = k1_p1;
    syscall_secp256k1_dbl(&mut p1);
    let mut p1_be = be_point256(&k1_p1);
    syscall_secp256k1_dbl_be(&mut p1_be);
    assert_eq!(be4(&p1_be.x), p1.x);
    assert_eq!(be4(&p1_be.y), p1.y);

    //////////////
    // Secp256r1 add / dbl
    //////////////

    let r1_p1 = SyscallPoint256 {
        x: [16892139108441294589, 2268514658981987564, 16660966552361989397, 8255624973293160409],
        y: [12156787655537736728, 12309038971325631416, 14142088599640780838, 10821218243387965909],
    };
    let r1_p2 = SyscallPoint256 {
        x: [16973845534452172152, 13329017265419983731, 4846551096244594695, 16860145613434125549],
        y: [6998788062897069940, 14598273846219819996, 11018065767077453577, 15562535120954986242],
    };
    let mut p1 = r1_p1;
    syscall_secp256r1_add(&mut SyscallSecp256r1AddParams { p1: &mut p1, p2: &r1_p2 });
    let mut p1_be = be_point256(&r1_p1);
    let p2_be = be_point256(&r1_p2);
    syscall_secp256r1_add_be(&mut SyscallSecp256r1AddParams { p1: &mut p1_be, p2: &p2_be });
    assert_eq!(be4(&p1_be.x), p1.x);
    assert_eq!(be4(&p1_be.y), p1.y);

    let r1_q = SyscallPoint256 {
        x: [14589376466333626517, 2958991646043105326, 18026143254603003885, 13357802854741894370],
        y: [12670586254094269143, 17744077126965479691, 406637352836607580, 2297867320012306620],
    };
    let mut p1 = r1_q;
    syscall_secp256r1_dbl(&mut p1);
    let mut p1_be = be_point256(&r1_q);
    syscall_secp256r1_dbl_be(&mut p1_be);
    assert_eq!(be4(&p1_be.x), p1.x);
    assert_eq!(be4(&p1_be.y), p1.y);

    //////////////
    // BN254 curve add / dbl
    //////////////

    let bn_p1 = SyscallPoint256 {
        x: [1937747923122538908, 1973324843328483090, 18142721628580188222, 2340501145950218557],
        y: [4703919470647230067, 10779413178007862371, 11339051302474013312, 1212824066902237910],
    };
    let bn_p2 = SyscallPoint256 {
        x: [13514185463848566744, 16303451592415587669, 11454991405554316314, 2074786116747213803],
        y: [6215561102844887725, 9765353320242779493, 12761554255656424377, 3362982526011321696],
    };
    let mut p1 = bn_p1;
    syscall_bn254_curve_add(&mut SyscallBn254CurveAddParams { p1: &mut p1, p2: &bn_p2 });
    let mut p1_be = be_point256(&bn_p1);
    let p2_be = be_point256(&bn_p2);
    syscall_bn254_curve_add_be(&mut SyscallBn254CurveAddParams { p1: &mut p1_be, p2: &p2_be });
    assert_eq!(be4(&p1_be.x), p1.x);
    assert_eq!(be4(&p1_be.y), p1.y);

    let bn_q = SyscallPoint256 {
        x: [12493447835972542528, 13188422351013697901, 16114864060047456162, 162574568017230268],
        y: [9272304904258690271, 6760237032834658942, 3603577630588605141, 1176692479148410544],
    };
    let mut p1 = bn_q;
    syscall_bn254_curve_dbl(&mut p1);
    let mut p1_be = be_point256(&bn_q);
    syscall_bn254_curve_dbl_be(&mut p1_be);
    assert_eq!(be4(&p1_be.x), p1.x);
    assert_eq!(be4(&p1_be.y), p1.y);

    //////////////
    // BN254 complex add / sub / mul
    //////////////

    let f1 = SyscallComplex256 { x: bn_p1.x, y: bn_p1.y };
    let f2 = SyscallComplex256 { x: bn_p2.x, y: bn_p2.y };

    let mut g = SyscallComplex256 { x: f1.x, y: f1.y };
    syscall_bn254_complex_add(&mut SyscallBn254ComplexAddParams { f1: &mut g, f2: &f2 });
    let mut g_be = be_complex256(&f1);
    let f2_be = be_complex256(&f2);
    syscall_bn254_complex_add_be(&mut SyscallBn254ComplexAddParams { f1: &mut g_be, f2: &f2_be });
    assert_eq!(be4(&g_be.x), g.x);
    assert_eq!(be4(&g_be.y), g.y);

    let mut g = SyscallComplex256 { x: f1.x, y: f1.y };
    syscall_bn254_complex_sub(&mut SyscallBn254ComplexSubParams { f1: &mut g, f2: &f2 });
    let mut g_be = be_complex256(&f1);
    syscall_bn254_complex_sub_be(&mut SyscallBn254ComplexSubParams { f1: &mut g_be, f2: &f2_be });
    assert_eq!(be4(&g_be.x), g.x);
    assert_eq!(be4(&g_be.y), g.y);

    let mut g = SyscallComplex256 { x: f1.x, y: f1.y };
    syscall_bn254_complex_mul(&mut SyscallBn254ComplexMulParams { f1: &mut g, f2: &f2 });
    let mut g_be = be_complex256(&f1);
    syscall_bn254_complex_mul_be(&mut SyscallBn254ComplexMulParams { f1: &mut g_be, f2: &f2_be });
    assert_eq!(be4(&g_be.x), g.x);
    assert_eq!(be4(&g_be.y), g.y);

    //////////////
    // Arith384Mod
    //////////////

    let a6 = [
        5136610938092865425,
        17935124486258304099,
        8889452088341721703,
        8081987522574618315,
        17020937355979903359,
        1352623576164215061,
    ];
    let b6 = [
        11536735903087902934,
        7558641295849164042,
        4531251747467874756,
        8656747436063809405,
        2217730653216186968,
        1601157506009751932,
    ];
    let c6 = [
        9566187608297515616,
        6669319752019508922,
        15726188127343628600,
        5212981008286159483,
        13098816980402680833,
        149896740736755094,
    ];
    let m6 = [
        7065999913023876908,
        17866142133974605858,
        7695340824857769759,
        761880360145594770,
        14206191995173209138,
        1275524973012338957,
    ];
    let mut d6 = [0u64; 6];
    syscall_arith384_mod(&mut SyscallArith384ModParams {
        a: &a6,
        b: &b6,
        c: &c6,
        module: &m6,
        d: &mut d6,
    });
    let (a6_be, b6_be, c6_be, m6_be) = (be6(&a6), be6(&b6), be6(&c6), be6(&m6));
    let mut d6_be = [0u64; 6];
    syscall_arith384_mod_be(&mut SyscallArith384ModParams {
        a: &a6_be,
        b: &b6_be,
        c: &c6_be,
        module: &m6_be,
        d: &mut d6_be,
    });
    assert_eq!(be6(&d6_be), d6);

    //////////////
    // BLS12-381 curve add / dbl
    //////////////

    let bls_p1 = SyscallPoint384 { x: a6, y: m6 };
    let bls_p2 = SyscallPoint384 { x: b6, y: c6 };
    let mut p1 = SyscallPoint384 { x: bls_p1.x, y: bls_p1.y };
    syscall_bls12_381_curve_add(&mut SyscallBls12_381CurveAddParams { p1: &mut p1, p2: &bls_p2 });
    let mut p1_be = be_point384(&bls_p1);
    let p2_be = be_point384(&bls_p2);
    syscall_bls12_381_curve_add_be(&mut SyscallBls12_381CurveAddParams {
        p1: &mut p1_be,
        p2: &p2_be,
    });
    assert_eq!(be6(&p1_be.x), p1.x);
    assert_eq!(be6(&p1_be.y), p1.y);

    let bls_q = SyscallPoint384 {
        x: [
            3246708282719638729,
            2123860621819329771,
            11151416973466624961,
            16715199801812194438,
            2373035793620641644,
            875050225373976086,
        ],
        y: [
            9181485018221855171,
            15327706859763244853,
            5036390123292329947,
            1065134743266288845,
            14607124613060412546,
            1683477985824916598,
        ],
    };
    let mut p1 = SyscallPoint384 { x: bls_q.x, y: bls_q.y };
    syscall_bls12_381_curve_dbl(&mut p1);
    let mut p1_be = be_point384(&bls_q);
    syscall_bls12_381_curve_dbl_be(&mut p1_be);
    assert_eq!(be6(&p1_be.x), p1.x);
    assert_eq!(be6(&p1_be.y), p1.y);

    //////////////
    // BLS12-381 complex add / sub / mul
    //////////////

    let f1 = SyscallComplex384 { x: a6, y: m6 };
    let f2 = SyscallComplex384 { x: b6, y: c6 };

    let mut g = SyscallComplex384 { x: f1.x, y: f1.y };
    syscall_bls12_381_complex_add(&mut SyscallBls12_381ComplexAddParams { f1: &mut g, f2: &f2 });
    let mut g_be = be_complex384(&f1);
    let f2_be = be_complex384(&f2);
    syscall_bls12_381_complex_add_be(&mut SyscallBls12_381ComplexAddParams {
        f1: &mut g_be,
        f2: &f2_be,
    });
    assert_eq!(be6(&g_be.x), g.x);
    assert_eq!(be6(&g_be.y), g.y);

    let mut g = SyscallComplex384 { x: f1.x, y: f1.y };
    syscall_bls12_381_complex_sub(&mut SyscallBls12_381ComplexSubParams { f1: &mut g, f2: &f2 });
    let mut g_be = be_complex384(&f1);
    syscall_bls12_381_complex_sub_be(&mut SyscallBls12_381ComplexSubParams {
        f1: &mut g_be,
        f2: &f2_be,
    });
    assert_eq!(be6(&g_be.x), g.x);
    assert_eq!(be6(&g_be.y), g.y);

    let mut g = SyscallComplex384 { x: f1.x, y: f1.y };
    syscall_bls12_381_complex_mul(&mut SyscallBls12_381ComplexMulParams { f1: &mut g, f2: &f2 });
    let mut g_be = be_complex384(&f1);
    syscall_bls12_381_complex_mul_be(&mut SyscallBls12_381ComplexMulParams {
        f1: &mut g_be,
        f2: &f2_be,
    });
    assert_eq!(be6(&g_be.x), g.x);
    assert_eq!(be6(&g_be.y), g.y);
}
