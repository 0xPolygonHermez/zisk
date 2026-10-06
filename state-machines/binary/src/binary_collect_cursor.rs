//! The cursor that decides, operation by operation, what one instance takes out of a chunk.
//!
//! Every operation belongs to one *kind*, and the instance carries a `(count, skip)` for each: how many
//! of that kind the airs and instances ahead of it take, and how many are then its own. Tracking the
//! kinds apart is what keeps the decision independent of the order they are interleaved in — the
//! planner only ever knows counts, and the cursor discovers the interleaving as it replays the chunk.
//!
//! Frequent operations take no row, so they are settled first and separately: the instance named
//! accountant for that kind in this chunk counts them, and nobody else does. That is what lets an
//! instance collect a kind whose frops belong to another air.
//!
//! Keeping this out of the collectors is what makes it testable: they need a live `Std` to exist, this
//! does not.

use crate::{ChunkCollect, KindCollect};

/// What a collector must do with one operation of the chunk it is replaying.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CollectAction {
    /// This instance is finished and nothing else in the chunk concerns it.
    Stop,

    /// Not this instance's operation: another air proves it, another instance collects it, or its
    /// share of that kind is already complete.
    Pass,

    /// A frequent operation this instance accounts for: bump its row in the frops table.
    CountFrop,

    /// Collect it as an input.
    Collect,
}

/// Tracks what one instance still has to take from the chunk being replayed.
#[derive(Clone, Copy, Debug)]
pub struct BinaryCollectCursor<const K: usize> {
    kinds: [KindCollect; K],
    collected: [u64; K],
    force_execute_to_end: bool,
}

impl<const K: usize> BinaryCollectCursor<K> {
    pub fn new(collect: ChunkCollect<K>) -> Self {
        Self {
            kinds: collect.kinds,
            collected: [0; K],
            force_execute_to_end: collect.force_execute_to_end,
        }
    }

    /// Operations of one kind collected so far.
    #[inline(always)]
    pub fn collected(&self, kind: usize) -> u64 {
        self.collected[kind]
    }

    /// Total operations collected so far.
    #[inline(always)]
    pub fn total_collected(&self) -> u64 {
        self.collected.iter().sum()
    }

    /// `true` once every kind's share is complete and there is no reason to keep walking the chunk.
    ///
    /// An instance that still has frops to account for has to walk to the end even then, which is what
    /// `force_execute_to_end` marks.
    #[inline(always)]
    pub fn is_done(&self) -> bool {
        !self.force_execute_to_end && (0..K).all(|k| self.collected[k] == self.kinds[k].count)
    }

