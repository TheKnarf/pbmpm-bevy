use bevy::prelude::*;
use serde::Deserialize;

use crate::json_shape::SimShape;
use pbmpm_bevy::*;

/// Event fired to request loading a scene by index.
#[derive(Event)]
pub struct LoadScene(pub usize);

#[derive(Debug, Clone, Deserialize)]
pub struct SceneManifestEntry {
    pub scene: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SceneFile {
    pub resolution: [f64; 2],
    #[serde(default)]
    pub settings: Vec<SceneSetting>,
    pub shapes: Vec<SimShape>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SceneSetting {
    pub name: String,
    pub value: serde_json::Value,
}

pub fn load_manifest() -> Vec<SceneManifestEntry> {
    let manifest_path = std::path::Path::new("assets/scenes/manifest.json");
    let data = match std::fs::read_to_string(manifest_path) {
        Ok(d) => d,
        Err(e) => {
            warn!("Failed to read scene manifest at {manifest_path:?}: {e}");
            return Vec::new();
        }
    };
    match serde_json::from_str(&data) {
        Ok(entries) => entries,
        Err(e) => {
            warn!("Failed to parse scene manifest: {e}");
            Vec::new()
        }
    }
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

    // Original PB-MPM scenes use HTML canvas conventions: top-left origin,
    // Y down. Convert to Bevy 2D world space: center origin, Y up.
    let half_w = res_w * 0.5;
    let half_h = res_h * 0.5;

    let mut shape_data_list = Vec::new();
    for mut shape in scene.shapes.clone() {
        shape.position.x = shape.position.x * width_scale as f64 - half_w as f64;
        shape.position.y = half_h as f64 - shape.position.y * height_scale as f64;
        shape.half_size.x *= scale_scale as f64;
        shape.half_size.y *= scale_scale as f64;
        shape.radius *= scale_scale;
        shape_data_list.push(SimShapeData::from(&shape));
    }

    // Apply scene settings via a table of (json key, applier).
    type Applier = fn(&mut SimParams, f64);
    const SETTING_APPLIERS: &[(&str, Applier)] = &[
        ("iterationCount", |p, v| p.iteration_count = v as u32),
        ("simResDivisor", |p, v| p.sim_res_divisor = v as u32),
        ("particlesPerCellAxis", |p, v| {
            p.particles_per_cell_axis = v as u32
        }),
        ("simRate", |p, v| p.sim_rate = v as u32),
        ("gravityStrength", |p, v| p.gravity_strength = v as f32),
        ("liquidViscosity", |p, v| p.liquid_viscosity = v as f32),
        ("elasticityRatio", |p, v| p.elasticity_ratio = v as f32),
        ("liquidRelaxation", |p, v| p.liquid_relaxation = v as f32),
        ("elasticRelaxation", |p, v| p.elastic_relaxation = v as f32),
        ("frictionAngle", |p, v| p.friction_angle = v as f32),
        ("plasticity", |p, v| p.plasticity = v as f32),
        ("borderFriction", |p, v| p.border_friction = v as f32),
    ];

    for setting in &scene.settings {
        let Some(value) = setting.value.as_f64() else {
            continue;
        };
        for (key, applier) in SETTING_APPLIERS {
            if setting.name == *key {
                applier(params, value);
                break;
            }
        }
    }

    shape_data_list
}

#[derive(Resource, Default)]
pub struct SceneManifest(pub Vec<SceneManifestEntry>);

/// Observer for `LoadScene` events. Despawns existing shape entities, parses
/// the scene JSON, applies its settings to `SimParams`, and spawns new shape
/// entities. UI synchronization is handled by a follow-up observer in `ui`.
pub fn on_load_scene(
    trigger: On<LoadScene>,
    mut commands: Commands,
    mut sim_state: ResMut<SimState>,
    mut params: ResMut<SimParams>,
    manifest: Res<SceneManifest>,
    windows: Query<&Window>,
    existing_shapes: Query<Entity, With<SimShapeData>>,
) {
    let idx = trigger.event().0;
    if idx >= manifest.0.len() {
        return;
    }
    sim_state.scene_index = idx;
    let entry = &manifest.0[idx];
    let Some(scene_file) = load_scene(&entry.scene) else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };

    // Reset params to defaults, then apply scene-specific overrides.
    *params = SimParams::default();

    for entity in existing_shapes.iter() {
        commands.entity(entity).despawn();
    }

    let new_shapes = apply_scene(&scene_file, &mut params, window.width(), window.height());
    for shape_data in new_shapes {
        commands.spawn(shape_data);
    }

    commands.trigger(SceneLoadedEvent {
        name: entry.name.clone(),
    });
    commands.trigger(ResetSimulation);
}

/// Fired by `on_load_scene` after a scene has been loaded into the world,
/// for UI observers to update labels and sliders.
#[derive(Event)]
pub struct SceneLoadedEvent {
    pub name: String,
}
