//! What computing one instance's witness costs, per air, so the pieces that place instances can
//! tell a heavy witness from a light one: the distribution, which spreads the witness load across
//! the workers of a job, and the executor, which has the heavy witnesses admitted first.
//!
//! One figure per air, in **milliseconds of one witness computation on a cluster worker**. Like
//! `air_costs`, this is meant to be edited or regenerated, not derived: the numbers are measured,
//! and whoever changes a witness (see the `Mem` fill) refreshes its figure.
//!
//! # Where the numbers come from
//!
//! The per-instance witness report of the workers -- the `··· Witness <air> #id | fill | took ms`
//! lines -- over the 779 mainnet blocks of the 2026-09-13 run, taking each air's median. The
//! ranking is what the placement uses, so a figure only has to be right relative to the others:
//! a `Mem` at 300 ms and a `BinaryExtension` at 70 ms tell the distribution not to put the `Mem`
//! where the other heavy witnesses already are, whatever the exact milliseconds on a given machine.
//!
//! `Mem`, `InputData` and `CompactMem` are set below what the 2026-09-14 run measured (328, 115
//! and 776 ms) because `zisk.pil` then halved their lanes per row (Mem 8 -> 4, InputData 4 -> 2,
//! and `CompactMem` with them): an instance holds half the operations and half the columns, so
//! both the fill and the padding halve. Refresh them from the first run with the new heights. The
//! fused `Compact*` airs and the airs no block has used are estimated from their siblings, and
//! anything not listed gets [`WITNESS_COST_DEFAULT_MS`].
//!
//! # How the figures are used
//!
//! * The distribution (`pil2-proofman`, `DistributionCtx::witness_slack`): among the workers whose
//!   proof cost is within [`WITNESS_BALANCE_SLACK`] of the least loaded one, an instance goes to
//!   the one with the least witness load. Proof cost still decides what each worker proves.
//! * The executor: an instance whose witness costs [`WITNESS_HEAVY_MS`] or more is reported ready
//!   on the priority channel, so when more witnesses are ready than the worker computes at once
//!   the long ones start first and the short ones fill in behind them.

use crate::AIR_NAMES;

/// Witness cost of an air that has no figure of its own.
pub const WITNESS_COST_DEFAULT_MS: u64 = 60;

/// From this cost up an instance's witness is admitted with priority.
pub const WITNESS_HEAVY_MS: u64 = 150;

/// Proof cost the distribution gives up, as a fraction of the least loaded worker's, to place an
/// instance on the worker with the lighter witness load. 5% is about half an instance's proof
/// cost on a typical 4-worker job.
pub const WITNESS_BALANCE_SLACK: f64 = 0.05;

/// Milliseconds one witness computation of the air takes, by air name.
pub const WITNESS_COSTS_MS: &[(&str, u64)] = &[
    // Measured (2026-09-13 run, median of the per-instance witness report)
    ("Keccakf", 420),
    ("ArithEqHuge", 520),
    ("ArithEq384Huge", 480),
    ("BinaryHuge", 380),
    ("Mem", 170),
    ("ArithEqLarge", 300),
    ("ArithEq384Large", 260),
    ("BinaryLarge", 230),
    ("Dma64AlignedMemCpy", 230),
    ("Main", 180),
    ("ArithEq384", 180),
    ("MemAlignLarge", 160),
    ("ArithEq", 150),
    ("Dma64AlignedLarge", 140),
    ("InputData", 70),
    ("Binary", 95),
    ("MemAlignByteLarge", 90),
    ("Dma64Aligned", 80),
    ("BinaryAddHiHuge", 80),
    ("JumpDest", 70),
    ("DmaUnaligned", 70),
    ("BinaryExtension", 70),
    ("Arith", 55),
    ("RomData", 55),
    ("MemAlignReadByteLarge", 50),
    ("BinaryAddHiLarge", 50),
    ("DmaWithPrePost", 50),
    ("BinaryAddHuge", 50),
    ("MemAlign", 45),
    ("MemAlignReadByte", 35),
    ("BinaryAddHi", 30),
    ("BinaryAdd", 25),
    ("MemAlignWriteByte", 25),
    ("Sha256f", 5),
    ("Rom", 1),
    // Estimated from siblings of the same height
    ("ArithBn254Large", 520),
    ("ArithSecp256K1Large", 520),
    ("Arith256XLarge", 520),
    ("ArithBn254", 130),
    ("ArithSecp256K1", 130),
    ("Arith256X", 130),
    ("CompactMem", 260),
    ("CompactMemAlignLarge", 180),
    ("CompactMemAlign", 90),
    ("BinaryExtensionLarge", 140),
    ("BinaryAddLarge", 25),
    ("DmaUnalignedLarge", 140),
    ("Dma64AlignedMemLarge", 140),
    ("Dma64AlignedMem", 100),
    ("Dma64AlignedMemSet", 100),
    ("Dma", 50),
    ("DmaPrePost", 50),
    ("MemAlignByte", 45),
    ("Blake3f", 200),
    ("Blake2br", 60),
    ("Poseidon", 60),
    ("BabyJubJub", 60),
    ("Add256", 60),
    ("VirtualTableZisk0", 30),
    ("VirtualTableZisk1", 30),
];

