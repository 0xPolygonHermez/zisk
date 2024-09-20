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

type SumAirsItem = (usize, usize, Vec<u64>, Vec<u64>);
pub struct StdSum<F> {
    _phantom: std::marker::PhantomData<F>,
    sum_airs: Mutex<Vec<SumAirsItem>>, // (airgroup_id, air_id, gsum_hints, im_hints)
}

impl<F: Copy + Debug + Field> Decider<F> for StdSum<F> {
    fn decide(&self, sctx: Arc<SetupCtx>, pctx: Arc<ProofCtx<F>>) {
        // Scan the pilout for airs that have sum-related hints
        let air_groups = pctx.pilout.air_groups();
        let mut sum_airs_guard = self.sum_airs.lock().unwrap();
        air_groups.iter().for_each(|air_group| {
            let airs = air_group.airs();
            airs.iter().for_each(|air| {
                let airgroup_id = air.airgroup_id;
                let air_id = air.air_id;
                let setup = sctx.setups.get_setup(airgroup_id, air_id).expect("REASON");
                let im_hints = get_hint_ids_by_name(*setup.p_setup, "im_col");
                let gsum_hints = get_hint_ids_by_name(*setup.p_setup, "gsum_col");
                if !gsum_hints.is_empty() {
                    // Save the air for latter witness computation
                    sum_airs_guard.push((airgroup_id, air_id, im_hints, gsum_hints));
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
        pctx: Arc<ProofCtx<F>>,
        sctx: Arc<SetupCtx>,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        if stage == 2 {
            let sum_airs = self.sum_airs.lock().unwrap();
            sum_airs
                .iter()
                .for_each(|(airgroup_id, air_id, im_hints, gsum_hints)| {
                    let air_instances = pctx
                        .air_instance_repo
                        .find_air_instances(*airgroup_id, *air_id);
                    air_instances.iter().for_each(|air_instance_id| {
                        let air_instaces_vec =
                            &mut pctx.air_instance_repo.air_instances.write().unwrap();

                        let air_instance = &mut air_instaces_vec[*air_instance_id];

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

                        // Populate the im columns
                        for hint in im_hints {
                            let mut im = get_hint_field::<F>(
                                sctx.setups.as_ref(),
                                &pctx.public_inputs,
                                &pctx.challenges,
                                air_instance,
                                *hint as usize,
                                "reference",
                                HintFieldOptions::dest(),
                            );
                            let num = get_hint_field::<F>(
                                sctx.setups.as_ref(),
                                &pctx.public_inputs,
                                &pctx.challenges,
                                air_instance,
                                *hint as usize,
                                "numerator",
                                HintFieldOptions::default(),
                            );
                            let den = get_hint_field::<F>(
                                sctx.setups.as_ref(),
                                &pctx.public_inputs,
                                &pctx.challenges,
                                air_instance,
                                *hint as usize,
                                "denominator",
                                HintFieldOptions::default(),
                            );

                            for i in 0..num_rows {
                                // TODO: We should perform the following division in batch using div_lib
                                im.set(i, num.get(i) / den.get(i));
                            }
                            set_hint_field(
                                sctx.setups.as_ref(),
                                air_instance,
                                *hint,
                                "reference",
                                &im,
                            );
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
                            sctx.setups.as_ref(),
                            &pctx.public_inputs,
                            &pctx.challenges,
                            air_instance,
                            gsum_hint,
                            "reference",
                            HintFieldOptions::dest(),
                        );
                        let expr = get_hint_field::<F>(
                            sctx.setups.as_ref(),
                            &pctx.public_inputs,
                            &pctx.challenges,
                            air_instance,
                            gsum_hint,
                            "expression",
                            HintFieldOptions::default(),
                        );

                        gsum.set(0, expr.get(0));
                        for i in 1..num_rows {
                            // TODO: We should perform the following division in batch using div_lib
                            gsum.set(i, gsum.get(i - 1) + expr.get(i));
                        }

                        // set the computed gsum column and its associated airgroup_val
                        set_hint_field(
                            sctx.setups.as_ref(),
                            air_instance,
                            gsum_hint as u64,
                            "reference",
                            &gsum,
                        );
                        set_hint_field_val(
                            sctx.clone(),
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
