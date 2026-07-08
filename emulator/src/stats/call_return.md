# Call and Return detection (callstack)

### CALL jal rd, imm   
- rd: r1(ra)|r5(t0)
- transpile => 1 zisk instruction
    - [pc]: **FLAG**  store_pc=1 | set_pc=0 | next_pc = imm | A=0 | B=0 | RD = PC + jmp_offset2
- **CONDITION**: op = **FLAG** | store_pc = 1 | set_pc = 0 | change_roi(pc+jmp_offset1) | jmp_offset1 == imm | same_roi(pc + jmp_offset2)

### CALL jalr rd, rs1, imm   
- rd: r1(ra)|r5(t0) rs1:*
- transpile [imm % 2 == 0] => 1 zisk instruction
    - [pc]: **AND**  store_pc=1 | set_pc=1 | next_pc = (RS1 & MASK) + jmp_offset1 | A=~1 | RD = PC + jmp_offset2;
    - **condition**: op = **AND** | store_pc = 1 | set_pc = 1 | change_roi(c + jmp_offset1) | jmp_offset1 == ??? | !change_roi(pc + jmp_offset1)

- transpile [imm % 2 == 1] => 2 zisk instructions
  - [pc]:  **ADD**  store_pc=0 | set_pc=0 | C = IMM + RS1
  - [ipc]: **AND**  store_pc=1 | set_pc=1 | next_pc = (RS1 & MASK) + jmp_offset1 | A=~1 | RD = PC + jmp_offset2;
  - **condition**: op = **AND** | store_pc = 1 | set_pc = 1 | change_roi(c + jmp_offset1) | jmp_offset1 == 0 | !change_roi(pc + jmp_offset1)

### RETURN jalr x0, rs1, imm     
- rd:x0 rs1:x1|x5
- transpile [imm % 2 == 0] => 1 zisk instruction
    - [pc]: AND  store_pc=0 | set_pc=1 | next_pc = (RS1 & MASK) + jmp_offset1 | A=~1
    - **condition**: op = **AND** | store_pc = 0 | set_pc = 1 | change_roi(c + jmp_offset1) | jmp_offset1 == ?? | jmp_offset2 = 2|4;

- transpile [imm % 2 == 1] => 2 instruction
    - [pc]: **ADD**  store_pc=0 | set_pc=0 | C = IMM + RS1
    - [ipc]: **AND**  store_pc=0 | set_pc=1 | next_pc = (RS1 & MASK) + jmp_offset1 | A=~1
    - **condition**: op = **AND** | store_pc = 0 | set_pc = 1 | change_roi(c) | jmp_offset1 == 0 | jmp_offset2 = 2|4
- **CONDITION**: op = **AND** | store_pc = 0 | set_pc = 1 | change_roi(c + jmp_offset1) | jmp_offset2 = 2|4 | !change_roi(pc + jmp_offset2)

### CALL auipc rd, imm0 +  jalr rd, x1, imm1     
- rd:x1(ra)|x5(t0) rs1:? 
- [pc] COPYB RD=X1 store_pc=0 | set_pc=0 | IMM = PC .....  | X1 = RETURN_ADDR | jmp_offset1 == jmp_offset2 ROI_DIFF(jmp_offset1 + PC)
- **CONDITION**: op = **COPYB** | store_pc=0 | set_pc=0 | B = pc + 2|4 | jmp_offset1 == jmp_offset2 | change_roi(pc + jmp_offset1)

### RETURN auipc rd, imm0 + jalr x0, rd, imm1     
- rd:x0 rs1:x1|x5
- [pc] COPYB store_pc=0 | set_pc=0 | IMM = PC .....  | jmp_offset1 == jmp_offset2 ROI_DIFF(jmp_offset1 + PC)
- **CONDITION**: op = **COPYB** | store_pc=0 | set_pc=0 | B = pc + 2|4 | jmp_offset1 == jmp_offset2 | change_roi(pc + jmp_offset1)

## Summary

| type | # | op | store | store offset |store pc | set_pc | change_roi | same_roi | others |
|-----| ---|-----|--|---|---|---|---|---|---|
| CALL | 1 | FLAG | REG | 1,5 |**1** | 0 | pc + jmp_offset1 | pc + jmp_offset2| A=0 && B=0
| CALL | 1 | AND | REG | 1,5 | **1** | **1** | c + jmp_offset1 | pc + jmp_offset1 ||
| CALL | **2** | AND | REG | 1,5 | **1** | **1** | c + jmp_offset1 | pc + jmp_offset1 ||
| CALL | 1 | COPYB | REG | 1,5 | 0 | 0 | pc +  jmp_offset2 | B | jmp_offset1 == jmp_offset2 | B=pc+[2,4]
| RETURN | 1 | AND | NONE | 0 | 0 | **1** | c + jmp_offset1 | |  | 2,4 | 
| RETURN | **2** | AND | NONE | 0 | 0 | **1** | c + jmp_offset1 | |  | | 
| RETURN | 1 | COPYB | NONE | 0 | 0 | 0 | pc + jmp_offset2 |  | jmp_offset1 == jmp_offset2 |
