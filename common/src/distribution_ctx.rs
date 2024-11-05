#[cfg(feature = "distributed")]
use mpi::traits::*;
#[cfg(feature = "distributed")]
use mpi::environment::Universe;
#[cfg(feature = "distributed")]
use mpi::collective::CommunicatorCollectives;
#[cfg(feature = "distributed")]
use mpi::datatype::PartitionMut;
#[cfg(feature = "distributed")]
use mpi::topology::Communicator;
use std::collections::HashMap;
use std::collections::BTreeMap;
#[cfg(feature = "distributed")]
use proofman_starks_lib_c::*;
use std::ffi::c_void;

/// Represents the context of distributed computing
pub struct DistributionCtx {
    pub rank: i32,
    pub n_processes: i32,
    #[cfg(feature = "distributed")]
    pub universe: Universe,
    #[cfg(feature = "distributed")]
    pub world: mpi::topology::SimpleCommunicator,
    pub n_instances: usize,
    pub my_instances: Vec<usize>,
    pub instances: Vec<(usize, usize)>,       //group_id, air_id
    pub instances_owner: Vec<(usize, usize)>, //owner_rank, owner_instance_idx
    pub owners_count: Vec<i32>,
    pub owners_weight: Vec<u64>,
    #[cfg(feature = "distributed")]
    pub roots_gatherv_count: Vec<i32>,
    #[cfg(feature = "distributed")]
    pub roots_gatherv_displ: Vec<i32>,
    pub my_groups: Vec<Vec<usize>>,
    pub my_air_groups: Vec<Vec<usize>>,
    pub airgroup_instances: Vec<Vec<usize>>,
    pub glob2loc: Vec<Option<usize>>,
}

impl DistributionCtx {
    pub fn new() -> Self {
        #[cfg(feature = "distributed")]
        {
            let (universe, _threading) = mpi::initialize_with_threading(mpi::Threading::Multiple).unwrap();
            let world = universe.world();
            let rank = world.rank();
            let n_processes = world.size();
            DistributionCtx {
                rank,
                n_processes,
                universe,
                world,
                n_instances: 0,
                my_instances: Vec::new(),
                instances: Vec::new(),
                instances_owner: Vec::new(),
                owners_count: vec![0; n_processes as usize],
                owners_weight: vec![0; n_processes as usize],
                roots_gatherv_count: vec![0; n_processes as usize],
                roots_gatherv_displ: vec![0; n_processes as usize],
                my_groups: Vec::new(),
                my_air_groups: Vec::new(),
                airgroup_instances: Vec::new(),
                glob2loc: Vec::new(),
            }
        }
        #[cfg(not(feature = "distributed"))]
        {
            DistributionCtx {
                rank: 0,
                n_processes: 1,
                n_instances: 0,
                my_instances: Vec::new(),
                instances: Vec::new(),
                instances_owner: Vec::new(),
                owners_count: vec![0; 1],
                owners_weight: vec![0; 1],
                my_groups: Vec::new(),
                my_air_groups: Vec::new(),
                airgroup_instances: Vec::new(),
                glob2loc: Vec::new(),
            }
        }
    }

    #[inline]
    pub fn barrier(&self) {
        #[cfg(feature = "distributed")]
        {
            self.world.barrier();
        }
    }

    #[inline]
    pub fn is_distributed(&self) -> bool {
        self.n_processes > 1
    }

    #[inline]
    pub fn is_my_instance(&self, instance_idx: usize) -> bool {
        self.owner(instance_idx) == self.rank as usize
    }

    #[inline]
    pub fn owner(&self, instance_idx: usize) -> usize {
        self.instances_owner[instance_idx].0
    }

    #[inline]
    pub fn add_instance(&mut self, airgroup_id: usize, air_id: usize, weight: usize) -> (bool, usize) {
        let mut is_mine = false;
        let owner = self.n_instances % self.n_processes as usize;
        self.instances.push((airgroup_id, air_id));
        self.instances_owner.push((owner, self.owners_count[owner] as usize));
        self.owners_count[owner] += 1;
        self.owners_weight[owner] += weight as u64;

        if owner == self.rank as usize {
            self.my_instances.push(self.n_instances);
            is_mine = true;
        }
        self.n_instances += 1;
        (is_mine, self.n_instances - 1)
    }

