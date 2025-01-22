#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct PrecompileCode(u16);

impl PrecompileCode {
    pub fn new(value: u16) -> Self {
        PrecompileCode(value)
    }

    pub fn value(&self) -> u16 {
        self.0
    }
}

impl From<u16> for PrecompileCode {
    fn from(value: u16) -> Self {
        PrecompileCode::new(value)
    }
}

impl From<PrecompileCode> for u16 {
    fn from(code: PrecompileCode) -> Self {
        code.value()
    }
}

pub struct PrecompileContext {}

pub trait PrecompileCall: Send + Sync {
    fn execute(&self, opcode: PrecompileCode, ctx: &mut InstContext) -> Option<(u64, bool)>;
}
