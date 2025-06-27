use crate::arith256_mod::{syscall_arith256_mod, SyscallArith256ModParams};

pub fn from_be_bytes_to_u64_array(bytes: &[u8; 32]) -> [u64; 4] {
    let mut result = [0u64; 4];
    for i in 0..4 {
        result[4 - 1 - i] = u64::from_be_bytes(bytes[i * 8..(i + 1) * 8].try_into().unwrap());
    }
    result
}

pub fn from_u64_array_to_be_bytes(arr: &[u64; 4]) -> [u8; 32] {
    let mut result = [0u8; 32];
    for i in 0..4 {
        result[i * 8..(i + 1) * 8].copy_from_slice(&arr[4 - 1 - i].to_be_bytes());
    }
    result
}

/// Given two n-bit number `x` and `y`, compares them and returns true if `x > y`; otherwise, false.
pub fn gt(x: &[u64], y: &[u64]) -> bool {
    let len = x.len();
    assert_eq!(len, y.len(), "x and y must have the same length");

    for i in (0..len).rev() {
        if x[i] > y[i] {
            return true;
        } else if x[i] < y[i] {
            return false;
        }
    }
    false
}

/// Given two n-bit number `x` and `y`, compares them and returns true if `x == y`; otherwise, false.
pub fn eq(x: &[u64], y: &[u64]) -> bool {
    let len = x.len();
    assert_eq!(len, y.len(), "x and y must have the same length");

    for i in 0..len {
        if x[i] != y[i] {
            return false;
        }
    }
    true
}

/// Raises `x` to (2^power_log) modulo `module` using repeated squaring
/// Performs all operations in RISC-V assembly for maximum performance
pub fn exp_power_of_two(x: &[u64; 4], module: &[u64; 4], power_log: usize) -> [u64; 4] {
    // x^1 = x
    if power_log == 0 {
        return *x;
    }

    let mut result = *x;
    let zero = [0u64; 4];
    for _ in 0..power_log {
        let mut params = SyscallArith256ModParams {
            a: &result,
            b: &result,
            c: &zero,
            module,
            d: &mut [0u64; 4],
        };
        syscall_arith256_mod(&mut params);
        result = *params.d;
    }

    result
}

/// Raises `x` to (2^power_log) modulo `module` using repeated squaring
/// Performs all operations in RISC-V assembly for maximum performance
pub fn exp_power_of_two_self(x: &mut [u64; 4], power_log: usize, module: &[u64; 4]) {
    if power_log == 0 {
        return;
    }

    let zero = [0u64; 4];
    for _ in 0..power_log {
        let mut params =
            SyscallArith256ModParams { a: x, b: x, c: &zero, module, d: &mut [0u64; 4] };
        syscall_arith256_mod(&mut params);
        *x = *params.d;
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_exp_power_of_two_self() {
        let module: [u64; 4] =
            [0xfffffffefffffc2f, 0xffffffffffffffff, 0xffffffffffffffff, 0xffffffffffffffff];
        
        // 1
        let mut x: [u64; 4] =
            [0xc9b03b176c169088, 0xd1d94829bb3cc946, 0x39349d1bf4b794cf, 0x004e92b17f7142c5];
        let expected: [u64; 4] =
            [0x3d7573f55003f36e, 0x1b217887da15fdac, 0xb069b2742d79dc5e, 0x73e89cfc6bb8fa9e];
        exp_power_of_two_self(&mut x, 10, &module);
        assert_eq!(x, expected);

        // 2
        let mut x: [u64; 4] =
            [0xc9b03b176c169088, 0xd1d94829bb3cc946, 0x39349d1bf4b794cf, 0x004e92b17f7142c5];
        let expected: [u64; 4] =
            [0x95b59929f9ea03da, 0x67cc3f4fd5301d0c, 0x422b3a9b826a93de, 0x429e698354dcbaaa];
        exp_power_of_two_self(&mut x, 80, &module);
        assert_eq!(x, expected);

        // 3
        let mut x: [u64; 4] =
            [0xc9b03b176c169088, 0xd1d94829bb3cc946, 0x39349d1bf4b794cf, 0x004e92b17f7142c5];
        let expected: [u64; 4] =
            [0x3e288b8628a05fe0, 0x578b5d1f083bee72, 0x91fc47418f5e2985, 0xbc0fb7af2f7fb40f];
        exp_power_of_two_self(&mut x, 255, &module);
        assert_eq!(x, expected);

        // 4
        let mut x: [u64; 4] =
            [0xc9b03b176c169088, 0xd1d94829bb3cc946, 0x39349d1bf4b794cf, 0x004e92b17f7142c5];
        let expected: [u64; 4] =
            [0x3e288b8628a05fe0, 0x578b5d1f083bee72, 0x91fc47418f5e2985, 0xbc0fb7af2f7fb40f];
        exp_power_of_two_self(&mut x, 255, &module);
        assert_eq!(x, expected);

        // 5
        let mut x: [u64; 4] =
            [0x3459b1f9580b9677, 0xc125fab48e837e21, 0xa5282714b8c69f4c, 0x0e091dcd4a038928];
        let expected: [u64; 4] =
            [0x6f492348f0f5a6f4, 0x0bb6684a47500bdd, 0x8304fa7877bfbc3f, 0x31148b042c7790f2];
        exp_power_of_two_self(&mut x, 1313, &module);
        assert_eq!(x, expected);

        // 6
        let mut x: [u64; 4] =
            [0x3459b1f9580b9677, 0xc125fab48e837e21, 0xa5282714b8c69f4c, 0x0e091dcd4a038928];
        let expected: [u64; 4] =
            [0x3459b1f9580b9677, 0xc125fab48e837e21, 0xa5282714b8c69f4c, 0x0e091dcd4a038928];
        exp_power_of_two_self(&mut x, 0, &module);
        assert_eq!(x, expected);

        // 7
        let mut x: [u64; 4] =
            [0x3459b1f9580b9677, 0xc125fab48e837e21, 0xa5282714b8c69f4c, 0x0e091dcd4a038928];
        let expected: [u64; 4] =
            [0x0748d85c03db12e3, 0x7c796302363bd056, 0xcd48b1ae22dea873, 0xc0cd0f9f5328c690];
        exp_power_of_two_self(&mut x, 1, &module);
        assert_eq!(x, expected);
    }
}