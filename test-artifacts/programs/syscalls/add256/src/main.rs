#![no_main]
ziskos::entrypoint!(main);

use ziskos::syscalls::{syscall_add256, SyscallAdd256Params};

fn main() {
    let a: [u64; 4] = [0, 0, 0, 0];
    let b: [u64; 4] = [0, 0, 0, 0];
    let mut c: [u64; 4] = [0, 0, 0, 0];
    let mut params = SyscallAdd256Params { a: &a, b: &b, cin: 0, c: &mut c };

    // Test #0: add256
    params.a = &[0, 0, 0, 0];
    params.b = &[0, 0, 0, 0];
    params.cin = 0;
    let cout = syscall_add256(&mut params);
    let expected_c: [u64; 4] = [0, 0, 0, 0];
    let expected_cout: u64 = 0;
    assert_eq!(params.c, &expected_c);
    assert_eq!(cout, expected_cout);

    // Test #1: add256
    params.a = &[11, 9, 7, 5];
    params.b = &[12, 10, 8, 6];
    params.cin = 0;
    let cout = syscall_add256(&mut params);
    let expected_c: [u64; 4] = [23, 19, 15, 11];
    let expected_cout: u64 = 0;
    assert_eq!(params.c, &expected_c);
    assert_eq!(cout, expected_cout);

    // Test #2: add256
    params.a =
        &[18446744073709551615, 18446744073709551615, 18446744073709551615, 18446744073709551615];
    params.b = &[1, 2, 3, 4];
    params.cin = 0;
    let cout = syscall_add256(&mut params);
    let expected_c: [u64; 4] = [0, 2, 3, 4];
    let expected_cout: u64 = 1;
    assert_eq!(params.c, &expected_c);
    assert_eq!(cout, expected_cout);

    // Test #3: add256
    params.a =
        &[18446744073709551615, 18446744073709551615, 18446744073709551615, 18446744073709551614];
    params.b = &[1, 0, 0, 0];
    params.cin = 0;
    let cout = syscall_add256(&mut params);
    let expected_c: [u64; 4] = [0, 0, 0, 18446744073709551615];
    let expected_cout: u64 = 0;
    assert_eq!(params.c, &expected_c);
    assert_eq!(cout, expected_cout);

    // Test #4: add256
    params.a = &[100, 200, 300, 400];
    params.b = &[50, 75, 125, 175];
    params.cin = 1;
    let cout = syscall_add256(&mut params);
    let expected_c: [u64; 4] = [151, 275, 425, 575];
    let expected_cout: u64 = 0;
    assert_eq!(params.c, &expected_c);
    assert_eq!(cout, expected_cout);

    // Test #5: add256
    params.a = &[0, 0, 0, 0];
    params.b = &[0, 0, 0, 0];
    params.cin = 0;
    let cout = syscall_add256(&mut params);
    let expected_c: [u64; 4] = [0, 0, 0, 0];
    let expected_cout: u64 = 0;
    assert_eq!(params.c, &expected_c);
    assert_eq!(cout, expected_cout);

    // Test #6: add256
    params.a = &[0, 0, 0, 0];
    params.b = &[0, 0, 0, 0];
    params.cin = 1;
    let cout = syscall_add256(&mut params);
    let expected_c: [u64; 4] = [1, 0, 0, 0];
    let expected_cout: u64 = 0;
    assert_eq!(params.c, &expected_c);
    assert_eq!(cout, expected_cout);

    // Test #7: add256
    params.a =
        &[9223372036854775808, 4611686018427387904, 2305843009213693952, 1152921504606846976];
    params.b =
        &[1152921504606846976, 2305843009213693952, 4611686018427387904, 9223372036854775808];
    params.cin = 0;
    let cout = syscall_add256(&mut params);
    let expected_c: [u64; 4] =
        [10376293541461622784, 6917529027641081856, 6917529027641081856, 10376293541461622784];
    let expected_cout: u64 = 0;
    assert_eq!(params.c, &expected_c);
    assert_eq!(cout, expected_cout);

    // Test #8: add256
    params.a = &[18446744073709551615, 0, 0, 0];
    params.b = &[1, 0, 0, 0];
    params.cin = 0;
    let cout = syscall_add256(&mut params);
    let expected_c: [u64; 4] = [0, 1, 0, 0];
    let expected_cout: u64 = 0;
    assert_eq!(params.c, &expected_c);
    assert_eq!(cout, expected_cout);

    // Test #9: add256
    params.a = &[0, 18446744073709551615, 0, 0];
    params.b = &[0, 1, 0, 0];
    params.cin = 0;
    let cout = syscall_add256(&mut params);
    let expected_c: [u64; 4] = [0, 0, 1, 0];
    let expected_cout: u64 = 0;
    assert_eq!(params.c, &expected_c);
    assert_eq!(cout, expected_cout);

    // Test #10: add256
    params.a = &[0, 0, 18446744073709551615, 0];
    params.b = &[0, 0, 1, 0];
    params.cin = 0;
    let cout = syscall_add256(&mut params);
    let expected_c: [u64; 4] = [0, 0, 0, 1];
    let expected_cout: u64 = 0;
    assert_eq!(params.c, &expected_c);
    assert_eq!(cout, expected_cout);

    // Test #11: add256
    params.a = &[18446744073709551615, 18446744073709551615, 18446744073709551615, 0];
    params.b = &[1, 0, 0, 0];
    params.cin = 0;
    let cout = syscall_add256(&mut params);
    let expected_c: [u64; 4] = [0, 0, 0, 1];
    let expected_cout: u64 = 0;
    assert_eq!(params.c, &expected_c);
    assert_eq!(cout, expected_cout);

    // Test #12: add256
    params.a =
        &[18446744073709551615, 18446744073709551615, 18446744073709551615, 18446744073709551615];
    params.b = &[1, 0, 0, 0];
    params.cin = 0;
    let cout = syscall_add256(&mut params);
    let expected_c: [u64; 4] = [0, 0, 0, 0];
    let expected_cout: u64 = 1;
    assert_eq!(params.c, &expected_c);
    assert_eq!(cout, expected_cout);

    // Test #13: add256
    params.a = &[1, 2, 4, 8];
    params.b = &[16, 32, 64, 128];
    params.cin = 0;
    let cout = syscall_add256(&mut params);
    let expected_c: [u64; 4] = [17, 34, 68, 136];
    let expected_cout: u64 = 0;
    assert_eq!(params.c, &expected_c);
    assert_eq!(cout, expected_cout);

    // Test #14: add256
    params.a =
        &[12297829382473034410, 6148914691236517205, 12297829382473034410, 6148914691236517205];
    params.b =
        &[6148914691236517205, 12297829382473034410, 6148914691236517205, 12297829382473034410];
    params.cin = 0;
    let cout = syscall_add256(&mut params);
    let expected_c: [u64; 4] =
        [18446744073709551615, 18446744073709551615, 18446744073709551615, 18446744073709551615];
    let expected_cout: u64 = 0;
    assert_eq!(params.c, &expected_c);
    assert_eq!(cout, expected_cout);

    // Test #15: add256
    params.a =
        &[1311768467463790320, 18364758544493064720, 1229801703532086340, 4919150518273996663];
    params.b = &[1147797409030816545, 81985529216486895, 11068065209510513868, 2459584641779389781];
    params.cin = 0;
    let cout = syscall_add256(&mut params);
    let expected_c: [u64; 4] =
        [2459565876494606865, 18446744073709551615, 12297866913042600208, 7378735160053386444];
    let expected_cout: u64 = 0;
    assert_eq!(params.c, &expected_c);
    assert_eq!(cout, expected_cout);

    // Test #16: add256
    params.a =
        &[18446744073709551615, 18446744073709551615, 18446744073709551615, 18446744073709551615];
    params.b = &[0, 0, 0, 0];
    params.cin = 0;
    let cout = syscall_add256(&mut params);
    let expected_c: [u64; 4] =
        [18446744073709551615, 18446744073709551615, 18446744073709551615, 18446744073709551615];
    let expected_cout: u64 = 0;
    assert_eq!(params.c, &expected_c);
    assert_eq!(cout, expected_cout);

    // Test #17: add256
    params.a = &[1, 1, 1, 1];
    params.b = &[1, 1, 1, 1];
    params.cin = 0;
    let cout = syscall_add256(&mut params);
    let expected_c: [u64; 4] = [2, 2, 2, 2];
    let expected_cout: u64 = 0;
    assert_eq!(params.c, &expected_c);
    assert_eq!(cout, expected_cout);

    // Test #18: add256
    params.a =
        &[9223372036854775807, 9223372036854775807, 9223372036854775807, 9223372036854775807];
    params.b =
        &[9223372036854775807, 9223372036854775807, 9223372036854775807, 9223372036854775807];
    params.cin = 0;
    let cout = syscall_add256(&mut params);
    let expected_c: [u64; 4] =
        [18446744073709551614, 18446744073709551614, 18446744073709551614, 18446744073709551614];
    let expected_cout: u64 = 0;
    assert_eq!(params.c, &expected_c);
    assert_eq!(cout, expected_cout);

    // Test #19: add256
    params.a = &[18446744073709551614, 18446744073709551615, 0, 0];
    params.b = &[2, 0, 0, 0];
    params.cin = 0;
    let cout = syscall_add256(&mut params);
    let expected_c: [u64; 4] = [0, 0, 1, 0];
    let expected_cout: u64 = 0;
    assert_eq!(params.c, &expected_c);
    assert_eq!(cout, expected_cout);

    // Test #20: add256
    params.a = &[1, 2, 3, 4];
    params.b = &[5, 6, 7, 8];
    params.cin = 0;
    let cout = syscall_add256(&mut params);
    let expected_c: [u64; 4] = [6, 8, 10, 12];
    let expected_cout: u64 = 0;
    assert_eq!(params.c, &expected_c);
    assert_eq!(cout, expected_cout);

    // Test #21: add256
    params.a = &[10, 100, 1000, 10000];
    params.b = &[1, 10, 100, 1000];
    params.cin = 0;
    let cout = syscall_add256(&mut params);
    let expected_c: [u64; 4] = [11, 110, 1100, 11000];
    let expected_cout: u64 = 0;
    assert_eq!(params.c, &expected_c);
    assert_eq!(cout, expected_cout);

    // Test #22: add256
    params.a = &[1, 1, 2, 3];
    params.b = &[5, 8, 13, 21];
    params.cin = 0;
    let cout = syscall_add256(&mut params);
    let expected_c: [u64; 4] = [6, 9, 15, 24];
    let expected_cout: u64 = 0;
    assert_eq!(params.c, &expected_c);
    assert_eq!(cout, expected_cout);

    // Test #23: add256
    params.a = &[2, 3, 5, 7];
    params.b = &[11, 13, 17, 19];
    params.cin = 0;
    let cout = syscall_add256(&mut params);
    let expected_c: [u64; 4] = [13, 16, 22, 26];
    let expected_cout: u64 = 0;
    assert_eq!(params.c, &expected_c);
    assert_eq!(cout, expected_cout);

    // Test #24: add256
    params.a = &[3735928559, 3405691582, 4207849484, 3235827725];
    params.b = &[195936478, 4277009102, 322420463, 3735928559];
    params.cin = 0;
    let cout = syscall_add256(&mut params);
    let expected_c: [u64; 4] = [3931865037, 7682700684, 4530269947, 6971756284];
    let expected_cout: u64 = 0;
    assert_eq!(params.c, &expected_c);
    assert_eq!(cout, expected_cout);

    // Test #25: add256
    params.a = &[1, 2, 4, 8];
    params.b = &[16, 32, 64, 128];
    params.cin = 0;
    let cout = syscall_add256(&mut params);
    let expected_c: [u64; 4] = [17, 34, 68, 136];
    let expected_cout: u64 = 0;
    assert_eq!(params.c, &expected_c);
    assert_eq!(cout, expected_cout);

    // Test #26: add256
    params.a =
        &[9223372036854775808, 9223372036854775808, 9223372036854775808, 9223372036854775808];
    params.b =
        &[9223372036854775807, 9223372036854775807, 9223372036854775807, 9223372036854775807];
    params.cin = 0;
    let cout = syscall_add256(&mut params);
    let expected_c: [u64; 4] =
        [18446744073709551615, 18446744073709551615, 18446744073709551615, 18446744073709551615];
    let expected_cout: u64 = 0;
    assert_eq!(params.c, &expected_c);
    assert_eq!(cout, expected_cout);

    // Test #27: add256
    params.a =
        &[18446744073709551615, 18446744073709551615, 18446744073709551615, 9223372036854775807];
    params.b = &[0, 0, 0, 9223372036854775808];
    params.cin = 1;
    let cout = syscall_add256(&mut params);
    let expected_c: [u64; 4] = [0, 0, 0, 0];
    let expected_cout: u64 = 1;
    assert_eq!(params.c, &expected_c);
    assert_eq!(cout, expected_cout);

    // Test #28: add256
    params.a =
        &[17361641481138401520, 1085102592571150095, 17361641481138401520, 1085102592571150095];
    params.b =
        &[1085102592571150095, 17361641481138401520, 1085102592571150095, 17361641481138401520];
    params.cin = 0;
    let cout = syscall_add256(&mut params);
    let expected_c: [u64; 4] =
        [18446744073709551615, 18446744073709551615, 18446744073709551615, 18446744073709551615];
    let expected_cout: u64 = 0;
    assert_eq!(params.c, &expected_c);
    assert_eq!(cout, expected_cout);

    // Test #29: add256
    params.a = &[1000000, 2000000, 3000000, 4000000];
    params.b = &[500000, 750000, 1250000, 1750000];
    params.cin = 0;
    let cout = syscall_add256(&mut params);
    let expected_c: [u64; 4] = [1500000, 2750000, 4250000, 5750000];
    let expected_cout: u64 = 0;
    assert_eq!(params.c, &expected_c);
    assert_eq!(cout, expected_cout);

    // Test #30: add256
    params.a =
        &[18446744073709551614, 18446744073709551615, 18446744073709551615, 18446744073709551615];
    params.b = &[0, 0, 0, 0];
    params.cin = 1;
    let cout = syscall_add256(&mut params);
    let expected_c: [u64; 4] =
        [18446744073709551615, 18446744073709551615, 18446744073709551615, 18446744073709551615];
    let expected_cout: u64 = 0;
    assert_eq!(params.c, &expected_c);
    assert_eq!(cout, expected_cout);

    // Test #31: add256
    params.a =
        &[18446744073709551615, 18446744073709551615, 18446744073709551615, 18446744073709551614];
    params.b = &[0, 0, 0, 0];
    params.cin = 1;
    let cout = syscall_add256(&mut params);
    let expected_c: [u64; 4] = [0, 0, 0, 18446744073709551615];
    let expected_cout: u64 = 0;
    assert_eq!(params.c, &expected_c);
    assert_eq!(cout, expected_cout);

    // Test #32: add256
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

    // Test #33: add256
    params.a = &[42, 1337, 3735928559, 2343432205];
    params.b = &[13, 42, 305419896, 2271560481];
    params.cin = 0;
    let cout = syscall_add256(&mut params);
    let expected_c: [u64; 4] = [55, 1379, 4041348455, 4614992686];
    let expected_cout: u64 = 0;
    assert_eq!(params.c, &expected_c);
    assert_eq!(cout, expected_cout);
}
