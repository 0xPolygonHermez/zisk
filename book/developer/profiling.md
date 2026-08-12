# Profiling Programs with ZiskEmu

ZiskEmu provides powerful profiling capabilities to analyze the cost and performance characteristics of your programs. This guide explains how to use these features to identify hotspots, optimize your code, and understand resource consumption.

## What This Guide Covers

This guide walks you through ZiskEmu's profiling capabilities, progressing from high-level overviews to detailed analysis:

1. **Introduction**: Understanding profiling costs vs. final costs, symbol-based analysis, and detecting optimization opportunities

2. **Basic Profiling**: Global statistics showing cost distribution across major categories (base, main, opcodes, precompiles, memory)

3. **Opcode Variant Breakdown**: Splitting an opcode into the cheaper-to-prove shapes its operands actually take

4. **Operand Pattern Analysis**: Finding operand shapes common enough to justify a cheaper state machine

5. **Precompile Duplicate Analysis**: Detecting precompile calls that repeat a computation already done, and where they come from

6. **Memory Statistics**: Breaking the memory cost down by region, alignment and access shape, and ranking functions by memory traffic

7. **Register Step Distance**: Measuring how long registers keep a value between accesses, against the proof's distance limit

8. **Opcode Coverage**: Which opcodes, precompiles and RISC-V instructions the run exercised

9. **Comparing Runs**: Saving a statistics snapshot and diffing a later run against it to validate an optimization

10. **HTML Report**: Rendering a run, or the comparison of two runs, as a standalone shareable page

11. **SDK Report Mode**: Streamlined, compact output format ideal for CI/CD and quick checks, with selective section display options

12. **Function Name Display Options**: Configure how long function names are displayed with compact and no-compact modes

13. **Profile Tags**: Instrument your code to measure specific sections, with immediate or deferred reporting of steps and costs

14. **Firefox Profiler Integration**: Export profiling data for advanced visualization and interactive analysis

15. **Function-Level Profiling**: Identifying which functions consume the most resources with cumulative analysis

16. **Customizing ROI Display**: Controlling how many functions to show and filtering by patterns

17. **Detailed Caller Analysis**: In-depth breakdown showing which operations are expensive within each function and who calls them

18. **Tracking Function Calls**: Logging individual call parameters to analyze usage patterns and optimize for common cases

19. **PC Histogram Analysis**: Low-level view of the most frequently executed RISC-V instruction sequences

20. **Instruction Tracing and Disassembly**: Step-by-step traces, change traces and an annotated disassembly with execution counts

21. **Additional Options**: Quick reference for other useful flags (steps, progress indicators, formatting)

22. **Practical Example**: Real-world case study analyzing Ethereum opcode costs in a block validator

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
STEPS                          3,159,193

COST DISTRIBUTION                   COST       %
------------------------------------------------
MAIN                         214,825,124  33.99%
OPCODES                      110,764,355  17.52%
PRECOMPILES                      836,980   0.13%
MEMORY                        18,327,393   2.90%
                        ------------------------
VARIABLE                     344,753,852  54.54%
BASE                         287,309,824  45.46%
                        ------------------------
TOTAL                        632,063,676 100.00%

FROPS                          7,289,417   2.11%
RAM USAGE                          1,120   0.00%
ROM USAGE                         14,114   0.34%

```

**Understanding the Report:**

**STEPS**: The number of processor cycles or instructions executed during program execution. This is an indicator of how long the program is—more steps mean a longer program execution.

**COST DISTRIBUTION**: This shows the **profiling cost** (see the [Understanding Profiling Costs](#understanding-profiling-costs-vs-final-costs) section for detailed explanation). Each operation is costed individually using the proof area as the metric, which is the best indicator of proof generation time—higher cost means longer proof generation.

The cost is broken down into these categories:

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

- **VARIABLE**: Sum of MAIN + OPCODES + PRECOMPILES + MEMORY, i.e. **everything the program controls**. This is the number to watch when optimizing: it is the only part that moves when you change your code.

- **BASE**: Cost of fixed components such as tables, range checks, and other constant overhead that exists regardless of program logic. It is the same for every program, so it dilutes percentages: in the example above, a small program pays 287M of BASE against only 344M of VARIABLE, which is why BASE is 45% of the total. On a large program the same 287M becomes a rounding error.

- **TOTAL**: VARIABLE + BASE. Each category shows the percentage (%) it represents of this total.

**Why VARIABLE matters**: because BASE is a constant, comparing two runs by TOTAL understates your improvement. If an optimization removes 60M of cost, VARIABLE drops by a visible fraction while TOTAL barely moves. All the percentages *below* the cost distribution (FROPS, the per-opcode tables, TOP COST FUNCTIONS) are therefore expressed as a share of **VARIABLE**, not of TOTAL.

**FROPS** (FRequent OPerationS): These are operations that are very frequently used by the processor, such as:
- Adding 1 to a relatively small number (common in loop counters)
- Adding 8 to an address (typical for pointer arithmetic)
- Working with values < 256

These frequent operations are analyzed, detected, and **pre-calculated**, becoming part of the BASE cost but representing significant savings. In this example, FROPS show 2.11% of the variable cost - this is the cost the program would have if these optimizations were not applied. The actual savings are already reflected in the lower costs of the affected operations.

**RAM USAGE**: The amount of memory used out of the total available. This information is **only available with the default allocator (bump allocator)**, which:
- Never frees memory - always allocates new memory
- Avoids the CPU cycles needed to manage the entire heap (typically >10% overhead)
- Is recommended as long as sufficient memory is available
- Provides better performance by eliminating heap management costs

**ROM USAGE**: How much of the ROM instance capacity is being used out of the total available. The ROM
holds two things that consume rows:
- the **program instructions** (one row each), and
- the **ROM/RAM initialization** operations, which are packed **4 per row** (hence the count is divided
  by 4 when computing usage).

A high ROM usage means the program (plus its initialization data) is close to filling the ROM instance.

### Detailed Opcode Breakdown

Below the summary you get three tables: the base (ALU) opcodes, the precompiled opcodes, and the
FROPS coverage. They are **sorted by cost, most expensive first** — the first rows are always the
ones worth looking at. Pass `--sort-by-units` to sort them by operation count instead, which is
what you want when you care about *how often* something runs rather than what it costs.

```
COST BY BASE OPCODE                COUNT       %            COST       %
------------------------------------------------------------------------
OP xor                           561,981  17.79%      33,718,860   9.78%
OP ror_w                         561,974  17.79%      31,470,544   9.13%
OP add                           674,936  21.36%      15,889,840   4.61%
OP and                           194,544   6.16%      11,672,640   3.39%
OP or                             82,038   2.60%       4,922,280   1.43%
OP srl_w                          78,992   2.50%       4,423,552   1.28%
OP andn                           64,000   2.03%       3,840,000   1.11%
OP srl                            28,088   0.89%       1,572,928   0.46%
OP rev8                           16,993   0.54%         951,608   0.28%
OP signextend_w                   16,007   0.51%         896,392   0.26%
OP signextend_b                   16,000   0.51%         896,000   0.26%
OP sll                             4,205   0.13%         235,480   0.07%
OP eq                              2,132   0.07%         127,920   0.04%
OP mul                               202   0.01%          19,594   0.01%
OP pubout                             32   0.00%               0   0.00%

COST BY PRECOMPILED OPCODE           COUNT       %            COST       %
--------------------------------------------------------------------------
OP dma_xmemcpy                     3,002   0.10%         406,226   0.12%
OP dma_xmemset                     2,003   0.06%         400,496   0.12%
OP dma_memcpy                        204   0.01%          30,258   0.01%

