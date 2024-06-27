#TODO

[X] Change proof_ctx airs[idx] structure to instances[subproof_id][air_id]
[X] Add macro executor!() to add _witness_computation function to control the entry and the exit of a witness_computation call
[X] Simplify code for the executors
[X] improve API for create messages
[X] Load public inputs from file
1.  [ ] Add TraceTable (Adapt code to TraceTable if needed or new TraceTable test)
1.1 [ ] fork and join with Trace test and test example 
1.2 [ ] fork and join with TraceTable test and test example
2.  [X] Add a table of tasks using shared memory
2.1 [X] Add a wait until resolved function: wait(subproof_id, air_id, [col_id, col_id2, ...])
2.2 [ ] Add deferred function test
3.  [ ] binary or arith examples
4.  [ ] Explore interface RUST/C++
5.  [ ] Change channel struct !!! This is a naive approach, we need to implement a shared memory one
6. Transform Vec<Vec<T>> to slice [u8] and access to the specific element with a function ???
7. [X] Organize different proof_ctx as in javascript
8. [ ] Change the way we are "sending" traces from threads. Box is used to be sized and this pushes down the performance to access the elements of a trace when used
9. type F = Goldilocks must be derived from pilout when creating the wc_manager.rs during a zisk new ...... !!!!

commands:
execute example
../target/debug/fibonacci prove -a ./src/fibonacci/fibonacci.airout -o ./src/fibonacci/proof.json -p ./src/fibonacci/settings.json

execute tests
cargo test -p proofman

execute with selected log level
RUST_LOG=debug cargo run --bin fibonacci

Command to generate protobuffer parsing RUST code:
protoc --rust_out=experimental-codegen=enabled,kernel=upb:. pilout.proto



NOVA ESTRATEGIA
A una taula centralitzada de tasques:
add_task(task_id, subproof_id, air_id, col_id, col_id2, ...)

find_task_dest(dest)
find_task_by_id(task_id)

resolve_task(task_id)

get_num_tasks()
get_num_pending_tasks()

