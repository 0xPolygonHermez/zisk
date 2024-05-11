# zkASM Components

## Registers

A **register** is a location available to the zkEVM that is manipulated through the zkEVM's instructions. Registers are of different types, some of them being of generic purpose and others being specific purpose. They are also of different sizes, represented as arrays of Goldilocks prime field numbers, i.e., in the range $[0,2^{64} - 2^{32} + 1]$.

### A, B, C, D, E
- Generic purpose.
- Arrays of 8 elements `[V0, V1,..., V7]`.

### SR
- Represents the State Root.
- An array of 8 elements `[V0, V1,..., V7]`.

### CTX
- Represents the ConTeXt. Its main use is being able to move through the zkEVM's memory.
- Array of 1 element `[V]`.

### SP
- Represents the Stack Pointer. Its main use is being able to move through the zkEVM's memory.
- Array of 1 element `[V]`.

### PC
- Represents the Program Counter. Its main use is being able to move through the zkEVM's memory.
- Array of 1 element `[V]`.

### zkPC
- Represents the zk Program Counter.
- Array of 1 element `[V]`.

### RR
- Return Register.
- Saves the origin `zkPC` in `RR` when a `CALL` instruction is performed. The `RETURN` instruction loads `RR` in `zkPC`.
- Array of 1 element `[V]`.

### STEP
- Represents the number of instructions performed within the program.
- Array of 1 element `[V]`.

### RCX
- Used to repeat instructions.
- Array of 1 element `[V]`.

## Instructions

### MLOAD(addr)

op = mem(addr)

addr = SP | SP++ | SP-- | SP+offset | SP-offset | SYS:E+offset | SYS:E+offset | SYS:E-offset | SYS:E | MEM:E | MEM:E+offset | MEM:E-offset | STACK:E | STACK:E+offset | STACK:E-offset | variable | variable + E | variable + E

### MSTORE(addr)

mem(addr) = op

### JMP (jmpaddr)

zkPC' = jmpaddr
jmpaddr = label | RR | E | reference + E | reference + RR
reference = @label

### JMPN/JMPC/JMPZ/JMPNC/JMPNZ (jmpaddr[,elseaddr])

JMPN: jump if op[0] was negative
JMPC: jump if carry bit, only use with binary operations
JMPZ: jump if op[0] was zero
JMPNC: jump if no carry bit, only use with binary operations
JMPNZ: jump if op[0] was different of zero

### CALL (calladdr)

calladdr = label | reference + RR | reference + E
RR' = zkPC + 1
JMP(calladdr)

### RETURN

JMP(RR)

### ROTL_C
Rotate the `C = [C[0], C[1], ..., C[6]]` register to the left:
  ```
  [op[0], op[1], ..., op[7]]= [C[7], C[0], ..., C[6]].
  ```

### REPEAT(RCX)

RCX != 0 => RCX' = RCX - 1
RCX != 0 => zkPC = zkPC
REPEAT was executed at least one time

## Constants

CONST, CONSTL %constname = expression

define constants
const set lsr (op0) and reset the rest (op1,....,op7)
constl set 8 registers (op0, op1, op2, ..,op7)

# zkASM Compiler
Compiles zkasm file to a json ready for the zkExecutor

## Setup

```sh
$ npm install
$ npm run build
```
## Usage
Generate json file from zkasm file:
```sh
$ node src/zkasm.js <input.zkasm> -o <output.json>
```
For test purposes (partial inclusion of files):
- allowUndefinedLabels: Allows to leave labels undefined.
- allowOverwriteLabels: Allows to overwrite labels.
- allowUndefinedVariables: Allows to leave variables without declaration (undefined)

```sh
node src/zkasm.js <input.zkasm> -o <output.json> -t allowUndefinedLabels -t allowOverwriteLabels -t allowUndefinedVariables
```