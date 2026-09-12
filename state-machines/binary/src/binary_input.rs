use zisk_common::{ExtOperationData, OperationBusData};

pub struct BinaryInput {
    pub op: u8,
    pub a: u64,
    pub b: u64,
}

impl BinaryInput {
    #[allow(dead_code)]
    pub fn new(op: u8, a: u64, b: u64) -> Self {
        Self { op, a, b }
    }

    pub fn from(data: &ExtOperationData<u64>) -> Self {
        Self {
            op: OperationBusData::get_op(data),
            a: OperationBusData::get_a(data),
            b: OperationBusData::get_b(data),
        }
    }
}

/// Measures a witness computation from the point it is created to the end of its scope, and reports
/// the operations it covered.
///
/// One instance's fill is what a worker pays in one indivisible piece, so this is the figure that
/// decides how wide an air's packing is worth making: splitting an air into more instances trades
/// this against one aggregation proof each. Comparing the two needs the number, not an estimate,
/// which is why it is logged in production rather than measured in a benchmark.
pub struct FillReport {
    pub name: &'static str,
    pub inputs: usize,
    pub started: std::time::Instant,
}

impl Drop for FillReport {
    fn drop(&mut self) {
        tracing::info!(
            "{} inputs: {} | fill {:.0}ms",
            self.name,
            self.inputs,
            self.started.elapsed().as_secs_f64() * 1e3,
        );
    }
}
