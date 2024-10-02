use core::panic;
use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
};

use num_bigint::BigInt;
use p3_field::PrimeField;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx};
use proofman_hints::{get_hint_field_constant, get_hint_ids_by_name, HintFieldOptions, HintFieldValue};
use rayon::Scope;

use crate::{Decider, Range, SpecifiedRanges, StdMode, U16Air, U8Air};

const BYTE: u8 = 255;
const TWOBYTES: u16 = 65535;

const STD_RANGE_CHECK_VARIANTS: usize = 3;

#[derive(Debug, Eq, Hash, PartialEq, Clone)]
pub enum RangeCheckAir {
    U8Air,
    U16Air,
    SpecifiedRanges,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum StdRangeCheckType {
    Valid(RangeCheckAir),
    U8AirDouble,
    U16AirDouble,
}

#[derive(Clone, Debug)]
struct StdRangeItem<F: PrimeField> {
    rc_type: StdRangeCheckType,
    range: Range<F>,
}

// TODO: Remove Arc
pub struct StdRangeCheck<F: PrimeField> {
    mode: StdMode,
    ranges: Mutex<Vec<StdRangeItem<F>>>,
    u8air: Option<Arc<U8Air<F>>>,
    u16air: Option<Arc<U16Air<F>>>,
    specified_ranges: Option<Arc<SpecifiedRanges<F>>>,
}

pub struct RCAirData {
    pub air_name: RangeCheckAir,
    pub airgroup_id: usize,
    pub air_id: usize,
}

impl<F: PrimeField> Decider<F> for StdRangeCheck<F> {
    fn decide(&self, sctx: Arc<SetupCtx>, pctx: Arc<ProofCtx<F>>) {
        // Scan the pilout for airs that have rc-related hints
        let air_groups = pctx.pilout.air_groups();

        air_groups.iter().for_each(|air_group| {
            let airs = air_group.airs();
            airs.iter().for_each(|air| {
                let airgroup_id = air.airgroup_id;
                let air_id = air.air_id;
                let setup = sctx.get_partial_setup(airgroup_id, air_id).expect("REASON");

                // Obtain info from the range hints
                let rc_hints = get_hint_ids_by_name((&setup.p_setup).into(), "range_def");
                for hint in rc_hints {
                    // Register the range
                    self.register_range(sctx.clone(), airgroup_id, air_id, hint);
                }
            });
        });
    }
}

impl<F: PrimeField> StdRangeCheck<F> {
    const _MY_NAME: &'static str = "STD Range Check";

    pub fn new(mode: StdMode, wcm: Arc<WitnessManager<F>>, air_data: Option<Vec<RCAirData>>) -> Arc<Self> {
        let mut u8air = None;
        let mut u16air = None;
        let mut specified_ranges = None;
        // Check the air data and register the range check airs if they exist
        if let Some(air_data) = air_data.as_ref() {
            if air_data.len() > STD_RANGE_CHECK_VARIANTS {
                log::error!(
                    "The air_data provided has incorrect lenght: expected at most {}, found {}",
                    STD_RANGE_CHECK_VARIANTS,
                    air_data.len()
                );
                panic!();
            }

            for air in air_data {
                let air_name = &air.air_name;
                let airgroup_id = air.airgroup_id;
                let air_id = air.air_id;

                match air_name {
                    RangeCheckAir::U8Air => {
                        u8air = Some(U8Air::new(wcm.clone(), airgroup_id, air_id));
                    }
                    RangeCheckAir::U16Air => {
                        u16air = Some(U16Air::new(wcm.clone(), airgroup_id, air_id));
                    }
                    RangeCheckAir::SpecifiedRanges => {
                        specified_ranges = Some(SpecifiedRanges::new(wcm.clone(), airgroup_id, air_id));
                    }
                }
            }
        }

        let std_range_check = Arc::new(Self { mode, ranges: Mutex::new(Vec::new()), u8air, u16air, specified_ranges });

        wcm.register_component(std_range_check.clone(), None, None);

        std_range_check
    }

