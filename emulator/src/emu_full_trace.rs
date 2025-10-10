#[cfg(not(feature = "packed"))]
use zisk_pil::MainTraceRow;
#[cfg(feature = "packed")]
use zisk_pil::MainTraceRowPacked;

#[cfg(feature = "packed")]
pub type EmuFullTraceStep<F> = MainTraceRowPacked<F>;
#[cfg(not(feature = "packed"))]
pub type EmuFullTraceStep<F> = MainTraceRow<F>;
