[]: # Title: TODO
[]: # Creation Date: 2019-01-01
[]: # Last Modified: 2019-01-01T15:00:00+01:00

[] Change channel struct !!! This is a naive approach, we need to implement a shared memory one
[x] Change proof_ctx airs[idx] structure to instances[subproof_id][air_id]
[] Explore interface RUST/C++
[] Add macro executor!() to add _witness_computation function to control the entry and the exit of a witness_computation call
[] Simplify code for the executors
[] improve API for create messages


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
    - Afegir tasques
    - Consultar tasques
    - Eliminar tasques
    - Consultar tasques completes
    - Consultar tasques pendents
    - Consultar tasques en execució
    - Consultar tasques fallades
    - Consultar tasques cancelades
    - Consultar tasques en espera

Per cada executor afegir tasques que s'han de completar:
wait_resolved(subproof_id, air_id, [col_id, col_id2, ...])