    pub fn register_range(&self, sctx: Arc<SetupCtx>, airgroup_id: usize, air_id: usize, hint: u64) {
        let predefined = get_hint_field_constant::<F>(
            &sctx,
            airgroup_id,
            air_id,
            hint as usize,
            "predefined",
            HintFieldOptions::default(),
        );
        let min =
            get_hint_field_constant::<F>(&sctx, airgroup_id, air_id, hint as usize, "min", HintFieldOptions::default());
        let min_neg = get_hint_field_constant::<F>(
            &sctx,
            airgroup_id,
            air_id,
            hint as usize,
            "min_neg",
            HintFieldOptions::default(),
        );
        let max =
            get_hint_field_constant::<F>(&sctx, airgroup_id, air_id, hint as usize, "max", HintFieldOptions::default());
        let max_neg = get_hint_field_constant::<F>(
            &sctx,
            airgroup_id,
            air_id,
            hint as usize,
            "max_neg",
            HintFieldOptions::default(),
        );

        let HintFieldValue::Field(predefined) = predefined else {
            log::error!("Predefined hint must be a field element");
            panic!();
        };
        let HintFieldValue::Field(min) = min else {
            log::error!("Min hint must be a field element");
            panic!();
        };
        let HintFieldValue::Field(min_neg) = min_neg else {
            log::error!("Min_neg hint must be a field element");
            panic!();
        };
        let HintFieldValue::Field(max) = max else {
            log::error!("Max hint must be a field element");
            panic!();
        };
        let HintFieldValue::Field(max_neg) = max_neg else {
            log::error!("Max_neg hint must be a field element");
            panic!();
        };

        let predefined = {
            if !predefined.is_zero() && !predefined.is_one() {
                log::error!("Predefined hint must be either 0 or 1");
                panic!();
            }
            predefined.is_one()
        };
        let min_neg = {
            if !min_neg.is_zero() && !min_neg.is_one() {
                log::error!("Predefined hint must be either 0 or 1");
                panic!();
            }
            min_neg.is_one()
        };
        let max_neg = {
            if !max_neg.is_zero() && !max_neg.is_one() {
                log::error!("Predefined hint must be either 0 or 1");
                panic!();
            }
            max_neg.is_one()
        };

        let range = Range(min, max, min_neg, max_neg);

        // If the range is already defined, skip
        let mut ranges = self.ranges.lock().unwrap();
        if ranges.iter().any(|r| r.range == range) {
            return;
        }

        // Otherwise, register the range
        let zero = F::zero();
        let byte = F::from_canonical_u8(BYTE);
        let twobytes = F::from_canonical_u16(TWOBYTES);
        // Associate to each unique range a range check type
        let r#type = if predefined && range.contained_in(&(0.into(), TWOBYTES.into())) {
            match range {
                Range(min, max, ..) if min == zero && max == byte => StdRangeCheckType::Valid(RangeCheckAir::U8Air),
                Range(min, max, ..) if min == zero && max == twobytes => {
                    StdRangeCheckType::Valid(RangeCheckAir::U16Air)
                }
                Range(_, max, ..) if max <= byte => StdRangeCheckType::U8AirDouble,
                Range(_, max, ..) if max <= twobytes => StdRangeCheckType::U16AirDouble,
                _ => panic!("Invalid predefined range"),
            }
        } else {
            StdRangeCheckType::Valid(RangeCheckAir::SpecifiedRanges)
        };

        // Update ranges
        ranges.push(StdRangeItem { rc_type: r#type, range });
    }

    pub fn assign_values(&self, value: F, min: BigInt, max: BigInt) {
        // If the range was not computed in the setup phase, error
        let ranges = self.ranges.lock().unwrap();
        let range_item = ranges.iter().find(|r| r.range == (min.clone(), max.clone()));

        if range_item.is_none() {
            log::error!("Range not found: [min,max] = [{},{}]", min, max);
            panic!();
        }

        let range_item = range_item.unwrap();
        let range = range_item.range;

        if self.mode == StdMode::Debug && !range.contains(value) {
            log::error!("Value {} is not in the range [min,max] = {:?}", value, range,);
            panic!();
        }

        match range_item.rc_type {
            StdRangeCheckType::Valid(RangeCheckAir::U8Air) => {
                self.u8air.as_ref().unwrap().update_inputs(value);
            }
            StdRangeCheckType::Valid(RangeCheckAir::U16Air) => {
                self.u16air.as_ref().unwrap().update_inputs(value);
            }
            StdRangeCheckType::U8AirDouble => {
                self.u8air.as_ref().unwrap().update_inputs(value - range.0);
                self.u8air.as_ref().unwrap().update_inputs(range.1 - value);
            }
            StdRangeCheckType::U16AirDouble => {
                self.u16air.as_ref().unwrap().update_inputs(value - range.0);
                self.u16air.as_ref().unwrap().update_inputs(range.1 - value);
            }
            StdRangeCheckType::Valid(RangeCheckAir::SpecifiedRanges) => {
                self.specified_ranges.as_ref().unwrap().update_inputs(value, range);
            }
        }
    }

    pub fn drain_inputs(&self, _pctx: Arc<ProofCtx<F>>, _scope: Option<&Scope>) {
        if let Some(u8air) = self.u8air.as_ref() {
            u8air.drain_inputs();
        }
        if let Some(u16air) = self.u16air.as_ref() {
            u16air.drain_inputs();
        }
        if let Some(specified_ranges) = self.specified_ranges.as_ref() {
            specified_ranges.drain_inputs();
        }
    }
}

impl<F: PrimeField> WitnessComponent<F> for StdRangeCheck<F> {
    fn start_proof(&self, pctx: Arc<ProofCtx<F>>, _ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        self.decide(sctx, pctx);
    }

    fn calculate_witness(
        &self,
        _stage: u32,
        _air_instance: Option<usize>,
        _pctx: Arc<ProofCtx<F>>,
        _ectx: Arc<ExecutionCtx>,
        _sctx: Arc<SetupCtx>,
    ) {
        // Nothing to do
    }
}
