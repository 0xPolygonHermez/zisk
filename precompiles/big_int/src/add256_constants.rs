use zisk_common::OPERATION_BUS_DATA_SIZE;

pub const PARAMS: usize = 4;
pub const READ_PARAMS: usize = 2;
pub const DIRECT_READ_PARAMS: usize = 1;
pub const WRITE_PARAMS: usize = 1;
pub const RESULT_PARAMS: usize = 1;
pub const PARAM_CHUNKS: usize = 4;
pub const START_READ_PARAMS: usize = OPERATION_BUS_DATA_SIZE + PARAMS;
pub const START_WRITE_PARAMS: usize =
    START_READ_PARAMS + READ_PARAMS * PARAM_CHUNKS + RESULT_PARAMS;
pub const WRITE_ADDR_PARAM: usize = READ_PARAMS + DIRECT_READ_PARAMS;