FROPS BY OPCODE                    COUNT    HIT            COST       %
-----------------------------------------------------------------------
FROP sll                          31,295  88.15%       1,752,520   0.51%
FROP xor                          16,027   2.77%         961,620   0.28%
FROP or                           15,046  15.50%         902,760   0.26%
FROP pack_h                       16,031 100.00%         897,736   0.26%
FROP ror_w                        14,026   2.44%         785,456   0.23%
FROP srl                          13,213  31.99%         739,928   0.21%
FROP rev8                          8,009  32.03%         448,504   0.13%
FROP eq                            5,837  73.25%         350,220   0.10%
FROP ltu                           3,367  99.79%         202,020   0.06%
FROP add                           6,790   1.00%         169,750   0.05%

```

**COST BY BASE OPCODE / COST BY PRECOMPILED OPCODE:**

The two tables have the same columns and are split so the cheap, high-volume ALU work does not hide
the handful of precompile calls that often dominate the cost:

- **COUNT**: Number of times this operation was executed
- **%**: Percentage of steps (cycles) that use this operation
- **COST**: Total profiling cost for all executions of this operation
- **%**: Percentage of the **variable** cost that this operation represents

Counts here exclude the executions that were resolved as FROPS — those are reported separately in
the FROPS table, so the two never double-count.

**Looking at operand shapes**: ZiskEmu also classifies the shape of each operation's `a`, `b` and
`c` operands (do they fit in 32 bits? is one of them all ones?), because an operation whose operands
always have a given shape can be proven by a narrower, cheaper state machine. That information is
*not* in this table — it has its own section, [`--pattern-analysis`](#operand-pattern-analysis),
where it is named, filtered and grouped per opcode. See
[Opcode Variant Breakdown](#opcode-variant-breakdown) for the shapes that are already exploited.

**FROPS BY OPCODE Table:**

FROPS (Frequently-used OPerationS) are highly common operations that have been analyzed and optimized through pre-calculation. These include operations like:
- Incrementing by 1 (loop counters)
- Adding 8 (pointer arithmetic)
- Working with small values (< 256)

The table shows:

- **COUNT**: Number of times the FROP variant was executed
- **HIT**: Hit rate percentage - how often the frequent operation pattern was matched and the optimization applied, i.e. `FROP count / (FROP count + non-FROP count)` for that opcode
- **COST**: Total cost with the optimization benefit already applied
- **%**: Percentage of the variable cost

High hit rates indicate that the program uses these common patterns frequently, benefiting from the pre-calculated optimizations. The FROPS total shown earlier (2.11% in this example) represents the cost that would be added if these optimizations were not available.

Use `--legacy-frops` to recompute the FROPS coverage against the **previous** FROPS tables (the
snapshot taken before the FROPS overhaul). That lets you measure what a new FROPS version actually
bought you on a real program, by running the same ELF twice and comparing.

**Key Insights from Statistics:**

Use this information to:
- Identify which operation types dominate your program's cost
- Find operations with high count but disproportionate cost (optimization candidates)
- Verify that precompiles are being used where expected
- Understand the balance between computation (OPCODES), memory access (MEMORY), and complex operations (PRECOMPILES)

## Opcode Variant Breakdown

Some opcodes have **variants that are cheaper to prove** when their operands take a particular
shape. ZisK already charges the reduced cost for them automatically; `--opcode-breakdown` shows you
how many of an opcode's executions took each cheap shape, as a tree under the opcode:

```bash
ziskemu -e <elf> -i <input> -X --opcode-breakdown
```

```
COST BY BASE OPCODE                COUNT       %            COST       %
------------------------------------------------------------------------
OP add                           674,936  21.36%      15,889,840   4.61%
├ add_hi0                         64,717   2.05%         970,755   0.28%
└ add_hif                         33,639   1.06%         504,585   0.15%
```

Currently the tracked variants are the **BinaryAddHi** shapes of `add`, which are proven by a
dedicated, much cheaper state machine:

- **`add_hi0`**: `hi32(a) = hi32(c) = 0` and `hi32(b) = 0` — both operands fit in 32 bits
- **`add_hif`**: `hi32(a) = hi32(c) = 0` and `hi32(b) = 0xFFFF_FFFF` — a subtraction encoded as an addition

The variant rows are a **subset** of the opcode row above them: their counts are already included in
the opcode's COUNT, and their reduced cost is already reflected in the opcode's COST. A high share of
variant rows means the opcode is cheaper than its nominal cost suggests.

## Operand Pattern Analysis

`--pattern-analysis` answers the question the raw operand-category columns are too noisy to answer:
**is there an operand shape common enough in this program to justify a cheaper state machine?**

```bash
ziskemu -e <elf> -i <input> -X --pattern-analysis
```

```
OPERAND PATTERN ANALYSIS (>10% of opcode ops and >=4,000)
PATTERN                                      COUNT        %
-----------------------------------------------------------
OP xor                                     561,981  100.00%
  a,b,c hi32=0                             136,409   24.27%
  a hi32=1,b hi32=0                        130,981   23.31%
  a,b hi32=1                                85,565   15.23%
  a hi32=0,b hi32=1                         85,543   15.22%
OP ror_w                                   561,974  100.00%
  b<64                                     561,974  100.00%
OP or                                       82,038  100.00%
  a,b,c hi32=0                              13,522   16.48%
OP srl_w                                    78,992  100.00%
  b<64                                      78,992  100.00%
  a,b,c hi32=0                              27,749   35.13%
OP rev8                                     16,993  100.00%
  b hi32=1                                   7,951   46.79%
OP signextend_b                             16,000  100.00%
  b,c hi32=0                                12,086   75.54%
  b<64                                      10,030   62.69%
```

**How to read it:**

- Rows are **grouped by opcode**. The `OP <name>` row is the group header: it carries the opcode's
  total operations, which is the 100% every pattern below it is measured against.
- The indented rows are the operand shapes that opcode's operands actually took, ordered by count.
- Groups are ordered like the opcode tables — by cost, or by count with `--sort-by-units`.

**What gets reported.** Only shapes that are **significant**: more than **10%** of that opcode's
operations *and* at least **4,000** operations in absolute terms. A shape that appears twice tells
you nothing about where to spend engineering effort.

**What gets filtered out**, so the signal is not buried:

- **Non-ALU opcodes.** Only arithmetic and binary opcodes are analyzed. The operand shape of an
  `fcall`, a `pubout` or a precompile says nothing about how it is proven.
- **Patterns that are true by definition rather than by data.** Single-operand opcodes (`rev8`,
  `brev8`, `signextend_b/h/w`, `clz`, `ctz`, `cpop`, `orc_b` and their `_w` variants) ignore `a`
  entirely — it is always zero. Any condition on `a` therefore holds 100% of the time and means
  nothing: reporting `rev8 a<64  100.00%` would be pure noise. For these opcodes the conditions on
  `a` are dropped from the label (`a,b,c hi32=0` is reported as `b,c hi32=0`) and the patterns that
  say nothing else are not reported at all.

**Using the result.** A shape at or near 100% is the strongest possible finding: it means *every*
execution of that opcode in this program has that property, so a specialized state machine would
apply universally. In the example, all 561,974 `ror_w` operations have `b<64`, and 75% of the
`signextend_b` ones have both operand and result inside 32 bits. Compare it against
[`--opcode-breakdown`](#opcode-variant-breakdown) to see which shapes are *already* exploited.

## Precompile Duplicate Analysis

Precompiles are the most expensive operations in a program, and real programs call them repeatedly
with **the same input**. Since a precompile is a pure function, every repeat recomputes a result the
proof has already paid for. `--duplicates` measures exactly how much that costs you:

```bash
ziskemu -e <elf> -i <input> -X --duplicates
```

The analysis keys on the **content** of the operands, dereferencing indirections, so two calls with
identical inputs stored in different buffers are still recognized as duplicates. It covers every
precompile except DMA.

```
PRECOMPILE DUPLICATES
PRECOMPILE                    TOTAL       UNIQUE        %          DUP        %   MAX DUP        DUP COST        %
------------------------------------------------------------------------------------------------------------------
keccak                        1,000           12    1.20%          988   98.80%       200      74,668,100  18.96%
```

- **TOTAL**: Calls to this precompile
- **UNIQUE**: Distinct inputs among them
- **DUP**: Calls that repeated an input already seen (`TOTAL - UNIQUE`), with its share of TOTAL
- **MAX DUP**: How many times the single most-repeated input was computed
- **DUP COST**: The cost spent on those repeats — **the saving available if you cache them** — and its share of the total cost

A `TOTAL` row is added when more than one precompile is reported.

### Restricting the analysis

Tracking input content costs memory and time. Limit it to the precompiles you care about with
`--duplicates-ops`, comma-separated by opcode name:

```bash
ziskemu -e <elf> -i <input> -X --duplicates --duplicates-ops keccak,sha256
```

### Finding where the duplicates come from

Knowing that 98.8% of your keccak calls are redundant is only useful if you can find the call site.
`--duplicates-detail` adds a second level showing the call paths responsible, most costly first.
It needs symbols (`-S`):

```bash
ziskemu -e <elf> -i <input> -X -S --duplicates --duplicates-detail --duplicates-depth 3
```

```
KECCAK DUPLICATES BY CALL PATH
------------------------------
796 duplicates / 800 total  (99.50%)
    ziskos::zisklib::lib::keccak256::keccak256
    <- dupguest::hash_block
    <- main
