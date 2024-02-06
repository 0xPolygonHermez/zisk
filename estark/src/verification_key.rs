use serde::{Deserialize, Serialize};
use goldilocks::{AbstractField, Goldilocks};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationKey<T> {
    #[serde(rename = "constRoot")]
    pub const_root: Vec<T>,
}

impl VerificationKey<Goldilocks> {
    pub fn from_json(filename: &str) -> VerificationKey<Goldilocks> {
        let json = std::fs::read_to_string(filename).expect(format!("Failed to read file {}", filename).as_str());
        let vk_json: VerificationKey<u64> = serde_json::from_str(&json).expect("Failed to parse JSON");

        VerificationKey { const_root: vk_json.const_root.iter().map(|x| Goldilocks::from_canonical_u64(*x)).collect() }
    }
}

#[cfg(test)]
mod tests {
    use goldilocks::Goldilocks;
    use super::*;

    #[test]
    fn test_parse_json() {
        let json_str = r#"
        {
            "constRoot": [
                14558785918046351557,
                4293137164016028685,
                4657202971016248147,
                8414627042162268474
            ]
        }
    "#;

        let expected = VerificationKey {
            const_root: vec![
                Goldilocks::from_canonical_u64(14558785918046351557),
                Goldilocks::from_canonical_u64(4293137164016028685),
                Goldilocks::from_canonical_u64(4657202971016248147),
                Goldilocks::from_canonical_u64(8414627042162268474),
            ],
        };

        let result = VerificationKey::<Goldilocks>::from_json(json_str);

        assert_eq!(expected.const_root[0], result.const_root[0]);
        assert_eq!(expected.const_root[1], result.const_root[1]);
        assert_eq!(expected.const_root[2], result.const_root[2]);
        assert_eq!(expected.const_root[3], result.const_root[3]);
    }

    #[test]
    #[should_panic]
    fn test_parse_json_invalid() {
        // Test with invalid JSON
        let invalid_json_str = r#"
        {
            "constRoot": [
                "invalid_value"
            ]
        }
    "#;

        VerificationKey::<Goldilocks>::from_json(invalid_json_str);
    }
}
