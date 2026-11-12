//! Fixture-only projection of the pinned release's common/src/proof.rs Serde schema.
//! The fixed input hash prevents this from becoming a second general proof decoder.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Serialize, Deserialize)]
enum Body {
    UnsupportedVadcop,
    UnsupportedPlonk,
    Pilfflonk {
        proof_bytes: Vec<u8>,
        vkey_json: String,
        publics: Vec<u8>,
        publics_full: Vec<u64>,
        rootc: Vec<u64>,
    },
}
#[derive(Serialize, Deserialize)]
enum HashMode {
    Poseidon1,
    Poseidon2,
    Blake3,
}
#[derive(Serialize, Deserialize)]
struct ProgramVk {
    vk: Vec<u64>,
    hash_mode: HashMode,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = std::fs::read("../zisk-contracts/test/blake3_pilfflonk.bin")?;
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "002e1c53bd52a05f733835d10d12b7506e215e33e9be08e4d5c210245596ad8b"
    );
    assert_eq!(&bytes[..8], b"ZISKPLNK");
    let config = bincode::config::standard().with_limit::<{ 16 << 20 }>();
    let (decoded, read): ((Body, ProgramVk), usize) =
        bincode::serde::decode_from_slice(&bytes[8..], config)?;
    // The release's from_bincode also decodes (ProofBody, ProgramVK) before its optional seed.
    assert_eq!(bincode::serde::encode_to_vec(&decoded, config)?, &bytes[8..8 + read]);
    let (Body::Pilfflonk { proof_bytes, vkey_json, publics: _, publics_full, rootc }, guest) =
        decoded
    else {
        panic!("wrong proof kind")
    };
    assert!(matches!(guest.hash_mode, HashMode::Blake3));
    assert_eq!(publics_full.len(), 68);
    assert_eq!(guest.vk, publics_full[..4]);
    assert_eq!(
        rootc,
        [17146226293136250724, 16833344589786796689, 10321050178608571017, 2095991909911874349]
    );
    assert!(publics_full.iter().chain(rootc.iter()).all(|x| *x < 0xffff_ffff_0000_0001));
    let vkey: serde_json::Value = serde_json::from_str(&vkey_json)?;
    assert_eq!(
        vkey["digest"],
        "0xe9eb2c78fc2cff7c38c459927e3afd00189a06af869a597c8109240ab0aedd3a"
    );
    // This pinned everyRow profile needs no auxiliary calldata inverses.
    assert_eq!(proof_bytes.len(), 6752);
    let be = |xs: &[u64]| xs.iter().flat_map(|x| x.to_be_bytes()).collect::<Vec<_>>();
    let outputs = publics_full[4..].iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<_>>();
    let fixture = serde_json::json!({
        "programVK": format!("0x{}", hex::encode(be(&guest.vk))),
        "rootCVadcopFinal": format!("0x{}", hex::encode(be(&rootc))),
        "publicValues": format!("0x{}", hex::encode(outputs)),
        "proofBytes": format!("0x{}", hex::encode(proof_bytes))
    });
    std::fs::create_dir_all("fixtures")?;
    std::fs::write("fixtures/blake3.json", serde_json::to_string_pretty(&fixture)? + "\n")?;
    std::fs::write("fixtures/vkey.json", vkey_json + "\n")?;
    println!("Extracted the hash-pinned release fixture; Serde prefix roundtrip is byte-identical ({read} bytes).");
    Ok(())
}
