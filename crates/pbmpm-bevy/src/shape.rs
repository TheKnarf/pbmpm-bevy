use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MaterialType {
    #[default]
    Liquid = 0,
    Elastic = 1,
    Sand = 2,
    Visco = 3,
}

impl MaterialType {
    pub fn from_u32(v: u32) -> Self {
        match v {
            1 => Self::Elastic,
            2 => Self::Sand,
            3 => Self::Visco,
            _ => Self::Liquid,
        }
    }
    pub fn cycle_next(self) -> Self {
        match self {
            Self::Liquid => Self::Elastic,
            Self::Elastic => Self::Sand,
            Self::Sand => Self::Visco,
            Self::Visco => Self::Liquid,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Liquid => "Liquid",
            Self::Elastic => "Elastic",
            Self::Sand => "Sand",
            Self::Visco => "Visco",
        }
    }
    pub fn to_gpu(self) -> f32 {
        self as u32 as f32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShapeType {
    #[default]
    Box = 0,
    Circle = 1,
}

impl ShapeType {
    pub fn from_u32(v: u32) -> Self {
        match v {
            1 => Self::Circle,
            _ => Self::Box,
        }
    }
    pub fn cycle_next(self) -> Self {
        match self {
            Self::Box => Self::Circle,
            Self::Circle => Self::Box,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Box => "Box",
            Self::Circle => "Circle",
        }
    }
    pub fn is_circle(self) -> bool {
        matches!(self, Self::Circle)
    }
    pub fn to_gpu(self) -> f32 {
        self as u32 as f32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShapeFunction {
    #[default]
    Emit = 0,
    Collider = 1,
    Drain = 2,
    InitialEmit = 3,
}

impl ShapeFunction {
    pub fn from_u32(v: u32) -> Self {
        match v {
            1 => Self::Collider,
            2 => Self::Drain,
            3 => Self::InitialEmit,
            _ => Self::Emit,
        }
    }
    pub fn cycle_next(self) -> Self {
        match self {
            Self::Emit => Self::Collider,
            Self::Collider => Self::Drain,
            Self::Drain => Self::InitialEmit,
            Self::InitialEmit => Self::Emit,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Emit => "Emit",
            Self::Collider => "Collider",
            Self::Drain => "Drain",
            Self::InitialEmit => "InitEmit",
        }
    }
    pub fn to_gpu(self) -> f32 {
        self as u32 as f32
    }
}

/// A simulation shape attached to an entity. Can act as a particle source,
/// collider, drain, or initial emitter depending on `function`.
#[derive(Component, Debug, Clone)]
pub struct SimShapeData {
    pub position: Vec2,
    pub half_size: Vec2,
    pub rotation: f32,
    pub shape_type: ShapeType,
    pub function: ShapeFunction,
    pub emit_material: MaterialType,
    pub emission_rate: f32,
    pub emission_speed: f32,
    pub radius: f32,
}
