//! `zisk_precompile!` macro — generates the Manager / Planner / Instance /
//! Collector / CounterInputGen shells for a precompile state-machine
//! component.
//!
//! ## Why this exists
//!
//! ZisK's uniform precompiles (`blake2`, `blake3`, `keccakf`, `sha256f`,
//! `poseidon2`, `add256`, `arith_eq`, `arith_eq_384`, `babyjubjub`) share
//! byte-isomorphic shell code. This module generates that boilerplate from
//! a small declarative invocation.
//!
//! `arith_eq` is deliberately not one of them: its config airs cover *different*
//! sets of operations, so choosing between them is a per-operation decision this
//! macro does not model. It has a hand-written family in
//! `precompiles/arith_eq/src/arith_eq_family.rs` instead.
//!
//! ## Two macros: façade + explicit
//!
//! [`zisk_precompile!`](crate::zisk_precompile) is the **façade**: `name`, `op_type`, the airs and
//! the `ops` list. It derives every shell name from `$name` and every
//! trace-related type from the trace aliases, then forwards to the explicit
//! form.
//!
//! [`zisk_precompile_explicit!`] is the **explicit form**: takes every name
//! spelled out, including an `airs = [...]` list with the air ids, heights,
//! capacities and row types. Use this directly only if your component cannot
//! follow the `${name}SM` / `${name}Input` / `${trace}Row` / `${trace}RowPacked`
//! convention — e.g. the SM type lives in another crate, or the trace type
//! doesn't follow the `*Row` / `*RowPacked` suffix pattern. All seven in-tree
//! precompiles use the façade.
//!
//! ## One air, or a size ladder
//!
//! A precompile may be instantiated at more than one height: the same air with a
//! taller `Large` sibling that commits the same columns over more rows. The
//! façade takes either shape:
//!
//! ```text
//! trace = Blake2brTrace, num_available = <ops per instance>          // one air
//!
//! row = ArithEq384Trace,                                            // a ladder
//! traces = [ (ArithEq384Trace, <ops>), (ArithEq384LargeTrace, <ops>) ],
//! ```
//!
//! The airs of a ladder must prove the *same* operations under the same
//! `op_type` — they are one air at several heights, not several airs. The
//! generated planner sizes them together under the shared criterion (fewest
//! instances first, least area to break a tie; see [`select_sizes`](crate::select_sizes))
//! and then cuts the operations at each granted instance's own capacity with
//! [`plan_ladder`](crate::plan_ladder).
//!
//! `row` sits outside the list because a ladder's airs share their row layout:
//! only the trace differs, carrying that air's `NUM_ROWS` and `AIR_ID`.
//!
//! ## Mono-op vs multi-op
//!
//! A *mono-op* precompile owns exactly one ZiskOp under its `op_type`
//! (e.g. `blake2`). A *multi-op* precompile (e.g. `arith_eq`) owns several
//! ZiskOps that share an AIR. The macro treats mono-op as the degenerate
//! 1-element case of multi-op. Each entry in the `ops = [...]` list is a
//! tuple of the form
//! `(ExtVariant, [EnumVariant =>] SubInputType)`:
//!
//! * mono-op (no enum wrapping): `(OperationBlake2Data, Blake2Input)`
//! * multi-op (enum-wrapped):    `(OperationArith256Data => Arith256, Arith256Input)`
//!
//! When the optional `=> EnumVariant` is present, the per-op input gets
//! wrapped into the aggregate enum named by `input = ...`.
//!
//! ## Usage
//!
//! See `precompiles/blake2/src/lib.rs` (mono-op, one air),
//! `precompiles/babyjubjub/src/lib.rs` (multi-op) and
//! `precompiles/arith_eq_384/src/lib.rs` (multi-op over a size ladder) for
//! canonical invocations.

/// Re-export of `paste::paste!` so consumers don't need a direct dep.
pub use paste::paste as __zisk_paste;

