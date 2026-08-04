//! The cursor that decides, operation by operation, what one add-capable instance takes out of a
//! chunk.
//!
//! Three limits act at once and the order between them is what makes them independent:
//!
//! 1. the additions another air proves are dropped first, so they consume neither skip budget nor
//!    rows ([`ShapeDrop`]);
//! 2. the operations of the instances before this one are skipped;
//! 3. frequent operations never take a row, and are accounted for by a single instance;
//! 4. once the quota is met the remaining operations just pass through.
//!
//! Keeping this out of the collectors themselves is what makes it testable: the collectors need a
//! live `Std` to exist, this does not.

use crate::{AddShape, BinaryCollectInfo};
use zisk_common::CollectSkipper;

/// What a collector must do with one operation of the stream it is watching.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CollectAction {
    /// This instance is finished and nothing else in the chunk concerns it.
    Stop,

    /// Not this instance's operation: another air proves it, another instance collects it, or the
    /// quota is already met.
    Pass,

    /// A frequent operation this instance accounts for: bump its row in the frops table.
    CountFrop,

    /// Collect it as an input.
    Collect,
}

/// Tracks what one instance of an add-capable air (`Binary`, `BinaryAdd` or `BinaryAddHi`) still has
/// to take from the chunk being replayed.
#[derive(Clone, Copy, Debug)]
pub struct BinaryCollectCursor {
    hi_drop: crate::ShapeDrop,
    full_drop: crate::ShapeDrop,
    skipper: CollectSkipper,
    num_operations: usize,
    collected: usize,
    force_execute_to_end: bool,
}

impl BinaryCollectCursor {
    pub fn new(info: BinaryCollectInfo) -> Self {
        Self {
            hi_drop: info.hi_drop,
            full_drop: info.full_drop,
            skipper: info.skipper,
            num_operations: info.count as usize,
            collected: 0,
            force_execute_to_end: info.force_execute_to_end,
        }
    }

    /// Number of operations collected so far.
    #[inline(always)]
    pub fn collected(&self) -> usize {
        self.collected
    }

    /// `true` once the quota is met and there is no reason to keep walking the chunk.
    ///
    /// An instance that still has frops to account for has to walk to the end even after its quota
    /// is met, which is what `force_execute_to_end` marks.
    #[inline(always)]
    pub fn is_done(&self) -> bool {
        self.collected == self.num_operations && !self.force_execute_to_end
    }

