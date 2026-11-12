// Example configuration only: pin the actual deployment ID and application guest key before deployment.
use solana_program::pubkey::Pubkey;
pub const VERIFIER_ID: Pubkey = Pubkey::new_from_array([41; 32]);
pub const GUEST_KEY: [u8; 32] = [
    237, 57, 164, 191, 55, 250, 222, 165, 173, 39, 253, 93, 53, 208, 68, 17, 204, 193, 174, 184,
    82, 167, 161, 120, 99, 77, 65, 231, 242, 83, 187, 11,
];
