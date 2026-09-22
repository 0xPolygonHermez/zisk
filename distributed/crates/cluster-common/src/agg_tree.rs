//! Greedy reduction scheduler for a job's aggregation tree.
//!
//! Whenever two sets are available they are folded together, and the worker that
//! produced one of them keeps its result resident and carries on to the next level.
//!
//! The node for a set is never chosen -- it is the worker already holding it, since
//! every worker's own phase-2 leaf is resident and nobody else can absorb that set
//! without double-counting. So seeding sends nothing, and only a sibling's proof
//! crosses the wire.
//!
//! One task per worker at a time: a second would run a concurrent proofman task over
//! the same GPU streams. The rest wait in `pending`, released by [`AggScheduler::on_ack`].

use std::collections::{BTreeSet, HashMap, VecDeque};

use crate::{AggProofData, WorkerId};

/// One recursive2 proof per airgroup, the leaf workers it covers, and the worker
/// holding it resident.
#[derive(Debug, Clone)]
pub struct AggSet {
    /// The proofs, one per airgroup.
    pub proofs: Vec<AggProofData>,
    /// Leaf workers folded into this set.
    pub covers: BTreeSet<u32>,
    /// The worker that produced it and still holds it.
    pub location: WorkerId,
}

/// A worker folding a group of sets.
#[derive(Debug, Clone)]
pub struct AggNode {
    /// The folding worker.
    pub worker: WorkerId,
    /// Retained so the group can be re-folded elsewhere if this node is lost.
    pub inputs: Vec<AggSet>,
    /// Union of the inputs' coverage.
    pub covers: BTreeSet<u32>,
    /// Whether this node produces the job's final proof.
    pub is_final: bool,
}

/// What a worker was asked to do. A response is checked against this, not against
/// scheduler state, which moves on while the worker folds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggTaskKind {
    /// Absorb only; the worker acks with no proof.
    Absorb,
    /// Drain and return the folded subtree.
    Drain,
    /// Drain and return the job's final proof.
    DrainFinal,
}

/// A task the coordinator must send as a result of a scheduling step.
#[derive(Debug, Clone)]
pub struct AggDispatch {
    /// The node to send to.
    pub worker: WorkerId,
    /// Empty when the node already holds the set resident.
    pub proofs: Vec<AggProofData>,
    /// Drain after absorbing this.
    pub last_proof: bool,
    /// This drain produces the job's final proof.
    pub final_proof: bool,
    /// Keep the drained proof resident for a further level.
    pub keep_resident: bool,
    /// Drop resident state before absorbing (recovery).
    pub reset_state: bool,
    /// Leaf workers the node covers once it has taken this input. For logging.
    pub covers: BTreeSet<u32>,
}

/// Greedy reduction over the sets a job produces.
#[derive(Debug)]
pub struct AggScheduler {
    arity: usize,
    n_leaves: usize,
    distributed: bool,
    /// Nodes still taking inputs. Several may be open at once: a set is held back
    /// rather than folded against a mismatched partner when an equal-weight one is
    /// still in flight, which is what keeps the tree from degenerating into a chain.
    /// Undistributed runs keep at most one (see `group_cap`).
    open: Vec<AggNode>,
    /// Leaves fed in so far, to tell "a leaf may still arrive" from "that was the last".
    leaves_seen: usize,
    /// Nodes folding, keyed by worker; inputs retained until they export.
    live: HashMap<WorkerId, AggNode>,
    /// Workers with an aggregation task sent but not yet acked, and what it was.
    inflight: HashMap<WorkerId, AggTaskKind>,
    /// Dispatches held back because their worker was busy, in send order.
    pending: VecDeque<AggDispatch>,
    /// Donors freed since the last drain. Freed when their set is accepted, not when
    /// the dispatch carrying it goes out: that left them queued behind other work, and
    /// lost them entirely if the dispatch was dropped.
    released: Vec<WorkerId>,
}

impl AggScheduler {
    /// With `distributed` false every fold stays on the first node, which is the
    /// single-aggregator behaviour this replaced.
    pub fn new(arity: usize, n_leaves: usize, distributed: bool) -> Self {
        Self {
            arity: arity.max(2),
            n_leaves,
            distributed,
            open: Vec::new(),
            leaves_seen: 0,
            live: HashMap::new(),
            inflight: HashMap::new(),
            pending: VecDeque::new(),
            released: Vec::new(),
        }
    }

