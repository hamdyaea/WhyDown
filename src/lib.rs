//! Evidence-based, read-only Linux system diagnostics.
//!
//! No network probes, privilege escalation, service modifications, or telemetry.

pub mod cli;
pub mod collect;
pub mod demo;
pub mod engine;
pub mod model;
pub mod render;
pub mod time;

pub use cli::{Cli, Command, OutputFormat};
pub use engine::analyze;
pub use model::{Focus, Report};
