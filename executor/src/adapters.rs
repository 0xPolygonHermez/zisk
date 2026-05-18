//! Concrete adapters bridging external library types to the executor's
//! port traits in [`crate::ports`].
//!
//! Today the only adapter is [`ProofmanAdapter`] wrapping
//! `proofman_common::ProofCtx<F>`. If a second adapter (e.g. a
//! remote-proof backend) appears, split this file into a directory
//! module.

use anyhow::Result;
use fields::PrimeField64;
use proofman_common::ProofCtx;
use zisk_pil::ZiskPublicValues;

use crate::ports::{Dctx, GlobalId, InstanceInfo, ProofRegistry};

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
}

impl<F: PrimeField64> Dctx for ProofmanAdapter<'_, F> {
    fn instance_info(&self, gid: GlobalId) -> Result<InstanceInfo> {
        let (airgroup_id, air_id) = self.pctx.dctx_get_instance_info(gid.0)?;
        Ok(InstanceInfo::new(airgroup_id, air_id))
    }

    fn is_my_process_instance(&self, gid: GlobalId) -> Result<bool> {
        Ok(self.pctx.dctx_is_my_process_instance(gid.0)?)
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
