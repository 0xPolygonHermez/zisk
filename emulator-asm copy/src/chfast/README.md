# Ethash keccak-f

C implementation of the Keccak-f permutation as part of the C/C++ implementation of Ethash
– the Ethereum Proof of Work algorithm - maintained by Paweł Bylica [@chfast] and licensed under
the [Apache License, Version 2.0].

We have modified the header files location to make the code fit in a single folder and set as 
extern the keccakf1600_generic() method.

[Apache License, Version 2.0]: LICENSE
[Ethash reference implementation]: https://github.com/ethereum/wiki/wiki/Ethash
