use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
};

use p3_field::Field;
use proofman_common::{ProofCtx, SetupCtx};
use proofman_hints::{get_hint_field, get_hint_ids_by_name, set_hint_field, set_hint_field_val};

use crate::Decider;

pub struct StdProd<F> {
    _phantom: std::marker::PhantomData<F>,
    prod_airs: Mutex<Vec<(usize, usize, Vec<u64>)>>, // (airgroup_id, air_id, prod_hints)
}

impl<F: Copy + Debug + Field> Decider<F> for StdProd<F> {
    fn decide(&self, sctx: &SetupCtx, pctx: &ProofCtx<F>) {
        // Scan the pilout for airs that have prod-related hints
        let air_groups = pctx.pilout.air_groups();
        air_groups.iter().for_each(|air_group| {
            let airs = air_group.airs();
            airs.iter().for_each(|air| {
                let airgroup_id = air.airgroup_id;
                let air_id = air.air_id;
                let setup = sctx.get_setup(airgroup_id, air_id).expect("REASON");
                let prod_hints = get_hint_ids_by_name(setup.p_setup, "gprod_col");
                if !prod_hints.is_empty() {
                    // Save the air for latter witness computation
                    self.prod_airs
                        .lock()
                        .unwrap()
                        .push((airgroup_id, air_id, prod_hints));
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
        pctx: &ProofCtx<F>,
        sctx: &SetupCtx,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        if stage == 2 {
            let prod_airs = self.prod_airs.lock().unwrap();
            prod_airs
                .iter()
                .for_each(|(airgroup_id, air_id, prod_hints)| {
                    let air_instances = pctx.find_air_instances(*airgroup_id, *air_id);
                    air_instances.iter().for_each(|air_instance_id| {
                        let air_instances_vec = &mut pctx.air_instances.write().unwrap();

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
                            sctx,
                            air_instance,
                            gprod_hint,
                            "reference",
                            true,
                            false,
                            false,
                        );
                        let num = get_hint_field::<F>(
                            sctx,
                            air_instance,
                            gprod_hint,
                            "numerator",
                            false,
                            false,
                            false,
                        );
                        let den = get_hint_field::<F>(
                            sctx,
                            air_instance,
                            gprod_hint,
                            "denominator",
                            false,
                            false,
                            false,
                        );

                        gprod.set(0, num.get(0) / den.get(0));
                        for i in 1..num_rows {
                            gprod.set(i, gprod.get(i - 1) * (num.get(i) / den.get(i)));
                        }

                        println!("gprod: {:?}", gprod.get(num_rows - 1));

                        // set the computed gprod column and its associated airgroup_val
                        set_hint_field(sctx, air_instance, gprod_hint as u64, "reference", &gprod);
                        set_hint_field_val(
                            sctx,
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
