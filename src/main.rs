#![allow(dead_code)]

mod scene;
mod simulation;
mod time_regulation;
mod types;
mod ui;

use bevy::prelude::*;
use bevy::render::camera::ClearColorConfig;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy_egui::{EguiContextPass, EguiPlugin};

use scene::*;
use simulation::PbmpmPlugin;
use time_regulation::TimeRegulation;
use types::*;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "PB-MPM (Bevy)".into(),
                        resolution: (1280., 720.).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(bevy::log::LogPlugin {
                    filter: "wgpu=warn,bevy_render=info,pbmpm_bevy=info".into(),
                    ..default()
                }),
        )
        .add_plugins(EguiPlugin { enable_multipass_for_primary_context: true })
        .add_plugins(PbmpmPlugin)
        .init_resource::<SimParams>()
        .init_resource::<SimState>()
        .init_resource::<InputState>()
        .init_resource::<TimeRegulation>()
        .init_resource::<SceneManifest>()
        .add_systems(Startup, setup)
        .add_systems(Update, (input_system, keyboard_system))
        .add_systems(EguiContextPass, ui::ui_system)
        .run();
}

fn setup(
    mut commands: Commands,
    mut sim_state: ResMut<SimState>,
    mut params: ResMut<SimParams>,
    mut manifest: ResMut<SceneManifest>,
    windows: Query<&Window>,
) {
    // Our custom node renders to ViewTarget before the camera pipeline runs.
    // Camera set to None so it doesn't clear our content.
    // Disable HDR and MSAA to avoid intermediate texture complications.
    commands.spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::None,
            hdr: false,
            ..default()
        },
        Msaa::Off,
    ));

    // Load scene manifest
    *manifest = SceneManifest(load_manifest());

    // Load first scene
    if let Some(entry) = manifest.0.first() {
        if let Some(scene_file) = load_scene(&entry.scene) {
            let Ok(window) = windows.single() else {
                info!("Window not ready at startup, using default resolution");
                apply_scene(&scene_file, &mut sim_state, &mut params, 1280.0, 720.0);
                info!("Loaded scene with {} shapes", sim_state.shapes.len());
                return;
            };
            apply_scene(
                &scene_file,
                &mut sim_state,
                &mut params,
                window.width(),
                window.height(),
            );
            info!("Loaded scene with {} shapes, grid {}x{}", sim_state.shapes.len(), sim_state.grid_size[0], sim_state.grid_size[1]);
        } else {
            warn!("Failed to load scene file");
        }
    }

    info!("PB-MPM Bevy initialized with {} scenes", manifest.0.len());
}

fn input_system(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut input: ResMut<InputState>,
) {
    let Ok(window) = windows.single() else { return };

    input.mouse_prev_position = input.mouse_position;

    if let Some(pos) = window.cursor_position() {
        input.mouse_position = pos;
    }

    input.mouse_down = mouse_buttons.pressed(MouseButton::Left);

    // Scroll to adjust mouse radius
    // (In Bevy, mouse wheel is handled via events, simplified here)
}

fn keyboard_system(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut sim_state: ResMut<SimState>,
    manifest: Res<SceneManifest>,
    mut params: ResMut<SimParams>,
    windows: Query<&Window>,
) {
    if keys.just_pressed(KeyCode::F5) {
        sim_state.do_reset = true;
    }

    if keys.just_pressed(KeyCode::Space) {
        sim_state.is_paused = !sim_state.is_paused;
    }

    if keys.just_pressed(KeyCode::F12) {
        commands.spawn(Screenshot::primary_window())
            .observe(save_to_disk("/tmp/pbmpm_screenshot.png"));
        info!("Screenshot requested");
    }

    if keys.just_pressed(KeyCode::Tab) && !manifest.0.is_empty() {
        sim_state.scene_index = (sim_state.scene_index + 1) % manifest.0.len();
        if let Some(entry) = manifest.0.get(sim_state.scene_index) {
            if let Some(scene_file) = load_scene(&entry.scene) {
                let Ok(window) = windows.single() else { return };
                *params = SimParams::default();
                apply_scene(
                    &scene_file,
                    &mut sim_state,
                    &mut params,
                    window.width(),
                    window.height(),
                );
            }
        }
    }
}