/// Explicit form — generates the per-precompile shell types from
/// spelled-out names. The [`zisk_precompile!`](crate::zisk_precompile) façade desugars to this.
///
/// Prefer the [`zisk_precompile!`](crate::zisk_precompile) façade unless you need to override one
/// of the conventional names. The façade derives all of the args this
/// macro takes from `$name` plus the trace aliases.
///
/// Generated symbols (with `name = Foo`):
///
/// * `FooManager<F>` — wraps `Arc<$sm<F>>`; impls `ComponentPlanBuilder<F>` + `ComponentBuilder<F>`
/// * `FooPlanner<F>` — sizes the airs as one ladder and cuts the operations; impls `Planner`
/// * `FooInstance<F>` — wraps `Arc<$sm<F>>` + `InstanceCtx`; impls `Instance<F>`
/// * `FooCollector` — input collector during witness gen; impls `BusDevice<PayloadType>`
/// * `FooCounterInputGen<F>` — bus device for Counter / CounterAsm / InputGenerator
///   modes; impls `BusDevice<u64>`, `Metrics`, `Add`
///
/// The `airs = [...]` list carries one entry per height the precompile is
/// instantiated at, each with:
///
/// * `air_id` / `air_group_id` — where the air lives in the pilout;
/// * `num_rows` — its height, which prices its area for the tie-break;
/// * `num_available` — a *compile-time expression* for the operations one of its
///   instances holds. Consumed by the static `ComponentPlanBuilder::planner()`
///   impl, so no constructed SM is required for the planning phase;
/// * `cost` — that air's `*_INSTANCE_COST` constant from `zisk_pil`. Named rather
///   than looked up by `air_id`, since air ids are positional;
/// * `row` / `row_packed` — the row layout, shared by every air of the ladder.
///
/// The SM (`$sm<F>`) must:
///
/// * impl `zisk_precomp_common::PrecompileMemInputs` (counter dispatches
///   per-op generation through this trait).
/// * expose `compute_witness::<R, const NUM_ROWS, const AIR_ID>(&self,
///   _sctx: &SetupCtx<F>, inputs: &[Vec<$input>], buf: Vec<F>)
///   -> ProofmanResult<AirInstance<F>>`, building its trace from those two
///   consts so one body serves every height.
#[macro_export]
macro_rules! zisk_precompile_explicit {
    (
        name = $name:ident,
        sm = $sm:path,
        op_type = $op_type:ident,
        input = $input:path,
        airs = [
            $(
                {
                    air_id = $air_id_path:expr,
                    air_group_id = $air_group_id_path:expr,
                    num_rows = $air_num_rows:expr,
                    num_available = $num_available:expr,
                    cost = $air_cost:expr,
                    row = $trace_row:path,
                    row_packed = $trace_row_packed:path $(,)?
                }
            ),+ $(,)?
        ],
        ops = [
            $(
                (
                    $ext_variant:ident
                    $( => $enum_variant:ident )?
                    , $sub_input:ident
                )
            ),* $(,)?
        ] $(,)?
    ) => {
        $crate::__zisk_paste! {
            // ============================================================
            // Manager
            // ============================================================
            #[allow(dead_code)]
            pub struct [<$name Manager>]<F: ::proofman_fields::PrimeField64> {
                [<$name:snake _sm>]: ::std::sync::Arc<$sm<F>>,
            }

            impl<F: ::proofman_fields::PrimeField64> [<$name Manager>]<F> {
                pub fn new(std: ::std::sync::Arc<::pil2_std_lib::Std<F>>) -> ::std::sync::Arc<Self> {
                    let [<$name:snake _sm>] = <$sm<F>>::new(std);
                    ::std::sync::Arc::new(Self { [<$name:snake _sm>] })
                }
            }

            impl<F: ::proofman_fields::PrimeField64> $crate::ComponentPlanBuilder<F>
                for [<$name Manager>]<F>
            {
                type Counter = [<$name CounterInputGen>]<F>;

                fn counter(is_asm_emulator: ::std::primitive::bool) -> Self::Counter {
                    let mode = if is_asm_emulator {
                        $crate::BusDeviceMode::CounterAsm
                    } else {
                        $crate::BusDeviceMode::Counter
                    };
                    [<$name CounterInputGen>]::<F>::new(mode)
                }

                fn planner(
                    _is_asm_emulator: ::std::primitive::bool,
                ) -> ::std::boxed::Box<dyn $crate::Planner> {
                    let mut planner = [<$name Planner>]::<F>::new();
                    $(
                        let num_available: ::std::primitive::usize = $num_available;
                        planner = planner.add_instance($crate::InstanceInfo::new(
                            $air_group_id_path,
                            $air_id_path,
                            num_available,
                            ::zisk_core::ZiskOperationType::$op_type,
                        ));
                    )+
                    ::std::boxed::Box::new(planner)
                }
            }

            impl<F: ::proofman_fields::PrimeField64> $crate::ComponentBuilder<F>
                for [<$name Manager>]<F>
            {
                fn build_instance(
                    &self,
                    ictx: $crate::InstanceCtx,
                ) -> ::std::boxed::Box<dyn $crate::Instance<F>> {
                    let air_id = ictx.plan.air_id;
                    if [ $( $air_id_path ),+ ].contains(&air_id) {
                        return ::std::boxed::Box::new(
                            [<$name Instance>]::new(self.[<$name:snake _sm>].clone(), ictx),
                        );
                    }
                    panic!(
                        concat!(stringify!($name), "Manager::build_instance() Unsupported air_id: {:?}"),
                        air_id,
                    )
                }
            }

            // ============================================================
            // Planner
            // ============================================================
            pub struct [<$name Planner>]<F: ::proofman_fields::PrimeField64> {
                instances_info: ::std::vec::Vec<$crate::InstanceInfo>,
                tables_info: ::std::vec::Vec<$crate::TableInfo>,
                _phantom: ::std::marker::PhantomData<F>,
            }

            impl<F: ::proofman_fields::PrimeField64> ::std::default::Default for [<$name Planner>]<F> {
                fn default() -> Self {
                    Self::new()
                }
            }

            impl<F: ::proofman_fields::PrimeField64> [<$name Planner>]<F> {
                pub fn new() -> Self {
                    Self {
                        instances_info: ::std::vec::Vec::new(),
                        tables_info: ::std::vec::Vec::new(),
                        _phantom: ::std::marker::PhantomData,
                    }
                }

                pub fn add_instance(mut self, instance_info: $crate::InstanceInfo) -> Self {
                    self.instances_info.push(instance_info);
                    self
                }

                /// Adds a table instance to the planner.
                ///
                /// No caller today: every ZisK table is currently a *virtual* table, planned apart
                /// from the instances. It is kept because that is a property of the current PIL,
                /// not of the design — a table air that is not virtual would be planned right here,
                /// and `plan` already emits one `InstanceType::Table` plan per entry.
                pub fn add_table_instance(mut self, table_info: $crate::TableInfo) -> Self {
                    self.tables_info.push(table_info);
                    self
                }
            }

            impl<F: ::proofman_fields::PrimeField64> $crate::Planner for [<$name Planner>]<F> {
                /// Plans the precompile's airs as one size ladder.
                ///
                /// The airs of a precompile prove the very same operations and differ only in how
                /// many of them one instance holds, so they are sized together under the shared
                /// criterion — fewest instances first, least area to break a tie (see
                /// [`select_sizes`](crate::select_sizes)) — and the operations are then cut at each
                /// granted instance's own capacity.
                fn plan(
                    &self,
                    counters: ::std::vec::Vec<(
                        $crate::ChunkId,
                        ::std::boxed::Box<dyn $crate::BusDeviceMetrics>,
                    )>,
                ) -> ::std::vec::Vec<$crate::Plan> {
                    // One count per chunk serves every air, because the airs are one air at
                    // several heights: they prove the same operations under the same op_type. An
                    // air with a different op_type would be silently planned against these counts.
                    let op_type = self.instances_info[0].op_type;
                    debug_assert!(
                        self.instances_info.iter().all(|info| info.op_type == op_type),
                        concat!(
                            stringify!($name),
                            "Planner: the airs of a precompile are one size ladder, so they must \
                             share their op_type",
                        ),
                    );

                    let mut count: ::std::vec::Vec<$crate::InstCount> =
                        ::std::vec::Vec::with_capacity(counters.len());

                    counters.iter().for_each(|(chunk_id, counter)| {
                        let reg_counter = $crate::Metrics::as_any(&**counter)
                            .downcast_ref::<[<$name CounterInputGen>]<F>>()
                            .unwrap();
                        count.push($crate::InstCount::new(
                            *chunk_id,
                            reg_counter.inst_count(op_type).unwrap(),
                        ));
                    });

                    // Capacity and area of each air, so the sizing can trade one against the other.
                    let ladder: ::std::vec::Vec<$crate::AirChoice> = self
                        .instances_info
                        .iter()
                        .zip([ $( $air_cost ),+ ])
                        .map(|(info, cost)| $crate::AirChoice {
                            airgroup_id: info.airgroup_id,
                            air_id: info.air_id,
                            rows: info.num_ops as u64,
                            area: cost as u64,
                        })
                        .collect();

                    let total: u64 = count.iter().map(|c| c.inst_count).sum();
                    let granted = $crate::select_sizes(total, &ladder);

                    // Fill the roomiest instances first, so the tail is what lands in the small one
                    // the sizing demoted.
                    let mut order: ::std::vec::Vec<usize> = (0..ladder.len()).collect();
                    order.sort_by_key(|&i| ::std::cmp::Reverse(ladder[i].rows));
                    let mut instance_air: ::std::vec::Vec<usize> = ::std::vec::Vec::new();
                    let mut capacities: ::std::vec::Vec<u64> = ::std::vec::Vec::new();
                    for air in order {
                        for _ in 0..granted[air] {
                            instance_air.push(air);
                            capacities.push(ladder[air].rows);
                        }
                    }

                    let mut plan_result: ::std::vec::Vec<$crate::Plan> = $crate::plan_ladder(
                        &count,
                        &capacities,
                    )
                    .into_iter()
                    .map(|(instance, check_point, collect_info)| {
                        let air = ladder[instance_air[instance]];
                        $crate::Plan::new(
                            air.airgroup_id,
                            air.air_id,
                            None,
                            $crate::InstanceType::Instance,
                            check_point,
                            Some(::std::boxed::Box::new(collect_info)),
                        )
                    })
                    .collect();

                    if !plan_result.is_empty() {
                        for table_instance in self.tables_info.iter() {
                            plan_result.push($crate::Plan::new(
                                table_instance.airgroup_id,
                                table_instance.air_id,
                                None,
                                $crate::InstanceType::Table,
                                $crate::CheckPoint::None,
                                None,
                            ));
                        }
                    }

                    plan_result
                }
            }

            // ============================================================
            // Instance
            // ============================================================
            pub struct [<$name Instance>]<F: ::proofman_fields::PrimeField64> {
                [<$name:snake _sm>]: ::std::sync::Arc<$sm<F>>,
                ictx: $crate::InstanceCtx,
            }

            impl<F: ::proofman_fields::PrimeField64> [<$name Instance>]<F> {
                pub fn new(
                    [<$name:snake _sm>]: ::std::sync::Arc<$sm<F>>,
                    ictx: $crate::InstanceCtx,
                ) -> Self {
                    Self { [<$name:snake _sm>], ictx }
                }

                pub fn [<build_ $name:snake _collector>](
                    &self,
                    chunk_id: $crate::ChunkId,
                ) -> [<$name Collector>] {
                    assert!(
                        [ $( $air_id_path ),+ ].contains(&self.ictx.plan.air_id),
                        concat!(stringify!($name), "Instance: Unsupported air_id: {:?}"),
                        self.ictx.plan.air_id,
                    );
                    let meta = self.ictx.plan.meta.as_ref().unwrap();
                    let collect_info = meta
                        .downcast_ref::<::std::collections::HashMap<
                            $crate::ChunkId,
                            (u64, $crate::CollectSkipper),
                        >>()
                        .unwrap();
                    let (num_ops, collect_skipper) = collect_info[&chunk_id];
                    [<$name Collector>]::new(num_ops, collect_skipper)
                }
            }

            impl<F: ::proofman_fields::PrimeField64> $crate::Instance<F> for [<$name Instance>]<F> {
                fn compute_witness(
                    &self,
                    _pctx: &::proofman_common::ProofCtx<F>,
                    _sctx: &::proofman_common::SetupCtx<F>,
                    collectors: ::std::vec::Vec<(
                        usize,
                        ::std::boxed::Box<dyn $crate::BusDevice<$crate::PayloadType>>,
                    )>,
                    trace_buffer: ::std::vec::Vec<F>,
                    packed: bool,
                ) -> ::proofman_common::ProofmanResult<
                    ::std::option::Option<::proofman_common::AirInstance<F>>,
                > {
                    let inputs: ::std::vec::Vec<_> = collectors
                        .into_iter()
                        .map(|(_, collector)| {
                            collector
                                .as_any()
                                .downcast::<[<$name Collector>]>()
                                .unwrap()
                                .inputs
                        })
                        .collect();

                    // The airs of the ladder commit the same columns and differ only in height, so
                    // the row type is shared and the height and air id are what select the air.
                    let air_id = self.ictx.plan.air_id;
                    $(
                        if air_id == $air_id_path {
                            return if packed {
                                Ok(Some(self.[<$name:snake _sm>].compute_witness::<
                                    $trace_row_packed<F>,
                                    { $air_num_rows },
                                    { $air_id_path },
                                >(_sctx, &inputs, trace_buffer)?))
                            } else {
                                Ok(Some(self.[<$name:snake _sm>].compute_witness::<
                                    $trace_row<F>,
                                    { $air_num_rows },
                                    { $air_id_path },
                                >(_sctx, &inputs, trace_buffer)?))
                            };
                        }
                    )+
                    panic!(
                        concat!(stringify!($name), "Instance: Unsupported air_id: {:?}"),
                        air_id,
                    )
                }

                fn check_point(&self) -> &$crate::CheckPoint {
                    &self.ictx.plan.check_point
                }

                fn instance_type(&self) -> $crate::InstanceType {
                    $crate::InstanceType::Instance
                }

                fn stats_type(&self) -> $crate::StatsType {
                    $crate::StatsType::Precompiled
                }

                fn build_inputs_collector(
                    &self,
                    chunk_id: $crate::ChunkId,
                ) -> ::std::option::Option<
                    ::std::boxed::Box<dyn $crate::BusDevice<$crate::PayloadType>>,
                > {
                    assert!(
                        [ $( $air_id_path ),+ ].contains(&self.ictx.plan.air_id),
                        concat!(stringify!($name), "Instance: Unsupported air_id: {:?}"),
                        self.ictx.plan.air_id,
                    );
                    let meta = self.ictx.plan.meta.as_ref().unwrap();
                    let collect_info = meta
                        .downcast_ref::<::std::collections::HashMap<
                            $crate::ChunkId,
                            (u64, $crate::CollectSkipper),
                        >>()
                        .unwrap();
                    let (num_ops, collect_skipper) = collect_info[&chunk_id];
                    Some(::std::boxed::Box::new(
                        [<$name Collector>]::new(num_ops, collect_skipper),
                    ))
                }

                fn as_any(&self) -> &dyn ::std::any::Any {
                    self
                }
            }

            // ============================================================
            // Collector (witness-gen input gathering)
            //
            // For each ops entry, pushes the per-op input into `inputs`.
            // The optional 2nd tuple element (`$enum_variant`) controls
            // whether the per-op input gets wrapped into an aggregate
            // enum variant — present for multi-op, absent for mono-op.
            // ============================================================
            pub struct [<$name Collector>] {
                inputs: ::std::vec::Vec<$input>,
                num_operations: u64,
                collect_skipper: $crate::CollectSkipper,
            }

            impl [<$name Collector>] {
                pub fn new(num_operations: u64, collect_skipper: $crate::CollectSkipper) -> Self {
                    Self {
                        inputs: ::std::vec::Vec::with_capacity(num_operations as usize),
                        num_operations,
                        collect_skipper,
                    }
                }

                #[inline(always)]
                pub fn process_data(
                    &mut self,
                    bus_id: &$crate::BusId,
                    data: &[$crate::PayloadType],
                ) -> bool {
                    debug_assert!(*bus_id == $crate::OPERATION_BUS_ID);

                    if self.inputs.len() == self.num_operations as usize {
                        return false;
                    }

                    if data[$crate::OP_TYPE] as u32
                        != ::zisk_core::ZiskOperationType::$op_type as u32
                    {
                        return true;
                    }

                    if self.collect_skipper.should_skip() {
                        return true;
                    }

                    let data: $crate::ExtOperationData<u64> =
                        data.try_into().expect("Regular Metrics: Failed to convert data");

                    self.inputs.push(match data {
                        $(
                            $crate::ExtOperationData::$ext_variant(bus_data) => {
                                let __converted = $sub_input::from(&bus_data);
                                $( let __converted = <$input>::$enum_variant(__converted); )?
                                __converted
                            }
                        )*
                        _ => panic!(concat!(
                            stringify!($name),
                            "Collector: unexpected ExtOperationData variant",
                        )),
                    });

                    self.inputs.len() < self.num_operations as usize
                }
            }

            impl $crate::BusDevice<$crate::PayloadType> for [<$name Collector>] {
                fn as_any(self: ::std::boxed::Box<Self>) -> ::std::boxed::Box<dyn ::std::any::Any> {
                    self
                }
            }

            // ============================================================
            // CounterInputGen (Counter / CounterAsm / InputGenerator modes)
            // Dispatches to `<$sm<F> as PrecompileMemInputs>::generate /
            // should_skip`. The SM's PrecompileMemInputs impl handles any
            // inner sub-op match for multi-op precompiles.
            // ============================================================
            pub struct [<$name CounterInputGen>]<F: ::proofman_fields::PrimeField64> {
                counter: $crate::Counter,
                mode: $crate::BusDeviceMode,
                _phantom: ::std::marker::PhantomData<F>,
            }

            impl<F: ::proofman_fields::PrimeField64> [<$name CounterInputGen>]<F> {
                pub fn new(mode: $crate::BusDeviceMode) -> Self {
                    Self {
                        counter: $crate::Counter::default(),
                        mode,
                        _phantom: ::std::marker::PhantomData,
                    }
                }

                pub fn inst_count(
                    &self,
                    op_type: ::zisk_core::ZiskOperationType,
                ) -> ::std::option::Option<u64> {
                    (op_type == ::zisk_core::ZiskOperationType::$op_type)
                        .then_some(self.counter.inst_count)
                }

                #[inline(always)]
                pub fn process_data<P: ::zisk_precomp_common::MemProcessor>(
                    &mut self,
                    bus_id: &$crate::BusId,
                    data: &[u64],
                    mem_processors: &mut P,
                ) -> bool {
                    debug_assert!(*bus_id == $crate::OPERATION_BUS_ID);

                    if data[$crate::OP_TYPE] as u32
                        != ::zisk_core::ZiskOperationType::$op_type as u32
                    {
                        return true;
                    }

                    let step_main = data[$crate::STEP];
                    let addr_main = data[$crate::B] as u32;

                    match self.mode {
                        $crate::BusDeviceMode::Counter => {
                            $crate::Metrics::measure(self, data);
                            <$sm<F> as ::zisk_precomp_common::PrecompileMemInputs>::generate(
                                addr_main, step_main, data, true, mem_processors,
                            );
                        }
                        $crate::BusDeviceMode::CounterAsm => {
                            $crate::Metrics::measure(self, data);
                        }
                        $crate::BusDeviceMode::InputGenerator => {
                            if <$sm<F> as ::zisk_precomp_common::PrecompileMemInputs>::should_skip(
                                addr_main, data, mem_processors,
                            ) {
                                return true;
                            }
                            <$sm<F> as ::zisk_precomp_common::PrecompileMemInputs>::generate(
                                addr_main, step_main, data, false, mem_processors,
                            );
                        }
                    }

                    true
                }
            }

            impl<F: ::proofman_fields::PrimeField64> $crate::Metrics for [<$name CounterInputGen>]<F> {
                #[inline(always)]
                fn measure(&mut self, _data: &[u64]) {
                    self.counter.update(1);
                }

                fn as_any(&self) -> &dyn ::std::any::Any {
                    self
                }
            }

            impl<F: ::proofman_fields::PrimeField64> ::std::ops::Add for [<$name CounterInputGen>]<F> {
                type Output = [<$name CounterInputGen>]<F>;

                fn add(self, other: Self) -> [<$name CounterInputGen>]<F> {
                    [<$name CounterInputGen>] {
                        counter: &self.counter + &other.counter,
                        mode: self.mode,
                        _phantom: ::std::marker::PhantomData,
                    }
                }
            }

            impl<F: ::proofman_fields::PrimeField64> $crate::BusDevice<u64> for [<$name CounterInputGen>]<F> {
                fn as_any(self: ::std::boxed::Box<Self>) -> ::std::boxed::Box<dyn ::std::any::Any> {
                    self
                }
            }
        }
    };
}

