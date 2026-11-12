//! zisk-patch-hashes / sha2: SHA-256 compression routed to `sha256f_compress_c`.

#![no_main]
ziskos::entrypoint!(main);

use patch_test_common::{Rng, Suite};
use sha2::{Digest, Sha224, Sha256, Sha512};

fn main() {
    let mut t = Suite::new("sha2");

    // FIPS 180-2 / NIST CSRC example vectors.
    t.eq_hex("sha256('')", &Sha256::digest(b""), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    t.eq_hex("sha256('abc')", &Sha256::digest(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    t.eq_hex(
        "sha256(448-bit msg)",
        &Sha256::digest(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
    );
    t.eq_hex(
        "sha256(1M x 'a')",
        &Sha256::digest(vec![b'a'; 1_000_000]),
        "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0",
    );
    // SHA-224 shares the patched compression function.
    t.eq_hex("sha224('abc')", &Sha224::digest(b"abc"), "23097d223405d8228642a477bda255b32aadbce4bda0b3f7e36c9da7");
    // SHA-512 is not patched; it must keep working alongside the patched code.
    t.eq_hex(
        "sha512('abc')",
        &Sha512::digest(b"abc"),
        "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f",
    );

    // Block-boundary lengths, and incremental updates split at awkward offsets.
    let mut rng = Rng::new(0x5a2);
    for len in [55usize, 56, 63, 64, 65, 119, 120, 127, 128, 129, 1000, 4097] {
        let msg = rng.bytes(len);
        let one_shot = Sha256::digest(&msg);
        let mut h = Sha256::new();
        for chunk in msg.chunks(7) {
            h.update(chunk);
        }
        t.eq(&format!("sha256 incremental == one-shot (len {len})"), h.finalize(), one_shot);
    }

    t.finish();
}
