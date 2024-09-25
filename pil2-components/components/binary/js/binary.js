const {assert} = require('chai');
const Component = require(__dirname + '/../../src/component.js');

const ADD = 0n;
const SUB = 1n;
const LT  = 2n;
const SLT = 3n;
const EQ  = 4n;
const AND = 5n;
const OR  = 6n;
const XOR = 7n;
const LT4 = 8n;

const OPERATION_TAGS = ['ADD', 'SUB', 'LT', 'SLT', 'EQ', 'AND', 'OR', 'XOR', 'LT4'];
module.exports = class Binary extends Component {
    constructor (config = {}) {
        super(config);
        // config params 
        this.bits = config.bits ?? 256;
        this.bpc = config.bpc ?? 8;
        this.inputChunks = config.inputChunks ?? 8;
        this.enableLt4 = (config.enableLt4 ?? true) ? true : false;

        this.bytes = this.bits / 8;
        assert(this.bytes % 2 == 0);
        this.inputBytes = this.bits / (this.inputChunks * 8);
        assert(this.bits == this.inputChunks * this.inputBytes * 8);
        this.clocks = this.bits / (8 * this.bpc);
        this.maxValue = (2n ** BigInt(this.bits)) - 1n;
        this.signBitMask = 2n ** BigInt(this.bits - 1);
        this.mask4 = ((2n ** BigInt(this.bits/4)) - 1n) << BigInt(3*(this.bits/4));
    }

    static get ADD() { return ADD; };
    static get SUB() { return SUB; };
    static get LT() { return LT; };
    static get SLT() { return SLT; };
    static get EQ() { return EQ; };
    static get AND() { return AND; };
    static get OR() { return OR; };
    static get XOR() { return XOR; };
    static get LT4() { return LT4; };

    static operationTag(operation) {
        return OPERATION_TAGS[operation] ?? '';
    }

    // 0:id, 1:opcode, 2:a[n], 2+n: b[n], 2+2*n: c[n], 3+2*n: cin, 4+2*n = cout
    calculateVerify(verify, values) {
        const ic = this.inputChunks;
        const [id, operation, a, b, c, cin, cout] = this.valuesToScalars(values,[1,1,ic,ic,ic,1,1], this.inputBytes * 8);
        // console.log(`${id}  A:0x${a.toString(16)} ${OPERATION_TAGS[operation]} B:0x${b.toString(16)} (CIN:${cin}) = C:0x${c.toString(16)} COUT:${cout}`);
        const [_c, _cout] = this.#calculate(operation, a, b, cin, this.maxValue, this.signBitMask);
        if (!verify) {
            return _c;
        }
        this.proves(id, operation, a, b, cin);
        if (c !== _c || cout !== _cout) {
            return this.onVerifyFails(`Binary result doesn't match A:0x${a.toString(16)} B:0x${b.toString(16)} C:0x${c.toString(16)} CIN:${cin} COUT:${cout} vs C:0x${_c.toString(16)} and COUT:${_cout}] for operation ${OPERATION_TAGS[operation]}`);
        }
        return true;
    }
    #calculate(operation, a, b, cin, maxValue, signBitMask, last = true) {
        let _c;
        let _cout;
        switch (operation) {    
            case ADD:
                _c = a + b + cin;
                if (_c <= maxValue) {
                    return [_c, 0n];
                }
                return [_c - (maxValue + 1n), 1n];
            case SUB:
                _c = a - b - cin;        
                if (_c >= 0n) {
                    return [_c, 0n];
                }
                return [_c + (maxValue + 1n), 1n];
            case SLT: 
                if (last) {               
                    const signs = (a & signBitMask ? 2:0) + (b & signBitMask ? 1:0);                
                    switch  (signs) {
                        case 0b00: // +a +b
                        case 0b11: // -a -b
                            _c = ((cin && a === b) || a < b) ? 1n:0n;
                            return [_c, _c];

                        case 0b01: // +a -b
                            return [0n, 0n];

                        case 0b10: // -a +b
                            return [1n, 1n];
                    }
                }
            case LT:                                
                _cout = ((cin && a === b) || a < b) ? 1n:0n;
                return [last ? _cout : 0n, _cout];

            case EQ:                
                _cout = (a == b && !cin) ? 0n : 1n;
                return [last ? 1n - _cout : 0n, last ? (1n - _cout) : _cout];

            case AND: 
                _c = a & b;
                return [_c, (_c || cin) ? 1n : 0n];

            case OR:
                return [a | b, 0n];

            case XOR:
                return [a ^ b, 0n];
 
            case LT4: {
                let mask4 = this.mask4;
                const mask4bits =  BigInt(this.bits) / 4n;
                _c = 1n;
                while (_c && mask4 !== 0n) {
                    _c = (a & mask4) < (b & mask4) ? 1n: 0n;
                    mask4 = mask4 >> mask4bits;
                }
                return [_c, _c];
            }
        }
        throw new Error(`Invalid binary operation ${operation}`);
    }
    proves(id, operation, a, b, cin) {
        this.inputs.push([id, operation, a, b, cin]);
    }
    execute(pols, base, id, operation, a, b, cin) {
        let shiftBits = 0n;
        let byteC = 0n;
        let ibyte = 0;
        let carry = cin;
        const bytesLt4 = this.bytes / 4;
        // console.log(`carry(cin):${cin}`);

        let previousLt4Count = 0n;
        for (let clock = 0; clock < this.clocks; ++clock) {
            const lastClock = (clock === this.clocks - 1);
            pols.carry[0][base + clock] = carry;
            for (let index = 0; index < this.bpc; ++index) {
                const reset4 = (operation === LT4 && (ibyte % bytesLt4) === 0) ? 1n : 0n;
                const lastByte = lastClock && index === (this.bpc - 1);
                const byteA = (a >> shiftBits) & 0xFFn;
                const byteB = (b >> shiftBits) & 0xFFn;
                shiftBits += 8n;

                pols.freeInA[index][base + clock] = byteA;
                pols.freeInB[index][base + clock] = byteB;
 
                [byteC, carry] = this.#calculate(operation === LT4 ? LT : operation, byteA, byteB, reset4 ? 0n : carry, 
                                                 255n, lastByte && operation === SLT ? 0x80n:0n, lastByte);
                if (operation === LT4) {
                    // console.log(['A lastByte,ibyte,carry,previousLt4Count', lastByte, ibyte, carry, previousLt4Count,(ibyte % bytesLt4),(bytesLt4 - 1)]);
                    if (!lastByte && (ibyte % bytesLt4) === (bytesLt4 - 1)) {
                        previousLt4Count = previousLt4Count + carry;
                    }
                    if (lastByte && previousLt4Count !== 3n) {
                        byteC = 0n;
                        carry = 0n;
                    }
                }
                pols.freeInC[index][base + clock] = byteC;
                pols.carry[index+1][base + clock] = carry;
                ++ibyte;
            }
        }
    }   
}