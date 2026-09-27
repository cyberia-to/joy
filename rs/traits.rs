//! Runtime traits dispatch to native soft3 proof profiles.
use crate::Warrior;
use trident::runtime::{Deployer, ProgramBundle, ProgramInput, ProofData, Prover, Verifier};

impl Prover for Warrior {
    fn prove(&self, bundle: &ProgramBundle, input: &ProgramInput) -> Result<ProofData, String> {
        if !input.secret.is_empty() {
            let (artifact, _) = self.prove_zk_execution(bundle, input, self.budget)?;
            return artifact.proof_data();
        }
        let (artifact, _) = self.prove_execution(bundle, input, self.budget)?;
        artifact.proof_data()
    }
}

impl Verifier for Warrior {
    fn verify(&self, proof: &ProofData) -> Result<bool, String> {
        if proof.format == crate::ZK_EXECUTION_FORMAT {
            return crate::zk_execution::verify_proof_data(proof);
        }
        if proof.format == crate::STATE_EXECUTION_FORMAT {
            return crate::state_execution::verify_proof_data(proof);
        }
        crate::execution::verify_proof_data(proof)
    }
}

impl Deployer for Warrior {
    fn deploy(
        &self,
        _bundle: &ProgramBundle,
        _proof: Option<&ProofData>,
    ) -> Result<String, String> {
        Err("deploy lands after the zheng prover (M4): particle + cyberlink emission".to_string())
    }
}
