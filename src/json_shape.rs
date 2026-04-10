use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use pbmpm_bevy::{MaterialType, ShapeFunction, ShapeType, SimShapeData};

/// Shape representation as it appears in scene JSON files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimShape {
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
            StringOrNumber::String(s) => s.parse().unwrap_or_else(|e| {
                warn!("Failed to parse scene value {s:?} as float: {e}");
                0.0
            }),
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

impl From<&SimShape> for SimShapeData {
    fn from(s: &SimShape) -> Self {
        Self {
            position: s.position.as_vec2(),
            half_size: s.half_size.as_vec2(),
            rotation: s.rotation,
            shape_type: ShapeType::from_u32(s.shape.as_u32()),
            function: ShapeFunction::from_u32(s.function.as_u32()),
            emit_material: MaterialType::from_u32(s.emit_material.as_u32()),
            emission_rate: s.emission_rate.as_f32(),
            emission_speed: s.emission_speed.as_f32(),
            radius: s.radius,
        }
    }
}

impl From<&SimShapeData> for SimShape {
    fn from(d: &SimShapeData) -> Self {
        Self {
            position: Vec2Json {
                x: d.position.x as f64,
                y: d.position.y as f64,
            },
            half_size: Vec2Json {
                x: d.half_size.x as f64,
                y: d.half_size.y as f64,
            },
            rotation: d.rotation,
            // Enum variants cast to i64 via their explicit discriminants.
            shape: StringOrNumber::Int(d.shape_type as i64),
            function: StringOrNumber::Int(d.function as i64),
            emit_material: StringOrNumber::Int(d.emit_material as i64),
            emission_rate: StringOrNumber::Float(d.emission_rate as f64),
            emission_speed: StringOrNumber::Float(d.emission_speed as f64),
            radius: d.radius,
        }
    }
}
