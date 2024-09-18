use core::num;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use num_traits::ToPrimitive;
use p3_field::PrimeField;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{
    AirInstance, AirInstancesRepository, ExecutionCtx, ProofCtx, SetupCtx, SetupRepository,
};
use proofman_hints::{get_hint_field, print_expression, set_hint_field, HintFieldOptions, HintFieldOutput};

const PROVE_CHUNK_SIZE: usize = 1 << 6 + 1;

pub struct U8Air<F> {
    // Proof-related data
    setup_repository: RefCell<Arc<SetupRepository>>,
    public_inputs: Arc<RefCell<Vec<u8>>>,
    challenges: Arc<RefCell<Vec<F>>>,
    air_instances_repository: RefCell<Arc<AirInstancesRepository<F>>>,
    // Parameters
    pub hint: Mutex<u64>,
    airgroup_id: usize,
    air_id: usize,
    // Inputs
    inputs: Mutex<Vec<F>>, // value -> multiplicity
}

impl<F: PrimeField> U8Air<F> {
    const MY_NAME: &'static str = "U8Air";

    pub fn new(wcm: &mut WitnessManager<F>, airgroup_id: usize, air_id: usize) -> Arc<Self> {
        let u8air = Arc::new(Self {
            setup_repository: RefCell::new(Arc::new(SetupRepository { setups: Vec::new() })),
            public_inputs: Arc::new(RefCell::new(Vec::new())),
            challenges: Arc::new(RefCell::new(Vec::new())),
            air_instances_repository: RefCell::new(Arc::new(AirInstancesRepository::new())),
            hint: Mutex::new(0),
            airgroup_id,
            air_id,
            inputs: Mutex::new(Vec::new()),
        });

        wcm.register_component(u8air.clone(), Some(airgroup_id), Some(&[air_id]));

        u8air
    }

    pub fn drain_inputs(&self) {
        let mut inputs = self.inputs.lock().unwrap();
        let drained_inputs = inputs.drain(..).collect();

        // Perform the last update
        self.update_multiplicity(drained_inputs);

        println!("{}: Drained inputs for AIR 'U8Air'", Self::MY_NAME);
    }

    pub fn update_inputs(&self, value: F) {
        if let Ok(mut inputs) = self.inputs.lock() {
            inputs.push(value);

            while inputs.len() >= PROVE_CHUNK_SIZE {
                let num_drained = std::cmp::min(PROVE_CHUNK_SIZE, inputs.len());
                let drained_inputs = inputs.drain(..num_drained).collect();

                println!("ROGEEEE");
                self.update_multiplicity(drained_inputs);
                println!("ROGEEEE2");
            }
        }
    }

    fn update_multiplicity(&self, drained_inputs: Vec<F>) {
        // TODO! Do it in parallel
        // Update the multiplicity column
        let num_rows = 1 << 8;
        let hint = *self.hint.lock().unwrap() as usize;

        let air_instance_id = self
            .air_instances_repository
            .borrow()
            .find_air_instances(self.airgroup_id, self.air_id)[0];
        println!("ROGEEEE3");
        let air_instance_bind = self.air_instances_repository.borrow();
        let mut air_instance_rw = air_instance_bind.air_instances.write().unwrap();
        let air_instance = &mut air_instance_rw[air_instance_id];

        let mut mul = get_hint_field::<F>(
            self.setup_repository.borrow().as_ref(),
            self.public_inputs.clone(),
            self.challenges.clone(),
            air_instance,
            hint,
            "reference",
            HintFieldOptions::dest(),
        );
        for i in 0..10 {
            println!("{}: {:?}", i, mul.get(i as usize));
        }

        for input in &drained_inputs {
            let value = input
                .as_canonical_biguint()
                .to_usize()
                .expect("Cannot convert to usize");
            // Note: to avoid non-expected panics, we perform a reduction to the value
            //       In debug mode, this is, in fact, checked before
            let index = value % num_rows;
            mul.add(index, F::one());
        }

        // TODO: To be removed
        set_hint_field(
            self.setup_repository.borrow().as_ref(),
            air_instance,
            hint as u64,
            "reference",
            &mul,
        );

        log::info!("{}: Updated inputs for AIR '{}'", Self::MY_NAME, "U8Air");
    }
}

impl<F: PrimeField> WitnessComponent<F> for U8Air<F> {
    fn start_proof(&self, pctx: &ProofCtx<F>, ectx: &ExecutionCtx, sctx: &SetupCtx) {
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
        let air_instance = AirInstance::new(self.airgroup_id, self.air_id, None, buffer);
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
        // // Set the multiplicity column as done
        // let hint = self.hint.lock().unwrap();

        // let air_instance_id = self
        //     .air_instances_repository
        //     .borrow()
        //     .find_air_instances(self.airgroup_id, self.air_id)[0];

        // let air_instances = self.air_instances_repository.borrow();
        // let mut air_instance_rw = air_instances.air_instances.write().unwrap();
        // let air_instance = &mut air_instance_rw[air_instance_id];

        // let mul = get_hint_field::<F>(
        //     self.setup_repository.borrow().as_ref(),
        //     self.public_inputs.clone(),
        //     self.challenges.clone(),
        //     air_instance,
        //     *hint as usize,
        //     "reference",
        //     true,
        //     false,
        //     true,
        // );
        // for i in 0..10 {
        //     println!("{}: {:?}", i, mul.get(i as usize));
        // }

        // set_hint_field(
        //     self.setup_repository.borrow().as_ref(),
        //     air_instance,
        //     *hint,
        //     "reference",
        //     &mul,
        // );
    }
}
