mod scene;
mod shape_editor;
mod simulation;
mod time_regulation;
mod types;
mod ui;

use bevy::dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin};
use bevy::feathers::FeathersPlugins;
use bevy::input::mouse::AccumulatedMouseScroll;
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
        .add_plugins(FpsOverlayPlugin {
            config: FpsOverlayConfig {
                text_config: TextFont {
                    font_size: 14.0,
                    ..default()
                },
                text_color: Color::srgba(0.0, 1.0, 0.0, 0.7),
                ..default()
            },
        })
        .insert_resource(bevy::feathers::theme::UiTheme(
            bevy::feathers::dark_theme::create_dark_theme(),
        ))
        .add_plugins(PbmpmPlugin)
        .init_resource::<SimParams>()
        .init_resource::<SimState>()
        .init_resource::<InputState>()
        .init_resource::<TimeRegulation>()
        .init_resource::<SceneManifest>()
        .init_resource::<ParticleCount>()
        .init_resource::<shape_editor::ShapeInteraction>()
        .add_observer(ui::on_scroll)
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
                ui::sync_shape_sliders,
                ui::update_stats,
                ui::update_shape_info,
                ui::toggle_ui,
                ui::send_scroll_events,
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
            let (w, h) = if let Ok(window) = windows.single() {
                (window.width(), window.height())
            } else {
                (1280.0, 720.0)
            };
            let new_shapes = apply_scene(&scene_file, &mut sim_state, &mut params, w, h);
            let shape_count = new_shapes.len();
            for shape_data in new_shapes {
                commands.spawn(shape_data);
            }
            commands.trigger(ResetSimulation);
            info!("Loaded scene: {} ({} shapes)", entry.name, shape_count);
        }
    }
    info!("PB-MPM initialized with {} scenes", manifest.0.len());
}

fn input_system(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    scroll: Res<AccumulatedMouseScroll>,
    windows: Query<&Window>,
    mut input: ResMut<InputState>,
    mut params: ResMut<SimParams>,
    interaction: Res<shape_editor::ShapeInteraction>,
) {
    let Ok(window) = windows.single() else { return };
    input.mouse_prev_position = input.mouse_position;
    if let Some(pos) = window.cursor_position() {
        input.mouse_position = pos;
    }
    // Mouse down only when not dragging a shape
    input.mouse_down = mouse_buttons.pressed(MouseButton::Left) && interaction.dragging.is_none();

    // Scroll wheel adjusts mouse interaction radius (only when not over UI panel)
    let over_panel = input.mouse_position.x > window.width() - UI_PANEL_WIDTH;
    if scroll.delta.y != 0.0 && !over_panel {
        params.mouse_radius *= 1.01_f32.powf(scroll.delta.y);
        params.mouse_radius = params.mouse_radius.clamp(10.0, 1000.0);
    }
}

#[allow(clippy::too_many_arguments)]
fn keyboard_system(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut sim_state: ResMut<SimState>,
    manifest: Res<SceneManifest>,
    mut params: ResMut<SimParams>,
    windows: Query<&Window>,
    mut q_name: Query<&mut Text, With<ui::SceneNameLabel>>,
    existing_shapes: Query<Entity, With<SimShapeData>>,
) {
    if keys.just_pressed(KeyCode::F5) {
        commands.trigger(ResetSimulation);
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
            &mut commands,
            &mut sim_state,
            &mut params,
            &manifest,
            &windows,
            &mut q_name,
            &existing_shapes,
        );
    }
}
