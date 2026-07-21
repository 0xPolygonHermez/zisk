use crate::hints::macros::define_hint_disabled;
use zisk_definitions::{
    HINT_ADD_MOD256, HINT_INV_MOD256, HINT_MULMOD256, HINT_POW_MOD256, HINT_REDUCE_MOD256,
    HINT_SQUARE_MOD256,
};

define_hint_disabled! {
    mulmod256 => {
        hint_id: HINT_MULMOD256,
        params: (a: 32, b: 32, m: 32),
        is_result: false,
    }
}

define_hint_disabled! {
    reduce_mod256 => {
        hint_id: HINT_REDUCE_MOD256,
        params: (a: 32, m: 32),
        is_result: false,
    }
}

define_hint_disabled! {
    add_mod256 => {
        hint_id: HINT_ADD_MOD256,
        params: (a: 32, b: 32, m: 32),
        is_result: false,
    }
}

define_hint_disabled! {
    square_mod256 => {
        hint_id: HINT_SQUARE_MOD256,
        params: (a: 32, m: 32),
        is_result: false,
    }
}

define_hint_disabled! {
    pow_mod256 => {
        hint_id: HINT_POW_MOD256,
        params: (base: 32, exp: 32, m: 32),
        is_result: false,
    }
}

define_hint_disabled! {
    inv_mod256 => {
        hint_id: HINT_INV_MOD256,
        params: (a: 32, m: 32),
        is_result: false,
    }
}
