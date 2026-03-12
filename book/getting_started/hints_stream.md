# Hints Stream

The hints stream accelerates proof generation by offloading expensive operations outside the zkVM execution, then feeding the results back as verifiable data through a high-performance, parallel pipeline. Hints are preprocessed results that allow operations to be handled externally while remaining fully verifiable inside the VM. The system supports two categories of hints:

1. **Precompile hints**: Cryptographic operations (SHA-256, Keccak-256, elliptic curve operations, pairings, etc.) that are computationally expensive inside a zkVM.
2. **Input hints**: Data that needs to be passed to the zkVM as input during execution.

The system is designed around three core principles:

1. **Pre-computing results outside the VM**: The guest program emits hint requests describing the operation and its inputs.
2. **Streaming results back**: A dedicated pipeline processes these requests in parallel, maintaining order, and feeds results to the prover via shared memory.
3. **Verifying inside the VM**: The zkVM circuits verify that the precomputed results are correct, avoiding the cost of computing them inside the zkVM.

```mermaid
flowchart LR
    A["Guest program<br/><small>Emits hints request</small>"] --> B["ZiskStream"]
    B --> C["HintsProcessor<br/><small>Parallel engine</small>"]
    C --> D["StreamSink<br/><small>ASM emulator/file output</small>"]
```

---

## Table of Contents

