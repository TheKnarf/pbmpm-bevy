use crate::types::*;
use bevy::prelude::*;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct SceneManifestEntry {
    pub scene: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SceneFile {
    #[allow(dead_code)]
    pub version: u32,
    pub resolution: [f64; 2],
    #[serde(default)]
    pub settings: Vec<SceneSetting>,
    pub shapes: Vec<SimShape>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct SceneSetting {
    pub name: String,
    pub value: serde_json::Value,
    #[serde(rename = "type")]
    pub setting_type: String,
}

pub fn load_manifest() -> Vec<SceneManifestEntry> {
    let manifest_path = std::path::Path::new("assets/scenes/manifest.json");
    if !manifest_path.exists() {
        warn!("Scene manifest not found at {:?}", manifest_path);
        return Vec::new();
    }
    let data = std::fs::read_to_string(manifest_path).unwrap();
    serde_json::from_str(&data).unwrap_or_default()
}

pub fn load_scene(path: &str) -> Option<SceneFile> {
    // Adjust path: original uses "./scenes/..." we use "assets/scenes/..."
    let adjusted = path
        .trim_start_matches("./scenes/")
        .trim_start_matches("scenes/");
    let full_path = format!("assets/scenes/{}", adjusted);
    let data = std::fs::read_to_string(&full_path).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn apply_scene(
    scene: &SceneFile,
    sim_state: &mut SimState,
    params: &mut SimParams,
    window_width: f32,
    window_height: f32,
) -> Vec<SimShapeData> {
    let scene_width = scene.resolution[0] as f32;
    let scene_height = scene.resolution[1] as f32;

    let aspect_ratio = scene_width / scene_height;

    // Compute actual resolution maintaining aspect ratio
    let (res_w, res_h) = {
        let wanted_h = window_width / aspect_ratio;
        if wanted_h > window_height {
            (window_height * aspect_ratio, window_height)
        } else {
            (window_width, wanted_h)
        }
    };

    let width_scale = res_w / scene_width;
    let height_scale = res_h / scene_height;
    let scale_scale = (width_scale * height_scale).sqrt();

    sim_state.resolution = [res_w, res_h];
    sim_state.grid_size = [
        (res_w / params.sim_res_divisor as f32) as u32,
        (res_h / params.sim_res_divisor as f32) as u32,
    ];

    let mut shape_data_list = Vec::new();
    for mut shape in scene.shapes.clone() {
        shape.position.x *= width_scale as f64;
        shape.position.y *= height_scale as f64;
        shape.half_size.x *= scale_scale as f64;
        shape.half_size.y *= scale_scale as f64;
        shape.radius *= scale_scale;
        shape_data_list.push(SimShapeData::from(&shape));
    }

    // Apply scene settings
    for setting in &scene.settings {
        match setting.name.as_str() {
            "iterationCount" => {
                if let Some(v) = setting.value.as_f64() {
                    params.iteration_count = v as u32;
                }
            }
            "simResDivisor" => {
                if let Some(v) = setting.value.as_f64() {
                    params.sim_res_divisor = v as u32;
                }
            }
            "particlesPerCellAxis" => {
                if let Some(v) = setting.value.as_f64() {
                    params.particles_per_cell_axis = v as u32;
                }
            }
            "gravityStrength" => {
                if let Some(v) = setting.value.as_f64() {
                    params.gravity_strength = v as f32;
                }
            }
            "liquidViscosity" => {
                if let Some(v) = setting.value.as_f64() {
                    params.liquid_viscosity = v as f32;
                }
            }
            "elasticityRatio" => {
                if let Some(v) = setting.value.as_f64() {
                    params.elasticity_ratio = v as f32;
                }
            }
            "liquidRelaxation" => {
                if let Some(v) = setting.value.as_f64() {
                    params.liquid_relaxation = v as f32;
                }
            }
            "elasticRelaxation" => {
                if let Some(v) = setting.value.as_f64() {
                    params.elastic_relaxation = v as f32;
                }
            }
            "frictionAngle" => {
                if let Some(v) = setting.value.as_f64() {
                    params.friction_angle = v as f32;
                }
            }
            "plasticity" => {
                if let Some(v) = setting.value.as_f64() {
                    params.plasticity = v as f32;
                }
            }
            "borderFriction" => {
                if let Some(v) = setting.value.as_f64() {
                    params.border_friction = v as f32;
                }
            }
            "simRate" => {
                if let Some(v) = setting.value.as_f64() {
                    params.sim_rate = v as u32;
                }
            }
            _ => {}
        }
    }

    shape_data_list
}

#[derive(Resource, Default)]
pub struct SceneManifest(pub Vec<SceneManifestEntry>);
