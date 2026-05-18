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
//! Per-precompile types derived via `paste!`:
//! * `${Name}CounterInputGen<F>` — counter / input-generator bus device.
//! * `${Name}Instance<F>` — witness instance (downcast target in
//!   `try_push_collector`).
//! * `${Name}Collector` — per-chunk collector vec element.
//! * Methods `build_${name:snake}_counter`,
//!   `build_${name:snake}_input_generator`, and
//!   `build_${name:snake}_collector` on the manager / instance.
//!
//! Caller must `use` each precompile crate's `Manager`, `CounterInputGen`,
//! `Instance`, and `Collector` types so the bare identifiers the macro
//! emits resolve correctly. The `op:` constants must also be in scope at
//! the invocation site — typically `KECCAK_OP_TYPE_ID` etc. from `zisk_core`.
//!
//! Built-in SMs stay hand-written in [`super::builtins`] (their dispatch
//! is irregular).

/// Declarative registry for precompiles.
///
/// Generates, from one declaration per precompile:
///
/// 1. `enum Precompiles<F>` + `build_planner` / `configure_instances` /
///    `build_instance` dispatch.
/// 2. `struct PrecompileCounters<F>` (one field per variant, type
///    `(usize, ${Name}CounterInputGen<F>)`) + `from_bundle` +
///    `dispatch_op` (counter-phase bus arm) + `into_device_entries`.
/// 3. `struct PrecompileCollectors<F>` (per-variant `Vec<(usize,
///    ${Name}Collector)>` + `${Name}CounterInputGen<F>` input gen) +
///    `start_chunk` + `try_push_collector` + `dispatch_op` (collect-phase
///    bus arm) + `into_device_entries`.
///
/// Identity inside the bundle is the variant's *position* in the
/// bundle's `Vec`, assigned at construction time.
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
            /// Dispatches to the active variant's manager and returns its
            /// `Planner`. See `ComponentBuilder::build_planner`.
            pub fn build_planner(&self) -> ::std::boxed::Box<dyn ::zisk_common::Planner> {
                match self {
                    $( Self::$variant(sm) => (**sm).build_planner(), )*
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
                /// Iterates the bundle once, building each precompile's
                /// counter via its `build_*_counter(is_asm)` method.
                /// `is_asm` is sourced from `bundle.is_asm()`, the same
                /// value the bundle was constructed with.
                pub fn from_bundle(
                    bundle: &$crate::StaticSMBundle<F>,
                ) -> ::anyhow::Result<Self> {
                    let is_asm_emulator = bundle.is_asm();
                    $( let mut [<$variant:snake>] = ::std::option::Option::None; )*

                    for (pos, (_, sm)) in bundle.entries().iter().enumerate() {
                        if let $crate::StateMachines::Precompile(p) = sm {
                            match p {
                                $(
                                    Precompiles::$variant(s) => {
                                        [<$variant:snake>] = ::std::option::Option::Some(
                                            (pos, s.[<build_ $variant:snake _counter>](is_asm_emulator))
                                        );
                                    }
                                )*
                            }
                        }
                    }

                    ::std::result::Result::Ok(Self {
                        $(
                            [<$variant:snake>]: [<$variant:snake>].ok_or_else(|| {
                                ::anyhow::anyhow!(concat!(
                                    "Counter not found: ",
                                    stringify!($variant),
                                ))
                            })?,
                        )*
                    })
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
                #[allow(clippy::type_complexity)]
                pub fn into_device_entries(
                    self,
                ) -> ::std::vec::Vec<(
                    ::std::option::Option<::std::primitive::usize>,
                    ::std::option::Option<
                        ::std::boxed::Box<dyn ::zisk_common::BusDeviceMetrics>,
                    >,
                )> {
                    ::std::vec![
                        $(
                            (
                                ::std::option::Option::Some(self.[<$variant:snake>].0),
                                ::std::option::Option::Some(
                                    ::std::boxed::Box::new(self.[<$variant:snake>].1),
                                ),
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
                /// Iterates the bundle once, building each precompile's
                /// input generator. Collector vecs start empty —
                /// populated by `try_push_collector` once per
                /// global_idx in the chunk.
                pub fn start_chunk(
                    bundle: &$crate::StaticSMBundle<F>,
                ) -> ::anyhow::Result<Self> {
                    $( let mut [<$variant:snake _inputs_generator>] = ::std::option::Option::None; )*

                    for (_, sm) in bundle.entries().iter() {
                        if let $crate::StateMachines::Precompile(p) = sm {
                            match p {
                                $(
                                    Precompiles::$variant(s) => {
                                        [<$variant:snake _inputs_generator>] =
                                            ::std::option::Option::Some(
                                                s.[<build_ $variant:snake _input_generator>](),
                                            );
                                    }
                                )*
                            }
                        }
                    }

                    ::std::result::Result::Ok(Self {
                        $(
                            [<$variant:snake _collector>]: ::std::vec::Vec::new(),
                            [<$variant:snake _inputs_generator>]:
                                [<$variant:snake _inputs_generator>].ok_or_else(|| {
                                    ::anyhow::anyhow!(concat!(
                                        "Counter not found: ",
                                        stringify!($variant),
                                        " input generator",
                                    ))
                                })?,
                        )*
                    })
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
                    chunk_id: ::std::primitive::usize,
                    global_idx: ::std::primitive::usize,
                ) -> ::anyhow::Result<::std::primitive::bool> {
                    $(
                        if air_id == $air[0] {
                            let inst = secn_instance
                                .as_any()
                                .downcast_ref::<[<$variant Instance>]<F>>()
                                .ok_or_else(|| {
                                    ::anyhow::anyhow!(concat!(
                                        "Downcast failed: expected ",
                                        stringify!($variant),
                                        "Instance",
                                    ))
                                })?;
                            self.[<$variant:snake _collector>].push((
                                global_idx,
                                inst.[<build_ $variant:snake _collector>](
                                    ::zisk_common::ChunkId(chunk_id),
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
                #[allow(clippy::type_complexity)]
                pub fn into_device_entries(
                    self,
                ) -> ::std::vec::Vec<(
                    ::std::option::Option<::std::primitive::usize>,
                    ::std::option::Option<
                        ::std::boxed::Box<
                            dyn ::zisk_common::BusDevice<::zisk_common::PayloadType>,
                        >,
                    >,
                )> {
                    let mut result = ::std::vec::Vec::new();
                    $(
                        for (id, c) in self.[<$variant:snake _collector>] {
                            result.push((
                                ::std::option::Option::Some(id),
                                ::std::option::Option::Some(
                                    ::std::boxed::Box::new(c) as ::std::boxed::Box<
                                        dyn ::zisk_common::BusDevice<::zisk_common::PayloadType>,
                                    >,
                                ),
                            ));
                        }
                    )*
                    result
                }
            }
        }
    };
}
