//! Greedy scheduler for a job's aggregation tree. A node is the worker already holding
//! a set resident (its own phase-2 leaf or its last fold), so only siblings' proofs
//! travel. One task per worker at a time: the rest wait in `pending`.

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
    /// Kept so the node can be replayed.
    pub inputs: Vec<AggSet>,
    /// Union of the inputs' coverage.
    pub covers: BTreeSet<u32>,
    /// Whether this node produces the job's final proof.
    pub is_final: bool,
}

/// What a worker was asked to do; its response is checked against this.
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
    /// Leaves the node covers once it has taken this input (for logging).
    pub covers: BTreeSet<u32>,
}

/// Greedy reduction over the sets a job produces.
#[derive(Debug)]
pub struct AggScheduler {
    arity: usize,
    n_leaves: usize,
    distributed: bool,
    /// Nodes still taking inputs. Several at once, so a set can wait for an equal-weight
    /// partner instead of turning the tree into a chain.
    open: Vec<AggNode>,
    /// Leaves fed in so far.
    leaves_seen: usize,
    /// Leaves that arrived with no proofs (a partition with no instances). They count
    /// toward completion but never join a node: a drain needs a proof per airgroup.
    empty: BTreeSet<u32>,
    /// Nodes folding, keyed by worker; inputs retained until they export.
    live: HashMap<WorkerId, AggNode>,
    /// Workers with an aggregation task sent but not yet acked, and what it was.
    inflight: HashMap<WorkerId, AggTaskKind>,
    /// Dispatches held back because their worker was busy, in send order.
    pending: VecDeque<AggDispatch>,
    /// Donors freed since the last `take_released`, as soon as their set is accepted.
    released: Vec<WorkerId>,
}

