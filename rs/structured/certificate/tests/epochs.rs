use super::*;

#[test]
fn epoch_snapshot_counts_prior_indices_and_canonical_atoms_are_enforced() {
    let f = fixture(1);
    let good = f.records();
    let malformed = vec![
        vec![Record::Atom(0)],
        vec![Record::Enter {
            object: 0,
            formula: 0,
        }],
        vec![Record::Reset { epoch: 1, nodes: 1 }],
        vec![Record::Reset { epoch: 0, nodes: 0 }],
        vec![Record::Reset {
            epoch: 0,
            nodes: caps().nouns + 1,
        }],
        vec![
            Record::Reset { epoch: 0, nodes: 2 },
            Record::Atom(0),
            Record::Reset { epoch: 1, nodes: 1 },
        ],
        vec![
            Record::Reset { epoch: 0, nodes: 2 },
            Record::Atom(0),
            enter(0, 0),
        ],
        vec![
            Record::Reset { epoch: 0, nodes: 1 },
            Record::Atom(nebu::field::P),
        ],
        vec![
            Record::Reset { epoch: 0, nodes: 1 },
            Record::Pair { left: 0, right: 0 },
        ],
        vec![
            Record::Reset { epoch: 0, nodes: 1 },
            Record::Atom(0),
            Record::Pair { left: 0, right: 2 },
        ],
    ];
    for bad in malformed {
        assert!(f.verify(&f.payload(&bad)).is_err(), "{bad:?}");
    }
    for epoch in [0, 2, u64::MAX] {
        let mut records = good.clone();
        records.insert(1 + f.ar.count() as usize, Record::Reset { epoch, nodes: 1 });
        assert!(f
            .verify(&f.payload(&records))
            .unwrap_err()
            .contains("epoch"));
    }
    // A full valid table can be remapped between Enter and Finish. A stale ID
    // now points at an unrelated atom and cannot authenticate the old result.
    let split = 1 + f.ar.count() as usize + 1;
    let mut records = good[..split].to_vec();
    records.push(Record::Reset { epoch: 1, nodes: 3 });
    records.push(Record::Atom(999));
    records.push(Record::Atom(998));
    records.push(Record::Atom(7));
    records.push(finish(2));
    assert!(f.verify(&f.payload(&records)).is_ok());
    *records.last_mut().unwrap() = finish(0);
    assert!(f
        .verify(&f.payload(&records))
        .unwrap_err()
        .contains("Output"));
    *records.last_mut().unwrap() = finish(f.result);
    assert!(f.verify(&f.payload(&records)).is_err());
}

#[test]
fn cache_reuse_survives_reset_but_generation_key_and_eviction_cannot_be_forged() {
    let mut f = fixture(5);
    let (q, seven) = quote(&mut f.ar, 7);
    f.formula = binary(&mut f.ar, 5, q, q);
    f.program = art(&mut f.ar, f.formula, 0);
    f.result = atom(&mut f.ar, 14);
    f.actions = vec![
        enter(f.object, f.formula),
        enter(f.object, q),
        Record::Finish {
            result: seven,
            cache_slot: Some(0),
        },
        Record::Reset { epoch: 1, nodes: 1 },
        Record::Atom(14),
        Record::Reuse(CacheHandle {
            slot: 0,
            generation: 1,
        }),
        finish(0),
    ];
    let done = f.verify(&f.payload(&f.records())).unwrap();
    assert_eq!(
        (
            done.report.invocations,
            done.report.expanded_steps,
            done.report.semantic_events
        ),
        (3, 6, 5)
    );
    for handle in [
        CacheHandle {
            slot: 0,
            generation: 0,
        },
        CacheHandle {
            slot: 0,
            generation: 2,
        },
        CacheHandle {
            slot: 1,
            generation: 1,
        },
        CacheHandle {
            slot: caps().cache_slots,
            generation: 1,
        },
    ] {
        let mut actions = f.actions.clone();
        actions[5] = Record::Reuse(handle);
        std::mem::swap(&mut actions, &mut f.actions);
        assert!(f
            .verify(&f.payload(&f.records()))
            .unwrap_err()
            .contains("Cache"));
        std::mem::swap(&mut actions, &mut f.actions);
    }
    // Replacing the first child completion with reuse cannot import a summary.
    let mut f = fixture(5);
    f.actions[1] = Record::Reuse(CacheHandle {
        slot: 0,
        generation: 1,
    });
    assert!(f
        .verify(&f.payload(&f.records()))
        .unwrap_err()
        .contains("Cache"));
    let mut f = fixture(5);
    let Record::Finish { result, .. } = f.actions[2] else {
        unreachable!()
    };
    f.actions[2] = Record::Finish {
        result,
        cache_slot: Some(0),
    };
    f.actions[3] = Record::Reuse(CacheHandle {
        slot: 0,
        generation: 1,
    });
    assert!(f
        .verify(&f.payload(&f.records()))
        .unwrap_err()
        .contains("Key"));
}

