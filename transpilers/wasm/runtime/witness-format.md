# Nitro native runtime witness v1

This format feeds `runtime_state(0, args, mem_end)` inside the transpiled replay
ROM. The witness is private and untrusted. The runtime derives the end state
from replay's global-state writes and derives both inbox claims from message
bytes actually requested during replay. No end state or claim is accepted as
input. Before-accumulator/count anchors are private openings committed by the
resulting claims; their external L1/binder authentication remains the consumer's
responsibility, as in the reference guest.

All integer fields below are unsigned, little endian, and unaligned. Lengths
count bytes. There is no padding. The fixed header is exactly 180 bytes:

| Offset | Length | Field |
| --- | --- | --- |
| 0 | 8 | ASCII `NZKWIT01` (magic and version) |
| 8 | 80 | Starting state: bytes32[0], bytes32[1], u64[0], u64[1] |
| 88 | 32 | Sequencer accumulator before |
| 120 | 32 | Delayed accumulator before |
| 152 | 8 | Sequencer count start |
| 160 | 8 | Delayed count start |
| 168 | 4 | Number of typed preimages |
| 172 | 4 | Number of sequencer messages |
| 176 | 4 | Number of delayed messages |

The header is followed by all typed preimages, then all sequencer messages,
then all delayed messages. Each preimage is `type:u8 | key:32 bytes |
length:u32 | bytes:length`. Entries must be strictly sorted by `(type, key)`,
with keys compared lexicographically as unsigned bytes. Each message is
`number:u64 | length:u32 | bytes:length`. Each inbox list must be strictly
increasing by number. Duplicates, descending entries, truncation, overflowing
lengths, and trailing bytes trap. Optional unused entries are permitted: only
requested inbox messages contribute to the consumed window.

Bounds are 8,192 preimages, 1,024 messages in each inbox, and 64 MiB total
payload. Indexes and verified/consumed flags use about 320 KiB BSS. Payload bytes
remain in read-only ZisK input memory and are referenced without allocating or
copying them. Increase these explicit bounds only when a required witness
exceeds them and the runtime's shared RAM budget has been checked.

A requested type-0 preimage is checked with Keccak-256; type 1 with SHA-256.
Other present requested types trap because their authentication is unsupported.
Successful checks alone populate the cache. Missing required data traps;
`validateCertificate` returns zero for a missing entry, including an unknown
type with no matching entry. A present certificate is authenticated before
returning one.

Inbox hashing, delayed header packing, domains, and claim packing match
`arb-zisk/acc-core/src/lib.rs` and `guest/src/env.rs`. Every used inbox number
must be contiguous from its count-start anchor, regardless of request order.
Repeated partial reads fold a message once. Count overflow traps. Every nonzero
sequencer `afterDelayed` must equal the delayed count snapshot: the starting
count if no delayed message was consumed, otherwise the ending count. A consumed
delayed message requires a nonzero sequencer snapshot. Multiple different delayed
checkpoints are unsupported and trap. Sequencer/delayed headers shorter than
40/113 bytes respectively trap when requested.

## Native call ABI

`uint64_t runtime_state(uint64_t op, const uint64_t args[8], uint64_t mem_end)`.
Only the compiler's bootstrap/finalizer calls operations 0 and 1. `mem_end` is
the absolute end of current Wasm linear memory; import pointers are offsets
checked by `runtime_checked_range` before reading or writing. The compiler
canonicalizes i32 Wasm values. Preimage types are truncated to u8, matching the
reference adapter. Memory accesses also reject out-of-range pointers.

| Op | Arguments in `args` | Result |
| --- | --- | --- |
| 0 | absolute input pointer, length, absolute ROM module-hash pointer | 0 |
| 1 | absolute output pointer | 0 |
| 10 | global index, output offset | 0 |
| 11 | global index, source offset | 0 |
| 12 | global index | u64 value |
| 13 | global index, u64 value | 0 |
| 14 | sequencer number, message offset, output offset | bytes copied, at most 32 |
| 15 | delayed number, message offset, output offset | bytes copied, at most 32 |
| 16 | preimage type, key offset, preimage offset, output offset | bytes copied, at most 32 |
| 17 | preimage type, key offset | 0 or 1 |
| 18 | preimage type, key offset, preimage offset, available, output offset | full remaining length |

