# ffiasm generated fields

The field files in this directory (`<field>.asm`, `<field>.hpp`, `<field>.cpp`, `<field>_element.hpp`,
`<field>_generic.cpp`, `<field>_raw_generic.cpp` and `<field>_raw_arm64.s`) are generated with
[iden3/ffiasm](https://github.com/iden3/ffiasm), commit `783dac7`, the merge of
[PR #8](https://github.com/iden3/ffiasm/pull/8), which added the safegcd modular inverse to the
generator: regenerating at that commit reproduces these files exactly, inverse included. Do not
edit them by hand: change the generator instead and regenerate them.

To regenerate them, run from this directory, with `<ffiasm>` a checkout of that repository after
`npm install`:

```sh
node <ffiasm>/src/buildzqfield.js -q 21888242871839275222246405745257275088548364400416034343698204186575808495617 -n Fr
node <ffiasm>/src/buildzqfield.js -q 21888242871839275222246405745257275088696311157297823662689037894645226208583 -n Fq
node <ffiasm>/src/buildzqfield.js -q 115792089237316195423570985008687907853269984665640564039457584007908834671663 -n Fec
node <ffiasm>/src/buildzqfield.js -q 115792089237316195423570985008687907852837564279074904382605163141518161494337 -n Fnec
node <ffiasm>/src/buildzqfield.js -q 52435875175126190479447740508185965837690552500527637822603658699938581184513 -n BLS12_381
node <ffiasm>/src/buildzqfield.js -q 4002409555221667393417789825735904156556882819939007885332058136124031650490837864442687629129015664037894272559787 -n BLS12_381_384
node <ffiasm>/src/buildzqfield.js -q 115792089210356248762697446949407573530086143415290314195533631308867097853951 -n pSecp256r1
node <ffiasm>/src/buildzqfield.js -q 115792089210356248762697446949407573529996955224135760342422259061068512044369 -n nSecp256r1
```

These are the BN254 scalar (`Fr`) and base (`Fq`) fields, the secp256k1 base (`Fec`) and scalar
(`Fnec`) fields, the BLS12-381 scalar (`BLS12_381`) and base (`BLS12_381_384`) fields, and the
secp256r1 base (`pSecp256r1`) and scalar (`nSecp256r1`) fields.

The other files in this directory (`alt_bn128`, `curve`, `f2field`, `fft`, `misc`, `multiexp`,
`naf`, `splitparstr`, ...) are not generated. They are support code from ffiasm's `c/` directory,
and several of them differ from the version at the commit above, so do not overwrite them when
regenerating.

After regenerating, check that the precompile results are unchanged with `lib-c/c/test/difftest.sh`.
