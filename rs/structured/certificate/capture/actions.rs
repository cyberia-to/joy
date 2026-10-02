use super::*;

impl<W: Write> Capture<W> {
    pub(super) fn transition(&mut self, t: Transition) -> Result<(), String> {
        if self.stats.transitions >= self.session.steps_limit() {
            return Err("observed transition allowance".into());
        }
        if self.snapshot_left != 0
            || t.sequence != self.stats.transitions
            || self.previous != Some(t.before)
            || self.fresh != t.fresh_nodes
        {
            return Err("certificate transition order/noun group".into());
        }
        let returning = matches!(t.before, LogicalAction::Return { .. });
        if t.popped.is_some() != (returning && t.depth_before != 0)
            || t.depth_before
                .checked_sub(u32::from(t.popped.is_some()))
                .and_then(|d| d.checked_add(u32::from(t.pushed.is_some())))
                != Some(t.depth_after)
        {
            return Err("certificate continuation depth".into());
        }
        match t.before {
            LogicalAction::Enter {
                object,
                formula,
                budget,
            } => {
                if self.calls.len() != t.depth_before as usize {
                    return Err("observed Enter depth".into());
                }
                self.enter((object, formula), budget)?;
            }
            LogicalAction::Return { value, remaining } => {
                if self.calls.len() != t.depth_before as usize + 1 {
                    return Err("observed Return depth".into());
                }
                self.returned(value, remaining)?;
                if self.calls.is_empty() {
                    if t.before != t.after || t.depth_after != 0 || t.pushed.is_some() {
                        return Err("observed root Return boundary".into());
                    }
                    self.root = Some((value, remaining));
                }
            }
            _ => return Err("certificate requires successful pure execution".into()),
        }
        match t.after {
            LogicalAction::Enter { .. } if self.calls.len() == t.depth_after as usize => (),
            LogicalAction::Return { .. }
                if self.calls.len() == t.depth_after as usize + 1 || self.root.is_some() =>
            {
                ()
            }
            _ => return Err("observed successor action/depth".into()),
        }
        self.fresh = 0;
        self.previous = Some(t.after);
        increment(&mut self.stats.transitions)
    }

    fn enter(&mut self, key: Key, budget: u64) -> Result<(), String> {
        if self.calls.len() >= self.context.frames as usize {
            return Err("observed invocation allowance".into());
        }
        // Even suppressed invocations must address the current arena epoch.
        let (object, formula) = (self.id(key.0)?, self.id(key.1)?);
        let kind = if self.suppression.is_some() {
            increment(&mut self.stats.suppressed_enters)?;
            CallKind::Suppressed
        } else if let Some(handle) = self.cache.get(&key).copied() {
            let (actual, summary) = self
                .session
                .cache(handle.slot)
                .ok_or("missing cached summary")?;
            if actual != handle || (summary.key().object, summary.key().formula) != key {
                return Err("cached summary generation/key".into());
            }
            self.emit(Record::Reuse(handle))?;
            self.suppression = Some(Suppression {
                depth: self.calls.len(),
                summary,
            });
            increment(&mut self.stats.reused_calls)?;
            CallKind::Reused
        } else {
            self.emit(Record::Enter { object, formula })?;
            CallKind::Checked
        };
        self.calls.push(Call { key, budget, kind });
        Ok(())
    }

    fn returned(&mut self, value: Particle, remaining: u64) -> Result<(), String> {
        let call = *self.calls.last().ok_or("empty observed invocation stack")?;
        let cost = call
            .budget
            .checked_sub(remaining)
            .ok_or("observed negative charge")?;
        let result = self.id(value)?;
        match call.kind {
            CallKind::Suppressed => {
                if self
                    .suppression
                    .as_ref()
                    .is_none_or(|s| s.depth + 1 >= self.calls.len())
                {
                    return Err("suppressed invocation boundary".into());
                }
            }
            CallKind::Reused => {
                let saved = self
                    .suppression
                    .as_ref()
                    .ok_or("missing suppression summary")?;
                if saved.depth + 1 != self.calls.len() {
                    return Err("reused invocation boundary".into());
                }
                check_summary(saved.summary, call.key, value, cost)?;
                self.suppression = None;
            }
            CallKind::Checked => self.finish(call.key, result, value, cost)?,
        }
        self.calls.pop();
        Ok(())
    }

    fn finish(&mut self, key: Key, result: u32, value: Particle, cost: u64) -> Result<(), String> {
        let slot = self.next_slot;
        let old = self.session.cache(slot);
        let record = Record::Finish {
            result,
            cache_slot: Some(slot),
        };
        let handle = self
            .session
            .apply(&record)?
            .ok_or("missing verified cache handle")?;
        let (actual, summary) = self.session.cache(slot).ok_or("missing finished summary")?;
        if handle != actual {
            return Err("finished cache handle mismatch".into());
        }
        check_summary(summary, key, value, cost)?;
        record.write(&mut self.writer)?;
        if let Some((old_handle, old_summary)) = old {
            let old_key = (old_summary.key().object, old_summary.key().formula);
            if self.cache.get(&old_key) == Some(&old_handle) {
                self.cache.remove(&old_key);
            }
        }
        if !self.cache.contains_key(&key) && self.cache.len() >= self.session.cache_slots() as usize
        {
            return Err("certificate cache dictionary allowance".into());
        }
        self.cache.insert(key, handle);
        self.next_slot = if slot + 1 == self.session.cache_slots() {
            0
        } else {
            slot + 1
        };
        Ok(())
    }
}

fn check_summary(
    summary: VerifiedSummary,
    key: Key,
    result: Particle,
    cost: u64,
) -> Result<(), String> {
    if (summary.key().object, summary.key().formula) != key
        || summary.result().particle() != result
        || summary.cost() != cost
    {
        return Err("observed result/charge disagrees with verified summary".into());
    }
    Ok(())
}
