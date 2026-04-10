//! The host↔simulation contact surface: events and resources the consuming
//! Bevy app reads or writes to drive the simulation.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use bevy::prelude::*;

/// Trigger this event to clear all particles and reset the substep counter.
///
/// The reset takes effect on the next frame's extraction; in-flight substeps
/// for the current frame are not cancelled.
#[derive(Event)]
pub struct ResetSimulation;

/// What kind of force the host is applying to the fluid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InteractionMode {
    /// Push particles radially away from `position`.
    Push = 0,
    /// Drag particles inside the radius along `velocity`.
    #[default]
    Grab = 1,
}

impl InteractionMode {
    pub fn to_gpu(self) -> f32 {
        self as u32 as f32
    }
}

/// Per-frame interaction with the fluid. The host populates this each
/// frame from whatever input source it likes (mouse, gamepad, AI, scripted
/// motion). The simulation reads it and applies a force.
///
/// `position`, `velocity`, and `radius` are in **Bevy 2D world space**
/// (origin at viewport center, Y up, units = pixels at zoom 1). The same
/// space you'd use for a Bevy `Sprite`.
#[derive(Resource, Default)]
pub struct SimInteraction {
    pub active: bool,
    pub mode: InteractionMode,
    pub position: Vec2,
    pub velocity: Vec2,
    pub radius: f32,
}

/// Host-provided viewport size. The simulation uses this to determine grid
/// resolution (combined with `SimParams::sim_res_divisor`) and to size its
/// render output. The host updates this each frame from its window.
#[derive(Resource, Default)]
pub struct SimViewport {
    pub resolution: Vec2,
}

/// Live particle count, updated by GPU readback (~once per second).
/// Cheap to clone — internally an `Arc`.
#[derive(Resource, Clone)]
pub struct ParticleCount(pub(crate) Arc<AtomicU32>);

impl Default for ParticleCount {
    fn default() -> Self {
        Self(Arc::new(AtomicU32::new(0)))
    }
}

impl ParticleCount {
    /// Most recent particle count read back from the GPU.
    pub fn get(&self) -> u32 {
        self.0.load(Ordering::Relaxed)
    }
}
