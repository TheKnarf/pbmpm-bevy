#![allow(dead_code)]

mod scene;
mod shape_editor;
mod simulation;
mod time_regulation;
mod types;
mod ui;

use bevy::feathers::FeathersPlugins;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};

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
                        resolution: bevy::window::WindowResolution::new(1280, 720),
                        ..default()
                    }),
                    ..default()
                })
                .set(bevy::log::LogPlugin {
                    filter: "wgpu=warn,bevy_render=info,pbmpm_bevy=info".into(),
                    ..default()
                }),
        )
        .add_plugins(FeathersPlugins)
        .insert_resource(bevy::feathers::theme::UiTheme(
            bevy::feathers::dark_theme::create_dark_theme(),
        ))
        .add_plugins(PbmpmPlugin)
        .init_resource::<SimParams>()
        .init_resource::<SimState>()
        .init_resource::<InputState>()
        .init_resource::<TimeRegulation>()
        .init_resource::<SceneManifest>()
        .init_resource::<shape_editor::ShapeInteraction>()
        .add_systems(Startup, (setup, ui::setup_ui).chain())
        .add_systems(
            Update,
            (
                input_system,
                keyboard_system,
                shape_editor::shape_mouse_interaction,
                shape_editor::shape_keyboard,
                shape_editor::draw_shape_overlay,
                ui::sync_params,
                ui::update_stats,
                ui::update_shape_info,
                ui::toggle_ui,
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut sim_state: ResMut<SimState>,
    mut params: ResMut<SimParams>,
    mut manifest: ResMut<SceneManifest>,
    windows: Query<&Window>,
) {
    commands.spawn((
        Camera2d,
        Camera {
            clear_color: bevy::camera::ClearColorConfig::None,
            ..default()
        },
        Msaa::Off,
    ));

    *manifest = SceneManifest(load_manifest());

    if let Some(entry) = manifest.0.first() {
        if let Some(scene_file) = load_scene(&entry.scene) {
            let Ok(window) = windows.single() else {
                apply_scene(&scene_file, &mut sim_state, &mut params, 1280.0, 720.0);
                return;
            };
            apply_scene(
                &scene_file,
                &mut sim_state,
                &mut params,
                window.width(),
                window.height(),
            );
            info!(
                "Loaded scene: {} ({} shapes)",
                entry.name,
                sim_state.shapes.len()
            );
        }
    }
    info!("PB-MPM initialized with {} scenes", manifest.0.len());
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
}

fn keyboard_system(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut sim_state: ResMut<SimState>,
    manifest: Res<SceneManifest>,
    mut params: ResMut<SimParams>,
    windows: Query<&Window>,
    mut q_name: Query<&mut Text, With<ui::SceneNameLabel>>,
) {
    if keys.just_pressed(KeyCode::F5) {
        sim_state.do_reset = true;
    }
    if keys.just_pressed(KeyCode::Space) {
        sim_state.is_paused = !sim_state.is_paused;
    }
    if keys.just_pressed(KeyCode::F12) {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk("/tmp/pbmpm_screenshot.png"));
        info!("Screenshot requested");
    }
    if keys.just_pressed(KeyCode::Tab) && !manifest.0.is_empty() {
        sim_state.scene_index = (sim_state.scene_index + 1) % manifest.0.len();
        ui::do_load_scene(
            &mut sim_state,
            &mut params,
            &manifest,
            &windows,
            &mut q_name,
        );
    }
}
