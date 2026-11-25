//! Canonical Nitro Stylus execution behind the transpiled Go program's import boundary.
use std::{cell::UnsafeCell, path::Path, sync::Arc};

use arbutil::{Bytes32, format::DebugBytes};
use eyre::{Result, bail, ensure, eyre};
use fnv::{FnvHashMap, FnvHashSet};
use prover::{
    Machine, binary,
    machine::{GlobalState, Module},
    value::Value,
};

#[cfg(target_os = "zkvm")]
mod allocator;

const SCRATCH: u32 = 8;
const MAX_BRIDGE_BYTES: usize = 512 * 1024 * 1024;

struct Adapter {
    machine: Machine,
    bridge: u32,
    prepared: FnvHashSet<Bytes32>,
}

impl Adapter {
    fn new() -> Result<Self> {
        let sources: &[(&str, &[u8])] = &[
            // User modules bind vm_hooks to module index one.
            ("forward", include_bytes!(concat!(env!("NITRO_STYLUS_WASM_DIR"), "/forward.wasm"))),
            (
                "soft-float",
                include_bytes!(concat!(env!("NITRO_STYLUS_WASM_DIR"), "/soft-float.wasm")),
            ),
            (
                "wasi_stub",
                include_bytes!(concat!(env!("NITRO_STYLUS_WASM_DIR"), "/wasi_stub.wasm")),
            ),
            (
                "user_host",
                include_bytes!(concat!(env!("NITRO_STYLUS_WASM_DIR"), "/user_host.wasm")),
            ),
            (
                "program_exec",
                include_bytes!(concat!(env!("NITRO_STYLUS_WASM_DIR"), "/program_exec.wasm")),
            ),
        ];
        let libraries = sources
            .iter()
            .map(|(name, wasm)| binary::parse(wasm, Path::new(name)))
            .collect::<Result<Vec<_>>>()?;
        let bridge = binary::parse(
            include_bytes!(concat!(env!("OUT_DIR"), "/bridge.wasm")),
            Path::new("bridge"),
        )?;
        let machine = Machine::from_binaries(
            &libraries,
            bridge,
            false,
            false,
            false,
            false,
            GlobalState::default(),
            FnvHashMap::default(),
            Arc::new(|_, _, _| None),
            None,
        )?;
        let bridge = machine.find_module("bridge")?;
        Ok(Self { machine, bridge, prepared: FnvHashSet::default() })
    }

    fn call(&mut self, function: &str, args: Vec<Value>) -> Result<u64> {
        self.machine.write_memory(self.bridge, 0, &[0; 4])?;
        let results = self.machine.call_function("bridge", function, args)?;
        // Nitro's direct-call sentinel is an error status; this proves the wrapper reached its epilogue.
        ensure!(
            self.machine.read_memory(self.bridge, 0, 4)? == 1u32.to_le_bytes(),
            "Stylus bridge did not complete"
        );
        match results.as_slice() {
            [] => Ok(0),
            [Value::I32(value)] => Ok(u64::from(*value)),
            [Value::I64(value)] => Ok(*value),
            _ => bail!("unexpected Stylus bridge result"),
        }
    }

    fn reserve(&mut self, bytes: usize) -> Result<()> {
        ensure!(bytes <= MAX_BRIDGE_BYTES, "Stylus bridge memory limit exceeded");
        self.call("reserve", vec![Value::I32(bytes as u32)])?;
        Ok(())
    }

    fn stage(&mut self, data: &[u8], cursor: &mut u32) -> Result<u32> {
        let start = *cursor;
        let end = usize::try_from(start)?
            .checked_add(data.len())
            .ok_or_else(|| eyre!("Stylus buffer overflow"))?;
        self.reserve(end)?;
        self.machine.write_memory(self.bridge, start, data)?;
        *cursor =
            u32::try_from(end)?.checked_add(7).ok_or_else(|| eyre!("Stylus buffer overflow"))? & !7;
        Ok(start)
    }

