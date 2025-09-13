pub enum InstructionFormats {
    /*
    CR-type | funct4 |   rd/rs1   |   rs2    | op |
            | 15-12  |    11-7    |   6-2    | 1-0|
    ------------------------------------------------
    */
    CR,
    /*
    CI-type | funct3 | imm |   rd/rs1   | imm | op |
            | 15-13  | 12  |    11-7    | 6-2 | 1-0|
    ------------------------------------------------
    */
    CI,
    /*
    CSS-type| funct3 |     imm     |   rs2    | op |
            | 15-13  |    12-7     |   6-2    | 1-0|
    ------------------------------------------------
    */
    CSS,
    /*
    CIW-type| funct3 |     imm      | rd' | op |
            | 15-13  |     12-5     | 4-2 | 1-0|
    ------------------------------------------
    */
    CIW,
    /*
    CL-type | funct3 | imm | rs1' | imm | rd' | op |
            | 15-13  |12-10| 9-7  | 6-5 | 4-2 | 1-0|
    ------------------------------------------------
    */
    CL,
    /*
    CS-type | funct3 | imm | rs1' | imm | rs2'| op |
            | 15-13  |12-10| 9-7  | 6-5 | 4-2 | 1-0|
    ------------------------------------------------
    */
    CS,
    /*
    CA-type | funct6 | rd'/rs1' | funct2 | rs2'| op |
            | 15-10  |   9-7    |  6-5   | 4-2 | 1-0|
    ------------------------------------------------
    */
    CA,
    /*
    CB-type | funct3 | off | rs1' |    offset    | op |
            | 15-13  | 12  | 9-7  |   6-2        | 1-0|
    ------------------------------------------------
    */
    CB,
    /*
    CJ-type | funct3 |        jump target        | op |
            | 15-13  |         12-2              | 1-0|
    ------------------------------------------------
    */
    CJ,
}

pub enum Opcode {
    // - Quadrant 0 (00): Stack-pointer based loads/stores, wide immediates
    Quadrant0,
    // - Quadrant 1 (01): Control transfers, integer constants and computations
    Quadrant1,
    // - Quadrant 2 (10): Stack-pointer based operations, register moves
    Quadrant2,
    // - Quadrant 3 (11): Reserved for 32-bit instructions
}

impl Opcode {
    pub fn from_bits(bits: u8) -> Option<Opcode> {
        match bits {
            0b00 => Some(Opcode::Quadrant0),
            0b01 => Some(Opcode::Quadrant1),
            0b10 => Some(Opcode::Quadrant2),
            _ => None,
        }
    }
}
