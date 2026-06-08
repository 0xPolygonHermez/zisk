    .global Fq_rawAdd
    .global Fq_rawAddLS
    .global Fq_rawSub
    .global Fq_rawSubRegular
    .global Fq_rawNeg
    .global Fq_rawNegLS
    .global Fq_rawSubSL
    .global Fq_rawSubLS
    .global Fq_rawMMul
    .global Fq_rawMMul1
    .global Fq_rawFromMontgomery
    .global Fq_rawCopy
    .global Fq_rawSwap
    .global Fq_rawIsEq
    .global Fq_rawIsZero
    .global Fq_rawCopyS2L
    .global Fq_rawCmp
    .global Fq_rawAnd
    .global Fq_rawOr
    .global Fq_rawXor
    .global Fq_rawShr
    .global Fq_rawShl
    .global Fq_rawNot

    .global _Fq_rawAdd
    .global _Fq_rawAddLS
    .global _Fq_rawSub
    .global _Fq_rawSubRegular
    .global _Fq_rawNeg
    .global _Fq_rawNegLS
    .global _Fq_rawSubSL
    .global _Fq_rawSubLS
    .global _Fq_rawMMul
    .global _Fq_rawMMul1
    .global _Fq_rawFromMontgomery
    .global _Fq_rawCopy
    .global _Fq_rawSwap
    .global _Fq_rawIsEq
    .global _Fq_rawIsZero
    .global _Fq_rawCopyS2L
    .global _Fq_rawCmp
    .global _Fq_rawAnd
    .global _Fq_rawOr
    .global _Fq_rawXor
    .global _Fq_rawShr
    .global _Fq_rawShl
    .global _Fq_rawNot

    .text
    .align 4

Fq_rawAdd:
_Fq_rawAdd:
        ldp    x8,  x9, [x1]
        ldp    x4,  x5, [x2]
        adds   x8,  x8,  x4
        adcs   x9,  x9,  x5

        ldp   x10, x11, [x1, 16]
        ldp    x6,  x7, [x2, 16]
        adcs  x10, x10,  x6
        adcs  x11, x11,  x7

        cset   x2,  cs

        adr    x3, Fq_rawq
        ldp   x12, x13, [x3]
        subs  x12,  x8, x12
        sbcs  x13,  x9, x13

        ldp   x14, x15, [x3, 16]
        sbcs  x14, x10, x14
        sbcs  x15, x11, x15

        cbnz   x2, Fq_rawAdd_done_s
        b.hs  Fq_rawAdd_done_s

        stp    x8,  x9, [x0]
        stp   x10, x11, [x0, 16]

        b     Fq_rawAdd_out

Fq_rawAdd_done_s:
        stp   x12, x13, [x0]
        stp   x14, x15, [x0, 16]

Fq_rawAdd_out:
        ret


Fq_rawAddLS:
_Fq_rawAddLS:
        ldp    x8,  x9, [x1]
        adds   x8,  x8,  x2
        adcs   x9,  x9, xzr

        ldp   x10, x11, [x1, 16]
        adcs  x10, x10, xzr
        adcs  x11, x11, xzr

        cset   x2,  cs

        adr    x3, Fq_rawq
        ldp   x12, x13, [x3]
        subs  x12,  x8, x12
        sbcs  x13,  x9, x13

        ldp   x14, x15, [x3, 16]
        sbcs  x14, x10, x14
        sbcs  x15, x11, x15

        cbnz   x2, Fq_rawAddLS_done_s
        b.hs  Fq_rawAddLS_done_s

        stp    x8,  x9, [x0]
        stp   x10, x11, [x0, 16]

        b     Fq_rawAddLS_out

Fq_rawAddLS_done_s:
        stp   x12, x13, [x0]
        stp   x14, x15, [x0, 16]

Fq_rawAddLS_out:
        ret


Fq_rawSub:
_Fq_rawSub:
        ldp    x8,  x9, [x1]
        ldp    x4,  x5, [x2]
        subs   x8,  x8,  x4
        sbcs   x9,  x9,  x5

        ldp   x10, x11, [x1, 16]
        ldp    x6,  x7, [x2, 16]
        sbcs  x10, x10,  x6
        sbcs  x11, x11,  x7

        b.cs  Fq_rawSub_done

        adr    x3, Fq_rawq
        ldp    x4,  x5, [x3]
        adds   x8,  x8,  x4
        adcs   x9,  x9,  x5

        ldp    x6,  x7, [x3, 16]
        adcs  x10, x10,  x6
        adcs  x11, x11,  x7

