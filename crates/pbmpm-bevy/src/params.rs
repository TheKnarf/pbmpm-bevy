use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderMode {
    #[default]
    Standard = 0,
    Compression = 1,
    Velocity = 2,
}

impl RenderMode {
    pub fn to_gpu(self) -> f32 {
        self as u32 as f32
    }
}

/// Tunable parameters that drive a single PB-MPM simulation.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct SimParams {
    pub sim_res_divisor: u32,
    pub particles_per_cell_axis: u32,
    pub sim_rate: u32,
    pub use_grid_volume_for_liquid: bool,
    pub fixed_point_multiplier_exponent: u32,
    pub gravity_strength: f32,
    pub liquid_viscosity: f32,
    pub iteration_count: u32,
    pub elasticity_ratio: f32,
    pub liquid_relaxation: f32,
    pub elastic_relaxation: f32,
    pub friction_angle: f32,
    pub plasticity: f32,
    pub border_friction: f32,
    pub render_mode: RenderMode,
}

impl Default for SimParams {
    fn default() -> Self {
        Self {
            sim_res_divisor: 8,
            particles_per_cell_axis: 2,
            sim_rate: 240,
            use_grid_volume_for_liquid: true,
            fixed_point_multiplier_exponent: 7,
            gravity_strength: 2.5,
            liquid_viscosity: 0.01,
            iteration_count: 5,
            elasticity_ratio: 1.0,
            liquid_relaxation: 1.5,
            elastic_relaxation: 1.5,
            friction_angle: 30.0,
            plasticity: 0.9,
            border_friction: 0.5,
            render_mode: RenderMode::Standard,
        }
    }
}

/// Per-frame mutable state owned by the simulation.
#[derive(Resource, Debug, Clone, Default)]
pub struct SimState {
    pub is_paused: bool,
    pub substep_index: u32,
    pub scene_index: usize,
}
