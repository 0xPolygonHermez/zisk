//! Real proofs, as `cargo-zisk prove` saves them. `tools/release-artifacts/regen.sh --prove`
//! regenerates them with the verifiers and keys.

use zisk_verifier::{
    decode_saved, decode_saved_words, seed_trailer, vadcop_vk, verify, verify_saved,
    verify_saved_words, VadcopFinalProof, VerifyError,
};

fn words(bytes: &[u8]) -> Vec<u64> {
    bytes.chunks_exact(8).map(|c| u64::from_le_bytes(c.try_into().unwrap())).collect()
}

struct Fixture {
    name: &'static str,
    bytes: &'static [u8],
    hash: &'static str,
    compressed: bool,
    program_vk_json: &'static str,
    outputs: &'static [u32],
}

/// `fib_mod` with `n = 100`, `module = 233` commits `100, 233, fib(100) % 233 = 2`.
const FIB_MOD_OUTPUTS: &[u32] = &[100 | 233 << 8 | 2 << 16, 0];

macro_rules! fixture {
    ($name:literal, $stem:literal, $hash:literal, $stage:literal, $compressed:literal) => {
        Fixture {
            name: $name,
            bytes: include_bytes!(concat!("fixtures/", $stem, "_", $stage, ".bin")),
            hash: $hash,
            compressed: $compressed,
            program_vk_json: include_str!(concat!("fixtures/", $stem, "_program_vk.json")),
            outputs: FIB_MOD_OUTPUTS,
        }
    };
}

const FIXTURES: &[Fixture] = &[
    fixture!("poseidon1 compressed", "poseidon1", "Poseidon1", "compressed", true),
    fixture!("poseidon1 leaf", "poseidon1", "Poseidon1", "leaf", false),
    fixture!("poseidon2 compressed", "poseidon2", "Poseidon2", "compressed", true),
    fixture!("poseidon2 leaf", "poseidon2", "Poseidon2", "leaf", false),
    fixture!("blake3 compressed", "blake3", "blake3", "compressed", true),
    fixture!("blake3 leaf", "blake3", "blake3", "leaf", false),
];

impl Fixture {
    fn program_vk(&self) -> [u64; 4] {
        let limbs: Vec<u64> = self
            .program_vk_json
            .trim()
            .trim_start_matches('[')
            .trim_end_matches(']')
            .split(',')
            .map(|l| l.trim().parse().unwrap())
            .collect();
        limbs.try_into().unwrap()
    }

    fn proof(&self) -> VadcopFinalProof {
        decode_saved(self.bytes).unwrap().proof
    }

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

#[test]
fn every_fixture_verifies_and_reports_what_it_proves() {
    for f in FIXTURES {
        let v = verify(&f.proof(), &f.key()).unwrap_or_else(|e| panic!("{}: {e}", f.name));
        assert_eq!(v.program_vk, f.program_vk(), "{}", f.name);
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

#[test]
fn no_fixture_verifies_under_another_published_key() {
    let stages = [
        ("blake3", false),
        ("blake3", true),
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

#[test]
fn verifying_in_place_matches_decode_then_verify() {
    for f in FIXTURES {
        let words = words(f.bytes);
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

/// A guest refuses the trailer so that it sees one encoding per proof.
#[test]
fn a_seed_trailer_is_read_by_hosts_and_refused_by_guests() {
    for f in FIXTURES {
        let mut words = words(f.bytes);
        words.extend(seed_trailer([1, 2, 3, 4]));
        assert!(matches!(
            verify_saved_words(&words, &f.key(), None).unwrap_err(),
            VerifyError::Malformed(_)
        ));
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        assert_eq!(verify_saved(&bytes, None).unwrap().program_vk, f.program_vk(), "{}", f.name);
        let saved = decode_saved_words(&words).unwrap();
        assert_eq!(saved.random_seed, Some([1, 2, 3, 4]), "{}", f.name);
        assert_eq!(saved.proof.proof, f.proof().proof, "{}", f.name);
    }
}

#[test]
fn verifying_in_place_pins_the_program() {
    for f in FIXTURES {
        let words = words(f.bytes);
        assert!(verify_saved_words(&words, &f.key(), Some(&f.program_vk())).is_ok(), "{}", f.name);

        let mut other = f.program_vk();
        other[0] ^= 1;
        assert_eq!(
            verify_saved_words(&words, &f.key(), Some(&other)).unwrap_err(),
            VerifyError::ProgramMismatch { committed: f.program_vk() },
            "{}",
            f.name
        );
    }
}

#[test]
fn every_fixture_verifies_from_its_bytes() {
    for f in FIXTURES {
        let v = verify_saved(f.bytes, Some(&f.program_vk()))
            .unwrap_or_else(|e| panic!("{}: {e}", f.name));
        assert_eq!(v.program_vk, f.program_vk(), "{}", f.name);

        let mut other = f.program_vk();
        other[0] ^= 1;
        assert!(matches!(
            verify_saved(f.bytes, Some(&other)).unwrap_err(),
            VerifyError::ProgramMismatch { .. }
        ));
    }
    let mut tampered = FIXTURES[0].bytes.to_vec();
    let mid = tampered.len() / 2;
    tampered[mid] ^= 1;
    assert!(verify_saved(&tampered, None).is_err());
}
