//! `register_precompiles!` macro — single source of truth for every
//! precompile in the executor.
//!
//! ```ignore
//! register_precompiles! {
//!     Keccakf [air: KECCAKF_AIR_IDS] => KeccakfManager<F>,
//!     // ... add yours here
//! }
//! ```
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
//! emits resolve correctly.
//!
//! Built-in SMs stay hand-written in [`sm_builtins.rs`](crate::sm_builtins)
//! (their dispatch is irregular).

/// Declarative registry for precompiles.
///
/// Generates, from one declaration per precompile:
///
/// 1. `enum Precompiles<F>` + `build_planner` / `configure_instances` /
///    `build_instance` dispatch.
/// 2. `struct PrecompileCounters<F>` (one field per variant, type
///    `(usize, ${Name}CounterInputGen<F>)`) + `from_bundle` +
///    `into_device_entries`.
/// 3. `struct PrecompileCollectors<F>` (per-variant `Vec<(usize,
///    ${Name}Collector)>` + `${Name}CounterInputGen<F>` input gen) +
///    `start_chunk` + `try_push_collector` + `into_device_entries`.
///
/// Identity inside the bundle is the variant's *position* in the
/// bundle's `Vec`, assigned at construction time.
#[macro_export]
macro_rules! register_precompiles {
    (
        $(
            $variant:ident [air: $air:expr] => $mgr:ty
        ),* $(,)?
    ) => {
        pub enum Precompiles<F: ::fields::PrimeField64> {
            $( $variant(::std::sync::Arc<$mgr>), )*
        }

        impl<F: ::fields::PrimeField64> Precompiles<F> {
            pub fn build_planner(&self) -> ::std::boxed::Box<dyn ::zisk_common::Planner> {
                match self {
                    $( Self::$variant(sm) => (**sm).build_planner(), )*
                }
            }

            pub fn configure_instances(
                &self,
                pctx: &::proofman_common::ProofCtx<F>,
                plans: &[::zisk_common::Plan],
            ) {
                match self {
                    $( Self::$variant(sm) => (**sm).configure_instances(pctx, plans), )*
                }
            }

            pub fn build_instance(
                &self,
                ictx: ::zisk_common::InstanceCtx,
            ) -> ::std::boxed::Box<dyn ::zisk_common::Instance<F>> {
                match self {
                    $( Self::$variant(sm) => (**sm).build_instance(ictx), )*
                }
            }
        }

        ::paste::paste! {
            // ────────────────────────────────────────────────────────
            // PrecompileCounters<F> — counter-phase bus slots.
            // ────────────────────────────────────────────────────────

            /// Counter-phase slots for every precompile registered via
            /// `register_precompiles!`. Each field stores
            /// `(bundle_position, counter_input_gen)`.
            pub struct PrecompileCounters<F: ::fields::PrimeField64> {
                $(
                    pub [<$variant:snake>]: (
                        ::std::primitive::usize,
                        [<$variant CounterInputGen>]<F>,
                    ),
                )*
            }

            impl<F: ::fields::PrimeField64> PrecompileCounters<F> {
                /// Iterates the bundle once, building each precompile's
                /// counter via its `build_*_counter(is_asm_emulator)`
                /// method.
                pub fn from_bundle(
                    bundle: &$crate::StaticSMBundle<F>,
                    is_asm_emulator: ::std::primitive::bool,
                ) -> ::anyhow::Result<Self> {
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
                    pub [<$variant:snake _collector>]: ::std::vec::Vec<(
                        ::std::primitive::usize,
                        [<$variant Collector>],
                    )>,
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
