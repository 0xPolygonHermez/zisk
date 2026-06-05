    .global BLS12_381_384_rawAdd
    .global BLS12_381_384_rawAddLS
    .global BLS12_381_384_rawSub
    .global BLS12_381_384_rawSubRegular
    .global BLS12_381_384_rawNeg
    .global BLS12_381_384_rawNegLS
    .global BLS12_381_384_rawSubSL
    .global BLS12_381_384_rawSubLS
    .global BLS12_381_384_rawMMul
    .global BLS12_381_384_rawMMul1
    .global BLS12_381_384_rawFromMontgomery
    .global BLS12_381_384_rawCopy
    .global BLS12_381_384_rawSwap
    .global BLS12_381_384_rawIsEq
    .global BLS12_381_384_rawIsZero
    .global BLS12_381_384_rawCopyS2L
    .global BLS12_381_384_rawCmp
    .global BLS12_381_384_rawAnd
    .global BLS12_381_384_rawOr
    .global BLS12_381_384_rawXor
    .global BLS12_381_384_rawShr
    .global BLS12_381_384_rawShl
    .global BLS12_381_384_rawNot

    .global _BLS12_381_384_rawAdd
    .global _BLS12_381_384_rawAddLS
    .global _BLS12_381_384_rawSub
    .global _BLS12_381_384_rawSubRegular
    .global _BLS12_381_384_rawNeg
    .global _BLS12_381_384_rawNegLS
    .global _BLS12_381_384_rawSubSL
    .global _BLS12_381_384_rawSubLS
    .global _BLS12_381_384_rawMMul
    .global _BLS12_381_384_rawMMul1
    .global _BLS12_381_384_rawFromMontgomery
    .global _BLS12_381_384_rawCopy
    .global _BLS12_381_384_rawSwap
    .global _BLS12_381_384_rawIsEq
    .global _BLS12_381_384_rawIsZero
    .global _BLS12_381_384_rawCopyS2L
    .global _BLS12_381_384_rawCmp
    .global _BLS12_381_384_rawAnd
    .global _BLS12_381_384_rawOr
    .global _BLS12_381_384_rawXor
    .global _BLS12_381_384_rawShr
    .global _BLS12_381_384_rawShl
    .global _BLS12_381_384_rawNot

    .text
    .align 4

