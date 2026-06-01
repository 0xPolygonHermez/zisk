    .global Fnec_rawAdd
    .global Fnec_rawAddLS
    .global Fnec_rawSub
    .global Fnec_rawSubRegular
    .global Fnec_rawNeg
    .global Fnec_rawNegLS
    .global Fnec_rawSubSL
    .global Fnec_rawSubLS
    .global Fnec_rawMMul
    .global Fnec_rawMMul1
    .global Fnec_rawFromMontgomery
    .global Fnec_rawCopy
    .global Fnec_rawSwap
    .global Fnec_rawIsEq
    .global Fnec_rawIsZero
    .global Fnec_rawCopyS2L
    .global Fnec_rawCmp
    .global Fnec_rawAnd
    .global Fnec_rawOr
    .global Fnec_rawXor
    .global Fnec_rawShr
    .global Fnec_rawShl
    .global Fnec_rawNot

    .global _Fnec_rawAdd
    .global _Fnec_rawAddLS
    .global _Fnec_rawSub
    .global _Fnec_rawSubRegular
    .global _Fnec_rawNeg
    .global _Fnec_rawNegLS
    .global _Fnec_rawSubSL
    .global _Fnec_rawSubLS
    .global _Fnec_rawMMul
    .global _Fnec_rawMMul1
    .global _Fnec_rawFromMontgomery
    .global _Fnec_rawCopy
    .global _Fnec_rawSwap
    .global _Fnec_rawIsEq
    .global _Fnec_rawIsZero
    .global _Fnec_rawCopyS2L
    .global _Fnec_rawCmp
    .global _Fnec_rawAnd
    .global _Fnec_rawOr
    .global _Fnec_rawXor
    .global _Fnec_rawShr
    .global _Fnec_rawShl
    .global _Fnec_rawNot

    .text
    .align 4

Fnec_rawAdd:
_Fnec_rawAdd:
        ldp    x8,  x9, [x1]
        ldp    x4,  x5, [x2]
        adds   x8,  x8,  x4
        adcs   x9,  x9,  x5

        ldp   x10, x11, [x1, 16]
        ldp    x6,  x7, [x2, 16]
        adcs  x10, x10,  x6
        adcs  x11, x11,  x7

        cset   x2,  cs

        adr    x3, Fnec_rawq
        ldp   x12, x13, [x3]
        subs  x12,  x8, x12
        sbcs  x13,  x9, x13

        ldp   x14, x15, [x3, 16]
        sbcs  x14, x10, x14
        sbcs  x15, x11, x15

        cbnz   x2, Fnec_rawAdd_done_s
        b.hs  Fnec_rawAdd_done_s

        stp    x8,  x9, [x0]
        stp   x10, x11, [x0, 16]

        b     Fnec_rawAdd_out

Fnec_rawAdd_done_s:
        stp   x12, x13, [x0]
        stp   x14, x15, [x0, 16]

Fnec_rawAdd_out:
        ret


Fnec_rawAddLS:
_Fnec_rawAddLS:
        ldp    x8,  x9, [x1]
        adds   x8,  x8,  x2
        adcs   x9,  x9, xzr

        ldp   x10, x11, [x1, 16]
        adcs  x10, x10, xzr
        adcs  x11, x11, xzr

        cset   x2,  cs

        adr    x3, Fnec_rawq
        ldp   x12, x13, [x3]
        subs  x12,  x8, x12
        sbcs  x13,  x9, x13

        ldp   x14, x15, [x3, 16]
        sbcs  x14, x10, x14
        sbcs  x15, x11, x15

        cbnz   x2, Fnec_rawAddLS_done_s
        b.hs  Fnec_rawAddLS_done_s

        stp    x8,  x9, [x0]
        stp   x10, x11, [x0, 16]

        b     Fnec_rawAddLS_out

Fnec_rawAddLS_done_s:
        stp   x12, x13, [x0]
        stp   x14, x15, [x0, 16]

