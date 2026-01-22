crate::hints::macros::define_hint! {
    bn254_g1_add => {
        hint_id: 0x0200,
        params: (result: 64),
    }
}

crate::hints::macros::define_hint! {
    bn254_g1_mul => {
        hint_id: 0x0201,
        params: (result: 64),
    }
}