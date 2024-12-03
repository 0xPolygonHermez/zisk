// use std::sync::Arc;

// use p3_field::PrimeField;
// use sm_common::{Expander, InstanceExpanderCtx};
// use zisk_common::InstObserver;
// use zisk_core::{InstContext, ZiskInst};
// use ziskemu::EmuTrace;

// pub struct BinaryExpander<F: PrimeField> {
//     _phantom: std::marker::PhantomData<F>,
// }

// impl<F: PrimeField> BinaryExpander<F> {
//     pub fn new(iectx: &mut InstanceExpanderCtx<F>) -> Self {
//         BinaryExpander {
//             _phantom: std::marker::PhantomData,
//         }
//     }
// }
// impl<F: PrimeField> Expander<F> for BinaryExpander<F> {
//     fn expand(
//         &self,
//         iectx: &mut InstanceExpanderCtx<F>,
//         min_traces: Arc<Vec<EmuTrace>>,
//     ) -> Result<(), Box<dyn std::error::Error + Send>> {
//         println!("Expanding Binary");
//         Ok(())
//     }
// }

// impl<F: PrimeField> InstObserver for BinaryExpander<F> {
//     #[inline(always)]
//     fn on_instruction(&mut self, zisk_inst: &ZiskInst, inst_ctx: &InstContext) -> bool {
//         false
//     }
// }
