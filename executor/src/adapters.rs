//! Concrete adapters bridging external library types to the executor's
//! port traits in [`crate::ports`].
//!
//! Today the only adapter is for `proofman_common` ([`ProofmanAdapter`]
//! wrapping `ProofCtx<F>`, [`ProofmanSetupAdapter`] wrapping `SetupCtx<F>`).
//! If a second adapter (e.g. a remote-proof backend) appears, split this
//! file into a directory module.
//!
//! Step 0.3 of the executor refactor — adapters are introduced but not
//! yet used by any call site. Later steps (3.3, 4.2) flip the phases to
//! consume `&dyn ProofRegistry` / `&dyn WitnessRegistry` via these
//! adapters.

use anyhow::Result;
use fields::PrimeField64;
use proofman_common::{AirInstance, ProofCtx, SetupCtx};
use zisk_pil::ZiskPublicValues;

use crate::ports::{
    CostDims, Dctx, GlobalId, InstanceInfo, ProofRegistry, SetupAccess, WitnessRegistry,
};

/// Adapter wrapping `ProofCtx<F>` for use through the port traits.
///
/// Holds the borrow for the duration of a phase call; constructed at
/// each `WitnessComponent::execute` / `calculate_witness` entry point.
pub struct ProofmanAdapter<'a, F: PrimeField64> {
    pctx: &'a ProofCtx<F>,
}

impl<'a, F: PrimeField64> ProofmanAdapter<'a, F> {
    /// Wrap a borrowed `ProofCtx<F>`.
    #[inline]
    pub fn new(pctx: &'a ProofCtx<F>) -> Self {
        Self { pctx }
    }

    /// Returns the wrapped `ProofCtx<F>`. Provided as an escape hatch for
    /// transitional call sites still using the concrete type.
    #[inline]
    pub fn inner(&self) -> &ProofCtx<F> {
        self.pctx
    }
}

impl<F: PrimeField64> Dctx for ProofmanAdapter<'_, F> {
    fn instance_info(&self, gid: GlobalId) -> Result<InstanceInfo> {
        let (airgroup_id, air_id) = self.pctx.dctx_get_instance_info(gid.0)?;
        Ok(InstanceInfo::new(airgroup_id, air_id))
    }

    fn is_my_process_instance(&self, gid: GlobalId) -> Result<bool> {
        Ok(self.pctx.dctx_is_my_process_instance(gid.0)?)
    }

    fn is_first_process(&self) -> bool {
        self.pctx.dctx_is_first_process()
    }

    fn set_witness_ready(&self, gid: GlobalId, ready: bool) {
        self.pctx.set_witness_ready(gid.0, ready);
    }
}

impl<F: PrimeField64> ProofRegistry for ProofmanAdapter<'_, F> {
    fn add_instance(&self, info: InstanceInfo) -> Result<GlobalId> {
        Ok(GlobalId(self.pctx.add_instance(info.airgroup_id, info.air_id)?))
    }

    fn add_instance_assign(&self, info: InstanceInfo) -> Result<GlobalId> {
        Ok(GlobalId(self.pctx.add_instance_assign(info.airgroup_id, info.air_id)?))
    }

    fn add_table(&self, info: InstanceInfo) -> Result<GlobalId> {
        Ok(GlobalId(self.pctx.add_table(info.airgroup_id, info.air_id)?))
    }

    fn find_instance_id(&self, info: InstanceInfo) -> Result<GlobalId> {
        // `dctx_find_instance_id` returns `(bool, usize)`; the first field
        // is a flag the executor doesn't currently consume.
        let (_, gid) = self.pctx.dctx_find_instance_id(info.airgroup_id, info.air_id)?;
        Ok(GlobalId(gid))
    }

    fn set_chunks(&self, gid: GlobalId, chunks: &[usize], is_memory_related: bool) {
        // `dctx_set_chunks` takes `Vec<usize>` by value; the trait takes
        // `&[usize]` so the caller doesn't have to clone unnecessarily
        // when only the slice is needed conceptually.
        self.pctx.dctx_set_chunks(gid.0, chunks.to_vec(), is_memory_related);
    }

    fn write_pub_outs(&self, pub_outs: &[(u64, u32)]) {
        let mut publics = ZiskPublicValues::from_vec_guard(self.pctx.get_publics());
        for &(index, value) in pub_outs {
            publics.inputs[index as usize] = F::from_u32(value);
        }
    }
}

impl<F: PrimeField64> WitnessRegistry<F> for ProofmanAdapter<'_, F> {
    fn add_air_instance(&self, air_instance: AirInstance<F>, gid: GlobalId) {
        self.pctx.add_air_instance(air_instance, gid.0);
    }
}

/// Adapter wrapping `SetupCtx<F>` for use through [`SetupAccess`].
pub struct ProofmanSetupAdapter<'a, F: PrimeField64> {
    sctx: &'a SetupCtx<F>,
}

impl<'a, F: PrimeField64> ProofmanSetupAdapter<'a, F> {
    /// Wrap a borrowed `SetupCtx<F>`.
    #[inline]
    pub fn new(sctx: &'a SetupCtx<F>) -> Self {
        Self { sctx }
    }

    /// Returns the wrapped `SetupCtx<F>`.
    #[inline]
    pub fn inner(&self) -> &SetupCtx<F> {
        self.sctx
    }
}

impl<F: PrimeField64> SetupAccess for ProofmanSetupAdapter<'_, F> {
    fn cost_dimensions(&self, info: InstanceInfo) -> Result<CostDims> {
        let setup = self.sctx.get_setup(info.airgroup_id, info.air_id)?;
        let n_bits = setup.stark_info.stark_struct.n_bits;
        let total_cols: u64 = setup
            .stark_info
            .map_sections_n
            .iter()
            .filter(|(key, _)| *key != "const")
            .map(|(_, value)| *value)
            .sum();
        Ok(CostDims { n_bits, total_cols })
    }
}
