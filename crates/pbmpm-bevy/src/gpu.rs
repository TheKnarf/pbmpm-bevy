use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct SimConstantsGpu {
    pub grid_size: [u32; 2],
    pub delta_time: f32,
    pub interaction_strength: f32,
    pub interaction_position: [f32; 2],
    pub interaction_velocity: [f32; 2],
    pub interaction_mode: f32,
    pub elasticity_ratio: f32,
    pub gravity_strength: f32,
    pub liquid_relaxation: f32,
    pub elastic_relaxation: f32,
    pub liquid_viscosity: f32,
    pub fixed_point_multiplier: u32,
    pub use_grid_volume_for_liquid: u32,
    pub particles_per_cell_axis: u32,
    pub friction_angle: f32,
    pub plasticity: f32,
    pub interaction_radius: f32,
    pub shape_count: u32,
    pub sim_frame: u32,
    pub bukkit_count: u32,
    pub bukkit_count_x: u32,
    pub bukkit_count_y: u32,
    pub iteration: u32,
    pub iteration_count: u32,
    pub border_friction: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct SimShapeGpu {
    pub position: [f32; 2],
    pub half_size: [f32; 2],
    pub radius: f32,
    pub rotation: f32,
    pub functionality: f32,
    pub shape_type: f32,
    pub emit_material: f32,
    pub emission_rate: f32,
    pub emission_speed: f32,
    pub padding: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct RenderConstantsGpu {
    pub particle_radius_timestamp: [f32; 2],
    pub canvas_size: [f32; 2],
    pub view_pos: [f32; 2],
    pub view_extent: [f32; 2],
    pub render_mode: f32,
    pub delta_time: f32,
    pub _pad0: f32,
    pub _pad1: f32,
}

// Dispatch sizes (must match WGSL constants in shaders/common.wgsl)
pub const GRID_DISPATCH_SIZE: u32 = 8;
pub const BUKKIT_SIZE: u32 = 6;
pub const MAX_PARTICLE_COUNT: u32 = 1_000_000;
pub const PARTICLE_FLOAT_COUNT: u32 = 24; // 96 bytes / 4

pub fn div_up(a: u32, b: u32) -> u32 {
    a.div_ceil(b)
}
