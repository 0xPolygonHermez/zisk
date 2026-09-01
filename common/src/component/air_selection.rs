//! Choosing which airs a family instantiates, and how many of each.
//!
//! # The criterion
//!
//! A solution is better when it needs **fewer instances**; between solutions with the same instance
//! count, the one with **less area** wins. Area is `rows × setup columns` per instance — see
//! [`zisk_pil::instance_area`] — so a half-empty instance of a wide air is dearer than a full one of a
//! narrow air of the same height, but neither is ever preferred to using one instance less.
//!
//! This is what makes the "large" airs (`BinaryLarge`, `MemAlignLarge`, `Dma64AlignedLarge`, …) worth
//! having: they hold twice the rows of their sibling at the same width, so the same work fits in half
//! the instances for the same area — and when they end up half empty, the extra area is still paid
//! gladly, because the instance count is what the criterion looks at first.
//!
//! # The two shapes of the problem
//!
//! [`select_airs`] is the general one: several *kinds* of work, each provable by some subset of the
//! airs at a kind-specific row cost. It is what a family with specialised airs needs — `Binary` and
//! its packed add airs, the DMA alignment airs, the mem-align byte airs.
//!
//! [`select_sizes`] is the degenerate one: a single kind of work and airs that differ only in size.
//! It is what a family whose only choice is "how big" needs — `ArithEq384`, `Keccakf`, and every air
//! that gained nothing but a `Large` sibling.

use zisk_pil::instance_area;

/// One air a family may instantiate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AirChoice {
    /// Air group the air belongs to.
    pub airgroup_id: usize,

    /// Air id within the group.
    pub air_id: usize,

    /// Rows one instance of this air holds.
    pub rows: u64,

    /// Area of one instance: its rows times the columns the setup commits for it.
    pub area: u64,
}

impl AirChoice {
    /// Builds a choice from the air's identity and height, taking its width from
    /// [`zisk_pil::setup_columns`].
    pub fn new(airgroup_id: usize, air_id: usize, rows: usize) -> Self {
        Self { airgroup_id, air_id, rows: rows as u64, area: instance_area(air_id, rows) }
    }
}

/// What one assignment costs, ordered the way the criterion ranks solutions: instances first, area
/// only to break a tie.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Cost {
    /// Instances the assignment opens, over every air of the family.
    pub instances: u64,

    /// Area those instances take together.
    pub area: u64,
}

/// The airs chosen for a family and what each is expected to hold.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    /// Instances of each air, indexed as the `airs` slice was.
    pub instances: Vec<u64>,

    /// The air each kind was assigned to, indexed as the `kinds` slice was. Meaningless for a kind
    /// with no work.
    pub assignment: Vec<usize>,

    /// What the chosen assignment costs.
    pub cost: Cost,
}

