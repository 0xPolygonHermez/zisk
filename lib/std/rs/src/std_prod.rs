use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
};

use p3_field::Field;
use proofman_common::{ProofCtx, SetupCtx};
use proofman_hints::{
    get_hint_field, get_hint_ids_by_name, set_hint_field, set_hint_field_val, HintFieldOptions,
};

use crate::Decider;

pub struct StdProd<F> {
    _phantom: std::marker::PhantomData<F>,
    prod_airs: Mutex<Vec<(usize, usize, Vec<u64>)>>, // (airgroup_id, air_id, prod_hints)
}

impl<F: Copy + Debug + Field> Decider<F> for StdProd<F> {
    fn decide(&self, sctx: Arc<SetupCtx>, pctx: Arc<ProofCtx<F>>) {
        // Scan the pilout for airs that have prod-related hints
        let air_groups = pctx.pilout.air_groups();
        let mut prod_airs_guard = self.prod_airs.lock().unwrap();
        air_groups.iter().for_each(|air_group| {
            let airs = air_group.airs();
            airs.iter().for_each(|air| {
                let airgroup_id = air.airgroup_id;
                let air_id = air.air_id;
                let setup = sctx.setups.get_setup(airgroup_id, air_id).expect("REASON");
                let prod_hints = get_hint_ids_by_name(*setup.p_setup, "gprod_col");
                if !prod_hints.is_empty() {
                    // Save the air for latter witness computation
                    prod_airs_guard.push((airgroup_id, air_id, prod_hints));
                }
            });
        });
    }
}

impl<F: Copy + Debug + Field> StdProd<F> {
    const MY_NAME: &'static str = "STD Prod";

    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            _phantom: std::marker::PhantomData,
            prod_airs: Mutex::new(Vec::new()),
        })
    }

    pub fn calculate_witness(
        &self,
        stage: u32,
        pctx: Arc<ProofCtx<F>>,
        sctx: Arc<SetupCtx>,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        if stage == 2 {
            let prod_airs = self.prod_airs.lock().unwrap();
            prod_airs
                .iter()
                .for_each(|(airgroup_id, air_id, prod_hints)| {
                    let air_instances = pctx
                        .air_instance_repo
                        .find_air_instances(*airgroup_id, *air_id);
                    air_instances.iter().for_each(|air_instance_id| {
                        let air_instances_vec =
                            &mut pctx.air_instance_repo.air_instances.write().unwrap();

                        let air_instance = &mut air_instances_vec[*air_instance_id];

                        // Get the air associated with the air_instance
                        let airgroup_id = air_instance.airgroup_id;
                        let air_id = air_instance.air_id;
                        let air = pctx.pilout.get_air(airgroup_id, air_id);

                        log::info!(
                            "{}: Initiating witness computation for AIR '{}' at stage {}",
                            Self::MY_NAME,
                            air.name().unwrap_or("unknown"),
                            stage
                        );

                        let num_rows = air.num_rows();

                        // We know that at most one product hint exists
                        let gprod_hint = if prod_hints.len() > 1 {
                            panic!(
                                "Multiple product hints found for AIR '{}'",
                                air.name().unwrap_or("unknown")
                            );
                        } else {
                            prod_hints[0] as usize
                        };

                        // Use the hint to populate the gprod column
                        let mut gprod = get_hint_field::<F>(
                            sctx.setups.as_ref(),
                            &pctx.public_inputs,
                            &pctx.challenges,
                            air_instance,
                            gprod_hint,
                            "reference",
                            HintFieldOptions::dest(),
                        );
                        let num = get_hint_field::<F>(
                            sctx.setups.as_ref(),
                            &pctx.public_inputs,
                            &pctx.challenges,
                            air_instance,
                            gprod_hint,
                            "numerator",
                            HintFieldOptions::default(),
                        );
                        let den = get_hint_field::<F>(
                            sctx.setups.as_ref(),
                            &pctx.public_inputs,
                            &pctx.challenges,
                            air_instance,
                            gprod_hint,
                            "denominator",
                            HintFieldOptions::default(),
                        );

                        gprod.set(0, num.get(0) / den.get(0));
                        for i in 1..num_rows {
                            gprod.set(i, gprod.get(i - 1) * (num.get(i) / den.get(i)));
                        }

                        // set the computed gprod column and its associated airgroup_val
                        set_hint_field(
                            sctx.setups.as_ref(),
                            air_instance,
                            gprod_hint as u64,
                            "reference",
                            &gprod,
                        );
                        set_hint_field_val(
                            sctx.clone(),
                            air_instance,
                            gprod_hint as u64,
                            "result",
                            gprod.get(num_rows - 1),
                        );

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