impl AggScheduler {
    /// With `distributed` false every fold stays on the first node (single aggregator).
    pub fn new(arity: usize, n_leaves: usize, distributed: bool) -> Self {
        Self {
            arity: arity.max(2),
            n_leaves,
            distributed,
            open: Vec::new(),
            leaves_seen: 0,
            empty: BTreeSet::new(),
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

    /// Leaves a set must cover to be the whole job: the empty ones have nothing to fold.
    fn needed(&self) -> usize {
        self.n_leaves.saturating_sub(self.empty.len())
    }

    /// Every leaf arrived and none had a proof, so there is nothing to aggregate.
    pub fn all_leaves_empty(&self) -> bool {
        self.leaves_seen >= self.n_leaves && self.needed() == 0
    }

    /// Feed a newly available set -- a phase-2 leaf or a node's export.
    pub fn on_set_ready(&mut self, set: AggSet) -> Vec<AggDispatch> {
        if set.covers.len() == 1 {
            self.leaves_seen += 1;
        }
        let mut out: Vec<AggDispatch> = Vec::new();
        if set.proofs.is_empty() {
            // Nothing to fold: account for it and free its worker.
            self.empty.extend(set.covers.iter().copied());
            self.released.push(set.location);
        } else {
            out.extend(self.schedule(set));
        }
        out.extend(self.flush());
        out.into_iter().filter_map(|d| self.gate(d)).collect()
    }

    /// When nothing more can arrive (every leaf in, nothing folding) no open node will fill:
    /// drain all but the heaviest, whose exports then fold into it. A lone open node is
    /// left only when an empty leaf came last; it already covers the job.
    fn flush(&mut self) -> Vec<AggDispatch> {
        if self.leaves_seen < self.n_leaves || !self.live.is_empty() || self.open.is_empty() {
            return Vec::new();
        }
        if self.open.len() == 1 {
            let node = self.open.pop().expect("one open");
            return self.drain(node).into_iter().collect();
        }
        let keep = (0..self.open.len()).max_by_key(|&i| self.open[i].covers.len()).expect("open");
        let kept = self.open.swap_remove(keep);
        let others = std::mem::replace(&mut self.open, vec![kept]);
        others.into_iter().filter_map(|node| self.drain(node)).collect()
    }

    /// Close an open node early. One that folded nothing just passes its set on.
    fn drain(&mut self, mut node: AggNode) -> Option<AggDispatch> {
        if node.inputs.len() == 1 {
            return self.schedule(node.inputs.pop().expect("one input"));
        }
        node.is_final = node.covers.len() >= self.needed();
        let dispatch = AggDispatch {
            worker: node.worker.clone(),
            proofs: Vec::new(),
            last_proof: true,
            final_proof: node.is_final,
            keep_resident: !node.is_final,
            reset_state: false,
            covers: node.covers.clone(),
        };
        self.live.insert(node.worker.clone(), node);
        Some(dispatch)
    }

    /// The workers freed since the last call, for the caller to return to the pool.
    pub fn take_released(&mut self) -> Vec<WorkerId> {
        std::mem::take(&mut self.released)
    }

    /// The task this worker owes a response for, if any.
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

    /// Hold a dispatch back while its worker is busy: a worker drops a second task.
    fn gate(&mut self, dispatch: AggDispatch) -> Option<AggDispatch> {
        if self.inflight.contains_key(&dispatch.worker) {
            self.pending.push_back(dispatch);
            return None;
        }
        self.inflight.insert(dispatch.worker.clone(), kind_of(&dispatch));
        Some(dispatch)
    }

    /// Whether a set of this weight can still turn up, from a folding node or from leaves
    /// yet to arrive. Open nodes do not count: they only export once something closes them.
    fn more_of_weight_expected(&self, weight: usize) -> bool {
        self.live.values().any(|n| n.covers.len() == weight)
            || self.n_leaves.saturating_sub(self.leaves_seen) >= weight
    }

    /// The open node to fold this set into, or `None` to hold it open. A node of its level
    /// (the weight of the sets it collects, not its growing coverage) first; else wait while
    /// one may still turn up; else the nearest level.
    fn pick_partner(&self, set: &AggSet) -> Option<usize> {
        if self.open.is_empty() {
            return None;
        }
        if !self.distributed {
            return Some(0);
        }
        let weight = set.covers.len();
        let level = |n: &AggNode| n.inputs[0].covers.len();
        if let Some(i) = self.open.iter().position(|n| level(n) == weight) {
            return Some(i);
        }
        if self.more_of_weight_expected(weight) {
            return None;
        }
        self.open.iter().enumerate().min_by_key(|(_, n)| level(n).abs_diff(weight)).map(|(i, _)| i)
    }

    fn schedule(&mut self, set: AggSet) -> Option<AggDispatch> {
        let partner = self.pick_partner(&set);

        let Some(idx) = partner else {
            // A set covering the whole job drains straight to the final proof.
            if set.covers.len() >= self.needed() {
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

        let is_final = node.covers.len() >= self.needed();
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

    /// Consume a node once its export is accepted; `None` for a replayed export.
    pub fn take_live(&mut self, worker: &WorkerId) -> Option<AggNode> {
        self.live.remove(worker)
    }

    /// What a reconnecting worker needs re-sent: a reset plus the inputs already delivered
    /// (queued ones follow as they are), draining only if its drain was already sent.
    pub fn replay_for(&mut self, worker: &WorkerId) -> Option<AggDispatch> {
        let node =
            self.live.get(worker).or_else(|| self.open.iter().find(|n| &n.worker == worker))?;

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

    /// Whether the job still depends on this worker: a node of its own or a task in flight.
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

    fn empty_leaf(n: u32) -> AggSet {
        AggSet { proofs: Vec::new(), covers: BTreeSet::from([n]), location: worker(n) }
    }

    /// The single dispatch a step produced, if any: these tests never trigger a flush.
    fn one(v: Vec<AggDispatch>) -> Option<AggDispatch> {
        assert!(v.len() <= 1, "expected at most one dispatch, got {}", v.len());
        v.into_iter().next()
    }

    fn index_of(worker: &WorkerId) -> u32 {
        worker.as_string().trim_start_matches('w').parse().expect("wN")
    }

    /// Run the scheduler as the coordinator does. `staggered` finishes every fold a leaf
    /// enables before the next leaf lands, as in production. Returns (folds, tree depth,
    /// leaves the final proof covers or 0).
    fn drive(
        order: &[u32],
        arity: usize,
        distributed: bool,
        staggered: bool,
    ) -> (usize, usize, usize) {
        drive_with_empty(order, &[], arity, distributed, staggered)
    }

    /// As `drive`, with the leaves in `empty` arriving without proofs.
    fn drive_with_empty(
        order: &[u32],
        empty: &[u32],
        arity: usize,
        distributed: bool,
        staggered: bool,
    ) -> (usize, usize, usize) {
        let mut sched = AggScheduler::new(arity, order.len(), distributed);
        let mut leaves: VecDeque<AggSet> = order
            .iter()
            .map(|&i| if empty.contains(&i) { empty_leaf(i) } else { leaf(i) })
            .collect();
        let mut exports: VecDeque<(AggSet, usize)> = VecDeque::new();
        let mut sent: VecDeque<AggDispatch> = VecDeque::new();
        let mut depth_of: HashMap<BTreeSet<u32>, usize> = HashMap::new();
        let (mut folds, mut deepest) = (0, 0);
        loop {
            let set = if staggered { None } else { leaves.pop_front().map(|s| (s, 0)) };
            if let Some((set, d)) = set.or_else(|| exports.pop_front()) {
                depth_of.insert(set.covers.clone(), d);
                sent.extend(sched.on_set_ready(set));
                continue;
            }
            if let Some(dispatch) = sent.pop_front() {
                if dispatch.last_proof {
                    folds += 1;
                    let node = sched.take_live(&dispatch.worker).expect("a closed node is live");
                    let d = 1 + node.inputs.iter().map(|i| depth_of[&i.covers]).max().unwrap_or(0);
                    deepest = deepest.max(d);
                    if dispatch.final_proof {
                        return (folds, deepest, node.covers.len());
                    }
                    let covers: Vec<u32> = node.covers.iter().copied().collect();
                    exports.push_back((set_of(index_of(&node.worker), &covers), d));
                }
                sent.extend(sched.on_ack(&dispatch.worker));
                continue;
            }
            let Some(set) = leaves.pop_front() else { return (folds, deepest, 0) };
            depth_of.insert(set.covers.clone(), 0);
            sent.extend(sched.on_set_ready(set));
        }
    }

    #[test]
    fn a_staggered_arrival_still_does_not_build_a_chain() {
        // Folding into whatever was open built a chain; phase 3 waits on the deepest branch.
        for n in [2usize, 3, 4, 5, 6, 7, 8, 11, 16] {
            let order: Vec<u32> = (0..n as u32).collect();
            let floor = (usize::BITS - (n - 1).leading_zeros()) as usize;
            let depth = drive(&order, 2, true, true).1;
            // An odd set left over costs one level.
            let allowed = if n.is_power_of_two() { floor } else { floor + 1 };
            assert!(
                depth <= allowed,
                "n={n} staggered depth={depth} allowed={allowed}: degenerating toward a chain"
            );
        }
    }

    #[test]
    fn the_tree_stays_balanced_whatever_the_arrival_order() {
        for n in [2usize, 3, 4, 5, 6, 7, 8, 12, 16] {
            let floor = (usize::BITS - (n - 1).leading_zeros()) as usize; // ceil(log2 n)
            for order in
                [(0..n as u32).collect::<Vec<_>>(), (0..n as u32).rev().collect::<Vec<_>>()]
            {
                let depth = drive(&order, 2, true, false).1;
                assert_eq!(depth, floor, "n={n} order={order:?} depth={depth} floor={floor}");
            }
        }
    }

    #[test]
    fn balancing_does_not_change_the_number_of_folds() {
        for n in [2usize, 3, 5, 8, 12, 16] {
            let order: Vec<u32> = (0..n as u32).collect();
            assert_eq!(drive(&order, 2, true, false).0, n - 1, "n={n}");
        }
    }

    #[test]
    fn every_supported_arity_completes_in_any_order() {
        // A wrong hold, or a node that never fills, hangs the job. Poseidon keys use arity 3.
        let mut seed = 0xD1B54A32D192ED03u64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for arity in [2usize, 3, 4] {
            for n in 1usize..=17 {
                for _ in 0..20 {
                    let mut order: Vec<u32> = (0..n as u32).collect();
                    for i in (1..order.len()).rev() {
                        order.swap(i, (next() % (i as u64 + 1)) as usize);
                    }
                    for (distributed, staggered) in
                        [(true, false), (true, true), (false, false), (false, true)]
                    {
                        let (folds, _, covered) = drive(&order, arity, distributed, staggered);
                        assert_eq!(
                            covered, n,
                            "arity={arity} n={n} distributed={distributed} {order:?}: wedged"
                        );
                        if arity == 2 && distributed {
                            assert_eq!(folds, n.saturating_sub(1).max(1), "n={n} {order:?}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn empty_leaves_are_counted_but_never_folded() {
        // A worker whose partition had no instances returns no proofs. It must still count
        // toward completion, or the job waits forever for coverage it cannot reach.
        let mut seed = 0x9E3779B97F4A7C15u64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for arity in [2usize, 3, 4] {
            for n in 2usize..=12 {
                for _ in 0..20 {
                    let mut order: Vec<u32> = (0..n as u32).collect();
                    for i in (1..order.len()).rev() {
                        order.swap(i, (next() % (i as u64 + 1)) as usize);
                    }
                    // Leaf 0 keeps its proofs, so there is always something to prove.
                    let empty: Vec<u32> = (1..n as u32).filter(|_| next() % 2 == 0).collect();
                    for (distributed, staggered) in
                        [(true, false), (true, true), (false, false), (false, true)]
                    {
                        let (_, _, covered) =
                            drive_with_empty(&order, &empty, arity, distributed, staggered);
                        assert_eq!(
                            covered,
                            n - empty.len(),
                            "arity={arity} n={n} distributed={distributed} {order:?} \
                             empty={empty:?}: wedged"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn an_empty_leaf_is_released_without_a_task() {
        let mut sched = AggScheduler::new(2, 3, true);
        assert!(sched.on_set_ready(empty_leaf(1)).is_empty(), "nothing to fold");
        assert_eq!(sched.take_released(), vec![worker(1)], "its worker is freed at once");
        assert!(!sched.holds_work(&worker(1)));
    }

    #[test]
    fn an_empty_last_leaf_closes_the_open_node() {
        // arity 3: leaves 0 and 1 share a node that is not full; the empty third leaf is
        // the last event, so it has to trigger the final drain itself.
        let mut sched = AggScheduler::new(3, 3, true);
        sched.on_set_ready(leaf(0));
        let absorb = one(sched.on_set_ready(leaf(1))).expect("absorb");
        assert!(!absorb.last_proof);
        sched.on_ack(&worker(0));
        let d = one(sched.on_set_ready(empty_leaf(2))).expect("final drain");
        assert!(d.final_proof && d.last_proof && d.proofs.is_empty());
        assert_eq!(d.worker, worker(0));
    }

    #[test]
    fn a_job_of_only_empty_leaves_reports_it() {
        let mut sched = AggScheduler::new(2, 2, true);
        sched.on_set_ready(empty_leaf(0));
        assert!(!sched.all_leaves_empty(), "one leaf still to come");
        sched.on_set_ready(empty_leaf(1));
        assert!(sched.all_leaves_empty());
    }

    #[test]
    fn a_single_worker_job_drains_straight_to_the_final_proof() {
        let mut sched = AggScheduler::new(2, 1, true);
        let d = one(sched.on_set_ready(leaf(0))).expect("the sole leaf is the whole tree");
        assert!(d.final_proof && d.last_proof);
        assert!(d.proofs.is_empty(), "the node already holds its own leaf");
        assert!(sched.live_node(&worker(0)).is_some());
    }

    #[test]
    fn a_worker_never_has_two_tasks_outstanding() {
        let mut sched = AggScheduler::new(2, 4, false);
        assert!(one(sched.on_set_ready(leaf(0))).is_none(), "seeding sends nothing");
        assert!(one(sched.on_set_ready(leaf(1))).is_some(), "first absorb goes out");
        assert!(one(sched.on_set_ready(leaf(2))).is_none(), "second is held back");
        assert!(one(sched.on_set_ready(leaf(3))).is_none(), "third is held back");

        let second = sched.on_ack(&worker(0)).expect("the ack releases the next one");
        assert_eq!(second.worker, worker(0));
        assert!(sched.on_ack(&worker(0)).is_some(), "and then the third");
        assert!(sched.on_ack(&worker(0)).is_none(), "nothing left");
    }

    #[test]
    fn the_last_queued_task_for_a_full_job_is_the_final_drain() {
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
        let d = one(sched.on_set_ready(leaf(2))).expect("the sibling is dispatched");
        assert_eq!(d.worker, worker(1), "the holder of the first set is the node");
        assert_eq!(sched.take_released(), vec![worker(2)], "the donor is freed");
        assert!(!d.proofs.is_empty(), "the sibling's proof does travel");
    }

    #[test]
    fn an_intermediate_group_keeps_its_result_resident() {
        let mut sched = AggScheduler::new(2, 8, true);
        sched.on_set_ready(leaf(1));
        let d = one(sched.on_set_ready(leaf(2))).expect("dispatch");
        assert!(d.last_proof && !d.final_proof);
        assert!(d.keep_resident, "it folds again at the next level");
    }

    #[test]
    fn a_worker_that_handed_its_set_over_holds_nothing() {
        let mut sched = AggScheduler::new(2, 8, true);
        sched.on_set_ready(leaf(1));
        let d = one(sched.on_set_ready(leaf(2))).expect("dispatch");

        assert!(sched.holds_work(&d.worker), "the node is folding");
        assert!(!sched.holds_work(&worker(2)), "the donor's set is already elsewhere");
    }

    #[test]
    fn a_donor_is_freed_when_its_set_is_accepted_not_when_the_dispatch_goes_out() {
        // Sets 2 and 3 queue behind set 1, but their donors are freed now.
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
        // The node has closed, but the absorb ack is still what it owes.
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
