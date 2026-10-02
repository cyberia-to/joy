use super::*;

#[test]
fn begin_noun_sequence_and_snapshot_mutations_cannot_complete() {
    let (context, original) = events();
    for mutation in 0..7 {
        let mut bad = original.clone();
        match mutation {
            0 => {
                if let EventV2::Event(Event::Begin { version, .. }) = &mut bad[0] {
                    *version = 1;
                }
            }
            1 => {
                if let EventV2::Event(Event::Begin {
                    initial: LogicalAction::Enter { budget, .. },
                    ..
                }) = &mut bad[0]
                {
                    *budget += 1;
                }
            }
            2 => {
                if let EventV2::Event(Event::Node(node)) = &mut bad[1] {
                    node.particle[0] ^= 1;
                }
            }
            3 => {
                if let EventV2::Event(Event::Node(node)) = &mut bad[1] {
                    node.bound = nox::Cost::Dynamic(0);
                }
            }
            4 => {
                let t = bad
                    .iter_mut()
                    .find_map(|e| {
                        if let EventV2::Event(Event::Transition(t)) = e {
                            Some(t)
                        } else {
                            None
                        }
                    })
                    .unwrap();
                t.sequence += 1;
            }
            5 => {
                let reset = bad
                    .iter_mut()
                    .find(|e| matches!(e, EventV2::ArenaReset { .. }))
                    .unwrap();
                if let EventV2::ArenaReset { next_sequence, .. } = reset {
                    *next_sequence += 1;
                }
            }
            _ => {
                let reset = bad
                    .iter_mut()
                    .find(|e| matches!(e, EventV2::ArenaReset { .. }))
                    .unwrap();
                if let EventV2::ArenaReset { live_nodes, .. } = reset {
                    *live_nodes += 1;
                }
            }
        }
        assert_ne!(replay_fails(context, &bad), "unfinished");
    }
}

#[test]
fn reused_scope_checks_actual_return_cost_even_after_semantic_reuse() {
    let (context, mut events) = events();
    let mut seen = BTreeMap::new();
    let mut selected = None;
    // The largest repeated subtree is the loop applied twice to the root object.
    // It differs from the outer pair formula and both calls start at depth1.
    for (i, event) in events.iter().enumerate() {
        if let EventV2::Event(Event::Transition(t)) = event {
            if let LogicalAction::Enter {
                object, formula, ..
            } = t.before
            {
                if t.depth_before == 1 && object == context.object && formula != context.formula {
                    if seen.insert((object, formula), i).is_some() {
                        selected = Some(i);
                        break;
                    }
                }
            }
        }
    }
    let i = selected.unwrap();
    let previous = (0..i)
        .rev()
        .find(|&j| matches!(events[j], EventV2::Event(Event::Transition(_))))
        .unwrap();
    if let EventV2::Event(Event::Transition(t)) = &mut events[previous] {
        let LogicalAction::Enter { budget, .. } = &mut t.after else {
            panic!()
        };
        *budget += 1;
    }
    if let EventV2::Event(Event::Transition(t)) = &mut events[i] {
        let LogicalAction::Enter { budget, .. } = &mut t.before else {
            panic!()
        };
        *budget += 1;
    }
    assert_eq!(
        replay_fails(context, &events),
        "observed result/charge disagrees with verified summary"
    );
}

#[test]
fn fresh_count_and_depth_mutations_poison_capture() {
    let (context, events) = events();
    for change_depth in [false, true] {
        let mut bad = events.clone();
        let t = bad
            .iter_mut()
            .find_map(|e| {
                if let EventV2::Event(Event::Transition(t)) = e {
                    Some(t)
                } else {
                    None
                }
            })
            .unwrap();
        if change_depth {
            t.depth_before += 1;
        } else {
            t.fresh_nodes += 1;
        }
        assert_ne!(replay_fails(context, &bad), "unfinished");
    }
}

#[test]
fn failed_sink_leaves_capture_unusable_and_never_complete() {
    struct Broken;
    impl Write for Broken {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("injected write failure"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let (context, events) = events();
    let mut capture =
        Capture::new(Broken, Session::new(context, limits(64)).unwrap(), context).unwrap();
    assert!(capture.record(events[0]).is_err());
    assert!(capture.record(events[0]).is_err());
    assert!(capture.complete().is_err());
}

#[test]
fn observed_work_cap_applies_while_semantic_records_are_suppressed() {
    let (context, events) = events();
    let mut probe = Capture::new(
        Vec::new(),
        Session::new(context, limits(64)).unwrap(),
        context,
    )
    .unwrap();
    let mut cap = None;
    for &event in &events {
        probe.record(event).unwrap();
        if probe.suppression.is_some() {
            cap = Some(probe.stats.transitions);
            break;
        }
    }
    let mut bounded = limits(64);
    bounded.steps = cap.unwrap();
    let mut capture =
        Capture::new(Vec::new(), Session::new(context, bounded).unwrap(), context).unwrap();
    let mut rejected = false;
    for &event in &events {
        if let Err(error) = capture.record(event) {
            assert_eq!(error, "observed transition allowance");
            assert!(capture.suppression.is_some());
            assert_eq!(capture.stats.transitions, bounded.steps);
            rejected = true;
            break;
        }
    }
    assert!(rejected);
    assert!(capture.complete().is_err());
}

#[test]
fn reused_scope_checks_actual_return_particle_before_completion() {
    let (context, mut events) = events();
    let mut probe = Capture::new(
        Vec::new(),
        Session::new(context, limits(64)).unwrap(),
        context,
    )
    .unwrap();
    let mut selected = None;
    for (i, &event) in events.iter().enumerate() {
        if let EventV2::Event(Event::Transition(t)) = event {
            if let LogicalAction::Return { value, .. } = t.before {
                if probe
                    .calls
                    .last()
                    .is_some_and(|call| matches!(call.kind, CallKind::Reused))
                {
                    let replacement = *probe.nouns.keys().find(|&&p| p != value).unwrap();
                    selected = Some((i, replacement));
                    break;
                }
            }
        }
        probe.record(event).unwrap();
    }
    let (i, replacement) = selected.unwrap();
    let previous = (0..i)
        .rev()
        .find(|&j| matches!(events[j], EventV2::Event(Event::Transition(_))))
        .unwrap();
    if let EventV2::Event(Event::Transition(t)) = &mut events[previous] {
        let LogicalAction::Return { value, .. } = &mut t.after else {
            panic!()
        };
        *value = replacement;
    }
    if let EventV2::Event(Event::Transition(t)) = &mut events[i] {
        let LogicalAction::Return { value, .. } = &mut t.before else {
            panic!()
        };
        *value = replacement;
    }
    assert_eq!(
        replay_fails(context, &events),
        "observed result/charge disagrees with verified summary"
    );
}
