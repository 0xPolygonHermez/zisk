// Re-exported so the guest and host can use the same `rkyv` version as the derive macros below.
pub use rkyv;

// Used by the host for serialization and deserialization of input data.
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct InputDTO {
    pub values: Vec<u64>,
}

// Used by the guest for zero-copy deserialization, avoiding heap allocation overhead in the guest.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct InputZeroCopyDTO {
    pub values: Vec<u64>,
}

/// Returns `true` if `n` is a prime number, `false` otherwise.
pub fn is_prime(n: &u64) -> bool {
    if *n < 2 {
        return false;
    }
    if *n == 2 || *n == 3 {
        return true;
    }
    if *n % 2 == 0 || *n % 3 == 0 {
        return false;
    }

    let mut i = 5;
    loop {
        if i * i > *n {
            return true;
        } else if *n % i == 0 || *n % (i + 2) == 0 {
            return false;
        } else {
            i += 6;
        }
    }
}
