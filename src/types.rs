#[allow(dead_code)]
use bevy::prelude::*;
use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

// --- Enums ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MaterialType {
    #[default]
    Liquid = 0,
    Elastic = 1,
    Sand = 2,
    Visco = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShapeType {
    #[default]
    Box = 0,
    Circle = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShapeFunction {
    #[default]
    Emit = 0,
    Collider = 1,
    Drain = 2,
    InitialEmit = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MouseFunction {
    Push = 0,
    #[default]
    Grab = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderMode {
    #[default]
    Standard = 0,
    Compression = 1,
    Velocity = 2,
}

// --- Simulation Shape ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimShape {
    pub id: String,
    pub position: Vec2Json,
    #[serde(default, rename = "halfSize")]
    pub half_size: Vec2Json,
    #[serde(default)]
    pub rotation: f32,
    #[serde(default)]
    pub shape: StringOrNumber,
    #[serde(default, rename = "function")]
    pub function: StringOrNumber,
    #[serde(default, rename = "emitMaterial")]
    pub emit_material: StringOrNumber,
    #[serde(default, rename = "emissionRate")]
    pub emission_rate: StringOrNumber,
    #[serde(default, rename = "emissionSpeed")]
    pub emission_speed: StringOrNumber,
    #[serde(default)]
    pub radius: f32,
}

// The original JSON stores some values as strings or numbers inconsistently
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(untagged)]
pub enum StringOrNumber {
    String(String),
    Float(f64),
    Int(i64),
    #[default]
    None,
}

impl StringOrNumber {
    pub fn as_f32(&self) -> f32 {
        match self {
            StringOrNumber::String(s) => s.parse().unwrap_or(0.0),
            StringOrNumber::Float(f) => *f as f32,
            StringOrNumber::Int(i) => *i as f32,
            StringOrNumber::None => 0.0,
        }
    }

    pub fn as_u32(&self) -> u32 {
        self.as_f32() as u32
    }
}

/// Vec2 that deserializes from JSON where x/y can be numbers or strings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Vec2Json {
    #[serde(deserialize_with = "deserialize_f64_or_string")]
    pub x: f64,
    #[serde(deserialize_with = "deserialize_f64_or_string")]
    pub y: f64,
}

fn deserialize_f64_or_string<'de, D: serde::Deserializer<'de>>(d: D) -> Result<f64, D::Error> {
    use serde::de;
    struct F64OrString;
    impl<'de> de::Visitor<'de> for F64OrString {
        type Value = f64;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("a number or string")
        }
        fn visit_f64<E: de::Error>(self, v: f64) -> Result<f64, E> {
            Ok(v)
        }
        fn visit_i64<E: de::Error>(self, v: i64) -> Result<f64, E> {
            Ok(v as f64)
        }
        fn visit_u64<E: de::Error>(self, v: u64) -> Result<f64, E> {
            Ok(v as f64)
        }
        fn visit_str<E: de::Error>(self, v: &str) -> Result<f64, E> {
            v.parse().map_err(de::Error::custom)
        }
    }
    d.deserialize_any(F64OrString)
}

impl Vec2Json {
    pub fn as_vec2(&self) -> Vec2 {
        Vec2::new(self.x as f32, self.y as f32)
    }
}

// --- GPU Structs ---

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct SimConstantsGpu {
    pub grid_size: [u32; 2],
    pub delta_time: f32,
    pub mouse_activation: f32,
    pub mouse_position: [f32; 2],
    pub mouse_velocity: [f32; 2],
    pub mouse_function: f32,
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
    pub mouse_radius: f32,
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

// --- Simulation Parameters Resource ---

#[derive(Resource, Debug, Clone, PartialEq)]
pub struct SimParams {
    pub sim_res_divisor: u32,
    pub particles_per_cell_axis: u32,
    pub sim_rate: u32,
    pub use_grid_volume_for_liquid: bool,
    pub fixed_point_multiplier_exponent: u32,
    pub gravity_strength: f32,
    pub liquid_viscosity: f32,
    pub mouse_function: MouseFunction,
    pub iteration_count: u32,
    pub elasticity_ratio: f32,
    pub liquid_relaxation: f32,
    pub elastic_relaxation: f32,
    pub friction_angle: f32,
    pub plasticity: f32,
    pub border_friction: f32,
    pub render_mode: RenderMode,
    pub mouse_radius: f32,
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
            mouse_function: MouseFunction::Grab,
            iteration_count: 5,
            elasticity_ratio: 1.0,
            liquid_relaxation: 1.5,
            elastic_relaxation: 1.5,
            friction_angle: 30.0,
            plasticity: 0.9,
            border_friction: 0.5,
            render_mode: RenderMode::Standard,
            mouse_radius: 100.0,
        }
    }
}

// --- Simulation State Resource ---

#[derive(Resource, Debug, Clone)]
pub struct SimState {
    pub shapes: Vec<SimShape>,
    pub do_reset: bool,
    pub is_paused: bool,
    pub substep_index: u32,
    pub grid_size: [u32; 2],
    pub resolution: [f32; 2],
    pub scene_index: usize,
}

impl Default for SimState {
    fn default() -> Self {
        Self {
            shapes: Vec::new(),
            do_reset: true,
            is_paused: false,
            substep_index: 0,
            grid_size: [128, 72],
            resolution: [1024.0, 576.0],
            scene_index: 0,
        }
    }
}

// --- Input State ---

#[derive(Resource, Debug, Clone, Default)]
pub struct InputState {
    pub mouse_down: bool,
    pub mouse_position: Vec2,
    pub mouse_prev_position: Vec2,
}

// Dispatch sizes
pub const PARTICLE_DISPATCH_SIZE: u32 = 64;
pub const GRID_DISPATCH_SIZE: u32 = 8;
pub const BUKKIT_SIZE: u32 = 6;
pub const MAX_PARTICLE_COUNT: u32 = 1_000_000;
pub const PARTICLE_FLOAT_COUNT: u32 = 24; // 96 bytes / 4
pub const GUARDIAN_SIZE: u32 = 3;

pub fn div_up(a: u32, b: u32) -> u32 {
    a.div_ceil(b)
}
