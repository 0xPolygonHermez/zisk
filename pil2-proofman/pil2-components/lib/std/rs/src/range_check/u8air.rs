use std::sync::{atomic::AtomicU64, Arc, Mutex};
use num_traits::ToPrimitive;
use p3_field::{Field, PrimeField};

use witness::WitnessComponent;
use proofman_common::{TraceInfo, AirInstance, ProofCtx, SetupCtx};
use proofman_hints::{get_hint_field, get_hint_ids_by_name, set_hint_field, HintFieldOptions, HintFieldValue};
use proofman_util::create_buffer_fast;
use std::sync::atomic::Ordering;

use crate::AirComponent;

const PROVE_CHUNK_SIZE: usize = 1 << 5;
const NUM_ROWS: usize = 1 << 8;

pub struct U8Air<F: Field> {
    // Parameters
    hint: AtomicU64,
    airgroup_id: usize,
    air_id: usize,

    // Inputs
    inputs: Mutex<Vec<(F, F)>>, // value -> multiplicity
    mul_column: Mutex<HintFieldValue<F>>,
}

impl<F: PrimeField> AirComponent<F> for U8Air<F> {
    const MY_NAME: &'static str = "U8Air   ";

    fn new(
        _pctx: Arc<ProofCtx<F>>,
        _sctx: Arc<SetupCtx>,
        airgroup_id: Option<usize>,
        air_id: Option<usize>,
    ) -> Arc<Self> {
        let airgroup_id = airgroup_id.expect("Airgroup ID must be provided");
        let air_id = air_id.expect("Air ID must be provided");
        Arc::new(Self {
            hint: AtomicU64::new(0),
            airgroup_id,
            air_id,
            inputs: Mutex::new(Vec::new()),
            mul_column: Mutex::new(HintFieldValue::Field(F::zero())),
        })
    }
}

impl<F: PrimeField> U8Air<F> {
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

    pub fn drain_inputs(&self, pctx: Arc<ProofCtx<F>>, sctx: Arc<SetupCtx>) {
        let mut inputs = self.inputs.lock().unwrap();
        let drained_inputs = inputs.drain(..).collect();

        // Perform the last update
        self.update_multiplicity(drained_inputs);

        let mut multiplicity = match &*self.mul_column.lock().unwrap() {
            HintFieldValue::Column(values) => {
                values.iter().map(|x| x.as_canonical_biguint().to_u64().unwrap()).collect::<Vec<u64>>()
            }
            _ => panic!("Multiplicities must be a column"),
        };

        let (instance_found, global_idx) = pctx.dctx_find_instance(self.airgroup_id, self.air_id);

        let (is_mine, global_idx) = if instance_found {
            (pctx.dctx_is_my_instance(global_idx), global_idx)
        } else {
            pctx.dctx_add_instance(self.airgroup_id, self.air_id, pctx.get_weight(self.airgroup_id, self.air_id))
        };

        pctx.dctx_distribute_multiplicity(&mut multiplicity, global_idx);

        if is_mine {
            let instance: Vec<usize> = pctx.air_instance_repo.find_air_instances(self.airgroup_id, self.air_id);
            if instance.is_empty() {
                let num_rows = pctx.global_info.airs[self.airgroup_id][self.air_id].num_rows;
                let buffer_size = num_rows;
                let buffer: Vec<F> = create_buffer_fast(buffer_size);
                let air_instance = AirInstance::new(TraceInfo::new(self.airgroup_id, self.air_id, buffer));
                pctx.add_air_instance(air_instance, global_idx);
            };

            let mut air_instances = pctx.air_instance_repo.air_instances.write().unwrap();
            let air_instance = air_instances.get_mut(&global_idx).unwrap();

            // copy multiplicitis back to mul_column
            let mul_column_2 =
                HintFieldValue::Column(multiplicity.iter().map(|x| F::from_canonical_u64(*x)).collect::<Vec<F>>());

            set_hint_field(&sctx, air_instance, self.hint.load(Ordering::Acquire), "reference", &mul_column_2);

            log::trace!("{}: ··· Drained inputs for AIR '{}'", Self::MY_NAME, "U8Air");
        }
    }

    fn update_multiplicity(&self, drained_inputs: Vec<(F, F)>) {
        let mut mul_column = self.mul_column.lock().unwrap();
        for (input, mul) in &drained_inputs {
            let value = input.as_canonical_biguint().to_usize().expect("Cannot convert to usize");
            // Note: to avoid non-expected panics, we perform a reduction to the value
            //       In debug mode, this is, in fact, checked before
            let index = value % NUM_ROWS;
            mul_column.add(index, *mul);
        }
    }

    pub fn airgroup_id(&self) -> usize {
        self.airgroup_id
    }

    pub fn air_id(&self) -> usize {
        self.air_id
    }
}

impl<F: PrimeField> WitnessComponent<F> for U8Air<F> {
    fn start_proof(&self, pctx: Arc<ProofCtx<F>>, sctx: Arc<SetupCtx>) {
        // Obtain info from the mul hints
        let setup = sctx.get_setup(self.airgroup_id, self.air_id);
        let u8air_hints = get_hint_ids_by_name(setup.p_setup.p_expressions_bin, "u8air");
        if !u8air_hints.is_empty() {
            self.hint.store(u8air_hints[0], Ordering::Release);
        }

        // self.setup_repository.replace(sctx.setups.clone());

        let num_rows = pctx.global_info.airs[self.airgroup_id][self.air_id].num_rows;
        let buffer_size = num_rows;
        let buffer = create_buffer_fast(buffer_size);

        // Add a new air instance. Since U8Air is a table, only this air instance is needed
        let mut air_instance = AirInstance::new(TraceInfo::new(self.airgroup_id, self.air_id, buffer));

        *self.mul_column.lock().unwrap() = get_hint_field::<F>(
            &sctx,
            &pctx,
            &mut air_instance,
            u8air_hints[0] as usize,
            "reference",
            HintFieldOptions::dest_with_zeros(),
        );
    }

    fn calculate_witness(&self, stage: u32, pctx: Arc<ProofCtx<F>>, sctx: Arc<SetupCtx>) {
        if stage == 1 {
            Self::drain_inputs(self, pctx, sctx);
        }
    }
}
