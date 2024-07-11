use p3_goldilocks::Goldilocks;
use p3_field::AbstractField;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationKey<T> {
    #[serde(rename = "constRoot")]
    pub const_root: Vec<T>,
}

impl VerificationKey<Goldilocks> {
    pub fn from_json(verkey_json: &str) -> VerificationKey<Goldilocks> {
        let verkey: VerificationKey<u64> = serde_json::from_str(&verkey_json).expect("Failed to parse JSON");

        VerificationKey { const_root: verkey.const_root.iter().map(|x| Goldilocks::from_canonical_u64(*x)).collect() }
    }
}

#[cfg(test)]
mod tests {
    use p3_goldilocks::Goldilocks;
    use p3_field::AbstractField;

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