    fn group_cap(&self) -> usize {
        if self.distributed {
            self.arity
        } else {
            usize::MAX
        }
    }

    /// Feed a newly available set -- a phase-2 leaf or a node's export.
    pub fn on_set_ready(&mut self, set: AggSet) -> Option<AggDispatch> {
        let dispatch = self.schedule(set)?;
        self.gate(dispatch)
    }

    /// Take the workers freed since the last call, so the caller can return them
    /// to the pool.
    pub fn take_released(&mut self) -> Vec<WorkerId> {
        std::mem::take(&mut self.released)
    }

    /// Whether this worker has a task sent but not yet acked. An ack from a worker
    /// with nothing outstanding is a duplicate and must not release the queue.
    pub fn outstanding(&self, worker: &WorkerId) -> Option<AggTaskKind> {
        self.inflight.get(worker).copied()
    }

    /// Release the next dispatch held back for `worker`, now that its task is done.
    pub fn on_ack(&mut self, worker: &WorkerId) -> Option<AggDispatch> {
        self.inflight.remove(worker);
        let idx = self.pending.iter().position(|d| !self.inflight.contains_key(&d.worker))?;
        let dispatch = self.pending.remove(idx)?;
        self.inflight.insert(dispatch.worker.clone(), kind_of(&dispatch));
        Some(dispatch)
    }

    /// Hold a dispatch back if its worker is busy: a second concurrent task is
    /// dropped by the worker, leaving the tree short a fold.
    fn gate(&mut self, dispatch: AggDispatch) -> Option<AggDispatch> {
        if self.inflight.contains_key(&dispatch.worker) {
            self.pending.push_back(dispatch);
            return None;
        }
        self.inflight.insert(dispatch.worker.clone(), kind_of(&dispatch));
        Some(dispatch)
    }

    /// Whether a set of this weight can still turn up: an unarrived leaf, or a node
    /// already folding that will export one. Open nodes are deliberately NOT counted --
    /// they only produce once something else closes them, so waiting on one could wait
    /// forever. Every hold this justifies is backed by work already in flight.
    fn more_of_weight_expected(&self, weight: usize) -> bool {
        if self.live.values().any(|n| n.covers.len() == weight) {
            return true;
        }
        // Leaves that have not landed yet will pair among themselves, so enough of them
        // can still build a partner of this weight. Both terms shrink monotonically --
        // leaves all arrive, live nodes all export -- so this cannot hold a set forever:
        // once it goes false the set folds against whatever is nearest.
        self.n_leaves - self.leaves_seen >= weight
    }

    /// The open node to fold this set into, or `None` to hold the set open instead.
    ///
    /// Equal weight first, so the tree grows level by level. Failing that the set waits,
    /// but only while an equal-weight partner is still in flight; once nothing more of
    /// its weight can arrive it takes the nearest partner rather than stalling. That
    /// fallback is what guarantees progress: after the last arrival nothing is live, so
    /// every subsequent set folds.
    fn pick_partner(&self, set: &AggSet) -> Option<usize> {
        if self.open.is_empty() {
            return None;
        }
        // Undistributed: one node takes everything, exactly as before.
        if !self.distributed {
            return Some(0);
        }
        let weight = set.covers.len();
        if let Some(i) = self.open.iter().position(|n| n.covers.len() == weight) {
            return Some(i);
        }
        if self.more_of_weight_expected(weight) {
            return None;
        }
        // Nearest weight, so the flush still pairs like with like where it can.
        self.open
            .iter()
            .enumerate()
            .min_by_key(|(_, n)| n.covers.len().abs_diff(weight))
            .map(|(i, _)| i)
    }