Fq_rawSub_done:
        stp    x8,  x9, [x0]
        stp   x10, x11, [x0, 16]
        ret


Fq_rawSubSL:
_Fq_rawSubSL:
        ldp    x8,  x9, [x2]
        subs   x8,  x1,  x8
        sbcs   x9, xzr,  x9

        ldp   x10, x11, [x2, 16]
        sbcs  x10, xzr, x10
        sbcs  x11, xzr, x11

        b.cs  Fq_rawSubSL_done

        adr    x3, Fq_rawq
        ldp    x4,  x5, [x3]
        adds   x8,  x8,  x4
        adcs   x9,  x9,  x5

        ldp    x6,  x7, [x3, 16]
        adcs  x10, x10,  x6
        adcs  x11, x11,  x7

Fq_rawSubSL_done:
        stp    x8,  x9, [x0]
        stp   x10, x11, [x0, 16]
        ret


Fq_rawSubLS:
_Fq_rawSubLS:
        ldp    x8,  x9, [x1]
        subs   x8,  x8,  x2
        sbcs   x9,  x9, xzr

        ldp   x10, x11, [x1, 16]
        sbcs  x10, x10, xzr
        sbcs  x11, x11, xzr

        b.cs  Fq_rawSubLS_done

        adr    x3, Fq_rawq
        ldp    x4,  x5, [x3]
        adds   x8,  x8,  x4
        adcs   x9,  x9,  x5

        ldp    x6,  x7, [x3, 16]
        adcs  x10, x10,  x6
        adcs  x11, x11,  x7

Fq_rawSubLS_done:
        stp    x8,  x9, [x0]
        stp   x10, x11, [x0, 16]
        ret


Fq_rawSubRegular:
_Fq_rawSubRegular:
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

        ret


Fq_rawNeg:
_Fq_rawNeg:
        mov    x2, xzr
        ldp    x8,  x9, [x1]
        orr    x4,  x8,  x9
        orr    x2,  x2,  x4

        ldp   x10, x11, [x1, 16]
        orr    x5, x10, x11
        orr    x2,  x2,  x5

        cbz    x2, Fq_rawNeg_done_zero

        adr    x3, Fq_rawq
        ldp    x4,  x5, [x3]
        subs   x8,  x4,  x8
        sbcs   x9,  x5,  x9
        stp    x8,  x9, [x0]

        ldp    x6,  x7, [x3, 16]
        sbcs  x10,  x6, x10
        sbcs  x11,  x7, x11
        stp   x10, x11, [x0, 16]

        ret

Fq_rawNeg_done_zero:
        stp   xzr, xzr, [x0]
        stp   xzr, xzr, [x0, 16]

        ret


Fq_rawNegLS:
_Fq_rawNegLS:
        adr    x3, Fq_rawq
        ldp    x8,  x9, [x3]
        subs  x12,  x8,  x2
        sbcs  x13,  x9, xzr

        ldp   x10, x11, [x3, 16]
        sbcs  x14, x10, xzr
        sbcs  x15, x11, xzr

        cset   x2,  cs

        ldp    x4,  x5, [x1]
        subs  x12, x12,  x4
        sbcs  x13, x13,  x5

        ldp    x6,  x7, [x1, 16]
        sbcs  x14, x14,  x6
        sbcs  x15, x15,  x7

        cset   x3,  cs
        orr    x3,  x3,  x2

        cbz    x3, Fq_rawNegLS_done

        adds  x12, x12,  x8
        adcs  x13, x13,  x9
        adcs  x14, x14, x10
        adcs  x15, x15, x11

Fq_rawNegLS_done:
        stp   x12, x13, [x0]
        stp   x14, x15, [x0, 16]
        ret


