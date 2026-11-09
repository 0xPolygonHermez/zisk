"""Translate six scalar-field stages of one hash-pinned verifier, not general Yul."""
import hashlib
import re
import sys
from pathlib import Path

SOURCE_HASH = 'a531f5bed7c5e8c85f0ab96e54706d718c42c003cc3c1c900f0eb118d9a70155'
R = 21888242871839275222246405745257275088548364400416034343698204186575808495617
STAGES = dict(zip(['computeZh', 'computeZi', 'computeQ', 'computeZ', 'computeInversions', 'computeR'],
                  ['compute_zh', 'compute_zi', 'compute_q', 'compute_z', 'compute_inversions', 'compute_r']))


class Parser:
    def __init__(self, text):
        text = re.sub(r'//[^\n]*', '', text)
        token = r'0x[0-9a-fA-F]+|[0-9]+|[A-Za-z_][A-Za-z_0-9]*|:=|[(),{}]'
        if re.sub(token, '', text).strip():
            raise ValueError('unrecognized syntax')
        self.tokens = re.findall(token, text)
        self.at = 0

    def pop(self, expected=None):
        if self.at == len(self.tokens):
            raise ValueError('unexpected end')
        value = self.tokens[self.at]
        self.at += 1
        if expected is not None and value != expected:
            raise ValueError(f'expected {expected}, got {value}')
        return value

    def peek(self):
        return self.tokens[self.at] if self.at < len(self.tokens) else None

    def expr(self):
        name = self.pop()
        if self.peek() != '(':
            return name
        self.pop('(')
        args = []
        while self.peek() != ')':
            if args:
                self.pop(',')
            args.append(self.expr())
        self.pop(')')
        return (name, *args)


def address(e, constants):
    if isinstance(e, str):
        return constants[e] if e in constants else int(e, 0)
    if e[0] == 'add' and len(e) == 3:
        return address(e[1], constants) + address(e[2], constants)
    raise ValueError(f'not a constant address: {e}')


def literal(n):
    if not 0 <= n < R:
        raise ValueError('noncanonical scalar constant')
    return "Fr([" + ", ".join(str(((n * pow(2, 261, R) % R) >> (29*i)) & ((1<<29)-1)) for i in range(9)) + "])"


def field(e, constants, locals_):
    if isinstance(e, str):
        if e in locals_:
            return e
        try:
            return literal(address(e, constants))
        except (ValueError, KeyError):
            raise ValueError(f'unknown field identifier {e}') from None
    name, *args = e
    f = lambda x: field(x, constants, locals_)
    if name in ('addmod', 'mulmod') and len(args) == 3 and args[2] == ('mload', '0x60'):
        if name == 'mulmod' and args[0] == args[1]:
            return f'square({f(args[0])})'
        if name == 'addmod':
            for left, right in [(args[0], args[1]), (args[1], args[0])]:
                if isinstance(right, tuple) and len(right) == 3 and right[:2] == ('sub', ('mload', '0x60')):
                    return f'sub({f(left)}, {f(right[2])})'
        return f'add({f(args[0])}, {f(args[1])})' if name == "addmod" else f'mul({f(args[0])}, {f(args[1])})'
    if name == 'sub' and len(args) == 2 and args[0] == ('mload', '0x60'):
        return f'(-{f(args[1])})'
    if name == 'mload' and len(args) == 1:
        a = address(args[0], constants)
        if a % 32 or not 128 <= a < 3136:
            raise ValueError('invalid scalar memory address')
        return f'mem[{a // 32}]'
    if name == 'calldataload' and len(args) == 1:
        a = address(args[0], constants)
        if a == 6756:
            return 'public'
        if (a - 4) % 32 or not 580 <= a <= 6724:
            raise ValueError('invalid scalar calldata address')
        return f'evals[{(a - 4) // 32 - 18}]'
    if name == 'powMod' and len(args) == 2:
        exponent = address(args[1], constants)
        if not 0 <= exponent < 2**64:
            raise ValueError('invalid exponent')
        return f'pow({f(args[0])}, {exponent}u64)'
    raise ValueError(f'unsupported field expression: {e}')


def field_expression(text, constants):
    p = Parser(text)
    result = field(p.expr(), constants, set())
    if p.peek() is not None:
        raise ValueError('trailing expression')
    return result


def condition(e, constants, locals_):
    if isinstance(e, tuple) and e[0] == 'iszero' and len(e) == 2:
        x = e[1]
        if isinstance(x, tuple) and x[0] == 'eq' and len(x) == 3:
            return f'{field(x[1], constants, locals_)} != {field(x[2], constants, locals_)}'
        return f'{field(x, constants, locals_)} == Fr::ZERO'
    raise ValueError(f'unsupported condition {e}')