    /// Decides what to do with the next operation of the chunk.
    ///
    /// `shape` is `None` for operations that are not additions, and `is_frop` marks the frequent
    /// ones.
    #[inline(always)]
    pub fn next(&mut self, shape: Option<AddShape>, is_frop: bool) -> CollectAction {
        if self.is_done() {
            return CollectAction::Stop;
        }

        // Additions another air proves must not consume this instance's skip budget nor its rows.
        if let Some(shape) = shape {
            let mine = match shape {
                AddShape::Hi | AddShape::HiNeg => self.hi_drop.accepts(is_frop),
                AddShape::Full => self.full_drop.accepts(is_frop),
            };
            if !mine {
                return CollectAction::Pass;
            }
        }

        // Operations belonging to the instances before this one. Frequent operations do not advance
        // the boundary, since they never took a row from anyone.
        if self.skipper.should_skip_query(!is_frop) {
            return CollectAction::Pass;
        }

        if is_frop {
            return CollectAction::CountFrop;
        }

        // Quota met: the rest of the chunk is another instance's, so it just passes through.
        if self.collected == self.num_operations {
            return CollectAction::Pass;
        }

        self.collected += 1;
        CollectAction::Collect
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ShapeDrop;

    /// A cursor over a plain stream: skip `skip`, collect `count`, no additions involved.
    fn plain(skip: u64, count: u64, force: bool) -> BinaryCollectCursor {
        BinaryCollectCursor::new(BinaryCollectInfo {
            count,
            skipper: CollectSkipper::new(skip),
            hi_drop: ShapeDrop::none(),
            full_drop: ShapeDrop::none(),
            force_execute_to_end: force,
        })
    }

    /// The core case: let `n` pass, keep `k`, and let everything after that pass too.
    #[test]
    fn skips_n_collects_k_then_lets_the_rest_pass() {
        let mut cursor = plain(3, 2, true);

        // The three operations of the instances before this one.
        for _ in 0..3 {
            assert_eq!(cursor.next(None, false), CollectAction::Pass);
        }
        // Its own two.
        assert_eq!(cursor.next(None, false), CollectAction::Collect);
        assert_eq!(cursor.next(None, false), CollectAction::Collect);
        // Everything after the quota belongs to the next instance.
        for _ in 0..4 {
            assert_eq!(cursor.next(None, false), CollectAction::Pass);
        }
        assert_eq!(cursor.collected(), 2);
    }

    /// Without frops to account for, the cursor stops as soon as its quota is met.
    #[test]
    fn stops_once_the_quota_is_met_when_not_forced() {
        let mut cursor = plain(0, 1, false);

        assert_eq!(cursor.next(None, false), CollectAction::Collect);
        assert_eq!(cursor.next(None, false), CollectAction::Stop);
        assert!(cursor.is_done());
    }

    /// A quota of zero is legitimate: the instance is only there to account for the chunk's frops.
    #[test]
    fn a_zero_quota_still_accounts_for_frops() {
        let mut cursor = plain(0, 0, true);

        assert_eq!(cursor.next(None, true), CollectAction::CountFrop);
        assert_eq!(cursor.next(None, false), CollectAction::Pass);
        assert_eq!(cursor.next(None, true), CollectAction::CountFrop);
        assert_eq!(cursor.collected(), 0);
    }

    /// A zero quota with nothing to account for stops immediately.
    #[test]
    fn a_zero_quota_without_frops_stops_immediately() {
        let mut cursor = plain(0, 0, false);
        assert_eq!(cursor.next(None, false), CollectAction::Stop);
    }

    /// Frequent operations never take a row, so they must not advance the skip boundary: an instance
    /// that has to skip `n` real operations still skips exactly `n`, however many frops appear
    /// among them. The frops of that stretch belong to the previous instance.
    #[test]
    fn frops_do_not_advance_the_skip_boundary() {
        let mut cursor = plain(2, 1, true);

        assert_eq!(cursor.next(None, true), CollectAction::Pass, "frop inside the skipped stretch");
        assert_eq!(cursor.next(None, false), CollectAction::Pass);
        assert_eq!(cursor.next(None, true), CollectAction::Pass, "still inside it");
        assert_eq!(cursor.next(None, false), CollectAction::Pass);
        // The skip is now exhausted, so this instance owns what follows.
        assert_eq!(cursor.next(None, true), CollectAction::CountFrop);
        assert_eq!(cursor.next(None, false), CollectAction::Collect);
    }

    /// A forced instance keeps accounting for frops after its quota is met — that is the whole point
    /// of forcing it — while the real operations pass through.
    #[test]
    fn a_forced_instance_accounts_for_the_trailing_frops() {
        let mut cursor = plain(0, 1, true);

        assert_eq!(cursor.next(None, false), CollectAction::Collect);
        assert_eq!(cursor.next(None, true), CollectAction::CountFrop);
        assert_eq!(cursor.next(None, false), CollectAction::Pass);
        assert_eq!(cursor.next(None, true), CollectAction::CountFrop);
    }

    /// Additions another air proves are dropped before the skipper sees them, so they do not eat
    /// into the skip budget: with a skip of 1, a dropped addition followed by one real operation
    /// still leaves the real one skipped.
    #[test]
    fn dropped_additions_do_not_consume_the_skip_budget() {
        let mut cursor = BinaryCollectCursor::new(BinaryCollectInfo {
            count: 1,
            skipper: CollectSkipper::new(1),
            // Every low-limb addition belongs to another air.
            hi_drop: ShapeDrop::all(),
            full_drop: ShapeDrop::none(),
            force_execute_to_end: false,
        });

        // Three low-limb additions that are not ours, interleaved with a frop.
        assert_eq!(cursor.next(Some(AddShape::Hi), false), CollectAction::Pass);
        assert_eq!(cursor.next(Some(AddShape::HiNeg), false), CollectAction::Pass);
        assert_eq!(cursor.next(Some(AddShape::Hi), true), CollectAction::Pass);

        // The skip budget is untouched, so the next full-shape addition is the one skipped...
        assert_eq!(cursor.next(Some(AddShape::Full), false), CollectAction::Pass);
        // ...and the one after it is collected.
        assert_eq!(cursor.next(Some(AddShape::Full), false), CollectAction::Collect);
    }

    /// `ShapeDrop::all` also keeps the frops of that shape away, which is what makes the air that
    /// owns the shape in this chunk its sole accountant.
    #[test]
    fn a_dropped_shape_hands_over_its_frops_too() {
        let mut cursor = BinaryCollectCursor::new(BinaryCollectInfo {
            count: 0,
            skipper: CollectSkipper::new(0),
            hi_drop: ShapeDrop::all(),
            full_drop: ShapeDrop::none(),
            force_execute_to_end: true,
        });

        assert_eq!(cursor.next(Some(AddShape::Hi), true), CollectAction::Pass, "not ours");
        assert_eq!(cursor.next(Some(AddShape::Full), true), CollectAction::CountFrop, "ours");
        assert_eq!(cursor.next(None, true), CollectAction::CountFrop, "basic frops are ours");
    }

    /// `ShapeDrop::first(n)` hands the tail of a shape over to this instance: the prefix and the
    /// frops interleaved in it belong to the air ahead of it in the chain.
    #[test]
    fn a_shape_prefix_goes_to_the_air_ahead() {
        let mut cursor = BinaryCollectCursor::new(BinaryCollectInfo {
            count: 2,
            skipper: CollectSkipper::new(0),
            // The first two low-limb additions of the chunk belong to the packed air.
            hi_drop: ShapeDrop::first(2),
            full_drop: ShapeDrop::all(),
            force_execute_to_end: true,
        });

        assert_eq!(
            cursor.next(Some(AddShape::Hi), true),
            CollectAction::Pass,
            "frop of the prefix"
        );
        assert_eq!(cursor.next(Some(AddShape::Hi), false), CollectAction::Pass);
        assert_eq!(cursor.next(Some(AddShape::HiNeg), false), CollectAction::Pass);
        // The prefix is done, so the tail is ours — frops included.
        assert_eq!(cursor.next(Some(AddShape::Hi), true), CollectAction::CountFrop);
        assert_eq!(cursor.next(Some(AddShape::HiNeg), false), CollectAction::Collect);
        assert_eq!(cursor.next(Some(AddShape::Hi), false), CollectAction::Collect);
        // Quota met.
        assert_eq!(cursor.next(Some(AddShape::Hi), false), CollectAction::Pass);
    }

    /// Basic operations are not additions, so no drop applies to them.
    #[test]
    fn basic_operations_ignore_the_shape_drops() {
        let mut cursor = BinaryCollectCursor::new(BinaryCollectInfo {
            count: 1,
            skipper: CollectSkipper::new(0),
            hi_drop: ShapeDrop::all(),
            full_drop: ShapeDrop::all(),
            force_execute_to_end: false,
        });

        assert_eq!(cursor.next(Some(AddShape::Hi), false), CollectAction::Pass);
        assert_eq!(cursor.next(None, false), CollectAction::Collect);
    }

    /// Two instances sharing a chunk must together take every operation exactly once, and exactly one
    /// of them must account for each frop.
    #[test]
    fn two_instances_tile_the_chunk() {
        // A chunk of 5 real operations with frops interleaved, split 2 + 3.
        let stream = [
            (false, false), // real
            (false, true),  // frop
            (false, false), // real
            (false, true),  // frop
            (false, false), // real
            (false, false), // real
            (false, true),  // frop
            (false, false), // real
            (false, true),  // frop  (trailing)
        ];

        let mut first = plain(0, 2, false);
        let mut second = plain(2, 3, true);

        let mut collected = 0;
        let mut frops = 0;
        for (_, is_frop) in stream {
            for cursor in [&mut first, &mut second] {
                match cursor.next(None, is_frop) {
                    CollectAction::Collect => collected += 1,
                    CollectAction::CountFrop => frops += 1,
                    CollectAction::Pass | CollectAction::Stop => {}
                }
            }
        }

        assert_eq!(collected, 5, "every real operation collected exactly once");
        assert_eq!(frops, 4, "every frop accounted for exactly once");
    }
}
