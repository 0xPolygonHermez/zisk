use std::sync::{atomic::AtomicU64, Arc, Mutex};

use num_traits::ToPrimitive;
use p3_field::{Field, PrimeField};

use proofman::{get_hint_field_gc, WitnessComponent, WitnessManager};
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx, SetupCtx};

use proofman_hints::{get_hint_field, get_hint_ids_by_name, set_hint_field, HintFieldOptions, HintFieldValue};
use proofman_util::create_buffer_fast;
use std::sync::atomic::Ordering;

const PROVE_CHUNK_SIZE: usize = 1 << 5;
const NUM_ROWS: usize = 1 << 16;

pub struct U16Air<F: Field> {
    wcm: Arc<WitnessManager<F>>,

    // Parameters
    hint: AtomicU64,
    airgroup_id: usize,
    air_id: usize,
    // Inputs
    inputs: Mutex<Vec<(F, F)>>, // value -> multiplicity
    mul_column: Mutex<HintFieldValue<F>>,
}

impl<F: PrimeField> U16Air<F> {
    const MY_NAME: &'static str = "U16Air  ";

    pub fn new(wcm: Arc<WitnessManager<F>>) -> Arc<Self> {
        let pctx = wcm.get_pctx();
        let sctx = wcm.get_sctx();

        // Scan global hints to get the airgroup_id and air_id
        let hint_global = get_hint_ids_by_name(sctx.get_global_bin(), "u16air");
        let airgroup_id = get_hint_field_gc::<F>(pctx.clone(), sctx.clone(), hint_global[0], "airgroup_id", false);
        let air_id = get_hint_field_gc::<F>(pctx.clone(), sctx.clone(), hint_global[0], "air_id", false);
        let airgroup_id = match airgroup_id {
            HintFieldValue::Field(value) => value
                .as_canonical_biguint()
                .to_usize()
                .unwrap_or_else(|| panic!("Aigroup_id cannot be converted to usize: {}", value)),
            _ => {
                log::error!("Aigroup_id hint must be a field element");
                panic!();
            }
        };
        let air_id = match air_id {
            HintFieldValue::Field(value) => value
                .as_canonical_biguint()
                .to_usize()
                .unwrap_or_else(|| panic!("Air_id cannot be converted to usize: {}", value)),
            _ => {
                log::error!("Air_id hint must be a field element");
                panic!();
            }
        };

        let u16air = Arc::new(Self {
            wcm: wcm.clone(),
            hint: AtomicU64::new(0),
            airgroup_id,
            air_id,
            inputs: Mutex::new(Vec::new()),
            mul_column: Mutex::new(HintFieldValue::Field(F::zero())),
        });

        wcm.register_component(u16air.clone(), Some(airgroup_id), Some(&[air_id]));

        u16air
    }

    pub fn update_inputs(&self, value: F, multiplicity: F) {
        let mut inputs = self.inputs.lock().unwrap();
        inputs.push((value, multiplicity));

        while inputs.len() >= PROVE_CHUNK_SIZE {
            let num_drained = std::cmp::min(PROVE_CHUNK_SIZE, inputs.len());
            let drained_inputs = inputs.drain(..num_drained).collect();

            // Update the multiplicity column
            self.update_multiplicity(drained_inputs);
        }
    }