/// Façade — declares a precompile's shells from `name`, `op_type`, its airs and
/// the ops list.
///
/// Two forms, and the short one is the common case:
///
/// ```text
/// trace = FooTrace,                                          // a single air
/// num_available = <ops per instance>,
/// cost = FOO_INSTANCE_COST,
///
/// row = FooTrace,                                           // a size ladder
/// traces = [
///     (FooTrace,      <ops>, FOO_INSTANCE_COST),
///     (FooLargeTrace, <ops>, FOO_LARGE_INSTANCE_COST),
/// ],
/// ```
///
/// The short form reexpands to the long one with a single entry, so there is one
/// body to maintain. `row` is separate from `traces` because the airs of a ladder
/// commit the same columns and therefore share their row layout.
///
/// Derives the args of [`zisk_precompile_explicit!`] from `$name` and the trace
/// aliases:
///
/// * `sm = ${name}SM`
/// * `input = ${name}Input`
/// * `row = ::zisk_pil::${row}Row`, `row_packed = ::zisk_pil::${row}RowPacked`
/// * per air: `air_id`, `air_group_id` and `num_rows` from
///   `::zisk_pil::${trace}::<()>`
///
/// If your component breaks any of these conventions, call
/// [`zisk_precompile_explicit!`] directly and override the offending name.
#[macro_export]
macro_rules! zisk_precompile {
    (
        name = $name:ident,
        op_type = $op_type:ident,
        row = $row_trace:ident,
        traces = [ $( ($trace:ident, $num_available:expr, $air_cost:expr) ),+ $(,)? ],
        ops = [
            $(
                (
                    $ext_variant:ident
                    $( => $enum_variant:ident )?
                    , $sub_input:ident
                )
            ),* $(,)?
        ] $(,)?
    ) => {
        $crate::__zisk_paste! {
            $crate::zisk_precompile_explicit! {
                name = $name,
                sm = [<$name SM>],
                op_type = $op_type,
                input = [<$name Input>],
                airs = [
                    $(
                        {
                            air_id = ::zisk_pil::$trace::<()>::AIR_ID,
                            air_group_id = ::zisk_pil::$trace::<()>::AIRGROUP_ID,
                            num_rows = ::zisk_pil::$trace::<()>::NUM_ROWS,
                            num_available = $num_available,
                            cost = $air_cost,
                            row = ::zisk_pil::[<$row_trace Row>],
                            row_packed = ::zisk_pil::[<$row_trace RowPacked>],
                        }
                    ),+
                ],
                ops = [
                    $(
                        ( $ext_variant $( => $enum_variant )? , $sub_input )
                    ),*
                ],
            }
        }
    };

    // Single-air form: the common case, where the precompile has no `Large` sibling.
    (
        name = $name:ident,
        op_type = $op_type:ident,
        trace = $trace:ident,
        num_available = $num_available:expr,
        cost = $air_cost:expr,
        ops = [
            $(
                (
                    $ext_variant:ident
                    $( => $enum_variant:ident )?
                    , $sub_input:ident
                )
            ),* $(,)?
        ] $(,)?
    ) => {
        $crate::zisk_precompile! {
            name = $name,
            op_type = $op_type,
            row = $trace,
            traces = [ ($trace, $num_available, $air_cost) ],
            ops = [
                $(
                    ( $ext_variant $( => $enum_variant )? , $sub_input )
                ),*
            ],
        }
    };
}
