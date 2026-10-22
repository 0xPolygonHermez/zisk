# zisk-verifier

Native verifiers for ZisK proofs, `no_std`: for hosts, the browser (WASM) and ZisK guests.

A ZisK proof has one byte format, the one `cargo-zisk prove` saves: a leaf (`vadcop_final`) or
a compressed (`--minimal`) proof of any hash family (Poseidon1, Poseidon2, blake3). The proof
names its family and stage; the crate picks the verifier.

The keys of this release's setup are compiled in, so a crate version verifies the proofs of
the ZisK release of the same version. A proof is never verified under the key it carries.

## Verify a proof

```rust
// The program you expect: its ROM root. From the SDK, `GuestProgram::vk()?.vk`; from a file
// of 4 little-endian u64 words, `zisk_verifier::decode_vk(&bytes)?`.
let program_vk: [u64; 4] = guest.vk()?.vk.try_into().unwrap();

let verified = zisk_verifier::verify_saved(&proof_bytes, Some(&program_vk))?;
let outputs: [u32; 64] = verified.outputs_u32().unwrap();
```

`verify_saved` checks the proof's shape, then that it is the expected program's, before the
STARK runs. Pass `None` to accept any program, and read `verified.program_vk`.

## A proof from another setup, or an aggregate

These verify under their own key: a custom setup's `vadcop_final.verkey.bin`
(`vadcop_final_compressed.verkey.bin` for a compressed proof), or a recurser's key for an
aggregate.

```rust
let key = zisk_verifier::decode_vk(&verkey_bytes)?;
let saved = zisk_verifier::decode_saved(&proof_bytes)?;
let verified = zisk_verifier::verify(&saved.proof, &key)?;
```

## In a ZisK guest

`verify_saved_words` verifies the words in place, without copying the proof:

```rust
let verified = zisk_verifier::verify_saved_words(&proof_words, &key, Some(&program_vk))?;
```

## The format

`[minimal][n_publics][publics][proof][vadcop_final_vk(4)][hash tag]`, little-endian u64 words.
A file from `cargo-zisk prove` may end with the execution's seed (`SEED_TRAILER_TAG`):
unauthenticated metadata, which `decode_saved` reads and `verify_saved` ignores. A guest's
`verify_saved_words` refuses it, so a guest sees one encoding per proof: strip it on the host
(`decode_saved`, or `Proof::to_bytes_without_seed_trailer`).
