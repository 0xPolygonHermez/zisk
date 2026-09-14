# ZisK Limits

## Introduction

Any software has limits.  You cannot compute an infinite amount of data using an infinite amount of resources.

This document explains the limitations of ZisK.  This information is relevant only if your program is very big, or the program execution is very long, or your hardware is very limited.  For the vast majority of cases, the current limits are more than enough, and therefore you can discard this information.

Current usage examples are based on an Ethereum client block validation.

## Summary table

| Name | Limit | Max usage | Percentage |
| :---- | :---- | :---- | :---- |
| INPUT SIZE | < 1 GB | 20 MB (ETH) | 2 % |
| ROM SIZE | 127 MB | 4 MB (ETH) | 3 % |
| RAM SIZE | 507.75 MB | 256 MB (ETH) | 50 % |
| TRACE SIZE | 32 GB | 7 GB (ETH) | 22 % |
| MAX STEPS | 2^36 = 64 G | 2 G (ETH) | 3 % |
| PROGRAM LENGTH | 2^22 = 4M | 600 k (ETH) | 14 % |
| PHYS MEM (ASM) | 64 GB | N/A | N/A |
| PHYS MEM (EMU) | 32 GB | N/A | N/A |
| MEM COUNT & PLAN SLOTS | 6 GB | < 2 GB | 33 % |

## INPUT SIZE

This limit consists of the maximum input binary data size in bytes.

In order to inject the input data into the assembly emulator a shared memory mechanism is used.  Shared memory provides the maximum performance, and it allows consuming the input data with zero-copy.  The input shared memory size is 1 GB.  There is an overhead due to the use of some headers to manage the input data chunks, but they are very small, so in practice, the input data size is limited to almost 1 GB.

An Ethereum client block verification normally needs an input data size of 15 MB to 20 MB.  This is approximately 2 % of the maximum input data size.

Some very specific programs might require a higher input data size.  Increasing the input data size would require PIL modification, assigning a larger virtual address space for the assembly, and it would consume more physical memory.

We have plans to add to ZisK the support to consume input data as a stream.  This is very useful if the guest program does not need to store input data after it is consumed.  In this case, the shared memory would be used as a ring buffer, becoming virtually infinite.  However, this will prevent the zero-copy nature of the current shared memory mechanism, and therefore the performance would be lower.

## ROM SIZE

This limit consists of the maximum ROM address space size in bytes.  Another way to describe it is the maximum size of the program code and the program constant data, together.  In essence, we are referring to the program size.

Currently, ZisK maps the ROM to the virtual address space between addresses 0x8000_0000 and 0x87FF_FFFF, i.e. 128 MB.  The top 1 MB of that region, from 0x87F0_0000 to 0x87FF_FFFF, is used by the float library ROM.  This leaves an effective limit of 127 MB for the program.

An Ethereum client normally uses about 4 MB.  This is approximately 3 % of the maximum ROM size.

Increasing the ROM size would require PIL modification, assigning a larger virtual address for the assembly, and it would consume more physical memory.

## RAM SIZE

This limit consists of the maximum RAM address space size in bytes.

ZisK uses the following RAM virtual address map:

| From | To | Usage |
|---|---|---|
| 0xA000_0000 | 0xA03F_FFFF | program stack (4 MB) |
| 0xA040_0000 | 0xA040_FFFF | ziskos system RAM (64 KB): memory-mapped GPR registers at +0x0, UART at +0x200, memory-mapped float registers at +0x1000, and CSRs at +0x8000 |
| 0xA041_0000 | 0xA042_FFFF | output |
| 0xA043_0000 | 0xBFFE_FFFF | program RAM (including heap) |
| 0xBFFF_0000 | 0xBFFF_FFFF | float library RAM address |

This leaves an effective program RAM limit of 507.75 MB.

An Ethereum client block verification normally consumes up to 256 MB (for very busy and complex real blocks).  This is approximately 50 % of the maximum ram size.

Increasing the RAM size would require PIL modification, assigning a larger virtual address for the assembly, and it would consume more physical memory.

The current ZisK Ethereum client guest program uses a memory manager that does not free the used memory.  This increases the execution performance, but of course requires more RAM space.  We could use a normal memory manager that frees unused memory, at the cost of decreasing the performance.

## TRACE SIZE

This limit consists of the maximum size of the minimal trace, or memory operations trace size, in bytes.

The trace size can occupy the 64-bit address space from address 0xD000_0000 to address 0x08_CFFF_FFFF, leaving a maximum trace size of 32 GB.

An Ethereum client block verification normally takes up to 7 GB.  This is approximately 22 % of the maximum trace size.

Increasing the maximum trace size with the current architecture has some implications.  Since virtual memory address space is reserved at linkage time, the OS does only allow to run programs that have reserved sections < physical memory, so we need more physical memory.  Since it does not require PIL modifications, it could be even configurable at compile time.

## MAX STEPS

This limit consists of the maximum number of execution steps.

Currently, the maximum number of steps is 2^36 = 68719476736 (64 G steps) = 0x1000000000, as per PIL configuration.  Note that the emulator's default cap is one step below that value, i.e. 2^36 - 1 = 68719476735 = 0xF_FFFF_FFFF.

An Ethereum client block execution normally takes up to 2 G steps.  This is approximately **3 %** of the maximum number of steps.

Increasing the maximum number of steps requires changes in the PIL configuration.

## PROGRAM LENGTH

This limit consists of the maximum number of program instructions.  In other words, it is the number of different instructions present in the program.

The current limit is 2^22 = 4 M instructions, as per PIL configuration.

An Ethereum client takes about **600 K** instructions.  This is approximately 14 % of the maximum program length.

Increasing the maximum program length implies increasing the ROM AIR length, or support inner continuations for ROM AIRs.  Both cases require PIL changes and a new ZisK setup.

## PHYSICAL MEMORY (ASSEMBLY)

This limit consists of the minimum physical memory required in the prover or worker system to execute the program, when using the ZisK assembly emulator.

The current limit is 64 GB, due to the virtual memory address reservation at link time of the assembly services, specifically for the trace shared memory (32 GB)

## PHYSICAL MEMORY (EMULATOR)

This limit consists of the minimum physical memory required in the prover or worker system to execute the program, when using the ZisK Rust emulator.

The current limit is 32 GB.

## MEM COUNT & PLAN SLOTS

This limit consists of the number of slots allocated to calculate the memory count & plan.

The current limit is 6 GB.

An Ethereum client consumes up to 2 GB.  This is approximately 33 % of the maximum.

If during the emulation this limit is reached, the following message is displayed (original trace code is shown):

```
ERROR: MemCounter: no free slots left for this thread(" << id << "). Increase MAX_SLOT_GB in state-machines/mem-cpp/cpp/mem_config.hpp and recompile zisk.
```

The trace itself provides information about how to modify this limit, i.e. to increase MAX_SLOT_GB in file state-machines/mem-cpp/cpp/mem_config.hpp, then recompile.
