use lazy_static::lazy_static;
use num_bigint::BigUint;

lazy_static! {
    pub(crate) static ref P: BigUint = BigUint::parse_bytes(
        b"30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47",
        16
    )
    .unwrap();
}
