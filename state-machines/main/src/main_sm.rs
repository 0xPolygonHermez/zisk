use log::{debug, info};
use rayon::{Scope, ThreadPoolBuilder};
use sm_arith::ArithSM;
use sm_common::{EmuTrace, MockEmulator};
use std::{collections::HashMap, sync::Arc, thread};

use common::{AirInstance, ExecutionCtx, ProofCtx};
use proofman::WCManager;
use sm_common::{Emulator, Provable};
use sm_mem::MemSM;
use wchelpers::{WCComponent, WCExecutor, WCOpCalculator};

pub struct MainSM {
    arith_sm: Arc<ArithSM>,
    mem_sm: Arc<MemSM>,
}

impl MainSM {
    pub fn new<F>(
        wcm: &mut WCManager<F>,
        mem_sm: Arc<MemSM>,
        arith_sm: Arc<ArithSM>,
        air_ids: &[usize],
    ) -> Arc<Self> {
        let main = Arc::new(Self { mem_sm, arith_sm });

        wcm.register_component(main.clone() as Arc<dyn WCComponent<F>>, Some(air_ids));

        main
    }

    // Callback method, now accepting a scope
    fn emulate_callback(&self, inputs: Vec<EmuTrace>, scope: &Scope) {
        let arith_sm = self.arith_sm.clone();
        scope.spawn(move |scope| {
            // This is the code to be executed in the new thread to manage the inputs and pass it to
            // the state machines
            // arith_sm.prove(&inputs, false, scope);
            println!("Emulate callback done");
        });
    }
}

impl<F> WCComponent<F> for MainSM {
    fn calculate_witness(
        &self,
        stage: u32,
        air_instance: &AirInstance,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
    ) {
    }

    fn suggest_plan(&self, _ectx: &mut ExecutionCtx) {}
}

impl<F> WCExecutor<F> for MainSM {
    fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx) {
        debug!("Executing MainSM");

        let pool = ThreadPoolBuilder::new().build().unwrap();

        let emulator = MockEmulator {};

        // Use rayon's scope to manage the lifetime of spawned threads
        pool.scope(|scope| {
            // Wrap the callback to capture self
            let callback = |inputs: Vec<EmuTrace>| self.emulate_callback(inputs, scope);
            let result = emulator.emulate(8, callback);

            println!("Result: {:?}", result);
        });

        // Terminate the state machines to drain remaining inputs
        pool.scope(|scope| {
            scope.spawn(move |scope| {
                println!("Terminating arith_sm");
                self.arith_sm.prove(&[], true, scope);
            });
        });

        println!("All threads completed, finishing the main execution");
    }
}
