package main

import "github.com/0xPolygonHermez/zisk/ziskasm/lang/tinygo"

// u256Case is one U256 operation: its big-endian hex operands and results.
type u256Case struct {
	op       string
	in, want []string
}

func parseU256(s string) (w zkvm.U256) {
	nibble := func(c byte) byte {
		if c >= 'a' {
			return c - 'a' + 10
		}
		return c - '0'
	}
	for i := range w {
		w[i] = nibble(s[2*i])<<4 | nibble(s[2*i+1])
	}
	return
}

func toLE(w zkvm.U256) (l zkvm.U256LE) {
	for i := range l {
		for j := 0; j < 8; j++ {
			l[i] |= uint64(w[31-8*i-j]) << (8 * j)
		}
	}
	return
}

// u256BE runs op on big-endian words, u256LE on little-endian ones.
func u256BE(op string, in []zkvm.U256) []zkvm.U256 {
	a, b, n := &in[0], &in[len(in)-1], &in[len(in)-1]
	if len(in) == 3 {
		b = &in[1]
	}
	one := func(r zkvm.U256) []zkvm.U256 { return []zkvm.U256{r} }
	two := func(q, r zkvm.U256) []zkvm.U256 { return []zkvm.U256{q, r} }
	switch op {
	case "div":
		return one(zkvm.U256Div(a, b))
	case "mod":
		return one(zkvm.U256Mod(a, b))
	case "divmod":
		return two(zkvm.U256DivMod(a, b))
	case "sdiv":
		return one(zkvm.U256SDiv(a, b))
	case "smod":
		return one(zkvm.U256SMod(a, b))
	case "sdivmod":
		return two(zkvm.U256SDivMod(a, b))
	case "add":
		return one(zkvm.U256Add(a, b))
	case "sub":
		return one(zkvm.U256Sub(a, b))
	case "mul":
		return one(zkvm.U256Mul(a, b))
	case "addmod":
		return one(zkvm.U256AddMod(a, b, n))
	case "mulmod":
		return one(zkvm.U256MulMod(a, b, n))
	case "exp":
		return one(zkvm.U256Exp(a, b))
	case "lt":
		return one(zkvm.U256Lt(a, b))
	case "gt":
		return one(zkvm.U256Gt(a, b))
	case "slt":
		return one(zkvm.U256SLt(a, b))
	case "sgt":
		return one(zkvm.U256SGt(a, b))
	case "eq":
		return one(zkvm.U256Eq(a, b))
	case "iszero":
		return one(zkvm.U256IsZero(a))
	case "and":
		return one(zkvm.U256And(a, b))
	case "or":
		return one(zkvm.U256Or(a, b))
	case "xor":
		return one(zkvm.U256Xor(a, b))
	case "not":
		return one(zkvm.U256Not(a))
	case "byte":
		return one(zkvm.U256Byte(a, b))
	case "shl":
		return one(zkvm.U256Shl(a, b))
	case "shr":
		return one(zkvm.U256Shr(a, b))
	case "sar":
		return one(zkvm.U256Sar(a, b))
	case "signextend":
		return one(zkvm.U256SignExtend(a, b))
	}
	panic("unknown op " + op)
}

func u256LE(op string, in []zkvm.U256LE) []zkvm.U256LE {
	a, b, n := &in[0], &in[len(in)-1], &in[len(in)-1]
	if len(in) == 3 {
		b = &in[1]
	}
	one := func(r zkvm.U256LE) []zkvm.U256LE { return []zkvm.U256LE{r} }
	two := func(q, r zkvm.U256LE) []zkvm.U256LE { return []zkvm.U256LE{q, r} }
	switch op {
	case "div":
		return one(zkvm.U256LEDiv(a, b))
	case "mod":
		return one(zkvm.U256LEMod(a, b))
	case "divmod":
		return two(zkvm.U256LEDivMod(a, b))
	case "sdiv":
		return one(zkvm.U256LESDiv(a, b))
	case "smod":
		return one(zkvm.U256LESMod(a, b))
	case "sdivmod":
		return two(zkvm.U256LESDivMod(a, b))
	case "add":
		return one(zkvm.U256LEAdd(a, b))
	case "sub":
		return one(zkvm.U256LESub(a, b))
	case "mul":
		return one(zkvm.U256LEMul(a, b))
	case "addmod":
		return one(zkvm.U256LEAddMod(a, b, n))
	case "mulmod":
		return one(zkvm.U256LEMulMod(a, b, n))
	case "exp":
		return one(zkvm.U256LEExp(a, b))
	case "lt":
		return one(zkvm.U256LELt(a, b))
	case "gt":
		return one(zkvm.U256LEGt(a, b))
	case "slt":
		return one(zkvm.U256LESLt(a, b))
	case "sgt":
		return one(zkvm.U256LESGt(a, b))
	case "eq":
		return one(zkvm.U256LEEq(a, b))
	case "iszero":
		return one(zkvm.U256LEIsZero(a))
	case "and":
		return one(zkvm.U256LEAnd(a, b))
	case "or":
		return one(zkvm.U256LEOr(a, b))
	case "xor":
		return one(zkvm.U256LEXor(a, b))
	case "not":
		return one(zkvm.U256LENot(a))
	case "byte":
		return one(zkvm.U256LEByte(a, b))
	case "shl":
		return one(zkvm.U256LEShl(a, b))
	case "shr":
		return one(zkvm.U256LEShr(a, b))
	case "sar":
		return one(zkvm.U256LESar(a, b))
	case "signextend":
		return one(zkvm.U256LESignExtend(a, b))
	}
	panic("unknown op " + op)
}

// checkU256 runs every case in both byte orders: 2 checks per case.
func checkU256() {
	for _, c := range u256Cases {
		be := make([]zkvm.U256, len(c.in))
		le := make([]zkvm.U256LE, len(c.in))
		for i, s := range c.in {
			be[i] = parseU256(s)
			le[i] = toLE(be[i])
		}
		gotBE, gotLE := u256BE(c.op, be), u256LE(c.op, le)
		okBE, okLE := len(gotBE) == len(c.want), len(gotLE) == len(c.want)
		for i, s := range c.want {
			want := parseU256(s)
			okBE = okBE && gotBE[i] == want
			okLE = okLE && gotLE[i] == toLE(want)
		}
		check("u256_"+c.op+" "+c.in[0][56:], okBE)
		check("u256_le_"+c.op+" "+c.in[0][56:], okLE)
	}
}
