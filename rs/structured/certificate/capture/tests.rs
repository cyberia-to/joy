use super::*;
use crate::structured::certificate::CertificateLimits;
use nox::sequential::observe::{reduce_compacting_observed_v2_controlled, CaptureLimits};
use nox::{sequential::CompactionLimits, Order, Outcome, Reduction};
use std::io::{Cursor, Read};

mod mutations;

fn atom(ar: &mut Reduction<256>, value: u64) -> Order {
    ar.atom(nebu::Goldilocks::new(value)).unwrap()
}
fn op(ar: &mut Reduction<256>, tag: u64, value: Order) -> Order {
    let tag = atom(ar, tag);
    ar.pair(tag, value).unwrap()
}
fn binary(ar: &mut Reduction<256>, tag: u64, a: Order, b: Order) -> Order {
    let body = ar.pair(a, b).unwrap();
    op(ar, tag, body)
}
fn quote(ar: &mut Reduction<256>, value: u64) -> Order {
    let value = atom(ar, value);
    op(ar, 1, value)
}
fn axis(ar: &mut Reduction<256>, value: u64) -> Order {
    let value = atom(ar, value);
    op(ar, 0, value)
}
fn particle(ar: &Reduction<256>, id: Order) -> Particle {
    ar.digest(id).unwrap().map(|word| word.as_u64())
}

fn fixture() -> (Reduction<256>, Order, Order, Context) {
    let mut ar = Reduction::<256>::new();
    let (q0, q1) = (quote(&mut ar, 0), quote(&mut ar, 1));
    let code = axis(&mut ar, 2);
    let remaining = axis(&mut ar, 6);
    let acc = axis(&mut ar, 7);
    let done = binary(&mut ar, 9, remaining, q0);
    let next = binary(&mut ar, 6, remaining, q1);
    let acc_next = binary(&mut ar, 5, acc, q1);
    let state = binary(&mut ar, 3, next, acc_next);
    let subject = binary(&mut ar, 3, code, state);
    let again = binary(&mut ar, 2, subject, code);
    let arms = ar.pair(acc, again).unwrap();
    let loop_formula = binary(&mut ar, 4, done, arms);
    let (zero, count) = (atom(&mut ar, 0), atom(&mut ar, 40));
    let state = ar.pair(count, zero).unwrap();
    let object = ar.pair(loop_formula, state).unwrap();
    let formula = binary(&mut ar, 3, loop_formula, loop_formula);
    assert!(ar.limit_allocations(100));
    let context = Context {
        program: particle(&ar, formula),
        formula: particle(&ar, formula),
        object: particle(&ar, object),
        profile: 0,
        budget: 3000,
        frames: 4096,
    };
    (ar, object, formula, context)
}

fn limits(slots: u32) -> CertificateLimits {
    CertificateLimits {
        nouns: 100,
        cache_slots: slots,
        records: 100_000,
        steps: 10_000,
        wire_bytes: 1 << 20,
        decoded_bytes: 1 << 20,
    }
}
fn runtime_limits() -> CompactionLimits {
    CompactionLimits {
        max_frames: 4096,
        max_total_allocations: 100_000,
        max_collection_work: 100_000_000,
    }
}
fn capture_limits() -> CaptureLimits {
    CaptureLimits {
        max_events: 100_000,
        max_bytes: 10_000_000,
        max_work: 1_000_000,
    }
}

fn checked_bytes(bytes: &[u8], context: Context, slots: u32, value: Particle, cost: u64) {
    let mut session = Session::new(context, limits(slots)).unwrap();
    let mut cursor = Cursor::new(bytes);
    let mut tag = [0];
    while cursor.read(&mut tag).unwrap() != 0 {
        let record = Record::read(tag[0], &mut cursor).unwrap();
        session.apply(&record).unwrap();
    }
    let terminal = session.terminal(value, cost).unwrap();
    assert_eq!(terminal.summary().cost(), cost);
    assert_eq!(terminal.remaining(), context.budget - cost);
}

