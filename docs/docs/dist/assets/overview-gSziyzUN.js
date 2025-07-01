import{f as t,j as e}from"./index-DBVKNYZ6.js";const a={description:"ZisK Overview",title:"ZisK Overview"};function s(i){const n={a:"a",div:"div",h1:"h1",h2:"h2",h3:"h3",h4:"h4",header:"header",img:"img",li:"li",ol:"ol",p:"p",strong:"strong",table:"table",tbody:"tbody",td:"td",th:"th",thead:"thead",tr:"tr",ul:"ul",...t(),...i.components};return e.jsxs(e.Fragment,{children:[e.jsx(n.header,{children:e.jsxs(n.h1,{id:"zisk-overview",children:["ZisK Overview",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#zisk-overview",children:e.jsx(n.div,{"data-autolink-icon":!0})})]})}),`
`,e.jsx(n.p,{children:"ZisK is a high-performance zero-knowledge virtual machine (zkVM) designed to generate zero-knowledge proofs of arbitrary program execution. It enables developers to prove the correctness of computations without revealing their internal state. ZisK abstracts the complexities of cryptographic operations by providing an optimized toolstack that minimizes computational overhead, making zero-knowledge technology accessible to developers."}),`
`,e.jsx(n.p,{children:"Key features of ZisK include:"}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsxs(n.li,{children:[e.jsx(n.strong,{children:"High-performance architecture"})," optimized for low-latency proof generation"]}),`
`,e.jsxs(n.li,{children:[e.jsx(n.strong,{children:"Rust-based zkVM"})," with future support for additional languages"]}),`
`,e.jsxs(n.li,{children:[e.jsx(n.strong,{children:"No recompilation required"})," across different programs"]}),`
`,e.jsxs(n.li,{children:[e.jsx(n.strong,{children:"Standardized prover interface"})," (JSON-RPC, GRPC, CLI)"]}),`
`,e.jsxs(n.li,{children:[e.jsx(n.strong,{children:"Flexible integration"})," as standalone service or library"]}),`
`,e.jsxs(n.li,{children:[e.jsx(n.strong,{children:"Fully open-source"}),", backed by Polygon zkEVM and Plonky3 technology"]}),`
`]}),`
`,e.jsx(n.p,{children:"ZisK converts RISC-V programs into provable form through a sophisticated pipeline involving emulation, witness generation, and constraint satisfaction."}),`
`,e.jsxs(n.h2,{id:"system-architecture-overview",children:["System Architecture Overview",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#system-architecture-overview",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:"ZisK consists of several interconnected components that work together to convert programs into provable form, execute them, generate witnesses, and create zero-knowledge proofs."}),`
`,e.jsx(n.p,{children:e.jsx(n.img,{src:"/zisk-system.png",alt:"ZisK System Architecture"})}),`
`,e.jsxs(n.h3,{id:"high-level-system-architecture",children:["High-Level System Architecture",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#high-level-system-architecture",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:"The ZisK system is organized into several key areas:"}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsxs(n.li,{children:[e.jsx(n.strong,{children:"Infrastructure"}),": Core VM system and development toolchain"]}),`
`,e.jsxs(n.li,{children:[e.jsx(n.strong,{children:"Input Processing"}),": RISC-V ELF files and transpilation"]}),`
`,e.jsxs(n.li,{children:[e.jsx(n.strong,{children:"Specialized State Machines"}),": Parallel processing of different operation types"]}),`
`,e.jsxs(n.li,{children:[e.jsx(n.strong,{children:"Witness Generation"}),": Coordination and AIR instance creation"]}),`
`,e.jsxs(n.li,{children:[e.jsx(n.strong,{children:"Core VM System"}),": Fundamental execution and constraint handling"]}),`
`]}),`
`,e.jsxs(n.h3,{id:"core-components",children:["Core Components",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#core-components",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsxs(n.table,{children:[e.jsx(n.thead,{children:e.jsxs(n.tr,{children:[e.jsx(n.th,{children:"Component"}),e.jsx(n.th,{children:"Package"}),e.jsx(n.th,{children:"Purpose"})]})}),e.jsxs(n.tbody,{children:[e.jsxs(n.tr,{children:[e.jsx(n.td,{children:"zisk-core"}),e.jsx(n.td,{children:"zisk-core"}),e.jsx(n.td,{children:"Fundamental definitions, types, and operations. Contains ZiskRom, ZiskInst, and instruction set definitions"})]}),e.jsxs(n.tr,{children:[e.jsx(n.td,{children:"ziskemu"}),e.jsx(n.td,{children:"ziskemu"}),e.jsx(n.td,{children:"RISC-V/ZisK emulator that executes programs and generates execution traces"})]}),e.jsxs(n.tr,{children:[e.jsx(n.td,{children:"zisk-witness"}),e.jsx(n.td,{children:"zisk-witness"}),e.jsx(n.td,{children:"Witness generation library that coordinates between emulator and state machines"})]}),e.jsxs(n.tr,{children:[e.jsx(n.td,{children:"zisk-pil"}),e.jsx(n.td,{children:"zisk-pil"}),e.jsx(n.td,{children:"Polynomial Identity Language definitions for mathematical constraints"})]}),e.jsxs(n.tr,{children:[e.jsx(n.td,{children:"executor"}),e.jsx(n.td,{children:"executor"}),e.jsx(n.td,{children:"Execution orchestrator that manages the overall proof generation process"})]}),e.jsxs(n.tr,{children:[e.jsx(n.td,{children:"data-bus"}),e.jsx(n.td,{children:"data-bus"}),e.jsx(n.td,{children:"Inter-component communication system for state machine coordination"})]}),e.jsxs(n.tr,{children:[e.jsx(n.td,{children:"zisk-common"}),e.jsx(n.td,{children:"zisk-common"}),e.jsx(n.td,{children:"Shared utilities and common functionality across components"})]})]})]}),`
`,e.jsxs(n.h3,{id:"state-machine-packages",children:["State Machine Packages",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#state-machine-packages",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsxs(n.table,{children:[e.jsx(n.thead,{children:e.jsxs(n.tr,{children:[e.jsx(n.th,{children:"State Machine"}),e.jsx(n.th,{children:"Package"}),e.jsx(n.th,{children:"Operations Handled"})]})}),e.jsxs(n.tbody,{children:[e.jsxs(n.tr,{children:[e.jsx(n.td,{children:"Main SM"}),e.jsx(n.td,{children:"sm-main"}),e.jsx(n.td,{children:"Execution coordination, register traces, memory steps"})]}),e.jsxs(n.tr,{children:[e.jsx(n.td,{children:"Binary SM"}),e.jsx(n.td,{children:"sm-binary"}),e.jsx(n.td,{children:"Binary operations (AND, OR, XOR, shifts)"})]}),e.jsxs(n.tr,{children:[e.jsx(n.td,{children:"Arithmetic SM"}),e.jsx(n.td,{children:"sm-arith"}),e.jsx(n.td,{children:"Arithmetic operations (add, sub, mul, div)"})]}),e.jsxs(n.tr,{children:[e.jsx(n.td,{children:"Memory SM"}),e.jsx(n.td,{children:"sm-mem"}),e.jsx(n.td,{children:"Memory access operations and constraints"})]}),e.jsxs(n.tr,{children:[e.jsx(n.td,{children:"ROM SM"}),e.jsx(n.td,{children:"sm-rom"}),e.jsx(n.td,{children:"Read-only memory access patterns"})]})]})]}),`
`,e.jsxs(n.h3,{id:"key-dependencies",children:["Key Dependencies",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#key-dependencies",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:"The witness computation system integrates multiple specialized components:"}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsxs(n.li,{children:[e.jsx(n.strong,{children:"Precompiles"}),": precomp-keccakf, precomp-sha256f, precomp-arith-eq for cryptographic operations"]}),`
`,e.jsxs(n.li,{children:[e.jsx(n.strong,{children:"External Systems"}),": proofman-common, proofman-util, proofman-macros for proof management"]}),`
`,e.jsxs(n.li,{children:[e.jsx(n.strong,{children:"Field Arithmetic"}),": p3-field, p3-goldilocks for finite field operations"]}),`
`]}),`
`,e.jsxs(n.h2,{id:"execution-pipeline",children:["Execution Pipeline",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#execution-pipeline",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:"ZisK transforms RISC-V programs into verifiable zero-knowledge proofs through a multi-stage pipeline:"}),`
`,e.jsxs(n.h3,{id:"execution-stages",children:["Execution Stages",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#execution-stages",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsxs(n.ol,{children:[`
`,e.jsxs(n.li,{children:[e.jsx(n.strong,{children:"Program Conversion"}),": RISC-V ELF files are converted to ZiskRom format using the riscv2zisk transpiler"]}),`
`,e.jsxs(n.li,{children:[e.jsx(n.strong,{children:"Emulation"}),": The ziskemu emulator executes the program, generating detailed EmuTrace objects"]}),`
`,e.jsxs(n.li,{children:[e.jsx(n.strong,{children:"Witness Coordination"}),": The zisk-witness library processes traces through the sm-main coordinator"]}),`
`,e.jsxs(n.li,{children:[e.jsx(n.strong,{children:"Parallel Processing"}),": Specialized state machines handle different operation types simultaneously"]}),`
`,e.jsxs(n.li,{children:[e.jsx(n.strong,{children:"AIR Generation"}),": State machines produce Algebraic Intermediate Representation instances"]}),`
`,e.jsxs(n.li,{children:[e.jsx(n.strong,{children:"Proof Creation"}),": The proofman system converts AIR instances into zero-knowledge proofs"]}),`
`]}),`
`,e.jsx(n.p,{children:"The data-bus component enables efficient communication between state machines during witness generation."}),`
`,e.jsxs(n.h2,{id:"state-machine-architecture",children:["State Machine Architecture",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#state-machine-architecture",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:"ZisK uses a system of interconnected state machines to process different aspects of program execution. These state machines collectively ensure that all operations are properly constrained and verifiable."}),`
`,e.jsxs(n.h3,{id:"main-state-machine-sm-main",children:["Main State Machine (sm-main)",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#main-state-machine-sm-main",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:"The sm-main package implements the central coordinator for witness generation. It processes execution traces from ziskemu and orchestrates interactions with specialized state machines."}),`
`,e.jsx(n.p,{children:"Key dependencies:"}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsxs(n.li,{children:[e.jsx(n.strong,{children:"ziskemu"}),": Receives execution traces"]}),`
`,e.jsxs(n.li,{children:[e.jsx(n.strong,{children:"zisk-core"}),": Uses core types and definitions"]}),`
`,e.jsxs(n.li,{children:[e.jsx(n.strong,{children:"zisk-pil"}),": Applies constraint definitions"]}),`
`,e.jsxs(n.li,{children:[e.jsx(n.strong,{children:"sm-mem"}),": Coordinates memory operations"]}),`
`,e.jsxs(n.li,{children:[e.jsx(n.strong,{children:"asm-runner"}),": Handles assembly execution"]}),`
`]}),`
`,e.jsx(n.p,{children:"The main state machine divides execution traces into segments for efficient parallel processing and manages the overall witness generation workflow."}),`
`,e.jsxs(n.h3,{id:"specialized-state-machines",children:["Specialized State Machines",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#specialized-state-machines",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsxs(n.h4,{id:"binary-operations-sm-binary",children:["Binary Operations (sm-binary)",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#binary-operations-sm-binary",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:"Handles bitwise and logical operations through multiple sub-components:"}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsx(n.li,{children:"Binary basic operations (AND, OR, XOR)"}),`
`,e.jsx(n.li,{children:"Binary extension operations (shifts, rotations)"}),`
`,e.jsx(n.li,{children:"Lookup tables for operation verification"}),`
`]}),`
`,e.jsxs(n.h4,{id:"arithmetic-operations-sm-arith",children:["Arithmetic Operations (sm-arith)",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#arithmetic-operations-sm-arith",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:"Processes mathematical computations including:"}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsx(n.li,{children:"Addition and subtraction with overflow handling"}),`
`,e.jsx(n.li,{children:"Multiplication and division operations"}),`
`,e.jsx(n.li,{children:"Range checks and arithmetic constraints"}),`
`]}),`
`,e.jsxs(n.h4,{id:"memory-management-sm-mem",children:["Memory Management (sm-mem)",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#memory-management-sm-mem",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:"Ensures memory operation correctness through:"}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsx(n.li,{children:"Load and store operation tracking"}),`
`,e.jsx(n.li,{children:"Memory consistency verification"}),`
`,e.jsx(n.li,{children:"Address range validation"}),`
`]}),`
`,e.jsxs(n.h4,{id:"rom-access-sm-rom",children:["ROM Access (sm-rom)",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#rom-access-sm-rom",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:"Manages program memory access including:"}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsx(n.li,{children:"Instruction fetch operations"}),`
`,e.jsx(n.li,{children:"Program counter management"}),`
`,e.jsx(n.li,{children:"Read-only memory constraints"}),`
`]}),`
`,e.jsxs(n.h3,{id:"communication-and-coordination",children:["Communication and Coordination",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#communication-and-coordination",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:"All state machines communicate through the data-bus component, which provides:"}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsxs(n.li,{children:[e.jsx(n.strong,{children:"Operation Bus"}),": Central message passing system with OPERATION_BUS_ID"]}),`
`,e.jsxs(n.li,{children:[e.jsx(n.strong,{children:"Parallel Processing"}),": Enables concurrent state machine execution"]}),`
`,e.jsxs(n.li,{children:[e.jsx(n.strong,{children:"Constraint Synchronization"}),": Coordinates PIL constraint evaluation"]}),`
`]}),`
`,e.jsx(n.p,{children:"The zisk-witness library serves as the top-level orchestrator, managing the flow from emulation traces to final AIR instances."}),`
`,e.jsxs(n.h2,{id:"developer-workflow",children:["Developer Workflow",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#developer-workflow",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:"Developers interact with ZisK through a structured workflow that abstracts the complexity of zero-knowledge proof generation:"}),`
`,e.jsxs(n.h3,{id:"core-development-tools",children:["Core Development Tools",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#core-development-tools",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsxs(n.table,{children:[e.jsx(n.thead,{children:e.jsxs(n.tr,{children:[e.jsx(n.th,{children:"Tool"}),e.jsx(n.th,{children:"Purpose"}),e.jsx(n.th,{children:"Package"})]})}),e.jsxs(n.tbody,{children:[e.jsxs(n.tr,{children:[e.jsx(n.td,{children:"ziskup"}),e.jsx(n.td,{children:"Toolchain installer and manager"}),e.jsx(n.td,{children:"Standalone installer"})]}),e.jsxs(n.tr,{children:[e.jsx(n.td,{children:"cargo-zisk"}),e.jsx(n.td,{children:"Command-line interface for ZisK projects"}),e.jsx(n.td,{children:"cli package"})]}),e.jsxs(n.tr,{children:[e.jsx(n.td,{children:"ziskemu"}),e.jsx(n.td,{children:"Emulator for testing and development"}),e.jsx(n.td,{children:"emulator package"})]})]})]}),`
`,e.jsxs(n.h3,{id:"testing-and-verification",children:["Testing and Verification",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#testing-and-verification",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:"ZisK includes comprehensive testing through the Riscof framework for RISC-V compliance:"}),`
`,e.jsxs(n.ol,{children:[`
`,e.jsxs(n.li,{children:[e.jsx(n.strong,{children:"Test Generation"}),": Docker-based riscof test generation"]}),`
`,e.jsxs(n.li,{children:[e.jsx(n.strong,{children:"ELF Conversion"}),": Convert test ELF files to ZisK ROMs"]}),`
`,e.jsxs(n.li,{children:[e.jsx(n.strong,{children:"Execution"}),": Run tests through ziskemu"]}),`
`,e.jsxs(n.li,{children:[e.jsx(n.strong,{children:"Verification"}),": Compare output against reference RISC-V implementation"]}),`
`]}),`
`,e.jsx(n.p,{children:"The testing process uses the ziskemu binary and specialized riscof configuration for automated compliance verification."}),`
`,e.jsxs(n.h2,{id:"development-status",children:["Development Status",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#development-status",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:"ZisK is currently under active development and is not yet ready for production use. The system is being refined and expanded to support more features and optimize performance."}),`
`,e.jsx(n.p,{children:"Key points about the current status:"}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsx(n.li,{children:"The software is not fully tested"}),`
`,e.jsx(n.li,{children:"It is not recommended for production environments"}),`
`,e.jsx(n.li,{children:"Additional functionalities and optimizations are planned for future releases"}),`
`,e.jsx(n.li,{children:"Future updates may introduce breaking changes"}),`
`]}),`
`,e.jsx(n.p,{children:"For the most up-to-date information on ZisK's development status, refer to the official repository and documentation."})]})}function o(i={}){const{wrapper:n}={...t(),...i.components};return n?e.jsx(n,{...i,children:e.jsx(s,{...i})}):s(i)}export{o as default,a as frontmatter};
