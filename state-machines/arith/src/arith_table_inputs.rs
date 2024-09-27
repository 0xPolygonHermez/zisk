use std::ops::Add;

const ARITH_TABLE_SIZE: usize = 36;
pub struct ArithTableInputs<F> {
    multiplicity: [u32; ARITH_TABLE_SIZE],
    _phantom: std::marker::PhantomData<F>,
}

impl<F> Add for ArithTableInputs<F> {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        let mut result = Self::new();
        for i in 0..ARITH_TABLE_SIZE {
            result.multiplicity[i] = self.multiplicity[i] + other.multiplicity[i];
        }
        result
    }
}

impl<F> ArithTableInputs<F> {
    const FLAGS_AND_RANGES: [u32; ARITH_TABLE_SIZE] = [
        0x000000, 0x000000, 0x010000, 0x020024, 0x050000, 0x0A0024, 0x050028, 0x0A002C, 0x050000,
        0x0A0024, 0x050028, 0x0A002C, 0x001501, 0x002665, 0x001929, 0x002A6D, 0x000002, 0x000002,
        0x550002, 0xAA0036, 0xA5002A, 0xAA003E, 0x550002, 0xAA0036, 0xA5002A, 0xAA003E, 0x009003,
        0x009003, 0x009503, 0x0066F7, 0x00692B, 0x006AFF, 0x009503, 0x0066F7, 0x00692B, 0x006AFF,
    ];
    pub fn new() -> Self {
        Self { multiplicity: [0; ARITH_TABLE_SIZE], _phantom: std::marker::PhantomData }
    }
    pub fn clear(&mut self) {
        self.multiplicity = [0; ARITH_TABLE_SIZE];
    }
    pub fn push(
        &mut self,
        op: u8,
        m32: u32,
        div: u32,
        na: u32,
        nb: u32,
        nr: u32,
        np: u32,
        na32: u32,
        nd32: u32,
        range_a1: u32,
        range_b1: u32,
        range_c1: u32,
        range_d1: u32,
        range_a3: u32,
        range_b3: u32,
        range_c3: u32,
        range_d3: u32,
    ) {
        // TODO: in debug mode
        let flags = Self::values_to_flags(
            m32, div, na, nb, nr, np, na32, nd32, range_a1, range_b1, range_c1, range_d1, range_a3,
            range_b3, range_c3, range_d3,
        );
        let variants = Self::get_variants(op);
        let row_offset = nb * 2 + nb;
        let row: usize = Self::get_row(op, na, nb);

        assert!(row_offset < variants);
        assert!(Self::FLAGS_AND_RANGES[row] == flags);

        self.multiplicity[row] += 1;
    }
    fn get_variants(op: u8) -> u32 {
        match op {
            0xb0 | 0xb1 | 0xb8 | 0xb9 | 0xbc | 0xbd => 1, // mulu|muluh|divu|remu|divu_w|remu_w
            0xb3 => 2,                                    // mulsuh
            0xb4 | 0xb5 | 0xb6 | 0xba | 0xbb | 0xbe | 0xbf => 4, /* mul|mulh|mul_w|div|rem|div_w|rem_w */
            _ => panic!("Invalid opcode"),
        }
    }
    fn get_offset(op: u8) -> u32 {
        match op {
            0xb0 => 0,  // mulu
            0xb1 => 1,  // muluh
            0xb3 => 2,  // mulsuh
            0xb4 => 4,  // mul
            0xb5 => 8,  // mulh
            0xb6 => 12, // mul_w
            0xb8 => 16, // divu
            0xb9 => 17, // remu
            0xba => 18, // div
            0xbb => 22, // rem
            0xbc => 26, // divu_w
            0xbd => 27, // remu_w
            0xbe => 28, // div_w
            0xbf => 32, // rem_w
            _ => panic!("Invalid opcode"),
        }
    }
    fn get_row(op: u8, na: u32, nb: u32) -> usize {
        usize::try_from(Self::get_offset(op) + na + 2 * nb).unwrap() % ARITH_TABLE_SIZE
    }
    pub fn fast_push(&mut self, op: u8, na: u32, nb: u32) {
        self.multiplicity[Self::get_row(op, na, nb)] += 1;
    }
    fn values_to_flags(
        m32: u32,
        div: u32,
        na: u32,
        nb: u32,
        nr: u32,
        np: u32,
        na32: u32,
        nd32: u32,
        range_a1: u32,
        range_b1: u32,
        range_c1: u32,
        range_d1: u32,
        range_a3: u32,
        range_b3: u32,
        range_c3: u32,
        range_d3: u32,
    ) -> u32 {
        m32 + 0x000002 * div
            + 0x000004 * na
            + 0x000008 * nb
            + 0x000010 * nr
            + 0x000020 * np
            + 0x000040 * na32
            + 0x000080 * nd32
            + 0x000100 * range_a1
            + 0x000400 * range_b1
            + 0x001000 * range_c1
            + 0x004000 * range_d1
            + 0x010000 * range_a3
            + 0x040000 * range_b3
            + 0x100000 * range_c3
            + 0x400000 * range_d3
    }
}