192 duplicates / 200 total  (96.00%)
    ziskos::zisklib::lib::keccak256::keccak256
    <- dupguest::hash_leaf
    <- main
```

Each entry is one call path: the leaf function that issued the precompile, then its callers with
`<-`. `--duplicates-depth N` sets how many frames are recorded (default 4, which is also the
maximum useful depth for most code); `-T` limits how many paths are listed per precompile.

This immediately tells you *which* call site to fix — here, `hash_block` recomputes the same four
hashes on every round of the loop, so hoisting them out removes 79% of the precompile cost. See
[Comparing Runs](#comparing-runs) for how to confirm the saving.

## Memory Statistics

The `MEMORY` line in the cost distribution can be broken down in detail with two **opt-in** flags.
They are **off by default**: a plain `-X` report does not print the memory sections. Each requires
stats (`-X`).

| Flag | Section it adds | Content |
|------|-----------------|---------|
| `--mem-stats` | **MEM COST BY TYPE** | Memory cost aggregated by category (region × alignment) plus totals. |
| `--mem-full-stats` | **DETAILED MEM COST** | Per-operation breakdown (also implies **MEM COST BY TYPE**, so both sections are shown). |

```bash
# By-type memory section only
ziskemu -e <elf> -i <input> -X --mem-stats

# Detailed per-operation section (also prints the by-type section)
ziskemu -e <elf> -i <input> -X --mem-full-stats
```

### Key concepts

Before reading the tables, a few definitions:

- **Aligned**: an access is *aligned* only when it reads/writes exactly **8 bytes** and its address is a
  **multiple of 8**. This is the natural, cheapest memory access (one memory row, no alignment state
  machine). Every other access (1/2/4 bytes, or 8 bytes at a non-8-aligned address) is **unaligned** and
  costs more.

- **Single vs. double**: an unaligned access is **single** when it touches a **single** memory address
  (row) and **double** when it spans **two** consecutive rows (it crosses an 8-byte boundary). A double
  access is more expensive because it reads/writes two rows instead of one. An unaligned 8-byte access is
  always **double**; 2-byte and 4-byte accesses are single or double depending on where they fall within
  the 8-byte word.

- **Region**: memory is split into **RAM STACK** (the stack area of RAM), **RAM NO STACK** (the rest of
  RAM), **ROM** (read-only data next to the program) and **INPUT** (the program input).

- **INIT**: `ROM INIT` / `RAM INIT` are the **aligned initialization** operations that set up the initial
  ROM and RAM contents (the initial data image). They are *not* the `.bss` (zero-initialized memory,
  which needs no operations); they are the memory operations required to lay down the initial values.

### MEM COST BY TYPE (`--mem-stats`)

```
MEM COST BY TYPE                   COUNT       %            COST       %
------------------------------------------------------------------------
RAM STACK ALIGNED                383,482  92.38%       6,533,298  79.77%
RAM NO STACK ALIGNED              17,568   4.23%         298,088   3.64%
ROM ALIGNED                        1,093   0.26%          15,302   0.19%
ROM INIT                           3,736   0.90%          52,304   0.64%
INPUT ALIGNED                         31   0.01%             899   0.01%
RAM STACK UNALIGNED                8,524   2.05%       1,250,979  15.27%
RAM NO STACK UNALIGNED               381   0.09%          27,103   0.33%
ROM UNALIGNED                        319   0.08%          12,522   0.15%
                         -----------------------------------------------
TOTAL ALIGNED                    405,910  97.78%       6,899,891  84.24%
TOTAL UNALIGNED                    9,224   2.22%       1,290,604  15.76%
                         -----------------------------------------------
TOTAL RAM STACK                  392,006  94.43%       7,784,277  95.04%
TOTAL RAM NO STACK                17,949   4.32%         325,191   3.97%
                         -----------------------------------------------
TOTAL RAM                        409,955  98.75%       8,109,468  99.01%
TOTAL ROM                          5,148   1.24%          80,128   0.98%
TOTAL INPUT                           31   0.01%             899   0.01%
```

Each row shows the **COUNT** of accesses and their **COST**, with the percentage each represents of the
total memory cost. The per-category rows are grouped by region and alignment; the `TOTAL …` rows roll
them up along different axes (aligned vs. unaligned, per region, per RAM sub-area). Unaligned accesses
are typically a small fraction of the count but a large fraction of the cost — in the example above they
are 2.22% of accesses but 15.76% of the memory cost.

### DETAILED MEM COST (`--mem-full-stats`)

The detailed section expands every category into the exact access shape, so you can see which specific
patterns dominate:

```
DETAILED MEM COST                                  COUNT       %            COST       %
----------------------------------------------------------------------------------------
RAM STACK aligned 8B read                        184,689  44.89%       2,955,024  36.31%
RAM STACK unaligned 4B single read                 1,694   0.41%         206,668   2.54%
RAM STACK aligned 8B write                       198,793  48.32%       3,578,274  43.97%
RAM STACK unaligned 1B clean write                 2,052   0.50%         135,432   1.66%
RAM STACK unaligned 4B single write                4,674   1.14%         902,082  11.08%
ROM aligned 8B read                                4,829   1.17%          67,606   0.83%
INPUT aligned 8B read                                 31   0.01%             899   0.01%
                                         -----------------------------------------------
TOTAL aligned 8B                                 405,910  98.67%       6,899,891  84.78%
TOTAL unaligned 4B single                          6,394   1.55%       1,113,127  13.68%
                                         -----------------------------------------------
TOTAL reads                                      200,754  48.80%       3,393,944  41.70%
TOTAL writes                                     214,380  52.11%       4,796,551  58.94%
```

Each line names the **region**, **alignment**, **width** (`1B`/`2B`/`4B`/`8B`), **single/double** and
**read/write**. For 1-byte writes you may also see **clean** vs **dirty**: a *clean* write targets a byte
whose surrounding word did not need to be read first, while a *dirty* write requires reading the word
before updating the byte (more expensive). The `TOTAL …` rows at the bottom summarize by access shape and
by read vs. write, which makes it easy to spot, for example, that unaligned 4-byte writes are cheap in
count but expensive in cost.

### DETAILED OFFSET BYTE MEMORY OPERATIONS (`--mem-full-stats`)

Byte accesses are also broken down by their **offset inside the 8-byte word**:

```
DETAILED OFFSET BYTE MEMORY OPERATIONS
--------------------------------------
offset                    0            1            2            3            4            5            6            7        total
reads                10,255        8,249        8,237        8,229        8,240        8,226        8,171        8,244       67,851
clean writes          2,585          105            8            8           16           36           46          107        2,911
dirty writes              0            0            0            0            2            2            2            0            6
```

A distribution concentrated on offset 0 means the byte accesses are at least word-aligned; a flat
distribution (as above, for the reads) means the code is walking a byte buffer and paying the
unaligned cost on every step — a candidate for reading whole words instead. **Dirty writes** are the
expensive ones: they force a read of the surrounding word before updating the byte.

### Ranking functions by memory cost

When symbols are available (`-S`), `--mem-stats` / `--mem-full-stats` also add three rankings that
attribute the memory cost to functions. Costs are shown in millions (marked `(M)` in the header);
the SDK report keeps raw values.

```bash
ziskemu -e <elf> -i <input> -X -S --mem-full-stats
```

**TOP MEMORY COST FUNCTIONS** — who spends the most on memory, in absolute terms:

```
TOP MEMORY COST FUNCTIONS (MEM COST (M), % MEM COST, CALLS, COST/CALL (M), FUNCTION)
------------------------------------------------------------------------------------
       18.30  99.99%          1        18.30 guest::__zisk_entry
       14.16  77.38%      1,000         0.01 sha2::sha256::compress256
        0.45   2.47%          3         0.15 std::io::stdio::_print
