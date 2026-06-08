use ziskos::syscalls::*;

pub fn diagnostic_bn254() {
    //////////////
    // Bn254 Add Tests
    //////////////

    let mut p1 = SyscallPoint256 { x: [0, 0, 0, 0], y: [0, 0, 0, 0] };
    let p2 = SyscallPoint256 { x: [0, 0, 0, 0], y: [0, 0, 0, 0] };
    let mut params = SyscallBn254CurveAddParams { p1: &mut p1, p2: &p2 };

    let mut p1 = SyscallPoint256 {
        x: [1937747923122538908, 1973324843328483090, 18142721628580188222, 2340501145950218557],
        y: [4703919470647230067, 10779413178007862371, 11339051302474013312, 1212824066902237910],
    };
    let p2 = SyscallPoint256 {
        x: [13514185463848566744, 16303451592415587669, 11454991405554316314, 2074786116747213803],
        y: [6215561102844887725, 9765353320242779493, 12761554255656424377, 3362982526011321696],
    };
    params.p1 = &mut p1;
    params.p2 = &p2;
    syscall_bn254_curve_add(&mut params);
    let p3 = SyscallPoint256 {
        x: [1988899826776024073, 6263940785541754410, 12804075266116178733, 824066112053521922],
        y: [6523803231338234751, 17167368501758989829, 13151069636843539260, 2060995759643982023],
    };
    assert_eq!(params.p1.x, p3.x);
    assert_eq!(params.p1.y, p3.y);

    //////////////
    // Bn254 Dbl Tests
    //////////////

    let mut p1 = SyscallPoint256 {
        x: [12493447835972542528, 13188422351013697901, 16114864060047456162, 162574568017230268],
        y: [9272304904258690271, 6760237032834658942, 3603577630588605141, 1176692479148410544],
    };
    syscall_bn254_curve_dbl(&mut p1);
    let p3 = SyscallPoint256 {
        x: [15160040376694794067, 12003148044313189826, 18438304264779973344, 77745216204838149],
        y: [5906181427586509466, 13809883834763246589, 8447866917983781356, 1777471803573943266],
    };
    assert_eq!(p1.x, p3.x);
    assert_eq!(p1.y, p3.y);

    //////////////
    // Complex Add Tests
    //////////////

    let mut f1 = SyscallComplex256 { x: [0, 0, 0, 0], y: [0, 0, 0, 0] };
    let f2 = SyscallComplex256 { x: [0, 0, 0, 0], y: [0, 0, 0, 0] };
    let mut params = SyscallBn254ComplexAddParams { f1: &mut f1, f2: &f2 };

    let mut f1 = SyscallComplex256 { x: [0, 0, 0, 0], y: [0, 0, 0, 0] };
    let f2 = SyscallComplex256 { x: [1, 0, 0, 0], y: [0, 0, 0, 0] };
    params.f1 = &mut f1;
    params.f2 = &f2;
    syscall_bn254_complex_add(&mut params);
    let f3 = SyscallComplex256 { x: [1, 0, 0, 0], y: [0, 0, 0, 0] };
    assert_eq!(params.f1.x, f3.x);
    assert_eq!(params.f1.y, f3.y);

    //////////////
    // Complex Sub Tests
    //////////////

    let mut params = SyscallBn254ComplexSubParams { f1: &mut f1, f2: &f2 };

    let mut f1 = SyscallComplex256 { x: [0, 0, 0, 0], y: [0, 0, 0, 0] };
    let f2 = SyscallComplex256 { x: [1, 0, 0, 0], y: [0, 0, 0, 0] };
    params.f1 = &mut f1;
    params.f2 = &f2;
    syscall_bn254_complex_sub(&mut params);
    let f3 = SyscallComplex256 {
        x: [4332616871279656262, 10917124144477883021, 13281191951274694749, 3486998266802970665],
        y: [0, 0, 0, 0],
    };
    assert_eq!(params.f1.x, f3.x);
    assert_eq!(params.f1.y, f3.y);

    //////////////
    // Complex Mul Tests
    //////////////

    let mut params = SyscallBn254ComplexMulParams { f1: &mut f1, f2: &f2 };

    let mut f1 = SyscallComplex256 {
        x: [4332616871279656262, 10917124144477883021, 13281191951274694749, 3486998266802970665],
        y: [4332616871279656262, 10917124144477883021, 13281191951274694749, 3486998266802970665],
    };
    let f2 = SyscallComplex256 {
        x: [4332616871279656262, 10917124144477883021, 13281191951274694749, 3486998266802970665],
        y: [4332616871279656262, 10917124144477883021, 13281191951274694749, 3486998266802970665],
    };
    params.f1 = &mut f1;
    params.f2 = &f2;
    syscall_bn254_complex_mul(&mut params);
    let f3 = SyscallComplex256 { x: [0, 0, 0, 0], y: [2, 0, 0, 0] };
    assert_eq!(params.f1.x, f3.x);
    assert_eq!(params.f1.y, f3.y);
}