#[test]
fn repeated_computed_subtree_reuses_summary_across_actual_gc_snapshots() {
    let (mut ar, object, formula, context) = fixture();
    let mut capture = Capture::new(
        Vec::new(),
        Session::new(context, limits(64)).unwrap(),
        context,
    )
    .unwrap();
    let result = reduce_compacting_observed_v2_controlled(
        &mut ar,
        object,
        formula,
        context.budget,
        runtime_limits(),
        capture_limits(),
        &mut capture,
        &mut || false,
    )
    .unwrap();
    let Outcome::Ok(value, remaining) = result.execution.outcome else {
        panic!()
    };
    let output = particle(&ar, value);
    let stats = capture.stats();
    assert!(stats.reused_calls > 0);
    assert!(stats.suppressed_enters > 0);
    assert!(stats.resets_during_reuse > 0);
    assert_eq!(stats.resets, result.execution.stats.collections + 1);
    assert_eq!(
        stats.transitions + 1,
        result.execution.stats.evaluator_checkpoints
    );
    let (bytes, session) = capture.complete().unwrap();
    let cost = context.budget - remaining;
    assert_eq!(cost, 1211);
    assert_eq!(
        session.terminal(output, cost).unwrap().summary().cost(),
        cost
    );
    checked_bytes(&bytes, context, 64, output, cost);
}

#[test]
fn one_slot_eviction_and_large_cache_verify_the_same_result_and_charge() {
    let mut expected = None;
    for slots in [1, 64] {
        let (mut ar, object, formula, context) = fixture();
        let mut capture = Capture::new(
            Vec::new(),
            Session::new(context, limits(slots)).unwrap(),
            context,
        )
        .unwrap();
        let result = reduce_compacting_observed_v2_controlled(
            &mut ar,
            object,
            formula,
            context.budget,
            runtime_limits(),
            capture_limits(),
            &mut capture,
            &mut || false,
        )
        .unwrap();
        let Outcome::Ok(value, remaining) = result.execution.outcome else {
            panic!()
        };
        let pair = (particle(&ar, value), context.budget - remaining);
        assert_eq!(*expected.get_or_insert(pair), pair);
        assert!(capture.cache.len() <= slots as usize);
        let (bytes, session) = capture.complete().unwrap();
        session.terminal(pair.0, pair.1).unwrap();
        checked_bytes(&bytes, context, slots, pair.0, pair.1);
    }
}

#[derive(Default)]
struct Events(Vec<EventV2>);
impl ObserverV2 for Events {
    type Error = String;
    fn record(&mut self, event: EventV2) -> Result<(), String> {
        self.0.push(event);
        Ok(())
    }
}
fn events() -> (Context, Vec<EventV2>) {
    let (mut ar, object, formula, context) = fixture();
    let mut sink = Events::default();
    reduce_compacting_observed_v2_controlled(
        &mut ar,
        object,
        formula,
        context.budget,
        runtime_limits(),
        capture_limits(),
        &mut sink,
        &mut || false,
    )
    .unwrap();
    (context, sink.0)
}

fn replay_fails(context: Context, events: &[EventV2]) -> String {
    let mut capture = Capture::new(
        Vec::new(),
        Session::new(context, limits(64)).unwrap(),
        context,
    )
    .unwrap();
    let mut failure = None;
    for &event in events {
        if let Err(error) = capture.record(event) {
            failure = Some(error);
            break;
        }
    }
    if let Some(error) = failure {
        assert!(capture.record(events[0]).is_err());
        assert!(capture.complete().is_err());
        error
    } else {
        assert!(capture.complete().is_err());
        "unfinished".into()
    }
}

#[test]
fn complete_requires_delivered_completed_and_rejects_later_events() {
    let (context, events) = events();
    assert!(matches!(
        events.last(),
        Some(EventV2::Event(Event::Completed { .. }))
    ));
    assert_eq!(
        replay_fails(context, &events[..events.len() - 1]),
        "unfinished"
    );
    let mut duplicate = events.clone();
    duplicate.push(*events.last().unwrap());
    assert_eq!(
        replay_fails(context, &duplicate),
        "certificate Completed binding"
    );
    let mut extra = events;
    extra.push(EventV2::Event(Event::Node(Node {
        particle: [0; 4],
        value: NodeValue::Atom(0),
        bound: nox::Cost::Exact(0),
    })));
    assert_eq!(
        replay_fails(context, &extra),
        "certificate capture event order"
    );
}
