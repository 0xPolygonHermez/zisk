# Patch Testing

One guest program per published ZisK patch crate, built against the
`ziskos` of this tree and executed in `ziskemu`. It answers, for every ZisK
change and release: _do the patches we tell users to pin still link, and do they
still compute the right thing?_

The crypto patches do not depend on the `ziskos` crate. They declare
`extern "C"` symbols (`sha256f_compress_c`, `pairing_batch_bn254_c`, …) that
ziskos exports at link time, so ZisK can break a patch without any compile
error on either side. The failure only appears when a guest links, which is what
these programs do.

## Layout

```
patch-testing/
├── run.sh          # build + emulate every guest, print a summary
├── common/         # tiny check harness (Suite, hex, deterministic Rng)
├── sha2/           # zisk-patch-hashes          sha2
├── sha3/           # zisk-patch-hashes          sha3
├── tiny-keccak/    # zisk-patch-tiny-keccak     tiny-keccak
├── k256/           # zisk-patch-elliptic-curves k256
├── substrate-bn/   # zisk-patch-bn              substrate-bn
├── blst/           # zisk-patch-blst            blst
├── modexp/         # zisk-patch-modexp          aurora-engine-modexp
└── ruint/          # zisk-patch-ruint           ruint
```

The exact patch revision (tag or branch) each guest tests lives in its
`Cargo.toml`, under `[patch.crates-io]`. Each guest is its own Cargo workspace,
so several revisions of the same patch can be tested side by side when needed.
`run.sh` links every guest's `target/` to `patch-testing/target` so ziskos is
only compiled once.

Every guest is self-checking: it runs all its checks, prints each failure, and
panics at the end if any failed. The checks fall into four groups:

- **Known-answer vectors** from the specs: NIST SHA-2/SHA-3, RFC 9380
  hash-to-curve, BIP-340, EIP-196/198 and the go-ethereum ecrecover vector.
- **Algebraic identities** that a wrong implementation will not satisfy by
  accident: pairing bilinearity, `[r]G = O`, `(a+b)G = aG + bG`, Fermat.
- **Differential checks** against a reference that is not accelerated: U512
  arithmetic for ruint, `num-bigint::modpow` for modexp, one-shot vs incremental
  hashing across block boundaries.
- **Negative cases**: tampered messages, off-curve and out-of-subgroup points,
  non-canonical field elements.

## Running

```sh
patch-testing/run.sh                    # all guests on the ZisK target (cargo-zisk + ziskemu)
patch-testing/run.sh k256 ruint         # a subset
patch-testing/run.sh --host             # natively: validates the test vectors themselves
```

The default mode builds `cargo-zisk` and `ziskemu` from this tree. Set
`CARGO_ZISK` and `ZISKEMU` to reuse existing binaries. `REPORT_MD=<file>` writes
a Markdown summary, and `XFAIL="<guest> ..."` marks known-broken guests. Their
failure is then reported as `XFAIL` and does not fail the run, and an XFAIL
guest that passes is reported as `XPASS`. Logs are in `patch-testing/target/logs/`.

In `--host` mode the patched crates fall back to their upstream code, so a host
pass means the expected values in the guest are right. A ZisK-target failure on
top of a host pass then points at the accelerated path. Host mode cannot
resolve guests that ZisK cannot resolve either (currently `blst`).

## Adding a guest

1. Copy an existing guest directory to `<crate>/` and point its
   `[dependencies]` and `[patch.crates-io]` at the patch revision. Use the tag
   or branch that the docs tell users to pin.
2. Exercise every function the patch routes to ZisK. `grep -rn 'target_vendor = "zisk"'`
   in the patch repo lists them. Prefer published vectors and identities over
   values copied from a previous run.
3. `run.sh --host <dir>` to validate the vectors, then `run.sh <dir>`.

## Keeping the patches current

On every ZisK release:

1. If every guest passes, the pinned patch revisions are compatible with the
   release. Each patch can get a `patch-<crate>-<ver>-zisk-<release>` tag on the
   same commit, and the zisk-docs patch table can list the new version.
2. If a guest fails, the run summary names the cause (an undefined symbol, or
   the failing check). Fix the patch and give the fix its own tag. Then update
   the guest's `[patch.crates-io]` to that tag and remove it from `XFAIL`.
3. When a patch moves to a new upstream release (e.g. ruint 1.20), bump the
   versions in that guest's `Cargo.toml`. Keep the previous revision as a
   `<crate>-legacy/` guest only while the docs still point users to it.
