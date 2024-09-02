use std::{fmt::Debug,sync::{Arc,Mutex}};

use p3_field::Field;
use proofman_common::{AirInstanceCtx, ExecutionCtx, ProofCtx};
use proofman_hints::{get_hint_field, get_hint_ids_by_name, set_hint_field, set_hint_field_val};
use proofman_setup::SetupCtx;

use crate::Decider;

pub struct StdProd<F> {
    _phantom: std::marker::PhantomData<F>,
    prod_airs: Vec<(usize, usize, Vec<u64>)>, // (air.air_group_id, air.air_id, prod_hints)
}

impl<F: Copy + Debug + Field> Decider<F> for StdProd<F> {
    fn decide(
        &mut self,
        pctx: &ProofCtx<F>,
        sctx: &SetupCtx,
    ) {
        // Scan the pilout for airs that have prod-related hints
        let air_groups = pctx.pilout.air_groups();
        air_groups.iter().for_each(|air_group| {
            let airs = air_group.airs();
            airs.iter().for_each(|air| {
                let air_group_id = air.air_group_id;
                let air_id = air.air_id;
                let setup = sctx
                    .get_setup(air_group_id, air_id)
                    .expect("REASON");
                let prod_hints = get_hint_ids_by_name(setup, "gprod_col");
                if !prod_hints.is_empty() {
                    // Save the air for latter witness computation
                    self.prod_airs.push((air.air_group_id, air.air_id, prod_hints));
                }
            });
        });
    }
}

impl<F: Copy + Debug + Field> StdProd<F> {
    const MY_NAME: &'static str = "STD Prod";

    pub fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            _phantom: std::marker::PhantomData,
            prod_airs: Vec::new(),
        }))
    }

    pub fn calculate_witness(
        &self,
        stage: u32,
        pctx: &ProofCtx<F>,
        sctx: &SetupCtx,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        if stage == 2 {
            self.prod_airs.iter().for_each(|(air_group_id, air_id, prod_hints)| {
                let air_instances = pctx.find_air_instances(*air_group_id, *air_id);
                air_instances.iter().for_each(|air_instance_id| {
                    let air_instaces_vec = pctx.air_instances.read().unwrap();

                    let air_instance = &air_instaces_vec[*air_instance_id];

                    // Get the air associated with the air_instance
                    let air_group_id = air_instance.air_group_id;
                    let air_id = air_instance.air_id;
                    let air = pctx.pilout.get_air(air_group_id, air_id);

                    log::info!(
                        "{}: Initiating witness computation for AIR '{}' at stage {}",
                        Self::MY_NAME,
                        air.name().unwrap_or("unknown"),
                        stage
                    );

                    let num_rows = air.num_rows();

                    // We know that at most one product hint exists
                    let gprod_hint = if prod_hints.len() > 1 {
                        panic!("Multiple product hints found for AIR '{}'", air.name().unwrap_or("unknown"));
                    } else {
                        prod_hints[0] as usize
                    };

                    let setup = sctx.get_setup(air_group_id, air_id).unwrap();

                    // Use the hint to populate the gprod column
                    let mut gprod = get_hint_field::<F>(setup, gprod_hint, "reference", true);
                    let num = get_hint_field::<F>(setup, gprod_hint, "numerator", false);
                    let den = get_hint_field::<F>(setup, gprod_hint, "denominator", false);

                    gprod.set(0, num.get(0) / den.get(0));
                    for i in 1..num_rows {
                        gprod.set(i, gprod.get(i - 1) * (num.get(i) / den.get(i)));
                    }
            
                    // set the computed gprod column and its associated airgroup_val
                    set_hint_field(setup, gprod_hint as u64, "reference", &gprod);
                    set_hint_field_val(setup, gprod_hint as u64, "result", gprod.get(num_rows - 1));
            
                    log::info!(
                        "{}: Completed witness computation for AIR '{}' at stage {}",
                        Self::MY_NAME,
                        air.name().unwrap_or("unknown"),
                        stage
                    );
                });
            });
        }

        Ok(0)
    }
}