    fn schedule(&mut self, set: AggSet) -> Option<AggDispatch> {
        if set.covers.len() == 1 {
            self.leaves_seen += 1;
        }

        let partner = self.pick_partner(&set);

        let Some(idx) = partner else {
            // Nothing to fold against. A set already covering the job is the whole tree,
            // so its holder drains straight to the final proof -- the single-worker job,
            // and the last set standing once everything else has folded into it.
            if set.covers.len() >= self.n_leaves {
                let worker = set.location.clone();
                let covers = set.covers.clone();
                self.live.insert(
                    worker.clone(),
                    AggNode {
                        worker: worker.clone(),
                        covers: set.covers.clone(),
                        inputs: vec![set],
                        is_final: true,
                    },
                );
                return Some(AggDispatch {
                    worker,
                    proofs: Vec::new(),
                    last_proof: true,
                    final_proof: true,
                    keep_resident: false,
                    reset_state: false,
                    covers,
                });
            }

            self.open.push(AggNode {
                worker: set.location.clone(),
                covers: set.covers.clone(),
                inputs: vec![set],
                is_final: false,
            });
            return None;
        };

        let mut node = self.open.remove(idx);

        // A set already held by this node never crosses the wire.
        let held = set.location == node.worker;
        let proofs = if held { Vec::new() } else { set.proofs.clone() };
        if !held {
            self.released.push(set.location.clone());
        }

        node.covers.extend(set.covers.iter().copied());
        node.inputs.push(set);

        let is_final = node.covers.len() >= self.n_leaves;
        let last = is_final || node.inputs.len() >= self.group_cap();
        node.is_final = is_final;

        let covers = node.covers.clone();
        let worker = node.worker.clone();
        if last {
            self.live.insert(worker.clone(), node);
        } else {
            self.open.push(node);
        }

        Some(AggDispatch {
            worker,
            proofs,
            last_proof: last,
            final_proof: is_final,
            keep_resident: last && !is_final,
            reset_state: false,
            covers,
        })
    }

    /// The node a worker is folding, if it is draining for this job.
    pub fn live_node(&self, worker: &WorkerId) -> Option<&AggNode> {
        self.live.get(worker)
    }

    /// Consume a node once its export is accepted. `None` makes a replayed export a
    /// no-op rather than a second fold of the same subtree.
    pub fn take_live(&mut self, worker: &WorkerId) -> Option<AggNode> {
        self.live.remove(worker)
    }

    /// What a reconnecting worker needs re-sent. It lost whatever it had folded, so
    /// the replay resets it and re-absorbs from scratch.
    ///
    /// Only the sets already delivered: `node.inputs` runs ahead of what the worker
    /// has seen, and replaying the queued tail would register those indexes twice.
    pub fn replay_for(&mut self, worker: &WorkerId) -> Option<AggDispatch> {
        let node =
            self.live.get(worker).or_else(|| self.open.iter().find(|n| &n.worker == worker))?;

        // Everything must come from the delivered prefix. A node closes when its
        // drain dispatch is created, which may still be queued -- replaying it as a
        // drain would fold over an incomplete input set. The queued drain follows.
        let queued = self.pending.iter().filter(|d| &d.worker == worker).count();
        let delivered = node.inputs.len().saturating_sub(queued);
        let drained = queued == 0 && self.live.contains_key(worker);

        let dispatch = AggDispatch {
            worker: worker.clone(),
            proofs: node.inputs[..delivered]
                .iter()
                .flat_map(|s| s.proofs.iter().cloned())
                .collect(),
            last_proof: drained,
            final_proof: drained && node.is_final,
            keep_resident: drained && !node.is_final,
            reset_state: true,
            covers: node.covers.clone(),
        };

        self.inflight.insert(worker.clone(), kind_of(&dispatch));
        Some(dispatch)
    }

    /// Whether this worker still carries work the job depends on: a node of its own,
    /// or a task it owes a response for. One that has handed its set over holds
    /// nothing the coordinator cannot replace, so losing it costs the job nothing.
    pub fn holds_work(&self, worker: &WorkerId) -> bool {
        self.live.contains_key(worker)
            || self.open.iter().any(|n| &n.worker == worker)
            || self.inflight.contains_key(worker)
    }
}