Operation 16 requires a 32-byte-aligned preimage offset. Operation 18 permits
unaligned offsets and copies at most `available`. Reads past the end return
zero bytes. Global indexes are bounded to 0 or 1. Operations before successful
initialization or after finalization trap. Reinitialization is reserved for
bootstrap; every parse resets all active verification/consumption flags.

In native builds input must be `NITRO_INPUT_DATA` (`0x10000010`), module hash
must be `NITRO_MODULE_HASH` (`0xa0540300`), and output must be `NITRO_OUTPUT`
(`0xa0410000`). The parent compiler loads the replay SHA-256 as a ROM constant
and makes input storage inaccessible to Wasm writes. The host-test-only
`NITRO_HOST_TEST` define permits ordinary fixture pointers; it must never be
set for the RISC-V runtime.

Finalization writes exactly 256 bytes:
`replay_sha256:32 | start_state:80 | end_state:80 | seq_claim:32 |
delayed_claim:32`. Each claim is Keccak-256 of
`domain:8 | start_state:80 | end_state:80 | accumulator_before:32 |
accumulator_after:32 | count_start:u64 | count_end:u64`, using `ARBZSEQ1` or
`ARBZDEL1` as its domain.

## Verification

Run from the task worktree. The native tests use the real parser/state code,
CommonCrypto SHA-256 on macOS (OpenSSL SHA-256 plus `-lcrypto` on Linux), and an
independent test-only Keccak implementation checked against the empty digest.
They check both final claim byte layouts independently and cover malformed
witnesses, forged/missing/unsupported preimages, verified caching, memory bounds,
state writes, inbox deduplication, header lengths, contiguous counts, count
overflow, missing/mismatched/multiple delayed snapshots, and lifecycle failures.

```sh
mkdir -p target/nitro-runtime-state
clang -std=c11 -Wall -Wextra -Werror -DNITRO_HOST_TEST \
  -fsanitize=address,undefined transpilers/wasm/runtime/state-tests.c \
  transpilers/wasm/runtime/state.c -o target/nitro-runtime-state/state-tests
target/nitro-runtime-state/state-tests
/opt/homebrew/opt/llvm/bin/clang --target=riscv64-unknown-elf -march=rv64im \
  -mabi=lp64 -mcmodel=medany -mno-relax -ffreestanding -fno-builtin -O2 \
  -Wall -Wextra -Werror -I transpilers/wasm/runtime/include \
  -c transpilers/wasm/runtime/state.c -o target/nitro-runtime-state/state.o
```

The parent task owns actual VM integration and real native crypto verification;
host tests alone do not establish VM, Assembly, or proof execution.

## Recorded transfer preparation

`prepare-witness.py` uses Python's standard library and emits one framed witness
record plus a separate 256-byte expected output. It accepts one complete
non-Stylus batch, zero start/end positions, and no consumed delayed message.
It follows the reference host's zero accumulator-before defaults and infers the
delayed starting count from the batch's big-endian `afterDelayed` header field.
It rejects unsupported ranges rather than guessing which messages replay uses.
`ExpectedEndState` is used exclusively for the separate test oracle; changing it
does not change the VM witness. The Python Keccak implementation is an oracle
for expected claims, while native C and VM crypto perform independent hashes.

```sh
python3 transpilers/wasm/runtime/prepare-witness.py --self-test
python3 transpilers/wasm/runtime/prepare-witness.py \
  --block-file /path/to/block_inputs_5.json \
  --replay-wasm /path/to/transfer-replay.wasm \
  --output target/nitro-runtime/transfer.stdin \
  --expected target/nitro-runtime/transfer.expected
target/nitro-runtime-state/state-tests \
  target/nitro-runtime/transfer.stdin target/nitro-runtime/transfer.expected
```

The last command checks the real recorded witness parser and crosschecks both
Python-generated claims with C's independent Keccak/packing. Its test harness
sets globals to the recorded expected end state to isolate claim encoding;
only the parent's integrated VM replay establishes that replay computes that
end state itself.
