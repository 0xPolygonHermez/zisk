use crate::{ensure, root, Result};
use serde_json::Value;
use std::fs;

fn seeds(fixture: &Value) -> Result<[(&'static str, &'static str, Vec<u8>); 2]> {
    let mut proof = Vec::new();
    for (key, length) in [("programVK", 32), ("publicValues", 512), ("proofBytes", 6752)] {
        let value = fixture[key].as_str().ok_or_else(|| format!("missing fixture field {key}"))?;
        let bytes = hex::decode(value.strip_prefix("0x").unwrap_or(value))?;
        ensure(bytes.len() == length, &format!("invalid fixture length: {key}"))?;
        proof.extend(bytes);
    }
    let mut sequence = vec![[0, 0, 0, 0, 0, 0]];
    sequence.extend([[1, 0, 0, 0, 7, 0]; 7]);
    sequence.extend([
        [1, 0, 0, 1, 7, 0],
        [2, 0, 0, 0, 0, 0],
        [3, 0, 0, 0, 0, 0],
        [5, 0, 0, 0, 0, 0],
        [4, 0, 0, 0, 0, 0],
        [0, 16, 0, 0, 0, 0],
        [6, 0, 0, 0, 0, 0],
    ]);
    Ok([
        ("verify", "release-proof", proof),
        ("buffer_state", "lifecycle", sequence.into_iter().flatten().collect()),
    ])
}

pub fn run() -> Result<()> {
    let fixture = serde_json::from_slice(&fs::read(root().join("fixtures/blake3.json"))?)?;
    for (target, name, bytes) in seeds(&fixture)? {
        let directory = root().join("fuzz/corpus").join(target);
        fs::create_dir_all(&directory)?;
        fs::write(directory.join(name), bytes)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    #[test]
    fn preserves_public_seed_bytes() {
        let fixture: Value =
            serde_json::from_slice(&fs::read(root().join("fixtures/blake3.json")).unwrap())
                .unwrap();
        let actual = seeds(&fixture).unwrap();
        for ((_, _, bytes), (size, hash)) in actual.iter().zip([
            (7296, "ff230c92fa411068c6136dbd3cf49f95836f449034ac9168f56ff28ad33c3a30"),
            (90, "277e039515a3af1f0c159f797fa3cab41554e0f002332cff479e7c5c8906e3a5"),
        ]) {
            assert_eq!(bytes.len(), size);
            assert_eq!(hex::encode(Sha256::digest(bytes)), hash);
        }
        let mut invalid = fixture;
        invalid["programVK"] = "0x00".into();
        assert!(seeds(&invalid).is_err());
    }
}
