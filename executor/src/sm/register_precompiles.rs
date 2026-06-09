//! `register_precompiles!` macro — single source of truth for every
//! precompile in the executor.
//!
//! ```ignore
//! register_precompiles! {
//!     Keccakf [
//!         op: KECCAK_OP_TYPE_ID,
//!         air: KECCAKF_AIR_IDS,
//!         rank_assign: true,
//!     ] => KeccakfManager<F>,
//!     // ... add yours here
//! }
//! ```
//!
//! Per-precompile fields:
//! * `op`     — `OP_TYPE_ID` constant from `zisk_core`. Drives bus dispatch.
//! * `air`    — `*_AIR_IDS` slice from `zisk_pil`. Drives planner/collector dispatch.
//! * `rank_assign` — `true` if instances need `pctx.add_instance_assign` (rank-owned)
//!   rather than the default `pctx.add_instance` (distributed). Today only `Keccakf` uses `true`.
//!
//! Per-precompile types derived via `paste!`: `${Name}Manager<F>`,
//! `${Name}CounterInputGen<F>`, `${Name}Instance<F>`, `${Name}Collector`.
//!
//! The `op:` constants must be in scope at the invocation site —
//! typically `KECCAK_OP_TYPE_ID` etc. from `zisk_core`. Built-in SMs
//! stay hand-written in [`super::builtins`] (their dispatch is irregular).