/// The witness cost of an air by name; [`WITNESS_COST_DEFAULT_MS`] for a name not listed.
pub fn witness_cost_ms_by_name(air_name: &str) -> u64 {
    WITNESS_COSTS_MS
        .iter()
        .find(|(name, _)| *name == air_name)
        .map_or(WITNESS_COST_DEFAULT_MS, |(_, cost)| *cost)
}

/// The witness cost of an air by id, through the generated air names.
pub fn witness_cost_ms(airgroup_id: usize, air_id: usize) -> u64 {
    AIR_NAMES
        .iter()
        .find(|&&(group, air, _)| group == airgroup_id && air == air_id)
        .map_or(WITNESS_COST_DEFAULT_MS, |&(_, _, name)| witness_cost_ms_by_name(name))
}

/// Whether an instance of the air is admitted with priority (see [`WITNESS_HEAVY_MS`]).
pub fn is_heavy_witness(airgroup_id: usize, air_id: usize) -> bool {
    witness_cost_ms(airgroup_id, air_id) >= WITNESS_HEAVY_MS
}

/// The witness cost of every air of the pilout, keyed by `(airgroup_id, air_id)`: what the
/// distribution is handed.
pub fn witness_costs_ms() -> Vec<((usize, usize), u64)> {
    AIR_NAMES
        .iter()
        .map(|&(group, air, name)| ((group, air), witness_cost_ms_by_name(name)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A name in the table that no air carries is a typo or a renamed air: the figure would be
    /// silently lost.
    #[test]
    fn every_listed_air_exists_in_the_pilout() {
        for (name, _) in WITNESS_COSTS_MS {
            assert!(
                AIR_NAMES.iter().any(|&(_, _, air)| air == *name),
                "witness cost listed for `{name}`, which is not an air of the pilout"
            );
        }
    }

    #[test]
    fn no_air_is_listed_twice() {
        for (i, (name, _)) in WITNESS_COSTS_MS.iter().enumerate() {
            assert!(
                !WITNESS_COSTS_MS[..i].iter().any(|(other, _)| other == name),
                "`{name}` is listed twice"
            );
        }
    }

    /// The ranking the placement relies on: the long witnesses are the heavy ones, the short ones
    /// are not, and an unknown air falls back to the default rather than to zero.
    #[test]
    fn heavy_and_light_airs_rank_as_measured() {
        let id = |name: &str| {
            AIR_NAMES.iter().find(|&&(_, _, air)| air == name).map(|&(g, a, _)| (g, a)).unwrap()
        };
        for name in ["Mem", "Keccakf", "BinaryHuge", "ArithEqLarge"] {
            let (g, a) = id(name);
            assert!(is_heavy_witness(g, a), "{name} must be a heavy witness");
        }
        for name in ["Sha256f", "Arith", "BinaryExtension", "RomData"] {
            let (g, a) = id(name);
            assert!(!is_heavy_witness(g, a), "{name} must not be a heavy witness");
        }
        assert_eq!(witness_cost_ms(usize::MAX, usize::MAX), WITNESS_COST_DEFAULT_MS);
        assert_eq!(witness_costs_ms().len(), AIR_NAMES.len());
    }
}