```

**TOP UNALIGNED MEMORY FUNCTIONS** — the same ranking restricted to the *unaligned* cost, with the
aligned cost alongside so you can see the ratio:

```
TOP UNALIGNED MEMORY FUNCTIONS (UNALIGNED (M), ALIGNED (M), % UNALIGNED, CALLS, FUNCTION)
-----------------------------------------------------------------------------------------
        8.06        10.24  44.05%          1 _zisk_main
        5.27         8.89  37.19%      1,000 sha2::sha256::compress256
        0.05         0.00  99.10%          3 <std::io::buffered::bufwriter::BufWriter<…>>::flush_buf
```

**TOP UNALIGNED/STEP RATIO FUNCTIONS** — the most useful of the three for optimization. It ranks
functions by how far their unaligned cost *per step* exceeds the program's average, so a small
function doing pathologically unaligned work rises to the top instead of being hidden behind the
big ones:

```
TOP UNALIGNED/STEP RATIO FUNCTIONS (RATIO vs GLOBAL AVG, UNALIGNED (M), % UNALIGNED, UNALIGNED ACCESSES/CALL, CALLS, FUNCTION)
------------------------------------------------------------------------------------------------------------------------------
  1.40         0.17   2.09%          985          3 std::io::stdio::_print
  1.39         0.17   2.05%          974          3 core::fmt::write
  1.00         8.06 100.00%      103,111          1 _zisk_main
  0.68         5.27  65.34%           81      1,000 sha2::sha256::compress256
```

A **RATIO** above 1 means the function is more unaligned-heavy than the program average. Only
functions accounting for **more than 1% of the total unaligned cost** are listed, to keep low-volume
outliers with a spectacular ratio but no impact out of the way.

### Logging individual costly accesses (`--log-costly-unaligned`)

When you have narrowed the problem down to a function and want to see the actual accesses,
`--log-costly-unaligned` prints one line per **costly unaligned access** (a double 4B/8B access that
crosses a word boundary) as it happens, with the execution context:

```bash
ziskemu -e <elf> -i <input> -X -S --log-costly-unaligned
```

```
MEM MONITOR pc=0x800a38ec fn='sha2::sha256::compress256' addr=0x00000000a0410003 width=4 read offset=3
MEM MONITOR pc=0x800a3904 fn='sha2::sha256::compress256' addr=0x00000000a0410007 width=8 write offset=7 value=0x00000000deadbeef
```

Each line gives the **pc**, the **function** it belongs to, the **address**, the access **width**,
whether it is a read or a write, the **offset** within the 8-byte word, and (for writes) the value.
This is verbose — redirect it to a file, and use it only after the rankings above have told you
where to look.

## Register Step Distance

Between two accesses to a register, the proof has to carry the fact that the register kept its
value across the whole gap. That gap is bounded: if a register goes untouched for more than a
certain number of steps, the value can no longer be carried and the execution has to be split. The
register step-distance analysis measures those gaps against that limit, so you can tell whether a
program is anywhere near it.

Two flags cover the two situations you will be in: a **detailed report** when you are investigating,
and a **fast one-line check** when you just want a verdict on a whole program.

### Detailed report (`--reg-step-distance`)

```bash
ziskemu -e <elf> -i <input> -X --reg-step-distance --reg-step-limit-bits 18 --reg-step-flush-bits 18
```

```
REGISTER STEP DISTANCE (limit 262,144, flush 2^18 = 262,144 steps, 3,159,192 steps)
REGISTER          ACCESSES      MAX DIST    RATIO     MAX FDIST   FRATIO      >=80%    >=LIMIT       >=2x
---------------------------------------------------------------------------------------------------------
x1 (ra)             46,892         1,490     0.01         1,490     0.01          0          0          0
x2 (sp)            570,318         1,497     0.01         1,497     0.01          0          0          0
x3 (gp)                  3     3,159,185    12.05       262,144     1.00          1          1          1
x5 (t0)            448,954         1,634     0.01         1,634     0.01          0          0          0
x7 (t2)            203,139        45,303     0.17        32,605     0.12          0          0          0
x10 (a0)           716,914           186     0.00           186     0.00          0          0          0
x28 (t3)           370,116        45,327     0.17        32,628     0.12          0          0          0
x31 (t6)           290,005        46,067     0.18        32,603     0.12          0          0          0
---------------------------------------------------------------------------------------------------------
TOTAL            7,784,170     3,159,185    12.05       262,144     1.00          1          1          1
```

One row per register that was accessed at least once, in register-number order:

- **ACCESSES**: Reads plus writes of that register
- **MAX DIST**: Largest gap, in steps, between two consecutive accesses
- **RATIO**: `MAX DIST` as a fraction of the limit — anything at or above `1.00` is over it
- **MAX FDIST** / **FRATIO**: The same maximum, but assuming a **periodic flush**: every 2^`--reg-step-flush-bits` steps, every register is forcibly accessed. A gap that straddles a flush is broken into pieces, so `MAX FDIST` can never exceed the flush period. These forced accesses are *not* counted in ACCESSES — they only feed these two columns.
- **>=80% / >=LIMIT / >=2x**: How many gaps reached 80%, 100% and 200% of the limit. `>=80%` is the early warning: gaps that are not yet a problem but would become one on a slightly different input.

The gaps at the boundaries count like any other: from the program start to a register's first
access, and from its last access to the program end. This is why `x3 (gp)` above — the global
pointer, set once at startup and read twice — shows a `MAX DIST` of essentially the whole program
and a ratio of 12.05. Its `MAX FDIST` of exactly 1.00 shows what the flush buys: with a forced
access every 262,144 steps, the longest that register ever holds a value unobserved is one flush
period.

The `TOTAL` row sums the accesses and the threshold counters, and takes the **maximum** (not the
sum) of the distance columns.

**Choosing the limits.** `--reg-step-limit-bits` sets the limit to 2^bits steps (default 22, i.e.
4,194,304) and `--reg-step-flush-bits` the flush period the same way (default 22). Lower the limit
below the real one to see which registers would break first as programs grow.

### Fast check (`--reg-step-check`)

The detailed report needs the statistics path (`-X`), which is slow. When all you want to know is
*"does this program overflow anywhere?"*, `--reg-step-check` answers it on the **fast emulation
path**, so a whole program can be simulated quickly just for this:

```bash
ziskemu -e <elf> -i <input> --reg-step-check --reg-step-limit-bits 16 --reg-step-flush-bits 18
```

```
REGISTER STEP CHECK: OK, no instance over the 65536 steps limit (limit=65536 instance=2^18=262144 steps max_dist=12702)
```

It splits the execution into **instances** of 2^`--reg-step-flush-bits` steps, each starting with a
flush that accesses every register, and reports how many instances hold at least one distance above
the limit. When the check fails it also names the registers involved:

```
REGISTER STEP CHECK: EXCEEDED in 2 of 4 instances (limit=5 instance=2^4=16 steps max_dist=12)
REGISTER STEP CHECK: registers over the limit: x2(sp) max=12 in 2 inst/2 gaps, x1(ra) max=8 in 1 inst/1 gaps
```

For each register over the limit it gives its worst distance, in how many distinct instances it went
over, and how many gaps did so in total — worst first. Unlike `--reg-step-distance`, this does not
need `-X`.

## Opcode Coverage

`--coverage` reports which opcodes, precompiles and RISC-V instructions the run actually exercised.
It answers a different question from the cost tables: not "what is expensive" but "what did this
input reach at all".

```bash
ziskemu -e <elf> -i <input> -X --coverage
```

```
OPS_COVERAGE:
AVAILABLE: 122
USED: 28
USED NO FROPS: 27 (22.13%) [ltu, lt, eq, add, sub, and, or, xor, add_w, sub_w, sll, srl, srl_w, …]
UNUSED NO FROPS: 95 (77.87%) [minu, min, maxu, max, leu, le, minu_w, min_w, maxu_w, max_w, …]
USED FROPS: 19 (15.57%) [ltu, lt, eq, add, sub, and, or, xor, add_w, sub_w, sll, srl, srl_w, …]

