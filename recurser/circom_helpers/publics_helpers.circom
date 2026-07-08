pragma circom 2.1.0;

// Read u32-sized values from the 64-slot publics array. numBytes capped at 4
// (returned signal always < 2^32 < p, no Goldilocks aliasing). Touched slots
// are range-checked to u32 via Num2Bits(32). For u64+, make multiple calls.
//
// Examples:
//   signal x   <== GetPublicLE(4, 0)(publics);   // u32 at slot 0
//   signal y   <== GetPublicLE(2, 2)(publics);   // u16 spanning slots 0+1
//   signal abi <== GetPublicBE(4, 28)(publics);  // Solidity ABI uint32

include "bitify.circom";


template GetPublicLE(numBytes, initialByte) {
    assert(numBytes >= 1);
    assert(numBytes <= 4);
    assert(initialByte + numBytes <= 256);

    signal input publics[64];
    signal output value;

    var firstSlot = initialByte \ 4;
    var lastSlot = (initialByte + numBytes - 1) \ 4;
    var nSlots = lastSlot - firstSlot + 1;

    component decomp[nSlots];
    for (var s = 0; s < nSlots; s++) {
        decomp[s] = Num2Bits(32);
        decomp[s].in <== publics[firstSlot + s];
    }

    var lc = 0;
    var coeff = 1;
    for (var i = 0; i < numBytes; i++) {
        var abs = initialByte + i;
        var slotIx = (abs \ 4) - firstSlot;
        var byteIx = abs % 4;
        for (var k = 0; k < 8; k++) {
            lc += decomp[slotIx].out[8 * byteIx + k] * coeff;
            coeff *= 2;
        }
    }
    value <== lc;
}


template GetPublicBE(numBytes, initialByte) {
    assert(numBytes >= 1);
    assert(numBytes <= 4);
    assert(initialByte + numBytes <= 256);

    signal input publics[64];
    signal output value;

    var firstSlot = initialByte \ 4;
    var lastSlot = (initialByte + numBytes - 1) \ 4;
    var nSlots = lastSlot - firstSlot + 1;

    component decomp[nSlots];
    for (var s = 0; s < nSlots; s++) {
        decomp[s] = Num2Bits(32);
        decomp[s].in <== publics[firstSlot + s];
    }

    // Reverse so the source's last byte becomes the LSB (BE).
    var lc = 0;
    var coeff = 1;
    for (var i = numBytes - 1; i >= 0; i--) {
        var abs = initialByte + i;
        var slotIx = (abs \ 4) - firstSlot;
        var byteIx = abs % 4;
        for (var k = 0; k < 8; k++) {
            lc += decomp[slotIx].out[8 * byteIx + k] * coeff;
            coeff *= 2;
        }
    }
    value <== lc;
}
