//! Bounded host observations; contract: specs/compiler-proof-census.md.
use super::*;
use nox::sequential::observe::{
    Event, LiveFrame, LogicalAction, Node, NodeValue, Observer, Particle,
};
use serde::Serialize;
use std::{cell::Cell, collections::BTreeMap};

mod driver;
mod tests;

#[derive(Clone, Copy, Debug, Serialize)]
struct Caps {
    transitions: u64,
    nouns: usize,
    evaluations: usize,
    frames: usize,
}

impl Caps {
    fn validate(self) -> Result<(), &'static str> {
        if self.transitions == 0
            || self.transitions > 1_000_000
            || self.nouns == 0
            || self.nouns > 262_144
            || self.evaluations == 0
            || self.evaluations > 262_144
            || self.frames == 0
            || self.frames > 65_536
        {
            return Err("census caps");
        }
        Ok(())
    }
}

#[derive(Default, Serialize)]
struct Retention {
    capacity: usize,
    retained: usize,
    hits: u64,
    refused: u64,
    distinct_is_lower_bound: bool,
}

struct Retained<K, V> {
    entries: BTreeMap<K, V>,
    stats: Retention,
}

impl<K: Ord, V: PartialEq> Retained<K, V> {
    fn new(capacity: usize) -> Self {
        Self {
            entries: BTreeMap::new(),
            stats: Retention {
                capacity,
                ..Retention::default()
            },
        }
    }

    fn check(&self, key: &K, value: &V) -> Result<(), &'static str> {
        if self.entries.get(key).is_some_and(|old| old != value) {
            Err("inconsistent retained observation")
        } else {
            Ok(())
        }
    }

    fn record(&mut self, key: K, value: V) {
        if self.entries.contains_key(&key) {
            self.stats.hits += 1;
        } else if self.entries.len() < self.stats.capacity {
            self.entries.insert(key, value);
            self.stats.retained += 1;
        } else {
            self.stats.refused += 1;
            self.stats.distinct_is_lower_bound = true;
        }
    }
}

type Key = (Particle, Particle);

#[derive(Clone, Copy)]
struct Invocation {
    key: Key,
    budget: u64,
}

#[derive(Default, Serialize)]
struct Counts {
    events: u64,
    encoded_bytes: u64,
    initial_node_events: u64,
    fresh_node_events: u64,
    transitions: u64,
    before_actions: [u64; 4],
    after_actions: [u64; 4],
    entered_opcodes: [u64; 18],
    unclassified_enters: u64,
    popped_phases: [u64; 6],
    pushed_phases: [u64; 6],
    peak_active_invocations: usize,
    successful_evaluations: u64,
    failed_evaluations: u64,
    completed_events: u64,
}

struct Census<'a> {
    caps: Caps,
    stop: &'a Cell<bool>,
    counts: Counts,
    nouns: Retained<Particle, Node>,
    entered: Retained<Key, ()>,
    completed: Retained<Key, (Particle, u64)>,
    stack: Vec<Invocation>,
    initial_left: u32,
    begun: bool,
}

fn action_kind(action: LogicalAction) -> usize {
    match action {
        LogicalAction::Enter { .. } => 0,
        LogicalAction::Return { .. } => 1,
        LogicalAction::Halt { .. } => 2,
        LogicalAction::Error(_) => 3,
    }
}

fn phase(frame: LiveFrame) -> usize {
    match frame {
        LiveFrame::Unary { .. } => 0,
        LiveFrame::BinaryLeft { .. } => 1,
        LiveFrame::BinaryRight { .. } => 2,
        LiveFrame::BranchTest { .. } => 3,
        LiveFrame::BranchChosen(_) => 4,
        LiveFrame::Compose => 5,
    }
}

