//! Evidence-based, read-only Linux system diagnostics.
//!
//! No network probes, privilege escalation, service modifications, or telemetry.

pub mod brand;
pub mod cli;
pub mod collect;
pub mod demo;
pub mod engine;
pub mod gui;
pub mod health;
pub mod model;
pub mod nagios;
pub mod nagios_graph;
pub mod progress;
pub mod render;
pub mod time;
pub mod tui;

pub use cli::{Cli, Command, OutputFormat};
pub use engine::analyze;
pub use model::{Focus, Report};
