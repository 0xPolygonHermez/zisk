# Profiling Programs with ZiskEmu

ZiskEmu provides powerful profiling capabilities to analyze the cost and performance characteristics of your programs. This guide explains how to use these features to identify hotspots, optimize your code, and understand resource consumption.

## What This Guide Covers

This guide walks you through ZiskEmu's profiling capabilities, progressing from high-level overviews to detailed analysis:

1. **Introduction**: Understanding profiling costs vs. final costs, symbol-based analysis, and detecting optimization opportunities

2. **Basic Profiling**: Global statistics showing cost distribution across major categories (base, main, opcodes, precompiles, memory)

3. **Function-Level Profiling**: Identifying which functions consume the most resources with cumulative analysis

4. **Customizing ROI Display**: Controlling how many functions to show and filtering by patterns

5. **Detailed Caller Analysis**: In-depth breakdown showing which operations are expensive within each function and who calls them

6. **Tracking Function Calls**: Logging individual call parameters to analyze usage patterns and optimize for common cases

7. **PC Histogram Analysis**: Low-level view of the most frequently executed RISC-V instruction sequences

8. **Additional Options**: Quick reference for other useful flags (steps, progress indicators, formatting)

9. **Practical Example**: Real-world case study analyzing Ethereum opcode costs in a block validator

## Introduction

### Understanding Profiling Costs vs. Final Costs

When profiling a program in ZisK, it's important to understand the difference between **profiling costs** and **final costs**:

#### Profiling Costs

**Profiling costs** represent the individual operational cost accrued directly within a function's own instructions, based on the best-case cost model for each operation. These costs:

- Exclude costs padding or aggregation costs
- Reflect a **direct cause-and-effect relationship** between code changes and cost variations
- Use the optimal cost for each operation type
- Allow you to observe how small program modifications affect performance
- Are ideal for **optimization work** because they show the direct impact of your code changes

For example, when you replace a function with a precompiled function or optimize a loop, the profiling cost will immediately reflect this improvement, making it easy to validate that your optimization is working as expected.

#### Final Costs

**Final costs** represent the **real and exact cost** of a specific execution, accounting for the actual resource consumption in the ZisK proving system. The key difference is that final costs measure cost at the **instance granularity**, not at the individual operation level.

In ZisK's architecture, multiple operations are grouped into **instances** (execution units in state machines), and the cost is determined by these instances:

- **Instance-based granularity**: If you use 1 Keccak operation or 5,242 Keccak operations, you pay for one full Keccak instance. However, if you use 5,243 operations, you need a second instance, effectively doubling the cost for that single additional operation.

- **Planner strategies**: The ZisK planner dynamically chooses execution strategies based on the operation mix. For example, depending on how many additions and binary operations you have, the planner might use a Binary state machine, a BinaryAdd state machine, or both. These decisions affect the final cost since each instance type has a different cost structure.

- **Aggregation across function calls**: Final costs include both the function's own profiling cost and all costs from functions it calls, summed at the instance level.

**Why use profiling costs for optimization?** Because profiling costs provide a **predictable and proportional metric** directly tied to your code changes. When optimizing, you want to see the immediate effect of your changes at the operation level. Final costs, while representing the true execution cost, can show non-linear behavior due to instance boundaries and planning strategies. Once you've optimized based on profiling costs, the final costs will reflect the real resource savings in the proving system.

#### Example: Keccak Operations

Consider a program that performs Keccak hash operations:

**Scenario 1: Using 1,000 Keccak operations**
- **Profiling cost**: Proportional to 1,000 operations
- **Final cost**: 1 Keccak instance (fits within instance capacity)

**Scenario 2: Using 5,000 Keccak operations**
- **Profiling cost**: 5× the cost of Scenario 1 (proportional to operations)
- **Final cost**: Still 1 Keccak instance (if capacity is 5,242 operations)

**Scenario 3: Using 5,243 Keccak operations**
- **Profiling cost**: ~5.24× the cost of Scenario 1 (proportional increase)
- **Final cost**: 2 Keccak instances (crossed the instance boundary with just 1 extra operation!)

The profiling cost grows linearly with the number of operations, making it easy to predict the impact of adding or removing operations. The final cost, however, stays constant until you cross an instance boundary, then jumps significantly. This is why profiling costs are better for optimization: you can see the effect of every change, while final costs help you understand the actual proving cost in production.

#### Example: Comparing Optimization Alternatives

Suppose you have implemented two different optimizations for your program, and you need to decide which one is better. The difference between them is 1 million operations:

- **Option A**: Uses 1M 64-bit ADD operations
- **Option B**: Uses 1M 64-bit OR operations

In ZisK's architecture, there are **specialized instances for 64-bit additions** (BinaryAdd) that are much cheaper than the general **binary instances** (Binary) that can perform ADD, SUB, AND, OR, XOR, and other operations.

**Analysis with Profiling Costs:**
- Option A (ADD): Lower profiling cost (uses efficient specialized instances)
- Option B (OR): Higher profiling cost (requires general binary instances)
- **Clear winner**: Option A is better ✓

**Analysis with Final Costs (Small Program):**

If your program is small and doesn't fill a Binary instance:
- Both options may end up using the same Binary instance
- **Final cost**: Same for both options (no clear winner)
- **Misleading conclusion**: No difference between optimizations ✗

**Analysis with Final Costs (Large Program):**

If your program is larger and already uses separate instances:
- Option A uses a dedicated BinaryAdd instance (cheaper)
- Option B uses a Binary instance (more expensive)
- **Final cost**: Option A is clearly cheaper ✓
- **Correct conclusion**: Matches profiling cost analysis

**Lesson**: Profiling costs consistently show that Option A is better, regardless of program size. Final costs may give conflicting signals depending on whether instance boundaries are crossed. This is why profiling costs are the reliable metric for making optimization decisions—they provide a consistent signal that doesn't depend on the overall program context.

