use crate::ops_core::*;
use crate::InstContext;
use zisk_definitions::TEMPORAL_REF_REQUEST_TAG;

/* Internal instructions */

/// InstContext-based wrapper over op_flag()
///
/// `c` carries the current `step`, which is what makes the `flag` operation usable as the guest's
/// source of temporal references for the `mt` DMA operations.  Every other user of `flag` (nop,
/// hint, jal) either discards `c` or stores the pc instead, so this is transparent to them.
///
/// A `flag` whose `b` is [`TEMPORAL_REF_REQUEST_TAG`] is a temporal reference *request*: it is
/// recorded in the context so that the `execute_advice` that follows knows which temporal
/// reference to tag its memory copy with.  The tag is out of reach of the 12-bit immediate that
/// feeds `b` on a hint `addi`, so an ordinary hint can never be mistaken for a request.
#[inline(always)]
pub fn opc_flag(ctx: &mut InstContext) {
    let (_, flag) = op_flag(ctx.a, ctx.b);
    ctx.c = ctx.step;
    ctx.flag = flag;
    if ctx.b == TEMPORAL_REF_REQUEST_TAG {
        ctx.temporal_ref = ctx.step;
    }
}

/// InstContext-based wrapper over op_copyb()
#[inline(always)]
pub fn opc_copyb(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_copyb(ctx.a, ctx.b);
}

/* SIGN EXTEND operations for different data widths (i8, i16 and i32) --> i64 --> u64 */

