use core::panic;
use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
};

use num_bigint::BigInt;
use p3_field::PrimeField;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx, StdMode, ModeName};
use proofman_hints::{get_hint_field_constant, get_hint_ids_by_name, HintFieldOptions, HintFieldValue};
use rayon::Scope;

use crate::{Decider, Range, SpecifiedRanges, U16Air, U8Air};

const BYTE: u8 = 255;
const TWOBYTES: u16 = 65535;

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

impl<F: PrimeField> Decider<F> for StdRangeCheck<F> {
    fn decide(&self, sctx: Arc<SetupCtx<F>>, pctx: Arc<ProofCtx<F>>) {
        // Scan the pilout for airs that have rc-related hints
        let air_groups = pctx.pilout.air_groups();

        air_groups.iter().for_each(|air_group| {
            let airs = air_group.airs();
            airs.iter().for_each(|air| {
                let airgroup_id = air.airgroup_id;
                let air_id = air.air_id;
                let setup = sctx.get_setup(airgroup_id, air_id);

                // Obtain info from the range hints
                let rc_hints = get_hint_ids_by_name(setup.p_setup.p_expressions_bin, "range_def");
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

    pub fn new(mode: StdMode, wcm: Arc<WitnessManager<F>>) -> Arc<Self> {
        let sctx = wcm.get_sctx();

        // Scan global hints to know which airs are associated with the range check
        let u8air_hint = get_hint_ids_by_name(sctx.get_global_bin(), "u8air");
        let u16air_hint = get_hint_ids_by_name(sctx.get_global_bin(), "u16air");
        let specified_ranges_hint = get_hint_ids_by_name(sctx.get_global_bin(), "specified_ranges");

        let u8air = if !u8air_hint.is_empty() { Some(U8Air::new(wcm.clone())) } else { None };
        let u16air = if !u16air_hint.is_empty() { Some(U16Air::new(wcm.clone())) } else { None };
        let specified_ranges =
            if !specified_ranges_hint.is_empty() { Some(SpecifiedRanges::new(wcm.clone())) } else { None };

        let std_range_check = Arc::new(Self { mode, ranges: Mutex::new(Vec::new()), u8air, u16air, specified_ranges });

        wcm.register_component(std_range_check.clone(), None, None);

        std_range_check
    }

    fn register_range(&self, sctx: Arc<SetupCtx<F>>, airgroup_id: usize, air_id: usize, hint: u64) {
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

        let range = Range(min, max, min_neg, max_neg, predefined);

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

        let range = StdRangeItem { rc_type: r#type, range };

        // Update ranges
        ranges.push(range);
    }

    pub fn get_range(&self, min: BigInt, max: BigInt, predefined: Option<bool>) -> usize {
        // Default predefined value in STD is true
        let predefined = predefined.unwrap_or(true);

        let ranges = self.ranges.lock().unwrap();
        if let Some((id, _)) =
            ranges.iter().enumerate().find(|(_, r)| r.range == (predefined, min.clone(), max.clone()))
        {
            id
        } else {
            // If the range was not computed in the setup phase, error
            let name = if predefined { "Predefined" } else { "Specified" };
            log::error!("{name} range not found: [min,max] = [{},{}]", min, max);
            panic!();
        }
    }

    pub fn assign_values(&self, value: F, multiplicity: F, id: usize) {
        let ranges = self.ranges.lock().unwrap();
        let range_item = ranges.get(id);

        if range_item.is_none() {
            log::error!("Range with id {} not found", id);
            panic!();
        }

        let range_item = range_item.unwrap();
        let range = range_item.range;

        if self.mode.name == ModeName::Debug && !range.contains(value) {
            log::error!("Value {} is not in the range [min,max] = {}", value, range);
            panic!();
        }

        match range_item.rc_type {
            StdRangeCheckType::Valid(RangeCheckAir::U8Air) => {
                self.u8air.as_ref().unwrap().update_inputs(value, multiplicity);
            }
            StdRangeCheckType::Valid(RangeCheckAir::U16Air) => {
                self.u16air.as_ref().unwrap().update_inputs(value, multiplicity);
            }
            StdRangeCheckType::U8AirDouble => {
                self.u8air.as_ref().unwrap().update_inputs(value - range.0, multiplicity);
                self.u8air.as_ref().unwrap().update_inputs(range.1 - value, multiplicity);
            }
            StdRangeCheckType::U16AirDouble => {
                self.u16air.as_ref().unwrap().update_inputs(value - range.0, multiplicity);
                self.u16air.as_ref().unwrap().update_inputs(range.1 - value, multiplicity);
            }
            StdRangeCheckType::Valid(RangeCheckAir::SpecifiedRanges) => {
                self.specified_ranges.as_ref().unwrap().update_inputs(value, range, multiplicity);
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
    fn start_proof(&self, pctx: Arc<ProofCtx<F>>, _ectx: Arc<ExecutionCtx<F>>, sctx: Arc<SetupCtx<F>>) {
        self.decide(sctx, pctx);
    }

    fn calculate_witness(
        &self,
        _stage: u32,
        _air_instance: Option<usize>,
        _pctx: Arc<ProofCtx<F>>,
        _ectx: Arc<ExecutionCtx<F>>,
        _sctx: Arc<SetupCtx<F>>,
    ) {
        // Nothing to do
    }
}
