# Proof upload and consumption

The buffer contains exactly 6,797 bytes. All instruction integer lengths and
offsets are little-endian. Proof scalar/point words remain big-endian, as in the
pinned Solidity verifier.

| Bytes | Meaning |
|---|---|
| 0..8 | ASCII `ZKPF0001`; fixed profile/version |
| 8..40 | Upload authority public key |
| 40..44 | Number of proof bytes written, u32 |
| 44 | Sealed flag, exactly 0 or 1 |
| 45..6797 | 6,752 proof bytes |

The account must be owned by the executing verifier program and have this exact
size. It stores no cached “verified” flag. Verification always executes against
the caller's guest key and output claim in the current transaction.

| Tag | Payload after tag | Accounts |
|---|---|---|
| 0 Initialize | u32 proof length, exactly 6752 | Writable buffer signer; authority signer |
| 1 Append | u32 offset; 1..900 proof bytes | Writable buffer; authority signer |
| 2 Seal | None | Writable buffer; authority signer |
| 3 Verify | 32-byte expected guest key; 512 output bytes | Read-only sealed buffer |
| 4 Close | None | Writable buffer; writable authority signer |

Initialization requires a zeroed account and the buffer's signature; create and
initialize it in one transaction. Append accepts only the current written
position. Holes, overwrites, overflow and extra instruction bytes are rejected.
Seal requires all proof bytes. Only the recorded authority can write, seal or
close; closing clears the data and returns its lamports. Anyone may verify a
sealed proof, but only against the guest key/output claim supplied by that
verification instruction. This standalone result is not an application action.

## Client

Use Node 22+ and locked dependencies (`npm ci`). The checked-in client uses
`@solana/web3.js` 3.2.0, including its asynchronous signing API.

```sh
node client/submit.mjs --rpc YOUR_RPC_URL --program YOUR_VERIFIER_ID \
  --wallet /path/to/keypair.json --proof /path/to/export.json
```

The client prints the buffer address before submitting account creation. It
checks each fully signed transaction's serialized size before sending. Eight
900-byte-or-smaller chunks upload the reference proof. If a confirmation is
interrupted, retain that address and resume with `--buffer ADDRESS`; the client
checks the owner, authority, version, written length and uploaded prefix against
the supplied proof. A mismatched prefix stops the upload.

After consuming the proof, reclaim the buffer's rent:

```sh
node client/submit.mjs --rpc YOUR_RPC_URL --program YOUR_VERIFIER_ID \
  --wallet /path/to/keypair.json --buffer ADDRESS --close
```

No client transaction or deployment was sent to a public cluster during local
validation. Runtime tests create, upload, verify and close using actual signed,
serialized transactions in LiteSVM. RPC confirmation/resumption still needs an
integration run against the intended cluster and deployed program.

## Consumer example

`consumer/src/config.rs` pins both the verifier program ID and expected guest
key. The checked-in values are test configuration, not a deployment address.
An application must pin its real deployment and guest key; never take the CPI
target from an untrusted caller without checking it against that configuration.
It must also decide whether to trust or remove the verifier's upgrade authority.

The example accepts exactly 512 output bytes and three accounts: a writable,
signing 32-byte receipt owned by the consumer, the proof buffer, and the pinned
executable verifier. It propagates CPI failure, then records SHA-256 of the exact
outputs. Repeating the proof writes the same digest. It demonstrates identity
binding and atomic failure handling, not application-level authorization of
payments, freshness, nonces or business rules. Applications must impose their
own output semantics and replay policy before acting on a proof.
