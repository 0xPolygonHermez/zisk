// Written by `cargo-zisk-dev release-artifacts render` (tools/release-artifacts/regen.sh).

//! The keys of the release setup the verifiers in this crate were generated from.
//!
//! `VADCOP_FINAL_*`: the `<stage>.verkey.json` of each family's proving key. A key is the root of
//! a stage's constant tree. It is shared by every ZisK program and identifies the prover, not the
//! program: check `Verified::program_vk` for that. It covers leaf proofs only; a recurser proof
//! verifies under its recurser's own key.
//!
//! `PILFFLONK_VKEY_DIGEST_*`: the `digest` of each family's `pilfflonk.vkey.json`, which pins the
//! key (see [`pilfflonk_vkey_digest`](crate::pilfflonk_vkey_digest)).
//!
//! Regenerate with `tools/release-artifacts/regen.sh`, together with the verifiers.

use crate::VADCOP_VK_LEN_WORDS;

pub const VADCOP_FINAL_VK_POSEIDON1: [u64; VADCOP_VK_LEN_WORDS] =
    [16807801129105003610, 1480129380269065486, 6842518844123700219, 2605612804213468774];

pub const VADCOP_FINAL_COMPRESSED_VK_POSEIDON1: [u64; VADCOP_VK_LEN_WORDS] =
    [3936316867833434652, 13002114981730899170, 17743900043057412555, 17794984613572371221];

pub const PILFFLONK_VKEY_DIGEST_POSEIDON1: Option<&str> =
    Some("0xe8f7e04ba2ff5bedda661368eb890d4b70fa8c139cac772ef42cb30cac2f454b");

pub const VADCOP_FINAL_VK_POSEIDON2: [u64; VADCOP_VK_LEN_WORDS] =
    [7235627588029747190, 4964294244843319518, 15944021041411876293, 9165814394267558940];

pub const VADCOP_FINAL_COMPRESSED_VK_POSEIDON2: [u64; VADCOP_VK_LEN_WORDS] =
    [4751769779349204665, 8025699667731212605, 10343005230049885991, 17515878728116835452];

pub const PILFFLONK_VKEY_DIGEST_POSEIDON2: Option<&str> =
    Some("0xe761f9f2dafc828573770c6ab829d0fa97e858133aaf46bed6ee6096f9fb569e");

pub const VADCOP_FINAL_VK_BLAKE3: [u64; VADCOP_VK_LEN_WORDS] =
    [17146226293136250724, 16833344589786796689, 10321050178608571017, 2095991909911874349];

pub const VADCOP_FINAL_COMPRESSED_VK_BLAKE3: [u64; VADCOP_VK_LEN_WORDS] =
    [12858410571421223333, 7366065819410512846, 4168917461533729924, 17247924614817440033];

pub const PILFFLONK_VKEY_DIGEST_BLAKE3: Option<&str> =
    Some("0xe9eb2c78fc2cff7c38c459927e3afd00189a06af869a597c8109240ab0aedd3a");