    fn register(&mut self, module: Module) {
        let hash = module.hash();
        if self.prepared.insert(hash) {
            // Only internally regenerated modules cross Nitro's trusted deserialization boundary.
            self.machine.add_stylus_module(hash, module.into_bytes());
        }
    }

    fn dispatch(&mut self, op: u64, args: &[u64; 16], go: &mut [u8]) -> Result<u64> {
        let mut cursor = SCRATCH;
        match op {
            30 => {
                let wasm = read(go, args[0], args[1])?;
                let page_limit = u16::from_le_bytes(read(go, args[2], 2)?.try_into()?);
                let codehash = Bytes32(read(go, args[9], 32)?.try_into()?);
                let mut gas = u64::from_le_bytes(read(go, args[11], 8)?.try_into()?);
                let activation = Module::activate(
                    wasm,
                    &codehash,
                    args[6] as u16,
                    args[7],
                    page_limit,
                    args[8] != 0,
                    &mut gas,
                );
                match activation {
                    Ok((module, data)) => {
                        write(go, args[11], &gas.to_le_bytes())?;
                        write(go, args[2], &data.footprint.to_le_bytes())?;
                        write(go, args[3], &data.asm_estimate.to_le_bytes())?;
                        write(go, args[4], &data.init_cost.to_le_bytes())?;
                        write(go, args[5], &data.cached_init_cost.to_le_bytes())?;
                        write(go, args[10], module.hash().as_slice())?;
                        self.register(module);
                        Ok(0)
                    }
                    Err(error) => {
                        let mut message = error.wrap_err("failed to activate").debug_bytes();
                        message.truncate(usize::try_from(args[13])?);
                        write(go, args[12], &message)?;
                        write(go, args[11], &0u64.to_le_bytes())?;
                        write(go, args[2], &0u16.to_le_bytes())?;
                        write(go, args[3], &0u32.to_le_bytes())?;
                        write(go, args[4], &0u16.to_le_bytes())?;
                        write(go, args[5], &0u16.to_le_bytes())?;
                        write(go, args[10], &[0; 32])?;
                        Ok(message.len() as u64)
                    }
                }
            }
            31 => self.call("config", args[..4].iter().map(|x| Value::I32(*x as u32)).collect()),
            32 => {
                let mut values: Vec<Value> =
                    args[..15].iter().map(|x| Value::I32(*x as u32)).collect();
                for index in [0, 2, 4, 5, 6] {
                    values[index] = Value::I64(args[index]);
                }
                for (index, length) in
                    [(1, 32), (3, 20), (7, 20), (8, 32), (9, 20), (10, 32), (11, 32), (12, 20)]
                {
                    values[index] =
                        Value::I32(self.stage(read(go, args[index], length)?, &mut cursor)?);
                }
                self.call("evm", values)
            }
            33 => {
                let hash = Bytes32(read(go, args[0], 32)?.try_into()?);
                ensure!(
                    self.prepared.contains(&hash),
                    "Stylus module was not authenticated by preparation"
                );
                let hash_ptr = self.stage(hash.as_slice(), &mut cursor)?;
                let data_ptr = self.stage(read(go, args[1], args[2])?, &mut cursor)?;
                self.call(
                    "new",
                    vec![
                        Value::I32(hash_ptr),
                        Value::I32(data_ptr),
                        Value::I32(args[2] as u32),
                        Value::I64(args[3]),
                        Value::I64(args[4]),
                        Value::I64(args[5]),
                    ],
                )
            }
            34 => self.call("start", vec![Value::I32(args[0] as u32)]),
            35 => {
                let result =
                    self.call("request", vec![Value::I32(args[0] as u32), Value::I32(SCRATCH)])?;
                if args[1] != 0 {
                    write(go, args[1], self.machine.read_memory(self.bridge, SCRATCH, 4)?)?;
                }
                Ok(result)
            }
            36 => {
                self.call("request", vec![Value::I32(args[0] as u32), Value::I32(SCRATCH)])?;
                let len = u32::from_le_bytes(
                    self.machine.read_memory(self.bridge, SCRATCH, 4)?.try_into()?,
                );
                read(go, args[1], u64::from(len))?;
                self.reserve(SCRATCH as usize + len as usize)?;
                self.call("request_data", vec![Value::I32(args[0] as u32), Value::I32(SCRATCH)])?;
                write(go, args[1], self.machine.read_memory(self.bridge, SCRATCH, len)?)?;
                Ok(0)
            }
            37 => {
                let result = self.stage(read(go, args[2], args[3])?, &mut cursor)?;
                let raw = self.stage(read(go, args[4], args[5])?, &mut cursor)?;
                self.call(
                    "response",
                    vec![
                        Value::I32(args[0] as u32),
                        Value::I64(args[1]),
                        Value::I32(result),
                        Value::I32(args[3] as u32),
                        Value::I32(raw),
                        Value::I32(args[5] as u32),
                    ],
                )
            }
            38 => self.call("send", vec![Value::I32(args[0] as u32)]),
            39 => self.call("pop_program", vec![]),
            40 => {
                let hash = Bytes32(read(go, args[0], 32)?.try_into()?);
                Ok(u64::from(!self.prepared.contains(&hash)))
            }
            41 => {
                let header = read(go, args[0], 24)?;
                let pointer = u64::from_le_bytes(header[..8].try_into()?);
                let length = u64::from_le_bytes(header[8..16].try_into()?);
                let capacity = u64::from_le_bytes(header[16..].try_into()?);
                ensure!(
                    length == args[1] && length <= capacity && capacity <= u32::MAX as u64,
                    "invalid Stylus Go slice header"
                );
                ensure!(length <= args[4], "Stylus wasm exceeds configured size limit");
                let wasm = read(go, pointer, length)?;
                let expected = Bytes32(read(go, args[2], 32)?.try_into()?);
                let codehash = Bytes32(read(go, args[3], 32)?.try_into()?);
                let page_limit = u16::try_from(args[5])?;
                let version = u16::try_from(args[7])?;
                let mut gas = u64::MAX;
                let (module, _) = Module::activate(
                    wasm,
                    &codehash,
                    version,
                    0,
                    page_limit,
                    args[6] != 0,
                    &mut gas,
                )?;
                ensure!(
                    module.hash() == expected,
                    "regenerated Stylus module hash does not match ArbOS state"
                );
                self.register(module);
                Ok(0)
            }
            _ => bail!("unknown Stylus operation {op}"),
        }
    }
}