    /// Decides what to do with the next operation of the chunk, given its kind.
    #[inline(always)]
    pub fn next(&mut self, kind: usize, is_frop: bool) -> CollectAction {
        if self.is_done() {
            return CollectAction::Stop;
        }

        // Frequent operations take no row, so the counts below do not govern them: the accountant of
        // this kind in this chunk counts every one of them, and nobody else does.
        if is_frop {
            return if self.kinds[kind].owns_frops {
                CollectAction::CountFrop
            } else {
                CollectAction::Pass
            };
        }

        // Operations of this kind belonging to the airs and instances ahead of this one.
        if self.kinds[kind].skipper.should_skip() {
            return CollectAction::Pass;
        }

        // Our share of this kind is complete: the rest is another instance's.
        if self.collected[kind] == self.kinds[kind].count {
            return CollectAction::Pass;
        }

        self.collected[kind] += 1;
        CollectAction::Collect
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zisk_common::CollectSkipper;

    const A: usize = 0;
    const B: usize = 1;

    /// A cursor over two kinds: `(skip, count, accountant)` for each.
    fn cursor(a: (u64, u64, bool), b: (u64, u64, bool), force: bool) -> BinaryCollectCursor<2> {
        let kind = |(skip, count, owns): (u64, u64, bool)| KindCollect {
            count,
            skipper: CollectSkipper::new(skip),
            owns_frops: owns,
        };
        BinaryCollectCursor::new(ChunkCollect {
            kinds: [kind(a), kind(b)],
            force_execute_to_end: force,
        })
    }

    /// The core case: let `n` of a kind pass, keep `k`, and let the rest pass too.
    #[test]
    fn skips_n_collects_k_then_lets_the_rest_pass() {
        let mut c = cursor((3, 2, false), (0, 0, false), true);

        for _ in 0..3 {
            assert_eq!(c.next(A, false), CollectAction::Pass);
        }
        assert_eq!(c.next(A, false), CollectAction::Collect);
        assert_eq!(c.next(A, false), CollectAction::Collect);
        for _ in 0..4 {
            assert_eq!(c.next(A, false), CollectAction::Pass);
        }
        assert_eq!(c.collected(A), 2);
    }

    /// Each kind is bounded on its own, so one cannot eat into another's share however they interleave.
    /// This is what a single count over their union could not guarantee.
    #[test]
    fn each_kind_is_bounded_on_its_own() {
        let mut c = cursor((0, 2, false), (0, 1, false), true);

        assert_eq!(c.next(A, false), CollectAction::Collect);
        assert_eq!(c.next(B, false), CollectAction::Collect);
        assert_eq!(c.next(B, false), CollectAction::Pass, "kind B's share is complete");
        assert_eq!(c.next(A, false), CollectAction::Collect, "kind A's is not");
        assert_eq!(c.next(A, false), CollectAction::Pass);
        assert_eq!(c.collected(A), 2);
        assert_eq!(c.collected(B), 1);
    }

    /// A kind this air does not prove has a share of zero, so it simply flows past.
    #[test]
    fn a_kind_with_no_share_flows_past() {
        let mut c = cursor((0, 0, false), (0, 1, false), false);
        assert_eq!(c.next(A, false), CollectAction::Pass);
        assert_eq!(c.next(B, false), CollectAction::Collect);
    }

    /// Without frops to account for, the cursor stops once every share is complete.
    #[test]
    fn stops_once_every_share_is_complete() {
        let mut c = cursor((0, 1, false), (0, 1, false), false);
        assert_eq!(c.next(A, false), CollectAction::Collect);
        assert!(!c.is_done(), "kind B is still pending");
        assert_eq!(c.next(B, false), CollectAction::Collect);
        assert!(c.is_done());
        assert_eq!(c.next(A, false), CollectAction::Stop);
    }

    /// An instance with no operations at all is legitimate: it is only there to account for frops.
    #[test]
    fn a_zero_share_still_accounts_for_frops() {
        let mut c = cursor((0, 0, true), (0, 0, false), true);
        assert_eq!(c.next(A, true), CollectAction::CountFrop);
        assert_eq!(c.next(A, false), CollectAction::Pass);
        assert_eq!(c.next(B, true), CollectAction::Pass, "kind B is another air's");
        assert_eq!(c.total_collected(), 0);
    }

    /// Frequent operations take no row, so they must not advance a kind's boundary.
    #[test]
    fn frops_do_not_advance_the_boundary() {
        let mut c = cursor((2, 1, true), (0, 0, false), true);

        assert_eq!(c.next(A, true), CollectAction::CountFrop);
        assert_eq!(c.next(A, false), CollectAction::Pass);
        assert_eq!(c.next(A, true), CollectAction::CountFrop);
        assert_eq!(c.next(A, false), CollectAction::Pass);
        // The boundary is reached after exactly two real operations.
        assert_eq!(c.next(A, false), CollectAction::Collect);
    }

    /// Accountancy is per kind and independent of what is collected: an instance can collect a kind
    /// whose frops are another air's, and account for a kind it collects none of.
    #[test]
    fn accountancy_is_independent_of_what_is_collected() {
        let mut c = cursor((0, 2, false), (0, 0, true), true);

        assert_eq!(c.next(A, true), CollectAction::Pass, "collected but not accounted for");
        assert_eq!(c.next(B, true), CollectAction::CountFrop, "accounted for but not collected");
        assert_eq!(c.next(A, false), CollectAction::Collect);
    }

    /// A forced instance keeps accounting for frops after its shares are complete, while the real
    /// operations pass through.
    #[test]
    fn a_forced_instance_accounts_for_the_trailing_frops() {
        let mut c = cursor((0, 1, true), (0, 0, false), true);
        assert_eq!(c.next(A, false), CollectAction::Collect);
        assert_eq!(c.next(A, true), CollectAction::CountFrop);
        assert_eq!(c.next(A, false), CollectAction::Pass);
    }

    /// Two instances sharing a chunk take every operation exactly once, and exactly one of them
    /// accounts for each frop — whatever the interleaving.
    #[test]
    fn two_instances_tile_the_chunk() {
        // Kind A: 5 real operations split 2 + 3. Kind B: 2, all the second instance's.
        let mut first = cursor((0, 2, false), (0, 0, false), false);
        let mut second = cursor((2, 3, true), (0, 2, true), true);

        let stream = [
            (A, false),
            (A, true),
            (A, false),
            (B, true),
            (A, false),
            (B, false),
            (A, false),
            (A, false),
            (B, false),
            (A, true),
        ];

        let mut collected = 0;
        let mut frops = 0;
        for (kind, is_frop) in stream {
            for c in [&mut first, &mut second] {
                match c.next(kind, is_frop) {
                    CollectAction::Collect => collected += 1,
                    CollectAction::CountFrop => frops += 1,
                    CollectAction::Pass | CollectAction::Stop => {}
                }
            }
        }

        assert_eq!(collected, 7, "every real operation collected exactly once");
        assert_eq!(frops, 3, "every frop accounted for exactly once");
    }
}
