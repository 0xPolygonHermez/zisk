crate::hints::macros::define_hint! {
    bls12_381_g1_add => {
        hint_id: 0x0400,
        params: (a: 96, b: 96),
    }
}

crate::hints::macros::define_hint! {
    bls12_381_g2_add => {
        hint_id: 0x0405,
        params: (a: 192, b: 192),
    }
}
