# Proof upload and consumption

This draft uses buffer format `ZKPF0002`, exactly 6,829 bytes. It rejects the
previous `ZKPF0001` format; create a fresh buffer with the matching client.
Instruction lengths and offsets are little-endian. Proof scalar/point words
remain big-endian, matching the pinned Solidity verifier.

| Bytes | Meaning |
|---|---|
| 0..8 | ASCII `ZKPF0002`; fixed profile and account format |
| 8..40 | Upload authority public key |
| 40..44 | Number of proof bytes written, u32 |
| 44 | State: 0 uploading, 1 sealed, 2 verified |
| 45..77 | Verified statement digest; zero before state 2 |
| 77..6829 | 6,752 proof bytes |

The account must be owned by the executing verifier and have this exact size.
States 1 and 2 require a complete proof and disallow further writes.
The statement digest is SHA-256 of the ASCII domain `ZisK:blake3-leaf:v1`,
the expected 32-byte guest key and all 512 output bytes, concatenated in that
order. The pinned proof profile binds the verification key and recursion root.

| Tag | Payload after tag | Accounts |
|---|---|---|
| 0 Initialize | u32 proof length, exactly 6752 | Writable buffer signer; authority signer |
| 1 Append | u32 offset; 1..900 proof bytes | Writable buffer; authority signer |
| 2 Seal | None | Writable buffer; authority signer |
| 3 Verify without saving | 32-byte expected guest key; 512 output bytes | Read-only sealed buffer |
| 4 Close | None | Writable buffer; writable authority signer |
| 5 Verify and record | 32-byte expected guest key; 512 output bytes | Writable sealed buffer |

Create and initialize in one transaction. Initialization requires a zeroed
account and the buffer's signature. Append accepts only the current written
position: no holes, overwrites, overflow or extra bytes. Seal requires the
complete proof. Only the recorded authority may append, seal or close.

Anyone may verify a sealed proof against an explicit guest/output claim.
Tag 5 executes the full verifier and writes state 2 and the digest only after
success. Failure preserves the preceding state. Repeated verification still
runs the verifier; it does not trust a cached flag. Tag 3 never creates a record.

Closing clears the buffer and returns rent to the authority. A saved result
exists only while its buffer exists, so consume it before closing. Recreating
the account does not restore its previous proof or verified result.

## Client

Use Node 22+ and locked dependencies (`npm ci`). The client uses
`@solana/web3.js` 3.2.0, including asynchronous signing.

```sh
node client/submit.mjs --rpc YOUR_RPC_URL --program YOUR_VERIFIER_ID \
  --wallet /path/to/keypair.json --proof /path/to/export.json \
  --guest EXPECTED_32_BYTE_GUEST_KEY_AS_HEX
```

Choose `--guest` from trusted application configuration, independently of the
proof file. The client rejects a missing/malformed key or a mismatched
`programVK` before contacting RPC or reading the wallet. The on-chain verifier
also checks that the proof matches the supplied key and outputs.

The client prints the buffer address before creation and checks every fully
signed transaction against the 1,232-byte packet limit. Eight chunks upload
the reference proof. Resume an interrupted upload with `--buffer ADDRESS`;
owner, authority, format, written length and the uploaded prefix must match.
After sealing, the client uses tag 5 to save the verified statement.

After consumption, return the buffer's rent:

```sh
node client/submit.mjs --rpc YOUR_RPC_URL --program YOUR_VERIFIER_ID \
  --wallet /path/to/keypair.json --buffer ADDRESS --close
```

The same client is exercised against an isolated real local validator by
`node tools/test_validator.mjs`. No public-cluster transaction is submitted.

## Consumer example

`consumer/src/config.rs` pins the verifier ID and expected guest. Replace those
test values with the application's reviewed deployment and guest before use.

The consumer requires the upgradeable loader's Program account and its linked,
loader-owned ProgramData account, with upgrade authority exactly `None`.
It rejects substituted metadata and a remaining upgrade authority. Pinning an
ID and removing its authority does not establish past integrity: use a trusted
deployment of the reviewed binary, and do not reuse records produced by
untrusted earlier code. A production application must also secure its own code.

Instruction data is a one-byte mode followed by exactly 512 output bytes:

- Mode 0 verifies through CPI and consumes the statement in the same transaction.
- Mode 1 checks an existing verified record and consumes it without rerunning
  the cryptography. This is the path for applications needing substantial compute.

Both modes take exactly these accounts, in order:

1. Writable payer signer.
2. Writable consumption receipt PDA.
3. Read-only proof buffer owned by the pinned verifier.
4. Read-only pinned executable verifier.
5. Read-only linked ProgramData account.
6. System program.

The receipt address is derived under the consumer program using seeds
`[b"consumed", statement_digest]`. After successful verification/record checking,
the consumer funds, allocates and assigns this 32-byte PDA and stores the digest.
An existing receipt rejects the action before verification. Prefunding the PDA
cannot prevent legitimate creation. Any failure rolls back receipt creation.

The receipt has no close instruction. It remains after the proof buffer closes,
so uploading the same proof to a different buffer does not bypass replay
protection. Its rent reserve is 1,113,600 lamports in the measured runtimes.
This policy allows each guest/output statement once per consumer deployment.
An application needing repeated actions with otherwise identical outputs must
prove and check an application nonce in those outputs; adding an unproved
instruction nonce would allow replay.

This example is a permissionless statement-consumption registry, not payment
authorization. Applications must interpret the committed outputs and bind
beneficiary, action, nonce, chain/domain and expiry as their semantics require.
The caller's signature only authorizes paying for the receipt.

The local-validator test exercises both modes with the actual compiled consumer.
It preloads the example IDs at genesis, so the hardcoded test ID does not prevent
RPC/CPI testing. The immutable deployment requirement is enforced in the test.
