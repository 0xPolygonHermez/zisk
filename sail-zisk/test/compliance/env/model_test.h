/* The RISC-V architectural tests' model hooks for ZisK. From ziskof's
 * zisk/env/model_test.h, with the marker read at its current address. The
 * halt copies the signature into tohost and, on ZisK, exits through the exit
 * syscall (a7 = 93). */
#ifndef _COMPLIANCE_MODEL_H
#define _COMPLIANCE_MODEL_H
#define RVMODEL_DATA_SECTION \
        .pushsection .tohost,"aw",@progbits;                            \
        .align 8; .global tohost; tohost: .dword 0;                     \
        .align 8; .global fromhost; fromhost: .dword 0;                 \
        .popsection;                                                    \
        .align 8; .global begin_regstate; begin_regstate:               \
        .word 128;                                                      \
        .align 8; .global end_regstate; end_regstate:                   \
        .word 4;

/* ZisK's marker: the BIOS stores ARCH_ID_ZISK at ARCH_ID_CSR_ADDR, marchid's
 * slot in the CSR area (CSR_ADDR + 0xF12 * 8, core/src/mem.rs). */
#define ARCH_ID_ZISK 0x0FFFEEEE
#define ARCH_ID_CSR_ADDR 0xa040f890
#define QEMU_EXIT_ADDR 0x100000
#define QEMU_EXIT_CODE 0x5555


//RV_COMPLIANCE_HALT
#define RVMODEL_HALT                                                          \
    la t0, begin_signature;                                                     \
    la t1, end_signature; \
    la t2, tohost; \
    sub t3, t1, t0; \
    srai t3, t3, 2; \
    sw t3, 0(t2); \
    addi t2, t2, 4; \
  next: \
    bge t0, t1, end; \
    lw t4, 0(t0);    \
    sw t4, 0(t2);       \
    addi t2, t2, 4;   \
    addi t0, t0, 4;    \
    j next;   \
  end: \
    li t1, ARCH_ID_CSR_ADDR; \
    lw t0, (t1); \
    li   t1, ARCH_ID_ZISK; \
    beq t0, t1, zisk_exit; \
  qemu_exit: \
    li t0, QEMU_EXIT_ADDR; \
    li t1, QEMU_EXIT_CODE; \
    sw t1, 0(t0); \
    j loop; \
  zisk_exit: \
    li   a7, 93; \
    ecall; \
  loop: \
    j loop;

#define RVMODEL_BOOT

//RV_COMPLIANCE_DATA_BEGIN
#define RVMODEL_DATA_BEGIN                                              \
  RVMODEL_DATA_SECTION                                                        \
  .align 4;\
  .global begin_signature; begin_signature:

//RV_COMPLIANCE_DATA_END
#define RVMODEL_DATA_END                                                      \
  .align 4;\
  .global end_signature; end_signature:

//RVTEST_IO_INIT
#define RVMODEL_IO_INIT
//RVTEST_IO_WRITE_STR
#define RVMODEL_IO_WRITE_STR(_R, _STR)
//RVTEST_IO_CHECK
#define RVMODEL_IO_CHECK()
//RVTEST_IO_ASSERT_GPR_EQ
#define RVMODEL_IO_ASSERT_GPR_EQ(_S, _R, _I)
//RVTEST_IO_ASSERT_SFPR_EQ
#define RVMODEL_IO_ASSERT_SFPR_EQ(_F, _R, _I)
//RVTEST_IO_ASSERT_DFPR_EQ
#define RVMODEL_IO_ASSERT_DFPR_EQ(_D, _R, _I)

#define RVMODEL_SET_MSW_INT       \
 li t1, 1;                         \
 li t2, 0x2000000;                 \
 sw t1, 0(t2);

#define RVMODEL_CLEAR_MSW_INT     \
 li t2, 0x2000000;                 \
 sw x0, 0(t2);

#define RVMODEL_CLEAR_MTIMER_INT

#define RVMODEL_CLEAR_MEXT_INT


#endif // _COMPLIANCE_MODEL_H
