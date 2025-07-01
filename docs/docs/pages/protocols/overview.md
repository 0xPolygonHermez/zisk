---
description: ZisK Overview 
---

# ZisK Overview 

ZisK is a high-performance zero-knowledge virtual machine (zkVM) designed to generate zero-knowledge proofs of arbitrary program execution. It enables developers to prove the correctness of computations without revealing their internal state. ZisK abstracts the complexities of cryptographic operations by providing an optimized toolstack that minimizes computational overhead, making zero-knowledge technology accessible to developers.

Key features of ZisK include:

- **High-performance architecture** optimized for low-latency proof generation
- **Rust-based zkVM** with future support for additional languages
- **No recompilation required** across different programs
- **Standardized prover interface** (JSON-RPC, GRPC, CLI)
- **Flexible integration** as standalone service or library
- **Fully open-source**, backed by Polygon zkEVM and Plonky3 technology

ZisK converts RISC-V programs into provable form through a sophisticated pipeline involving emulation, witness generation, and constraint satisfaction.

## System Architecture Overview

ZisK consists of several interconnected components that work together to convert programs into provable form, execute them, generate witnesses, and create zero-knowledge proofs.

![ZisK System Architecture](/zisk-system.png)

### High-Level System Architecture

The ZisK system is organized into several key areas:

- **Infrastructure**: Core VM system and development toolchain
- **Input Processing**: RISC-V ELF files and transpilation
- **Specialized State Machines**: Parallel processing of different operation types
- **Witness Generation**: Coordination and AIR instance creation
- **Core VM System**: Fundamental execution and constraint handling

### Core Components

| Component | Package | Purpose |
|-----------|---------|---------|
| zisk-core | zisk-core | Fundamental definitions, types, and operations. Contains ZiskRom, ZiskInst, and instruction set definitions |
| ziskemu | ziskemu | RISC-V/ZisK emulator that executes programs and generates execution traces |
| zisk-witness | zisk-witness | Witness generation library that coordinates between emulator and state machines |
| zisk-pil | zisk-pil | Polynomial Identity Language definitions for mathematical constraints |
| executor | executor | Execution orchestrator that manages the overall proof generation process |
| data-bus | data-bus | Inter-component communication system for state machine coordination |
| zisk-common | zisk-common | Shared utilities and common functionality across components |

### State Machine Packages

| State Machine | Package | Operations Handled |
|---------------|---------|-------------------|
| Main SM | sm-main | Execution coordination, register traces, memory steps |
| Binary SM | sm-binary | Binary operations (AND, OR, XOR, shifts) |
| Arithmetic SM | sm-arith | Arithmetic operations (add, sub, mul, div) |
| Memory SM | sm-mem | Memory access operations and constraints |
| ROM SM | sm-rom | Read-only memory access patterns |

### Key Dependencies

The witness computation system integrates multiple specialized components:

- **Precompiles**: precomp-keccakf, precomp-sha256f, precomp-arith-eq for cryptographic operations
- **External Systems**: proofman-common, proofman-util, proofman-macros for proof management
- **Field Arithmetic**: p3-field, p3-goldilocks for finite field operations

## Execution Pipeline

ZisK transforms RISC-V programs into verifiable zero-knowledge proofs through a multi-stage pipeline:

### Execution Stages

1. **Program Conversion**: RISC-V ELF files are converted to ZiskRom format using the riscv2zisk transpiler
2. **Emulation**: The ziskemu emulator executes the program, generating detailed EmuTrace objects
3. **Witness Coordination**: The zisk-witness library processes traces through the sm-main coordinator
4. **Parallel Processing**: Specialized state machines handle different operation types simultaneously
5. **AIR Generation**: State machines produce Algebraic Intermediate Representation instances
6. **Proof Creation**: The proofman system converts AIR instances into zero-knowledge proofs

The data-bus component enables efficient communication between state machines during witness generation.

## State Machine Architecture

ZisK uses a system of interconnected state machines to process different aspects of program execution. These state machines collectively ensure that all operations are properly constrained and verifiable.

### Main State Machine (sm-main)

The sm-main package implements the central coordinator for witness generation. It processes execution traces from ziskemu and orchestrates interactions with specialized state machines.

Key dependencies:
- **ziskemu**: Receives execution traces
- **zisk-core**: Uses core types and definitions  
- **zisk-pil**: Applies constraint definitions
- **sm-mem**: Coordinates memory operations
- **asm-runner**: Handles assembly execution

The main state machine divides execution traces into segments for efficient parallel processing and manages the overall witness generation workflow.

### Specialized State Machines

#### Binary Operations (sm-binary)
Handles bitwise and logical operations through multiple sub-components:
- Binary basic operations (AND, OR, XOR)
- Binary extension operations (shifts, rotations)
- Lookup tables for operation verification

#### Arithmetic Operations (sm-arith)
Processes mathematical computations including:
- Addition and subtraction with overflow handling
- Multiplication and division operations
- Range checks and arithmetic constraints

#### Memory Management (sm-mem)
Ensures memory operation correctness through:
- Load and store operation tracking
- Memory consistency verification
- Address range validation

#### ROM Access (sm-rom)
Manages program memory access including:
- Instruction fetch operations
- Program counter management
- Read-only memory constraints

### Communication and Coordination

All state machines communicate through the data-bus component, which provides:

- **Operation Bus**: Central message passing system with OPERATION_BUS_ID
- **Parallel Processing**: Enables concurrent state machine execution
- **Constraint Synchronization**: Coordinates PIL constraint evaluation

The zisk-witness library serves as the top-level orchestrator, managing the flow from emulation traces to final AIR instances.

## Developer Workflow

Developers interact with ZisK through a structured workflow that abstracts the complexity of zero-knowledge proof generation:

### Core Development Tools

| Tool | Purpose | Package |
|------|---------|---------|
| ziskup | Toolchain installer and manager | Standalone installer |
| cargo-zisk | Command-line interface for ZisK projects | cli package |
| ziskemu | Emulator for testing and development | emulator package |

### Testing and Verification

ZisK includes comprehensive testing through the Riscof framework for RISC-V compliance:

1. **Test Generation**: Docker-based riscof test generation
2. **ELF Conversion**: Convert test ELF files to ZisK ROMs
3. **Execution**: Run tests through ziskemu
4. **Verification**: Compare output against reference RISC-V implementation

The testing process uses the ziskemu binary and specialized riscof configuration for automated compliance verification.

## Development Status

ZisK is currently under active development and is not yet ready for production use. The system is being refined and expanded to support more features and optimize performance.

Key points about the current status:

- The software is not fully tested
- It is not recommended for production environments
- Additional functionalities and optimizations are planned for future releases
- Future updates may introduce breaking changes

For the most up-to-date information on ZisK's development status, refer to the official repository and documentation.