Fq_rawMMul:
_Fq_rawMMul:
        stp   x19, x20, [sp, #-16]!
        stp   x21, x22, [sp, #-16]!

        ldp   x14, x15, [x2]
        ldp   x16, x17, [x2, 16]

        adr    x4, Fq_np
        ldr    x4, [x4]

        adr    x6, Fq_rawq
        ldp   x19, x20, [x6]
        ldp   x21, x22, [x6, 16]

        // product0 = pRawB * pRawA[0]
        ldr    x3, [x1]
        mul    x9, x14,  x3
        umulh x10, x14,  x3
        mul    x7, x15,  x3
        adds  x10, x10,  x7
        umulh x11, x15,  x3
        mul    x7, x16,  x3
        adcs  x11, x11,  x7
        umulh x12, x16,  x3
        mul    x7, x17,  x3
        adcs  x12, x12,  x7
        umulh x13, x17,  x3
        adc   x13, x13, xzr

        // np0 = Fq_np * product0[0]
        mul    x5,  x4,  x9

        // product0 = product0 + Fq_rawq * np0
        mul    x7, x19,  x5
        adds   x9,  x9,  x7
        mul    x3, x20,  x5
        adcs  x10, x10,  x3
        mul    x7, x21,  x5
        adcs  x11, x11,  x7
        mul    x3, x22,  x5
        adcs  x12, x12,  x3
        adc   x13, x13, xzr

        umulh  x7, x19,  x5
        adds  x10, x10,  x7
        umulh  x3, x20,  x5
        adcs  x11, x11,  x3
        umulh  x7, x21,  x5
        adcs  x12, x12,  x7
        umulh  x3, x22,  x5
        adcs  x13, x13,  x3
        adc    x5, xzr, xzr

        // product1 = product0 + pRawB * pRawA[1]
        ldr    x3, [x1, 8]
        mul    x9, x14,  x3
        adds   x9,  x9, x10
        mul   x10, x15,  x3
        adcs  x10, x10, x11
        mul   x11, x16,  x3
        adcs  x11, x11, x12
        mul   x12, x17,  x3
        adcs  x12, x12, x13
        adc   x13, xzr, xzr

        adds  x10, x10,  x5
        umulh  x7, x14,  x3
        adcs  x10, x10,  x7
        umulh  x5, x15,  x3
        adcs  x11, x11,  x5
        umulh  x7, x16,  x3
        adcs  x12, x12,  x7
        umulh  x5, x17,  x3
        adc   x13, x13,  x5

        // np0 = Fq_np * product1[0]
        mul    x5,  x4,  x9

        // product1 = product1 + Fq_rawq * np0
        mul    x7, x19,  x5
        adds   x9,  x9,  x7
        mul    x3, x20,  x5
        adcs  x10, x10,  x3
        mul    x7, x21,  x5
        adcs  x11, x11,  x7
        mul    x3, x22,  x5
        adcs  x12, x12,  x3
        adc   x13, x13, xzr

        umulh  x7, x19,  x5
        adds  x10, x10,  x7
        umulh  x3, x20,  x5
        adcs  x11, x11,  x3
        umulh  x7, x21,  x5
        adcs  x12, x12,  x7
        umulh  x3, x22,  x5
        adcs  x13, x13,  x3
        adc    x5, xzr, xzr

        // product2 = product1 + pRawB * pRawA[2]
        ldr    x3, [x1, 16]
        mul    x9, x14,  x3
        adds   x9,  x9, x10
        mul   x10, x15,  x3
        adcs  x10, x10, x11
        mul   x11, x16,  x3
        adcs  x11, x11, x12
        mul   x12, x17,  x3
        adcs  x12, x12, x13
        adc   x13, xzr, xzr

        adds  x10, x10,  x5
        umulh  x7, x14,  x3
        adcs  x10, x10,  x7
        umulh  x5, x15,  x3
        adcs  x11, x11,  x5
        umulh  x7, x16,  x3
        adcs  x12, x12,  x7
        umulh  x5, x17,  x3
        adc   x13, x13,  x5

        // np0 = Fq_np * product2[0]
        mul    x5,  x4,  x9

        // product2 = product2 + Fq_rawq * np0
        mul    x7, x19,  x5
        adds   x9,  x9,  x7
        mul    x3, x20,  x5
        adcs  x10, x10,  x3
        mul    x7, x21,  x5
        adcs  x11, x11,  x7
        mul    x3, x22,  x5
        adcs  x12, x12,  x3
        adc   x13, x13, xzr

        umulh  x7, x19,  x5
        adds  x10, x10,  x7
        umulh  x3, x20,  x5
        adcs  x11, x11,  x3
        umulh  x7, x21,  x5
        adcs  x12, x12,  x7
        umulh  x3, x22,  x5
        adcs  x13, x13,  x3
        adc    x5, xzr, xzr

        // product3 = product2 + pRawB * pRawA[3]
        ldr    x3, [x1, 24]
        mul    x9, x14,  x3
        adds   x9,  x9, x10
        mul   x10, x15,  x3
        adcs  x10, x10, x11
        mul   x11, x16,  x3
        adcs  x11, x11, x12
        mul   x12, x17,  x3
        adcs  x12, x12, x13
        adc   x13, xzr, xzr

        adds  x10, x10,  x5
        umulh  x7, x14,  x3
        adcs  x10, x10,  x7
        umulh  x5, x15,  x3
        adcs  x11, x11,  x5
        umulh  x7, x16,  x3
        adcs  x12, x12,  x7
        umulh  x5, x17,  x3
        adc   x13, x13,  x5

        // np0 = Fq_np * product3[0]
        mul    x5,  x4,  x9

        // product3 = product3 + Fq_rawq * np0
        mul    x7, x19,  x5
        adds   x9,  x9,  x7
        mul    x3, x20,  x5
        adcs  x10, x10,  x3
        mul    x7, x21,  x5
        adcs  x11, x11,  x7
        mul    x3, x22,  x5
        adcs  x12, x12,  x3
        adc   x13, x13, xzr

        umulh  x7, x19,  x5
        adds  x10, x10,  x7
        umulh  x3, x20,  x5
        adcs  x11, x11,  x3
        umulh  x7, x21,  x5
        adcs  x12, x12,  x7
        umulh  x3, x22,  x5
        adcs  x13, x13,  x3

        // result ge Fq_rawq
        subs  x14, x10, x19
        sbcs  x15, x11, x20
        sbcs  x16, x12, x21
        sbcs  x17, x13, x22

        csel  x10, x14, x10,  hs
        csel  x11, x15, x11,  hs
        stp   x10, x11, [x0]

        csel  x12, x16, x12,  hs
        csel  x13, x17, x13,  hs
        stp   x12, x13, [x0, 16]


        ldp   x21, x22, [sp], #16
        ldp   x19, x20, [sp], #16
        ret


Fq_rawMMul1:
_Fq_rawMMul1:
        stp   x19, x20, [sp, #-16]!
        stp   x21, x22, [sp, #-16]!

        ldp   x14, x15, [x1]
        ldp   x16, x17, [x1, 16]

        adr    x4, Fq_np
        ldr    x4, [x4]

        adr    x6, Fq_rawq
        ldp   x19, x20, [x6]
        ldp   x21, x22, [x6, 16]

        // product0 = pRawB * pRawA
        mul    x9, x14,  x2
        umulh x10, x14,  x2
        mul    x7, x15,  x2
        adds  x10, x10,  x7
        umulh x11, x15,  x2
        mul    x7, x16,  x2
        adcs  x11, x11,  x7
        umulh x12, x16,  x2
        mul    x7, x17,  x2
        adcs  x12, x12,  x7
        umulh x13, x17,  x2
        adc   x13, x13, xzr

        // np0 = Fq_np * product0[0]
        mul    x5,  x4,  x9
        // product0 = product0 + Fq_rawq * np0
        mul    x7, x19,  x5
        adds   x9,  x9,  x7
        mul    x3, x20,  x5
        adcs  x10, x10,  x3
        mul    x7, x21,  x5
        adcs  x11, x11,  x7
        mul    x3, x22,  x5
        adcs  x12, x12,  x3
        adc   x13, x13, xzr

        umulh  x7, x19,  x5
        adds  x10, x10,  x7
        umulh  x3, x20,  x5
        adcs  x11, x11,  x3
        umulh  x7, x21,  x5
        adcs  x12, x12,  x7
        umulh  x3, x22,  x5
        adcs  x13, x13,  x3
        adc    x8, xzr, xzr

        // np0 = Fq_np * product1[0]
        mul    x5,  x4, x10
        // product1 = product1 + Fq_rawq * np0
        mul    x7, x19,  x5
        adds   x9, x10,  x7
        mul    x3, x20,  x5
        adcs  x10, x11,  x3
        mul    x7, x21,  x5
        adcs  x11, x12,  x7
        mul    x3, x22,  x5
        adcs  x12, x13,  x3
        adc   x13, xzr, xzr

        adds  x10, x10,  x8
        umulh  x7, x19,  x5
        adds  x10, x10,  x7
        umulh  x3, x20,  x5
        adcs  x11, x11,  x3
        umulh  x7, x21,  x5
        adcs  x12, x12,  x7
        umulh  x3, x22,  x5
        adcs  x13, x13,  x3
        adc    x8, xzr, xzr

        // np0 = Fq_np * product2[0]
        mul    x5,  x4, x10
        // product2 = product2 + Fq_rawq * np0
        mul    x7, x19,  x5
        adds   x9, x10,  x7
        mul    x3, x20,  x5
        adcs  x10, x11,  x3
        mul    x7, x21,  x5
        adcs  x11, x12,  x7
        mul    x3, x22,  x5
        adcs  x12, x13,  x3
        adc   x13, xzr, xzr

        adds  x10, x10,  x8
        umulh  x7, x19,  x5
        adds  x10, x10,  x7
        umulh  x3, x20,  x5
        adcs  x11, x11,  x3
        umulh  x7, x21,  x5
        adcs  x12, x12,  x7
        umulh  x3, x22,  x5
        adcs  x13, x13,  x3
        adc    x8, xzr, xzr

        // np0 = Fq_np * product3[0]
        mul    x5,  x4, x10
        // product3 = product3 + Fq_rawq * np0
        mul    x7, x19,  x5
        adds   x9, x10,  x7
        mul    x3, x20,  x5
        adcs  x10, x11,  x3
        mul    x7, x21,  x5
        adcs  x11, x12,  x7
        mul    x3, x22,  x5
        adcs  x12, x13,  x3
        adc   x13, xzr, xzr

        adds  x10, x10,  x8
        umulh  x7, x19,  x5
        adds  x10, x10,  x7
        umulh  x3, x20,  x5
        adcs  x11, x11,  x3
        umulh  x7, x21,  x5
        adcs  x12, x12,  x7
        umulh  x3, x22,  x5
        adcs  x13, x13,  x3

        // result ge Fq_rawq
        subs  x14, x10, x19
        sbcs  x15, x11, x20
        sbcs  x16, x12, x21
        sbcs  x17, x13, x22

        csel  x10, x14, x10,  hs
        csel  x11, x15, x11,  hs
        stp   x10, x11, [x0]

        csel  x12, x16, x12,  hs
        csel  x13, x17, x13,  hs
        stp   x12, x13, [x0, 16]


        ldp   x21, x22, [sp], #16
        ldp   x19, x20, [sp], #16
        ret


Fq_rawFromMontgomery:
_Fq_rawFromMontgomery:
        stp   x19, x20, [sp, #-16]!
        stp   x21, x22, [sp, #-16]!

        ldp    x9, x10, [x1]
        ldp   x11, x12, [x1, 16]
        mov   x13, xzr

        adr    x4, Fq_np
        ldr    x4, [x4]

        adr    x6, Fq_rawq
        ldp   x19, x20, [x6]
        ldp   x21, x22, [x6, 16]

        // np0 = Fq_np * product0[0]
        mul    x5,  x4,  x9
        // product0 = product0 + Fq_rawq * np0
        mul    x7, x19,  x5
        adds   x9,  x9,  x7
        mul    x3, x20,  x5
        adcs  x10, x10,  x3
        mul    x7, x21,  x5
        adcs  x11, x11,  x7
        mul    x3, x22,  x5
        adcs  x12, x12,  x3
        adc   x13, x13, xzr

        umulh  x7, x19,  x5
        adds  x10, x10,  x7
        umulh  x3, x20,  x5
        adcs  x11, x11,  x3
        umulh  x7, x21,  x5
        adcs  x12, x12,  x7
        umulh  x3, x22,  x5
        adcs  x13, x13,  x3
        adc    x8, xzr, xzr

        // np0 = Fq_np * product1[0]
        mul    x5,  x4, x10
        // product1 = product1 + Fq_rawq * np0
        mul    x7, x19,  x5
        adds   x9, x10,  x7
        mul    x3, x20,  x5
        adcs  x10, x11,  x3
        mul    x7, x21,  x5
        adcs  x11, x12,  x7
        mul    x3, x22,  x5
        adcs  x12, x13,  x3
        adc   x13, xzr, xzr

        adds  x10, x10,  x8
        umulh  x7, x19,  x5
        adds  x10, x10,  x7
        umulh  x3, x20,  x5
        adcs  x11, x11,  x3
        umulh  x7, x21,  x5
        adcs  x12, x12,  x7
        umulh  x3, x22,  x5
        adcs  x13, x13,  x3
        adc    x8, xzr, xzr

        // np0 = Fq_np * product2[0]
        mul    x5,  x4, x10
        // product2 = product2 + Fq_rawq * np0
        mul    x7, x19,  x5
        adds   x9, x10,  x7
        mul    x3, x20,  x5
        adcs  x10, x11,  x3
        mul    x7, x21,  x5
        adcs  x11, x12,  x7
        mul    x3, x22,  x5
        adcs  x12, x13,  x3
        adc   x13, xzr, xzr

        adds  x10, x10,  x8
        umulh  x7, x19,  x5
        adds  x10, x10,  x7
        umulh  x3, x20,  x5
        adcs  x11, x11,  x3
        umulh  x7, x21,  x5
        adcs  x12, x12,  x7
        umulh  x3, x22,  x5
        adcs  x13, x13,  x3
        adc    x8, xzr, xzr

        // np0 = Fq_np * product3[0]
        mul    x5,  x4, x10
        // product3 = product3 + Fq_rawq * np0
        mul    x7, x19,  x5
        adds   x9, x10,  x7
        mul    x3, x20,  x5
        adcs  x10, x11,  x3
        mul    x7, x21,  x5
        adcs  x11, x12,  x7
        mul    x3, x22,  x5
        adcs  x12, x13,  x3
        adc   x13, xzr, xzr

        adds  x10, x10,  x8
        umulh  x7, x19,  x5
        adds  x10, x10,  x7
        umulh  x3, x20,  x5
        adcs  x11, x11,  x3
        umulh  x7, x21,  x5
        adcs  x12, x12,  x7
        umulh  x3, x22,  x5
        adcs  x13, x13,  x3

        // result ge Fq_rawq
        subs  x14, x10, x19
        sbcs  x15, x11, x20
        sbcs  x16, x12, x21
        sbcs  x17, x13, x22

        csel  x10, x14, x10,  hs
        csel  x11, x15, x11,  hs
        stp   x10, x11, [x0]

        csel  x12, x16, x12,  hs
        csel  x13, x17, x13,  hs
        stp   x12, x13, [x0, 16]


        ldp   x21, x22, [sp], #16
        ldp   x19, x20, [sp], #16
        ret


Fq_rawIsZero:
_Fq_rawIsZero:
        ldp    x1,  x2, [x0]
        orr    x3,  x1,  x2

        ldp    x4,  x5, [x0, 16]
        orr    x6,  x4,  x5
        orr    x7,  x3,  x6

        cmp    x7, xzr
        cset   x0,  eq
        ret

Fq_rawIsEq:
_Fq_rawIsEq:
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

        cmp    x4, xzr
        cset   x0,  eq
        ret

Fq_rawCmp:
_Fq_rawCmp:
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

        cneg   x0,  x2,  lo
        ret

Fq_rawCopy:
_Fq_rawCopy:
        ldp    x2,  x3, [x1]
        stp    x2,  x3, [x0]

        ldp    x4,  x5, [x1, 16]
        stp    x4,  x5, [x0, 16]

        ret

Fq_rawCopyS2L:
_Fq_rawCopyS2L:
        cmp    x1, xzr
        b.lt  Fq_rawCopyS2L_adjust_neg

        stp    x1, xzr, [x0]
        stp   xzr, xzr, [x0, 16]
        ret

Fq_rawCopyS2L_adjust_neg:
        mov    x2,  -1
        adr    x3, Fq_rawq

        ldp    x4,  x5, [x3]
        adds  x10,  x1,  x4
        adcs  x11,  x2,  x5
        stp   x10, x11, [x0]

        ldp    x6,  x7, [x3, 16]
        adcs  x12,  x2,  x6
        adcs  x13,  x2,  x7
        stp   x12, x13, [x0, 16]

        ret

Fq_rawSwap:
_Fq_rawSwap:
        ldp    x2,  x3, [x0]
        ldp   x10, x11, [x1]
        stp    x2,  x3, [x1]
        stp   x10, x11, [x0]

        ldp    x4,  x5, [x0, 16]
        ldp   x12, x13, [x1, 16]
        stp    x4,  x5, [x1, 16]
        stp   x12, x13, [x0, 16]

        ret

Fq_rawAnd:
_Fq_rawAnd:
        ldp    x8,  x9, [x1]
        ldp    x4,  x5, [x2]
        and    x8,  x8,  x4
        and    x9,  x9,  x5

        ldp   x10, x11, [x1, 16]
        ldp    x6,  x7, [x2, 16]
        and   x10, x10,  x6
        and   x11, x11,  x7

        adr    x2, Fq_lboMask
        ldr    x2, [x2]
        and   x11, x11,  x2

        adr    x3, Fq_rawq
        ldp   x12, x13, [x3]
        subs  x12,  x8, x12
        sbcs  x13,  x9, x13

        ldp   x14, x15, [x3, 16]
        sbcs  x14, x10, x14
        sbcs  x15, x11, x15

        csel   x8, x12,  x8,  hs
        csel   x9, x13,  x9,  hs
        stp    x8,  x9, [x0]

        csel  x10, x14, x10,  hs
        csel  x11, x15, x11,  hs
        stp   x10, x11, [x0, 16]

        ret

Fq_rawOr:
_Fq_rawOr:
        ldp    x8,  x9, [x1]
        ldp    x4,  x5, [x2]
        orr    x8,  x8,  x4
        orr    x9,  x9,  x5

        ldp   x10, x11, [x1, 16]
        ldp    x6,  x7, [x2, 16]
        orr   x10, x10,  x6
        orr   x11, x11,  x7

        adr    x2, Fq_lboMask
        ldr    x2, [x2]
        and   x11, x11,  x2

        adr    x3, Fq_rawq
        ldp   x12, x13, [x3]
        subs  x12,  x8, x12
        sbcs  x13,  x9, x13

        ldp   x14, x15, [x3, 16]
        sbcs  x14, x10, x14
        sbcs  x15, x11, x15

        csel   x8, x12,  x8,  hs
        csel   x9, x13,  x9,  hs
        stp    x8,  x9, [x0]

        csel  x10, x14, x10,  hs
        csel  x11, x15, x11,  hs
        stp   x10, x11, [x0, 16]

        ret

Fq_rawXor:
_Fq_rawXor:
        ldp    x8,  x9, [x1]
        ldp    x4,  x5, [x2]
        eor    x8,  x8,  x4
        eor    x9,  x9,  x5

        ldp   x10, x11, [x1, 16]
        ldp    x6,  x7, [x2, 16]
        eor   x10, x10,  x6
        eor   x11, x11,  x7

        adr    x2, Fq_lboMask
        ldr    x2, [x2]
        and   x11, x11,  x2

        adr    x3, Fq_rawq
        ldp   x12, x13, [x3]
        subs  x12,  x8, x12
        sbcs  x13,  x9, x13

        ldp   x14, x15, [x3, 16]
        sbcs  x14, x10, x14
        sbcs  x15, x11, x15

        csel   x8, x12,  x8,  hs
        csel   x9, x13,  x9,  hs
        stp    x8,  x9, [x0]

        csel  x10, x14, x10,  hs
        csel  x11, x15, x11,  hs
        stp   x10, x11, [x0, 16]

        ret

Fq_rawNot:
_Fq_rawNot:
        ldp    x8,  x9, [x1]
        mvn    x8,  x8
        mvn    x9,  x9

        ldp   x10, x11, [x1, 16]
        mvn   x10, x10
        mvn   x11, x11

        adr    x2, Fq_lboMask
        ldr    x2, [x2]
        and   x11, x11,  x2

        adr    x3, Fq_rawq
        ldp   x12, x13, [x3]
        subs  x12,  x8, x12
        sbcs  x13,  x9, x13

        ldp   x14, x15, [x3, 16]
        sbcs  x14, x10, x14
        sbcs  x15, x11, x15

        csel   x8, x12,  x8,  hs
        csel   x9, x13,  x9,  hs
        stp    x8,  x9, [x0]

        csel  x10, x14, x10,  hs
        csel  x11, x15, x11,  hs
        stp   x10, x11, [x0, 16]

        ret

Fq_rawShr:
_Fq_rawShr:
        ldp    x8,  x9, [x1]
        ldp   x10, x11, [x1, 16]

        and    x3,  x2, 0x3f
        mov    x4, 0x3f
        sub    x4,  x4,  x3

        lsr    x2,  x2,  #6
        adr    x5, Fq_rawShr_word_shift
        ldr    x5, [x5, x2, lsl 3]
        br     x5

Fq_rawShr_word_shift_0:
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

        stp    x8,  x9, [x0]
        stp   x10, x11, [x0, 16]
        ret

Fq_rawShr_word_shift_1:
        lsr    x9,  x9,  x3
        lsl    x7, x10,  x4
        orr    x9,  x9,  x7, lsl #1

        lsr   x10, x10,  x3
        lsl    x6, x11,  x4
        orr   x10, x10,  x6, lsl #1

        lsr   x11, x11,  x3

        stp    x9, x10, [x0]
        stp   x11, xzr, [x0, 16]
        ret

Fq_rawShr_word_shift_2:
        lsr   x10, x10,  x3
        lsl    x7, x11,  x4
        orr   x10, x10,  x7, lsl #1

        lsr   x11, x11,  x3

        stp   x10, x11, [x0]
        stp   xzr, xzr, [x0, 16]
        ret

Fq_rawShr_word_shift_3:
        lsr   x11, x11,  x3

        stp   x11, xzr, [x0]
        stp   xzr, xzr, [x0, 16]
        ret

Fq_rawShr_word_shift:
        .quad Fq_rawShr_word_shift_0
        .quad Fq_rawShr_word_shift_1
        .quad Fq_rawShr_word_shift_2
        .quad Fq_rawShr_word_shift_3


Fq_rawShl:
_Fq_rawShl:
        ldp    x9, x10, [x1]
        ldp   x11, x12, [x1, 16]

        and    x3,  x2, 0x3f
        mov    x4, 0x3f
        sub    x4,  x4,  x3

        lsr    x2,  x2,  #6
        adr    x5, Fq_rawShl_word_shift
        ldr    x5, [x5, x2, lsl 3]
        br     x5

Fq_rawShl_word_shift_0:
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

        b     Fq_rawShl_sub

Fq_rawShl_word_shift_1:
        lsl   x12, x11,  x3
        lsr    x8, x10,  x4
        orr   x12, x12,  x8, lsr #1

        lsl   x11, x10,  x3
        lsr    x7,  x9,  x4
        orr   x11, x11,  x7, lsr #1

        lsl   x10,  x9,  x3
        mov    x9, xzr

        b     Fq_rawShl_sub

Fq_rawShl_word_shift_2:
        lsl   x12, x10,  x3
        lsr    x8,  x9,  x4
        orr   x12, x12,  x8, lsr #1

        lsl   x11,  x9,  x3
        mov   x10, xzr
        mov    x9, xzr

        b     Fq_rawShl_sub

Fq_rawShl_word_shift_3:
        lsl   x12,  x9,  x3
        mov   x11, xzr
        mov   x10, xzr
        mov    x9, xzr

Fq_rawShl_sub:
        adr    x6, Fq_lboMask
        ldr    x6, [x6]
        and   x12, x12,  x6

        adr    x1, Fq_rawq
        ldp   x13, x14, [x1]
        subs  x13,  x9, x13
        sbcs  x14, x10, x14

        ldp   x15, x16, [x1, 16]
        sbcs  x15, x11, x15
        sbcs  x16, x12, x16

        csel   x9, x13,  x9,  hs
        csel  x10, x14, x10,  hs
        stp    x9, x10, [x0]

        csel  x11, x15, x11,  hs
        csel  x12, x16, x12,  hs
        stp   x11, x12, [x0, 16]

        ret
Fq_rawShl_word_shift:
        .quad Fq_rawShl_word_shift_0
        .quad Fq_rawShl_word_shift_1
        .quad Fq_rawShl_word_shift_2
        .quad Fq_rawShl_word_shift_3




    .align 8
Fq_rawq:    .quad 0x3c208c16d87cfd47,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029
Fq_np:      .quad 0x87d20782e4866389
Fq_lboMask: .quad 0x3fffffffffffffff
