use proofman_common as common;
pub use proofman_macros::trace;

trace!(Arith32Row, Arith32Trace<F> { fake: F });
trace!(Arith64Row, Arith64Trace<F> { fake: F });
trace!(Arith3264Row, Arith3264Trace<F> { fake: F });
