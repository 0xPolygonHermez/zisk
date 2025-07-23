use crate::{
    fcall_div_rem_256,
    syscalls::arith256::{syscall_arith256, SyscallArith256Params},
};

#[no_mangle]
unsafe extern "C" fn mul256(result: *mut u64, x: *const u64, y: *const u64) {
    let a = &*(x as *const [u64; 4]);
    let b = &*(y as *const [u64; 4]);
    let r = &mut *(result as *mut [u64; 4]);

    let mut params = SyscallArith256Params { a, b, c: &[0, 0, 0, 0], dl: r, dh: &mut [0, 0, 0, 0] };
    syscall_arith256(&mut params);
}

#[no_mangle]
unsafe extern "C" fn div256(result: *mut u64, x: *const u64, y: *const u64) {
    let x = &*(x as *const [u64; 4]);
    let y = &*(y as *const [u64; 4]);
    let r = &mut *(result as *mut [u64; 4]);

    // Hint the result of the division
    let (div, rem) = fcall_div_rem_256(x, y);

    // Check that x = div·y + rem (over the integers)
    let mut params = SyscallArith256Params {
        a: &div,
        b: y,
        c: &rem,
        dl: r,
        dh: &mut [0, 0, 0, 0],
    };
    syscall_arith256(&mut params);
    assert_eq!(*params.dl, *x);
    assert_eq!(*params.dh, [0, 0, 0, 0]);

    *r = div;
}

#[no_mangle]
unsafe extern "C" fn rem256(result: *mut u64, x: *const u64, y: *const u64) {
    let x = &*(x as *const [u64; 4]);
    let y = &*(y as *const [u64; 4]);
    let r = &mut *(result as *mut [u64; 4]);

    // Hint the result of the division
    let (div, rem) = fcall_div_rem_256(x, y);

    // Check that x = div·y + rem (over the integers)
    let mut params = SyscallArith256Params {
        a: &div,
        b: y,
        c: &rem,
        dl: r,
        dh: &mut [0, 0, 0, 0],
    };
    syscall_arith256(&mut params);
    assert_eq!(*params.dl, *x);
    assert_eq!(*params.dh, [0, 0, 0, 0]);

    *r = rem;
}