Fnec_rawAddLS_out:
        ret


Fnec_rawSub:
_Fnec_rawSub:
        ldp    x8,  x9, [x1]
        ldp    x4,  x5, [x2]
        subs   x8,  x8,  x4
        sbcs   x9,  x9,  x5

        ldp   x10, x11, [x1, 16]
        ldp    x6,  x7, [x2, 16]
        sbcs  x10, x10,  x6
        sbcs  x11, x11,  x7

        b.cs  Fnec_rawSub_done

        adr    x3, Fnec_rawq
        ldp    x4,  x5, [x3]
        adds   x8,  x8,  x4
        adcs   x9,  x9,  x5

        ldp    x6,  x7, [x3, 16]
        adcs  x10, x10,  x6
        adcs  x11, x11,  x7

Fnec_rawSub_done:
        stp    x8,  x9, [x0]
        stp   x10, x11, [x0, 16]
        ret


Fnec_rawSubSL:
_Fnec_rawSubSL:
        ldp    x8,  x9, [x2]
        subs   x8,  x1,  x8
        sbcs   x9, xzr,  x9

        ldp   x10, x11, [x2, 16]
        sbcs  x10, xzr, x10
        sbcs  x11, xzr, x11

        b.cs  Fnec_rawSubSL_done

        adr    x3, Fnec_rawq
        ldp    x4,  x5, [x3]
        adds   x8,  x8,  x4
        adcs   x9,  x9,  x5

        ldp    x6,  x7, [x3, 16]
        adcs  x10, x10,  x6
        adcs  x11, x11,  x7

Fnec_rawSubSL_done:
        stp    x8,  x9, [x0]
        stp   x10, x11, [x0, 16]
        ret


Fnec_rawSubLS:
_Fnec_rawSubLS:
        ldp    x8,  x9, [x1]
        subs   x8,  x8,  x2
        sbcs   x9,  x9, xzr

        ldp   x10, x11, [x1, 16]
        sbcs  x10, x10, xzr
        sbcs  x11, x11, xzr

        b.cs  Fnec_rawSubLS_done

        adr    x3, Fnec_rawq
        ldp    x4,  x5, [x3]
        adds   x8,  x8,  x4
        adcs   x9,  x9,  x5

        ldp    x6,  x7, [x3, 16]
        adcs  x10, x10,  x6
        adcs  x11, x11,  x7

Fnec_rawSubLS_done:
        stp    x8,  x9, [x0]
        stp   x10, x11, [x0, 16]
        ret


Fnec_rawSubRegular:
_Fnec_rawSubRegular:
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


Fnec_rawNeg:
_Fnec_rawNeg:
        mov    x2, xzr
        ldp    x8,  x9, [x1]
        orr    x4,  x8,  x9
        orr    x2,  x2,  x4

        ldp   x10, x11, [x1, 16]
        orr    x5, x10, x11
        orr    x2,  x2,  x5

        cbz    x2, Fnec_rawNeg_done_zero

        adr    x3, Fnec_rawq
        ldp    x4,  x5, [x3]
        subs   x8,  x4,  x8
        sbcs   x9,  x5,  x9
        stp    x8,  x9, [x0]

        ldp    x6,  x7, [x3, 16]
        sbcs  x10,  x6, x10
        sbcs  x11,  x7, x11
        stp   x10, x11, [x0, 16]

        ret

Fnec_rawNeg_done_zero:
        stp   xzr, xzr, [x0]
        stp   xzr, xzr, [x0, 16]

        ret


Fnec_rawNegLS:
_Fnec_rawNegLS:
        adr    x3, Fnec_rawq
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

        cbz    x3, Fnec_rawNegLS_done

        adds  x12, x12,  x8
        adcs  x13, x13,  x9
        adcs  x14, x14, x10
        adcs  x15, x15, x11

Fnec_rawNegLS_done:
        stp   x12, x13, [x0]
        stp   x14, x15, [x0, 16]
        ret