RISC-V INSTRUCTION COVERAGE: 20.73% (40 out of 193)
UNSUPPORTED RISC-V INSTRUCTIONS EXECUTED: roriw andn andn roriw roriw rev8 …
EXECUTED RISC-V INSTRUCTIONS: add addi addiw and andi auipc beq bge bgeu blt bltu bne …
NON_EXECUTED RISC-V INSTRUCTIONS: addw amoadd.d amoadd.w amoand.d amoand.w …
```

This is useful to:

- **Validate a test input**: low coverage means the input is not exercising the paths you think it is
- **Check that the toolchain emits what you expect**: seeing `roriw`, `andn` or `rev8` in the
  executed set confirms the bit-manipulation extensions are being used instead of multi-instruction
  sequences
- **Understand FROPS reach**: `USED FROPS` vs `USED NO FROPS` shows how many of the opcodes the
  program uses have a pre-calculated fast path at all

## Comparing Runs

Optimization work is a loop of *change something, measure, confirm*. Reading two full reports side
by side does not scale, so ZiskEmu can save an aggregate snapshot of a run and diff a later run
against it.

### Saving a snapshot (`--save-stats`)

```bash
ziskemu -e <elf> -i <input> -X --save-stats before.csv
```

The snapshot holds the aggregate counters only — cost distribution, per-opcode counts and costs,
precompiles, FROPS and memory — with no per-function detail, so it is small enough to commit
alongside a benchmark. `--csv-separator` changes the field separator (default `,`).

### Diffing against a snapshot (`--ref-stats`)

Run the modified program and compare it against the saved reference in the same command. The full
report is printed as usual, followed by the comparison:

```bash
ziskemu -e <elf> -i <input> -X --ref-stats before.csv
```

```
COMPARISON   before.csv  →  current run
red = higher / worse   green = lower / better   % = share of total   sign always shown

STEPS                         163,609               -148,968 (-47.66%)

COST DISTRIBUTION                   COST        %                     Δ (Δ%)
------------------------------------------------------------------------------
MAIN                          11,125,412    3.50%      -10,129,824 (-47.66%)
OPCODES                          188,671    0.06%         -650,730 (-77.52%)
PRECOMPILES                   15,553,188    4.89%      -60,706,144 (-79.60%)
MEMORY                         3,775,257    1.19%       -4,310,854 (-53.31%)
VARIABLE                      30,642,528    9.64%      -75,797,552 (-71.21%)
BASE                         287,309,824   90.36%                +0 (+0.00%)
TOTAL                        317,952,352  100.00%      -75,797,552 (-19.25%)
FROPS                          2,340,075    7.64%         -997,172 (-29.88%)

COST BY PRECOMPILED OPCODE
                                COUNT       %            Δ            COST       %                   Δ (Δ%)
--------------------------------------------------------------------------------------------------------------
OP keccak                         204   0.12%         -796      15,417,300   4.85%    -60,157,700 (-79.60%)
OP dma_xmemset                    413   0.25%       -2,388          75,348   0.02%       -368,548 (-83.03%)

FROPS BY OPCODE                 COUNT      HIT      ΔHIT            COST       %                   Δ (Δ%)
----------------------------------------------------------------------------------------------------------------
FROP xor                       34,979   98.62%   +11.0pp       2,098,740   6.85%       -477,600 (-18.54%)
FROP eq                         1,447   89.54%    -5.2pp          86,820   0.28%        -95,520 (-52.39%)
```

Every section shows the **current** value alongside the **delta** against the reference, absolute
and relative, with the sign always displayed. FROPS hit rates are compared in **percentage points**
(`pp`), since a percentage of a percentage would be meaningless.

This example is the payoff of the [duplicate analysis](#precompile-duplicate-analysis) shown
earlier: hoisting the four repeated hashes out of the loop removed 796 of the 1,000 keccak calls,
cutting the precompile cost by 79.6% and the variable cost by 71.2%. Note how **TOTAL** only moves
19.25% — the constant BASE dilutes it — which is exactly why **VARIABLE** is the line to read.

### Diffing two snapshots (`--diff-stats`)

To compare two saved snapshots without running anything, pass both files. No ELF or input is
needed:

```bash
ziskemu --diff-stats before.csv after.csv
```

The first file is the reference, so deltas are `after - before`. This is the form to use in CI,
where the runs happen on different machines or at different times.

Both comparison forms can also be rendered as a page instead of printed — see
[HTML Report](#html-report).

### Comparison output format

| Flag | Effect |
|------|--------|
| `--color <auto\|always\|never>` | Colourize the comparison. `auto` (default) colours only when stdout is a terminal — so a redirected log stays clean. |
| `--diff-format <color\|csv>` | `color` (default) is the human-readable view above; `csv` is a plain semicolon-separated view for scripting. |
| `--legacy-display` | Equivalent to `--diff-format csv`. Also implied by `--sdk`. |
| `--csv-separator <SEP>` | Field separator for `--save-stats` and the `csv` view (default `,`). |

## HTML Report

`--html-report` renders the statistics as a **standalone HTML page** instead of a CSV snapshot. The
snapshot content is handed straight to the report renderer, so no intermediate CSV file is written
unless you also ask for one with `--save-stats`.

```bash
# Single-run report, written to report.html
ziskemu -e <elf> -i <input> -X --html-report

# Choose the output path
ziskemu -e <elf> -i <input> -X --html-report profile.html
```

The page contains the same data as the text report — cost distribution, base and precompiled
opcodes, FROPS, and the memory breakdown — laid out as sortable tables and bars, which is easier to
scan and to attach to a PR or a ticket than a wall of terminal output.

### Comparison report

Give it a reference and it renders the **comparison** of both snapshots instead of a single run.
Two ways, matching the two text-mode forms:

```bash
# Run the program and compare it against a saved snapshot
ziskemu -e <elf> -i <input> -X --ref-stats before.csv --html-report compare.html

