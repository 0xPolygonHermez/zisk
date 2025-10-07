#[cfg(any(not(feature = "gpu"), not(feature = "packed")))]
use zisk_pil::MainTraceRow;
#[cfg(all(feature = "gpu", feature = "packed"))]
use zisk_pil::MainTraceRowPacked;

#[cfg(all(feature = "gpu", feature = "packed"))]
pub type EmuFullTraceStep<F> = MainTraceRowPacked<F>;
#[cfg(any(not(feature = "gpu"), not(feature = "packed")))]
pub type EmuFullTraceStep<F> = MainTraceRow<F>;
