//! Independently authored records; no evaluator or producer constructs proofs.
use super::{records::Record, *};
use crate::structured::RunLimits;
use nebu::Goldilocks as F;
use nox::{artifact, data::Data, Reduction};
use std::io::{Cursor, Write};
use zheng::execution::disclosed::stream::CacheHandle;

mod attacks;
mod compiler;
mod epochs;

type Arena = Reduction<4096>;
fn atom(ar: &mut Arena, value: u64) -> u32 {
    ar.atom(F::new(value)).unwrap()
}
fn op(ar: &mut Arena, tag: u64, body: u32) -> u32 {
    let tag = atom(ar, tag);
    ar.pair(tag, body).unwrap()
}
fn quote(ar: &mut Arena, value: u64) -> (u32, u32) {
    let value = atom(ar, value);
    (op(ar, 1, value), value)
}
fn binary(ar: &mut Arena, tag: u64, a: u32, b: u32) -> u32 {
    let body = ar.pair(a, b).unwrap();
    op(ar, tag, body)
}
fn art(ar: &mut Arena, formula: u32, profile: u64) -> u32 {
    let zero = atom(ar, 0);
    let profile = atom(ar, profile);
    let mut tail = zero;
    for value in [zero, profile, profile, formula].into_iter().rev() {
        tail = ar.pair(value, tail).unwrap();
    }
    op(ar, 0x41525431, tail)
}
fn particle(ar: &Arena, id: u32) -> [u64; 4] {
    ar.digest(id).unwrap().map(F::as_u64)
}
fn host() -> RunLimits {
    RunLimits {
        arena_nodes: 2048,
        frames: 128,
        budget: 10_000,
        artifact_bytes: 16_384,
        artifact_nodes: 1024,
        artifact_depth: 256,
        ..RunLimits::default()
    }
}
fn caps() -> CertificateLimits {
    CertificateLimits {
        nouns: 2048,
        cache_slots: 8,
        records: 10000,
        steps: 10000,
        wire_bytes: 1 << 20,
        decoded_bytes: 1 << 20,
    }
}
fn encode(ar: &Arena, root: u32) -> Vec<u8> {
    artifact::encode(ar, root, host().transport()).unwrap()
}
fn enter(object: u32, formula: u32) -> Record {
    Record::Enter { object, formula }
}
fn finish(result: u32) -> Record {
    Record::Finish {
        result,
        cache_slot: None,
    }
}

struct Fixture {
    ar: Box<Arena>,
    program: u32,
    object: u32,
    formula: u32,
    result: u32,
    cost: u64,
    actions: Vec<Record>,
    profile: u8,
}
impl Fixture {
    fn context(&self) -> Context {
        Context {
            program: particle(&self.ar, self.program),
            object: particle(&self.ar, self.object),
            formula: particle(&self.ar, self.formula),
            profile: self.profile,
            budget: host().budget,
            frames: host().frames,
        }
    }
    fn records(&self) -> Vec<Record> {
        let mut out = vec![Record::Reset {
            epoch: 0,
            nodes: self.ar.count(),
        }];
        for id in 0..self.ar.count() {
            out.push(match self.ar.get(id).unwrap().inner {
                Data::Atom { value } => Record::Atom(value.as_u64()),
                Data::Pair { left, right } => Record::Pair { left, right },
            });
        }
        out.extend_from_slice(&self.actions);
        out
    }
    fn payload(&self, records: &[Record]) -> Vec<u8> {
        let mut out = Vec::new();
        for record in records {
            record.write(&mut out).unwrap();
        }
        terminal(
            &mut out,
            particle(&self.ar, self.result),
            self.cost,
            &encode(&self.ar, self.result),
        );
        out
    }
    fn verify(&self, payload: &[u8]) -> Result<CertificateResult, String> {
        self.verify_with(payload, self.context(), host(), caps())
    }
    fn verify_with(
        &self,
        payload: &[u8],
        context: Context,
        host: RunLimits,
        caps: CertificateLimits,
    ) -> Result<CertificateResult, String> {
        let proof = envelope(payload, context);
        pipeline::verify::<4096, _>(
            &encode(&self.ar, self.program),
            &encode(&self.ar, self.object),
            Cursor::new(proof),
            host,
            caps,
        )
    }
}
fn terminal(out: &mut Vec<u8>, particle: [u64; 4], cost: u64, result: &[u8]) {
    out.push(6);
    for limb in particle {
        out.extend_from_slice(&limb.to_le_bytes());
    }
    out.extend_from_slice(&cost.to_le_bytes());
    out.extend_from_slice(&(result.len() as u32).to_le_bytes());
    out.extend_from_slice(result);
}
fn envelope(payload: &[u8], context: Context) -> Vec<u8> {
    let mut writer =
        transport::Writer::new(Vec::new(), context.digest(), caps().transport()).unwrap();
    writer.write_all(payload).unwrap();
    writer.finish().unwrap().0
}
fn fixture(tag: u64) -> Fixture {
    let mut ar = Arena::try_new_boxed().unwrap();
    let object = atom(&mut ar, 0);
    let (q7, seven) = quote(&mut ar, 7);
    let (q9, nine) = quote(&mut ar, 9);
    let (formula, result, cost, children) = match tag {
        1 => (q7, seven, 1, vec![]),
        5 => {
            let f = binary(&mut ar, 5, q7, q9);
            let r = atom(&mut ar, 16);
            (
                f,
                r,
                3,
                vec![
                    enter(object, q7),
                    finish(seven),
                    enter(object, q9),
                    finish(nine),
                ],
            )
        }
        2 => {
            let one = atom(&mut ar, 1);
            let identity = op(&mut ar, 0, one);
            let qf = op(&mut ar, 1, identity);
            let f = binary(&mut ar, 2, q7, qf);
            (
                f,
                seven,
                4,
                vec![
                    enter(object, q7),
                    finish(seven),
                    enter(object, qf),
                    finish(identity),
                    enter(seven, identity),
                    finish(seven),
                ],
            )
        }
        4 | 40 => {
            let condition = if tag == 4 { 0 } else { 1 };
            let (test, value) = quote(&mut ar, condition);
            let arms = ar.pair(q7, q9).unwrap();
            let f = binary(&mut ar, 4, test, arms);
            let (selected, r) = if tag == 4 { (q7, seven) } else { (q9, nine) };
            (
                f,
                r,
                3,
                vec![
                    enter(object, test),
                    finish(value),
                    enter(object, selected),
                    finish(r),
                ],
            )
        }
        _ => panic!("unknown fixture"),
    };
    let program = art(&mut ar, formula, 0);
    let mut actions = vec![enter(object, formula)];
    actions.extend(children);
    actions.push(finish(result));
    Fixture {
        ar,
        program,
        object,
        formula,
        result,
        cost,
        actions,
        profile: 0,
    }
}

#[test]
fn handcrafted_quote_add_compose_and_both_branch_derivations_verify() {
    for tag in [1, 5, 2, 4, 40] {
        let f = fixture(tag);
        let done = f.verify(&f.payload(&f.records())).unwrap();
        assert_eq!(done.output, encode(&f.ar, f.result));
        assert!(done.compiled.is_none());
        assert_eq!(done.report.charged_reductions, f.cost);
        assert_eq!(done.report.semantic_events, f.actions.len() as u64);
        assert_eq!(done.report.expanded_steps, f.actions.len() as u64);
        assert_eq!(done.report.records, f.records().len() as u64 + 1);
    }
}
