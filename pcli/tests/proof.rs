//! Tests guard against drift in the current constraints versus the provided
//! proving/verification key.

use ark_ff::UniformRand;
use decaf377::Fr;
use penumbra_crypto::{
    asset,
    keys::{SeedPhrase, SpendKey},
    proofs::groth16::SpendProof,
    rdsa::{SpendAuth, VerificationKey},
    Note, Value,
};
use penumbra_proof_params::{SPEND_PROOF_PROVING_KEY, SPEND_PROOF_VERIFICATION_KEY};
use penumbra_tct as tct;
use rand_core::OsRng;

#[test]
fn spend_proof_parameters_vs_current_spend_circuit() {
    let pk = &*SPEND_PROOF_PROVING_KEY;
    let vk = &*SPEND_PROOF_VERIFICATION_KEY;

    let seed_phrase = SeedPhrase::generate(OsRng);
    let sk_sender = SpendKey::from_seed_phrase(seed_phrase, 0);
    let fvk_sender = sk_sender.full_viewing_key();
    let ivk_sender = fvk_sender.incoming();
    let (sender, _dtk_d) = ivk_sender.payment_address(0u32.into());

    let value_to_send = Value {
        amount: 1u64.into(),
        asset_id: asset::REGISTRY.parse_denom("upenumbra").unwrap().id(),
    };

    let note = Note::generate(&mut OsRng, &sender, value_to_send);
    let note_commitment = note.commit();
    let spend_auth_randomizer = Fr::rand(&mut OsRng);
    let rsk = sk_sender.spend_auth_key().randomize(&spend_auth_randomizer);
    let nk = *sk_sender.nullifier_key();
    let ak: VerificationKey<SpendAuth> = sk_sender.spend_auth_key().into();
    let mut nct = tct::Tree::new();
    nct.insert(tct::Witness::Keep, note_commitment).unwrap();
    let anchor = nct.root();
    let note_commitment_proof = nct.witness(note_commitment).unwrap();
    let v_blinding = Fr::rand(&mut OsRng);
    let balance_commitment = value_to_send.commit(v_blinding);
    let rk: VerificationKey<SpendAuth> = rsk.into();
    let nf = nk.derive_nullifier(0.into(), &note_commitment);

    let proof = SpendProof::prove(
        &mut OsRng,
        pk,
        note_commitment_proof,
        note,
        v_blinding,
        spend_auth_randomizer,
        ak,
        nk,
        anchor,
        balance_commitment,
        nf,
        rk,
    )
    .expect("can create proof");

    let proof_result = proof.verify(vk, anchor, balance_commitment, nf, rk);
    assert!(proof_result.is_ok());
}