//! `zisk_precompile!` macro — generates the Manager / Planner / Instance /
//! Collector / CounterInputGen shells for a precompile state-machine
//! component.
//!
//! ## Why this exists
//!
//! ZisK's seven uniform precompiles (`blake2`, `keccakf`, `sha256f`,
//! `poseidon2`, `add256`, `arith_eq`, `arith_eq_384`) share byte-isomorphic
//! shell code. This module generates that boilerplate from a small
//! declarative invocation.
//!
//! ## Two macros: façade + explicit
//!
//! [`zisk_precompile!`] is the **façade**: 4 declarative parameters
//! (`name`, `op_type`, `trace`, `num_available_field`) plus the `ops`
//! list. It derives every shell name from `$name` and every trace-related
//! type from `$trace`, then forwards to the explicit form.
//!
//! [`zisk_precompile_explicit!`] is the **explicit form**: takes every
//! name spelled out (SM type, input type, trace row + packed, AIR id
//! paths). Use this directly only if your component cannot follow the
//! `${name}SM` / `${name}Input` / `${trace}Row` / `${trace}RowPacked`
//! convention — e.g. the SM type lives in another crate, or the trace
//! type doesn't follow the `*Row` / `*RowPacked` suffix pattern. All
//! seven in-tree precompiles use the façade.
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
//! See `precompiles/blake2/src/lib.rs` (mono-op) and
//! `precompiles/arith_eq/src/lib.rs` (multi-op) for canonical invocations.

/// Re-export of `paste::paste!` so consumers don't need a direct dep.
pub use paste::paste as __zisk_paste;

