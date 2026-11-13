(module
  (import "programs" "create_stylus_config" (func $config (param i32 i32 i32 i32) (result i64)))
  (import "programs" "create_evm_data_v2" (func $evm (param i64 i32 i64 i32 i64 i64 i64 i32 i32 i32 i32 i32 i32 i32 i32) (result i64)))
  (import "programs" "new_program" (func $new (param i32 i32 i32 i64 i64 i64) (result i32)))
  (import "programs" "start_program" (func $start (param i32) (result i32)))
  (import "programs" "get_request" (func $request (param i32 i32) (result i32)))
  (import "programs" "get_request_data" (func $request_data (param i32 i32)))
  (import "programs" "set_response" (func $response (param i32 i64 i32 i32 i32 i32)))
  (import "programs" "send_response" (func $send (param i32) (result i32)))
  (import "programs" "pop" (func $pop))
  (memory (export "memory") 1 8192)
  (func $done i32.const 0 i32.const 1 i32.store)
  (func (export "reserve") (param $bytes i32)
    (local $pages i32)
    local.get $bytes
    i32.const 65535
    i32.add
    i32.const 16
    i32.shr_u
    local.tee $pages
    memory.size
    i32.gt_u
    if
      local.get $pages
      memory.size
      i32.sub
      memory.grow
      i32.const -1
      i32.eq
      if unreachable end
    end
    call $done)
  (func (export "config") (param i32 i32 i32 i32) (result i64)
    local.get 0 local.get 1 local.get 2 local.get 3 call $config call $done)
  (func (export "evm") (param i64 i32 i64 i32 i64 i64 i64 i32 i32 i32 i32 i32 i32 i32 i32) (result i64)
    local.get 0 local.get 1 local.get 2 local.get 3 local.get 4
    local.get 5 local.get 6 local.get 7 local.get 8 local.get 9
    local.get 10 local.get 11 local.get 12 local.get 13 local.get 14 call $evm call $done)
  (func (export "new") (param i32 i32 i32 i64 i64 i64) (result i32)
    local.get 0 local.get 1 local.get 2 local.get 3 local.get 4 local.get 5 call $new call $done)
  (func (export "start") (param i32) (result i32) local.get 0 call $start call $done)
  (func (export "request") (param i32 i32) (result i32) local.get 0 local.get 1 call $request call $done)
  (func (export "request_data") (param i32 i32) local.get 0 local.get 1 call $request_data call $done)
  (func (export "response") (param i32 i64 i32 i32 i32 i32)
    local.get 0 local.get 1 local.get 2 local.get 3 local.get 4 local.get 5 call $response call $done)
  (func (export "send") (param i32) (result i32) local.get 0 call $send call $done)
  (func (export "pop_program") call $pop call $done))
