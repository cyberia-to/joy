//! Bounded observer admission; contract: specs/structured-certificates.md.
use super::{records::Record, session::Session, Context};
use nox::sequential::observe::{
    Event, EventV2, LogicalAction, Node, NodeValue, ObserverV2, Particle, Transition,
};
use std::{collections::BTreeMap, io::Write};
use zheng::execution::disclosed::stream::{CacheHandle, VerifiedSummary};

mod actions;
#[cfg(test)]
mod tests;

type Key = (Particle, Particle);

#[derive(Clone, Copy)]
enum CallKind {
    Checked,
    Reused,
    Suppressed,
}

#[derive(Clone, Copy)]
struct Call {
    key: Key,
    budget: u64,
    kind: CallKind,
}

struct Suppression {
    depth: usize,
    summary: VerifiedSummary,
}

#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub struct CaptureStats {
    pub events: u64,
    pub transitions: u64,
    pub snapshot_nodes: u64,
    pub fresh_nodes: u64,
    /// Includes the initial Reset record; GC count is reported by nox separately.
    pub resets: u64,
    pub reused_calls: u64,
    pub suppressed_enters: u64,
    pub resets_during_reuse: u64,
}

pub(super) struct Capture<W> {
    writer: W,
    session: Session,
    context: Context,
    nouns: BTreeMap<Particle, u32>,
    cache: BTreeMap<Key, CacheHandle>,
    calls: Vec<Call>,
    suppression: Option<Suppression>,
    next_slot: u32,
    epoch: u64,
    snapshot_left: u32,
    fresh: u32,
    previous: Option<LogicalAction>,
    root: Option<(Particle, u64)>,
    completed: bool,
    poisoned: bool,
    stats: CaptureStats,
}

impl<W: Write> Capture<W> {
    pub(super) fn new(writer: W, session: Session, context: Context) -> Result<Self, String> {
        if context.frames == 0 || context.frames > 65_536 || session.cache_slots() == 0 {
            return Err("certificate capture frame/cache allowance".into());
        }
        let mut calls = Vec::new();
        calls
            .try_reserve_exact(context.frames as usize)
            .map_err(|_| "certificate capture stack allocation")?;
        Ok(Self {
            writer,
            session,
            context,
            nouns: BTreeMap::new(),
            cache: BTreeMap::new(),
            calls,
            suppression: None,
            next_slot: 0,
            epoch: 0,
            snapshot_left: 0,
            fresh: 0,
            previous: None,
            root: None,
            completed: false,
            poisoned: false,
            stats: CaptureStats::default(),
        })
    }

    pub(super) fn stats(&self) -> CaptureStats {
        self.stats
    }

    /// The caller must also check successful runtime return and admit the result.
    pub(super) fn complete(self) -> Result<(W, Session), String> {
        if self.poisoned
            || !self.completed
            || self.root.is_none()
            || !self.calls.is_empty()
            || self.suppression.is_some()
            || self.snapshot_left != 0
            || self.fresh != 0
        {
            return Err("incomplete certificate capture".into());
        }
        Ok((self.writer, self.session))
    }

    fn emit(&mut self, record: Record) -> Result<(), String> {
        self.session.apply(&record)?;
        record.write(&mut self.writer)
    }

    fn id(&self, particle: Particle) -> Result<u32, String> {
        self.nouns
            .get(&particle)
            .copied()
            .ok_or_else(|| "unknown current-epoch noun".into())
    }

    fn snapshot(&mut self, epoch: u64, nodes: u32) -> Result<(), String> {
        if self.snapshot_left != 0 || self.fresh != 0 {
            return Err("reset inside incomplete noun group".into());
        }
        self.emit(Record::Reset { epoch, nodes })?;
        self.nouns.clear();
        self.epoch = epoch;
        self.snapshot_left = nodes;
        increment(&mut self.stats.resets)
    }