#[test]
fn session_cannot_recover_after_any_failed_record() {
    let f = fixture(1);
    for bad in [
        Record::Atom(0),
        Record::Reset { epoch: 2, nodes: 1 },
        Record::Reuse(CacheHandle {
            slot: 0,
            generation: 1,
        }),
    ] {
        let mut session = session::Session::new(f.context(), caps()).unwrap();
        assert!(session.apply(&bad).is_err());
        assert!(session
            .apply(&Record::Reset { epoch: 0, nodes: 1 })
            .unwrap_err()
            .contains("previously failed"));
        assert!(session.terminal(particle(&f.ar, f.result), f.cost).is_err());
    }
}

#[test]
fn unknown_record_tags_and_short_record_fields_are_rejected_after_rechaining() {
    let f = fixture(1);
    for tag in [7, 127, 255] {
        assert!(f.verify(&[tag]).unwrap_err().contains("unknown"));
    }
    for record in f.records() {
        let mut encoded = Vec::new();
        record.write(&mut encoded).unwrap();
        for length in 1..encoded.len() {
            assert!(f.verify(&encoded[..length]).is_err());
        }
    }
}

#[test]
fn complete_epoch_remapping_between_every_continuation_phase_preserves_derivation() {
    for tag in [1, 5, 2, 4, 40] {
        let f = fixture(tag);
        let mut records = Vec::new();
        for (epoch, &action) in f.actions.iter().enumerate() {
            let padding = 1 + epoch as u32 % 3;
            records.push(Record::Reset {
                epoch: epoch as u64,
                nodes: f.ar.count() + padding,
            });
            for n in 0..padding {
                records.push(Record::Atom(50_000 + u64::from(n)));
            }
            for id in 0..f.ar.count() {
                records.push(match f.ar.get(id).unwrap().inner {
                    Data::Atom { value } => Record::Atom(value.as_u64()),
                    Data::Pair { left, right } => Record::Pair {
                        left: left + padding,
                        right: right + padding,
                    },
                });
            }
            records.push(match action {
                Record::Enter { object, formula } => enter(object + padding, formula + padding),
                Record::Finish { result, cache_slot } => Record::Finish {
                    result: result + padding,
                    cache_slot,
                },
                _ => unreachable!(),
            });
        }
        let done = f.verify(&f.payload(&records)).unwrap();
        assert_eq!(done.output, encode(&f.ar, f.result));
        assert_eq!(done.report.records, records.len() as u64 + 1);
    }
}

#[test]
fn real_cache_eviction_invalidates_previous_generation() {
    let mut f = fixture(5);
    for index in [2, 4] {
        let Record::Finish { result, .. } = f.actions[index] else {
            unreachable!()
        };
        f.actions[index] = Record::Finish {
            result,
            cache_slot: Some(0),
        };
    }
    f.verify(&f.payload(&f.records())).unwrap();
    f.actions.insert(
        5,
        Record::Reuse(CacheHandle {
            slot: 0,
            generation: 1,
        }),
    );
    assert!(f
        .verify(&f.payload(&f.records()))
        .unwrap_err()
        .contains("Cache"));
}