/// Assigns each kind of work to the air that proves it most cheaply *as a whole*, minimising the
/// instance count first and the area second.
///
/// # Parameters
/// * `kinds` — for each kind, the airs able to prove it as `(air index, rows it takes there)`. The
///   row cost is per kind because an air may pack a kind several to a row: what the caller passes is
///   the rows that kind's whole count occupies in that air. A kind with an empty option list, or
///   with zero rows everywhere, costs nothing — so a kind the caller *does* have work for must
///   never be listed empty: that would plan no room for it and lose it silently. Callers that
///   filter the options by capability are expected to leave a kind out only when it has no work.
/// * `airs` — the airs of the family, with their capacity and per-instance area.
///
/// # Why whole kinds
/// Splitting one kind across two airs never lowers the instance count — the rows have to live
/// somewhere either way — and the airs that could take the split are exactly the ones the caller
/// lists, so the hand-out that follows (`zisk_sm_binary::distribute` and its like) is free to spill
/// a residual onward within the counts chosen here. What this decides is which air *sizes* the family pays for.
///
/// # Panics
/// Panics if a kind names an air index outside `airs`.
pub fn select_airs(kinds: &[Vec<(usize, u64)>], airs: &[AirChoice]) -> Selection {
    if kinds.is_empty() || airs.is_empty() {
        return Selection { instances: vec![0; airs.len()], ..Default::default() };
    }

    let mut best: Option<(Cost, Vec<u64>, Vec<usize>)> = None;

    // Mixed-radix counter over the option lists: combo[i] indexes into kinds[i].
    let mut combo = vec![0usize; kinds.len()];
    loop {
        // Rows each air receives under this combination.
        let mut rows = vec![0u64; airs.len()];
        for (kind, &choice) in combo.iter().enumerate() {
            if let Some(&(air, kind_rows)) = kinds[kind].get(choice) {
                assert!(air < airs.len(), "kind {kind} names air {air}, out of {}", airs.len());
                rows[air] += kind_rows;
            }
        }

        let instances: Vec<u64> =
            rows.iter().zip(airs).map(|(&r, air)| r.div_ceil(air.rows)).collect();
        let cost = Cost {
            instances: instances.iter().sum(),
            area: instances.iter().zip(airs).map(|(&n, air)| n * air.area).sum(),
        };

        if best.as_ref().map_or(true, |(b, _, _)| cost < *b) {
            let assignment = combo
                .iter()
                .enumerate()
                .map(|(k, &c)| kinds[k].get(c).map_or(0, |o| o.0))
                .collect();
            best = Some((cost, instances, assignment));
        }

        // Advance the counter from the rightmost digit; a kind with no options has radix 1.
        let mut pos = kinds.len();
        loop {
            if pos == 0 {
                let (cost, instances, assignment) = best.expect("one combination is always seen");
                return Selection { instances, assignment, cost };
            }
            pos -= 1;
            combo[pos] += 1;
            if combo[pos] < kinds[pos].len().max(1) {
                break;
            }
            combo[pos] = 0;
        }
    }
}