/// Explicit form — generates the per-precompile shell types from
/// spelled-out names. The [`zisk_precompile!`] façade desugars to this.
///
/// Prefer the [`zisk_precompile!`] façade unless you need to override one
/// of the conventional names. The façade derives all of the args this
/// macro takes from just `$name` + `$trace`.
///
/// Generated symbols (with `name = Foo`):
///
/// * `FooManager<F>` — wraps `Arc<$sm<F>>`; impls `ComponentBuilder<F>`
/// * `FooPlanner<F>` — single-instance planner; impls `Planner`
/// * `FooInstance<F>` — wraps `Arc<$sm<F>>` + `InstanceCtx`; impls `Instance<F>`
/// * `FooCollector` — input collector during witness gen; impls `BusDevice<PayloadType>`
/// * `FooCounterInputGen<F>` — bus device for Counter / CounterAsm / InputGenerator
///   modes; impls `BusDevice<u64>`, `Metrics`, `Add`
///
/// The associated method names (e.g. `build_foo_counter`, `foo_sm` field)
/// are derived from `$name` via `paste`'s `:snake` case conversion.
///
/// The SM (`$sm<F>`) must:
///
/// * impl `precompiles_common::PrecompileMemInputs` (counter dispatches
///   per-op generation through this trait).
/// * expose `compute_witness::<R>(&self, _sctx: &SetupCtx<F>,
///   inputs: &[Vec<$input>], buf: Vec<F>) -> ProofmanResult<AirInstance<F>>`.
/// * expose a `pub $num_available_field: usize` field giving the number of
///   ops a single instance can hold.
#[macro_export]
macro_rules! zisk_precompile_explicit {
    (
        name = $name:ident,
        sm = $sm:path,
        op_type = $op_type:ident,
        input = $input:path,
        trace_row = $trace_row:path,
        trace_row_packed = $trace_row_packed:path,
        air_id_path = $air_id_path:path,
        air_group_id_path = $air_group_id_path:path,
        num_available_field = $num_available_field:ident,
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
            pub struct [<$name Manager>]<F: ::fields::PrimeField64> {
                [<$name:snake _sm>]: ::std::sync::Arc<$sm<F>>,
            }

            impl<F: ::fields::PrimeField64> [<$name Manager>]<F> {
                pub fn new(std: ::std::sync::Arc<::pil_std_lib::Std<F>>) -> ::std::sync::Arc<Self> {
                    let [<$name:snake _sm>] = <$sm<F>>::new(std);
                    ::std::sync::Arc::new(Self { [<$name:snake _sm>] })
                }

                pub fn [<build_ $name:snake _counter>](
                    &self,
                    asm_execution: bool,
                ) -> [<$name CounterInputGen>]<F> {
                    match asm_execution {
                        true => [<$name CounterInputGen>]::<F>::new(
                            $crate::BusDeviceMode::CounterAsm,
                        ),
                        false => [<$name CounterInputGen>]::<F>::new(
                            $crate::BusDeviceMode::Counter,
                        ),
                    }
                }

                pub fn [<build_ $name:snake _input_generator>](
                    &self,
                ) -> [<$name CounterInputGen>]<F> {
                    [<$name CounterInputGen>]::<F>::new(
                        $crate::BusDeviceMode::InputGenerator,
                    )
                }
            }

            impl<F: ::fields::PrimeField64> $crate::ComponentBuilder<F>
                for [<$name Manager>]<F>
            {
                fn build_planner(&self) -> ::std::boxed::Box<dyn $crate::Planner> {
                    let num_available = self.[<$name:snake _sm>].$num_available_field;
                    ::std::boxed::Box::new(
                        [<$name Planner>]::<F>::new().add_instance(
                            $crate::InstanceInfo::new(
                                $air_group_id_path,
                                $air_id_path,
                                num_available,
                                ::zisk_core::ZiskOperationType::$op_type,
                            ),
                        ),
                    )
                }

                fn build_instance(
                    &self,
                    ictx: $crate::InstanceCtx,
                ) -> ::std::boxed::Box<dyn $crate::Instance<F>> {
                    match ictx.plan.air_id {
                        id if id == $air_id_path => ::std::boxed::Box::new(
                            [<$name Instance>]::new(self.[<$name:snake _sm>].clone(), ictx),
                        ),
                        _ => panic!(
                            concat!(stringify!($name), "Manager::build_instance() Unsupported air_id: {:?}"),
                            ictx.plan.air_id,
                        ),
                    }
                }
            }

            // ============================================================
            // Planner
            // ============================================================
            pub struct [<$name Planner>]<F: ::fields::PrimeField64> {
                instances_info: ::std::vec::Vec<$crate::InstanceInfo>,
                tables_info: ::std::vec::Vec<$crate::TableInfo>,
                _phantom: ::std::marker::PhantomData<F>,
            }

            impl<F: ::fields::PrimeField64> ::std::default::Default for [<$name Planner>]<F> {
                fn default() -> Self {
                    Self::new()
                }
            }

            impl<F: ::fields::PrimeField64> [<$name Planner>]<F> {
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

                pub fn add_table_instance(mut self, table_info: $crate::TableInfo) -> Self {
                    self.tables_info.push(table_info);
                    self
                }
            }

            impl<F: ::fields::PrimeField64> $crate::Planner for [<$name Planner>]<F> {
                fn plan(
                    &self,
                    counters: ::std::vec::Vec<(
                        $crate::ChunkId,
                        ::std::boxed::Box<dyn $crate::BusDeviceMetrics>,
                    )>,
                ) -> ::std::vec::Vec<$crate::Plan> {
                    let mut count: ::std::vec::Vec<::std::vec::Vec<$crate::InstCount>> =
                        ::std::vec::Vec::with_capacity(self.instances_info.len());
                    for _ in 0..self.instances_info.len() {
                        count.push(::std::vec::Vec::new());
                    }

                    counters.iter().for_each(|(chunk_id, counter)| {
                        let reg_counter = $crate::Metrics::as_any(&**counter)
                            .downcast_ref::<[<$name CounterInputGen>]<F>>()
                            .unwrap();

                        for (index, instance_info) in self.instances_info.iter().enumerate() {
                            let inst_count = $crate::InstCount::new(
                                *chunk_id,
                                reg_counter.inst_count(instance_info.op_type).unwrap(),
                            );
                            count[index].push(inst_count);
                        }
                    });

                    let mut plan_result = ::std::vec::Vec::new();
                    for (idx, instance) in self.instances_info.iter().enumerate() {
                        let plan: ::std::vec::Vec<_> =
                            $crate::plan(&count[idx], instance.num_ops as u64)
                                .into_iter()
                                .map(|(check_point, collect_info)| {
                                    let converted: ::std::boxed::Box<dyn ::std::any::Any> =
                                        ::std::boxed::Box::new(collect_info);
                                    $crate::Plan::new(
                                        instance.airgroup_id,
                                        instance.air_id,
                                        None,
                                        $crate::InstanceType::Instance,
                                        check_point,
                                        Some(converted),
                                    )
                                })
                                .collect();
                        plan_result.extend(plan);
                    }

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
            pub struct [<$name Instance>]<F: ::fields::PrimeField64> {
                [<$name:snake _sm>]: ::std::sync::Arc<$sm<F>>,
                ictx: $crate::InstanceCtx,
            }

            impl<F: ::fields::PrimeField64> [<$name Instance>]<F> {
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
                    assert_eq!(
                        self.ictx.plan.air_id,
                        $air_id_path,
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

            impl<F: ::fields::PrimeField64> $crate::Instance<F> for [<$name Instance>]<F> {
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

                    if packed {
                        Ok(Some(self.[<$name:snake _sm>]
                            .compute_witness::<$trace_row_packed<F>>(_sctx, &inputs, trace_buffer)?))
                    } else {
                        Ok(Some(self.[<$name:snake _sm>]
                            .compute_witness::<$trace_row<F>>(_sctx, &inputs, trace_buffer)?))
                    }
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
                    assert_eq!(
                        self.ictx.plan.air_id,
                        $air_id_path,
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
            pub struct [<$name CounterInputGen>]<F: ::fields::PrimeField64> {
                counter: $crate::Counter,
                mode: $crate::BusDeviceMode,
                _phantom: ::std::marker::PhantomData<F>,
            }

            impl<F: ::fields::PrimeField64> [<$name CounterInputGen>]<F> {
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
                pub fn process_data<P: ::precompiles_common::MemProcessor>(
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
                            <$sm<F> as ::precompiles_common::PrecompileMemInputs>::generate(
                                addr_main, step_main, data, true, mem_processors,
                            );
                        }
                        $crate::BusDeviceMode::CounterAsm => {
                            $crate::Metrics::measure(self, data);
                        }
                        $crate::BusDeviceMode::InputGenerator => {
                            if <$sm<F> as ::precompiles_common::PrecompileMemInputs>::should_skip(
                                addr_main, data, mem_processors,
                            ) {
                                return true;
                            }
                            <$sm<F> as ::precompiles_common::PrecompileMemInputs>::generate(
                                addr_main, step_main, data, false, mem_processors,
                            );
                        }
                    }

                    true
                }
            }

            impl<F: ::fields::PrimeField64> $crate::Metrics for [<$name CounterInputGen>]<F> {
                #[inline(always)]
                fn measure(&mut self, _data: &[u64]) {
                    self.counter.update(1);
                }

                fn as_any(&self) -> &dyn ::std::any::Any {
                    self
                }
            }

            impl<F: ::fields::PrimeField64> ::std::ops::Add for [<$name CounterInputGen>]<F> {
                type Output = [<$name CounterInputGen>]<F>;

                fn add(self, other: Self) -> [<$name CounterInputGen>]<F> {
                    [<$name CounterInputGen>] {
                        counter: &self.counter + &other.counter,
                        mode: self.mode,
                        _phantom: ::std::marker::PhantomData,
                    }
                }
            }

            impl<F: ::fields::PrimeField64> $crate::BusDevice<u64> for [<$name CounterInputGen>]<F> {
                fn as_any(self: ::std::boxed::Box<Self>) -> ::std::boxed::Box<dyn ::std::any::Any> {
                    self
                }
            }
        }
    };
}

/// Façade — declares a precompile's shells using only `name`, `op_type`,
/// `trace`, and `num_available_field` plus the ops list.
///
/// Derives the args of [`zisk_precompile_explicit!`] from `$name` and `$trace`:
///
/// * `sm = ${name}SM`
/// * `input = ${name}Input`
/// * `trace_row = ::zisk_pil::${trace}Row`
/// * `trace_row_packed = ::zisk_pil::${trace}RowPacked`
/// * `air_id_path = ::zisk_pil::${trace}::<()>::AIR_ID`
/// * `air_group_id_path = ::zisk_pil::${trace}::<()>::AIRGROUP_ID`
///
/// If your component breaks any of these conventions, call
/// [`zisk_precompile_explicit!`] directly and override the offending name.
#[macro_export]
macro_rules! zisk_precompile {
    (
        name = $name:ident,
        op_type = $op_type:ident,
        trace = $trace:ident,
        num_available_field = $num_available_field:ident,
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
                trace_row = ::zisk_pil::[<$trace Row>],
                trace_row_packed = ::zisk_pil::[<$trace RowPacked>],
                air_id_path = ::zisk_pil::$trace::<()>::AIR_ID,
                air_group_id_path = ::zisk_pil::$trace::<()>::AIRGROUP_ID,
                num_available_field = $num_available_field,
                ops = [
                    $(
                        ( $ext_variant $( => $enum_variant )? , $sub_input )
                    ),*
                ],
            }
        }
    };
}