# Compare two saved snapshots, without running anything
ziskemu --diff-stats before.csv after.csv --html-report compare.html
```

In both cases the first snapshot is the **baseline (A)** and the second is **B**, with the change
shown as `B - A`: green for a lower cost, red for a higher one. With `--ref-stats` the current run
is B, labelled *current run*; with `--diff-stats` each side is labelled with its snapshot path.

**Note**: `--diff-stats` needs no ELF, input or emulation, which makes it the form to use in CI —
save a snapshot per commit and render the page comparing the two.

### Notes

- `--html-report` implies collecting the full statistics, exactly like `--save-stats`, so it works
  with or without `-X` (with `-X` you also get the text report on stdout).
- Snapshots are read back with whatever separator they were saved with, so a reference saved using
  `--csv-separator ';'` renders correctly.
- The same renderer is available as a standalone binary that reads snapshots from disk:
  `cargo run --bin report -- stats.csv [other.csv]`.

## SDK Report Mode

For a **cleaner, more compact output** ideal for continuous integration or quick checks, use the `--sdk` flag. This provides a streamlined report with only the essential summary information.

### Command

```bash
ziskemu -e <elf> -i <input> --sdk -X
```

**Note**: `--sdk` selects the *format* of the report; `-X` is what makes the statistics be collected
in the first place. Without `-X` nothing is printed.

### Output Example

```
╔══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╗
║  ◆ REPORT SUMMARY                                                                                                    ║
╠══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╣
║  STEPS                                                                                                    3,159,193  ║
║  COST                                                                                                   632,033,308  ║
║  RAM                                                                                            0.00 MB / 507.75 MB  ║
╚══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╝

╔══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╗
║  ◆ COST DISTRIBUTION SUMMARY                                                                                         ║
╠══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╣
║  CATEGORY                                                                                               COST      %  ║
║  ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄  ║
║  Base         ███████████████████████████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░     287,309,824  45.5%  ║
║  Main         ███████████████████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░     214,825,124  34.0%  ║
║  Opcodes      ██████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░     110,764,355  17.5%  ║
║  Precompiles  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░         836,980   0.1%  ║
║  Memory       ██░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░      18,297,025   2.9%  ║
║  ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄  ║
║  Total                                                                                           632,033,308 100.0%  ║
╚══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╝
```

The SDK report provides:
- **Clean visual layout** with box-drawing characters
- **Progress bars** showing the proportional cost of each category
- **Essential metrics only**: steps, total cost, RAM usage, and cost distribution
- **No detailed breakdowns** - ideal for automated testing or quick cost checks

### SDK Selective Sections

By default, the SDK report shows only the summary. You can selectively enable additional sections:

#### Show Opcode Details (`--opcodes`)

Adds a section showing the top 10 most expensive opcodes with their cost distribution and FROPS hit rates:

```bash
ziskemu -e <elf> -i <input> --sdk --opcodes
```

This adds a **COST DISTRIBUTION BY OPCODE** section comparing regular operations vs frequent operations (FROPS).

#### Show Top Functions (`--top-functions`)

Lists the functions with highest cost. **Requires `-S`** to read symbols:

```bash
ziskemu -e <elf> -i <input> --sdk --top-functions -S
```

This adds a **TOP COST FUNCTIONS** section with automatic compacting of long function names.

**Note**: Using `--top-functions` automatically enables symbol reading (`-S`), so you can omit the `-S` flag if you only need it for this feature.

#### Show Profile Tags (`--profile-tags`)

Displays accumulated profile tag measurements from your code. **Requires profile tags** in your program (see [Profile Tags](#profile-tags) section):

```bash
ziskemu -e <elf> -i <input> --sdk --profile-tags
```

This shows sections like **STEPS PROFILE TAGS** and **COST PROFILE TAGS** if you've instrumented your code with profile markers.

#### Combining Options

You can combine multiple flags to customize the report:

```bash
# Show summary + opcodes + top functions
ziskemu -e <elf> -i <input> --sdk --opcodes --top-functions -S

# Show all optional sections
ziskemu -e <elf> -i <input> --sdk --opcodes --top-functions --profile-tags -S
```

**Behavior Note**: If you specify any of the selective flags (`--opcodes`, `--top-functions`, `--profile-tags`), **only the summary plus the explicitly requested sections** will be shown. If you don't specify any selective flags, you get only the summary.

### SDK Width Configuration

Control the width of the SDK report output with `--sdk-width`:

```bash
# Use wider report (150 characters)
ziskemu -e <elf> -i <input> --sdk --sdk-width=150

# Use narrower report (100 characters) 
ziskemu -e <elf> -i <input> --sdk --sdk-width=100
```

**Default width**: 120 characters. Wider reports provide more space for progress bars and function names, while narrower reports fit better in smaller terminals or log viewers.

## Function Name Display Options

When displaying function-level profiling information with `-S`, function names can become very long, especially in Rust with its fully-qualified paths and generic parameters. ZiskEmu provides options to control how these names are displayed.

### Compact Names (Default)

**By default**, long function names are automatically shortened to 160 characters using intelligent compacting:

```bash
# Default behavior - compact to 160 characters
ziskemu -e <elf> -i <input> -X -S
```

The compacting algorithm:
1. Collapses nested generic parameters: `<A<B<C>>>` → `<A<…>>`
2. Elides intermediate path segments: `std::io::default_write_fmt::Adapter` → `std::..::Adapter`
3. Maintains readability while reducing length

### Custom Compact Length

Specify a different maximum length:

```bash
# Compact to 80 characters
ziskemu -e <elf> -i <input> -X -S --compact-names=80

# Compact to 200 characters  
ziskemu -e <elf> -i <input> -X -S --compact-names=200
```

### Disable Compacting

To see complete, uncompacted function names:

```bash
ziskemu -e <elf> -i <input> -X -S --no-compact-names
```

**When to use each option:**

- **Default (160 chars)**: Good balance for most terminal widths and readability
- **Shorter (80-100 chars)**: When viewing in narrow terminals or want very concise output
- **Longer (200+ chars)**: When you need more context from the function path
- **No compacting**: When you need to see the complete, exact function signatures (e.g., for copy-pasting into code searches)

## Profile Tags

Profile tags allow you to **instrument your code** to measure specific code sections, loops, or algorithms. This is useful when you want to:

- Measure the cost or steps of a specific algorithm
- Compare different implementation approaches
- Track performance of critical sections across multiple calls
- Identify hotspots within a single function

### How Profile Tags Work

You add markers in your guest code using macros provided by `ziskos`. These markers:
- Have **zero overhead** when not running in the ZiskEmu profiler
- Work at the **source code level** - you decide what to measure
- Can measure either **steps** (execution cycles) or **cost** (profiling cost)
- Can either **print immediately** or **accumulate for a summary report**

### Setting Up Profile Tags

In your guest code's `Cargo.toml`, add the ziskos dependency:

```toml
[dependencies]
ziskos = { path = "../../ziskos" }  # Adjust path as needed
```

In your guest source code:

```rust
use ziskos::{profile_start, profile_end};
use ziskos::{profile_report_start, profile_report_end};
use ziskos::{profile_steps_start, profile_steps_end};
use ziskos::{profile_report_steps_start, profile_report_steps_end};

fn main() {
    // Example usage in your code
    profile_start!(hash_computation);
    let result = expensive_hash_function(&data);
    profile_end!(hash_computation);
    
    // ... more code
}
```

### Profile Tag Macros

There are **8 macros** organized in 2 dimensions:

**Dimension 1 - What to measure:**
- **Cost macros** (`profile_start!` / `profile_end!`): Measure profiling cost
- **Steps macros** (`profile_steps_start!` / `profile_steps_end!`): Measure execution steps

**Dimension 2 - When to report:**
- **Immediate** (`profile_start!` / `profile_end!`): Print result after each `end!` call
- **Report** (`profile_report_start!` / `profile_report_end!`): Accumulate and show at program end

#### Immediate Output Macros

Print the measurement immediately after the `end!` call:

```rust
// Measure and print COST after each execution
profile_start!(my_algorithm);
run_my_algorithm();
profile_end!(my_algorithm);
// Prints: [my_algorithm] 12345

// Measure and print STEPS after each execution  
profile_steps_start!(my_loop);
for i in 0..1000 {
    expensive_operation(i);
}
profile_steps_end!(my_loop);
// Prints: [my_loop] 45678
```

**Use case**: When you want to track each individual execution, or when the measured section is called only once or a few times.

#### Report Macros

Accumulate measurements and show statistics at the end:

```rust
for batch in batches {
    profile_report_start!(process_batch);
    process_batch(&batch);
    profile_report_end!(process_batch);
}
// No output during execution

