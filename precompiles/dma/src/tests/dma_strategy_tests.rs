//! Unit tests for the DMA air-selection strategy.
//! Declared from `dma_strategy.rs` via `#[cfg(test)] #[path = …] mod tests;`, so it stays a child
//! module of `dma_strategy` and keeps `super::` access to privates.

use super::*;
use proofman_fields::Goldilocks;

type Strategy = DmaStrategy<Goldilocks>;

/// Rows one instance of each 64-bit-aligned air holds, in [`air`] order.
fn caps() -> [usize; air::COUNT] {
    Strategy::dma_64_aligned_airs().map(|choice| choice.rows as usize)
}

/// Builds the counters slice the 64-bit-aligned strategy expects, indexed by `DMA_COUNTER_*`.
///
/// The packed counts default to an eighth of the plain ones, which is the ratio the packed airs
/// actually achieve (`op_x_row: 8`).
fn rows_of(
    memcpy: usize,
    memset: usize,
    memcmp: usize,
    inputcpy: usize,
) -> [usize; DMA_COUNTER_OPS_EXT] {
    let mut rows = [0usize; DMA_COUNTER_OPS_EXT];
    rows[DMA_COUNTER_MEMCPY] = memcpy;
    rows[DMA_COUNTER_MEMSET] = memset;
    rows[DMA_COUNTER_MEMCMP] = memcmp;
    rows[DMA_COUNTER_INPUTCPY] = inputcpy;
    rows[DMA_COUNTER_MEMCPY_8] = memcpy.div_ceil(8);
    rows[DMA_COUNTER_MEMSET_8] = memset.div_ceil(8);
    rows
}

fn plan(rows: &[usize]) -> Dma64AlignedInstances {
    let mut info = Dma64AlignedInstances::default();
    Strategy::calculate_dma_64_alignment_strategy(rows, &mut info);
    info
}

/// Every air a kind was routed to must be able to prove it, and the rows routed to each air must fit
/// in the instances the strategy asked for. This is exactly what `DmaInstancesBuilder` panics on when
/// the strategy over-promises.
fn assert_fits(rows: &[usize], info: &Dma64AlignedInstances) {
    let caps = caps();
    let mut routed = [0usize; air::COUNT];

    for (kind, &target) in info.assignment.iter().enumerate() {
        let (plain, packed) = match kind {
            kind::MEMCPY => (rows[DMA_COUNTER_MEMCPY], Some(rows[DMA_COUNTER_MEMCPY_8])),
            kind::MEMSET => (rows[DMA_COUNTER_MEMSET], Some(rows[DMA_COUNTER_MEMSET_8])),
            kind::MEMCMP => (rows[DMA_COUNTER_MEMCMP], None),
            kind::INPUTCPY => (rows[DMA_COUNTER_INPUTCPY], None),
            _ => unreachable!(),
        };
        if plain == 0 {
            continue;
        }

        match (kind, target) {
            (kind::MEMCPY, air::MEMCPY) => routed[target] += packed.unwrap(),
            (kind::MEMSET, air::MEMSET) => routed[target] += packed.unwrap(),
            (_, air::MEMCPY | air::MEMSET) => {
                std::panic!("kind {kind} routed to the packed air {target}, cannot prove it")
            }
            (kind::INPUTCPY, air::MEM | air::MEM_LARGE) => {
                std::panic!("an input copy was routed to a mem air, which cannot prove it")
            }
            _ => routed[target] += plain,
        }
    }

    for (a, &rows_in_air) in routed.iter().enumerate() {
        assert!(
            rows_in_air <= info.instances[a] * caps[a],
            "air {a} overflows: {rows_in_air} rows in {} instances of {}: {rows:?} → {info:?}",
            info.instances[a],
            caps[a],
        );
    }
}

fn total_instances(info: &Dma64AlignedInstances) -> usize {
    info.instances.iter().sum()
}

#[test]
fn no_rows_plans_nothing() {
    let rows = rows_of(0, 0, 0, 0);
    let info = plan(&rows);
    assert_fits(&rows, &info);
    assert_eq!(total_instances(&info), 0);
}