Fnec_rawMMul:
_Fnec_rawMMul:
        stp   x19, x20, [sp, #-16]!
        stp   x21, x22, [sp, #-16]!

        ldp   x14, x15, [x2]
        ldp   x16, x17, [x2, 16]

        adr    x4, Fnec_np
        ldr    x4, [x4]

        adr    x6, Fnec_rawq
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
        adcs  x13, x13, xzr
        adc    x8, xzr, xzr

        umulh  x7, x19,  x5
        adds  x10, x10,  x7
        umulh  x3, x20,  x5
        adcs  x11, x11,  x3
        umulh  x7, x21,  x5
        adcs  x12, x12,  x7
        umulh  x3, x22,  x5
        adcs  x13, x13,  x3
        adc    x8,  x8, xzr

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
        adcs  x13, xzr,  x8
        adc    x8, xzr, xzr

        umulh  x7, x14,  x3
        adds  x10, x10,  x7
        umulh  x5, x15,  x3
        adcs  x11, x11,  x5
        umulh  x7, x16,  x3
        adcs  x12, x12,  x7
        umulh  x5, x17,  x3
        adcs  x13, x13,  x5
        adc    x8,  x8, xzr

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
        adcs  x13, x13, xzr
        adc    x8,  x8, xzr

        umulh  x7, x19,  x5
        adds  x10, x10,  x7
        umulh  x3, x20,  x5
        adcs  x11, x11,  x3
        umulh  x7, x21,  x5
        adcs  x12, x12,  x7
        umulh  x3, x22,  x5
        adcs  x13, x13,  x3
        adc    x8,  x8, xzr

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
        adcs  x13, xzr,  x8
        adc    x8, xzr, xzr

        umulh  x7, x14,  x3
        adds  x10, x10,  x7
        umulh  x5, x15,  x3
        adcs  x11, x11,  x5
        umulh  x7, x16,  x3
        adcs  x12, x12,  x7
        umulh  x5, x17,  x3
        adcs  x13, x13,  x5
        adc    x8,  x8, xzr

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
        adcs  x13, x13, xzr
        adc    x8,  x8, xzr

        umulh  x7, x19,  x5
        adds  x10, x10,  x7
        umulh  x3, x20,  x5
        adcs  x11, x11,  x3
        umulh  x7, x21,  x5
        adcs  x12, x12,  x7
        umulh  x3, x22,  x5
        adcs  x13, x13,  x3
        adc    x8,  x8, xzr

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
        adcs  x13, xzr,  x8
        adc    x8, xzr, xzr

        umulh  x7, x14,  x3
        adds  x10, x10,  x7
        umulh  x5, x15,  x3
        adcs  x11, x11,  x5
        umulh  x7, x16,  x3
        adcs  x12, x12,  x7
        umulh  x5, x17,  x3
        adcs  x13, x13,  x5
        adc    x8,  x8, xzr

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
        adcs  x13, x13, xzr
        adc    x8,  x8, xzr

        umulh  x7, x19,  x5
        adds  x10, x10,  x7
        umulh  x3, x20,  x5
        adcs  x11, x11,  x3
        umulh  x7, x21,  x5
        adcs  x12, x12,  x7
        umulh  x3, x22,  x5
        adcs  x13, x13,  x3
        adc    x8,  x8, xzr

        // result ge Fnec_rawq
        subs  x14, x10, x19
        sbcs  x15, x11, x20
        sbcs  x16, x12, x21
        sbcs  x17, x13, x22

        cinc   x8,  x8,  hs
        cmp    x8,   1

        csel  x10, x14, x10,  hs
        csel  x11, x15, x11,  hs
        stp   x10, x11, [x0]

        csel  x12, x16, x12,  hs
        csel  x13, x17, x13,  hs
        stp   x12, x13, [x0, 16]


        ldp   x21, x22, [sp], #16
        ldp   x19, x20, [sp], #16
        ret


Fnec_rawMMul1:
_Fnec_rawMMul1:
        stp   x19, x20, [sp, #-16]!
        stp   x21, x22, [sp, #-16]!

        ldp   x14, x15, [x1]
        ldp   x16, x17, [x1, 16]

        adr    x4, Fnec_np
        ldr    x4, [x4]

        adr    x6, Fnec_rawq
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
        adcs  x13, xzr,  x8
        adc    x8, xzr, xzr

        umulh  x7, x19,  x5
        adds  x10, x10,  x7
        umulh  x3, x20,  x5
        adcs  x11, x11,  x3
        umulh  x7, x21,  x5
        adcs  x12, x12,  x7
        umulh  x3, x22,  x5
        adcs  x13, x13,  x3
        adc    x8,  x8, xzr

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
        adcs  x13, xzr,  x8
        adc    x8, xzr, xzr

        umulh  x7, x19,  x5
        adds  x10, x10,  x7
        umulh  x3, x20,  x5
        adcs  x11, x11,  x3
        umulh  x7, x21,  x5
        adcs  x12, x12,  x7
        umulh  x3, x22,  x5
        adcs  x13, x13,  x3
        adc    x8,  x8, xzr

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
        adcs  x13, xzr,  x8
        adc    x8, xzr, xzr

        umulh  x7, x19,  x5
        adds  x10, x10,  x7
        umulh  x3, x20,  x5
        adcs  x11, x11,  x3
        umulh  x7, x21,  x5
        adcs  x12, x12,  x7
        umulh  x3, x22,  x5
        adcs  x13, x13,  x3
        adc    x8,  x8, xzr

        // result ge Fnec_rawq
        subs  x14, x10, x19
        sbcs  x15, x11, x20
        sbcs  x16, x12, x21
        sbcs  x17, x13, x22

        cinc   x8,  x8,  hs
        cmp    x8,   1

        csel  x10, x14, x10,  hs
        csel  x11, x15, x11,  hs
        stp   x10, x11, [x0]

        csel  x12, x16, x12,  hs
        csel  x13, x17, x13,  hs
        stp   x12, x13, [x0, 16]


        ldp   x21, x22, [sp], #16
        ldp   x19, x20, [sp], #16
        ret


Fnec_rawFromMontgomery:
_Fnec_rawFromMontgomery:
        stp   x19, x20, [sp, #-16]!
        stp   x21, x22, [sp, #-16]!

        ldp    x9, x10, [x1]
        ldp   x11, x12, [x1, 16]
        mov   x13, xzr

        adr    x4, Fnec_np
        ldr    x4, [x4]

        adr    x6, Fnec_rawq
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
        adcs  x13, xzr,  x8
        adc    x8, xzr, xzr

        umulh  x7, x19,  x5
        adds  x10, x10,  x7
        umulh  x3, x20,  x5
        adcs  x11, x11,  x3
        umulh  x7, x21,  x5
        adcs  x12, x12,  x7
        umulh  x3, x22,  x5
        adcs  x13, x13,  x3
        adc    x8,  x8, xzr

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
        adcs  x13, xzr,  x8
        adc    x8, xzr, xzr

        umulh  x7, x19,  x5
        adds  x10, x10,  x7
        umulh  x3, x20,  x5
        adcs  x11, x11,  x3
        umulh  x7, x21,  x5
        adcs  x12, x12,  x7
        umulh  x3, x22,  x5
        adcs  x13, x13,  x3
        adc    x8,  x8, xzr

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
        adcs  x13, xzr,  x8
        adc    x8, xzr, xzr

        umulh  x7, x19,  x5
        adds  x10, x10,  x7
        umulh  x3, x20,  x5
        adcs  x11, x11,  x3
        umulh  x7, x21,  x5
        adcs  x12, x12,  x7
        umulh  x3, x22,  x5
        adcs  x13, x13,  x3
        adc    x8,  x8, xzr

        // result ge Fnec_rawq
        subs  x14, x10, x19
        sbcs  x15, x11, x20
        sbcs  x16, x12, x21
        sbcs  x17, x13, x22

        cinc   x8,  x8,  hs
        cmp    x8,   1

        csel  x10, x14, x10,  hs
        csel  x11, x15, x11,  hs
        stp   x10, x11, [x0]

        csel  x12, x16, x12,  hs
        csel  x13, x17, x13,  hs
        stp   x12, x13, [x0, 16]


        ldp   x21, x22, [sp], #16
        ldp   x19, x20, [sp], #16
        ret


Fnec_rawIsZero:
_Fnec_rawIsZero:
        ldp    x1,  x2, [x0]
        orr    x3,  x1,  x2

        ldp    x4,  x5, [x0, 16]
        orr    x6,  x4,  x5
        orr    x7,  x3,  x6

        cmp    x7, xzr
        cset   x0,  eq
        ret

Fnec_rawIsEq:
_Fnec_rawIsEq:
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

Fnec_rawCmp:
_Fnec_rawCmp:
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

Fnec_rawCopy:
_Fnec_rawCopy:
        ldp    x2,  x3, [x1]
        stp    x2,  x3, [x0]

        ldp    x4,  x5, [x1, 16]
        stp    x4,  x5, [x0, 16]

        ret

Fnec_rawCopyS2L:
_Fnec_rawCopyS2L:
        cmp    x1, xzr
        b.lt  Fnec_rawCopyS2L_adjust_neg

        stp    x1, xzr, [x0]
        stp   xzr, xzr, [x0, 16]
        ret

Fnec_rawCopyS2L_adjust_neg:
        mov    x2,  -1
        adr    x3, Fnec_rawq

        ldp    x4,  x5, [x3]
        adds  x10,  x1,  x4
        adcs  x11,  x2,  x5
        stp   x10, x11, [x0]

        ldp    x6,  x7, [x3, 16]
        adcs  x12,  x2,  x6
        adcs  x13,  x2,  x7
        stp   x12, x13, [x0, 16]

        ret

Fnec_rawSwap:
_Fnec_rawSwap:
        ldp    x2,  x3, [x0]
        ldp   x10, x11, [x1]
        stp    x2,  x3, [x1]
        stp   x10, x11, [x0]

        ldp    x4,  x5, [x0, 16]
        ldp   x12, x13, [x1, 16]
        stp    x4,  x5, [x1, 16]
        stp   x12, x13, [x0, 16]

        ret

Fnec_rawAnd:
_Fnec_rawAnd:
        ldp    x8,  x9, [x1]
        ldp    x4,  x5, [x2]
        and    x8,  x8,  x4
        and    x9,  x9,  x5

        ldp   x10, x11, [x1, 16]
        ldp    x6,  x7, [x2, 16]
        and   x10, x10,  x6
        and   x11, x11,  x7

        adr    x2, Fnec_lboMask
        ldr    x2, [x2]
        and   x11, x11,  x2

        adr    x3, Fnec_rawq
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

Fnec_rawOr:
_Fnec_rawOr:
        ldp    x8,  x9, [x1]
        ldp    x4,  x5, [x2]
        orr    x8,  x8,  x4
        orr    x9,  x9,  x5

        ldp   x10, x11, [x1, 16]
        ldp    x6,  x7, [x2, 16]
        orr   x10, x10,  x6
        orr   x11, x11,  x7

        adr    x2, Fnec_lboMask
        ldr    x2, [x2]
        and   x11, x11,  x2

        adr    x3, Fnec_rawq
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

Fnec_rawXor:
_Fnec_rawXor:
        ldp    x8,  x9, [x1]
        ldp    x4,  x5, [x2]
        eor    x8,  x8,  x4
        eor    x9,  x9,  x5

        ldp   x10, x11, [x1, 16]
        ldp    x6,  x7, [x2, 16]
        eor   x10, x10,  x6
        eor   x11, x11,  x7

        adr    x2, Fnec_lboMask
        ldr    x2, [x2]
        and   x11, x11,  x2

        adr    x3, Fnec_rawq
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

Fnec_rawNot:
_Fnec_rawNot:
        ldp    x8,  x9, [x1]
        mvn    x8,  x8
        mvn    x9,  x9

        ldp   x10, x11, [x1, 16]
        mvn   x10, x10
        mvn   x11, x11

        adr    x2, Fnec_lboMask
        ldr    x2, [x2]
        and   x11, x11,  x2

        adr    x3, Fnec_rawq
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

Fnec_rawShr:
_Fnec_rawShr:
        ldp    x8,  x9, [x1]
        ldp   x10, x11, [x1, 16]

        and    x3,  x2, 0x3f
        mov    x4, 0x3f
        sub    x4,  x4,  x3

        lsr    x2,  x2,  #6
        adr    x5, Fnec_rawShr_word_shift
        ldr    x5, [x5, x2, lsl 3]
        br     x5

Fnec_rawShr_word_shift_0:
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

Fnec_rawShr_word_shift_1:
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

Fnec_rawShr_word_shift_2:
        lsr   x10, x10,  x3
        lsl    x7, x11,  x4
        orr   x10, x10,  x7, lsl #1

        lsr   x11, x11,  x3

        stp   x10, x11, [x0]
        stp   xzr, xzr, [x0, 16]
        ret

Fnec_rawShr_word_shift_3:
        lsr   x11, x11,  x3

        stp   x11, xzr, [x0]
        stp   xzr, xzr, [x0, 16]
        ret

Fnec_rawShr_word_shift:
        .quad Fnec_rawShr_word_shift_0
        .quad Fnec_rawShr_word_shift_1
        .quad Fnec_rawShr_word_shift_2
        .quad Fnec_rawShr_word_shift_3


Fnec_rawShl:
_Fnec_rawShl:
        ldp    x9, x10, [x1]
        ldp   x11, x12, [x1, 16]

        and    x3,  x2, 0x3f
        mov    x4, 0x3f
        sub    x4,  x4,  x3

        lsr    x2,  x2,  #6
        adr    x5, Fnec_rawShl_word_shift
        ldr    x5, [x5, x2, lsl 3]
        br     x5

Fnec_rawShl_word_shift_0:
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

        b     Fnec_rawShl_sub

Fnec_rawShl_word_shift_1:
        lsl   x12, x11,  x3
        lsr    x8, x10,  x4
        orr   x12, x12,  x8, lsr #1

        lsl   x11, x10,  x3
        lsr    x7,  x9,  x4
        orr   x11, x11,  x7, lsr #1

        lsl   x10,  x9,  x3
        mov    x9, xzr

        b     Fnec_rawShl_sub

Fnec_rawShl_word_shift_2:
        lsl   x12, x10,  x3
        lsr    x8,  x9,  x4
        orr   x12, x12,  x8, lsr #1

        lsl   x11,  x9,  x3
        mov   x10, xzr
        mov    x9, xzr

        b     Fnec_rawShl_sub

Fnec_rawShl_word_shift_3:
        lsl   x12,  x9,  x3
        mov   x11, xzr
        mov   x10, xzr
        mov    x9, xzr

Fnec_rawShl_sub:
        adr    x6, Fnec_lboMask
        ldr    x6, [x6]
        and   x12, x12,  x6

        adr    x1, Fnec_rawq
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
Fnec_rawShl_word_shift:
        .quad Fnec_rawShl_word_shift_0
        .quad Fnec_rawShl_word_shift_1
        .quad Fnec_rawShl_word_shift_2
        .quad Fnec_rawShl_word_shift_3




    .align 8
Fnec_rawq:    .quad 0xbfd25e8cd0364141,0xbaaedce6af48a03b,0xfffffffffffffffe,0xffffffffffffffff
Fnec_np:      .quad 0x4b0dff665588b13f
Fnec_lboMask: .quad 0xffffffffffffffff
