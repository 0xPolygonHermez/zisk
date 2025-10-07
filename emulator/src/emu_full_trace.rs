#[cfg(not(feature = "gpu"))]
use zisk_pil::MainTraceRow;
#[cfg(feature = "gpu")]
use zisk_pil::MainTraceRowPacked;

#[cfg(feature = "gpu")]
pub type EmuFullTraceStep<F> = MainTraceRowPacked<F>;
#[cfg(not(feature = "gpu"))]
pub type EmuFullTraceStep<F> = MainTraceRow<F>;
