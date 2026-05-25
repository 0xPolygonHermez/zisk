use ziskos::syscalls::*;

pub fn diagnostic_arith256() {
    //////////////
    // Add256 Tests
    //////////////

    let a: [u64; 4] = [0, 0, 0, 0];
    let b: [u64; 4] = [0, 0, 0, 0];
    let mut c: [u64; 4] = [0, 0, 0, 0];
    let mut params = SyscallAdd256Params { a: &a, b: &b, cin: 0, c: &mut c };

    params.a =
        &[1229782938247303441, 2459565876494606882, 3689348814741910323, 4919131752989213764];
    params.b =
        &[17216961135462248174, 15987178197214944733, 14757395258967641292, 13527612320720337851];
    params.cin = 1;
    let cout = syscall_add256(&mut params);
    let expected_c: [u64; 4] = [0, 0, 0, 0];
    let expected_cout: u64 = 1;
    assert_eq!(params.c, &expected_c);
    assert_eq!(cout, expected_cout);

    params.a = &[42, 1337, 3735928559, 2343432205];
    params.b = &[13, 42, 305419896, 2271560481];
    params.cin = 0;
    let cout = syscall_add256(&mut params);
    let expected_c: [u64; 4] = [55, 1379, 4041348455, 4614992686];
    let expected_cout: u64 = 0;
    assert_eq!(params.c, &expected_c);
    assert_eq!(cout, expected_cout);

    //////////////
    // Arith256 Tests
    //////////////

    let a: [u64; 4] = [0, 0, 0, 0];
    let b: [u64; 4] = [0, 0, 0, 0];
    let c: [u64; 4] = [0, 0, 0, 0];
    let mut dl: [u64; 4] = [0, 0, 0, 0];
    let mut dh: [u64; 4] = [0, 0, 0, 0];
    let mut params = SyscallArith256Params { a: &a, b: &b, c: &c, dl: &mut dl, dh: &mut dh };

    params.a = &[3, 0, 0, 0];
    params.b = &[2, 0, 0, 0];
    params.c = &[5, 0, 0, 0];
    syscall_arith256(&mut params);
    let expected_dh: [u64; 4] = [0, 0, 0, 0];
    let expected_dl: [u64; 4] = [11, 0, 0, 0];
    assert_eq!(params.dh, &expected_dh);
    assert_eq!(params.dl, &expected_dl);

    params.a =
        &[13970229013151504741, 8476296752562947313, 11810450538887363942, 511990551865481398];
    params.b =
        &[11990850244716481796, 14558188671963395327, 9424388055416098482, 1459171711273467932];
    params.c =
        &[16528603495754341937, 8893271371239080203, 9406449307822347647, 250213327518958686];
    syscall_arith256(&mut params);
    let expected_dh: [u64; 4] =
        [4910774022637574197, 12870152955407492665, 17746475360205808972, 40499403403452059];
    let expected_dl: [u64; 4] =
        [3242244678432810181, 2099669192879440901, 14496343886419199978, 10002311647969911313];
    assert_eq!(params.dh, &expected_dh);
    assert_eq!(params.dl, &expected_dl);

    //////////////
    // Arith256Mod Tests
    //////////////

    let a: [u64; 4] = [0, 0, 0, 0];
    let b: [u64; 4] = [0, 0, 0, 0];
    let c: [u64; 4] = [0, 0, 0, 0];
    let module: [u64; 4] = [0, 0, 0, 0];
    let mut d: [u64; 4] = [0, 0, 0, 0];
    let mut params = SyscallArith256ModParams { a: &a, b: &b, c: &c, module: &module, d: &mut d };

    params.a = &[0, 0, 0, 0];
    params.b = &[0, 0, 0, 0];
    params.c = &[0, 0, 0, 0];
    params.module = &[1, 0, 0, 0];
    syscall_arith256_mod(&mut params);
    let expected_d: [u64; 4] = [0, 0, 0, 0];
    assert_eq!(params.d, &expected_d);

    params.a =
        &[4332616871279656262, 10917124144477883021, 13281191951274694749, 3486998266802970665];
    params.b =
        &[4332616871279656262, 10917124144477883021, 13281191951274694749, 3486998266802970665];
    params.c =
        &[4332616871279656262, 10917124144477883021, 13281191951274694749, 3486998266802970665];
    params.module =
        &[4332616871279656262, 10917124144477883021, 13281191951274694749, 3486998266802970665];
    syscall_arith256_mod(&mut params);
    let expected_d: [u64; 4] = [0, 0, 0, 0];
    assert_eq!(params.d, &expected_d);
}
