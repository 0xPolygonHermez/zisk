# Hint File

Utility to read, analyze, and filter Zisk hints files.

## Features

- **Summary mode**: Display statistics by hint type (count, total bytes, min/max)
- **Detail mode**: Show detailed list of all hints in the file
- **Filter mode**: Exclude specific hint types from output (remove unwanted hints)
- **Extract mode**: Include only specific hint types in output (keep wanted hints)
- **Inject mode**: Interleave binary input file chunks with existing hints
- **Validation**: Check file integrity and detect garbage data

## Hints File Format

The hints file has the following structure:

1. **8-byte header** at the beginning (skipped during processing)
2. **Multiple hints**, each with:
   - 8-byte header (length + type)
   - Data payload (8-byte aligned)
3. **Final tag**: A hint with type=1 and length=0 marking the end

Each hint has the following structure:

```
┌────────────────────────────────┐
│  Length (u32 LE) - 4 bytes     │  ← Actual length in bytes
├────────────────────────────────┤
│  Type (u32 LE) - 4 bytes       │  ← Hint type
├────────────────────────────────┤
│  Data - aligned bytes          │  ← Hint payload (padded to 8-byte boundary)
└────────────────────────────────┘
```

- **Length**: Actual number of bytes of data (u32, little-endian)
- **Type**: Hint type identifier (e.g., 0xF0000 for INPUT, 0x0100 for SHA256, etc.)
- **Data**: The hint payload, stored in multiples of 8 bytes (padded if needed)

⚠️ **Important notes**: 
- The length field contains the actual data size in bytes
- Data is stored aligned to 8-byte boundaries (e.g., 12 bytes of data will occupy 16 bytes in the file)
- The length does NOT include the 8-byte hint header (u32 + u32)
- The file ends when a hint with type=1 and length=0 is found
- The tool verifies there is no garbage data after the final tag

## Build

```bash
cd tools/hint_file
cargo build --release
```

The binary will be generated at `target/release/hint_file`.

## Usage

### Basic Usage (Summary Mode)

By default, the tool shows a summary of hints found in the file:

```bash
./target/release/hint_file <hints_file>
```

Or:

```bash
cargo run --release -- <hints_file>
```

### Show Detailed List

Use `-d` or `--detail` to see all hints:

```bash
./target/release/hint_file --detail <hints_file>
```

### Filter Hints (Exclude)

Remove specific hint types from the output file using `-f` or `--filter` and `-o` or `--output`:

```bash
# Filter out single type (hex) - keeps all EXCEPT 0xF0000
./target/release/hint_file -f 0xF0000 -o output.bin input.bin

# Filter out multiple types - keeps all EXCEPT these
./target/release/hint_file -f 0xF0000,256,0x0100 -o output.bin input.bin

# Filter with detail view
./target/release/hint_file -d -f 0x0100 -o no_sha256.bin hints.bin
```

### Extract Hints (Include)

Keep only specific hint types in the output file using `-e` or `--extract` and `-o` or `--output`:

```bash
# Extract single type (hex) - keeps ONLY 0xF0000
./target/release/hint_file -e 0xF0000 -o output.bin input.bin

# Extract multiple types - keeps ONLY these
./target/release/hint_file -e 0xF0000,256,0x0100 -o output.bin input.bin

# Extract with detail view
./target/release/hint_file -d -e 0x0100 -o sha256_only.bin hints.bin
```

### Inject Input (Interleave)

Interleave chunks from a binary input file with existing hints using `--inject-input` and related options:

```bash
# Basic inject - split input.dat into chunks and interleave with hints
./target/release/hint_file --inject-input input.dat -o output.bin hints.bin

# Custom chunk size (1024 bytes including 8-byte header = 1016 bytes of data per chunk)
./target/release/hint_file --inject-input input.dat --inject-chunk-size 1024 -o output.bin hints.bin

# Start injection after first 10 hints
./target/release/hint_file --inject-input input.dat --inject-start 10 -o output.bin hints.bin

# Inject 2 chunks at a time, then skip 3 original hints, repeat
./target/release/hint_file --inject-input input.dat \
  --inject-group-size 2 \
  --inject-distance 3 \
  -o output.bin hints.bin

# Full example with all parameters
./target/release/hint_file --inject-input input.dat \
  --inject-start 5 \
  --inject-chunk-size 512 \
  --inject-group-size 3 \
  --inject-distance 2 \
  --inject-type 0xF0000 \
  -o output.bin hints.bin
```

