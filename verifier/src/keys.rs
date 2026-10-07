//! The vadcop_final verification keys of the setup the verifiers in this crate were
//! generated from: the `<stage>.verkey.json` of each family's proving key. blake3 builds
//! no compressed stage, so it has one key.
//!
//! A key is the root of a stage's constant tree. It is shared by every ZisK program and
//! identifies the prover, not the program: check `Verified::program_vk` for that. It
//! covers leaf proofs only; a recurser proof verifies under its recurser's own key.
//!
//! Regenerate together with the verifiers whenever the setup changes.

use crate::PROGRAM_VK_LEN;

pub const VADCOP_FINAL_VK_POSEIDON1: [u64; PROGRAM_VK_LEN] =
    [7720455071669353216, 17350242473202279177, 12167441154672932695, 6676005994927600229];

pub const VADCOP_FINAL_COMPRESSED_VK_POSEIDON1: [u64; PROGRAM_VK_LEN] =
    [11460674294544897374, 2320585223040589506, 14134567294518986765, 13867062699228975549];

pub const VADCOP_FINAL_VK_POSEIDON2: [u64; PROGRAM_VK_LEN] =
    [6096219998437121679, 2254708086484257796, 6364632100581660156, 17097783617149698521];

pub const VADCOP_FINAL_COMPRESSED_VK_POSEIDON2: [u64; PROGRAM_VK_LEN] =
    [14842246717591143122, 11392347458397675102, 10173955383267062289, 14893614764771371224];

pub const VADCOP_FINAL_VK_BLAKE3: [u64; PROGRAM_VK_LEN] =
    [3213505036805938264, 17028655392078661840, 5593895546430464126, 13641597543864939016];
