pub type OpResult = (u64, bool);

#[derive(Debug, Clone)]
pub enum MemOp {
    Read(u64),
    Write(u64, u64),
}

#[derive(Debug, Clone)]
pub enum MemUnalignedOp {
    Read(u64, usize),
    Write(u64, usize, u64),
}