// At program end, you'll see accumulated statistics:
// Total, average, min, max for all executions
```

**Use case**: When measuring sections called many times (loops, repeated operations) and you want aggregate statistics rather than individual measurements.

### Complete Example

```rust
use ziskos::{
    profile_start, profile_end,
    profile_report_start, profile_report_end,
    profile_steps_start, profile_steps_end,
    profile_report_steps_start, profile_report_steps_end
};

fn main() {
    // Measure total cost once
    profile_start!(total_execution);
    
    // Accumulate statistics for repeated calls
    for i in 0..100 {
        profile_report_steps_start!(loop_iteration);
        expensive_computation(i);
        profile_report_steps_end!(loop_iteration);
    }
    
    // Nested measurements
    profile_steps_start!(data_processing);
    
    profile_report_start!(hash_phase);
    for item in items {
        compute_hash(item);
    }
    profile_report_end!(hash_phase);
    
    profile_steps_end!(data_processing);
    
    profile_end!(total_execution);
}
```

### Viewing Profile Tag Results

To see the accumulated profile tag statistics, add `--profile-tags` to your command:

```bash
# With standard report
ziskemu -e <elf> -i <input> -X --profile-tags

# With SDK report  
ziskemu -e <elf> -i <input> --sdk --profile-tags
```

The output shows aggregated statistics for all profile tags used with the `report` variants:

```
PROFILE TAGS STEPS (STEPS, % STEPS, CALLS, AVG, MIN, MAX)
----------------------------------------------------------
     10,234,567  11.02%        100     102,345     98,123     125,678  loop_iteration
      3,456,789   3.72%         50      69,135     45,000      89,000  hash_phase

PROFILE TAGS COST (COST, % COST, CALLS, AVG, MIN, MAX)
-------------------------------------------------------
  1,234,567,890  10.79%        100  12,345,678  10,000,000  15,000,000  total_execution
    456,789,012   3.99%         50   9,135,780   5,000,000  12,000,000  hash_phase
```

**Statistics shown:**
- **TOTAL**: Sum of all measurements
- **% TOTAL**: Percentage of total steps or cost
- **CALLS**: Number of times the tag was executed
- **AVG**: Average per call
- **MIN**: Minimum value observed
- **MAX**: Maximum value observed

### Best Practices

1. **Use descriptive tag names**: `hash_computation` is better than `tag1`
2. **Choose report vs. immediate based on frequency**:
   - Few calls (1-10): Use immediate variants
   - Many calls (100+): Use report variants
3. **Match start/end pairs**: Always use matching macro pairs (same tag name, same variant)
4. **Don't nest same tag names**: Each tag should represent a unique code section
5. **Combine with function profiling**: Profile tags show "what", function profiling shows "where"

## Firefox Profiler Integration

ZiskEmu can export profiling data to **Firefox Profiler format**, enabling advanced visualization and analysis of your program's execution.

### Generating Profiler Data

Use `--profiler-output` to specify the output file:

```bash
# Generate compressed profiler data (recommended)
ziskemu -e <elf> -i <input> -X -S --profiler-output=profile.json.gz

# Generate uncompressed JSON
ziskemu -e <elf> -i <input> -X -S --profiler-output=profile.json
```

**Requirements**: The `-S` flag is **required** to load symbol information. The `-X` flag is **recommended** for complete profiling data.

**Default**: If you use `-X -S` without specifying `--profiler-output`, a file named `profile.json.gz` is created automatically.

### Viewing in Firefox Profiler

1. Go to https://profiler.firefox.com
2. Click "Load a profile from file"
3. Select your `profile.json.gz` file

The Firefox Profiler provides:
- **Call tree visualization** showing the function call hierarchy
- **Flame graphs** for identifying performance hotspots
- **Timeline view** showing execution progress over time
- **Function details** with cumulative costs
- **Search and filtering** capabilities

### Use Cases

Firefox Profiler is particularly useful when:
- You need to **visualize complex call graphs**
- Standard text reports are too verbose
- You want to **share profiling results** with team members
- You need to **compare multiple profiling runs**
- You want **interactive exploration** of the call stack

### File Format

The exported file follows the [Firefox Profiler format specification](https://github.com/firefox-devtools/profiler/blob/main/docs-developer/processed-profile-format.md), making it compatible with other tools that support this format.

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
TOP COST FUNCTIONS (COST, % VARIABLE COST, CALLS, COST/CALL, FUNCTION)
----------------------------------------------------------------------
  5,255,204,123  45.95%          1   5,255,204,123 <reth_evm::execute::BasicBlockExecutor<&reth_evm
  5,172,696,823  45.23%          1   5,172,696,823 <alloy_evm::eth::block::EthBlockExecutor<alloy_e
  4,997,989,104  43.70%         70      71,399,844 <revm_handler::mainnet_handler::MainnetHandler<r
  4,530,507,470  39.61%     41,793         108,403 <zeth_mpt::mpt::node::Node<zeth_mpt::mpt::memoiz
  4,014,605,785  35.10%          1   4,014,605,785 <zeth_mpt_state::SparseState as stateless::trie:
  3,759,934,537  32.87%     10,505         357,918 ziskos::zisklib::lib::keccak256::keccak256
```

Shows for each function:
- **COST**: Total cumulative profiling cost of the function (including all nested calls)
- **% VARIABLE COST**: Percentage of the program's **variable** cost this function represents. It is measured against VARIABLE rather than TOTAL because the constant BASE cost belongs to no function — including it would shrink every percentage by the same arbitrary factor.
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
|    STEPS                          1,516,032   1.99%

|    MAIN COST                    103,090,176  57.43%
|    OPCODES COST                   1,451,520   0.81%
|    PRECOMPILES COST                       0   0.00%
|    MEMORY COST                   74,973,696  41.76%
|                             -----------------------
|    TOTAL COST                   179,515,392 100.00%

|    DETAILED MEM COST                                  COUNT       %            COST       %
|    ----------------------------------------------------------------------------------------
|    RAM STACK aligned 8B read                         21,504   1.49%         344,064   0.46%
|    RAM STACK aligned 8B write                        21,504   1.49%         387,072   0.52%
|    RAM NO STACK aligned 8B read                      10,752   0.75%         172,032   0.23%
|    RAM NO STACK unaligned 1B read                   698,880  48.51%      28,654,080  38.22%
|    RAM NO STACK unaligned 1B clean write            688,128  47.76%      45,416,448  60.58%
|                                             -----------------------------------------------
|    TOTAL aligned 8B                                  53,760   3.73%         903,168   1.20%
|    TOTAL unaligned 1B single                      1,387,008  96.27%      74,070,528  98.80%
|                                             -----------------------------------------------
|    TOTAL aligned 8B                                  53,760   3.73%         903,168   1.20%
|    TOTAL unaligned 1B                             1,387,008  96.27%      74,070,528  98.80%
|                                             -----------------------------------------------
|    TOTAL reads                                      731,136  50.75%      29,170,176  38.91%
|    TOTAL writes                                     709,632  49.25%      45,803,520  61.09%
|                                             -----------------------------------------------
|    TOTAL                                          1,440,768 100.00%      74,973,696 100.00%

|    COST BY OPCODE                     COUNT       %            COST       %
|    ------------------------------------------------------------------------
|    OP keccak                         32,650   0.04%  2,466,707,500  65.61%
|    OP or                          2,489,249   3.27%    149,354,940   3.97%
|    OP xor                           492,192   0.65%     29,531,520   0.79%
|    OP sll                           360,008   0.47%     19,080,424   0.51%
|    OP and                            94,545   0.12%      5,672,700   0.15%
|    OP add                           169,207   0.22%      4,230,175   0.11%
|    OP ltu                            28,516   0.04%      1,710,960   0.05%
|    OP dma_memcpy                     21,010   0.03%        882,420   0.02%
|    OP dma_xmemset                    21,010   0.03%        882,420   0.02%
|    OP sub                             3,644   0.00%        218,640   0.01%

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