    pub fn drain_inputs(&self) {
        let mut inputs = self.inputs.lock().unwrap();
        let drained_inputs = inputs.drain(..).collect();
        let pctx = self.wcm.get_pctx();
        let sctx = self.wcm.get_sctx();
        let ectx = self.wcm.get_ectx();

        // Perform the last update
        self.update_multiplicity(drained_inputs);

        let mut dctx: std::sync::RwLockWriteGuard<'_, proofman_common::DistributionCtx> = ectx.dctx.write().unwrap();

        let (is_myne, global_idx) = dctx.add_instance(self.airgroup_id, self.air_id, 1);
        let mut multiplicity = match &*self.mul_column.lock().unwrap() {
            HintFieldValue::Column(values) => {
                values.iter().map(|x| x.as_canonical_biguint().to_u64().unwrap()).collect::<Vec<u64>>()
            }
            _ => panic!("Multiplicities must be a column"),
        }; //rick: definir multiplicities com u32 directe?

        let owner = dctx.owner(global_idx);
        dctx.distribute_multiplicity(&mut multiplicity, owner);

        if is_myne {
            let air_instance_repo = &self.wcm.get_pctx().air_instance_repo;
            let instance: Vec<usize> = air_instance_repo.find_air_instances(self.airgroup_id, self.air_id);
            let air_instance_id = if instance.len() != 0 {
                //rick: this code will desapear
                air_instance_repo.find_air_instances(self.airgroup_id, self.air_id)[0]
            } else {
                // create instance
                let (buffer_size, _) =
                    ectx.buffer_allocator.as_ref().get_buffer_info(&sctx, self.airgroup_id, self.air_id).unwrap();
                let buffer: Vec<F> = create_buffer_fast(buffer_size as usize);
                let air_instance = AirInstance::new(sctx.clone(), self.airgroup_id, self.air_id, None, buffer);
                pctx.air_instance_repo.add_air_instance(air_instance, Some(global_idx));
                pctx.air_instance_repo.air_instances.read().unwrap().len() - 1
            };

            let mut air_instance_rw = air_instance_repo.air_instances.write().unwrap();
            let air_instance = &mut air_instance_rw[air_instance_id];

            // copy multiplicitis back to mul_column
            let mul_column_2 =
                HintFieldValue::Column(multiplicity.iter().map(|x| F::from_canonical_u64(*x)).collect::<Vec<F>>());

            set_hint_field(
                &self.wcm.get_sctx(),
                air_instance,
                self.hint.load(Ordering::Acquire),
                "reference",
                &mul_column_2,
            );

            log::trace!("{}: ··· Drained inputs for AIR '{}'", Self::MY_NAME, "U16Air");
        }
    }

    fn update_multiplicity(&self, drained_inputs: Vec<(F, F)>) {
        // TODO! Do it in parallel
        for (input, mul) in &drained_inputs {
            let value = input.as_canonical_biguint().to_usize().expect("Cannot convert to usize");
            // Note: to avoid non-expected panics, we perform a reduction to the value
            //       In debug mode, this is, in fact, checked before
            let index = value % NUM_ROWS;
            let mut mul_column = self.mul_column.lock().unwrap();
            mul_column.add(index, *mul);
        }
    }
}

impl<F: PrimeField> WitnessComponent<F> for U16Air<F> {
    fn start_proof(&self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        // Obtain info from the mul hints
        let setup = sctx.get_partial_setup(self.airgroup_id, self.air_id).unwrap_or_else(|_| {
            panic!("Setup not found for airgroup_id: {}, air_id: {}", self.airgroup_id, self.air_id)
        });
        let u16air_hints = get_hint_ids_by_name(setup.p_setup.p_expressions_bin, "u16air");
        if !u16air_hints.is_empty() {
            self.hint.store(u16air_hints[0], Ordering::Release);
        }

        // self.setup_repository.replace(sctx.setups.clone());

        let (buffer_size, _) =
            ectx.buffer_allocator.as_ref().get_buffer_info(&sctx, self.airgroup_id, self.air_id).unwrap();
        let buffer = create_buffer_fast(buffer_size as usize);

        // Add a new air instance. Since U16Air is a table, only this air instance is needed
        let mut air_instance = AirInstance::new(sctx.clone(), self.airgroup_id, self.air_id, None, buffer);

        *self.mul_column.lock().unwrap() = get_hint_field::<F>(
            &self.wcm.get_sctx(),
            &pctx,
            &mut air_instance,
            u16air_hints[0] as usize,
            "reference",
            HintFieldOptions::dest_with_zeros(),
        );

        //pctx.air_instance_repo.add_air_instance(air_instance);
        //rick: simplificar per operar només amb la multipliciata, res més... no cal crear buffer, etc...
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