/// Spreads `rows` of one kind of work over airs that differ only in size, minimising the instance
/// count first and the area second.
///
/// The fewest instances that can hold the work is `ceil(rows / tallest)`, so that count is fixed
/// first and the area is then shaved by demoting each instance to the shortest air that still leaves
/// the rest able to cover what remains. Demoting is what lowers the area because within a family the
/// airs share their width, so a shorter air is strictly cheaper.
///
/// # Returns
/// Instances of each air, indexed as `airs` was.
///
/// # Panics
/// Panics if `airs` is empty, or if a shorter air is not also cheaper — the family would then not be
/// a pure size ladder and demoting could raise the area instead of lowering it.
pub fn select_sizes(rows: u64, airs: &[AirChoice]) -> Vec<u64> {
    assert!(!airs.is_empty(), "a family must offer at least one air");

    let mut ladder: Vec<usize> = (0..airs.len()).collect();
    ladder.sort_by_key(|&i| (airs[i].rows, airs[i].area));
    for pair in ladder.windows(2) {
        let (shorter, taller) = (&airs[pair[0]], &airs[pair[1]]);
        assert!(
            shorter.rows == taller.rows || shorter.area < taller.area,
            "air {} is shorter than air {} but not cheaper ({} vs {} area): the family is not a \
             size ladder, so select_airs is the tool for it",
            shorter.air_id,
            taller.air_id,
            shorter.area,
            taller.area,
        );
    }

    let mut instances = vec![0u64; airs.len()];
    if rows == 0 {
        return instances;
    }

    let tallest = *ladder.last().expect("ladder is non-empty");
    let count = rows.div_ceil(airs[tallest].rows);

    // Hand out the instances one at a time, each the shortest air that still lets the ones after it
    // cover what would be left.
    let mut pending = rows;
    for filled in 0..count {
        let after = (count - filled - 1) * airs[tallest].rows;
        let pick =
            ladder.iter().copied().find(|&i| airs[i].rows + after >= pending).unwrap_or(tallest);
        instances[pick] += 1;
        pending = pending.saturating_sub(airs[pick].rows);
    }

    instances
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A size ladder: same width, the taller air twice the rows and twice the area.
    fn ladder() -> [AirChoice; 2] {
        [
            AirChoice { airgroup_id: 0, air_id: 0, rows: 100, area: 100 },
            AirChoice { airgroup_id: 0, air_id: 1, rows: 200, area: 200 },
        ]
    }

    /// The whole point of the criterion: one big instance beats two small ones even though they cost
    /// the same area, and beats them again when the big one is left half empty.
    #[test]
    fn fewer_instances_wins_over_less_area() {
        assert_eq!(
            select_sizes(200, &ladder()),
            vec![0, 1],
            "two smalls would be one instance more"
        );
        assert_eq!(select_sizes(101, &ladder()), vec![0, 1], "a half-empty big beats two smalls");
    }

    /// Once the instance count is settled, area decides: work that fits in the short air must not be
    /// given the tall one.
    #[test]
    fn area_breaks_the_tie() {
        assert_eq!(select_sizes(100, &ladder()), vec![1, 0]);
        assert_eq!(select_sizes(1, &ladder()), vec![1, 0]);
    }

    /// The tail instance is demoted on its own, so a ladder covers 3.5 big instances with three bigs
    /// and one small rather than four bigs.
    #[test]
    fn only_the_tail_is_demoted() {
        assert_eq!(select_sizes(700, &ladder()), vec![1, 3]);
        assert_eq!(select_sizes(800, &ladder()), vec![0, 4]);
    }

    #[test]
    fn nothing_to_prove_needs_no_instance() {
        assert_eq!(select_sizes(0, &ladder()), vec![0, 0]);
        assert_eq!(
            select_airs(&[], &ladder()),
            Selection { instances: vec![0, 0], ..Default::default() }
        );
    }

    /// Two kinds that each half-fill an air are put in the same one, which is the packing that saves
    /// the instance.
    #[test]
    fn kinds_share_an_air_rather_than_open_two() {
        // Kind 0 can go to the specialised air 0 or the general air 1; kind 1 only to air 1.
        let airs = [
            AirChoice { airgroup_id: 0, air_id: 0, rows: 100, area: 50 },
            AirChoice { airgroup_id: 0, air_id: 1, rows: 100, area: 100 },
        ];
        let kinds = vec![vec![(0, 40), (1, 40)], vec![(1, 40)]];

        let selection = select_airs(&kinds, &airs);
        assert_eq!(selection.instances, vec![0, 1], "both kinds ride in one general instance");
        assert_eq!(selection.cost, Cost { instances: 1, area: 100 });
    }

    /// When the kinds do not fit together, the specialised air is used and the area falls — the
    /// instance count is the same either way, so the tie-break decides.
    #[test]
    fn the_cheaper_air_takes_what_it_can_on_a_tie() {
        let airs = [
            AirChoice { airgroup_id: 0, air_id: 0, rows: 100, area: 50 },
            AirChoice { airgroup_id: 0, air_id: 1, rows: 100, area: 100 },
        ];
        let kinds = vec![vec![(0, 80), (1, 80)], vec![(1, 80)]];

        let selection = select_airs(&kinds, &airs);
        assert_eq!(selection.instances, vec![1, 1]);
        assert_eq!(selection.assignment[0], 0, "the specialised air is the cheaper home");
        assert_eq!(selection.cost, Cost { instances: 2, area: 150 });
    }

    /// A kind with no option contributes nothing rather than panicking, so a family may list a kind
    /// its build has no air for — the caller's side of the contract being that it has no work
    /// either.
    #[test]
    fn a_kind_with_no_air_is_ignored() {
        let airs = ladder();
        let selection = select_airs(&[vec![], vec![(0, 50)]], &airs);
        assert_eq!(selection.instances, vec![1, 0]);
    }
}
