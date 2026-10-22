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
    [16379923861911953426, 16352910408364517494, 10294560753330969230, 16923079741480914316];

pub const VADCOP_FINAL_COMPRESSED_VK_POSEIDON1: [u64; VADCOP_VK_LEN_WORDS] =
    [11702255945344880271, 9731968916266252191, 4914293114478674038, 1367108278841341304];

pub const PILFFLONK_VKEY_DIGEST_POSEIDON1: Option<&str> =
    Some("0xe8f7e04ba2ff5bedda661368eb890d4b70fa8c139cac772ef42cb30cac2f454b");

pub const VADCOP_FINAL_VK_POSEIDON2: [u64; VADCOP_VK_LEN_WORDS] =
    [1106121089452141517, 14399029462071009484, 15740669276159672011, 3272630857112546816];

pub const VADCOP_FINAL_COMPRESSED_VK_POSEIDON2: [u64; VADCOP_VK_LEN_WORDS] =
    [3701038814197501879, 14507965817467464059, 2235769493965136114, 15969808776192813664];

pub const PILFFLONK_VKEY_DIGEST_POSEIDON2: Option<&str> =
    Some("0xe761f9f2dafc828573770c6ab829d0fa97e858133aaf46bed6ee6096f9fb569e");

pub const VADCOP_FINAL_VK_BLAKE3: [u64; VADCOP_VK_LEN_WORDS] =
    [14783525120172297970, 4088576520197439045, 2474267324338179562, 18034754851431410334];

pub const VADCOP_FINAL_COMPRESSED_VK_BLAKE3: [u64; VADCOP_VK_LEN_WORDS] =
    [6598209204420492057, 16583075846798299704, 12407543501387025518, 9867045755372685837];

pub const PILFFLONK_VKEY_DIGEST_BLAKE3: Option<&str> =
    Some("0xe9eb2c78fc2cff7c38c459927e3afd00189a06af869a597c8109240ab0aedd3a");
