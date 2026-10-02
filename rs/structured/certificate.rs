//! Bounded transport for public native evaluation certificates.
//!
//! Transport completion authenticates framing only. Production semantic
//! admission must separately verify the evaluation and bind all public roots.
pub mod transport;
