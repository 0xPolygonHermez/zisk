//! Real proofs, as `cargo-zisk prove` saves them (ZisK's flat layout).
//!
//! The fixtures are tied to the committed verifiers and keys: re-prove them and update the
//! table below whenever the setup changes.

use zisk_verifier::{
    decode_saved, vadcop_vk, verify, verify_saved_words, VadcopFinalProof, VerifyError,
};

struct Fixture {
    name: &'static str,
    bytes: &'static [u8],
    hash: &'static str,
    compressed: bool,
    program_vk: [u64; 4],
    /// Leading output words.
    outputs: &'static [u32],
}

/// `fib_mod` with `n = 100`, `module = 233`: commits the bytes `100, 233, fib(100) % 233 = 2`.
/// The program key is the ROM root, hashed with the proof's own family: one per family.
const FIB_MOD_VK_POSEIDON1: [u64; 4] =
    [849280929739825263, 11332809024285158711, 14612506331956425990, 8500145639271242349];
const FIB_MOD_VK_POSEIDON2: [u64; 4] =
    [1235102246277954017, 14605898302766356739, 13657465950397323756, 6719762526413802671];
const FIB_MOD_VK_BLAKE3: [u64; 4] =
    [17885856439396720575, 14851487842048450457, 5252325657547320468, 9487799266351193817];
const FIB_MOD_OUTPUTS: &[u32] = &[100 | 233 << 8 | 2 << 16, 0];

const FIXTURES: &[Fixture] = &[
    Fixture {
        name: "poseidon1 compressed",
        bytes: include_bytes!("fixtures/poseidon1_compressed.bin"),
        hash: "Poseidon1",
        compressed: true,
        program_vk: FIB_MOD_VK_POSEIDON1,
        outputs: FIB_MOD_OUTPUTS,
    },
    Fixture {
        name: "poseidon1 leaf",
        bytes: include_bytes!("fixtures/poseidon1_leaf.bin"),
        hash: "Poseidon1",
        compressed: false,
        program_vk: FIB_MOD_VK_POSEIDON1,
        outputs: FIB_MOD_OUTPUTS,
    },
    Fixture {
        name: "poseidon2 compressed",
        bytes: include_bytes!("fixtures/poseidon2_compressed.bin"),
        hash: "Poseidon2",
        compressed: true,
        program_vk: FIB_MOD_VK_POSEIDON2,
        outputs: FIB_MOD_OUTPUTS,
    },
    Fixture {
        name: "poseidon2 leaf",
        bytes: include_bytes!("fixtures/poseidon2_leaf.bin"),
        hash: "Poseidon2",
        compressed: false,
        program_vk: FIB_MOD_VK_POSEIDON2,
        outputs: FIB_MOD_OUTPUTS,
    },
    Fixture {
        name: "blake3 leaf",
        bytes: include_bytes!("fixtures/blake3_leaf.bin"),
        hash: "blake3",
        compressed: false,
        program_vk: FIB_MOD_VK_BLAKE3,
        outputs: FIB_MOD_OUTPUTS,
    },
];

impl Fixture {
    fn proof(&self) -> VadcopFinalProof {
        decode_saved(self.bytes).unwrap().proof
    }

    /// The crate's published key for the fixture's stage.
    fn key(&self) -> [u64; 4] {
        vadcop_vk(self.hash, self.compressed)
            .unwrap_or_else(|| panic!("{}: key not published", self.name))
    }
}

#[test]
fn every_fixture_decodes_to_its_stage() {
    for f in FIXTURES {
        let saved = decode_saved(f.bytes).unwrap();
        assert_eq!(saved.proof.hash, f.hash, "{}", f.name);
        assert_eq!(saved.proof.compressed, f.compressed, "{}", f.name);
        assert_eq!(saved.claimed_vadcop_vk, f.key(), "{}", f.name);
    }
}

/// Each fixture verifies under the crate's constant for its stage: this is what pins a key
/// to its verifier.
#[test]
fn every_fixture_verifies_and_reports_what_it_proves() {
    for f in FIXTURES {
        let v = verify(&f.proof(), &f.key()).unwrap_or_else(|e| panic!("{}: {e}", f.name));
        assert_eq!(v.program_vk, f.program_vk, "{}", f.name);
        assert_eq!(v.is_aggregate, if f.compressed { None } else { Some(false) }, "{}", f.name);
        let outputs = v.outputs_u32().expect("a leaf's outputs fit in u32");
        assert_eq!(&outputs[..f.outputs.len()], f.outputs, "{}", f.name);
    }
}

#[test]
fn a_wrong_key_or_a_flipped_word_is_refused() {
    for f in FIXTURES {
        let mut key = f.key();
        key[0] ^= 1;
        assert_eq!(verify(&f.proof(), &key).unwrap_err(), VerifyError::InvalidProof, "{}", f.name);

        let mut p = f.proof();
        let mid = p.proof.len() / 2;
        p.proof[mid] ^= 1;
        assert_eq!(verify(&p, &f.key()).unwrap_err(), VerifyError::InvalidProof, "{}", f.name);

        let mut p = f.proof();
        p.public_values[5] ^= 1;
        assert_eq!(verify(&p, &f.key()).unwrap_err(), VerifyError::InvalidProof, "{}", f.name);
    }
}

/// Each published key opens only its own family and stage: a swapped or duplicated
/// constant fails here.
#[test]
fn no_fixture_verifies_under_another_published_key() {
    let stages = [
        ("blake3", false),
        ("Poseidon1", false),
        ("Poseidon1", true),
        ("Poseidon2", false),
        ("Poseidon2", true),
    ];
    for f in FIXTURES {
        for &(hash, compressed) in &stages {
            let key = vadcop_vk(hash, compressed).unwrap();
            if key == f.key() {
                continue;
            }
            assert_eq!(
                verify(&f.proof(), &key).unwrap_err(),
                VerifyError::InvalidProof,
                "{} under {hash} compressed={compressed}",
                f.name
            );
        }
    }
}

/// The in-place path a guest takes reaches the same verdict as decode-then-verify.
#[test]
fn verifying_in_place_matches_decode_then_verify() {
    for f in FIXTURES {
        let words: Vec<u64> =
            f.bytes.chunks_exact(8).map(|c| u64::from_le_bytes(c.try_into().unwrap())).collect();
        assert_eq!(
            verify_saved_words(&words, &f.key(), None),
            verify(&f.proof(), &f.key()),
            "{}",
            f.name
        );

        let mut flipped = words.clone();
        let mid = flipped.len() / 2;
        flipped[mid] ^= 1;
        assert_eq!(
            verify_saved_words(&flipped, &f.key(), None).unwrap_err(),
            VerifyError::InvalidProof,
            "{}",
            f.name
        );
        assert!(matches!(
            verify_saved_words(&words[..words.len() - 1], &f.key(), None).unwrap_err(),
            VerifyError::Malformed(_)
        ));
    }
}

/// A guest pinning the program refuses another program's proof before the STARK, and
/// accepts its own.
#[test]
fn verifying_in_place_pins_the_program() {
    for f in FIXTURES {
        let words: Vec<u64> =
            f.bytes.chunks_exact(8).map(|c| u64::from_le_bytes(c.try_into().unwrap())).collect();
        assert!(verify_saved_words(&words, &f.key(), Some(&f.program_vk)).is_ok(), "{}", f.name);

        let mut other = f.program_vk;
        other[0] ^= 1;
        assert_eq!(
            verify_saved_words(&words, &f.key(), Some(&other)).unwrap_err(),
            VerifyError::ProgramMismatch { committed: f.program_vk },
            "{}",
            f.name
        );
    }
}
