# Adding a New State Machine

This guide outlines the steps to integrate a new state machine into the Zisk executor framework.

---


## 1. Update `static_data_bus.rs`

- Add counters for the new state machine in the `StaticDataBus` struct.
- Update the `new` function to initialize these counters.
- Modify `route_data`:
    - Handle the corresponding `BusId` and operation type(s) for the new state machine.
    - Route payloads to the new counters.
- Update `on_close` and `into_devices` to include the new counters

## 2. Update `static_data_bus_collect.rs`

- Add collectors for each new air ID in the `StaticDataBusCollect` struct.
- Update `route_data` to process bus information for the new state machine:
    - Consider the correct `BusId` and operation type(s).
    - Route payloads to the new collectors.
- Update `on_close` and `into_devices` to include the new collectors

## 3. Update `sm_static_bundle.rs`

### 3.1 Add the new state machine manager to the `StateMachines` enum

```rust
pub enum StateMachines<F: PrimeField64> {
    // existing variants...
    MyNewSM(Arc<MyNewSM<F>>),
}
```

### 3.2 Update associated methods for the new state machine

- `type_id` → assign a unique ID for your new state machine.
- `build_planner` → return a planner for your new SM.
- `configure_instances` → configure instances for your new SM.
- `build_instance` → handle instance creation for your new SM.

### 3.3 Update `build_data_bus_counters`

Add logic to extract counters from your new state machine:

```rust
let my_new_counter = None;
for (_, sm) in self.sm.values() {
    match sm {
        // existing SM...
        StateMachines::MyNewSM(sm) => {
            my_new_counter = Some((sm.type_id(), sm.build_mynew_counter()));
        }
        _ => {}
    }
}
```

Provide this new counter to StaticDataBus initialization


### 3.4 Update `build_data_bus_collectors`

- Add a new vector of collectors for the new air ID(s).
- Include a memory input collector if required.
- Adjust the `StaticDataBusCollect` initialization to include your new collector(s).

```rust
air_id if air_id == MYNEW_AIR_IDS[0] => {
    let mynew_instance = secn_instance
        .as_any()
        .downcast_ref::<MyNewInstance<F>>()
        .unwrap();
    let mynew_collector = mynew_instance
        .build_mynew_collector(ChunkId(chunk_id));
    mynew_collectors.push((*global_idx, mynew_collector));
}
```

Make sure to push the tuple `(global_idx, mynew_collector)` to your collector vector, and later include this vector in the `StaticDataBusCollect` initialization.

## 4. Update `zisk_lib.rs`

Instantiate the new state machine when initializing `StaticSMBundle`:

```rust
let sm_bundle = StaticSMBundle::new(
    process_only_operation_bus,
    vec![
        // existing state machines...
        (new_air_ids, StateMachines::MyNewSM(Arc::new(MyNewSM::new()))),
    ],
);
```