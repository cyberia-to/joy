use clap::Args;
use joy_rs::structured::{CompilerCaps, RunLimits};

#[derive(Args)]
pub struct LimitArgs {
    #[arg(long, default_value_t = 1_000_000)]
    pub budget: u64,
    #[arg(long, default_value_t = 196_608)]
    pub arena_nodes: u32,
    #[arg(long, default_value_t = 16_384)]
    pub frames: u32,
    #[arg(long, default_value_t = 16_777_216)]
    pub artifact_bytes: usize,
    #[arg(long, default_value_t = 196_608)]
    pub artifact_nodes: u32,
    #[arg(long, default_value_t = 4096)]
    pub artifact_depth: u32,
    /// Cooperative deadline inside the execution worker
    #[arg(long, default_value_t = 30_000)]
    pub time_ms: u64,
    #[arg(long, default_value_t = 4_194_304)]
    pub source_bytes: u32,
    #[arg(long, default_value_t = 4096)]
    pub modules: u32,
    #[arg(long, default_value_t = 1024)]
    pub diagnostics: u32,
    #[arg(long, default_value_t = 65_536)]
    pub sequence_length: u32,
    #[arg(long, default_value_t = 1_000_000)]
    pub validation_visits: u32,
}

impl LimitArgs {
    pub fn values(&self) -> RunLimits {
        RunLimits {
            budget: self.budget,
            arena_nodes: self.arena_nodes,
            frames: self.frames,
            artifact_bytes: self.artifact_bytes,
            artifact_nodes: self.artifact_nodes,
            artifact_depth: self.artifact_depth,
            time_ms: self.time_ms,
            compiler: CompilerCaps {
                source_bytes: self.source_bytes,
                modules: self.modules,
                diagnostics: self.diagnostics,
                sequence_length: self.sequence_length,
                validation_visits: self.validation_visits,
            },
        }
    }
}