impl<'a> Census<'a> {
    fn new(caps: Caps, stop: &'a Cell<bool>) -> Result<Self, &'static str> {
        caps.validate()?;
        let mut stack = Vec::new();
        stack
            .try_reserve_exact(caps.frames)
            .map_err(|_| "census stack allocation")?;
        Ok(Self {
            caps,
            stop,
            counts: Counts::default(),
            nouns: Retained::new(caps.nouns),
            entered: Retained::new(caps.evaluations),
            completed: Retained::new(caps.evaluations),
            stack,
            initial_left: 0,
            begun: false,
        })
    }

    fn opcode(&self, formula: Particle) -> Option<usize> {
        let NodeValue::Pair { left, .. } = self.nouns.entries.get(&formula)?.value else {
            return None;
        };
        let NodeValue::Atom(tag) = self.nouns.entries.get(&left)?.value else {
            return None;
        };
        usize::try_from(tag).ok().filter(|&tag| tag < 18)
    }

    fn summary(&self) -> serde_json::Value {
        serde_json::json!({
            "caps": self.caps, "counts": self.counts,
            "noun_particles": self.nouns.stats, "entered_evaluation_keys": self.entered.stats,
            "completed_evaluation_keys": self.completed.stats, "open_invocations": self.stack.len(),
            "prefix_ceiling_reached": self.stop.get(),
            "retained_noun_payload_bytes": self.nouns.entries.len() * std::mem::size_of::<(Particle, Node)>(),
            "retained_enter_key_payload_bytes": self.entered.entries.len() * std::mem::size_of::<Key>(),
            "retained_completed_payload_bytes": self.completed.entries.len() * std::mem::size_of::<(Key, (Particle, u64))>(),
            "reserved_invocation_bytes": self.stack.capacity() * std::mem::size_of::<Invocation>(),
            "storage_note": "Payload sizes exclude BTreeMap and allocator overhead; no event stream retained",
            "action_order": ["Enter", "Return", "Halt", "Error"],
            "phase_order": ["Unary", "BinaryLeft", "BinaryRight", "BranchTest", "BranchChosen", "Compose"]
        })
    }
}

impl Observer for Census<'_> {
    type Error = &'static str;

    fn record(&mut self, event: Event) -> Result<(), Self::Error> {
        // Validate fallible conditions before publishing any part of this event.
        match event {
            Event::Begin { .. } if self.begun => return Err("duplicate Begin"),
            Event::Begin { .. } => (),
            _ if !self.begun => return Err("missing Begin"),
            Event::Node(node) => self.nouns.check(&node.particle, &node)?,
            Event::Transition(t) => {
                if self.initial_left != 0 || t.sequence != self.counts.transitions {
                    return Err("transition order");
                }
                match t.before {
                    LogicalAction::Enter { .. } => {
                        if self.stack.len() >= self.caps.frames
                            || self.stack.len() != t.depth_before as usize
                        {
                            return Err("invocation Enter depth");
                        }
                    }
                    _ => {
                        if self.stack.len() != t.depth_before as usize + 1 {
                            return Err("invocation Return depth");
                        }
                        if let LogicalAction::Return { value, remaining } = t.before {
                            let call = self.stack.last().ok_or("empty invocation stack")?;
                            let cost = call
                                .budget
                                .checked_sub(remaining)
                                .ok_or("negative observed cost")?;
                            self.completed.check(&call.key, &(value, cost))?;
                        }
                    }
                }
            }
            Event::Completed { steps, .. } => {
                if !self.stack.is_empty()
                    || steps != self.counts.transitions
                    || self.counts.completed_events != 0
                {
                    return Err("Completed boundary");
                }
            }
        }
        self.counts.events += 1;
        self.counts.encoded_bytes += event.encode().as_bytes().len() as u64;
        match event {
            Event::Begin { initial_nodes, .. } => {
                self.begun = true;
                self.initial_left = initial_nodes;
            }
            Event::Node(node) => {
                if self.initial_left != 0 {
                    self.initial_left -= 1;
                    self.counts.initial_node_events += 1;
                } else {
                    self.counts.fresh_node_events += 1;
                }
                self.nouns.record(node.particle, node);
            }
            Event::Transition(t) => {
                self.counts.transitions += 1;
                self.counts.before_actions[action_kind(t.before)] += 1;
                self.counts.after_actions[action_kind(t.after)] += 1;
                if let Some(frame) = t.popped {
                    self.counts.popped_phases[phase(frame)] += 1;
                }
                if let Some(frame) = t.pushed {
                    self.counts.pushed_phases[phase(frame)] += 1;
                }
                match t.before {
                    LogicalAction::Enter {
                        object,
                        formula,
                        budget,
                    } => {
                        self.entered.record((object, formula), ());
                        if let Some(tag) = self.opcode(formula) {
                            self.counts.entered_opcodes[tag] += 1;
                        } else {
                            self.counts.unclassified_enters += 1;
                        }
                        self.stack.push(Invocation {
                            key: (object, formula),
                            budget,
                        });
                        self.counts.peak_active_invocations =
                            self.counts.peak_active_invocations.max(self.stack.len());
                    }
                    action => {
                        let call = self.stack.pop().expect("checked invocation");
                        if let LogicalAction::Return { value, remaining } = action {
                            self.counts.successful_evaluations += 1;
                            self.completed
                                .record(call.key, (value, call.budget - remaining));
                        } else {
                            self.counts.failed_evaluations += 1;
                        }
                    }
                }
                if self.counts.transitions == self.caps.transitions {
                    self.stop.set(true);
                }
            }
            Event::Completed { .. } => self.counts.completed_events += 1,
        }
        Ok(())
    }
}