BLS12_381_384_rawAdd:
_BLS12_381_384_rawAdd:
        stp   x19, x20, [sp, #-16]!

        ldp    x8,  x9, [x1]
        ldp    x4,  x5, [x2]
        adds   x8,  x8,  x4
        adcs   x9,  x9,  x5

        ldp   x10, x11, [x1, 16]
        ldp    x6,  x7, [x2, 16]
        adcs  x10, x10,  x6
        adcs  x11, x11,  x7

        ldp   x12, x13, [x1, 32]
        ldp    x4,  x5, [x2, 32]
        adcs  x12, x12,  x4
        adcs  x13, x13,  x5

        cset   x2,  cs

        adr    x3, BLS12_381_384_rawq
        ldp   x14, x15, [x3]
        subs  x14,  x8, x14
        sbcs  x15,  x9, x15

        ldp   x16, x17, [x3, 16]
        sbcs  x16, x10, x16
        sbcs  x17, x11, x17

        ldp   x19, x20, [x3, 32]
        sbcs  x19, x12, x19
        sbcs  x20, x13, x20

        cbnz   x2, BLS12_381_384_rawAdd_done_s
        b.hs  BLS12_381_384_rawAdd_done_s

        stp    x8,  x9, [x0]
        stp   x10, x11, [x0, 16]
        stp   x12, x13, [x0, 32]

        b     BLS12_381_384_rawAdd_out

BLS12_381_384_rawAdd_done_s:
        stp   x14, x15, [x0]
        stp   x16, x17, [x0, 16]
        stp   x19, x20, [x0, 32]

BLS12_381_384_rawAdd_out:

        ldp   x19, x20, [sp], #16
        ret


BLS12_381_384_rawAddLS:
_BLS12_381_384_rawAddLS:
        stp   x19, x20, [sp, #-16]!

        ldp    x8,  x9, [x1]
        adds   x8,  x8,  x2
        adcs   x9,  x9, xzr

        ldp   x10, x11, [x1, 16]
        adcs  x10, x10, xzr
        adcs  x11, x11, xzr

        ldp   x12, x13, [x1, 32]
        adcs  x12, x12, xzr
        adcs  x13, x13, xzr

        cset   x2,  cs

        adr    x3, BLS12_381_384_rawq
        ldp   x14, x15, [x3]
        subs  x14,  x8, x14
        sbcs  x15,  x9, x15

        ldp   x16, x17, [x3, 16]
        sbcs  x16, x10, x16
        sbcs  x17, x11, x17

        ldp   x19, x20, [x3, 32]
        sbcs  x19, x12, x19
        sbcs  x20, x13, x20

        cbnz   x2, BLS12_381_384_rawAddLS_done_s
        b.hs  BLS12_381_384_rawAddLS_done_s

        stp    x8,  x9, [x0]
        stp   x10, x11, [x0, 16]
        stp   x12, x13, [x0, 32]

        b     BLS12_381_384_rawAddLS_out

BLS12_381_384_rawAddLS_done_s:
        stp   x14, x15, [x0]
        stp   x16, x17, [x0, 16]
        stp   x19, x20, [x0, 32]

BLS12_381_384_rawAddLS_out:

        ldp   x19, x20, [sp], #16
        ret


BLS12_381_384_rawSub:
_BLS12_381_384_rawSub:
        stp   x19, x20, [sp, #-16]!

        ldp    x8,  x9, [x1]
        ldp    x4,  x5, [x2]
        subs   x8,  x8,  x4
        sbcs   x9,  x9,  x5

        ldp   x10, x11, [x1, 16]
        ldp    x6,  x7, [x2, 16]
        sbcs  x10, x10,  x6
        sbcs  x11, x11,  x7

        ldp   x12, x13, [x1, 32]
        ldp    x4,  x5, [x2, 32]
        sbcs  x12, x12,  x4
        sbcs  x13, x13,  x5

        b.cs  BLS12_381_384_rawSub_done

        adr    x3, BLS12_381_384_rawq
        ldp    x4,  x5, [x3]
        adds   x8,  x8,  x4
        adcs   x9,  x9,  x5

        ldp    x6,  x7, [x3, 16]
        adcs  x10, x10,  x6
        adcs  x11, x11,  x7

        ldp    x4,  x5, [x3, 32]
        adcs  x12, x12,  x4
        adcs  x13, x13,  x5

BLS12_381_384_rawSub_done:
        stp    x8,  x9, [x0]
        stp   x10, x11, [x0, 16]
        stp   x12, x13, [x0, 32]

        ldp   x19, x20, [sp], #16
        ret


BLS12_381_384_rawSubSL:
_BLS12_381_384_rawSubSL:
        stp   x19, x20, [sp, #-16]!

        ldp    x8,  x9, [x2]
        subs   x8,  x1,  x8
        sbcs   x9, xzr,  x9

        ldp   x10, x11, [x2, 16]
        sbcs  x10, xzr, x10
        sbcs  x11, xzr, x11

        ldp   x12, x13, [x2, 32]
        sbcs  x12, xzr, x12
        sbcs  x13, xzr, x13

        b.cs  BLS12_381_384_rawSubSL_done

        adr    x3, BLS12_381_384_rawq
        ldp    x4,  x5, [x3]
        adds   x8,  x8,  x4
        adcs   x9,  x9,  x5

        ldp    x6,  x7, [x3, 16]
        adcs  x10, x10,  x6
        adcs  x11, x11,  x7

        ldp    x4,  x5, [x3, 32]
        adcs  x12, x12,  x4
        adcs  x13, x13,  x5

BLS12_381_384_rawSubSL_done:
        stp    x8,  x9, [x0]
        stp   x10, x11, [x0, 16]
        stp   x12, x13, [x0, 32]

        ldp   x19, x20, [sp], #16
        ret


BLS12_381_384_rawSubLS:
_BLS12_381_384_rawSubLS:
        stp   x19, x20, [sp, #-16]!

        ldp    x8,  x9, [x1]
        subs   x8,  x8,  x2
        sbcs   x9,  x9, xzr

        ldp   x10, x11, [x1, 16]
        sbcs  x10, x10, xzr
        sbcs  x11, x11, xzr

        ldp   x12, x13, [x1, 32]
        sbcs  x12, x12, xzr
        sbcs  x13, x13, xzr

        b.cs  BLS12_381_384_rawSubLS_done

        adr    x3, BLS12_381_384_rawq
        ldp    x4,  x5, [x3]
        adds   x8,  x8,  x4
        adcs   x9,  x9,  x5

        ldp    x6,  x7, [x3, 16]
        adcs  x10, x10,  x6
        adcs  x11, x11,  x7

        ldp    x4,  x5, [x3, 32]
        adcs  x12, x12,  x4
        adcs  x13, x13,  x5

BLS12_381_384_rawSubLS_done:
        stp    x8,  x9, [x0]
        stp   x10, x11, [x0, 16]
        stp   x12, x13, [x0, 32]

        ldp   x19, x20, [sp], #16
        ret


BLS12_381_384_rawSubRegular:
_BLS12_381_384_rawSubRegular:
        ldp    x4,  x5, [x1]
        ldp    x8,  x9, [x2]
        subs   x4,  x4,  x8
        sbcs   x5,  x5,  x9
        stp    x4,  x5, [x0]

        ldp    x6,  x7, [x1, 16]
        ldp   x10, x11, [x2, 16]
        sbcs   x6,  x6, x10
        sbcs   x7,  x7, x11
        stp    x6,  x7, [x0, 16]

        ldp    x4,  x5, [x1, 32]
        ldp    x8,  x9, [x2, 32]
        sbcs   x4,  x4,  x8
        sbcs   x5,  x5,  x9
        stp    x4,  x5, [x0, 32]

        ret


BLS12_381_384_rawNeg:
_BLS12_381_384_rawNeg:
        mov    x2, xzr
        ldp    x8,  x9, [x1]
        orr    x4,  x8,  x9
        orr    x2,  x2,  x4

        ldp   x10, x11, [x1, 16]
        orr    x5, x10, x11
        orr    x2,  x2,  x5

        ldp   x12, x13, [x1, 32]
        orr    x6, x12, x13
        orr    x2,  x2,  x6

        cbz    x2, BLS12_381_384_rawNeg_done_zero

        adr    x3, BLS12_381_384_rawq
        ldp    x4,  x5, [x3]
        subs   x8,  x4,  x8
        sbcs   x9,  x5,  x9
        stp    x8,  x9, [x0]

        ldp    x6,  x7, [x3, 16]
        sbcs  x10,  x6, x10
        sbcs  x11,  x7, x11
        stp   x10, x11, [x0, 16]

        ldp    x4,  x5, [x3, 32]
        sbcs  x12,  x4, x12
        sbcs  x13,  x5, x13
        stp   x12, x13, [x0, 32]

        ret

BLS12_381_384_rawNeg_done_zero:
        stp   xzr, xzr, [x0]
        stp   xzr, xzr, [x0, 16]
        stp   xzr, xzr, [x0, 32]

        ret


BLS12_381_384_rawNegLS:
_BLS12_381_384_rawNegLS:
        stp   x19, x20, [sp, #-16]!

        adr    x3, BLS12_381_384_rawq
        ldp    x8,  x9, [x3]
        subs  x14,  x8,  x2
        sbcs  x15,  x9, xzr

        ldp   x10, x11, [x3, 16]
        sbcs  x16, x10, xzr
        sbcs  x17, x11, xzr

        ldp   x12, x13, [x3, 32]
        sbcs  x19, x12, xzr
        sbcs  x20, x13, xzr

        cset   x2,  cs

        ldp    x4,  x5, [x1]
        subs  x14, x14,  x4
        sbcs  x15, x15,  x5

        ldp    x6,  x7, [x1, 16]
        sbcs  x16, x16,  x6
        sbcs  x17, x17,  x7

        ldp    x4,  x5, [x1, 32]
        sbcs  x19, x19,  x4
        sbcs  x20, x20,  x5

        cset   x3,  cs
        orr    x3,  x3,  x2

        cbz    x3, BLS12_381_384_rawNegLS_done

        adds  x14, x14,  x8
        adcs  x15, x15,  x9
        adcs  x16, x16, x10
        adcs  x17, x17, x11
        adcs  x19, x19, x12
        adcs  x20, x20, x13

BLS12_381_384_rawNegLS_done:
        stp   x14, x15, [x0]
        stp   x16, x17, [x0, 16]
        stp   x19, x20, [x0, 32]

        ldp   x19, x20, [sp], #16
        ret


BLS12_381_384_rawMMul:
_BLS12_381_384_rawMMul:
        stp   x19, x20, [sp, #-16]!
        stp   x21, x22, [sp, #-16]!
        stp   x23, x24, [sp, #-16]!
        stp   x25, x26, [sp, #-16]!
        stp   x27, x28, [sp, #-16]!

        ldp   x16, x17, [x2]
        ldp   x19, x20, [x2, 16]
        ldp   x21, x22, [x2, 32]

        adr    x4, BLS12_381_384_np
        ldr    x4, [x4]

        adr    x6, BLS12_381_384_rawq
        ldp   x23, x24, [x6]
        ldp   x25, x26, [x6, 16]
        ldp   x27, x28, [x6, 32]

        // product0 = pRawB * pRawA[0]
        ldr    x3, [x1]
        mul    x9, x16,  x3
        umulh x10, x16,  x3
        mul    x7, x17,  x3
        adds  x10, x10,  x7
        umulh x11, x17,  x3
        mul    x7, x19,  x3
        adcs  x11, x11,  x7
        umulh x12, x19,  x3
        mul    x7, x20,  x3
        adcs  x12, x12,  x7
        umulh x13, x20,  x3
        mul    x7, x21,  x3
        adcs  x13, x13,  x7
        umulh x14, x21,  x3
        mul    x7, x22,  x3
        adcs  x14, x14,  x7
        umulh x15, x22,  x3
        adc   x15, x15, xzr

        // np0 = Fq_np * product0[0]
        mul    x5,  x4,  x9

        // product0 = product0 + Fq_rawq * np0
        mul    x7, x23,  x5
        adds   x9,  x9,  x7
        mul    x3, x24,  x5
        adcs  x10, x10,  x3
        mul    x7, x25,  x5
        adcs  x11, x11,  x7
        mul    x3, x26,  x5
        adcs  x12, x12,  x3
        mul    x7, x27,  x5
        adcs  x13, x13,  x7
        mul    x3, x28,  x5
        adcs  x14, x14,  x3
        adc   x15, x15, xzr

        umulh  x7, x23,  x5
        adds  x10, x10,  x7
        umulh  x3, x24,  x5
        adcs  x11, x11,  x3
        umulh  x7, x25,  x5
        adcs  x12, x12,  x7
        umulh  x3, x26,  x5
        adcs  x13, x13,  x3
        umulh  x7, x27,  x5
        adcs  x14, x14,  x7
        umulh  x3, x28,  x5
        adcs  x15, x15,  x3
        adc    x5, xzr, xzr

        // product1 = product0 + pRawB * pRawA[1]
        ldr    x3, [x1, 8]
        mul    x9, x16,  x3
        adds   x9,  x9, x10
        mul   x10, x17,  x3
        adcs  x10, x10, x11
        mul   x11, x19,  x3
        adcs  x11, x11, x12
        mul   x12, x20,  x3
        adcs  x12, x12, x13
        mul   x13, x21,  x3
        adcs  x13, x13, x14
        mul   x14, x22,  x3
        adcs  x14, x14, x15
        adc   x15, xzr, xzr

        adds  x10, x10,  x5
        umulh  x7, x16,  x3
        adcs  x10, x10,  x7
        umulh  x5, x17,  x3
        adcs  x11, x11,  x5
        umulh  x7, x19,  x3
        adcs  x12, x12,  x7
        umulh  x5, x20,  x3
        adcs  x13, x13,  x5
        umulh  x7, x21,  x3
        adcs  x14, x14,  x7
        umulh  x5, x22,  x3
        adc   x15, x15,  x5

        // np0 = Fq_np * product1[0]
        mul    x5,  x4,  x9

        // product1 = product1 + Fq_rawq * np0
        mul    x7, x23,  x5
        adds   x9,  x9,  x7
        mul    x3, x24,  x5
        adcs  x10, x10,  x3
        mul    x7, x25,  x5
        adcs  x11, x11,  x7
        mul    x3, x26,  x5
        adcs  x12, x12,  x3
        mul    x7, x27,  x5
        adcs  x13, x13,  x7
        mul    x3, x28,  x5
        adcs  x14, x14,  x3
        adc   x15, x15, xzr

        umulh  x7, x23,  x5
        adds  x10, x10,  x7
        umulh  x3, x24,  x5
        adcs  x11, x11,  x3
        umulh  x7, x25,  x5
        adcs  x12, x12,  x7
        umulh  x3, x26,  x5
        adcs  x13, x13,  x3
        umulh  x7, x27,  x5
        adcs  x14, x14,  x7
        umulh  x3, x28,  x5
        adcs  x15, x15,  x3
        adc    x5, xzr, xzr

        // product2 = product1 + pRawB * pRawA[2]
        ldr    x3, [x1, 16]
        mul    x9, x16,  x3
        adds   x9,  x9, x10
        mul   x10, x17,  x3
        adcs  x10, x10, x11
        mul   x11, x19,  x3
        adcs  x11, x11, x12
        mul   x12, x20,  x3
        adcs  x12, x12, x13
        mul   x13, x21,  x3
        adcs  x13, x13, x14
        mul   x14, x22,  x3
        adcs  x14, x14, x15
        adc   x15, xzr, xzr

        adds  x10, x10,  x5
        umulh  x7, x16,  x3
        adcs  x10, x10,  x7
        umulh  x5, x17,  x3
        adcs  x11, x11,  x5
        umulh  x7, x19,  x3
        adcs  x12, x12,  x7
        umulh  x5, x20,  x3
        adcs  x13, x13,  x5
        umulh  x7, x21,  x3
        adcs  x14, x14,  x7
        umulh  x5, x22,  x3
        adc   x15, x15,  x5

        // np0 = Fq_np * product2[0]
        mul    x5,  x4,  x9

        // product2 = product2 + Fq_rawq * np0
        mul    x7, x23,  x5
        adds   x9,  x9,  x7
        mul    x3, x24,  x5
        adcs  x10, x10,  x3
        mul    x7, x25,  x5
        adcs  x11, x11,  x7
        mul    x3, x26,  x5
        adcs  x12, x12,  x3
        mul    x7, x27,  x5
        adcs  x13, x13,  x7
        mul    x3, x28,  x5
        adcs  x14, x14,  x3
        adc   x15, x15, xzr

        umulh  x7, x23,  x5
        adds  x10, x10,  x7
        umulh  x3, x24,  x5
        adcs  x11, x11,  x3
        umulh  x7, x25,  x5
        adcs  x12, x12,  x7
        umulh  x3, x26,  x5
        adcs  x13, x13,  x3
        umulh  x7, x27,  x5
        adcs  x14, x14,  x7
        umulh  x3, x28,  x5
        adcs  x15, x15,  x3
        adc    x5, xzr, xzr

        // product3 = product2 + pRawB * pRawA[3]
        ldr    x3, [x1, 24]
        mul    x9, x16,  x3
        adds   x9,  x9, x10
        mul   x10, x17,  x3
        adcs  x10, x10, x11
        mul   x11, x19,  x3
        adcs  x11, x11, x12
        mul   x12, x20,  x3
        adcs  x12, x12, x13
        mul   x13, x21,  x3
        adcs  x13, x13, x14
        mul   x14, x22,  x3
        adcs  x14, x14, x15
        adc   x15, xzr, xzr

        adds  x10, x10,  x5
        umulh  x7, x16,  x3
        adcs  x10, x10,  x7
        umulh  x5, x17,  x3
        adcs  x11, x11,  x5
        umulh  x7, x19,  x3
        adcs  x12, x12,  x7
        umulh  x5, x20,  x3
        adcs  x13, x13,  x5
        umulh  x7, x21,  x3
        adcs  x14, x14,  x7
        umulh  x5, x22,  x3
        adc   x15, x15,  x5

        // np0 = Fq_np * product3[0]
        mul    x5,  x4,  x9

        // product3 = product3 + Fq_rawq * np0
        mul    x7, x23,  x5
        adds   x9,  x9,  x7
        mul    x3, x24,  x5
        adcs  x10, x10,  x3
        mul    x7, x25,  x5
        adcs  x11, x11,  x7
        mul    x3, x26,  x5
        adcs  x12, x12,  x3
        mul    x7, x27,  x5
        adcs  x13, x13,  x7
        mul    x3, x28,  x5
        adcs  x14, x14,  x3
        adc   x15, x15, xzr

        umulh  x7, x23,  x5
        adds  x10, x10,  x7
        umulh  x3, x24,  x5
        adcs  x11, x11,  x3
        umulh  x7, x25,  x5
        adcs  x12, x12,  x7
        umulh  x3, x26,  x5
        adcs  x13, x13,  x3
        umulh  x7, x27,  x5
        adcs  x14, x14,  x7
        umulh  x3, x28,  x5
        adcs  x15, x15,  x3
        adc    x5, xzr, xzr

        // product4 = product3 + pRawB * pRawA[4]
        ldr    x3, [x1, 32]
        mul    x9, x16,  x3
        adds   x9,  x9, x10
        mul   x10, x17,  x3
        adcs  x10, x10, x11
        mul   x11, x19,  x3
        adcs  x11, x11, x12
        mul   x12, x20,  x3
        adcs  x12, x12, x13
        mul   x13, x21,  x3
        adcs  x13, x13, x14
        mul   x14, x22,  x3
        adcs  x14, x14, x15
        adc   x15, xzr, xzr

        adds  x10, x10,  x5
        umulh  x7, x16,  x3
        adcs  x10, x10,  x7
        umulh  x5, x17,  x3
        adcs  x11, x11,  x5
        umulh  x7, x19,  x3
        adcs  x12, x12,  x7
        umulh  x5, x20,  x3
        adcs  x13, x13,  x5
        umulh  x7, x21,  x3
        adcs  x14, x14,  x7
        umulh  x5, x22,  x3
        adc   x15, x15,  x5

        // np0 = Fq_np * product4[0]
        mul    x5,  x4,  x9

        // product4 = product4 + Fq_rawq * np0
        mul    x7, x23,  x5
        adds   x9,  x9,  x7
        mul    x3, x24,  x5
        adcs  x10, x10,  x3
        mul    x7, x25,  x5
        adcs  x11, x11,  x7
        mul    x3, x26,  x5
        adcs  x12, x12,  x3
        mul    x7, x27,  x5
        adcs  x13, x13,  x7
        mul    x3, x28,  x5
        adcs  x14, x14,  x3
        adc   x15, x15, xzr

        umulh  x7, x23,  x5
        adds  x10, x10,  x7
        umulh  x3, x24,  x5
        adcs  x11, x11,  x3
        umulh  x7, x25,  x5
        adcs  x12, x12,  x7
        umulh  x3, x26,  x5
        adcs  x13, x13,  x3
        umulh  x7, x27,  x5
        adcs  x14, x14,  x7
        umulh  x3, x28,  x5
        adcs  x15, x15,  x3
        adc    x5, xzr, xzr

        // product5 = product4 + pRawB * pRawA[5]
        ldr    x3, [x1, 40]
        mul    x9, x16,  x3
        adds   x9,  x9, x10
        mul   x10, x17,  x3
        adcs  x10, x10, x11
        mul   x11, x19,  x3
        adcs  x11, x11, x12
        mul   x12, x20,  x3
        adcs  x12, x12, x13
        mul   x13, x21,  x3
        adcs  x13, x13, x14
        mul   x14, x22,  x3
        adcs  x14, x14, x15
        adc   x15, xzr, xzr

        adds  x10, x10,  x5
        umulh  x7, x16,  x3
        adcs  x10, x10,  x7
        umulh  x5, x17,  x3
        adcs  x11, x11,  x5
        umulh  x7, x19,  x3
        adcs  x12, x12,  x7
        umulh  x5, x20,  x3
        adcs  x13, x13,  x5
        umulh  x7, x21,  x3
        adcs  x14, x14,  x7
        umulh  x5, x22,  x3
        adc   x15, x15,  x5

        // np0 = Fq_np * product5[0]
        mul    x5,  x4,  x9

        // product5 = product5 + Fq_rawq * np0
        mul    x7, x23,  x5
        adds   x9,  x9,  x7
        mul    x3, x24,  x5
        adcs  x10, x10,  x3
        mul    x7, x25,  x5
        adcs  x11, x11,  x7
        mul    x3, x26,  x5
        adcs  x12, x12,  x3
        mul    x7, x27,  x5
        adcs  x13, x13,  x7
        mul    x3, x28,  x5
        adcs  x14, x14,  x3
        adc   x15, x15, xzr

        umulh  x7, x23,  x5
        adds  x10, x10,  x7
        umulh  x3, x24,  x5
        adcs  x11, x11,  x3
        umulh  x7, x25,  x5
        adcs  x12, x12,  x7
        umulh  x3, x26,  x5
        adcs  x13, x13,  x3
        umulh  x7, x27,  x5
        adcs  x14, x14,  x7
        umulh  x3, x28,  x5
        adcs  x15, x15,  x3

        // result ge BLS12_381_384_rawq
        subs  x16, x10, x23
        sbcs  x17, x11, x24
        sbcs  x19, x12, x25
        sbcs  x20, x13, x26
        sbcs  x21, x14, x27
        sbcs  x22, x15, x28

        csel  x10, x16, x10,  hs
        csel  x11, x17, x11,  hs
        stp   x10, x11, [x0]

        csel  x12, x19, x12,  hs
        csel  x13, x20, x13,  hs
        stp   x12, x13, [x0, 16]

        csel  x14, x21, x14,  hs
        csel  x15, x22, x15,  hs
        stp   x14, x15, [x0, 32]


        ldp   x27, x28, [sp], #16
        ldp   x25, x26, [sp], #16
        ldp   x23, x24, [sp], #16
        ldp   x21, x22, [sp], #16
        ldp   x19, x20, [sp], #16
        ret


BLS12_381_384_rawMMul1:
_BLS12_381_384_rawMMul1:
        stp   x19, x20, [sp, #-16]!
        stp   x21, x22, [sp, #-16]!
        stp   x23, x24, [sp, #-16]!
        stp   x25, x26, [sp, #-16]!
        stp   x27, x28, [sp, #-16]!

        ldp   x16, x17, [x1]
        ldp   x19, x20, [x1, 16]
        ldp   x21, x22, [x1, 32]

        adr    x4, BLS12_381_384_np
        ldr    x4, [x4]

        adr    x6, BLS12_381_384_rawq
        ldp   x23, x24, [x6]
        ldp   x25, x26, [x6, 16]
        ldp   x27, x28, [x6, 32]

        // product0 = pRawB * pRawA
        mul    x9, x16,  x2
        umulh x10, x16,  x2
        mul    x7, x17,  x2
        adds  x10, x10,  x7
        umulh x11, x17,  x2
        mul    x7, x19,  x2
        adcs  x11, x11,  x7
        umulh x12, x19,  x2
        mul    x7, x20,  x2
        adcs  x12, x12,  x7
        umulh x13, x20,  x2
        mul    x7, x21,  x2
        adcs  x13, x13,  x7
        umulh x14, x21,  x2
        mul    x7, x22,  x2
        adcs  x14, x14,  x7
        umulh x15, x22,  x2
        adc   x15, x15, xzr

        // np0 = Fq_np * product0[0]
        mul    x5,  x4,  x9
        // product0 = product0 + Fq_rawq * np0
        mul    x7, x23,  x5
        adds   x9,  x9,  x7
        mul    x3, x24,  x5
        adcs  x10, x10,  x3
        mul    x7, x25,  x5
        adcs  x11, x11,  x7
        mul    x3, x26,  x5
        adcs  x12, x12,  x3
        mul    x7, x27,  x5
        adcs  x13, x13,  x7
        mul    x3, x28,  x5
        adcs  x14, x14,  x3
        adc   x15, x15, xzr

        umulh  x7, x23,  x5
        adds  x10, x10,  x7
        umulh  x3, x24,  x5
        adcs  x11, x11,  x3
        umulh  x7, x25,  x5
        adcs  x12, x12,  x7
        umulh  x3, x26,  x5
        adcs  x13, x13,  x3
        umulh  x7, x27,  x5
        adcs  x14, x14,  x7
        umulh  x3, x28,  x5
        adcs  x15, x15,  x3
        adc    x8, xzr, xzr

        // np0 = Fq_np * product1[0]
        mul    x5,  x4, x10
        // product1 = product1 + Fq_rawq * np0
        mul    x7, x23,  x5
        adds   x9, x10,  x7
        mul    x3, x24,  x5
        adcs  x10, x11,  x3
        mul    x7, x25,  x5
        adcs  x11, x12,  x7
        mul    x3, x26,  x5
        adcs  x12, x13,  x3
        mul    x7, x27,  x5
        adcs  x13, x14,  x7
        mul    x3, x28,  x5
        adcs  x14, x15,  x3
        adc   x15, xzr, xzr

        adds  x10, x10,  x8
        umulh  x7, x23,  x5
        adds  x10, x10,  x7
        umulh  x3, x24,  x5
        adcs  x11, x11,  x3
        umulh  x7, x25,  x5
        adcs  x12, x12,  x7
        umulh  x3, x26,  x5
        adcs  x13, x13,  x3
        umulh  x7, x27,  x5
        adcs  x14, x14,  x7
        umulh  x3, x28,  x5
        adcs  x15, x15,  x3
        adc    x8, xzr, xzr

        // np0 = Fq_np * product2[0]
        mul    x5,  x4, x10
        // product2 = product2 + Fq_rawq * np0
        mul    x7, x23,  x5
        adds   x9, x10,  x7
        mul    x3, x24,  x5
        adcs  x10, x11,  x3
        mul    x7, x25,  x5
        adcs  x11, x12,  x7
        mul    x3, x26,  x5
        adcs  x12, x13,  x3
        mul    x7, x27,  x5
        adcs  x13, x14,  x7
        mul    x3, x28,  x5
        adcs  x14, x15,  x3
        adc   x15, xzr, xzr

        adds  x10, x10,  x8
        umulh  x7, x23,  x5
        adds  x10, x10,  x7
        umulh  x3, x24,  x5
        adcs  x11, x11,  x3
        umulh  x7, x25,  x5
        adcs  x12, x12,  x7
        umulh  x3, x26,  x5
        adcs  x13, x13,  x3
        umulh  x7, x27,  x5
        adcs  x14, x14,  x7
        umulh  x3, x28,  x5
        adcs  x15, x15,  x3
        adc    x8, xzr, xzr

        // np0 = Fq_np * product3[0]
        mul    x5,  x4, x10
        // product3 = product3 + Fq_rawq * np0
        mul    x7, x23,  x5
        adds   x9, x10,  x7
        mul    x3, x24,  x5
        adcs  x10, x11,  x3
        mul    x7, x25,  x5
        adcs  x11, x12,  x7
        mul    x3, x26,  x5
        adcs  x12, x13,  x3
        mul    x7, x27,  x5
        adcs  x13, x14,  x7
        mul    x3, x28,  x5
        adcs  x14, x15,  x3
        adc   x15, xzr, xzr

        adds  x10, x10,  x8
        umulh  x7, x23,  x5
        adds  x10, x10,  x7
        umulh  x3, x24,  x5
        adcs  x11, x11,  x3
        umulh  x7, x25,  x5
        adcs  x12, x12,  x7
        umulh  x3, x26,  x5
        adcs  x13, x13,  x3
        umulh  x7, x27,  x5
        adcs  x14, x14,  x7
        umulh  x3, x28,  x5
        adcs  x15, x15,  x3
        adc    x8, xzr, xzr

        // np0 = Fq_np * product4[0]
        mul    x5,  x4, x10
        // product4 = product4 + Fq_rawq * np0
        mul    x7, x23,  x5
        adds   x9, x10,  x7
        mul    x3, x24,  x5
        adcs  x10, x11,  x3
        mul    x7, x25,  x5
        adcs  x11, x12,  x7
        mul    x3, x26,  x5
        adcs  x12, x13,  x3
        mul    x7, x27,  x5
        adcs  x13, x14,  x7
        mul    x3, x28,  x5
        adcs  x14, x15,  x3
        adc   x15, xzr, xzr

        adds  x10, x10,  x8
        umulh  x7, x23,  x5
        adds  x10, x10,  x7
        umulh  x3, x24,  x5
        adcs  x11, x11,  x3
        umulh  x7, x25,  x5
        adcs  x12, x12,  x7
        umulh  x3, x26,  x5
        adcs  x13, x13,  x3
        umulh  x7, x27,  x5
        adcs  x14, x14,  x7
        umulh  x3, x28,  x5
        adcs  x15, x15,  x3
        adc    x8, xzr, xzr

        // np0 = Fq_np * product5[0]
        mul    x5,  x4, x10
        // product5 = product5 + Fq_rawq * np0
        mul    x7, x23,  x5
        adds   x9, x10,  x7
        mul    x3, x24,  x5
        adcs  x10, x11,  x3
        mul    x7, x25,  x5
        adcs  x11, x12,  x7
        mul    x3, x26,  x5
        adcs  x12, x13,  x3
        mul    x7, x27,  x5
        adcs  x13, x14,  x7
        mul    x3, x28,  x5
        adcs  x14, x15,  x3
        adc   x15, xzr, xzr

        adds  x10, x10,  x8
        umulh  x7, x23,  x5
        adds  x10, x10,  x7
        umulh  x3, x24,  x5
        adcs  x11, x11,  x3
        umulh  x7, x25,  x5
        adcs  x12, x12,  x7
        umulh  x3, x26,  x5
        adcs  x13, x13,  x3
        umulh  x7, x27,  x5
        adcs  x14, x14,  x7
        umulh  x3, x28,  x5
        adcs  x15, x15,  x3

        // result ge BLS12_381_384_rawq
        subs  x16, x10, x23
        sbcs  x17, x11, x24
        sbcs  x19, x12, x25
        sbcs  x20, x13, x26
        sbcs  x21, x14, x27
        sbcs  x22, x15, x28

        csel  x10, x16, x10,  hs
        csel  x11, x17, x11,  hs
        stp   x10, x11, [x0]

        csel  x12, x19, x12,  hs
        csel  x13, x20, x13,  hs
        stp   x12, x13, [x0, 16]

        csel  x14, x21, x14,  hs
        csel  x15, x22, x15,  hs
        stp   x14, x15, [x0, 32]


        ldp   x27, x28, [sp], #16
        ldp   x25, x26, [sp], #16
        ldp   x23, x24, [sp], #16
        ldp   x21, x22, [sp], #16
        ldp   x19, x20, [sp], #16
        ret


BLS12_381_384_rawFromMontgomery:
_BLS12_381_384_rawFromMontgomery:
        stp   x19, x20, [sp, #-16]!
        stp   x21, x22, [sp, #-16]!
        stp   x23, x24, [sp, #-16]!
        stp   x25, x26, [sp, #-16]!
        stp   x27, x28, [sp, #-16]!

        ldp    x9, x10, [x1]
        ldp   x11, x12, [x1, 16]
        ldp   x13, x14, [x1, 32]
        mov   x15, xzr

        adr    x4, BLS12_381_384_np
        ldr    x4, [x4]

        adr    x6, BLS12_381_384_rawq
        ldp   x23, x24, [x6]
        ldp   x25, x26, [x6, 16]
        ldp   x27, x28, [x6, 32]

        // np0 = Fq_np * product0[0]
        mul    x5,  x4,  x9
        // product0 = product0 + Fq_rawq * np0
        mul    x7, x23,  x5
        adds   x9,  x9,  x7
        mul    x3, x24,  x5
        adcs  x10, x10,  x3
        mul    x7, x25,  x5
        adcs  x11, x11,  x7
        mul    x3, x26,  x5
        adcs  x12, x12,  x3
        mul    x7, x27,  x5
        adcs  x13, x13,  x7
        mul    x3, x28,  x5
        adcs  x14, x14,  x3
        adc   x15, x15, xzr

        umulh  x7, x23,  x5
        adds  x10, x10,  x7
        umulh  x3, x24,  x5
        adcs  x11, x11,  x3
        umulh  x7, x25,  x5
        adcs  x12, x12,  x7
        umulh  x3, x26,  x5
        adcs  x13, x13,  x3
        umulh  x7, x27,  x5
        adcs  x14, x14,  x7
        umulh  x3, x28,  x5
        adcs  x15, x15,  x3
        adc    x8, xzr, xzr

        // np0 = Fq_np * product1[0]
        mul    x5,  x4, x10
        // product1 = product1 + Fq_rawq * np0
        mul    x7, x23,  x5
        adds   x9, x10,  x7
        mul    x3, x24,  x5
        adcs  x10, x11,  x3
        mul    x7, x25,  x5
        adcs  x11, x12,  x7
        mul    x3, x26,  x5
        adcs  x12, x13,  x3
        mul    x7, x27,  x5
        adcs  x13, x14,  x7
        mul    x3, x28,  x5
        adcs  x14, x15,  x3
        adc   x15, xzr, xzr

        adds  x10, x10,  x8
        umulh  x7, x23,  x5
        adds  x10, x10,  x7
        umulh  x3, x24,  x5
        adcs  x11, x11,  x3
        umulh  x7, x25,  x5
        adcs  x12, x12,  x7
        umulh  x3, x26,  x5
        adcs  x13, x13,  x3
        umulh  x7, x27,  x5
        adcs  x14, x14,  x7
        umulh  x3, x28,  x5
        adcs  x15, x15,  x3
        adc    x8, xzr, xzr

        // np0 = Fq_np * product2[0]
        mul    x5,  x4, x10
        // product2 = product2 + Fq_rawq * np0
        mul    x7, x23,  x5
        adds   x9, x10,  x7
        mul    x3, x24,  x5
        adcs  x10, x11,  x3
        mul    x7, x25,  x5
        adcs  x11, x12,  x7
        mul    x3, x26,  x5
        adcs  x12, x13,  x3
        mul    x7, x27,  x5
        adcs  x13, x14,  x7
        mul    x3, x28,  x5
        adcs  x14, x15,  x3
        adc   x15, xzr, xzr

        adds  x10, x10,  x8
        umulh  x7, x23,  x5
        adds  x10, x10,  x7
        umulh  x3, x24,  x5
        adcs  x11, x11,  x3
        umulh  x7, x25,  x5
        adcs  x12, x12,  x7
        umulh  x3, x26,  x5
        adcs  x13, x13,  x3
        umulh  x7, x27,  x5
        adcs  x14, x14,  x7
        umulh  x3, x28,  x5
        adcs  x15, x15,  x3
        adc    x8, xzr, xzr

        // np0 = Fq_np * product3[0]
        mul    x5,  x4, x10
        // product3 = product3 + Fq_rawq * np0
        mul    x7, x23,  x5
        adds   x9, x10,  x7
        mul    x3, x24,  x5
        adcs  x10, x11,  x3
        mul    x7, x25,  x5
        adcs  x11, x12,  x7
        mul    x3, x26,  x5
        adcs  x12, x13,  x3
        mul    x7, x27,  x5
        adcs  x13, x14,  x7
        mul    x3, x28,  x5
        adcs  x14, x15,  x3
        adc   x15, xzr, xzr

        adds  x10, x10,  x8
        umulh  x7, x23,  x5
        adds  x10, x10,  x7
        umulh  x3, x24,  x5
        adcs  x11, x11,  x3
        umulh  x7, x25,  x5
        adcs  x12, x12,  x7
        umulh  x3, x26,  x5
        adcs  x13, x13,  x3
        umulh  x7, x27,  x5
        adcs  x14, x14,  x7
        umulh  x3, x28,  x5
        adcs  x15, x15,  x3
        adc    x8, xzr, xzr

        // np0 = Fq_np * product4[0]
        mul    x5,  x4, x10
        // product4 = product4 + Fq_rawq * np0
        mul    x7, x23,  x5
        adds   x9, x10,  x7
        mul    x3, x24,  x5
        adcs  x10, x11,  x3
        mul    x7, x25,  x5
        adcs  x11, x12,  x7
        mul    x3, x26,  x5
        adcs  x12, x13,  x3
        mul    x7, x27,  x5
        adcs  x13, x14,  x7
        mul    x3, x28,  x5
        adcs  x14, x15,  x3
        adc   x15, xzr, xzr

        adds  x10, x10,  x8
        umulh  x7, x23,  x5
        adds  x10, x10,  x7
        umulh  x3, x24,  x5
        adcs  x11, x11,  x3
        umulh  x7, x25,  x5
        adcs  x12, x12,  x7
        umulh  x3, x26,  x5
        adcs  x13, x13,  x3
        umulh  x7, x27,  x5
        adcs  x14, x14,  x7
        umulh  x3, x28,  x5
        adcs  x15, x15,  x3
        adc    x8, xzr, xzr

        // np0 = Fq_np * product5[0]
        mul    x5,  x4, x10
        // product5 = product5 + Fq_rawq * np0
        mul    x7, x23,  x5
        adds   x9, x10,  x7
        mul    x3, x24,  x5
        adcs  x10, x11,  x3
        mul    x7, x25,  x5
        adcs  x11, x12,  x7
        mul    x3, x26,  x5
        adcs  x12, x13,  x3
        mul    x7, x27,  x5
        adcs  x13, x14,  x7
        mul    x3, x28,  x5
        adcs  x14, x15,  x3
        adc   x15, xzr, xzr

        adds  x10, x10,  x8
        umulh  x7, x23,  x5
        adds  x10, x10,  x7
        umulh  x3, x24,  x5
        adcs  x11, x11,  x3
        umulh  x7, x25,  x5
        adcs  x12, x12,  x7
        umulh  x3, x26,  x5
        adcs  x13, x13,  x3
        umulh  x7, x27,  x5
        adcs  x14, x14,  x7
        umulh  x3, x28,  x5
        adcs  x15, x15,  x3

        // result ge BLS12_381_384_rawq
        subs  x16, x10, x23
        sbcs  x17, x11, x24
        sbcs  x19, x12, x25
        sbcs  x20, x13, x26
        sbcs  x21, x14, x27
        sbcs  x22, x15, x28

        csel  x10, x16, x10,  hs
        csel  x11, x17, x11,  hs
        stp   x10, x11, [x0]

        csel  x12, x19, x12,  hs
        csel  x13, x20, x13,  hs
        stp   x12, x13, [x0, 16]

        csel  x14, x21, x14,  hs
        csel  x15, x22, x15,  hs
        stp   x14, x15, [x0, 32]


        ldp   x27, x28, [sp], #16
        ldp   x25, x26, [sp], #16
        ldp   x23, x24, [sp], #16
        ldp   x21, x22, [sp], #16
        ldp   x19, x20, [sp], #16
        ret


BLS12_381_384_rawIsZero:
_BLS12_381_384_rawIsZero:
        ldp    x1,  x2, [x0]
        orr    x3,  x1,  x2

        ldp    x4,  x5, [x0, 16]
        orr    x6,  x4,  x5
        orr    x7,  x3,  x6

        ldp    x8,  x9, [x0, 32]
        orr   x10,  x8,  x9
        orr   x17,  x7, x10

        cmp   x17, xzr
        cset   x0,  eq
        ret

BLS12_381_384_rawIsEq:
_BLS12_381_384_rawIsEq:
        ldp    x5,  x6, [x0]
        ldp    x9, x10, [x1]
        eor   x13,  x5,  x9
        eor   x14,  x6, x10
        orr    x2, x13, x14

        ldp    x7,  x8, [x0, 16]
        ldp   x11, x12, [x1, 16]
        eor   x15,  x7, x11
        eor   x16,  x8, x12
        orr    x3, x15, x16
        orr    x4,  x2,  x3

        ldp    x5,  x6, [x0, 32]
        ldp    x9, x10, [x1, 32]
        eor   x13,  x5,  x9
        eor   x14,  x6, x10
        orr    x2, x13, x14
        orr   x17,  x4,  x2

        cmp   x17, xzr
        cset   x0,  eq
        ret

BLS12_381_384_rawCmp:
_BLS12_381_384_rawCmp:
        ldp    x3,  x4, [x0]
        ldp    x7,  x8, [x1]
        subs   x3,  x3,  x7
        cset   x2,  ne
        sbcs   x4,  x4,  x8
        cinc   x2,  x2,  ne

        ldp    x5,  x6, [x0, 16]
        ldp    x9, x10, [x1, 16]
        sbcs   x5,  x5,  x9
        cinc   x2,  x2,  ne
        sbcs   x6,  x6, x10
        cinc   x2,  x2,  ne

        ldp    x3,  x4, [x0, 32]
        ldp    x7,  x8, [x1, 32]
        sbcs   x3,  x3,  x7
        cinc   x2,  x2,  ne
        sbcs   x4,  x4,  x8
        cinc   x2,  x2,  ne

        cneg   x0,  x2,  lo
        ret

BLS12_381_384_rawCopy:
_BLS12_381_384_rawCopy:
        ldp    x2,  x3, [x1]
        stp    x2,  x3, [x0]

        ldp    x4,  x5, [x1, 16]
        stp    x4,  x5, [x0, 16]

        ldp    x6,  x7, [x1, 32]
        stp    x6,  x7, [x0, 32]

        ret

BLS12_381_384_rawCopyS2L:
_BLS12_381_384_rawCopyS2L:
        cmp    x1, xzr
        b.lt  BLS12_381_384_rawCopyS2L_adjust_neg

        stp    x1, xzr, [x0]
        stp   xzr, xzr, [x0, 16]
        stp   xzr, xzr, [x0, 32]
        ret

BLS12_381_384_rawCopyS2L_adjust_neg:
        mov    x2,  -1
        adr    x3, BLS12_381_384_rawq

        ldp    x4,  x5, [x3]
        adds  x10,  x1,  x4
        adcs  x11,  x2,  x5
        stp   x10, x11, [x0]

        ldp    x6,  x7, [x3, 16]
        adcs  x12,  x2,  x6
        adcs  x13,  x2,  x7
        stp   x12, x13, [x0, 16]

        ldp    x8,  x9, [x3, 32]
        adcs  x14,  x2,  x8
        adcs  x15,  x2,  x9
        stp   x14, x15, [x0, 32]

        ret

BLS12_381_384_rawSwap:
_BLS12_381_384_rawSwap:
        ldp    x2,  x3, [x0]
        ldp   x10, x11, [x1]
        stp    x2,  x3, [x1]
        stp   x10, x11, [x0]

        ldp    x4,  x5, [x0, 16]
        ldp   x12, x13, [x1, 16]
        stp    x4,  x5, [x1, 16]
        stp   x12, x13, [x0, 16]

        ldp    x6,  x7, [x0, 32]
        ldp   x14, x15, [x1, 32]
        stp    x6,  x7, [x1, 32]
        stp   x14, x15, [x0, 32]

        ret

BLS12_381_384_rawAnd:
_BLS12_381_384_rawAnd:
        stp   x19, x20, [sp, #-16]!

        ldp    x8,  x9, [x1]
        ldp    x4,  x5, [x2]
        and    x8,  x8,  x4
        and    x9,  x9,  x5

        ldp   x10, x11, [x1, 16]
        ldp    x6,  x7, [x2, 16]
        and   x10, x10,  x6
        and   x11, x11,  x7

        ldp   x12, x13, [x1, 32]
        ldp    x4,  x5, [x2, 32]
        and   x12, x12,  x4
        and   x13, x13,  x5

        adr    x2, BLS12_381_384_lboMask
        ldr    x2, [x2]
        and   x13, x13,  x2

        adr    x3, BLS12_381_384_rawq
        ldp   x14, x15, [x3]
        subs  x14,  x8, x14
        sbcs  x15,  x9, x15

        ldp   x16, x17, [x3, 16]
        sbcs  x16, x10, x16
        sbcs  x17, x11, x17

        ldp   x19, x20, [x3, 32]
        sbcs  x19, x12, x19
        sbcs  x20, x13, x20

        csel   x8, x14,  x8,  hs
        csel   x9, x15,  x9,  hs
        stp    x8,  x9, [x0]

        csel  x10, x16, x10,  hs
        csel  x11, x17, x11,  hs
        stp   x10, x11, [x0, 16]

        csel  x12, x19, x12,  hs
        csel  x13, x20, x13,  hs
        stp   x12, x13, [x0, 32]


        ldp   x19, x20, [sp], #16
        ret

BLS12_381_384_rawOr:
_BLS12_381_384_rawOr:
        stp   x19, x20, [sp, #-16]!

        ldp    x8,  x9, [x1]
        ldp    x4,  x5, [x2]
        orr    x8,  x8,  x4
        orr    x9,  x9,  x5

        ldp   x10, x11, [x1, 16]
        ldp    x6,  x7, [x2, 16]
        orr   x10, x10,  x6
        orr   x11, x11,  x7

        ldp   x12, x13, [x1, 32]
        ldp    x4,  x5, [x2, 32]
        orr   x12, x12,  x4
        orr   x13, x13,  x5

        adr    x2, BLS12_381_384_lboMask
        ldr    x2, [x2]
        and   x13, x13,  x2

        adr    x3, BLS12_381_384_rawq
        ldp   x14, x15, [x3]
        subs  x14,  x8, x14
        sbcs  x15,  x9, x15

        ldp   x16, x17, [x3, 16]
        sbcs  x16, x10, x16
        sbcs  x17, x11, x17

        ldp   x19, x20, [x3, 32]
        sbcs  x19, x12, x19
        sbcs  x20, x13, x20

        csel   x8, x14,  x8,  hs
        csel   x9, x15,  x9,  hs
        stp    x8,  x9, [x0]

        csel  x10, x16, x10,  hs
        csel  x11, x17, x11,  hs
        stp   x10, x11, [x0, 16]

        csel  x12, x19, x12,  hs
        csel  x13, x20, x13,  hs
        stp   x12, x13, [x0, 32]


        ldp   x19, x20, [sp], #16
        ret

BLS12_381_384_rawXor:
_BLS12_381_384_rawXor:
        stp   x19, x20, [sp, #-16]!

        ldp    x8,  x9, [x1]
        ldp    x4,  x5, [x2]
        eor    x8,  x8,  x4
        eor    x9,  x9,  x5

        ldp   x10, x11, [x1, 16]
        ldp    x6,  x7, [x2, 16]
        eor   x10, x10,  x6
        eor   x11, x11,  x7

        ldp   x12, x13, [x1, 32]
        ldp    x4,  x5, [x2, 32]
        eor   x12, x12,  x4
        eor   x13, x13,  x5

        adr    x2, BLS12_381_384_lboMask
        ldr    x2, [x2]
        and   x13, x13,  x2

        adr    x3, BLS12_381_384_rawq
        ldp   x14, x15, [x3]
        subs  x14,  x8, x14
        sbcs  x15,  x9, x15

        ldp   x16, x17, [x3, 16]
        sbcs  x16, x10, x16
        sbcs  x17, x11, x17

        ldp   x19, x20, [x3, 32]
        sbcs  x19, x12, x19
        sbcs  x20, x13, x20

        csel   x8, x14,  x8,  hs
        csel   x9, x15,  x9,  hs
        stp    x8,  x9, [x0]

        csel  x10, x16, x10,  hs
        csel  x11, x17, x11,  hs
        stp   x10, x11, [x0, 16]

        csel  x12, x19, x12,  hs
        csel  x13, x20, x13,  hs
        stp   x12, x13, [x0, 32]


        ldp   x19, x20, [sp], #16
        ret

BLS12_381_384_rawNot:
_BLS12_381_384_rawNot:
        stp   x19, x20, [sp, #-16]!

        ldp    x8,  x9, [x1]
        mvn    x8,  x8
        mvn    x9,  x9

        ldp   x10, x11, [x1, 16]
        mvn   x10, x10
        mvn   x11, x11

        ldp   x12, x13, [x1, 32]
        mvn   x12, x12
        mvn   x13, x13

        adr    x2, BLS12_381_384_lboMask
        ldr    x2, [x2]
        and   x13, x13,  x2

        adr    x3, BLS12_381_384_rawq
        ldp   x14, x15, [x3]
        subs  x14,  x8, x14
        sbcs  x15,  x9, x15

        ldp   x16, x17, [x3, 16]
        sbcs  x16, x10, x16
        sbcs  x17, x11, x17

        ldp   x19, x20, [x3, 32]
        sbcs  x19, x12, x19
        sbcs  x20, x13, x20

        csel   x8, x14,  x8,  hs
        csel   x9, x15,  x9,  hs
        stp    x8,  x9, [x0]

        csel  x10, x16, x10,  hs
        csel  x11, x17, x11,  hs
        stp   x10, x11, [x0, 16]

        csel  x12, x19, x12,  hs
        csel  x13, x20, x13,  hs
        stp   x12, x13, [x0, 32]


        ldp   x19, x20, [sp], #16
        ret

BLS12_381_384_rawShr:
_BLS12_381_384_rawShr:
        ldp    x8,  x9, [x1]
        ldp   x10, x11, [x1, 16]
        ldp   x12, x13, [x1, 32]

        and    x3,  x2, 0x3f
        mov    x4, 0x3f
        sub    x4,  x4,  x3

        lsr    x2,  x2,  #6
        adr    x5, BLS12_381_384_rawShr_word_shift
        ldr    x5, [x5, x2, lsl 3]
        br     x5

BLS12_381_384_rawShr_word_shift_0:
        lsr    x8,  x8,  x3
        lsl    x6,  x9,  x4
        orr    x8,  x8,  x6, lsl #1

        lsr    x9,  x9,  x3
        lsl    x7, x10,  x4
        orr    x9,  x9,  x7, lsl #1

        lsr   x10, x10,  x3
        lsl    x6, x11,  x4
        orr   x10, x10,  x6, lsl #1

        lsr   x11, x11,  x3
        lsl    x7, x12,  x4
        orr   x11, x11,  x7, lsl #1

        lsr   x12, x12,  x3
        lsl    x6, x13,  x4
        orr   x12, x12,  x6, lsl #1

        lsr   x13, x13,  x3

        stp    x8,  x9, [x0]
        stp   x10, x11, [x0, 16]
        stp   x12, x13, [x0, 32]
        ret

BLS12_381_384_rawShr_word_shift_1:
        lsr    x9,  x9,  x3
        lsl    x7, x10,  x4
        orr    x9,  x9,  x7, lsl #1

        lsr   x10, x10,  x3
        lsl    x6, x11,  x4
        orr   x10, x10,  x6, lsl #1

        lsr   x11, x11,  x3
        lsl    x7, x12,  x4
        orr   x11, x11,  x7, lsl #1

        lsr   x12, x12,  x3
        lsl    x6, x13,  x4
        orr   x12, x12,  x6, lsl #1

        lsr   x13, x13,  x3

        stp    x9, x10, [x0]
        stp   x11, x12, [x0, 16]
        stp   x13, xzr, [x0, 32]
        ret

BLS12_381_384_rawShr_word_shift_2:
        lsr   x10, x10,  x3
        lsl    x7, x11,  x4
        orr   x10, x10,  x7, lsl #1

        lsr   x11, x11,  x3
        lsl    x6, x12,  x4
        orr   x11, x11,  x6, lsl #1

        lsr   x12, x12,  x3
        lsl    x7, x13,  x4
        orr   x12, x12,  x7, lsl #1

        lsr   x13, x13,  x3

        stp   x10, x11, [x0]
        stp   x12, x13, [x0, 16]
        stp   xzr, xzr, [x0, 32]
        ret

BLS12_381_384_rawShr_word_shift_3:
        lsr   x11, x11,  x3
        lsl    x6, x12,  x4
        orr   x11, x11,  x6, lsl #1

        lsr   x12, x12,  x3
        lsl    x7, x13,  x4
        orr   x12, x12,  x7, lsl #1

        lsr   x13, x13,  x3

        stp   x11, x12, [x0]
        stp   x13, xzr, [x0, 16]
        stp   xzr, xzr, [x0, 32]
        ret

BLS12_381_384_rawShr_word_shift_4:
        lsr   x12, x12,  x3
        lsl    x6, x13,  x4
        orr   x12, x12,  x6, lsl #1

        lsr   x13, x13,  x3

        stp   x12, x13, [x0]
        stp   xzr, xzr, [x0, 16]
        stp   xzr, xzr, [x0, 32]
        ret

BLS12_381_384_rawShr_word_shift_5:
        lsr   x13, x13,  x3

        stp   x13, xzr, [x0]
        stp   xzr, xzr, [x0, 16]
        stp   xzr, xzr, [x0, 32]
        ret

BLS12_381_384_rawShr_word_shift:
        .quad BLS12_381_384_rawShr_word_shift_0
        .quad BLS12_381_384_rawShr_word_shift_1
        .quad BLS12_381_384_rawShr_word_shift_2
        .quad BLS12_381_384_rawShr_word_shift_3
        .quad BLS12_381_384_rawShr_word_shift_4
        .quad BLS12_381_384_rawShr_word_shift_5


BLS12_381_384_rawShl:
_BLS12_381_384_rawShl:
        stp   x19, x20, [sp, #-16]!
        str   x21, [sp, #-16]!

        ldp    x9, x10, [x1]
        ldp   x11, x12, [x1, 16]
        ldp   x13, x14, [x1, 32]

        and    x3,  x2, 0x3f
        mov    x4, 0x3f
        sub    x4,  x4,  x3

        lsr    x2,  x2,  #6
        adr    x5, BLS12_381_384_rawShl_word_shift
        ldr    x5, [x5, x2, lsl 3]
        br     x5

BLS12_381_384_rawShl_word_shift_0:
        lsl   x14, x14,  x3
        lsr    x7, x13,  x4
        orr   x14, x14,  x7, lsr #1

        lsl   x13, x13,  x3
        lsr    x8, x12,  x4
        orr   x13, x13,  x8, lsr #1

        lsl   x12, x12,  x3
        lsr    x7, x11,  x4
        orr   x12, x12,  x7, lsr #1

        lsl   x11, x11,  x3
        lsr    x8, x10,  x4
        orr   x11, x11,  x8, lsr #1

        lsl   x10, x10,  x3
        lsr    x7,  x9,  x4
        orr   x10, x10,  x7, lsr #1

        lsl    x9,  x9,  x3

        b     BLS12_381_384_rawShl_sub

BLS12_381_384_rawShl_word_shift_1:
        lsl   x14, x13,  x3
        lsr    x8, x12,  x4
        orr   x14, x14,  x8, lsr #1

        lsl   x13, x12,  x3
        lsr    x7, x11,  x4
        orr   x13, x13,  x7, lsr #1

        lsl   x12, x11,  x3
        lsr    x8, x10,  x4
        orr   x12, x12,  x8, lsr #1

        lsl   x11, x10,  x3
        lsr    x7,  x9,  x4
        orr   x11, x11,  x7, lsr #1

        lsl   x10,  x9,  x3
        mov    x9, xzr

        b     BLS12_381_384_rawShl_sub

BLS12_381_384_rawShl_word_shift_2:
        lsl   x14, x12,  x3
        lsr    x8, x11,  x4
        orr   x14, x14,  x8, lsr #1

        lsl   x13, x11,  x3
        lsr    x7, x10,  x4
        orr   x13, x13,  x7, lsr #1

        lsl   x12, x10,  x3
        lsr    x8,  x9,  x4
        orr   x12, x12,  x8, lsr #1

        lsl   x11,  x9,  x3
        mov   x10, xzr
        mov    x9, xzr

        b     BLS12_381_384_rawShl_sub

BLS12_381_384_rawShl_word_shift_3:
        lsl   x14, x11,  x3
        lsr    x7, x10,  x4
        orr   x14, x14,  x7, lsr #1

        lsl   x13, x10,  x3
        lsr    x8,  x9,  x4
        orr   x13, x13,  x8, lsr #1

        lsl   x12,  x9,  x3
        mov   x11, xzr
        mov   x10, xzr
        mov    x9, xzr

        b     BLS12_381_384_rawShl_sub

BLS12_381_384_rawShl_word_shift_4:
        lsl   x14, x10,  x3
        lsr    x7,  x9,  x4
        orr   x14, x14,  x7, lsr #1

        lsl   x13,  x9,  x3
        mov   x12, xzr
        mov   x11, xzr
        mov   x10, xzr
        mov    x9, xzr

        b     BLS12_381_384_rawShl_sub

BLS12_381_384_rawShl_word_shift_5:
        lsl   x14,  x9,  x3
        mov   x13, xzr
        mov   x12, xzr
        mov   x11, xzr
        mov   x10, xzr
        mov    x9, xzr

BLS12_381_384_rawShl_sub:
        adr    x6, BLS12_381_384_lboMask
        ldr    x6, [x6]
        and   x14, x14,  x6

        adr    x1, BLS12_381_384_rawq
        ldp   x15, x16, [x1]
        subs  x15,  x9, x15
        sbcs  x16, x10, x16

        ldp   x17, x19, [x1, 16]
        sbcs  x17, x11, x17
        sbcs  x19, x12, x19

        ldp   x20, x21, [x1, 32]
        sbcs  x20, x13, x20
        sbcs  x21, x14, x21

        csel   x9, x15,  x9,  hs
        csel  x10, x16, x10,  hs
        stp    x9, x10, [x0]

        csel  x11, x17, x11,  hs
        csel  x12, x19, x12,  hs
        stp   x11, x12, [x0, 16]

        csel  x13, x20, x13,  hs
        csel  x14, x21, x14,  hs
        stp   x13, x14, [x0, 32]


        ldr   x21, [sp], #16
        ldp   x19, x20, [sp], #16
        ret
BLS12_381_384_rawShl_word_shift:
        .quad BLS12_381_384_rawShl_word_shift_0
        .quad BLS12_381_384_rawShl_word_shift_1
        .quad BLS12_381_384_rawShl_word_shift_2
        .quad BLS12_381_384_rawShl_word_shift_3
        .quad BLS12_381_384_rawShl_word_shift_4
        .quad BLS12_381_384_rawShl_word_shift_5




    .align 8
BLS12_381_384_rawq:    .quad 0xb9feffffffffaaab,0x1eabfffeb153ffff,0x6730d2a0f6b0f624,0x64774b84f38512bf,0x4b1ba7b6434bacd7,0x1a0111ea397fe69a
BLS12_381_384_np:      .quad 0x89f3fffcfffcfffd
BLS12_381_384_lboMask: .quad 0x1fffffffffffffff
