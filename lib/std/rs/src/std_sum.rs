use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
};

use p3_field::Field;
use proofman_common::{ProofCtx, SetupCtx};
use proofman_hints::{get_hint_field, get_hint_ids_by_name, set_hint_field, set_hint_field_val};

use crate::Decider;

pub struct StdSum<F> {
    _phantom: std::marker::PhantomData<F>,
    sum_airs: Mutex<Vec<(usize, usize, Vec<u64>, Vec<u64>)>>, // (air_group_id, air_id, gsum_hints, im_hints)
}

impl<F: Copy + Debug + Field> Decider<F> for StdSum<F> {
    fn decide(&self, sctx: &SetupCtx, pctx: &ProofCtx<F>) {
        // Scan the pilout for airs that have sum-related hints
        let air_groups = pctx.pilout.air_groups();
        air_groups.iter().for_each(|air_group| {
            let airs = air_group.airs();
            airs.iter().for_each(|air| {
                let air_group_id = air.air_group_id;
                let air_id = air.air_id;
                let setup = sctx.get_setup(air_group_id, air_id).expect("REASON");
                let im_hints = get_hint_ids_by_name(setup.p_setup, "im_col");
                let gsum_hints = get_hint_ids_by_name(setup.p_setup, "gsum_col");
                if !gsum_hints.is_empty() {
                    // Save the air for latter witness computation
                    self.sum_airs.lock().unwrap().push((
                        air_group_id,
                        air_id,
                        im_hints,
                        gsum_hints,
                    ));
                }
            });
        });
    }
}

impl<F: Copy + Debug + Field> StdSum<F> {
    const MY_NAME: &'static str = "STD Sum";

    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            _phantom: std::marker::PhantomData,
            sum_airs: Mutex::new(Vec::new()),
        })
    }

    pub fn calculate_witness(
        &self,
        stage: u32,
        pctx: &ProofCtx<F>,
        sctx: &SetupCtx,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        if stage == 2 {
            let sum_airs = self.sum_airs.lock().unwrap();
            sum_airs
                .iter()
                .for_each(|(air_group_id, air_id, im_hints, gsum_hints)| {
                    let air_instances = pctx.find_air_instances(*air_group_id, *air_id);
                    air_instances.iter().for_each(|air_instance_id| {
                        let air_instaces_vec = &mut pctx.air_instances.write().unwrap();

                        let air_instance = &mut air_instaces_vec[*air_instance_id];

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

                        // Populate the im columns
                        for hint in im_hints {
                            let mut im = get_hint_field::<F>(
                                sctx,
                                air_instance,
                                *hint as usize,
                                "reference",
                                true,
                                false,
                                false,
                            );
                            let num = get_hint_field::<F>(
                                sctx,
                                air_instance,
                                *hint as usize,
                                "numerator",
                                false,
                                false,
                                false,
                            );
                            let den = get_hint_field::<F>(
                                sctx,
                                air_instance,
                                *hint as usize,
                                "denominator",
                                false,
                                false,
                                false,
                            );

                            for i in 0..num_rows {
                                // TODO: We should perform the following division in batch using div_lib
                                im.set(i, num.get(i) / den.get(i));
                            }
                            set_hint_field(sctx, air_instance, *hint as u64, "reference", &im);
                        }

                        // We know that at most one product hint exists
                        let gsum_hint = if gsum_hints.len() > 1 {
                            panic!(
                                "Multiple product hints found for AIR '{}'",
                                air.name().unwrap_or("unknown")
                            );
                        } else {
                            gsum_hints[0] as usize
                        };

                        // Use the hint to populate the gsum column
                        let mut gsum = get_hint_field::<F>(
                            sctx,
                            air_instance,
                            gsum_hint,
                            "reference",
                            true,
                            false,
                            false,
                        );
                        let expr = get_hint_field::<F>(
                            sctx,
                            air_instance,
                            gsum_hint,
                            "expression",
                            false,
                            false,
                            false,
                        );

                        gsum.set(0, expr.get(0));
                        for i in 1..num_rows {
                            // TODO: We should perform the following division in batch using div_lib
                            gsum.set(i, gsum.get(i - 1) + expr.get(i));
                        }

                        println!("gsum: {:?}", gsum.get(num_rows - 1));
                        // set the computed gsum column and its associated airgroup_val
                        set_hint_field(sctx, air_instance, gsum_hint as u64, "reference", &gsum);
                        set_hint_field_val(
                            sctx,
                            air_instance,
                            gsum_hint as u64,
                            "result",
                            gsum.get(num_rows - 1),
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