/// InstContext-based wrapper over op_signextend_b()
#[inline(always)]
pub fn opc_signextend_b(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_signextend_b(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_signextend_h()
#[inline(always)]
pub fn opc_signextend_h(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_signextend_h(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_signextend_w()
#[inline(always)]
pub fn opc_signextend_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_signextend_w(ctx.a, ctx.b);
}

/* ADD AND SUB operations for different data widths (i32 and u64) */

/// InstContext-based wrapper over op_add()
#[inline(always)]
pub fn opc_add(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_add(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_add_w()
#[inline(always)]
pub fn opc_add_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_add_w(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_sub()
#[inline(always)]
pub fn opc_sub(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_sub(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_sub_w()
#[inline(always)]
pub fn opc_sub_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_sub_w(ctx.a, ctx.b);
}

/* SHIFT operations */

/// InstContext-based wrapper over op_sll()
#[inline(always)]
pub fn opc_sll(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_sll(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_sll_w()
#[inline(always)]
pub fn opc_sll_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_sll_w(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_sra()
#[inline(always)]
pub fn opc_sra(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_sra(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_srl()
#[inline(always)]
pub fn opc_srl(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_srl(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_sra_w()
#[inline(always)]
pub fn opc_sra_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_sra_w(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_srl_w()
#[inline(always)]
pub fn opc_srl_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_srl_w(ctx.a, ctx.b);
}

/* COMPARISON operations */

/// InstContext-based wrapper over op_eq()
#[inline(always)]
pub fn opc_eq(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_eq(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_eq_w()
#[inline(always)]
pub fn opc_eq_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_eq_w(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_ltu()
#[inline(always)]
pub fn opc_ltu(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_ltu(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_lt()
#[inline(always)]
pub fn opc_lt(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_lt(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_ltu_w()
#[inline(always)]
pub fn opc_ltu_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_ltu_w(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_lt_w()
#[inline(always)]
pub fn opc_lt_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_lt_w(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_leu()
#[inline(always)]
pub fn opc_leu(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_leu(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_le()
#[inline(always)]
pub fn opc_le(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_le(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_leu_w()
#[inline(always)]
pub fn opc_leu_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_leu_w(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_le_w()
#[inline(always)]
pub fn opc_le_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_le_w(ctx.a, ctx.b);
}

/* LOGICAL operations */

/// InstContext-based wrapper over op_and()
#[inline(always)]
pub fn opc_and(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_and(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_or()
#[inline(always)]
pub fn opc_or(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_or(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_xor()
#[inline(always)]
pub fn opc_xor(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_xor(ctx.a, ctx.b);
}

/* ARITHMETIC operations: div / mul / rem */

/// InstContext-based wrapper over op_mulu()
#[inline(always)]
pub fn opc_mulu(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_mulu(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_mul()
#[inline(always)]
pub fn opc_mul(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_mul(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_mul_w()
#[inline(always)]
pub fn opc_mul_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_mul_w(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_muluh()
#[inline(always)]
pub fn opc_muluh(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_muluh(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_mulh()
#[inline(always)]
pub fn opc_mulh(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_mulh(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_mulsuh()
#[inline(always)]
pub fn opc_mulsuh(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_mulsuh(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_divu()
#[inline(always)]
pub fn opc_divu(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_divu(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_div()
#[inline(always)]
pub fn opc_div(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_div(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_divu_w()
#[inline(always)]
pub fn opc_divu_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_divu_w(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_div_w()
#[inline(always)]
pub fn opc_div_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_div_w(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_remu()
#[inline(always)]
pub fn opc_remu(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_remu(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_rem()
#[inline(always)]
pub fn opc_rem(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_rem(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_remu_w()
#[inline(always)]
pub fn opc_remu_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_remu_w(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_rem_w()
#[inline(always)]
pub fn opc_rem_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_rem_w(ctx.a, ctx.b);
}

/* MIN / MAX operations */

/// InstContext-based wrapper over op_minu()
#[inline(always)]
pub fn opc_minu(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_minu(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_min()
#[inline(always)]
pub fn opc_min(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_min(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_minu_w()
#[inline(always)]
pub fn opc_minu_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_minu_w(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_min_w()
#[inline(always)]
pub fn opc_min_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_min_w(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_maxu()
#[inline(always)]
pub fn opc_maxu(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_maxu(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_max()
#[inline(always)]
pub fn opc_max(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_max(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_maxu_w()
#[inline(always)]
pub fn opc_maxu_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_maxu_w(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_max_w()
#[inline(always)]
pub fn opc_max_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_max_w(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_rev8()
#[inline(always)]
pub fn opc_rev8(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_rev8(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_brev8()
#[inline(always)]
pub fn opc_brev8(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_brev8(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_andn()
#[inline(always)]
pub fn opc_andn(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_andn(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_orn()
#[inline(always)]
pub fn opc_orn(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_orn(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_xnor()
#[inline(always)]
pub fn opc_xnor(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_xnor(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_pack()
#[inline(always)]
pub fn opc_pack(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_pack(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_pack_h()
#[inline(always)]
pub fn opc_pack_h(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_pack_h(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_pack_w()
#[inline(always)]
pub fn opc_pack_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_pack_w(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_rol()
#[inline(always)]
pub fn opc_rol(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_rol(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_rol_w()
#[inline(always)]
pub fn opc_rol_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_rol_w(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_ror()
#[inline(always)]
pub fn opc_ror(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_ror(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_ror_w()
#[inline(always)]
pub fn opc_ror_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_ror_w(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_clz()
#[inline(always)]
pub fn opc_clz(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_clz(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_clz_w()
#[inline(always)]
pub fn opc_clz_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_clz_w(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_ctz()
#[inline(always)]
pub fn opc_ctz(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_ctz(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_ctz_w()
#[inline(always)]
pub fn opc_ctz_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_ctz_w(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_cpop()
#[inline(always)]
pub fn opc_cpop(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_cpop(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_cpop_w()
#[inline(always)]
pub fn opc_cpop_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_cpop_w(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_orc_b()
#[inline(always)]
pub fn opc_orc_b(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_orc_b(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_bclr()
#[inline(always)]
pub fn opc_bclr(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_bclr(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_bext()
#[inline(always)]
pub fn opc_bext(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_bext(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_binv()
#[inline(always)]
pub fn opc_binv(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_binv(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_bset()
#[inline(always)]
pub fn opc_bset(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_bset(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_add_u_w()
#[inline(always)]
pub fn opc_add_u_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_add_u_w(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_sh1add()
#[inline(always)]
pub fn opc_sh1add(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_sh1add(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_sh1add_u_w()
#[inline(always)]
pub fn opc_sh1add_u_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_sh1add_u_w(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_sh2add()
#[inline(always)]
pub fn opc_sh2add(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_sh2add(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_sh2add_u_w()
#[inline(always)]
pub fn opc_sh2add_u_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_sh2add_u_w(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_sh3add()
#[inline(always)]
pub fn opc_sh3add(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_sh3add(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_sh3add_u_w()
#[inline(always)]
pub fn opc_sh3add_u_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_sh3add_u_w(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_sll_u_w()
#[inline(always)]
pub fn opc_sll_u_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_sll_u_w(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_clmul()
#[inline(always)]
pub fn opc_clmul(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_clmul(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_clmul_h()
#[inline(always)]
pub fn opc_clmul_h(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_clmul_h(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_clmul_r()
#[inline(always)]
pub fn opc_clmul_r(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_clmul_r(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_xperm4()
#[inline(always)]
pub fn opc_xperm4(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_xperm4(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_xperm8()
#[inline(always)]
pub fn opc_xperm8(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_xperm8(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_czero_eqz()
#[inline(always)]
pub fn opc_czero_eqz(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_czero_eqz(ctx.a, ctx.b);
}

/// InstContext-based wrapper over op_czero_nez()
#[inline(always)]
pub fn opc_czero_nez(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_czero_nez(ctx.a, ctx.b);
}