    fn node(&mut self, node: Node) -> Result<(), String> {
        let record = match node.value {
            NodeValue::Atom(value) => Record::Atom(value),
            NodeValue::Pair { left, right } => Record::Pair {
                left: self.id(left)?,
                right: self.id(right)?,
            },
        };
        let id = self.session.node_count();
        if id >= self.session.noun_limit() {
            return Err("certificate noun allowance".into());
        }
        self.session.apply(&record)?;
        let checked = self.session.node(id)?;
        if checked.particle() != node.particle
            || checked.cost().value() != node.bound.value()
            || checked.cost().is_dynamic() != node.bound.is_dynamic()
        {
            return Err("observed noun metadata disagrees with checked definition".into());
        }
        record.write(&mut self.writer)?;
        self.nouns.insert(node.particle, id);
        if self.snapshot_left != 0 {
            self.snapshot_left -= 1;
            increment(&mut self.stats.snapshot_nodes)?;
        } else {
            self.fresh = self
                .fresh
                .checked_add(1)
                .ok_or("fresh noun count overflow")?;
            increment(&mut self.stats.fresh_nodes)?;
        }
        Ok(())
    }

    fn event(&mut self, event: EventV2) -> Result<(), String> {
        if let EventV2::Event(Event::Begin {
            version,
            initial,
            initial_nodes,
            max_frames,
            max_total_allocations,
            resident_limit,
            ..
        }) = event
        {
            let expected = LogicalAction::Enter {
                object: self.context.object,
                formula: self.context.formula,
                budget: self.context.budget,
            };
            if self.previous.is_some()
                || version != 2
                || initial != expected
                || max_frames != self.context.frames
                || initial_nodes > resident_limit
                || u64::from(initial_nodes) > max_total_allocations
            {
                return Err("certificate Begin binding".into());
            }
            self.snapshot(0, initial_nodes)?;
            self.previous = Some(initial);
            return Ok(());
        }
        if self.previous.is_none() || self.completed || self.root.is_some() {
            if !matches!(event, EventV2::Event(Event::Completed { .. })) {
                return Err("certificate capture event order".into());
            }
        }
        match event {
            EventV2::Event(Event::Begin { .. }) => Err("duplicate Begin".into()),
            EventV2::Event(Event::Node(node)) => self.node(node),
            EventV2::ArenaReset {
                next_sequence,
                live_nodes,
            } => {
                if next_sequence != self.stats.transitions {
                    return Err("reset transition sequence".into());
                }
                let epoch = self.epoch.checked_add(1).ok_or("noun epoch overflow")?;
                self.snapshot(epoch, live_nodes)?;
                if self.suppression.is_some() {
                    increment(&mut self.stats.resets_during_reuse)?;
                }
                Ok(())
            }
            EventV2::Event(Event::Transition(transition)) => self.transition(transition),
            EventV2::Event(Event::Completed {
                steps,
                value,
                remaining,
            }) => {
                if self.completed
                    || self.root != Some((value, remaining))
                    || self.previous != Some(LogicalAction::Return { value, remaining })
                    || steps != self.stats.transitions
                    || !self.calls.is_empty()
                    || self.suppression.is_some()
                    || self.snapshot_left != 0
                    || self.fresh != 0
                {
                    return Err("certificate Completed binding".into());
                }
                self.completed = true;
                Ok(())
            }
        }
    }
}

impl<W: Write> ObserverV2 for Capture<W> {
    type Error = String;
    fn record(&mut self, event: EventV2) -> Result<(), String> {
        if self.poisoned {
            return Err("certificate capture previously failed".into());
        }
        self.poisoned = true;
        self.event(event)?;
        increment(&mut self.stats.events)?;
        self.poisoned = false;
        Ok(())
    }
}

fn increment(value: &mut u64) -> Result<(), String> {
    *value = value.checked_add(1).ok_or("capture counter overflow")?;
    Ok(())
}