**Inject Parameters:**
- `--inject-input <FILE>`: Binary file to split and inject (required for inject mode)
- `--inject-start <N>`: Start position (0 = beginning, 1 = after first hint) [default: 0]
- `--inject-chunk-size <BYTES>`: Size of each chunk including 8-byte header [default: 1024]
- `--inject-group-size <N>`: Number of chunks to inject together [default: 1]
- `--inject-distance <N>`: Number of original hints between groups [default: 1]
- `--inject-type <TYPE>`: Hint type for injected chunks (hex/decimal) [default: 0xF0000]

**How it works:**
1. The input file is split into chunks of `inject-chunk-size` (minus 8 bytes for header)
2. Injection starts at position `inject-start` in the original hints
3. Each group of `inject-group-size` chunks is written together
4. Between groups, `inject-distance` original hints are preserved
5. Any remaining chunks are appended at the end

**Example pattern** (start=1, group=2, distance=3):
```
Original: [H0, H1, H2, H3, H4, H5, H6, H7, ...]
Result:   [H0, I0, I1, H1, H2, H3, I2, I3, H4, H5, H6, I4, I5, ...]
          └─┘  └────┘  └────────┘  └────┘  └────────┘  └────┘
        start  group    distance    group    distance    group
```

### Combined Options

```bash
# Show both detail and summary
./target/release/hint_file --detail --summary <hints_file>

# Filter (exclude types) and show summary
./target/release/hint_file -f 0xF0000 -o filtered.bin -s input.bin

# Extract (include only types) and show summary
./target/release/hint_file -e 0x0100,0x0200 -o extracted.bin -s input.bin

# Inject with detail view
./target/release/hint_file --inject-input data.bin --detail -o output.bin hints.bin
```

## Command-Line Options

- `<INPUT>`: Input hints file (required)
- `-d, --detail`: Show detailed list of all hints
- `-s, --summary`: Show summary statistics (default if --detail not used)
- `-o, --output <FILE>`: Output file for filtered/extracted/injected hints
- `-f, --filter <TYPES>`: Exclude hint types (comma-separated, keeps all EXCEPT these)
  - Supports decimal: `-f 256,512`
  - Supports hexadecimal: `-f 0xF0000,0x0100`
  - Mixed formats: `-f 0xF0000,256,0x0100`
  - Cannot be used with `--extract`
- `-e, --extract <TYPES>`: Include only hint types (comma-separated, keeps ONLY these)
  - Supports decimal: `-e 256,512`
  - Supports hexadecimal: `-e 0xF0000,0x0100`
  - Mixed formats: `-e 0xF0000,256,0x0100`
  - Cannot be used with `--filter`
- `--inject-input <FILE>`: Binary input file to inject as hints (requires `--output`)
- `--inject-start <N>`: Start position for injection [default: 0]
- `--inject-chunk-size <BYTES>`: Chunk size including header [default: 1024]
- `--inject-group-size <N>`: Chunks per group [default: 1]
- `--inject-distance <N>`: Original hints between groups [default: 1]
- `--inject-type <TYPE>`: Hint type for injected chunks [default: 0xF0000]

## Example Outputs

### Summary Mode (Default)

```
=== Summary ===
File: hints_results_0.bin
Total hints: 150

  Type (hex) |    Count |  Total (bytes) |  Min (bytes) |  Max (bytes)
--------------------------------------------------------------------------------
  0x00000100 |       10 |           2560 |          256 |          256
  0x00000200 |       20 |           2560 |          128 |          128
  0x000F0000 |      120 |         122880 |         1024 |         1024
```

### Detail Mode

