//! GPU fluid and granular simulation as a Bevy plugin.
//!
//! Port of EA SEED's PB-MPM (Position Based Material Point Method). The
//! crate is **input- and UI-agnostic**: it owns the GPU compute pipeline,
//! the render graph node, and the simulation state, and exposes a small
//! resource/component surface that the host app drives each frame.
//!
//! # Quick start
//!
//! ```no_run
//! use bevy::prelude::*;
//! use pbmpm_bevy::*;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(PbmpmPlugin)
//!         .add_systems(Startup, setup)
//!         .add_systems(Update, drive_sim)
//!         .run();
//! }
//!
//! fn setup(mut commands: Commands) {
//!     commands.spawn(Camera2d);
//!
//!     // Spawn an emitter shape near the top of the viewport.
//!     // Positions are in Bevy 2D world space — same convention as a Sprite.
//!     commands.spawn(SimShapeData {
//!         position: Vec2::new(0.0, 200.0),
//!         radius: 40.0,
//!         shape_type: ShapeType::Circle,
//!         function: ShapeFunction::Emit,
//!         emit_material: MaterialType::Liquid,
//!         emission_rate: 2.5,
//!         ..default()
//!     });
//! }
//!
//! fn drive_sim(
//!     windows: Query<&Window>,
//!     mut viewport: ResMut<SimViewport>,
//!     mut interaction: ResMut<SimInteraction>,
//! ) {
//!     // The simulation needs viewport pixels every frame; the host
//!     // computes them however it likes.
//!     let Ok(window) = windows.single() else { return };
//!     viewport.resolution = Vec2::new(window.width(), window.height());
//!
//!     // Drive interaction from any input source — keyboard, gamepad,
//!     // AI, scripted motion. Position/velocity/radius are in Bevy world
//!     // space (origin at viewport center, Y up, pixels at zoom 1).
//!     interaction.active = false; // set true to apply force
//!     interaction.position = Vec2::ZERO;
//!     interaction.radius = 100.0;
//!     interaction.mode = InteractionMode::Grab;
//! }
//! ```
//!
//! # Host responsibilities
//!
//! Each frame the host must update:
//! - [`SimViewport::resolution`] — the pixel dimensions of the area the
//!   simulation should fill. The grid resolution is derived from this.
//! - [`SimInteraction`] — the current force/grab being applied (or
//!   `active = false` for none).
//!
//! The host can also:
//! - Spawn / despawn [`SimShapeData`] entities to add emitters, colliders,
//!   drains, and initial-emit volumes.
//! - Mutate [`SimParams`] to change physics tunables at runtime.
//! - Trigger [`ResetSimulation`] to clear particles.
//! - Read [`ParticleCount`] for HUD / debug display.
//!
//! # Limitations
//!
//! Only one simulation per Bevy `World` is supported — all state lives in
//! singleton resources and a single render graph node.

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
