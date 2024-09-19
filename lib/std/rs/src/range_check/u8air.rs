use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use num_traits::ToPrimitive;
use p3_field::PrimeField;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{
    AirInstance, AirInstancesRepository, ExecutionCtx, ProofCtx, SetupCtx, SetupRepository,
};
use proofman_hints::{
    get_hint_field, get_hint_ids_by_name, set_hint_field, HintFieldOptions, HintFieldValue,
};

const PROVE_CHUNK_SIZE: usize = 1 << 5;
const NUM_ROWS: usize = 1 << 8;

pub struct U8Air<F: Copy> {
    // Proof-related data
    setup_repository: RefCell<Arc<SetupRepository>>,
    public_inputs: Arc<RefCell<Vec<u8>>>,
    challenges: Arc<RefCell<Vec<F>>>,
    air_instances_repository: RefCell<Arc<AirInstancesRepository<F>>>,
    // Parameters
    hint: RefCell<u64>,
    airgroup_id: usize,
    air_id: usize,
    // Inputs
    inputs: Mutex<Vec<F>>, // value -> multiplicity
    mul: RefCell<HintFieldValue<F>>,
}

impl<F: PrimeField> U8Air<F> {
    const MY_NAME: &'static str = "U8Air";

    pub fn new(wcm: &mut WitnessManager<F>, airgroup_id: usize, air_id: usize) -> Arc<Self> {
        let u8air = Arc::new(Self {
            setup_repository: RefCell::new(Arc::new(SetupRepository { setups: Vec::new() })),
            public_inputs: Arc::new(RefCell::new(Vec::new())),
            challenges: Arc::new(RefCell::new(Vec::new())),
            air_instances_repository: RefCell::new(Arc::new(AirInstancesRepository::new())),
            hint: RefCell::new(0),
            airgroup_id,
            air_id,
            inputs: Mutex::new(Vec::new()),
            mul: RefCell::new(HintFieldValue::Field(F::zero())),
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

        // Set the multiplicity column as done
        let hint = self.hint.borrow();

        let air_instance_id = self
            .air_instances_repository
            .borrow()
            .find_air_instances(self.airgroup_id, self.air_id)[0];

        let air_instances = self.air_instances_repository.borrow();
        let mut air_instance_rw = air_instances.air_instances.write().unwrap();
        let air_instance = &mut air_instance_rw[air_instance_id];

        set_hint_field(
            self.setup_repository.borrow().as_ref(),
            air_instance,
            *hint,
            "reference",
            &self.mul.borrow(),
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
            self.mul.borrow_mut().add(index, F::one());
        }
    }
}

impl<F: PrimeField> WitnessComponent<F> for U8Air<F> {
    fn start_proof(&self, pctx: &ProofCtx<F>, ectx: &ExecutionCtx, sctx: &SetupCtx) {
        // TODO: We can optimize this
        // Scan the pilout for airs that have rc-related hints
        let air_groups = pctx.pilout.air_groups();
        for air_group in air_groups.iter() {
            let airs = air_group.airs();
            for air in airs.iter() {
                let airgroup_id = air.airgroup_id;
                let air_id = air.air_id;
                let setup = sctx.setups.get_setup(airgroup_id, air_id).expect("REASON");

                // Obtain info from the mul hints
                let u8air_hints = get_hint_ids_by_name(setup.p_setup, "u8air");
                if u8air_hints.len() > 0 {
                    self.hint.replace(u8air_hints[0]);
                }
            }
        }

        self.setup_repository.replace(sctx.setups.clone());
        self.air_instances_repository
            .replace(pctx.air_instance_repo.clone());

        let (buffer_size, _) = ectx
            .buffer_allocator
            .as_ref()
            .get_buffer_info("U8Air".into(), self.air_id)
            .unwrap();
        let buffer = vec![F::zero(); buffer_size as usize];

        // Add a new air instance. Since U8Air is a table, only this air instance is needed
        let mut air_instance = AirInstance::new(self.airgroup_id, self.air_id, None, buffer);

        let hint = self.hint.borrow().to_usize().unwrap();
        self.mul.replace(get_hint_field::<F>(
            self.setup_repository.borrow().as_ref(),
            self.public_inputs.clone(),
            self.challenges.clone(),
            &mut air_instance,
            hint,
            "reference",
            HintFieldOptions::dest(),
        ));

        self.air_instances_repository
            .borrow_mut()
            .add_air_instance(air_instance);
    }

    fn calculate_witness(
        &self,
        _stage: u32,
        _air_instance: Option<usize>,
        _pctx: &mut ProofCtx<F>,
        _ectx: &ExecutionCtx,
        _sctx: &SetupCtx,
    ) {
    }
}