**Function Header and per-function cost distribution:**
```
DETAIL FUNCTION ziskos::zisklib::lib::keccak256::keccak256
----------------------------------------------------------
|    STEPS                          1,516,032   1.99%

|    MAIN COST                    103,090,176  57.43%
|    OPCODES COST                   1,451,520   0.81%
|    PRECOMPILES COST                       0   0.00%
|    MEMORY COST                   74,973,696  41.76%
|                             -----------------------
|    TOTAL COST                   179,515,392 100.00%

```
Shows the total cumulative steps for this function (including nested calls) and, broken down the same
way as the top-level **COST DISTRIBUTION**, how that function's cost splits across MAIN, OPCODES,
PRECOMPILES and MEMORY. This makes it easy to see whether a function is dominated by computation,
precompiles or memory traffic.

When `--mem-stats` or `--mem-full-stats` is passed, a **per-function memory breakdown** is inserted
right after the cost distribution — the same **MEM COST BY TYPE** / **DETAILED MEM COST** tables
described in [Memory Statistics](#memory-statistics), but scoped to this function. This lets you attribute
unaligned/double accesses to the exact function that causes them.

**COST BY OPCODE section:**
```
|    COST BY OPCODE                     COUNT       %            COST       %
|    ------------------------------------------------------------------------
|    OP keccak                         32,650   0.04%  2,466,707,500  65.61%
|    OP or                          2,489,249   3.27%    149,354,940   3.97%
|    OP xor                           492,192   0.65%     29,531,520   0.79%
```
Breaks down which operations consume resources within this function, **sorted by cost**, base and
precompiled opcodes together (unlike the top-level report, which splits them into two tables):
- **COUNT**: Number of times each operation was executed, and its share of the program's steps
- **COST**: Total profiling cost for all executions
- **%**: Percentage of this function's total cost

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

The `-H` (or `--top-histogram`) flag provides a **low-level view** of the most frequently executed code positions in your program. Unlike function-level profiling, this analysis operates at the **program counter (PC)** level, showing you the exact assembly instructions that execute most often.

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

## Instruction Tracing and Disassembly

When the aggregate reports are not enough and you need to see the program instruction by
instruction, there are three tools, from most verbose to most compact.

### Full instruction trace (`--trace-steps`)

Prints every executed instruction to stdout — step, pc and the decoded instruction. Unlike
`--log-step`, it works in release builds:

```bash
ziskemu -e <elf> -i <input> --trace-steps > trace.txt
```

```
### S:0 PC 1000: Jump over end instruction
### S:1 PC 1008: Set marchid: fffeeee
### S:2 PC 100c: Set mtvec: 4188
### S:3 PC 1010: Set 1st Param (pInput): 0x40000000
```

This forces full (non-fast) emulation and produces one line per step, so **always redirect it to a
file** — a program of a few million steps produces hundreds of MB.

### Change trace over a step window (`--trace-from` / `--trace-to`)

Usually you do not want the whole program, only the few thousand steps around a divergence. The
change trace prints each executed instruction in a window **followed by every register and stack
write it caused**, as `prev (0xhex) => post (0xhex)`:

```bash
ziskemu -e <elf> -i <input> --trace-from 1000 --trace-to 1006
```

```
### S:1000 PC 800034f4: xor r12, r12, r14
### S:1001 PC 800034f8: srliw r14, r6, 0xa
    reg x14 (a4): 0 (0x0) => 281248 (0x44aa0)
### S:1002 PC 800034fc: xor r22, r15, r14
    reg x22 (s6): 2563236514 (0x98c7e2a2) => 268435472 (0x10000010)
### S:1003 PC 80003500: srliw r14, r11, 0x3
    reg x14 (a4): 281248 (0x44aa0) => 0 (0x0)
### S:1004 PC 80003504: sd r11, 0x380(r2)
    stack 0xa03ffd40 [sp+0x380]: 0 (0x0) => 0 (0x0)
```

Register writes show the register number and its ABI name. **Stack writes** are RAM writes in the
range `[RAM_ADDR, SYS_ADDR)` and show both the absolute address and its offset relative to `sp`
(`x2`), which is what makes a trace comparable across runs whose stacks sit at different addresses.

`--trace-from` defaults to 0 and `--trace-to` to the end, so either can be given alone. Like
`--trace-steps`, this forces full emulation.

### Annotated disassembly (`--disasm`)

Writes an objdump-like disassembly of the whole program to a file, with the **execution count** of
each instruction next to it. Requires `-S -X`:

```bash
ziskemu -e <elf> -i <input> -X -S --disasm program.asm
```

```
  00001018:             1                                  copyb x1, 0, 0x80000000 ; store_pc
  0000101c:             1                                  copyb x11, 0, 0x20
  00001020:             1                                  copyb x12, 0, 0
```

Unlike [the PC histogram](#pc-histogram-analysis), which lists only the hottest sequences, this
covers the whole program including the instructions that never executed (count 0) — useful to spot
dead code, or to confirm that a branch you expected to be taken never was.

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

### Sort Statistics by Operation Count

By default the opcode, precompile and FROPS tables are sorted by cost. `--sort-by-units` sorts them
by operation count instead, which is what you want when hunting for the most *frequent* operation
rather than the most expensive one:

```bash
ziskemu -e target/elf/riscv64ima-zisk-zkvm-elf/release/guest -i input.bin -X --sort-by-units
```

It also changes the group order of the [operand pattern analysis](#operand-pattern-analysis).

### Call-Stack Tracking Mode

The per-function report (`-S -X`) reconstructs a call stack by watching calls and returns. Some
code — notably GCC/C++ tail recursion such as `std::sort`'s `__introsort_loop` — returns in a way
that does not match the recorded stack. `--callstack-mode` decides what happens then:

```bash
# auto (default): resync the call stack when a mismatch is detected
ziskemu -e <elf> -i <input> -X -S --callstack-mode auto

# strict: disable call-stack tracking on the first mismatch
ziskemu -e <elf> -i <input> -X -S --callstack-mode strict
```

Use `strict` when you suspect the resync is inventing call relationships and want to know whether a
mismatch happened at all; `auto` otherwise, since it keeps the per-function report usable on C++
code.

### Legacy Statistics Format

`-x` (lowercase) prints the legacy statistics report, kept for compatibility with older tooling.
Use `-X` for everything described in this guide.

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

### Follow the Cost to the Right Tool

Once the cost distribution tells you *which category* dominates, there is a specific analysis for
each:

| If the cost is in… | Use | To find |
|--------------------|-----|---------|
| PRECOMPILES | [`--duplicates`](#precompile-duplicate-analysis) | Calls that recompute a result already proven |
| OPCODES | [`--pattern-analysis`](#operand-pattern-analysis) / [`--opcode-breakdown`](#opcode-variant-breakdown) | Operand shapes that a cheaper state machine could prove |
| MEMORY | [`--mem-full-stats`](#memory-statistics) | Unaligned accesses and the functions causing them |
| MAIN | [`-H`](#pc-histogram-analysis) / [`-S -D`](#detailed-caller-analysis) | Hot instruction sequences and the functions running them |

Then use [`--save-stats` / `--ref-stats`](#comparing-runs) to confirm the change actually paid off.

### Focus on High Impact

Use the final_cost percentage to identify functions with the highest impact. Optimizing a function that represents 50% of execution time will have much more effect than optimizing one at 1%.

### Compare Against VARIABLE, Not TOTAL

The BASE cost is a constant that every program pays. When you compare two runs, a 70% cut in the
work your code does can look like a 19% cut in TOTAL simply because BASE did not move. Read the
**VARIABLE** line, which is what your changes actually control.

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
TOP COST FUNCTIONS (COST, % VARIABLE COST, CALLS, COST/CALL, FUNCTION)
----------------------------------------------------------------------
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
