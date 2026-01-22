crate::hints::macros::define_hint! {
    bn254_g1_add => {
        hint_id: 0x0200,
        params: (p1: 64, p2: 64),
    }
}

crate::hints::macros::define_hint! {
    bn254_g1_mul => {
        hint_id: 0x0201,
        params: (point: 64, scalar: 32),
    }
}