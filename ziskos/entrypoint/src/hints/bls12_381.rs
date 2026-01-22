crate::hints::macros::define_hint! {
    bls12_381_g1_add => {
        hint_id: 0x0400,
        params: (ret: 96),
    }
}

crate::hints::macros::define_hint! {
    bls12_381_g2_add => {
        hint_id: 0x0405,
        params: (ret: 192),
    }
}
