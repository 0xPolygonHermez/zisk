//! Strongly-typed RISC-V instruction format (`RiscvInstType`) enum.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RiscvInstType {
    A,
    B,
    C,
    Ca,
    Cb,
    Ci,
    Cinvalid,
    Ciw,
    Cj,
    Cl,
    Cr,
    Cs,
    Css,
    F,
    I,
    #[default]
    Invalid,
    J,
    R,
    R4,
    S,
    U,
}

impl RiscvInstType {
    /// Returns the canonical RISC-V mnemonic string for this variant.
    pub fn as_str(&self) -> &'static str {
        match self {
            RiscvInstType::A => "A",
            RiscvInstType::B => "B",
            RiscvInstType::C => "C",
            RiscvInstType::Ca => "CA",
            RiscvInstType::Cb => "CB",
            RiscvInstType::Ci => "CI",
            RiscvInstType::Cinvalid => "CINVALID",
            RiscvInstType::Ciw => "CIW",
            RiscvInstType::Cj => "CJ",
            RiscvInstType::Cl => "CL",
            RiscvInstType::Cr => "CR",
            RiscvInstType::Cs => "CS",
            RiscvInstType::Css => "CSS",
            RiscvInstType::F => "F",
            RiscvInstType::I => "I",
            RiscvInstType::Invalid => "INVALID",
            RiscvInstType::J => "J",
            RiscvInstType::R => "R",
            RiscvInstType::R4 => "R4",
            RiscvInstType::S => "S",
            RiscvInstType::U => "U",
        }
    }
}

impl core::fmt::Display for RiscvInstType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}