def statements(p, constants, locals_):
    lines = []
    while p.peek() not in (None, '}'):
        name = p.peek()
        if name == '{':
            p.pop('{'); body = statements(p, constants, set(locals_)); p.pop('}')
            lines.append('{\n' + '\n'.join(body) + '\n}')
        elif name == 'let':
            p.pop(); variable = p.pop()
            if variable in locals_:
                raise ValueError('shadowed variable')
            value = 'Fr::ZERO'
            if p.peek() == ':=':
                p.pop(); value = field(p.expr(), constants, locals_)
            locals_.add(variable)
            lines.append(f'let mut {variable} = {value};')
        elif name == 'if':
            p.pop(); test = condition(p.expr(), constants, locals_)
            p.pop('{'); body = statements(p, constants, set(locals_)); p.pop('}')
            lines.append(f'if {test} {{\n' + '\n'.join(body) + '\n}')
        elif name in locals_:
            p.pop(); p.pop(':=')
            lines.append(f'{name} = {field(p.expr(), constants, locals_)};')
        else:
            e = p.expr()
            if not isinstance(e, tuple):
                raise ValueError(f'unknown statement {e}')
            name, *args = e
            if name == 'mstore' and len(args) == 2:
                a = address(args[0], constants)
                if a % 32 or not 128 <= a < 3136:
                    raise ValueError('invalid store')
                lines.append(f'mem[{a // 32}] = {field(args[1], constants, locals_)};')
            elif name == 'fail' and not args:
                lines.append('return Err(VerifyError::InvalidProof);')
            elif name == 'checkInv' and len(args) == 2:
                lines.append(f'if mul({field(args[0], constants, locals_)}, {field(args[1], constants, locals_)}) != Fr::ONE {{ return Err(VerifyError::InvalidProof); }}')
            elif name == 'batchInverse' and len(args) == 3:
                start, length = address(args[0], constants) // 32, address(args[1], constants)
                if (start, length) != (50, 10):
                    raise ValueError('unexpected inverse batch')
                value = field(args[2], constants, locals_)
                lines.append(f'let inverse = {value};\nbatch_inverse(&mut mem[50..60], inverse)?;')
            else:
                raise ValueError(f'unsupported statement {e}')
    return lines


def generate(source):
    if hashlib.sha256(source.encode()).hexdigest() != SOURCE_HASH:
        raise ValueError('reference source hash mismatch')
    constants = {k: int(v) for k, v in re.findall(r'uint\d+ constant (\w+)\s*=\s*(\d+)\s*;', source)}
    constants['pPublic'] = 6756
    out = ['// Generated by tools/generate_profile.py from SHA-256 ' + SOURCE_HASH + '.',
           '// Do not edit; differential tests compare these stages with the release JS oracle.',
           '#![allow(unused_assignments, unused_mut, unused_variables, non_snake_case, unused_parens)]',
           'use crate::scalar::{Fr, pow};\nuse crate::{add, batch_inverse, mul, scalar::{sub, square}, VerifyError};']
    for original, name in STAGES.items():
        m = re.search(r'function ' + original + r'\([^)]*\)\s*\{', source)
        if not m:
            raise ValueError(f'missing stage {original}')
        at, depth = m.end(), 1
        end = at
        while depth:
            depth += (source[end] == '{') - (source[end] == '}'); end += 1
        p = Parser(source[at:end-1])
        body = statements(p, constants, set())
        if p.peek() is not None:
            raise ValueError('unconsumed stage')
        if name == 'compute_z':
            replacements = {
                'mem[16] = pow(y, 2u64);': 'mem[16] = square(y);\nlet y4 = square(mem[16]);\nlet y8 = square(y4);',
                'mem[17] = pow(y, 16u64);': 'mem[17] = square(y8);',
                'mem[18] = pow(y, 18u64);': 'mem[18] = mul(mem[17], mem[16]);',
                'mem[19] = pow(y, 24u64);': 'mem[19] = mul(mem[17], y8);',
                'mem[20] = pow(y, 26u64);': 'mem[20] = mul(mem[19], mem[16]);',
                'mem[21] = pow(y, 29u64);': 'mem[21] = mul(mul(mem[19], y4), y);',
            }
            if not all(body.count(line) == 1 for line in replacements):
                raise ValueError('unexpected y-power schedule')
            body = [replacements.get(line, line) for line in body]
        if name == 'compute_inversions':
            optimized = []
            for line in body:
                if 'pow(mem[10], 216u64)' in line:
                    optimized.append('let xi215 = pow(mem[10], 215u64);')
                optimized.append(line.replace('pow(mem[10], 216u64)', 'mul(xi215, mem[10])').replace('pow(mem[10], 215u64)', 'xi215'))
            body = optimized
        if name == 'compute_q':
            # Keep each generated block below SBF's 4 KiB stack-frame limit.
            if not all(line.startswith('mem[') and '\n' not in line for line in body):
                raise ValueError('Q blocks must communicate only through scalar memory')
            calls = []
            for index, start in enumerate(range(0, len(body), 8)):
                part = f'{name}_{index}'
                out += ['#[inline(never)]', f'fn {part}(mem: &mut [Fr], evals: &[Fr], public: Fr) {{',
                        *body[start:start+8], '}']
                calls.append(f'{part}(mem, evals, public);')
            body = calls
        out += ['#[inline(never)]', f'pub(crate) fn {name}(mem: &mut [Fr], evals: &[Fr], public: Fr) -> Result<(), VerifyError> {{',
                *body, 'Ok(())\n}']
    def packed(names):
        return ', '.join(str(b) for n in names for b in constants[n].to_bytes(32, 'big'))
    out += ['pub(crate) const FIXED: [[u8; 64]; 4] = [']
    out += ['[' + packed([f'f{i}x', f'f{i}y']) + '],' for i in range(4)]
    out += ['];']
    for label, prefix in [('G2', 'G2'), ('TAU', 'X2')]:
        out += [f'pub(crate) const {label}: [u8; 128] = [' + packed([prefix+'x2',prefix+'x1',prefix+'y2',prefix+'y1']) + '];']
    out += [f'pub(crate) const DIGEST: Fr = {literal(constants["DIGEST"])};']
    return '\n'.join(out) + '\n'


if __name__ == '__main__':
    root = Path(__file__).resolve().parents[2]
    output = generate((root/'zisk-contracts/blake3/PilfflonkVerifier.sol').read_text())
    target = root/'solana/verifier/src/profile.rs'
    if '--check' in sys.argv:
        if target.read_text() != output:
            raise SystemExit('generated profile differs; regenerate')
    else:
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(output)
