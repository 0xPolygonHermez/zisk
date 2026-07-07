//! Recurser example — folding private-vector hash proofs. Each leaf proves only
//! `Poseidon1(secret)`; the fold sums the secret vectors (via free inputs) and
//! exposes `Poseidon1(sum)`. See the crate README.
//!
//! Run: `cargo run --release -p recurser-hash-host`

use std::error::Error;

use proofman_util::{timer_start_info, timer_stop_and_log_info};
use recurser_hash_common::{add_vecs, field_from_limbs, hash12, DIGEST, RATE};
use zisk_sdk::{
    load_aggregation_program, load_program, AggregationProgram, GuestProgram, ProofExt,
    ProverClient, ZiskStdin,
};

static LEAF: GuestProgram = load_program!("recurser_hash_guest");
static AGG_HASH: AggregationProgram = load_aggregation_program!("hash");

/// Leaf digest: 8 u32 limbs (each < 2^32), reassembled pairwise from the u32 view.
fn leaf_digest(proof: &zisk_sdk::Proof) -> [u64; DIGEST] {
    let slots = proof.get_publics().public_u64();
    std::array::from_fn(|k| field_from_limbs(slots[2 * k] as u32, slots[2 * k + 1] as u32))
}

/// Folded digest: 4 full field elements in user slots [0..4). They exceed 32 bits,
/// so read `publics_full()` (`[program_vk(4) | user(64)]`), not the truncating
/// `public_u64()`.
fn folded_digest(proof: &zisk_sdk::Proof) -> [u64; DIGEST] {
    let full = proof.publics_full().expect("vadcop proof has full publics");
    std::array::from_fn(|k| full[4 + k]) // user publics start after the 4-limb VK
}

fn as_free(v: &[u64; RATE]) -> Vec<u64> {
    v.to_vec()
}

async fn prove_leaf(
    client: &zisk_sdk::EmbeddedClient,
    secret: &[u64; RATE],
) -> Result<zisk_sdk::Proof, Box<dyn Error>> {
    let stdin = ZiskStdin::new();
    stdin.write(secret); // one [u64; 12], matching the guest's single read
    Ok(client.prove(&LEAF, stdin).run()?.await?.get_proof().clone())
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = ProverClient::embedded().build()?;

    client.setup(&LEAF).run()?.await?;
    client.setup(&*AGG_HASH).run()?.await?;

    // Three leaves with distinct secret vectors.
    let va: [u64; RATE] = std::array::from_fn(|i| (i as u64) + 1); //  1..12
    let vb: [u64; RATE] = std::array::from_fn(|i| (i as u64) + 100); // 100..111
    let vc: [u64; RATE] = std::array::from_fn(|i| (i as u64) + 1000); // 1000..1011

    let pa = prove_leaf(&client, &va).await?;
    let pb = prove_leaf(&client, &vb).await?;
    let pc = prove_leaf(&client, &vc).await?;

    assert_eq!(leaf_digest(&pa), hash12(&va), "leaf A digest");
    assert_eq!(leaf_digest(&pb), hash12(&vb), "leaf B digest");
    println!("Leaf digests match Poseidon1(secret).");

    // Fold A + B: each side's own vector is its free input; output = H(va + vb).
    timer_start_info!(AGGREGATE_A_B);
    let ab = client
        .aggregate_proofs(
            &AGG_HASH,
            pa.with_free_inputs(as_free(&va)),
            pb.with_free_inputs(as_free(&vb)),
        )
        .run()?
        .await?
        .get_proof()
        .clone();
    timer_stop_and_log_info!(AGGREGATE_A_B);

    let sum_ab = add_vecs(&va, &vb);
    assert_eq!(folded_digest(&ab), hash12(&sum_ab), "A+B digest == Poseidon1(va+vb)");
    println!("A + B folded: digest == Poseidon1(va + vb).");

    // Fold AB + C: AB's free input is the running sum (va+vb), C's is vc.
    timer_start_info!(AGGREGATE_AB_C);
    let abc = client
        .aggregate_proofs(
            &AGG_HASH,
            ab.with_free_inputs(as_free(&sum_ab)),
            pc.with_free_inputs(as_free(&vc)),
        )
        .run()?
        .await?
        .get_proof()
        .clone();
    timer_stop_and_log_info!(AGGREGATE_AB_C);

    let sum_abc = add_vecs(&sum_ab, &vc);
    assert_eq!(folded_digest(&abc), hash12(&sum_abc), "A+B+C digest == Poseidon1(sum)");
    println!("AB + C folded: digest == Poseidon1(va + vb + vc).");

    println!("Verifying the final folded proof...");
    abc.verify()?;
    println!("Final folded proof verified successfully.");

    // Negative: a free vector that doesn't hash to A's digest fails the binding.
    println!("Testing an invalid free input...");
    let mut wrong = va;
    wrong[0] = wrong[0].wrapping_add(1);
    let rejected = client
        .aggregate_proofs(
            &AGG_HASH,
            pa.with_free_inputs(as_free(&wrong)),
            pb.with_free_inputs(as_free(&vb)),
        )
        .run()?
        .await;
    assert!(rejected.is_err(), "a free vector not matching the proof's digest must be rejected");
    println!("Invalid free input correctly rejected by the digest binding.");

    Ok(())
}