### Symbol-Based Analysis

One of ZiskEmu's key advantages is that profiling works on **any ELF file** without requiring special instrumentation or debug information. The profiler uses symbol information already present in the binary, which means:

- Works with **release builds** (optimized binaries)
- No need to recompile with special flags
- No runtime overhead during execution
- Analyzes production-ready binaries (not stripped)

### Detecting Optimization Opportunities

One of the most powerful uses of ZiskEmu's profiling is **identifying where to apply patches and optimizations**. The profiling costs help you answer critical questions:

**Which crates/libraries are most performant for proof generation?**
- Compare different library implementations to see their effect on verification costs
- Test alternative dependencies to find the most ZisK-efficient options
- Evaluate different algorithm implementations (e.g., hash libraries, cryptographic crates, serialization libraries) to determine which performs best in the ZisK proving system
- Make data-driven decisions when choosing between equivalent functionality from different crates

**Validating optimizations:**
- After applying a optimization or patch, run the profiler again to confirm the profiling cost decreased
- Compare before/after profiles to ensure the optimization is effective

**Is patching being applied correctly?**
- Verify that precompiles are being used where expected
- Detect cases or paths where generic code is running instead of optimized ZisK-specific implementations
- Identify functions that should be patched but aren't

**Where should you apply patches?**
- Find hotspot functions that would benefit most from ZisK precompiles
- Identify expensive cryptographic operations (SHA-256, Keccak, etc.) that could use hardware acceleration
- Locate arithmetic-heavy code that could leverage ZisK's optimized arithmetic operations



**Example workflow:**
1. Profile your program to identify expensive functions
2. Look for patterns that match available precompiles (hashing, big integer math, etc.)
3. Patch the code to use:
   - ZisK-optimized implementations
   - Precompiles  
   - Change operations or how they're used, considering you're optimizing for ZisK architecture, not hardware
4. Re-profile to verify the profiling cost reduction

This iterative approach, guided by profiling costs, ensures your optimizations target the right areas and produce measurable improvements.

## Basic Profiling (statistics)

The simplest way to profile your program is to use the `-X` (or `--stats`) flag. This provides an overview of execution statistics including total costs, memory operations, and opcode usage.

### Command

```bash
ziskemu -e \<elf\> -i \<input\> -X
```

### Output Explanation

```
REPORT                                  
----------------------------------------
STEPS                         92,875,129

COST DISTRIBUTION                   COST       %
------------------------------------------------
BASE                         293,601,280   2.57%
MAIN                       6,315,508,772  55.22%
OPCODES                    1,334,639,984  11.67%
PRECOMPILES                2,565,960,716  22.43%
MEMORY                       927,932,629   8.11%

TOTAL                     11,437,643,381 100.00%

FROPS                        963,440,253   8.42%
RAM USAGE                     18,465,008   3.47%

```

**Understanding the Report:**

**STEPS**: The number of processor cycles or instructions executed during program execution. This is an indicator of how long the program is—more steps mean a longer program execution.

