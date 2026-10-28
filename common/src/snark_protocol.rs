/// The protocol of a final SNARK proof. A SNARK proving key is set up for one
/// (`setup-snark --final-snark`), so choosing it checks the key rather than changing the proof.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnarkProtocol {
    /// PLONK, of a `final.zkey`: a poseidon key only, set up by hand (`--final-snark plonk`); no
    /// release ships one, nor its Solidity verifier.
    Plonk,
    /// pilfflonk, of a `provingKey/`: the default of `setup-snark`, and the only one of a blake3
    /// key.
    Pilfflonk,
}

impl SnarkProtocol {
    /// The protocol's name in messages: `PLONK` or `pilfflonk`.
    pub fn name(self) -> &'static str {
        match self {
            SnarkProtocol::Plonk => "PLONK",
            SnarkProtocol::Pilfflonk => "pilfflonk",
        }
    }
}