/// Declarative registry for precompiles.
///
/// Emits from one declaration per precompile:
/// * `enum Precompiles<F>` + `planner_for_air_id` / `configure_instances` /
///   `build_instance` dispatch.
/// * `PrecompileCounters<F>` + `build(is_asm)` (static) + `dispatch_op` + `into_device_entries`.
/// * `PrecompileCollectors<F>` + `new()` (static) + `try_push_collector` + `dispatch_op` +
///   `into_device_entries`.
/// * `__PrecompileSlot` enum giving each variant a `BUILTIN_COUNT + n` bundle position.
#[macro_export]
macro_rules! register_precompiles {
    (
        $(
            $variant:ident [
                op: $op:expr,
                air: $air:expr,
                rank_assign: $rank_assign:expr $(,)?
            ] => $mgr:ty
        ),* $(,)?
    ) => {
        /// Tagged union of every precompile state machine registered via
        /// [`register_precompiles!`](crate::register_precompiles). One variant
        /// per declaration.
        pub enum Precompiles<F: ::fields::PrimeField64> {
            $(
                #[doc = concat!(
                    "`", stringify!($variant),
                    "` precompile, backed by `", stringify!($mgr), "`.",
                )]
                $variant(::std::sync::Arc<$mgr>),
            )*
        }

        impl<F: ::fields::PrimeField64> Precompiles<F> {
            /// Static planner dispatch by AIR id — no instance needed.
            /// Used by the plan path without constructing the precompile.
            pub fn planner_for_air_id(
                air_id: ::std::primitive::usize,
                is_asm_emulator: ::std::primitive::bool,
            ) -> ::std::boxed::Box<dyn ::zisk_common::Planner> {
                match air_id {
                    $(
                        id if id == $air[0] => <$mgr as
                            ::zisk_common::ComponentPlanBuilder<F>>::planner(is_asm_emulator),
                    )*
                    _ => panic!("planner_for_air_id: unknown precompile air_id {air_id}"),
                }
            }

            /// Dispatches to the active variant's manager to register its
            /// instances on `pctx` for the supplied `plans`. See
            /// `ComponentBuilder::configure_instances`.
            pub fn configure_instances(
                &self,
                pctx: &::proofman_common::ProofCtx<F>,
                plans: &[::zisk_common::Plan],
            ) {
                match self {
                    $( Self::$variant(sm) => (**sm).configure_instances(pctx, plans), )*
                }
            }

            /// Dispatches to the active variant's manager and returns the
            /// witness `Instance` for `ictx`. See
            /// `ComponentBuilder::build_instance`.
            pub fn build_instance(
                &self,
                ictx: ::zisk_common::InstanceCtx,
            ) -> ::std::boxed::Box<dyn ::zisk_common::Instance<F>> {
                match self {
                    $( Self::$variant(sm) => (**sm).build_instance(ictx), )*
                }
            }

            /// Canonical default precompile set — one entry per registered
            /// variant. Macro-generated from the registration list; mirrors
            /// `BuiltinSMs::all` on the built-in side.
            pub(crate) fn all(
                std: ::std::sync::Arc<::pil_std_lib::Std<F>>,
            ) -> ::std::vec::Vec<(::std::primitive::usize, Self)> {
                ::std::vec![
                    $(
                        ($air[0], Self::$variant(<$mgr>::new(std.clone()))),
                    )*
                ]
            }
        }

        /// Variant slot index used to compute the bundle position
        /// (`BUILTIN_COUNT + slot as usize`) for each precompile.
        #[allow(dead_code, non_camel_case_types)]
        #[repr(usize)]
        enum __PrecompileSlot { $( $variant, )* }

        /// Air-id slice for every registered precompile, in declaration order.
        /// Macro-generated from the registration list.
        pub(crate) const PRECOMPILE_AIR_IDS: &[::std::primitive::usize] = &[
            $( $air[0], )*
        ];

        /// Parallel to [`PRECOMPILE_AIR_IDS`]: `true` for precompiles whose
        /// instances need `pctx.add_instance_assign` (rank-owned) instead of
        /// the default `pctx.add_instance` (distributed). Macro-generated.
        pub(crate) const PRECOMPILE_RANK_ASSIGN: &[::std::primitive::bool] = &[
            $( $rank_assign, )*
        ];

        ::paste::paste! {
            // ────────────────────────────────────────────────────────
            // PrecompileCounters<F> — counter-phase bus slots.
            // ────────────────────────────────────────────────────────

            /// Counter-phase slots for every precompile registered via
            /// `register_precompiles!`. Each field stores
            /// `(bundle_position, counter_input_gen)`.
            pub struct PrecompileCounters<F: ::fields::PrimeField64> {
                $(
                    #[doc = concat!(
                        "Counter for the `", stringify!($variant),
                        "` precompile: `(bundle_position, ",
                        stringify!($variant), "CounterInputGen<F>)`.",
                    )]
                    pub [<$variant:snake>]: (
                        ::std::primitive::usize,
                        [<$variant CounterInputGen>]<F>,
                    ),
                )*
            }

            impl<F: ::fields::PrimeField64> PrecompileCounters<F> {
                /// Build each precompile's counter via static dispatch.
                /// Bundle position is `BUILTIN_COUNT + slot`.
                pub fn build(is_asm_emulator: ::std::primitive::bool) -> Self {
                    Self {
                        $(
                            [<$variant:snake>]: (
                                $crate::BUILTIN_COUNT + __PrecompileSlot::$variant as usize,
                                <$mgr as ::zisk_common::ComponentPlanBuilder<F>>::counter(is_asm_emulator),
                            ),
                        )*
                    }
                }

                /// Counter-phase bus dispatch. Routes an operation by its
                /// `OP_TYPE_ID` to the matching precompile's counter
                /// `process_data` call. Returns the precompile's continue
                /// flag, or `true` if `op_type` does not match any registered
                /// precompile (caller handles built-in ops upstream).
                ///
                /// `mem_counter` is the parent bus's optional memory counter,
                /// passed through as `MemCounterProcessor::new(mem_counter)`
                /// for use by each precompile. `None` under the ASM emulator.
                #[inline(always)]
                pub fn dispatch_op(
                    &mut self,
                    op_type: ::std::primitive::u32,
                    bus_id: &::zisk_common::BusId,
                    data: &[::zisk_common::PayloadType],
                    mem_counter: ::std::option::Option<&mut ::mem_common::MemCounters>,
                ) -> ::std::primitive::bool {
                    $(
                        const [<__ $variant:upper _OP>]: ::std::primitive::u32 = $op;
                    )*
                    let mut mem_processor =
                        ::precompiles_common::MemCounterProcessor::new(mem_counter);
                    match op_type {
                        $(
                            [<__ $variant:upper _OP>] => self.[<$variant:snake>].1.process_data(
                                bus_id,
                                data,
                                &mut mem_processor,
                            ),
                        )*
                        _ => true,
                    }
                }

                /// Consumes the slots, producing the per-precompile
                /// entries that `StaticDataBus::into_devices` splices
                /// into its full device list. Order matches
                /// declaration order in `register_precompiles!`.
                pub fn into_device_entries(
                    self,
                ) -> ::std::vec::Vec<(
                    ::std::primitive::usize,
                    ::std::boxed::Box<dyn ::zisk_common::BusDeviceMetrics>,
                )> {
                    ::std::vec![
                        $(
                            (
                                self.[<$variant:snake>].0,
                                ::std::boxed::Box::new(self.[<$variant:snake>].1),
                            ),
                        )*
                    ]
                }
            }

            // ────────────────────────────────────────────────────────
            // PrecompileCollectors<F> — collector-phase bus slots.
            // ────────────────────────────────────────────────────────

            /// Collector-phase slots for every precompile registered
            /// via `register_precompiles!`. Per precompile: a
            /// `Vec<(global_idx, Collector)>` populated by
            /// `try_push_collector` during per-chunk setup, plus a
            /// `CounterInputGen` used by the bus to emit derived
            /// mem-ops on each operation message.
            pub struct PrecompileCollectors<F: ::fields::PrimeField64> {
                $(
                    #[doc = concat!(
                        "Per-chunk collectors for the `", stringify!($variant),
                        "` precompile, each tagged with the `global_idx` of the ",
                        "instance it feeds. Populated by `try_push_collector`.",
                    )]
                    pub [<$variant:snake _collector>]: ::std::vec::Vec<(
                        ::std::primitive::usize,
                        [<$variant Collector>],
                    )>,
                    #[doc = concat!(
                        "Input generator for the `", stringify!($variant),
                        "` precompile. Used by the bus to emit derived mem-ops ",
                        "on each matching operation message.",
                    )]
                    pub [<$variant:snake _inputs_generator>]: [<$variant CounterInputGen>]<F>,
                )*
            }

            impl<F: ::fields::PrimeField64> PrecompileCollectors<F> {
                pub fn new() -> Self {
                    Self {
                        $(
                            [<$variant:snake _collector>]: ::std::vec::Vec::new(),
                            [<$variant:snake _inputs_generator>]:
                                [<$variant CounterInputGen>]::<F>::new(
                                    ::zisk_common::BusDeviceMode::InputGenerator,
                                ),
                        )*
                    }
                }

                /// Per-chunk air-id dispatch. If `air_id` belongs to a
                /// registered precompile, downcasts `secn_instance` to
                /// that precompile's `*Instance<F>`, builds its
                /// collector for this chunk, and pushes it onto the
                /// matching vec.
                ///
                /// Returns:
                /// * `Ok(true)` — matched a precompile, pushed.
                /// * `Ok(false)` — `air_id` isn't a precompile's;
                ///   caller handles built-in air-ids itself.
                /// * `Err(_)` — `air_id` matched but the downcast
                ///   failed (bundle-construction invariant violation).
                pub fn try_push_collector(
                    &mut self,
                    air_id: ::std::primitive::usize,
                    secn_instance: &dyn ::zisk_common::Instance<F>,
                    chunk_id: ::zisk_common::ChunkId,
                    global_idx: ::std::primitive::usize,
                ) -> $crate::error::ExecutorResult<::std::primitive::bool> {
                    $(
                        if air_id == $air[0] {
                            let inst = secn_instance
                                .as_any()
                                .downcast_ref::<[<$variant Instance>]<F>>()
                                .ok_or(
                                    $crate::error::ExecutorError::InstanceTypeMismatch {
                                        global_id: global_idx,
                                        air_id,
                                        expected: concat!(stringify!($variant), "Instance"),
                                    }
                                )?;
                            self.[<$variant:snake _collector>].push((
                                global_idx,
                                inst.[<build_ $variant:snake _collector>](
                                    chunk_id,
                                ),
                            ));
                            return ::std::result::Result::Ok(true);
                        }
                    )*
                    ::std::result::Result::Ok(false)
                }

                /// Collect-phase bus dispatch. Routes an operation by its
                /// `OP_TYPE_ID` to the matching precompile: fans out to every
                /// collector for that precompile, then drives the input
                /// generator with a `MemCollectorProcessor` over the parent
                /// bus's memory + memory-align collectors.
                ///
                /// Returns `true` always (route_data on the collect side
                /// discards the result); `false` is reserved for non-matching
                /// ops so the caller can distinguish handled vs. unhandled.
                ///
                /// The `&mut Vec<_>` parameters are kept as `Vec` (not slices)
                /// because `MemCollectorProcessor::new` requires `&mut Vec`
                /// to push during memory-op derivation.
                #[inline(always)]
                #[allow(clippy::ptr_arg)]
                pub fn dispatch_op(
                    &mut self,
                    op_type: ::std::primitive::u32,
                    bus_id: &::zisk_common::BusId,
                    data: &[::zisk_common::PayloadType],
                    mem_collector: &mut ::std::vec::Vec<(
                        ::std::primitive::usize,
                        ::sm_mem::MemModuleCollector,
                    )>,
                    mem_align_collector: &mut ::std::vec::Vec<(
                        ::std::primitive::usize,
                        ::sm_mem::MemAlignCollector,
                    )>,
                ) -> ::std::primitive::bool {
                    $(
                        const [<__ $variant:upper _OP>]: ::std::primitive::u32 = $op;
                    )*
                    match op_type {
                        $(
                            [<__ $variant:upper _OP>] => {
                                for (_, c) in &mut self.[<$variant:snake _collector>] {
                                    c.process_data(bus_id, data);
                                }
                                self.[<$variant:snake _inputs_generator>].process_data(
                                    bus_id,
                                    data,
                                    &mut ::precompiles_common::MemCollectorProcessor::new(
                                        mem_collector,
                                        mem_align_collector,
                                    ),
                                );
                                true
                            }
                        )*
                        _ => false,
                    }
                }

                /// Consumes the slots, producing all per-precompile
                /// collector entries that `StaticDataBusCollect::into_devices`
                /// splices into its full device list. Order matches
                /// declaration order in `register_precompiles!`.
                pub fn into_device_entries(
                    self,
                ) -> ::std::vec::Vec<(
                    ::std::primitive::usize,
                    ::std::boxed::Box<dyn ::zisk_common::BusDevice<::zisk_common::PayloadType>>,
                )> {
                    let mut result = ::std::vec::Vec::new();
                    $(
                        for (id, c) in self.[<$variant:snake _collector>] {
                            result.push((
                                id,
                                ::std::boxed::Box::new(c) as ::std::boxed::Box<
                                    dyn ::zisk_common::BusDevice<::zisk_common::PayloadType>,
                                >,
                            ));
                        }
                    )*
                    result
                }
            }
        }
    };
}
