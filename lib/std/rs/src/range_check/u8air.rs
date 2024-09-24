use std::sync::{atomic::AtomicU64, Arc, Mutex};

use num_traits::ToPrimitive;
use p3_field::PrimeField;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx, SetupCtx};
use proofman_hints::{
    get_hint_field, get_hint_ids_by_name, set_hint_field, HintFieldOptions, HintFieldValue,
};
use std::sync::atomic::Ordering;

const PROVE_CHUNK_SIZE: usize = 1 << 5;
const NUM_ROWS: usize = 1 << 8;

pub struct U8Air<F: Copy> {
    wcm: Arc<WitnessManager<F>>,

    // Parameters
    hint: AtomicU64,
    airgroup_id: usize,
    air_id: usize,

    // Inputs
    inputs: Mutex<Vec<F>>, // value -> multiplicity
    mul: Mutex<HintFieldValue<F>>,
}

impl<F: PrimeField> U8Air<F> {
    const MY_NAME: &'static str = "U8Air   ";

    pub fn new(wcm: Arc<WitnessManager<F>>, airgroup_id: usize, air_id: usize) -> Arc<Self> {
        let u8air = Arc::new(Self {
            wcm: wcm.clone(),
            hint: AtomicU64::new(0),
            airgroup_id,
            air_id,
            inputs: Mutex::new(Vec::new()),
            mul: Mutex::new(HintFieldValue::Field(F::zero())),
        });

        wcm.register_component(u8air.clone(), Some(airgroup_id), Some(&[air_id]));

        u8air
    }

    pub fn update_inputs(&self, value: F) {
        let mut inputs = self.inputs.lock().unwrap();
        inputs.push(value);

        while inputs.len() >= PROVE_CHUNK_SIZE {
            let num_drained = std::cmp::min(PROVE_CHUNK_SIZE, inputs.len());
            let drained_inputs = inputs.drain(..num_drained).collect();

            // Update the multiplicity column
            self.update_multiplicity(drained_inputs);

            log::info!("{}: Updated inputs for AIR '{}'", Self::MY_NAME, "U8Air");
        }
    }

    pub fn drain_inputs(&self) {
        let mut inputs = self.inputs.lock().unwrap();
        let drained_inputs = inputs.drain(..).collect();

        // Perform the last update
        self.update_multiplicity(drained_inputs);

        let air_instance_repo = &self.wcm.get_pctx().air_instance_repo;
        let air_instance_id =
            air_instance_repo.find_air_instances(self.airgroup_id, self.air_id)[0];

        let mut air_instance_rw = air_instance_repo.air_instances.write().unwrap();
        let air_instance = &mut air_instance_rw[air_instance_id];

        let mul = &*self.mul.lock().unwrap();
        set_hint_field(
            self.wcm.get_sctx(),
            air_instance,
            self.hint.load(Ordering::Acquire),
            "reference",
            mul,
        );

        log::info!("{}: Drained inputs for AIR '{}'", Self::MY_NAME, "U8Air");
    }

    fn update_multiplicity(&self, drained_inputs: Vec<F>) {
        // TODO! Do it in parallel
        for input in &drained_inputs {
            let value = input
                .as_canonical_biguint()
                .to_usize()
                .expect("Cannot convert to usize");
            // Note: to avoid non-expected panics, we perform a reduction to the value
            //       In debug mode, this is, in fact, checked before
            let index = value % NUM_ROWS;
            let mut mul = self.mul.lock().unwrap();
            mul.add(index, F::one());
        }
    }
}

impl<F: PrimeField> WitnessComponent<F> for U8Air<F> {
    fn start_proof(&self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        // TODO: We can optimize this
        // Scan the pilout for airs that have rc-related hints
        let air_groups = pctx.pilout.air_groups();
        for air_group in air_groups.iter() {
            let airs = air_group.airs();
            for air in airs.iter() {
                let airgroup_id = air.airgroup_id;
                let air_id = air.air_id;
                let setup = sctx.get_setup(airgroup_id, air_id).expect("REASON");

                // Obtain info from the mul hints
                let u8air_hints = get_hint_ids_by_name(*setup.p_setup, "u8air");
                if !u8air_hints.is_empty() {
                    self.hint.store(u8air_hints[0], Ordering::Release);
                }
            }
        }

        // self.setup_repository.replace(sctx.setups.clone());

        let (buffer_size, _) = ectx
            .buffer_allocator
            .as_ref()
            .get_buffer_info(&sctx, self.airgroup_id, self.air_id)
            .unwrap();
        let buffer = vec![F::zero(); buffer_size as usize];

        // Add a new air instance. Since U8Air is a table, only this air instance is needed
        let mut air_instance = AirInstance::new(self.airgroup_id, self.air_id, None, buffer);

        *self.mul.lock().unwrap() = get_hint_field::<F>(
            &sctx,
            &pctx.public_inputs,
            &pctx.challenges,
            &mut air_instance,
            self.hint.load(Ordering::Acquire) as usize,
            "reference",
            HintFieldOptions::dest(),
        );

        pctx.air_instance_repo.add_air_instance(air_instance);
    }

    fn calculate_witness(
        &self,
        _stage: u32,
        _air_instance: Option<usize>,
        _pctx: Arc<ProofCtx<F>>,
        _ectx: Arc<ExecutionCtx>,
        _sctx: Arc<SetupCtx>,
    ) {
    }
}