fn bounds(memory_len: usize, pointer: u64, length: u64) -> Result<std::ops::Range<usize>> {
    ensure!(
        pointer <= u32::MAX as u64 && length <= u32::MAX as u64,
        "invalid wasm32 Stylus buffer"
    );
    if length == 0 {
        return Ok(0..0);
    }
    let start = usize::try_from(pointer)?;
    let end = start
        .checked_add(usize::try_from(length)?)
        .ok_or_else(|| eyre!("Stylus buffer overflow"))?;
    ensure!(end <= memory_len, "Stylus buffer out of bounds");
    Ok(start..end)
}

fn read(memory: &[u8], pointer: u64, length: u64) -> Result<&[u8]> {
    Ok(&memory[bounds(memory.len(), pointer, length)?])
}

fn write(memory: &mut [u8], pointer: u64, data: &[u8]) -> Result<()> {
    let range = bounds(memory.len(), pointer, data.len() as u64)?;
    memory[range].copy_from_slice(data);
    Ok(())
}

struct Persistent(UnsafeCell<Option<Adapter>>);
// The replay runtime calls this boundary on one thread, after each previous call returns.
unsafe impl Sync for Persistent {}
static ADAPTER: Persistent = Persistent(UnsafeCell::new(None));

/// Dispatches one programs.* call from the proved Go Wasm execution.
///
/// # Safety
/// `args` must contain sixteen slots; `wasm_base` must identify `wasm_byte_size` writable bytes.
/// Calls must not overlap or run concurrently.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nitro_stylus_dispatch(
    op: u64,
    args: *const u64,
    wasm_base: *mut u8,
    wasm_byte_size: u64,
) -> u64 {
    #[cfg(target_os = "zkvm")]
    allocator::initialize();
    assert!(wasm_byte_size <= u32::MAX as u64 && !args.is_null() && !wasm_base.is_null());
    let args: [u64; 16] = unsafe { std::ptr::read_unaligned(args.cast()) };
    let go = unsafe { std::slice::from_raw_parts_mut(wasm_base, wasm_byte_size as usize) };
    let adapter = unsafe { &mut *ADAPTER.0.get() };
    if adapter.is_none() {
        *adapter = Some(Adapter::new().expect("initialize canonical Stylus runtime"));
    }
    adapter.as_mut().unwrap().dispatch(op, &args, go).expect("canonical Stylus import failed")
}