**COST DISTRIBUTION**: This shows the **profiling cost** (see the [Understanding Profiling Costs](#understanding-profiling-costs-vs-final-costs) section for detailed explanation). Each operation is costed individually using the proof area as the metric, which is the best indicator of proof generation time—higher cost means longer proof generation.

The cost is broken down into these categories:

- **BASE**: Cost of fixed components such as tables, range checks, and other constant overhead that exists regardless of program logic.

- **MAIN**: Cost of the processor itself without operation costs. This is **directly proportional to the steps** count and represents the base cost of executing instructions.

- **OPCODES**: Cost of simple operations performed by the processor (additions, subtractions, etc.) in the format `a operation b = c, flag`, where a, b, and c are 64-bit values. These are basic arithmetic and logical operations.

- **PRECOMPILES**: Cost of complex operations whose parameters don't fit in 64 bits, requiring memory as an exchange system. Examples include:
  - 256-bit additions
  - Elliptic curve operations
  - Keccak hashing
  - DMA operations

- **MEMORY**: Cost of direct memory operations (read, write) and the additional state machines required for non-aligned memory access. This includes cases where:
  - The address is not aligned to 8 bytes
  - Operations don't work with 8-byte chunks (e.g., reading a single byte)

- **TOTAL**: Sum of all costs. Each category shows the percentage (%) it represents of the total cost.

**FROPS** (FRequent OPerationS): These are operations that are very frequently used by the processor, such as:
- Adding 1 to a relatively small number (common in loop counters)
- Adding 8 to an address (typical for pointer arithmetic)
- Working with values < 256

These frequent operations are analyzed, detected, and **pre-calculated**, becoming part of the BASE cost but representing significant savings. In this example, FROPS show 8.42% - this is the cost the program would have if these optimizations were not applied. The actual savings are already reflected in the lower costs of the affected operations.

**RAM USAGE**: The amount of memory used out of the total available. This information is **only available with the default allocator (bump allocator)**, which:
- Never frees memory - always allocates new memory
- Avoids the CPU cycles needed to manage the entire heap (typically >10% overhead)
- Is recommended as long as sufficient memory is available
- Provides better performance by eliminating heap management costs

**Detailed Opcode Breakdown:**

Below the summary, you'll see a detailed breakdown of each operation:
```
COST BY OPCODE                     COUNT       %            COST       % RANK
-----------------------------------------------------------------------------
OP ltu                         1,767,360   1.90%     106,041,600   0.93%
OP lt                            389,360   0.42%      23,361,600   0.20%
OP eq                            543,251   0.58%      32,595,060   0.28%
OP add                         7,086,411   7.63%     177,160,275   1.55% #4
OP sub                           693,157   0.75%      41,589,420   0.36%
OP and                         3,740,044   4.03%     224,402,640   1.96% #3
OP or                          7,482,273   8.06%     448,936,380   3.93% #2
OP xor                         1,027,290   1.11%      61,637,400   0.54%
OP add_w                          15,804   0.02%         948,240   0.01%
OP sub_w                           4,085   0.00%         245,100   0.00%
OP sll                         1,551,879   1.67%      82,249,587   0.72%
OP srl                           611,361   0.66%      32,402,133   0.28%
OP sra                           807,976   0.87%      42,822,728   0.37%
OP srl_w                          84,289   0.09%       4,467,317   0.04%
OP sra_w                              62   0.00%           3,286   0.00%
OP signextend_b                  121,977   0.13%       6,464,781   0.06%
OP signextend_h                    1,684   0.00%          89,252   0.00%
OP signextend_w                   27,460   0.03%       1,455,380   0.01%
OP pubout                             32   0.00%               0   0.00%
OP muluh                          86,682   0.09%       8,234,790   0.07%
OP mul                           409,765   0.44%      38,927,675   0.34%
OP divu                            6,368   0.01%         604,960   0.01%
OP remu                                4   0.00%             380   0.00%
OP dma_memcpy                    302,551   0.33%      12,707,142   0.11%
OP dma_memcmp                     91,454   0.10%       3,841,068   0.03%
OP dma_inputcpy                       90   0.00%           3,780   0.00%
OP dma_xmemset                    32,381   0.03%       1,360,002   0.01%
OP _dma_pre                      140,043   0.15%      12,323,784   0.11%
OP _dma_post                     164,752   0.18%      14,498,176   0.13%
OP keccak                         32,650   0.04%   2,466,707,500  21.57% #1
OP arith256_mod                      714   0.00%       1,016,736   0.01%
OP secp256k1_add                  17,688   0.02%      25,187,712   0.22%
OP secp256k1_dbl                  19,884   0.02%      28,314,816   0.25%
OP fcall_param                       652   0.00%               0   0.00%
OP fcall                             172   0.00%               0   0.00%
OP fcall_get                         156   0.00%               0   0.00%

FROPS BY OPCODE                    COUNT    HIT            COST       % RANK
----------------------------------------------------------------------------
FROP ltu                         942,288  34.78%      56,537,280   0.49% #4
FROP lt                          641,963  62.25%      38,517,780   0.34%
FROP eq                        3,273,419  85.77%     196,405,140   1.72% #2
FROP add                       1,597,142  18.39%      39,928,550   0.35%
FROP sub                         357,871  34.05%      21,472,260   0.19%
FROP and                         471,898  11.20%      28,313,880   0.25%
FROP or                        1,303,629  14.84%      78,217,740   0.68% #3
FROP xor                         105,118   9.28%       6,307,080   0.06%
FROP add_w                        75,366  82.67%       4,521,960   0.04%
FROP sub_w                         2,177  34.77%         130,620   0.00%
FROP sll                       8,729,869  84.91%     462,683,057   4.05% #1
FROP srl                         376,620  38.12%      19,960,860   0.17%
FROP sra                           5,962   0.73%         315,986   0.00%
FROP srl_w                        66,935  44.26%       3,547,555   0.03%
FROP sra_w                            60  49.18%           3,180   0.00%
FROP muluh                        25,590  22.79%       2,431,050   0.02%
FROP mul                          43,603   9.62%       4,142,285   0.04%
FROP divu                             42   0.66%           3,990   0.00%

```

**COST BY OPCODE Table:**

This table shows detailed statistics for each operation or precompile executed:

- **COUNT**: Number of times this operation was called
- **%**: Percentage of steps (cycles) that use this operation
- **COST**: Total profiling cost for all executions of this operation
- **%**: Percentage of total cost that this operation represents
- **RANK**: The top 4 most expensive operations are marked with `#1`, `#2`, `#3`, `#4`

**Important**: Operations are **not sorted by cost**. They maintain a consistent order across executions to facilitate comparison between different runs. Look for the `#N` markers to identify the most expensive operations.

For example, in this output, `keccak` was executed 32,650 times (0.03% of steps) but accounts for 21.41% of the total cost, making it the #1 most expensive operation. This indicates that Keccak operations dominate the cost despite being relatively infrequent.

**FROPS BY OPCODE Table:**

FROPS (Frequently-used OPerationS) are highly common operations that have been analyzed and optimized through pre-calculation. These include operations like:
- Incrementing by 1 (loop counters)
- Adding 8 (pointer arithmetic)
- Working with small values (< 256)

The table shows:

- **COUNT**: Number of times the FROP variant was executed
- **HIT**: Hit rate percentage - how often the frequent operation pattern was matched and the optimization applied
- **COST**: Total cost with the optimization benefit already applied
- **%**: Percentage of total cost
- **RANK**: Top ranked FROPS by cost

High hit rates indicate that the program uses these common patterns frequently, benefiting from the pre-calculated optimizations. The FROPS total shown earlier (8.42% in this example) represents the cost that would be added if these optimizations were not available.

**Key Insights from Statistics:**

Use this information to:
- Identify which operation types dominate your program's cost
- Find operations with high count but disproportionate cost (optimization candidates)
- Verify that precompiles are being used where expected
- Understand the balance between computation (OPCODES), memory access (MEMORY), and complex operations (PRECOMPILES)

## Function-Level Profiling

To understand which functions contribute most to your program's cost, add the `-S` (or `--read-symbols`) flag to read symbol information from the ELF file.

### Command

```bash
ziskemu -e target/elf/riscv64ima-zisk-zkvm-elf/release/guest -i input.bin -X -S
```

### Output Explanation

When symbol reading is enabled, ZiskEmu **simulates a call stack** to evaluate functions cumulatively. This means it tracks not only the cycles and cost of each function's own code, but also **all the calls made within that function**. This cumulative analysis provides a complete picture of each function's contribution to the total execution cost.

**Note**: Initial calls to `_start` or `_main` are filtered out as they represent 100% of the program and don't provide useful optimization insights.

ZiskEmu provides **two complementary analyses**:

**1. TOP STEP FUNCTIONS** - Analysis by execution cycles:

```
TOP STEP FUNCTIONS (STEPS, % STEPS, CALLS, STEPS/CALL, FUNCTION)
----------------------------------------------------------------
     54,831,894  59.04%          1      54,831,894 <reth_evm::execute::BasicBlockExecutor<&reth_evm
     53,951,767  58.09%          1      53,951,767 <alloy_evm::eth::block::EthBlockExecutor<alloy_e
     52,133,363  56.13%         70         744,762 <revm_handler::mainnet_handler::MainnetHandler<r
     48,406,973  52.12%     41,793           1,158 <zeth_mpt::mpt::node::Node<zeth_mpt::mpt::memoiz
     26,004,168  28.00%          1      26,004,168 <zeth_mpt_state::SparseState as stateless::trie:
     21,389,831  23.03%     41,590             514 <zeth_mpt::mpt::node::Node<zeth_mpt::mpt::memoiz
     16,104,120  17.34%      1,039          15,499 <revm_context::journal::inner::JournalInner<revm
     15,999,662  17.23%        841          19,024 <revm_context::journal::inner::JournalInner<revm
     15,635,579  16.84%      1,239          12,619 <revm_database::states::state::State<stateless::
     15,498,490  16.69%        388          39,944 <&mut revm_database::states::state::State<statel
     15,014,347  16.17%        770          19,499 <revm_context::context::Context<revm_context::bl
     14,994,327  16.14%        770          19,473 <revm_context::journal::Journal<&mut revm_databa
     14,299,020  15.40%        618          23,137 revm_interpreter::instructions::contract::call_h
     14,253,493  15.35%        618          23,063 revm_interpreter::instructions::contract::call_h
     14,230,009  15.32%        618          23,025 revm_interpreter::instructions::contract::call_h
     13,714,388  14.77%     10,505           1,305 ziskos::zisklib::lib::keccak256::keccak256

```

Shows for each function:
- **STEPS**: Total cumulative cycles used by the function (including all nested calls)
- **% STEPS**: Percentage of total program cycles this function represents
- **CALLS**: Number of times this function was called
- **STEPS/CALL**: Average cycles per call to this function
- **FUNCTION**: Function name from symbol table

**2. TOP COST FUNCTIONS** - Analysis by profiling cost:

```
TOP COST FUNCTIONS (COST, % COST, CALLS, COST/CALL, FUNCTION)
-------------------------------------------------------------
  5,255,204,123  45.95%          1   5,255,204,123 <reth_evm::execute::BasicBlockExecutor<&reth_evm
  5,172,696,823  45.23%          1   5,172,696,823 <alloy_evm::eth::block::EthBlockExecutor<alloy_e
  4,997,989,104  43.70%         70      71,399,844 <revm_handler::mainnet_handler::MainnetHandler<r
  4,530,507,470  39.61%     41,793         108,403 <zeth_mpt::mpt::node::Node<zeth_mpt::mpt::memoiz
  4,014,605,785  35.10%          1   4,014,605,785 <zeth_mpt_state::SparseState as stateless::trie:
  3,759,934,537  32.87%     10,505         357,918 ziskos::zisklib::lib::keccak256::keccak256
```

Shows for each function:
- **COST**: Total cumulative profiling cost of the function (including all nested calls)
- **% COST**: Percentage of total program cost this function represents
- **CALLS**: Number of times this function was called
- **COST/CALL**: Average profiling cost per call to this function
- **FUNCTION**: Function name from symbol table

**Key insights:**

Both tables show **cumulative metrics** - each function includes the cost/cycles of everything it calls. This helps identify:
- Which high-level functions consume the most resources
- Whether optimization should focus on a function's implementation or the functions it calls
- Functions with high cost per call that might benefit from caching or optimization
- Functions called frequently that could benefit from batching or precompiles

By comparing the STEPS and COST analyses, you can identify cases where functions have many cycles but relatively low cost (efficient operations) versus high cost per cycle (expensive operations like precompiles).

For example, `ziskos::zisklib::lib::keccak256::keccak256` shows:
- Called 10,505 times
- 13,714,388 steps (14.77% of total) with ~1,305 steps/call
- 3,759,934,537 cost (32.87% of total) with ~357,918 cost/call

This indicates that while Keccak uses 14.77% of cycles, it represents 32.87% of the total cost - showing it's an expensive operation relative to its cycle count, typical of precompile operations.



## Customizing ROI Display

### Showing More or Fewer Functions

Use the `-T` (or `--top-roi`) flag to control how many top functions are displayed:

```bash
# Show top 50 functions
ziskemu -e target/elf/riscv64ima-zisk-zkvm-elf/release/guest -i input.bin -X -S -T 50

# Show only top 10 functions
ziskemu -e target/elf/riscv64ima-zisk-zkvm-elf/release/guest -i input.bin -X -S -T 10
```

### Specifying the Main Entry Point

If your program's entry point isn't named `main`, use the `-M` (or `--main-name`) flag:

```bash
ziskemu -e target/elf/riscv64ima-zisk-zkvm-elf/release/guest -i input.bin -X -S -M custom_entry
```

### Filtering Functions by Pattern

For large programs, you may want to focus analysis on specific functions or modules. Use the `--roi-filter` flag with a regular expression pattern to mark functions of interest:

```bash
# Filter functions containing "sha256" in their name
ziskemu -e target/elf/riscv64ima-zisk-zkvm-elf/release/guest -i input.bin -X -S --roi-filter "sha256"

# Filter multiple patterns
ziskemu -e target/elf/riscv64ima-zisk-zkvm-elf/release/guest -i input.bin -X -S --roi-filter "hash|crypto|encode"
```

When combined with `--top-roi-filter`, the display will show **only** functions that match the specified pattern:

```bash
# Show only functions matching the filter pattern
ziskemu -e target/elf/riscv64ima-zisk-zkvm-elf/release/guest -i input.bin -X -S \
  --roi-filter "keccak" --top-roi-filter
```

This is useful when you want to:
- Focus optimization efforts on a specific subsystem or module
- Analyze only cryptographic functions
- Compare different implementations of similar functionality
- Filter out noise from unrelated code

## Detailed Caller Analysis

The `-D` (or `--top-roi-detail`) flag provides an **in-depth breakdown** of each top function, showing exactly where costs come from and who calls the function. This detailed analysis helps pinpoint optimization opportunities at a granular level.

### Command

```bash
ziskemu -e target/elf/riscv64ima-zisk-zkvm-elf/release/guest -i input.bin -X -S -D
```

### What This Shows

For each top function, the detailed analysis provides:

1. **Overall metrics**: Total steps and cost for the function
2. **Cost by opcode**: Breakdown showing which operations (opcodes and precompiles) consume the most resources within this function, with ranking of the top 4 most expensive operations
3. **Top step callers**: List of functions that call this function, showing:
   - Number of calls from each caller
   - Total steps attributed to calls from that caller
   - Percentage of this function's total steps coming from each caller

This information helps you understand:
- **What** makes a function expensive (which operations dominate)
- **Who** is responsible for calling it (caller distribution)
- **Where** to focus optimization (expensive operations vs. frequent callers)

### Output Explanation

```
DETAIL FUNCTION ziskos::zisklib::lib::keccak256::keccak256
----------------------------------------------------------
STEPS                         13,714,388  14.77%
COST                       3,759,934,537  32.87%

|    COST BY OPCODE                     COUNT            COST       % RANK
|    ---------------------------------------------------------------------
|    OP ltu                            28,516       1,710,960   0.05%
|    OP add                           169,207       4,230,175   0.11%
|    OP sub                             3,644         218,640   0.01%
|    OP and                            94,545       5,672,700   0.15%
|    OP or                          2,489,249     149,354,940   3.97% #2
|    OP xor                           492,192      29,531,520   0.79% #3
|    OP sll                           360,008      19,080,424   0.51% #4
|    OP dma_memcpy                     21,010         882,420   0.02%
|    OP dma_xmemset                    21,010         882,420   0.02%
|    OP _dma_pre                        2,346         206,448   0.01%
|    OP _dma_post                       9,863         867,944   0.02%
|    OP keccak                         32,650   2,466,707,500  65.61% #1

|    TOP STEP CALLERS (calls, steps)
|    -------------------------------
|              3,974       9,749,694  71.09% <zeth_mpt_state::SparseState as stateless::trie::State
|              2,332       2,778,890  20.26% <zeth_mpt::mpt::node::Node<zeth_mpt::mpt::memoize::Cac
|              1,284         217,150   1.58% revm_interpreter::instructions::system::keccak256::<re
|              1,266         188,634   1.38% <revm_database::states::state::State<stateless::witnes
|                720         107,280   0.78% <alloy_primitives::bits::bloom::Bloom>::accrue_log
|                429          63,921   0.47% <reth_trie_common::hashed_state::HashedPostState>::fro
|                202          30,098   0.22% <revm_database::states::state::State<stateless::witnes
|                144         350,053   2.55% <alloy_trie::hash_builder::HashBuilder>::update
|                 66         102,536   0.75% stateless::recover_block::verify_and_compute_sender
|                 58         110,681   0.81% alloy_primitives::utils::keccak256_impl

```

**Understanding the detailed report:**

**Function Header:**
```
DETAIL FUNCTION ziskos::zisklib::lib::keccak256::keccak256
----------------------------------------------------------
STEPS                         13,714,388  14.77%
COST                       3,759,934,537  32.87%
```
Shows the total cumulative steps and profiling cost for this function (including nested calls).

**COST BY OPCODE section:**
```
|    COST BY OPCODE                     COUNT            COST       % RANK
|    ---------------------------------------------------------------------
|    OP keccak                         32,650   2,466,707,500  65.61% #1
|    OP or                          2,489,249     149,354,940   3.97% #2
|    OP xor                           492,192      29,531,520   0.79% #3
```
Breaks down which operations consume resources within this function:
- **COUNT**: Number of times each operation was executed
- **COST**: Total profiling cost for all executions
- **%**: Percentage of this function's total cost
- **RANK**: Top 4 most expensive operations marked `#1` through `#4`

This shows that `keccak` precompile dominates this function's cost at 65.61%, making it the primary optimization target.

**TOP STEP CALLERS section:**
```
|    TOP STEP CALLERS (calls, steps)
|    -------------------------------
|              3,974       9,749,694  71.09% <zeth_mpt_state::SparseState...
|              2,332       2,778,890  20.26% <zeth_mpt::mpt::node::Node...
```
Shows which functions call this function and how steps are distributed:
- **First column**: Number of calls from this caller
- **Second column**: Total steps consumed when called from this caller
- **Percentage**: How much of this function's total steps come from this caller
- **Function name**: The calling function

This reveals that `SparseState` is responsible for 71% of this function's execution, making it the primary call path to analyze.

### Controlling Detail Level

Use the `-C` (or `--roi-callers`) flag to control how many callers are shown in the detailed analysis for each function:

```bash
# Show top 20 callers for each function in the detailed report
ziskemu -e target/elf/riscv64ima-zisk-zkvm-elf/release/guest -i input.bin -X -S -D -C 20

# Show only top 5 callers for each function
ziskemu -e target/elf/riscv64ima-zisk-zkvm-elf/release/guest -i input.bin -X -S -D -C 5
```

The default value is 10 callers per function. Increasing this number provides more complete call path information but may make the output more verbose.

## Tracking Function Calls

Sometimes you need to analyze **each individual call** to a function to understand:
- Which parameter values are most frequently used
- What patterns exist in the arguments
- Which specific input values trigger expensive code paths

This information is valuable for optimization strategies. For example, if you discover that certain parameter values are very common, you could:
- Add fast paths for those frequent values
- Use lookup tables or caching for common inputs
- Optimize the general case based on typical parameter distributions

### How It Works

Use the `--track-call-args` feature combined with `--roi-filter` to log parameter values for each call to matching functions:

- `--roi-filter "pattern"`: Specifies which functions to track (using a regular expression)
- `--track-call-args N`: Specifies how many parameters to log (up to 8, corresponding to RISC-V a0-a7 registers)

**Important limitation**: The tool logs the **raw parameter values** from registers. This means:
- For scalar values (integers, booleans): You get the actual value
- For pointers/addresses: You get only the address itself, **not** the data it points to
- This makes tracking most useful for functions with scalar parameters or when you're interested in address patterns

### Command

```bash
# Track calls to filtered functions, logging first 4 parameters
ziskemu -e target/elf/riscv64ima-zisk-zkvm-elf/release/guest -i input.bin -S \
  --roi-filter "hash_function" --track-call-args 4 --track-output-path ./traces
```

### Options

- `--roi-filter "pattern"`: Regular expression to match function names you want to track (required)
- `--track-call-args N`: Number of parameters to log (1-8, corresponding to RISC-V a0-a7 registers)
- `--track-separator "SEP"`: Character used to separate parameter values in output (default: `;`)
- `--track-output-path PATH`: Directory where tracking files will be written (default: current directory)

### Output

For each matched function, a text file is created (`<function_name>.txt`) with one line per call:

```
# ROI: hash_function (PC: 0x00012a0-0x00012f8)
# Separator: ';'
# Parameters: a0-a3
0x7fff8200;0x00000100;0x7fff8400;0x00000000
0x7fff8300;0x00000040;0x7fff8400;0x00000001
0x7fff8450;0x00000080;0x7fff8400;0x00000002
```

Each line contains the parameter values (in hexadecimal) for one function call, separated by the chosen separator. You can then analyze this file to:
- Find the most common parameter combinations
- Identify patterns in memory addresses
- Detect outliers or unusual parameter values
- Build histograms of value distributions

## PC Histogram Analysis

The `-H` (or `--histogram`) flag provides a **low-level view** of the most frequently executed code positions in your program. Unlike function-level profiling, this analysis operates at the **program counter (PC)** level, showing you the exact assembly instructions that execute most often.

### What This Shows

This analysis:
- Identifies the most executed individual instructions by their program counter address
- Groups consecutive instructions together automatically
- Attributes these instruction groups to their parent function (when symbols are loaded with `-S`)
- Helps identify hot loops, critical paths, and instruction-level bottlenecks

This is particularly useful for:
- Understanding which specific code sequences dominate execution time
- Identifying tight loops that could benefit from optimization
- Verifying that optimizations are affecting the intended code paths
- Finding unexpected hotspots at the instruction level

### Command

```bash
# Show top 50 most executed instruction groups
ziskemu -e target/elf/riscv64ima-zisk-zkvm-elf/release/guest -i input.bin -X -S -H 50
```

The histogram requires `-S` to display function names. The number after `-H` controls how many instruction groups to display.

### Output Explanation

```
TOP PC HISTOGRAM (EXECUTIONS, % EXECUTIONS, PC)
-----------------------------------------------
        796,670   0.86%  0x801230b8:   lbu r16, 0x0(r14)
        796,670   0.86%  0x801230bc:   beq r16, r12, 0xffffffd4
      1,593,340   1.72%  -----------   <revm_bytecode::legacy::raw::LegacyRawBytecode>::into_analyzed

        755,644   0.81%  0x801230c0:   slli r17, r16, 0x38
        755,644   0.81%  0x801230c4:   srai r17, r17, 0x38
        755,644   0.81%  0x801230c8:   bge r15, r17, 0x14
      2,266,932   2.44%  -----------   <revm_bytecode::legacy::raw::LegacyRawBytecode>::into_analyzed

        547,858   0.59%  0x801230dc:   addi r14, r14, 0x1
        547,858   0.59%  0x801230e0:   bltu r14, r10, 0xffffffd8
      1,095,716   1.18%  -----------   <revm_bytecode::legacy::raw::LegacyRawBytecode>::into_analyzed

        429,174   0.46%  0x800a38ec:   ld r10, 0x60(r21)
        429,174   0.46%  0x800a38f0:   lbu r11, 0x0(r10)
        429,174   0.46%  0x800a38f4:   addi r10, r10, 0x1
        429,174   0.46%  0x800a38f8:   sd r10, 0x60(r21)
        429,174   0.46%  0x800a38fc:   slli r10, r11, 0x4
        429,174   0.46%  0x800a3900:   add r10, r19, r10
        429,174   0.46%  0x800a3904:   ld r11, 0x8(r10)
        429,174   0.46%  0x800a3908:   ld r12, 0x180(r21)
        429,174   0.46%  0x800a390c:   sub r13, r12, r11
        429,174   0.46%  0x800a3910:   sd r13, 0x180(r21)
        429,174   0.46%  0x800a3914:   bltu r12, r11, 0x20
        429,174   0.46%  0x800a3918:   ld r12, 0x0(r10)
        429,174   0.46%  0x800a391c:   addi r10, r21, 0x0 => copyb
        429,174   0.46%  0x800a3920:   addi r11, r9, 0x0 => copyb
        429,174   0.46%  0x800a3924:   jalr r1, r12, 0x0
        429,174   0.46%  0x800a3928:   lbu r10, 0x68(r21)
        429,174   0.46%  0x800a392c:   bne r10, r0, 0xffffffc0
      7,295,958   7.86%  -----------   <revm_handler::mainnet_handler::MainnetHandler<revm_context::evm::Ev
```

**Understanding the histogram:**

The output is organized into **instruction groups**, where each group consists of:

1. **Individual instruction lines**: Each shows:
   - **EXECUTIONS**: Number of times this specific instruction was executed
   - **% EXECUTIONS**: Percentage of total program steps
   - **PC**: Program counter address in hexadecimal
   - **Instruction**: The RISC-V assembly instruction at that address

2. **Group summary line** (with dashes):
   - **Total executions**: Sum of all instructions in this group
   - **% EXECUTIONS**: Cumulative percentage for the entire group
   - **Function name**: The function to which these instructions belong

**Key insights from the example:**

The first group shows a simple loop checking bytes:
```
        796,670   0.86%  0x801230b8:   lbu r16, 0x0(r14)     # Load byte
        796,670   0.86%  0x801230bc:   beq r16, r12, 0xffffffd4  # Branch if equal
      1,593,340   1.72%  -----------   <revm_bytecode::legacy::raw::LegacyRawBytecode>::into_analyzed
```
This tight 2-instruction sequence executed 796,670 times, representing 1.72% of total execution.

The large group at the bottom represents a complex instruction dispatcher:
```
        429,174   0.46%  0x800a38ec:   ld r10, 0x60(r21)     # Load from context
        ...
        429,174   0.46%  0x800a392c:   bne r10, r0, 0xffffffc0   # Loop back
      7,295,958   7.86%  -----------   <revm_handler::mainnet_handler::MainnetHandler...
```
This 17-instruction sequence accounts for 7.86% of total execution, making it a prime optimization target.

**When to use histogram analysis:**

- **After function-level profiling**: Once you identify expensive functions, use histograms to see which specific instruction sequences within those functions dominate
- **Validating compiler optimizations**: Verify that loops are unrolled or optimized as expected
- **Finding unexpected hotspots**: Sometimes a small instruction sequence accounts for disproportionate execution time
- **Comparing implementations**: See how different code structures affect instruction-level execution patterns

## Additional Options

### Show Steps Without Full Statistics

For quick execution time checks without generating full statistics, use the `--steps` flag:

```bash
ziskemu -e target/elf/riscv64ima-zisk-zkvm-elf/release/guest -i input.bin --steps
```

### Progress Indicators

For long-running programs, show progress updates every 16M steps with `--with-progress`:

```bash
ziskemu -e target/elf/riscv64ima-zisk-zkvm-elf/release/guest -i input.bin --with-progress
```

### Disable Thousands Separator

For machine-readable output, disable the thousands separator with `--no-thousands-sep`:

```bash
ziskemu -e target/elf/riscv64ima-zisk-zkvm-elf/release/guest -i input.bin -X --no-thousands-sep
```

## Complete Example: Comprehensive Profiling

Here's a complete example that uses most profiling features together:

```bash
ziskemu -e target/elf/riscv64ima-zisk-zkvm-elf/release/guest \
  -i input.bin \
  -X \
  -S \
  -D \
  -T 30 \
  -C 15 \
  -H 50 \
  --roi-filter "sha256|hash" \
  --track-call-args 6 \
  --track-output-path ./profiling_data \
  -m
```

This command will:
1. Generate full statistics (`-X`)
2. Read and use symbol information (`-S`)
3. Show detailed caller analysis (`-D`)
4. Display top 30 functions by cost (`-T 30`)
5. Show top 15 callers for each function (`-C 15`)
6. Display top 50 most executed instructions (`-H 50`)
7. Filter to sha256/hash-related functions (`--roi-filter`)
8. Track first 6 parameters of filtered function calls (`--track-call-args`)
9. Save tracking data to ./profiling_data directory
10. Show performance metrics (`-m`)

## Tips for Effective Profiling

### Start Simple, Add Detail

Begin with basic statistics (`-X`) to get an overview, then progressively add more detailed analysis:

1. Basic: `ziskemu -e program.elf -i input.bin -X`
2. Functions: `ziskemu -e program.elf -i input.bin -X -S`
3. Callers: `ziskemu -e program.elf -i input.bin -X -S -D`
4. Detailed: Add `-H` as needed

### Focus on High Impact

Use the final_cost percentage to identify functions with the highest impact. Optimizing a function that represents 50% of execution time will have much more effect than optimizing one at 1%.

### Understand Profiling Cost vs. Final Cost

When a function has high final cost but low profiling cost, the optimization opportunity lies in the functions it calls, not in the function itself. Focus your optimization efforts where profiling costs are highest, as these represent direct computational work that can be improved through code changes or patching with precompiles.

### Use Filtering for Large Codebases

In programs with hundreds of functions, use `--roi-filter` to focus on specific subsystems or modules of interest.

### Track Representative Inputs

Profile with realistic, representative inputs. The cost distribution can vary significantly based on input characteristics.

## Practical Example: Analyzing Ethereum Opcode Costs

This example demonstrates how to analyze the cost distribution of Ethereum opcodes in a real-world client implementation. By filtering for the EVM instruction interpreter functions, we can obtain a detailed breakdown of which Ethereum operations consume the most resources during block validation.

### Scenario

You want to understand which Ethereum opcodes are most expensive in terms of ZisK proving costs when validating a specific block. This information helps you:
- Identify which EVM operations would benefit most from optimization
- Understand the cost profile of real-world Ethereum transactions
- Guide decisions about which precompiles or patches to prioritize

### Command

```bash
target/release/ziskemu \
  -S \
  -X \
  -e ../zisk-eth-client/bin/guests/stateless-validator-reth/target/riscv64ima-zisk-zkvm-elf/release/zec-reth \
  -i ../data/benchmark_inputs/24654304_30c8b8.bin \
  --roi-filter "revm_interpreter::instructions::" \
  --top-roi-filter \
  -T 200
```

**What this does:**

- `-S`: Load symbol information from the ELF file
- `-X`: Generate full statistics with cost breakdown
- `-e <path>`: Path to the compiled Ethereum client (reth implementation)
- `-i <input>`: Block data to validate (block 24,654,304)
- `--roi-filter "revm_interpreter::instructions::"`: Filter to show only functions in the EVM instruction interpreter namespace (where all Ethereum opcodes are implemented)
- `--top-roi-filter`: Display only the filtered functions in the top ROI lists
- `-T 200`: Show top 200 functions (to capture all EVM opcodes)

### Expected Output

The output will show the TOP COST FUNCTIONS filtered to only include EVM instruction implementations, giving you a clear view of which Ethereum opcodes dominate the proving cost for this specific block:

```
TOP COST FUNCTIONS (COST, % COST, CALLS, COST/CALL, FUNCTION)
-------------------------------------------------------------
  9,433,353,231  10.32%      5,824       1,619,737 revm_interpreter::instructions::contract::call_helpers::load_acc_
  9,396,093,086  10.28%      5,824       1,613,340 revm_interpreter::instructions::contract::call_helpers::load_acco
  9,377,741,662  10.26%      5,824       1,610,189 revm_interpreter::instructions::contract::call_helpers::load_acco
  8,344,978,788   9.13%      1,695       4,923,291 revm_interpreter::instructions::contract::call::<revm_interpreter
  4,599,658,812   5.03%    342,951          13,412 revm_interpreter::instructions::stack::swap::<1, revm_interpreter
  2,772,734,752   3.03%    128,956          21,501 revm_interpreter::instructions::memory::mload::<revm_interpreter:
  2,580,388,569   2.82%     10,675         241,722 revm_interpreter::instructions::host::sload::<revm_interpreter::i
  1,726,257,923   1.89%    105,903          16,300 revm_interpreter::instructions::memory::mstore::<revm_interpreter
  1,599,904,068   1.75%    119,289          13,412 revm_interpreter::instructions::stack::swap::<2, revm_interpreter
  1,576,416,043   1.72%     13,627         115,683 revm_interpreter::instructions::arithmetic::mulmod::<revm_interpr
  1,499,796,900   1.64%    111,825          13,412 revm_interpreter::instructions::stack::swap::<3, revm_interpreter
  1,430,041,088   1.56%    106,624          13,412 revm_interpreter::instructions::stack::swap::<4, revm_interpreter
  1,045,628,445   1.14%      2,201         475,069 revm_interpreter::instructions::contract::static_call::<revm_inte
    896,353,301   0.98%    184,312           4,863 revm_interpreter::instructions::control::jumpi::<revm_interpreter
    812,869,552   0.89%    561,374           1,448 revm_interpreter::instructions::stack::push::<1, revm_interpreter
    806,652,474   0.88%    465,922           1,731 revm_interpreter::instructions::stack::push::<2, revm_interpreter
    763,874,190   0.84%      6,781         112,649 revm_interpreter::instructions::host::sstore::<revm_interpreter::
    691,435,073   0.76%      5,682         121,688 revm_interpreter::instructions::system::keccak256::<revm_interpre
    669,514,638   0.73%    245,798           2,723 revm_interpreter::instructions::arithmetic::add::<revm_interprete
    638,632,995   0.70%    102,549           6,227 revm_interpreter::instructions::arithmetic::mul::<revm_interprete
    620,675,903   0.68%    239,701           2,589 revm_interpreter::instructions::control::jump::<revm_interpreter:
    527,546,726   0.58%     83,391           6,326 revm_interpreter::instructions::bitwise::shr::<revm_interpreter::
    452,376,936   0.49%    302,391           1,496 revm_interpreter::instructions::stack::dup::<2, revm_interpreter:
    325,487,994   0.36%     41,683           7,808 revm_interpreter::instructions::bitwise::sar::<revm_interpreter::
    311,851,955   0.34%     25,502          12,228 revm_interpreter::instructions::system::codecopy::<revm_interpret
    289,141,110   0.32%    120,407           2,401 revm_interpreter::instructions::bitwise::iszero::<revm_interprete
    264,613,976   0.29%    176,881           1,496 revm_interpreter::instructions::stack::dup::<3, revm_interpreter:
    262,969,735   0.29%     18,608          14,132 revm_interpreter::instructions::system::calldataload::<revm_inter
    252,430,047   0.28%     41,031           6,152 revm_interpreter::instructions::bitwise::sgt::<revm_interpreter::
    248,940,076   0.27%      1,928         129,118 revm_interpreter::instructions::contract::delegate_call::<revm_in
    242,086,315   0.26%        192       1,260,866 revm_interpreter::instructions::host::extcodesize::<revm_interpre
    229,785,355   0.25%     10,852          21,174 revm_interpreter::instructions::stack::push::<32, revm_interprete

```

This filtered view allows you to quickly identify:
- **Most expensive opcodes**: Which EVM operations have the highest total cost
- **Frequently called opcodes**: Operations with many calls but lower individual cost
- **Optimization targets**: Opcodes that would benefit most from ZisK-specific optimizations or precompiles


**Important note**: With this method, **no modification to the ELF file is required**. The profiling works directly on the compiled binary using existing symbol information. However, you do need to know the naming convention used for the functions that implement each opcode. In this case, the REVM interpreter uses the namespace `revm_interpreter::instructions::` consistently, making it easy to filter all opcode implementations with a single pattern.

## Conclusion

ZiskEmu's profiling capabilities provide deep insights into your program's resource consumption and performance characteristics. By understanding profiling and final costs, analyzing regions of interest, and using the various filtering and tracking options, you can effectively identify optimization opportunities and improve the efficiency of your ZisK programs.

Use profiling costs as your primary optimization metric, as they provide a direct cause-and-effect relationship with code changes. This makes them ideal for detecting where patches should be applied, validating that optimizations are working correctly, and ensuring that precompiles are being used where expected.

Remember that profiling works on any ELF file with symbols, including release builds, making it easy to analyze production-ready code without special compilation flags or instrumentation.
