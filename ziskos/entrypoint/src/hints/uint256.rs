use crate::hints::macros::define_hint;
use zisk_common::{
    HINT_ADD_MOD256, HINT_INV_MOD256, HINT_MULMOD256, HINT_POW_MOD256, HINT_REDUCE_MOD256,
    HINT_SQUARE_MOD256,
};

define_hint! {
    mulmod256 => {
        hint_id: HINT_MULMOD256,
        params: (a: 32, b: 32, m: 32),
        is_result: false,
    }
}

define_hint! {
    reduce_mod256 => {
        hint_id: HINT_REDUCE_MOD256,
        params: (a: 32, m: 32),
        is_result: false,
    }
}

define_hint! {
    add_mod256 => {
        hint_id: HINT_ADD_MOD256,
        params: (a: 32, b: 32, m: 32),
        is_result: false,
    }
}

define_hint! {
    square_mod256 => {
        hint_id: HINT_SQUARE_MOD256,
        params: (a: 32, m: 32),
        is_result: false,
    }
}

define_hint! {
    pow_mod256 => {
        hint_id: HINT_POW_MOD256,
        params: (base: 32, exp: 32, m: 32),
        is_result: false,
    }
}

define_hint! {
    inv_mod256 => {
        hint_id: HINT_INV_MOD256,
        params: (a: 32, m: 32),
        is_result: false,
    }
}