/// The criterion's headline: work that would need several short instances is given the tall air, even
/// though the area is no smaller.
#[test]
fn a_big_memcmp_goes_to_the_tall_air() {
    let caps = caps();
    // More than the short general air holds, but within the tall one.
    let rows = rows_of(0, 0, caps[air::FULL] + 1, 0);
    let info = plan(&rows);
    assert_fits(&rows, &info);

    assert_eq!(total_instances(&info), 1, "one tall instance beats two short ones");
    assert!(caps[air::FULL_LARGE] > caps[air::FULL]);
}

/// Once one instance is enough either way, the area tie-break sends the work to the narrowest,
/// shortest air that can prove it.
#[test]
fn area_breaks_the_tie_for_a_small_workload() {
    let rows = rows_of(0, 0, 1000, 0);
    let info = plan(&rows);
    assert_fits(&rows, &info);

    assert_eq!(total_instances(&info), 1);
    assert_eq!(
        info.assignment[kind::MEMCMP],
        air::MEM,
        "the narrow mem air is the cheapest home for a memcmp"
    );
}

/// Kinds that fit together share an instance rather than taking one each — which is the packing that
/// the instance-first criterion is there to find.
#[test]
fn kinds_share_an_instance_rather_than_opening_two() {
    let rows = rows_of(0, 0, 1000, 1000);
    let info = plan(&rows);
    assert_fits(&rows, &info);

    assert_eq!(total_instances(&info), 1, "memcmp rides with the input copy in the general air");
    assert_eq!(info.assignment[kind::MEMCMP], info.assignment[kind::INPUTCPY]);
}

/// An input copy has no specialised air, so it can only go to a general one however small it is.
#[test]
fn an_input_copy_only_ever_goes_to_a_general_air() {
    for inputcpy in [1, 1000, 10_000_000] {
        let rows = rows_of(0, 0, 0, inputcpy);
        let info = plan(&rows);
        assert_fits(&rows, &info);
        assert!(matches!(info.assignment[kind::INPUTCPY], air::FULL | air::FULL_LARGE));
    }
}

/// The packed airs hold eight operations per row, so a memcpy big enough to need several general
/// instances fits in one packed instance — fewer instances *and* less area.
#[test]
fn a_big_memcpy_takes_the_packed_air() {
    let caps = caps();
    let rows = rows_of(4 * caps[air::MEMCPY], 0, 0, 0);
    let info = plan(&rows);
    assert_fits(&rows, &info);

    assert_eq!(info.assignment[kind::MEMCPY], air::MEMCPY);
    assert_eq!(total_instances(&info), 1);
}

/// The rows each kind owes its air are what the per-chunk hand-out draws down, so they must be
/// counted in that air's own row cost — the packed count for a packed air, the plain one otherwise.
#[test]
fn the_owed_rows_are_counted_in_the_chosen_air_s_cost() {
    let caps = caps();
    let rows = rows_of(4 * caps[air::MEMCPY], 0, 0, 0);
    let info = plan(&rows);
    assert_eq!(info.assignment[kind::MEMCPY], air::MEMCPY);
    assert_eq!(info.rows[kind::MEMCPY], rows[DMA_COUNTER_MEMCPY_8]);

    let rows = rows_of(0, 0, 1000, 0);
    let info = plan(&rows);
    assert_eq!(info.rows[kind::MEMCMP], 1000);
}

/// Whatever the mix, the granted instances must hold everything routed to them — the invariant the
/// builders assert at hand-out time.
#[test]
fn capacity_invariant_holds_around_instance_boundaries() {
    let caps = caps();
    let values = [
        0,
        1,
        caps[air::MEM] - 1,
        caps[air::MEM],
        caps[air::MEM] + 1,
        caps[air::FULL_LARGE],
        caps[air::FULL_LARGE] + 1,
    ];
    for memcpy in values {
        for memset in values {
            for memcmp in values {
                for inputcpy in values {
                    let rows = rows_of(memcpy, memset, memcmp, inputcpy);
                    let info = plan(&rows);
                    assert_fits(&rows, &info);
                }
            }
        }
    }
}
