//! Public nox execution proofs. No private input is accepted by this non-ZK format.
use crate::Warrior;
use serde::{Deserialize, Serialize};
use std::{fs, io::Read, path::Path};
use trident::runtime::{ExecutionResult, ProgramBundle, ProgramInput, ProofData, Runner};
use zheng::execution::{DirectProof, ExecutionNoun, ExecutionStatement};

pub const EXECUTION_FORMAT: &str = "zheng-nox-public-execution-v1";
const MAGIC: &[u8] = b"JOYEXEC1";
const MAX_ARTIFACT_BYTES: usize = 32 * 1024 * 1024;
const MAX_ASSEMBLY_BYTES: usize = 256 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionArtifact {
    pub format: String,
    pub program: String,
    pub assembly: String,
    pub statement: ExecutionStatement,
    pub proof: DirectProof,
}

/// Bounded bracket parser shared semantically with nox's binary/n-ary syntax.
/// Converts atoms canonically exactly as the native field-valued noun parser.
fn parse_program(text: &str) -> Result<ExecutionNoun, String> {
    if text.len() > MAX_ASSEMBLY_BYTES {
        return Err("execution program is too large".into());
    }
    let mut frames: Vec<Vec<(ExecutionNoun, usize)>> = vec![Vec::new()];
    let b = text.as_bytes();
    let mut pos = 0;
    let mut nodes = 0;
    while pos < b.len() {
        match b[pos] {
            c if c.is_ascii_whitespace() => pos += 1,
            b'[' => {
                if frames.len() > 128 {
                    return Err("formula depth limit".into());
                }
                frames.push(vec![]);
                pos += 1;
            }
            b']' => {
                if frames.len() == 1 {
                    return Err("unexpected closing bracket".into());
                }
                let mut values = frames.pop().ok_or("formula frame missing")?;
                let (mut tail, mut depth) = values.pop().ok_or("empty cell")?;
                if values.is_empty() {
                    return Err("cell needs at least two elements".into());
                }
                while let Some((head, hd)) = values.pop() {
                    depth = 1 + depth.max(hd);
                    if depth > 128 {
                        return Err("formula depth limit".into());
                    }
                    tail = ExecutionNoun::Pair(Box::new(head), Box::new(tail));
                    nodes += 1;
                }
                frames
                    .last_mut()
                    .ok_or("formula frame missing")?
                    .push((tail, depth));
                pos += 1;
            }
            c if c.is_ascii_digit() => {
                let start = pos;
                while pos < b.len() && b[pos].is_ascii_digit() {
                    pos += 1;
                }
                let word = text[start..pos]
                    .parse::<u64>()
                    .map_err(|_| "invalid atom")?;
                let word = nebu::Goldilocks::new(word).canonicalize().as_u64();
                frames
                    .last_mut()
                    .ok_or("formula frame missing")?
                    .push((ExecutionNoun::Atom(word), 0));
                nodes += 1;
            }
            _ => return Err("invalid formula character".into()),
        }
        if nodes > 4096 {
            return Err("formula node limit".into());
        }
    }
    if frames.len() != 1 {
        return Err("unclosed bracket".into());
    }
    let mut values = frames.pop().ok_or("empty program")?;
    if values.len() != 1 {
        return Err("program must contain exactly one noun".into());
    }
    let (noun, _) = values.pop().ok_or("empty program")?;
    ExecutionStatement::encode_program(&noun)?;
    Ok(noun)
}