#[cfg(test)]
mod tests {
    use super::*;

    const HASH: u64 = 0x140;
    const GAS: u64 = 0x160;
    const DATA: u64 = 0x10000;
    const RESULT: u64 = 0x20000;
    const INITIAL_GAS: u64 = 1_000_000;

    fn dispatch(adapter: &mut Adapter, memory: &mut [u8], op: u64, values: &[u64]) -> Result<u64> {
        let mut args = [0; 16];
        args[..values.len()].copy_from_slice(values);
        adapter.dispatch(op, &args, memory)
    }

    fn echo() -> Vec<u8> {
        wat::parse_str(
            r#"(module
          (import "vm_hooks" "read_args" (func $read (param i32)))
          (import "vm_hooks" "write_result" (func $write (param i32 i32)))
          (memory (export "memory") 1 1)
          (func (export "user_entrypoint") (param i32) (result i32)
            i32.const 0 call $read
            i32.const 0 local.get 0 call $write
            i32.const 0))"#,
        )
        .unwrap()
    }

    fn storage() -> Vec<u8> {
        wat::parse_str(
            r#"(module
          (import "vm_hooks" "storage_load_bytes32" (func $load (param i32 i32)))
          (import "vm_hooks" "write_result" (func $write (param i32 i32)))
          (memory (export "memory") 1 1)
          (func (export "user_entrypoint") (param i32) (result i32)
            i32.const 0 i32.const 64 call $load
            i32.const 64 i32.const 32 call $write
            i32.const 0))"#,
        )
        .unwrap()
    }

    fn activate(adapter: &mut Adapter, memory: &mut [u8], wasm: &[u8], gas: u64) -> Result<u64> {
        write(memory, 0x1000, wasm)?;
        write(memory, 0x100, &128u16.to_le_bytes())?;
        write(memory, 0x120, &[7; 32])?;
        write(memory, GAS, &gas.to_le_bytes())?;
        dispatch(
            adapter,
            memory,
            30,
            &[
                0x1000,
                wasm.len() as u64,
                0x100,
                0x104,
                0x108,
                0x10a,
                2,
                60,
                0,
                0x120,
                HASH,
                GAS,
                0x180,
                1024,
            ],
        )
    }

    fn launch(
        adapter: &mut Adapter,
        memory: &mut [u8],
        hash: &[u8],
        calldata: &[u8],
    ) -> Result<u64> {
        launch_with_limits(adapter, memory, hash, calldata, INITIAL_GAS, 65536)
    }

    fn launch_with_limits(
        adapter: &mut Adapter,
        memory: &mut [u8],
        hash: &[u8],
        calldata: &[u8],
        gas: u64,
        depth: u64,
    ) -> Result<u64> {
        write(memory, HASH, hash)?;
        write(memory, DATA, calldata)?;
        let config = dispatch(adapter, memory, 31, &[2, depth, 10000, 0])?;
        let context = dispatch(
            adapter,
            memory,
            32,
            &[60, 0, 42161, 0, 100_000_000, 1, 1, 0, HASH, 0, 0, 0, 0, 0, 0],
        )?;
        let module = dispatch(
            adapter,
            memory,
            33,
            &[HASH, DATA, calldata.len() as u64, config, context, gas],
        )?;
        dispatch(adapter, memory, 34, &[module])
    }

    fn request(adapter: &mut Adapter, memory: &mut [u8], id: u64) -> Result<(u64, Vec<u8>)> {
        let kind = dispatch(adapter, memory, 35, &[id, 0x100])?;
        let len = u32::from_le_bytes(read(memory, 0x100, 4)?.try_into()?) as u64;
        dispatch(adapter, memory, 36, &[id, RESULT])?;
        Ok((kind, read(memory, RESULT, len)?.to_vec()))
    }

    #[test]
    fn echo_uses_canonical_meter_and_caller_memory() -> Result<()> {
        let mut adapter = Adapter::new()?;
        let mut memory = vec![0; 1024 * 1024];
        assert_eq!(activate(&mut adapter, &mut memory, &echo(), INITIAL_GAS)?, 0);
        let hash = read(&memory, HASH, 32)?.to_vec();
        assert_eq!(dispatch(&mut adapter, &mut memory, 40, &[HASH])?, 0);
        let id = launch(&mut adapter, &mut memory, &hash, b"canonical stylus")?;
        let (kind, output) = request(&mut adapter, &mut memory, id)?;
        assert_eq!(kind, 0);
        let gas_left = u64::from_be_bytes(output[..8].try_into()?);
        assert!(gas_left > 0 && gas_left < INITIAL_GAS);
        assert_eq!(&output[8..], b"canonical stylus");
        dispatch(&mut adapter, &mut memory, 39, &[])?;
        Ok(())
    }

    #[test]
    fn suspended_parent_survives_nested_program() -> Result<()> {
        let mut adapter = Adapter::new()?;
        let mut memory = vec![0; 1024 * 1024];
        assert_eq!(activate(&mut adapter, &mut memory, &storage(), INITIAL_GAS)?, 0);
        let parent_hash = read(&memory, HASH, 32)?.to_vec();
        assert_eq!(activate(&mut adapter, &mut memory, &echo(), INITIAL_GAS)?, 0);
        let child_hash = read(&memory, HASH, 32)?.to_vec();
        let parent_id = launch(&mut adapter, &mut memory, &parent_hash, &[])?;
        let (kind, input) = request(&mut adapter, &mut memory, parent_id)?;
        assert_eq!(kind, 0x10000000);
        assert_eq!(input, vec![0; 32]);
        let child_id = launch(&mut adapter, &mut memory, &child_hash, b"nested")?;
        let (kind, output) = request(&mut adapter, &mut memory, child_id)?;
        assert_eq!(kind, 0);
        assert_eq!(&output[8..], b"nested");
        dispatch(&mut adapter, &mut memory, 39, &[])?;
        write(&mut memory, DATA, &[0xab; 32])?;
        dispatch(&mut adapter, &mut memory, 37, &[parent_id, 100, DATA, 32, 0, 0])?;
        let done = dispatch(&mut adapter, &mut memory, 38, &[parent_id])?;
        let (kind, output) = request(&mut adapter, &mut memory, done)?;
        assert_eq!(kind, 0);
        assert_eq!(&output[8..], &[0xab; 32]);
        dispatch(&mut adapter, &mut memory, 39, &[])?;
        Ok(())
    }

    #[test]
    fn prepare_checks_raw_module_hash_and_go_slice_header() -> Result<()> {
        let mut original = Adapter::new()?;
        let mut memory = vec![0; 1024 * 1024];
        let wasm = echo();
        assert_eq!(activate(&mut original, &mut memory, &wasm, INITIAL_GAS)?, 0);
        let mut adapter = Adapter::new()?;
        assert_eq!(dispatch(&mut adapter, &mut memory, 40, &[HASH])?, 1);
        write(&mut memory, 0x300, &0x1000u64.to_le_bytes())?;
        write(&mut memory, 0x308, &(wasm.len() as u64).to_le_bytes())?;
        write(&mut memory, 0x310, &(wasm.len() as u64).to_le_bytes())?;
        let prepare = [0x300, wasm.len() as u64, HASH, 0x120, 65536, 128, 0, 2];
        memory[HASH as usize] ^= 1;
        assert!(dispatch(&mut adapter, &mut memory, 41, &prepare).is_err());
        assert_eq!(dispatch(&mut adapter, &mut memory, 40, &[HASH])?, 1);
        memory[HASH as usize] ^= 1;
        write(&mut memory, 0x310, &0u64.to_le_bytes())?;
        assert!(dispatch(&mut adapter, &mut memory, 41, &prepare).is_err());
        write(&mut memory, 0x310, &(wasm.len() as u64).to_le_bytes())?;
        dispatch(&mut adapter, &mut memory, 41, &prepare)?;
        assert_eq!(dispatch(&mut adapter, &mut memory, 40, &[HASH])?, 0);
        Ok(())
    }

    #[test]
    fn activation_failure_burns_gas_and_zeros_outputs() -> Result<()> {
        let mut adapter = Adapter::new()?;
        let mut memory = vec![0xff; 1024 * 1024];
        let length = activate(&mut adapter, &mut memory, b"invalid wasm", INITIAL_GAS)?;
        assert!(length > 0 && length <= 1024);
        assert_eq!(read(&memory, GAS, 8)?, &[0; 8]);
        assert_eq!(read(&memory, HASH, 32)?, &[0; 32]);
        assert_eq!(read(&memory, 0x100, 2)?, &[0; 2]);
        assert_eq!(read(&memory, 0x104, 8)?, &[0; 8]);
        assert!(
            String::from_utf8_lossy(read(&memory, 0x180, length)?).contains("failed to activate")
        );
        Ok(())
    }

    #[test]
    fn contract_faults_return_canonical_completion_status() -> Result<()> {
        let mut adapter = Adapter::new()?;
        let mut memory = vec![0; 1024 * 1024];
        for (body, gas, depth, expected) in [
            ("i32.const 1", INITIAL_GAS, 65536, 1),
            ("unreachable", INITIAL_GAS, 65536, 2),
            ("i32.const 0", 0, 65536, 3),
            ("i32.const 0", INITIAL_GAS, 0, 4),
        ] {
            let wasm = wat::parse_str(format!(
                r#"(module
                (memory (export "memory") 1 1)
                (func (export "user_entrypoint") (param i32) (result i32) {body}))"#
            ))?;
            assert_eq!(activate(&mut adapter, &mut memory, &wasm, INITIAL_GAS)?, 0);
            let hash = read(&memory, HASH, 32)?.to_vec();
            let id = launch_with_limits(&mut adapter, &mut memory, &hash, &[], gas, depth)?;
            let (kind, output) = request(&mut adapter, &mut memory, id)?;
            assert_eq!(kind, expected);
            assert_eq!(output.len(), 8);
            if expected >= 3 {
                assert_eq!(output, vec![0; 8]);
            }
            dispatch(&mut adapter, &mut memory, 39, &[])?;
        }
        Ok(())
    }

    #[test]
    fn invalid_request_order_fails_closed() -> Result<()> {
        for duplicate in [false, true] {
            let mut adapter = Adapter::new()?;
            let mut memory = vec![0; 1024 * 1024];
            assert!(dispatch(&mut adapter, &mut memory, 99, &[]).is_err());
            assert_eq!(activate(&mut adapter, &mut memory, &echo(), INITIAL_GAS)?, 0);
            let hash = read(&memory, HASH, 32)?.to_vec();
            let id = launch(&mut adapter, &mut memory, &hash, b"request")?;
            if duplicate {
                request(&mut adapter, &mut memory, id)?;
                assert!(dispatch(&mut adapter, &mut memory, 36, &[id, RESULT]).is_err());
            } else {
                assert!(dispatch(&mut adapter, &mut memory, 35, &[id + 1, 0]).is_err());
            }
        }
        Ok(())
    }

    #[test]
    fn wasm_buffers_reject_overflow_and_allow_empty_inaccessible_pointers() {
        assert!(bounds(1024, 1020, 8).is_err());
        assert!(bounds(1024, u32::MAX as u64 + 1, 0).is_err());
        assert!(bounds(1024, 0, u64::MAX).is_err());
        assert_eq!(bounds(1024, u32::MAX as u64, 0).unwrap(), 0..0);
    }
}