    pub fn close(&mut self, n_airgroups: usize) {
        let mut group_indices: BTreeMap<usize, Vec<usize>> = BTreeMap::new();

        // Calculate the partial sums of owners_count
        #[cfg(feature = "distributed")]
        {
            let mut total_instances = 0;
            for i in 0..self.n_processes as usize {
                self.roots_gatherv_displ[i] = total_instances;
                self.roots_gatherv_count[i] = self.owners_count[i] * 4;
                total_instances += self.roots_gatherv_count[i];
            }
        }

        // Populate the HashMap based on group_id and buffer positions
        for (idx, &(group_id, _)) in self.instances.iter().enumerate() {
            #[cfg(feature = "distributed")]
            let pos_buffer =
                self.roots_gatherv_displ[self.instances_owner[idx].0] as usize + self.instances_owner[idx].1 * 4;
            #[cfg(not(feature = "distributed"))]
            let pos_buffer = idx * 4;
            group_indices.entry(group_id).or_default().push(pos_buffer);
        }

        // Flatten the HashMap into a single vector for my_groups
        for (_, indices) in group_indices {
            self.my_groups.push(indices);
        }

        // Create my eval groups
        let mut my_air_groups_indices: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
        for (loc_idx, glob_idx) in self.my_instances.iter().enumerate() {
            let instance_idx = self.instances[*glob_idx];
            my_air_groups_indices.entry(instance_idx).or_default().push(loc_idx);
        }

        // Flatten the HashMap into a single vector for my_air_groups
        for (_, indices) in my_air_groups_indices {
            self.my_air_groups.push(indices);
        }

        //Calculate instances of each airgroup
        self.airgroup_instances = vec![Vec::new(); n_airgroups];
        for (idx, &(group_id, _)) in self.instances.iter().enumerate() {
            self.airgroup_instances[group_id].push(idx);
        }

        //Evaluate glob2loc
        self.glob2loc = vec![None; self.n_instances];
        for (loc_idx, glob_idx) in self.my_instances.iter().enumerate() {
            self.glob2loc[*glob_idx] = Some(loc_idx);
        }
    }

    pub fn distribute_roots(&self, roots: Vec<u64>) -> Vec<u64> {
        #[cfg(feature = "distributed")]
        {
            let mut all_roots: Vec<u64> = vec![0; 4 * self.n_instances];
            let counts = &self.roots_gatherv_count;
            let displs = &self.roots_gatherv_displ;

            let mut partitioned_all_roots = PartitionMut::new(&mut all_roots, counts.as_slice(), displs.as_slice());

            self.world.all_gather_varcount_into(&roots, &mut partitioned_all_roots);

            all_roots
        }
        #[cfg(not(feature = "distributed"))]
        {
            roots
        }
    }

    pub fn distribute_multiplicity(&self, _multiplicity: &mut [u64], _owner: usize) {
        #[cfg(feature = "distributed")]
        {
            //assert that I can operate with u32
            assert!(_multiplicity.len() < std::u32::MAX as usize);

            if _owner != self.rank as usize {
                //pack multiplicities in a sparce vector
                let mut packed_multiplicity = Vec::new();
                packed_multiplicity.push(0 as u32); //this will be the counter
                for (idx, &m) in _multiplicity.iter().enumerate() {
                    if m != 0 {
                        assert!(m < std::u32::MAX as u64);
                        packed_multiplicity.push(idx as u32);
                        packed_multiplicity.push(m as u32);
                        packed_multiplicity[0] += 2;
                    }
                }
                self.world.process_at_rank(_owner as i32).send(&packed_multiplicity[..]);
            } else {
                let mut packed_multiplicity: Vec<u32> = vec![0; _multiplicity.len() * 2 + 1];
                for i in 0..self.n_processes {
                    if i != _owner as i32 {
                        self.world.process_at_rank(i).receive_into(&mut packed_multiplicity);
                        for j in (1..packed_multiplicity[0]).step_by(2) {
                            let idx = packed_multiplicity[j as usize] as usize;
                            let m = packed_multiplicity[j as usize + 1] as u64;
                            _multiplicity[idx] += m;
                        }
                    }
                }
            }
        }
    }