```
Reading hints file: hints_results_0.bin
Header: [00, 00, 00, 00, 00, 00, 00, 00]
--------------------------------------------------------------------------------
 Index |   Type (hex) |   Len (bytes) | Aligned (bytes)
--------------------------------------------------------------------------------
     0 |   0x000F0000 |         1024 |           1024
     1 |   0x00000100 |          256 |            256
     2 |   0x00000200 |          128 |            128
     3 |   0x00000300 |           12 |             16
--------------------------------------------------------------------------------
Total hints processed: 4
Final tag: length=0, type=1 (0x00000001)
```

### Filter Mode (Exclude)

```
=== Summary ===
File: input.bin
Total hints: 150

  Type (hex) |    Count |  Total (bytes) |  Min (bytes) |  Max (bytes)
--------------------------------------------------------------------------------
  0x00000100 |       10 |           2560 |          256 |          256
  0x00000200 |       20 |           2560 |          128 |          128

Filtered hints written to: output.bin (excluded types: 0x000F0000)
```

### Extract Mode (Include)

```
=== Summary ===
File: input.bin
Total hints: 150

  Type (hex) |    Count |  Total (bytes) |  Min (bytes) |  Max (bytes)
--------------------------------------------------------------------------------
  0x000F0000 |      120 |         122880 |         1024 |         1024

Extracted hints written to: output.bin (included types: 0x000F0000)
```

### Inject Mode (Interleave)

```
=== Summary ===
File: input.bin
Total hints: 150

  Type (hex) |    Count |  Total (bytes) |  Min (bytes) |  Max (bytes)
--------------------------------------------------------------------------------
  0x00000100 |       10 |           2560 |          256 |          256
  0x00000200 |       20 |           2560 |          128 |          128
  0x000F0000 |      145 |         147456 |         1016 |         1024

Hints with injected input written to: output.bin (25 chunks injected, type: 0x000F0000)
  Inject parameters: start=0, chunk_size=1024, group_size=1, distance=5
```

## Known Hint Codes

### Control Codes
- `0x0000` - CTRL_START: Reset state
- `0x0001` - CTRL_END: End processing
- `0x0002` - CTRL_CANCEL: Cancel stream
- `0x0003` - CTRL_ERROR: Error

### Data Hints
- `0xF0000` - HINT_INPUT: Input data
- `0x0100` - HINT_SHA256: SHA-256
- `0x0200` - HINT_BN254_G1_ADD: BN254 G1 Add
- `0x0201` - HINT_BN254_G1_MUL: BN254 G1 Mul
- `0x0205` - HINT_BN254_PAIRING_CHECK: BN254 Pairing
- `0x0300` - HINT_SECP256K1_ECDSA_ADDRESS_RECOVER: secp256k1 recover
- `0x0301` - HINT_SECP256K1_ECDSA_VERIFY_ADDRESS_RECOVER: secp256k1 verify+recover
- `0x0380` - HINT_SECP256R1_ECDSA_VERIFY: secp256r1 verify
- `0x0400` - HINT_BLS12_381_G1_ADD: BLS12-381 G1 Add
- `0x0401` - HINT_BLS12_381_G1_MSM: BLS12-381 G1 MSM
- `0x0405` - HINT_BLS12_381_G2_ADD: BLS12-381 G2 Add
- `0x0406` - HINT_BLS12_381_G2_MSM: BLS12-381 G2 MSM
- `0x040A` - HINT_BLS12_381_PAIRING_CHECK: BLS12-381 Pairing
- `0x0410` - HINT_BLS12_381_FP_TO_G1: BLS12-381 Fp to G1
- `0x0411` - HINT_BLS12_381_FP2_TO_G2: BLS12-381 Fp2 to G2
- `0x0500` - HINT_MODEXP: Modular exponentiation
- `0x0600` - HINT_VERIFY_KZG_PROOF: KZG verification
- `0x0700` - HINT_KECCAK256: Keccak-256
- `0x0800` - HINT_BLAKE2B_COMPRESS: Blake2b

For more details, see `common/src/hints.rs`.
