use super::{reader::Result, RunLimits};
use nox::artifact;
use serde::{Deserialize, Serialize};

/// Independent host admission ceilings for the compiler schema.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct CompilerCaps {
    pub source_bytes: u32,
    pub modules: u32,
    pub diagnostics: u32,
    pub sequence_length: u32,
    pub validation_visits: u32,
}

impl Default for CompilerCaps {
    fn default() -> Self {
        Self {
            source_bytes: 4 << 20,
            modules: 4096,
            diagnostics: 1024,
            sequence_length: 65_536,
            validation_visits: 1_000_000,
        }
    }
}

impl CompilerCaps {
    pub(super) fn validate(self) -> Result<()> {
        for (name, value, max) in [
            ("source_bytes", self.source_bytes, 16 << 20),
            ("modules", self.modules, 65_536),
            ("diagnostics", self.diagnostics, 65_536),
            ("sequence_length", self.sequence_length, 1 << 20),
            ("validation_visits", self.validation_visits, 16 << 20),
        ] {
            if value == 0 || value > max {
                return Err(format!("limit {name} must be in 1..={max}"));
            }
        }
        Ok(())
    }
}

/// Exact admitted LIM1 request. Integer conversion follows range checking.
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JobLimits {
    pub source_bytes: u32,
    pub modules: u32,
    pub diagnostics: u32,
    pub sequence_length: u32,
    pub validation_visits: u32,
    pub artifact_bytes: u32,
    pub artifact_nodes: u32,
    pub artifact_depth: u32,
    pub reductions: u64,
    pub arena_nodes: u32,
    pub evaluator_frames: u32,
}

impl JobLimits {
    pub(super) fn values(self) -> [u64; 11] {
        [
            self.source_bytes as u64,
            self.modules as u64,
            self.diagnostics as u64,
            self.sequence_length as u64,
            self.validation_visits as u64,
            self.artifact_bytes as u64,
            self.artifact_nodes as u64,
            self.artifact_depth as u64,
            self.reductions,
            self.arena_nodes as u64,
            self.evaluator_frames as u64,
        ]
    }

    pub(super) fn admit(values: [u64; 11], host: RunLimits) -> Result<Self> {
        let c = host.compiler;
        let caps = [
            c.source_bytes as u64,
            c.modules as u64,
            c.diagnostics as u64,
            c.sequence_length as u64,
            c.validation_visits as u64,
            host.artifact_bytes as u64,
            host.artifact_nodes as u64,
            host.artifact_depth as u64,
            host.budget,
            host.arena_nodes as u64,
            host.frames as u64,
        ];
        for (i, value) in values.iter().enumerate() {
            if *value == 0 || *value > caps[i] || (i != 8 && *value > u32::MAX as u64) {
                return Err(format!("unsupported job limit at LIM1 field {i}"));
            }
        }
        Ok(Self {
            source_bytes: values[0] as u32,
            modules: values[1] as u32,
            diagnostics: values[2] as u32,
            sequence_length: values[3] as u32,
            validation_visits: values[4] as u32,
            artifact_bytes: values[5] as u32,
            artifact_nodes: values[6] as u32,
            artifact_depth: values[7] as u32,
            reductions: values[8],
            arena_nodes: values[9] as u32,
            evaluator_frames: values[10] as u32,
        })
    }

    pub(super) fn transport(self) -> artifact::Limits {
        artifact::Limits {
            max_bytes: self.artifact_bytes as usize,
            max_nodes: self.artifact_nodes,
            max_depth: self.artifact_depth,
        }
    }
}