1. [Hint Format and Protocol](#1-hint-format-and-protocol)
2. [Hints in CLI Execution](#2-hints-in-cli-execution)
3. [Hints in Distributed Execution](#3-hints-in-distributed-execution)
4. [Custom Hint Handlers](#4-custom-hint-handlers)
5. [Generating Hints in Guest Programs](#5-generating-hints-in-guest-programs)

---

## 1. Hint Format and Protocol

### 1.1. Hint Request Format

Hints are transmitted as a stream of `u64` values. Each hint request consists of a **header** (1 `u64`) followed by **data** (N `u64` values).

```
┌─────────────────────────────────────────────────────────────┐
│                         Header (u64)                        │
├·····························································┤
│      Hint Code (32 bits)           Length (32 bits).        │
├─────────────────────────────────────────────────────────────┤
│                        Data[0] (u64)                        │
├─────────────────────────────────────────────────────────────┤
│                        Data[1] (u64)                        │
├─────────────────────────────────────────────────────────────┤
│                             ...                             │
├─────────────────────────────────────────────────────────────┤
│                       Data[N-1] (u64)                       │
└─────────────────────────────────────────────────────────────┘
where N = ceil(Length / 8)
```
- **Hint Code** (upper 32 bits): Control code or Data Hint Type
- **Length** (lower 32 bits): Payload data size in **bytes**. The last `u64` may contain padding bytes.

### 1.2. Control Hint Types:

The following control codes are defined:
- `0x00` (START): Start a new hint stream. Resets processor state and sequence counters. Must be the first hint in the first batch.
- `0x01` (END): End the current hint stream. The processor will wait for all pending hints to be processed before returning. Must be the last hint in its batch; only a `CTRL_START` may follow in a subsequent batch.
- `0x02` (CANCEL): **[Reserved for future use]** Cancel current stream and stop processing further hints.
- `0x03` (ERROR): **[Reserved for future use]** Indicate an error has occurred; stop processing further hints.

Control codes are for control only and do not have any associated data (Length should be zero).

### 1.3. Data Hint Types

For data hints, the hint code (32 bits) is structured as follows:
- **Bit 31 (MSB)**: Pass-through flag. When set, the data bypasses computation and is forwarded directly to the sink.
- **Bits 0-30**: The hint type identifier (control, built-in, or custom code).
  (e.g., `HINT_SHA256`, `HINT_BN254_G1_ADD`, `HINT_SECP256K1_RECOVER`, etc.)

**Example**: A SHA-256 hint (`0x0100`) with a 32-byte input:
```
Header: 0x00000100_00000020
Data[0]: first_8_input_bytes_as_u64
Data[1]: next_8_input_bytes_as_u64
Data[2]: next_8_input_bytes_as_u64
Data[3]: last_8_input_bytes_as_u64
```

The same hint with the **pass-through flag** set (bit 31), forwarding pre-computed data directly to the sink without invoking the SHA-256 handler:
```
Header: 0x80000100_00000020
```

#### 1.3.1 Stream Batching

The hints protocol supports chunking for individual hints that exceed the transport’s message size limit (currently 128 KB). Each message in the stream contains either a single complete hint or one chunk of a larger hint — hints are never combined in the same message.

When a hint exceeds the size limit, it must be split into multiple sequential chunks, each sent as a separate message. Each chunk includes a header specifying the total length of the complete hint, allowing the receiver to reassemble all chunks before processing. For example, a hint with a 300 KB payload would be split into three messages:
```Message 1: Header (code + total length), Data[0..N] (first 128 KB chunk)
Message 2: Header (code + total length), Data[0..N] (second 128 KB chunk)
Message 3: Header (code + total length), Data[0..M] (final 44 KB chunk)
```
The receiver buffers incoming chunks and reassembles them based on the total length specified in the header before invoking the hint handler. This allows the system to handle arbitrarily large hints while respecting transport limitations.

#### 1.3.2 Pass-Through Hints

When bit 31 of the hint code is set (e.g., `0x8000_0000 | actual_code`), the hint is marked as **pass-through**:

- The data payload is forwarded directly to the sink without invoking any handler.
- No worker thread is spawned; the data is queued immediately in the reorder buffer.
- This is useful for pre-computed results that don't need processing.

### 1.4. Hint Code Types

| Category     | Code Range          | Description                         |
|--------------|---------------------|-------------------------------------|
| **Control**  | `0x0000`-`0x000F`   | Stream lifecycle management         |
| **Built-in** | `0x0100`-`0x0800`   | Cryptographic precompile operations |
| **Input**    | `0xF0000`           | Input data hints                    |
| **Custom**   | User-defined        | Application-specific handlers       |

> **Note:** Custom hint codes can technically use any value not occupied by control or built-in codes. By convention, codes `0xA000`-`0xFFFF` are recommended for custom use to avoid future conflicts as new built-in types are added. The processor does not enforce a range restriction — any unrecognized code is treated as custom.

#### 1.4.1. Control Codes

Control codes manage the stream lifecycle and do not carry computational data:

| Code | Name | Description |
|------|------|-------------|
| `0x0000` | `CTRL_START`   | Resets processor state. Must be the first hint in the first batch. |
| `0x0001` | `CTRL_END`     | Signals end of stream. Blocks until all pending hints complete. Must be the last hint. |
| `0x0002` | `CTRL_CANCEL`  | **[Reserved for future use]** Cancels the current stream. Sets error flag and stops processing. |
| `0x0003` | `CTRL_ERROR`   | **[Reserved for future use]** External error signal. Sets error flag and stops processing. |

#### 1.4.2. Built-in Hint Types

| Code | Name | Description |
|------|------|-------------|
| `0x0100` | `Sha256` | SHA-256 hash computation |
| `0x0200` | `Bn254G1Add` | BN254 G1 point addition |
| `0x0201` | `Bn254G1Mul` | BN254 G1 scalar multiplication |
| `0x0205` | `Bn254PairingCheck` | BN254 pairing check |
| `0x0300` | `Secp256k1EcdsaAddressRecover` | Secp256k1 ECDSA address recovery |
| `0x0301` | `Secp256k1EcdsaVerifyAddressRecover` | Secp256k1 ECDSA verify + address recovery |
| `0x0380` | `Secp256r1EcdsaVerify` | Secp256r1 (P-256) ECDSA verification |
| `0x0400` | `Bls12_381G1Add` | BLS12-381 G1 point addition |
| `0x0401` | `Bls12_381G1Msm` | BLS12-381 G1 multi-scalar multiplication |
| `0x0405` | `Bls12_381G2Add` | BLS12-381 G2 point addition |
| `0x0406` | `Bls12_381G2Msm` | BLS12-381 G2 multi-scalar multiplication |
| `0x040A` | `Bls12_381PairingCheck` | BLS12-381 pairing check |
| `0x0410` | `Bls12_381FpToG1` | BLS12-381 map field element to G1 |
| `0x0411` | `Bls12_381Fp2ToG2` | BLS12-381 map field element to G2 |
| `0x0500` | `ModExp` | Modular exponentiation |
| `0x0600` | `VerifyKzgProof` | KZG polynomial commitment proof verification |
| `0x0700` | `Keccak256` | Keccak-256 hash computation |
| `0x0800` | `Blake2bCompress` | Blake2b compression function |

#### 1.4.3. Input Hint Type

Input hints allow passing data to the zkVM during execution. Unlike precompile hints that are processed by worker threads, input hints are forwarded directly to a separate inputs sink.

| Code | Name | Description |
|------|------|-------------|
| `0xF0000` | `Input` | Input data for the zkVM |

The input hint payload format is:
- **First 8 bytes**: Length of the input data (as `u64` little-endian)
- **Remaining bytes**: The actual input data, padded to 8-byte alignment

Input hints are not processed by the parallel worker pool; instead, they are immediately submitted to the inputs sink for consumption by the zkVM.

#### 1.4.4. Custom Hint Types

Custom hint types allow users to define their own hint handlers for application-specific logic. Users can register custom handlers via the `HintsProcessor` builder API, providing a mapping from hint code to a processing function (see [Custom Hint Handlers](#4-custom-hint-handlers)). By convention, codes in the range `0xA000`-`0xEFFFF` are recommended for custom use to avoid conflicts with current and future built-in types. If a data hint is received with an unregistered code, the processor returns an error and stops processing immediately.

### 1.5. Stream Protocol

A valid hint stream follows this protocol:

```
CTRL_START                          ← Reset state, begin stream
  [Hint_1] [Hint_2] ... [Hint_N]   ← Data hints (precompile, input, or custom)
CTRL_END                            ← Wait for completion, end stream
```

## 2. Hints in CLI Execution

There are four CLI commands (`execute`, `prove`, `verify-constraints`, `stats`) that support hints stream system by providing a URI via the `--hints` option. The URI determines the input stream source for hints, which can be a file, Unix socket, QUIC stream, or other custom transport.
The supported schemes are:
```
--hints file://path      → File stream reader
--hints unix://path      → Unix socket stream reader
--hints quic://host:port → Quic stream reader
--hints (plain path)     → File stream reader
```

> **Note:** Only ASM mode supports hints. The emulator mode does not use the hints pipeline.

## 3. Hints in Distributed Execution

In the distributed proving system, hints are received by the `coordinator` and broadcasted to all **workers** via gRPC. The coordinator runs a relay that validates incoming hint messages, assigns sequence numbers for ordering, and dispatches them to `workers` asynchronously. `Workers` buffer incoming messages and reorder them by sequence number before processing. The processed hints are then submitted to the sink in the correct order.
There is another mode where workers can load hints from a local path/URI instead of streaming from the coordinator, which is useful for debugging.

### 3.1. Architecture

```mermaid
flowchart TD
    A["Guest program<br/><small>Emits hints request</small>"] --> B

    subgraph H["Coordinator"]
        B["ZiskStream"]
        B --> C["Hints Relay<br/><small>Validates<br>Broadcast to all workers (async)</small>"]
    end

    C --> E["Worker 1<br/><small>Stream incoming hints + Reorder</small>"]
    C --> F["Worker 2<br/><small>Stream incoming hints + Reorder</small>"]
    C --> G["Worker N<br/><small>Stream incoming hints + Reorder</small>"]

    E --> E1["HintsProcessor<br/><small>Parallel engine</small>"]
    E1 --> E2["StreamSink<br/><small>ASM emulator/file output</small>"]

    F --> F1["HintsProcessor<br/><small>Parallel engine</small>"]
    F1 --> F2["StreamSink<br/><small>ASM emulator/file output</small>"]

    G --> G1["HintsProcessor<br/><small>Parallel engine</small>"]
    G1 --> G2["StreamSink<br/><small>ASM emulator/file output</small>"]

    style H fill:transparent,stroke-dasharray: 5 5
```

When the `coordinator` receives a hint request from the guest program, it parses the incoming `u64` stream, validates control codes, assigns sequence numbers for ordering, and broadcasts the data to all workers.

Three message types are sent over gRPC to workers:

| StreamMessageKind | When | Payload |
|---|---|---|
| `Start` | On `CTRL_START`     | None |
| `Data`  | For each data batch | Sequence number + raw bytes |
| `End`   | On `CTRL_END`       | None |

Each worker receives the stream of hints, buffers them if they arrive out of order, and sends them to the `HintsProcessor` for parallel processing. The `HintsProcessor` ensures that results are submitted to the sink in the original order.

### 3.2. Hints Mode Configuration

When starting a worker, if the `--hints` option is provided, the worker prepares to receive hints from the coordinator.
When launching a proof generation job where hints will be provided, the workers must be started to receive and process hints.
A hints stream system can be configured in two ways:
* **Streaming mode**: Workers receive hints from the coordinator via gRPC. This is the default and recommended mode for production, as it allows real-time processing of hints as they are generated.
* **Path mode**: Workers load hints from a local path/URI. This is useful for debugging or when hints are pre-generated and stored in a file. In this mode, the coordinator does not send hints to workers; instead, each worker reads the hints directly from the specified path.

#### 3.2.1 Coordinator Hints Streaming Mode

To start the coordinator in streaming mode, provide the `--hints-uri` option with a URI that the `coordinator` will connect to, and set `--stream-hints` to enable broadcasting to workers. The URI determines the input stream source for hints.
The supported schemes are:
```
--hints-uri file://path      → File stream reader
--hints-uri unix://path      → Unix socket stream reader
--hints-uri quic://host:port → Quic stream reader
--hints-uri (plain path)     → File stream reader
```

Example to launch a prove command in streaming mode:
```
zisk-coordinator prove --hints-uri unix:///tmp/hints.sock --stream-hints ...
```

#### 3.2.2 Worker Hints non-Streaming Mode

To start a worker in non-streaming mode, provide the `--hints-uri` option with a URI that points to the local workers path where hints are stored, without the `--stream-hints` option. In this mode the worker(s) will load hints from the specified URI instead of receiving them from the coordinator. This mode is useful for debugging or when hints are pre-generated and stored in a file.

## 4. Custom Hint Handlers

Register custom handlers via the builder pattern:

```rust
let processor = HintsProcessor::builder(my_sink)
    .custom_hint(0xA000, |data: &[u64]| -> Result<Vec<u64>> {
        // Custom processing logic
        Ok(vec![data[0] * 2])
    })
    .custom_hint(0xA001, |data| {
        // Another custom handler
        Ok(transform(data))
    })
    .build()?;
```

**Requirements:**
- Handler function must be `Fn(&[u64]) -> Result<Vec<u64>> + Send + Sync + 'static`.
- Custom hint codes should not conflict with built-in codes (`0x0000`-`0x0700`). By convention, use codes in the range `0xA000`-`0xFFFF`.

## 5. Generating Hints in Guest Programs

To generate hints from the guest program you need to follow these steps and requirements:

1. **Emit hint requests**: Patch your code or dependent crates to call the external FFI Hints helper functions that generate the hints input data required later by the `HintsProcessor`. See [FFI Hints Helper Functions](#55-ffi-hints-helper-functions) for the list of available built-in FFI Hints helper functions, or [Custom Hints Generation](#56-custom-hints-generation) to learn how to generate custom hints from the guest program.
2. **Add the `ziskos` crate** to your guest `Cargo.toml`.
3. **Initialize and finalize the hint stream**: Call the hints init and close functions immediately before and after the section of code that executes precompile logic.
4. **Enable hints at compile time**: Compile your guest program with `RUSTFLAGS='--cfg zisk_hints'` for the native target to activate hint code generation and FFI helper functions in the `ziskos` crate.
5. **Ensure deterministic execution**: Verify that both the native execution that generates hints and the guest compiled for the `zkvm/zisk` target execute deterministically and produce/consume hints in the exact same order. See [Deterministic Execution Requirement](#54-deterministic-execution-requirement).

To illustrate these steps, consider the `zec-reth` guest program, which executes and verifies Ethereum Mainnet blocks using the ZisK zkVM:

https://github.com/0xPolygonHermez/zisk-eth-client/tree/main-reth/bin/guest

### 5.1 Emit Hint Requests

`zec-reth` relies on `reth` crates, which expose a `Crypto` trait that allows a guest program to override precompile implementations. This enables zkVM-optimized implementations while also emitting hints so the computation can be performed outside the zkVM.

For example, the BN254 elliptic curve addition (`bn254_g1_add`) implementation for the `Crypto` trait can be found here:

https://github.com/0xPolygonHermez/zisk-eth-client/blob/86b71b39d35efb9894696cab115a1177f3e47dbf/crates/guest-reth/src/crypto/impls.rs#L87

In that file, two target-specific implementations are provided: one for `zkvm/zisk` and one for native (non-zkVM) targets. When compiling with `--cfg zisk_hints` for the native target, the zkVM-specific implementation emits a hint request using the FFI helper:

```rust
#[cfg(zisk_hints)]
unsafe {
    pub fn hint_bn254_g1_add(p1: *const u8, p2: *const u8);
}
```

This call generates the hint input data using the exact input values that will later be used by the ZisK zkVM when executing the `zkvm/zisk` target code. This hint input data is consumed later by the `HintsProcessor`, allowing the `bn254_g1_add` computation to be performed outside the zkVM while remaining fully verifiable inside the circuit.

After the hint generation, execution continues in the native target code to compute the `bn254_g1_add` result.

From the guest program, we generate hints containing the input data for the corresponding `zisklib` functions (in this example, the `bn254_g1_add_c` function). These `zisklib` functions may internally invoke one or more precompiles to produce the final result.

When the hints are processed by the `HintsProcessor`, it executes the same `zisklib` function using the implementation code for the zkvm/zisk target. This produces the exact precompile results expected when executing the guest ELF inside the zkVM.

As a result, for each `zisklib` function invocation, the `HintsProcessor` may generate one or more precompile hint results corresponding to the precompile inputs originally emitted by the guest.

### 5.2 Initialize/Finalize Hint Stream

To start hints generation from your guest program you must call one of the following functions from the `ziskos::hints` crate:

```rust
pub fn init_hints_file(hints_file_path: PathBuf, ready: Option<oneshot::Sender<()>>) -> Result<()>
```

This function stores the generated hints in the file specified by the `hints_file_path` parameter.

```rust
pub fn init_hints_socket(socket_path: PathBuf, debug_file: Option<PathBuf>, ready: Option<oneshot::Sender<()>>) -> Result<()>
```

This function sends the hints through the Unix socket specified by the `socket_path` parameter.

The optional `ready` parameter can be used for synchronization with the host when the guest program is executed in a separate thread to generate hints in parallel. It signals `ready` when the hints generation is ready to start writing hints through the Unix socket.

The optional `debug_file` parameter can be used to store, in the specified file, a copy of the hints sent through the socket. This file can later be used for debugging purposes.

To close hints generation you must call:

```rust
pub fn close_hints() -> Result<()>
```

You should call these functions only when the guest is compiled for the native target used for hints generation. This can be achieved by placing the code under the following configuration flag:

```rust
#[cfg(zisk_hints)]
{
    // Initialization/Finalize Hints generation code
    ...
}
```

You can review how hints generation is initialized and finalized in the `zec-reth` guest here:

https://github.com/0xPolygonHermez/zisk-eth-client/blob/main-reth/bin/guest/src/main.rs

### 5.3 Enable Hints at Compile Time

Once the guest program is set up to generate hints for the native target, it must be compiled with the `zisk_hints` configuration flag enabled:

```bash
RUSTFLAGS='--cfg zisk_hints' cargo build --release
```

After compiling, executing the guest program will generate the hints binary file at the specified location (if `init_hints_file` was used) or start writing hints to the specified Unix socket (if `init_hints_socket` was used).

If a hints file was generated, it can be consumed using the `--hints` flag in the `cargo-zisk` commands that support hints (as explained in [Hints in CLI Execution](#2-hints-in-cli-execution)).

If you want to display metrics in the console about the number of hints generated during native guest execution, you can additionally compile the guest with the `--cfg zisk_hints_metrics` flag.

To enable hint support when executing the guest inside the zkVM (ELF guest), you must pass the `--hints` flag when generating the assembly ROM using the `cargo-zisk rom-setup` command.

**NOTE:** Hint processing is not supported when executing the guest ELF file in emulation mode.

### 5.4 Deterministic Execution Requirement

An important requirement of the hints generation flow is that the native execution that generates the hints must be fully deterministic and always produce hints in the exact same order.

Furthermore, the order of hints generated during native execution must match the order in which the guest program compiled for the `zkvm/zisk` target expects to receive them. Since the zkVM execution is also deterministic, any divergence in hint ordering between native execution and zkVM execution will result in incorrect behavior.

To guarantee deterministic hint generation, the code paths that directly or indirectly generate hints must avoid:

- The use of threads or parallel execution.
- Data structures such as `HashMap` (or any structure based on randomized hash seeds) when iterated in loops that directly or indirectly call precompile/hint functions.

Using threads or iterating over non-deterministically ordered data structures may cause the hint generation order to vary between runs, breaking the required alignment between native and zkVM executions.

### 5.5 FFI Hints Helper Functions

| Code | Function |
| ---- | -------- |
| `0x0100` | `fn hint_sha256(f_ptr: *const u8, f_len: usize);` |
| `0x0200` | `fn hint_bn254_g1_add(p1: *const u8, p2: *const u8);`|
| `0x0201` | `fn hint_bn254_g1_mul(point: *const u8, scalar: *const u8);` |
| `0x0205` | `fn hint_bn254_pairing_check(pairs: *const u8, num_pairs: usize);` |
| `0x0300` | `fn hint_secp256k1_ecdsa_address_recover(sig: *const u8, recid: *const u8, msg: *const u8);` |
| `0x0301` | `fn hint_secp256k1_ecdsa_verify_and_address_recover(sig: *const u8, msg: *const u8, pk: *const u8);` |
| `0x0380` | `fn hint_secp256r1_ecdsa_verify(msg: *const u8, sig: *const u8, pk: *const u8);` |
| `0x0400` | `fn hint_bls12_381_g1_add(a: *const u8, b: *const u8);` |
| `0x0401` | `fn hint_bls12_381_g1_msm(pairs: *const u8, num_pairs: usize);` |
| `0x0405` | `fn hint_bls12_381_g2_add(a: *const u8, b: *const u8);` |
| `0x0406` | `fn hint_bls12_381_g2_msm(pairs: *const u8, num_pairs: usize);` |
| `0x040A` | `fn hint_bls12_381_pairing_check(pairs: *const u8, num_pairs: usize);` |
| `0x0410` | `fn hint_bls12_381_fp_to_g1(fp: *const u8);` |
| `0x0411` | `fn hint_bls12_381_fp2_to_g2(fp2: *const u8);` |
| `0x0500` | `fn hint_modexp_bytes(base_ptr: *const u8, base_len: usize, exp_ptr: *const u8, exp_len: usize, modulus_ptr: *const u8, modulus_len: usize);` |
| `0x0600` | `fn hint_verify_kzg_proof(z: *const u8, y: *const u8, commitment: *const u8, proof: *const u8);` |
| `0x0700` | `fn hint_keccak256(input_ptr: *const u8, input_len: usize);` |
| `0x0800` | `fn hint_blake2b_compress(...);` |
| `0xF0000` | `fn hint_input_data(input_data_ptr: *const u8, input_data_len: usize);` |

### 5.6 Custom Hints Generation
To extend the built-in hints, you can generate custom hints for new operations. The first step is to register the new hint in the `HintsProcessor`, as explained in section [Custom Hint Handlers](#4-custom-hint-handlers). Once the hint is registered, you can generate hints for it from the guest program using the following FFI function:

```rust
fn hint_custom(hint_id: u32, data_ptr: *const u8, data_len: usize, is_result: u8);
```

and following the same guidelines described for the built-in FFI hint helper functions.