    pub fn distribute_multiplicities(&self, _multiplicities: &mut [Vec<u64>], _owner: usize) {
        #[cfg(feature = "distributed")]
        {
            // Ensure that each multiplicity vector can be operated with u32
            let mut buff_size = 0;
            for multiplicity in _multiplicities.iter() {
                assert!(multiplicity.len() < std::u32::MAX as usize);
                buff_size += multiplicity.len() + 1;
            }

            let n_columns = _multiplicities.len();
            if _owner != self.rank as usize {
                // Pack multiplicities in a sparse vector
                let mut packed_multiplicities = vec![0u32; n_columns];
                for (col_idx, multiplicity) in _multiplicities.iter().enumerate() {
                    for (idx, &m) in multiplicity.iter().enumerate() {
                        if m != 0 {
                            assert!(m < std::u32::MAX as u64);
                            packed_multiplicities[col_idx] += 1;
                            packed_multiplicities.push(idx as u32);
                            packed_multiplicities.push(m as u32);
                        }
                    }
                }
                self.world.process_at_rank(_owner as i32).send(&packed_multiplicities[..]);
            } else {
                let mut packed_multiplicities: Vec<u32> = vec![0; buff_size * 2];
                for i in 0..self.n_processes {
                    if i != _owner as i32 {
                        self.world.process_at_rank(i).receive_into(&mut packed_multiplicities);

                        // Read counters
                        let mut counters = vec![0usize; n_columns];
                        for col_idx in 0..n_columns {
                            counters[col_idx] = packed_multiplicities[col_idx] as usize;
                        }

                        // Unpack multiplicities
                        let mut idx = n_columns;
                        for col_idx in 0..n_columns {
                            for _ in 0..counters[col_idx] {
                                let row_idx = packed_multiplicities[idx] as usize;
                                let m = packed_multiplicities[idx + 1] as u64;
                                _multiplicities[col_idx][row_idx] += m;
                                idx += 2;
                            }
                        }
                    }
                }
            }
        }
    }
    #[allow(unused_variables)]
    pub fn distribute_recursive2_proofs(&mut self, alives: &[usize], proofs: &mut [Vec<Option<*mut c_void>>]) {
        #[cfg(feature = "distributed")]
        {
            // Count number of aggregations that will be done
            let n_groups = alives.len();
            let n_agregations: usize = alives.iter().map(|&alive| alive / 2).sum();
            let aggs_per_process = (n_agregations / self.n_processes as usize).max(1);

            let mut i_proof = 0;
            // tags codes:
            // 0,...,ngroups-1: proofs that need to be sent to rank0 from another rank for a group with alive == 1
            // ngroups, ..., ngroups + 2*n_aggregations - 1: proofs that need to be sent to the owner of the aggregation task

            for (group_idx, &alive) in alives.iter().enumerate() {
                let group_proofs: &mut Vec<Option<*mut c_void>> = &mut proofs[group_idx];
                let n_aggs_group = alive / 2;

                if n_aggs_group == 0 {
                    assert!(alive == 1);
                    if self.rank == 0 {
                        if group_proofs[0].is_none() {
                            // Receive proof from the owner process
                            let tag = group_idx as i32;
                            let (mut msg, _status) = self.world.any_process().receive_vec_with_tag::<i8>(tag);
                            group_proofs[0] = Some(deserialize_zkin_proof_c(msg.as_mut_ptr()));
                        }
                    } else if group_proofs[0].is_some() {
                        let (ptr, size) = get_serialized_proof_c(group_proofs[0].unwrap());
                        let tag = group_idx as i32;
                        let buffer = unsafe { std::slice::from_raw_parts(ptr as *const i8, size as usize) };
                        self.world.process_at_rank(0).send_with_tag(buffer, tag);
                        zkin_proof_free_c(group_proofs[0].unwrap());
                        group_proofs[0] = None;
                    }
                }

                for i in 0..n_aggs_group {
                    let chunk = i_proof / aggs_per_process;
                    let owner_rank =
                        if chunk < self.n_processes as usize { chunk } else { i_proof % self.n_processes as usize };
                    let left_idx = i * 2;
                    let right_idx = i * 2 + 1;

                    if owner_rank == self.rank as usize {
                        for &idx in &[left_idx, right_idx] {
                            if group_proofs[idx].is_none() {
                                let tag =
                                    if idx == left_idx { i_proof * 2 + n_groups } else { i_proof * 2 + n_groups + 1 };
                                let (mut msg, _status) =
                                    self.world.any_process().receive_vec_with_tag::<i8>(tag as i32);
                                group_proofs[idx] = Some(deserialize_zkin_proof_c(msg.as_mut_ptr()));
                            }
                        }
                    } else {
                        for &idx in &[left_idx, right_idx] {
                            if group_proofs[idx].is_some() {
                                let tag =
                                    if idx == left_idx { i_proof * 2 + n_groups } else { i_proof * 2 + n_groups + 1 };
                                let (ptr, size) = get_serialized_proof_c(group_proofs[idx].unwrap());
                                let buffer = unsafe { std::slice::from_raw_parts(ptr as *const i8, size as usize) };
                                self.world.process_at_rank(owner_rank as i32).send_with_tag(buffer, tag as i32);
                                zkin_proof_free_c(group_proofs[idx].unwrap());
                                group_proofs[idx] = None;
                            }
                        }
                    }
                    i_proof += 1;
                }
            }
        }
    }
}

impl Default for DistributionCtx {
    fn default() -> Self {
        DistributionCtx::new()
    }
}
unsafe impl Send for DistributionCtx {}
unsafe impl Sync for DistributionCtx {}