fn kind_of(dispatch: &AggDispatch) -> AggTaskKind {
    match (dispatch.last_proof, dispatch.final_proof) {
        (false, _) => AggTaskKind::Absorb,
        (true, false) => AggTaskKind::Drain,
        (true, true) => AggTaskKind::DrainFinal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn worker(n: u32) -> WorkerId {
        WorkerId::from(format!("w{n}"))
    }

    fn set_of(location: u32, covers: &[u32]) -> AggSet {
        AggSet {
            proofs: vec![AggProofData {
                worker_indexes: covers.to_vec(),
                airgroup_id: 0,
                values: vec![location as u64],
            }],
            covers: covers.iter().copied().collect(),
            location: worker(location),
        }
    }

    fn leaf(n: u32) -> AggSet {
        set_of(n, &[n])
    }

    fn index_of(worker: &WorkerId) -> u32 {
        worker.as_string().trim_start_matches('w').parse().expect("wN")
    }

    /// Drive the scheduler to completion the way the coordinator does: feed sets,
    /// send what it hands back, ack each one, and feed exports back in. Returns the
    /// number of folds performed.
    fn run(order: &[u32], arity: usize, distributed: bool) -> usize {
        let mut sched = AggScheduler::new(arity, order.len(), distributed);
        let mut ready: VecDeque<AggSet> = order.iter().map(|&i| leaf(i)).collect();
        let mut sent: VecDeque<AggDispatch> = VecDeque::new();
        let mut folds = 0;

        loop {
            if let Some(set) = ready.pop_front() {
                sent.extend(sched.on_set_ready(set));
                continue;
            }
            let Some(dispatch) = sent.pop_front() else { break };

            if dispatch.last_proof {
                folds += 1;
                let node = sched.take_live(&dispatch.worker).expect("a closed node is live");
                if !dispatch.final_proof {
                    let covers: Vec<u32> = node.covers.iter().copied().collect();
                    ready.push_back(set_of(index_of(&node.worker), &covers));
                }
            }
            sent.extend(sched.on_ack(&dispatch.worker));
        }
        folds
    }

    /// Drive the scheduler like `run`, but return the depth of the finished tree --
    /// the number of folds on its longest root-to-leaf path, which is what phase 3
    /// actually waits through.
    fn run_depth(order: &[u32], arity: usize) -> usize {
        let mut sched = AggScheduler::new(arity, order.len(), true);
        let mut ready: VecDeque<(AggSet, usize)> =
            order.iter().map(|&i| (leaf(i), 0usize)).collect();
        let mut sent: VecDeque<AggDispatch> = VecDeque::new();
        // Depth of each set currently held by a node, keyed by the leaves it covers.
        let mut depth_of: HashMap<BTreeSet<u32>, usize> = HashMap::new();
        let mut deepest = 0;

        loop {
            if let Some((set, depth)) = ready.pop_front() {
                depth_of.insert(set.covers.clone(), depth);
                sent.extend(sched.on_set_ready(set));
                continue;
            }
            let Some(dispatch) = sent.pop_front() else { break };

            if dispatch.last_proof {
                let node = sched.take_live(&dispatch.worker).expect("a closed node is live");
                let d = node
                    .inputs
                    .iter()
                    .map(|i| depth_of.get(&i.covers).copied().unwrap_or(0))
                    .max()
                    .unwrap_or(0)
                    + 1;
                deepest = deepest.max(d);
                if !dispatch.final_proof {
                    let covers: Vec<u32> = node.covers.iter().copied().collect();
                    ready.push_back((set_of(index_of(&node.worker), &covers), d));
                }
            }
            sent.extend(sched.on_ack(&dispatch.worker));
        }
        deepest
    }

    /// Depth when leaves trickle in rather than arriving together: after each leaf,
    /// every fold it made possible runs to completion before the next leaf shows up.
    /// This is the shape production actually sees -- phase 2 finishes over ~400ms while
    /// a fold takes ~215ms -- and it is what turns a greedy scheduler into a chain.
    fn run_depth_staggered(order: &[u32], arity: usize) -> usize {
        let mut sched = AggScheduler::new(arity, order.len(), true);
        let mut leaves: VecDeque<AggSet> = order.iter().map(|&i| leaf(i)).collect();
        let mut sent: VecDeque<AggDispatch> = VecDeque::new();
        let mut exports: VecDeque<(AggSet, usize)> = VecDeque::new();
        let mut depth_of: HashMap<BTreeSet<u32>, usize> = HashMap::new();
        let mut deepest = 0;

        loop {
            // Exports first: they are already in flight when the next leaf lands.
            if let Some((set, d)) = exports.pop_front() {
                depth_of.insert(set.covers.clone(), d);
                sent.extend(sched.on_set_ready(set));
                continue;
            }
            if let Some(dispatch) = sent.pop_front() {
                if dispatch.last_proof {
                    let node = sched.take_live(&dispatch.worker).expect("closed node is live");
                    let d = node
                        .inputs
                        .iter()
                        .map(|i| depth_of.get(&i.covers).copied().unwrap_or(0))
                        .max()
                        .unwrap_or(0)
                        + 1;
                    deepest = deepest.max(d);
                    if !dispatch.final_proof {
                        let covers: Vec<u32> = node.covers.iter().copied().collect();
                        exports.push_back((set_of(index_of(&node.worker), &covers), d));
                    }
                }
                sent.extend(sched.on_ack(&dispatch.worker));
                continue;
            }
            let Some(set) = leaves.pop_front() else { break };
            depth_of.insert(set.covers.clone(), 0);
            sent.extend(sched.on_set_ready(set));
        }
        deepest
    }

    #[test]
    fn a_staggered_arrival_still_does_not_build_a_chain() {
        // The regression this guards: folding whatever arrived next against whatever was
        // open left one branch several levels deeper than the rest, and phase 3 waits on
        // the deepest branch. Two levels of slack is generous -- a chain over 8 leaves is
        // 7 deep -- while leaving room for the flush to pair unlike weights at the end.
        for n in [2usize, 3, 4, 5, 6, 7, 8, 11, 16] {
            let order: Vec<u32> = (0..n as u32).collect();
            let floor = (usize::BITS - (n - 1).leading_zeros()) as usize;
            let depth = run_depth_staggered(&order, 2);
            // Powers of two pair exactly and sit on the floor. Other sizes leave an odd
            // set over that can only fold against a heavier partner, costing one level.
            let allowed = if n.is_power_of_two() { floor } else { floor + 1 };
            assert!(
                depth <= allowed,
                "n={n} staggered depth={depth} allowed={allowed}: degenerating toward a chain"
            );
        }
    }

    #[test]
    fn the_tree_stays_balanced_whatever_the_arrival_order() {
        // A chain over n leaves is n-1 deep; the floor is ceil(log2(n)). The scheduler
        // holds a set back rather than fold it against a mismatched partner, so it should
        // sit on the floor -- this is the whole point of pick_partner.
        for n in [2usize, 3, 4, 5, 6, 7, 8, 12, 16] {
            let floor = (usize::BITS - (n - 1).leading_zeros()) as usize; // ceil(log2 n)
            for order in
                [(0..n as u32).collect::<Vec<_>>(), (0..n as u32).rev().collect::<Vec<_>>()]
            {
                let depth = run_depth(&order, 2);
                assert_eq!(depth, floor, "n={n} order={order:?} depth={depth} floor={floor}");
            }
        }
    }

    #[test]
    fn balancing_does_not_change_the_number_of_folds() {
        // Depth is bought by ordering the folds better, not by doing more of them.
        for n in [2usize, 3, 5, 8, 12, 16] {
            let order: Vec<u32> = (0..n as u32).collect();
            assert_eq!(run(&order, 2, true), n - 1, "n={n}");
        }
    }

    #[test]
    fn no_arrival_order_can_wedge_the_scheduler() {
        // pick_partner holds a set back when a better partner may still turn up. If that
        // expectation is ever wrong the set sits open forever and the job hangs until the
        // phase-3 timeout, so every order must still complete all n-1 folds. Deterministic
        // pseudo-random orders, both feeding styles.
        let mut seed = 0x9E3779B97F4A7C15u64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for n in 1usize..=17 {
            for _ in 0..40 {
                let mut order: Vec<u32> = (0..n as u32).collect();
                for i in (1..order.len()).rev() {
                    order.swap(i, (next() % (i as u64 + 1)) as usize);
                }
                // One leaf is the whole tree: it drains straight to the final proof,
                // which is a fold of its own rather than n-1 = 0 of them.
                let expected = if n == 1 { 1 } else { n - 1 };
                assert_eq!(run(&order, 2, true), expected, "batched, n={n}, order={order:?}");
                // Staggered completes too: reaching a depth at all means it terminated.
                let d = run_depth_staggered(&order, 2);
                assert!(d >= 1 || n == 1, "staggered wedged, n={n}, order={order:?}");
            }
        }
    }

    #[test]
    fn eight_workers_fold_seven_times_in_any_order() {
        for order in [
            vec![1, 2, 3, 4, 5, 6, 7, 8],
            vec![8, 7, 6, 5, 4, 3, 2, 1],
            vec![3, 1, 8, 2, 7, 4, 6, 5],
        ] {
            let folds = run(&order, 2, true);
            assert_eq!(folds, 7, "order={order:?}");
        }
    }

    #[test]
    fn a_single_worker_job_drains_straight_to_the_final_proof() {
        let mut sched = AggScheduler::new(2, 1, true);
        let d = sched.on_set_ready(leaf(0)).expect("the sole leaf is the whole tree");
        assert!(d.final_proof && d.last_proof);
        assert!(d.proofs.is_empty(), "the node already holds its own leaf");
        assert!(sched.live_node(&worker(0)).is_some());
    }

    #[test]
    fn a_worker_never_has_two_tasks_outstanding() {
        // Undistributed: every set goes to the one node, so the second and later
        // arrivals must queue behind the first rather than being sent.
        let mut sched = AggScheduler::new(2, 4, false);
        assert!(sched.on_set_ready(leaf(0)).is_none(), "seeding sends nothing");
        assert!(sched.on_set_ready(leaf(1)).is_some(), "first absorb goes out");
        assert!(sched.on_set_ready(leaf(2)).is_none(), "second is held back");
        assert!(sched.on_set_ready(leaf(3)).is_none(), "third is held back");

        let second = sched.on_ack(&worker(0)).expect("the ack releases the next one");
        assert_eq!(second.worker, worker(0));
        assert!(sched.on_ack(&worker(0)).is_some(), "and then the third");
        assert!(sched.on_ack(&worker(0)).is_none(), "nothing left");
    }

    #[test]
    fn the_last_queued_task_for_a_full_job_is_the_final_drain() {
        // Three leaves, one node: leaf 0 seeds it, leaf 1 is dispatched, leaf 2
        // queues. Releasing the queued one completes coverage, so it is the drain.
        let mut sched = AggScheduler::new(2, 3, false);
        sched.on_set_ready(leaf(0));
        sched.on_set_ready(leaf(1));
        sched.on_set_ready(leaf(2));
        let last = sched.on_ack(&worker(0)).expect("the queued third set");
        assert!(last.final_proof, "coverage is complete, so this drains the job");
    }

    #[test]
    fn a_seeded_node_is_never_sent_the_set_it_already_holds() {
        let mut sched = AggScheduler::new(2, 4, true);
        sched.on_set_ready(leaf(1));
        let d = sched.on_set_ready(leaf(2)).expect("the sibling is dispatched");
        assert_eq!(d.worker, worker(1), "the holder of the first set is the node");
        assert_eq!(sched.take_released(), vec![worker(2)], "the donor is freed");
        assert!(!d.proofs.is_empty(), "the sibling's proof does travel");
    }

    #[test]
    fn an_intermediate_group_keeps_its_result_resident() {
        let mut sched = AggScheduler::new(2, 8, true);
        sched.on_set_ready(leaf(1));
        let d = sched.on_set_ready(leaf(2)).expect("dispatch");
        assert!(d.last_proof && !d.final_proof);
        assert!(d.keep_resident, "it folds again at the next level");
    }

    #[test]
    fn a_replayed_export_is_a_no_op() {
        let mut sched = AggScheduler::new(2, 8, true);
        sched.on_set_ready(leaf(1));
        let d = sched.on_set_ready(leaf(2)).expect("dispatch");
        assert!(sched.take_live(&d.worker).is_some(), "the first export consumes the node");
        assert!(sched.take_live(&d.worker).is_none(), "a replay finds nothing to consume");
    }

    #[test]
    fn a_worker_that_handed_its_set_over_holds_nothing() {
        let mut sched = AggScheduler::new(2, 8, true);
        sched.on_set_ready(leaf(1));
        let d = sched.on_set_ready(leaf(2)).expect("dispatch");

        assert!(sched.holds_work(&d.worker), "the node is folding");
        assert!(!sched.holds_work(&worker(2)), "the donor's set is already elsewhere");
    }

    #[test]
    fn a_donor_is_freed_when_its_set_is_accepted_not_when_the_dispatch_goes_out() {
        // Undistributed: sets 2 and 3 queue behind set 1. Their donors must still
        // be freed now -- waiting for the dispatch would hold the fleet across the
        // whole of phase 3, and lose them entirely if that dispatch is dropped.
        let mut sched = AggScheduler::new(2, 8, false);
        sched.on_set_ready(leaf(0));
        sched.on_set_ready(leaf(1));
        sched.on_set_ready(leaf(2));
        sched.on_set_ready(leaf(3));

        assert_eq!(sched.take_released(), vec![worker(1), worker(2), worker(3)]);
        assert!(sched.take_released().is_empty(), "draining twice yields nothing");
    }

    #[test]
    fn the_outstanding_task_says_what_a_response_must_be() {
        // The node closes into `live` while an earlier absorb is still unacked, so
        // scheduler state alone would misread that absorb's ack as the drain's.
        let mut sched = AggScheduler::new(2, 3, false);
        sched.on_set_ready(leaf(0));
        sched.on_set_ready(leaf(1));
        sched.on_set_ready(leaf(2));

        assert!(sched.live_node(&worker(0)).is_some(), "the node has closed");
        assert_eq!(
            sched.outstanding(&worker(0)),
            Some(AggTaskKind::Absorb),
            "but what is outstanding is still the absorb"
        );

        let drain = sched.on_ack(&worker(0)).expect("the queued drain");
        assert_eq!(sched.outstanding(&worker(0)), Some(AggTaskKind::DrainFinal));
        assert!(drain.final_proof);
    }

    #[test]
    fn a_replay_of_a_node_whose_drain_is_still_queued_is_not_a_drain() {
        let mut sched = AggScheduler::new(2, 3, false);
        sched.on_set_ready(leaf(0));
        sched.on_set_ready(leaf(1));
        sched.on_set_ready(leaf(2)); // closes the node, but this drain is queued

        assert!(sched.live_node(&worker(0)).is_some(), "the node has closed");
        let replay = sched.replay_for(&worker(0)).expect("replayable");
        assert!(!replay.last_proof, "draining now would fold over an incomplete set");
        assert!(!replay.final_proof);
        assert_eq!(replay.proofs.len(), 2, "only what was delivered");
    }

    #[test]
    fn a_replay_carries_only_what_the_worker_has_actually_seen() {
        let mut sched = AggScheduler::new(2, 8, false);
        sched.on_set_ready(leaf(0));
        sched.on_set_ready(leaf(1));
        sched.on_set_ready(leaf(2)); // queued behind the first

        let replay = sched.replay_for(&worker(0)).expect("the open node is replayable");
        assert_eq!(replay.proofs.len(), 2, "the queued set has not been delivered yet");
        assert!(
            sched.on_ack(&worker(0)).is_some(),
            "and it is still queued, so it is delivered once the replay is acked"
        );
    }

    #[test]
    fn an_ack_from_an_idle_worker_does_not_release_the_queue() {
        let mut sched = AggScheduler::new(2, 8, false);
        sched.on_set_ready(leaf(0));
        sched.on_set_ready(leaf(1));
        sched.on_set_ready(leaf(2));
        assert_eq!(
            sched.outstanding(&worker(0)),
            Some(AggTaskKind::Absorb),
            "the node owes an absorb ack"
        );
        assert_eq!(sched.outstanding(&worker(5)), None, "an uninvolved worker owes nothing");
    }

    #[test]
    fn an_open_node_is_replayable_too() {
        // The undistributed aggregator stays open all job, so replay must cover it.
        let mut sched = AggScheduler::new(2, 8, false);
        sched.on_set_ready(leaf(1));
        sched.on_set_ready(leaf(2));
        let replay = sched.replay_for(&worker(1)).expect("the open node is replayable");
        assert!(replay.reset_state, "it lost what it had folded");
        assert!(!replay.last_proof, "it was still absorbing");
        assert_eq!(replay.proofs.len(), 2, "both inputs are re-sent, its own leaf included");
    }
}
