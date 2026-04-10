//! PB-MPM (Position Based Material Point Method) — Bevy plugin.
//!
//! Headless simulation core. Add [`PbmpmPlugin`] to a Bevy `App` to enable
//! the GPU compute pipeline. The plugin owns the render graph node, GPU
//! buffers, and the per-frame data extraction. Editor UI and scene loading
//! live in the consuming app.

pub mod gpu;
pub mod interface;
pub mod params;
pub mod shape;
pub mod simulation;
pub mod time_regulation;

pub use gpu::*;
pub use interface::*;
pub use params::*;
pub use shape::*;
pub use simulation::PbmpmPlugin;
pub use time_regulation::TimeRegulation;