impl ExecutionArtifact {
    pub fn has_header(path: &Path) -> bool {
        let Ok(mut file) = fs::File::open(path) else {
            return false;
        };
        let mut header = [0u8; 8];
        file.read_exact(&mut header).is_ok() && header == MAGIC
    }
    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        let mut bytes = MAGIC.to_vec();
        bytes.extend(postcard::to_allocvec(self).map_err(|e| e.to_string())?);
        if bytes.len() > MAX_ARTIFACT_BYTES {
            return Err("execution artifact size limit".into());
        }
        Ok(bytes)
    }
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_ARTIFACT_BYTES {
            return Err("execution artifact size limit".into());
        }
        let payload = bytes
            .strip_prefix(MAGIC)
            .ok_or("not a public execution artifact")?;
        let (artifact, rest): (Self, &[u8]) =
            postcard::take_from_bytes(payload).map_err(|e| e.to_string())?;
        if !rest.is_empty() || artifact.format != EXECUTION_FORMAT {
            return Err("invalid execution artifact format".into());
        }
        Ok(artifact)
    }
    pub fn load(path: &Path) -> Result<Self, String> {
        if fs::metadata(path).map_err(|e| e.to_string())?.len() > MAX_ARTIFACT_BYTES as u64 {
            return Err("execution artifact size limit".into());
        }
        Self::from_bytes(&fs::read(path).map_err(|e| e.to_string())?)
    }
    pub fn save(&self, path: &Path) -> Result<usize, String> {
        let bytes = self.to_bytes()?;
        fs::write(path, &bytes).map_err(|e| e.to_string())?;
        Ok(bytes.len())
    }
    pub fn matches_program(&self, assembly: &str) -> Result<bool, String> {
        Ok(
            ExecutionStatement::encode_program(&parse_program(assembly)?)?
                == self.statement.program,
        )
    }
    pub fn verify(&self) -> Result<(), String> {
        if self.format != EXECUTION_FORMAT || self.program.len() > 4096 {
            return Err("invalid execution artifact metadata".into());
        }
        let program = parse_program(&self.assembly)?;
        if ExecutionStatement::encode_program(&program)? != self.statement.program {
            return Err("assembly differs from execution statement".into());
        }
        zheng::execution::verify_execution(&self.statement, &self.proof)
    }
    pub fn claim_matches(
        &self,
        claim: Option<&[u64]>,
        inputs: Option<&[u64]>,
        budget: u64,
    ) -> bool {
        claim.is_none_or(|c| c == self.statement.public_output)
            && inputs.is_none_or(|i| i == self.statement.public_input)
            && self.statement.budget <= budget
    }
    pub fn proof_data(&self) -> Result<ProofData, String> {
        Ok(ProofData {
            claim: trident::field::proof::Claim {
                program_hash: crate::program_hash(&self.assembly)
                    .chunks_exact(8)
                    .map(|c| {
                        let mut a = [0; 8];
                        a.copy_from_slice(c);
                        u64::from_le_bytes(a)
                    })
                    .collect(),
                public_input: self.statement.public_input.clone(),
                public_output: self.statement.public_output.clone(),
            },
            proof_bytes: self.to_bytes()?,
            format: EXECUTION_FORMAT.into(),
        })
    }
}

impl Warrior {
    /// Produce a verifier-checked public execution proof; never downgrades to a
    /// relaxed trace statement. Native execution is an independent prover check.
    pub fn prove_execution(
        &self,
        bundle: &ProgramBundle,
        input: &ProgramInput,
        budget: u64,
    ) -> Result<(ExecutionArtifact, ExecutionResult), String> {
        if bundle.target_vm != "nox" || bundle.reads_state || !input.digests.is_empty() {
            return Err("public execution proofs require stateless nox inputs".into());
        }
        if !input.secret.is_empty() {
            return Err(
                "public execution proofs are not zero knowledge; secret inputs are refused".into(),
            );
        }
        let program = parse_program(&bundle.assembly)?;
        let (statement, proof) =
            zheng::execution::prove_execution(&program, &input.public, budget)?;
        let native = self.run(bundle, input)?;
        if native.output != statement.public_output || native.cycle_count != statement.cycles {
            return Err("execution relation disagrees with native nox".into());
        }
        let artifact = ExecutionArtifact {
            format: EXECUTION_FORMAT.into(),
            program: bundle.name.clone(),
            assembly: bundle.assembly.clone(),
            statement,
            proof,
        };
        Ok((artifact, native))
    }
}

pub fn verify_proof_data(data: &ProofData) -> Result<bool, String> {
    if data.format != EXECUTION_FORMAT {
        return Err("unsupported execution proof format".into());
    }
    let artifact = ExecutionArtifact::from_bytes(&data.proof_bytes)?;
    if artifact.proof_data()?.claim != data.claim {
        return Ok(false);
    }
    Ok(artifact.verify().is_ok())
}
