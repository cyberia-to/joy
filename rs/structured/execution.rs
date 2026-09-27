//! Select the explicit pure execution policy without changing guest semantics.
use super::{CompactionPolicy, RunLimits};
use nox::{sequential, Order, Outcome, Reduction};
use serde::Serialize;
use std::time::{Duration, Instant};

#[derive(Debug, Serialize)]
pub struct CompactionReport {
    pub profile: &'static str,
    pub resident_limit: u32,
    pub allocation_limit: u64,
    pub collection_work_limit: u64,
    pub pinned_nodes: u32,
    pub resident_nodes: u32,
    pub peak_resident_nodes: u32,
    pub total_allocations: u64,
    pub reclaimed_nodes: u64,
    pub collections: u64,
    pub collection_work: u64,
    pub scratch_bytes: usize,
    pub evaluator_checkpoints: u64,
    pub collection_checkpoints: u64,
}

impl CompactionReport {
    fn new(
        policy: CompactionPolicy,
        allocation_limit: u32,
        stats: sequential::CompactionStats,
    ) -> Self {
        Self {
            profile: "bounded-compaction-v1",
            resident_limit: policy.resident_nodes.min(allocation_limit),
            allocation_limit: u64::from(allocation_limit),
            collection_work_limit: policy.collection_work,
            pinned_nodes: stats.pinned_nodes,
            resident_nodes: stats.resident_nodes,
            peak_resident_nodes: stats.peak_resident_nodes,
            total_allocations: stats.total_allocations,
            reclaimed_nodes: stats.reclaimed_nodes,
            collections: stats.collections,
            collection_work: stats.collection_work,
            scratch_bytes: stats.scratch_bytes,
            evaluator_checkpoints: stats.evaluator_checkpoints,
            collection_checkpoints: stats.collection_checkpoints,
        }
    }
}

pub(super) struct Execution {
    pub result: Order,
    pub remaining: u64,
    pub peak_frames: u32,
    pub allocated_nodes: u32,
    pub compaction: Option<CompactionReport>,
}

pub(super) struct Request {
    pub input: Order,
    pub formula: Order,
    pub budget: u64,
    pub allocations: u32,
    pub frames: sequential::Limits,
}

pub(super) fn run<const N: usize>(
    ar: &mut Reduction<N>,
    request: Request,
    limits: RunLimits,
    started: Instant,
) -> Result<Execution, String> {
    let mut cancelled = || started.elapsed() >= Duration::from_millis(limits.time_ms);
    let (outcome, peak_frames, allocated_nodes, compaction) = match limits.compaction {
        Some(policy) => {
            let execution = sequential::reduce_compacting_cached_controlled(
                ar,
                request.input,
                request.formula,
                request.budget,
                sequential::CompactionLimits {
                    max_frames: request.frames.max_frames,
                    max_total_allocations: u64::from(request.allocations),
                    max_collection_work: policy.collection_work,
                },
                &mut cancelled,
            )
            .map_err(|e| format!("execution resource/profile: {e:?}"))?;
            let allocated = u32::try_from(execution.stats.total_allocations)
                .map_err(|_| "execution allocation counter overflow")?;
            let report = CompactionReport::new(policy, request.allocations, execution.stats);
            (
                execution.outcome,
                execution.peak_frames,
                allocated,
                Some(report),
            )
        }
        None => {
            let execution = sequential::reduce_cached_controlled(
                ar,
                request.input,
                request.formula,
                request.budget,
                request.frames,
                &mut cancelled,
            )
            .map_err(|e| format!("execution resource/profile: {e:?}"))?;
            (execution.outcome, execution.peak_frames, ar.count(), None)
        }
    };
    let (result, remaining) = match outcome {
        Outcome::Ok(result, remaining) => (result, remaining),
        Outcome::Halt(_) => return Err(failed("execution budget exhausted".into(), &compaction)),
        Outcome::Error(error) => {
            return Err(failed(format!("execution failed: {error:?}"), &compaction))
        }
    };
    Ok(Execution {
        result,
        remaining,
        peak_frames,
        allocated_nodes,
        compaction,
    })
}

fn failed(error: String, compaction: &Option<CompactionReport>) -> String {
    match compaction {
        Some(stats) => format!("{error}; compaction: {stats:?}"),
        None => error,
    }
}
