mod json_shape;
mod mouse_input;
mod scene;
mod shape_editor;
mod ui;

use bevy::dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin};
use bevy::feathers::FeathersPlugins;
use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};

use mouse_input::MouseConfig;
use pbmpm_bevy::*;
use scene::*;
use ui::UI_PANEL_WIDTH;

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
                    filter: "wgpu=warn,bevy_render=info,pbmpm_bevy=info,pbmpm_app=info".into(),
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
        .init_resource::<SceneManifest>()
        .init_resource::<MouseConfig>()
        .init_resource::<shape_editor::ShapeInteraction>()
        .add_observer(ui::on_scroll)
        .add_observer(scene::on_load_scene)
        .add_observer(ui::on_scene_loaded)
        .add_systems(Startup, (setup, ui::setup_ui).chain())
        .add_systems(
            Update,
            (
                input_system,
                keyboard_system,
                mouse_input::drive_sim_interaction,
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

fn setup(mut commands: Commands, mut manifest: ResMut<SceneManifest>) {
    commands.spawn((
        Camera2d,
        Camera {
            clear_color: bevy::camera::ClearColorConfig::None,
            ..default()
        },
        Msaa::Off,
    ));

    *manifest = SceneManifest(load_manifest());
    info!("PB-MPM initialized with {} scenes", manifest.0.len());

    if !manifest.0.is_empty() {
        commands.trigger(LoadScene(0));
    }
}

fn input_system(
    scroll: Res<AccumulatedMouseScroll>,
    windows: Query<&Window>,
    mut mouse_config: ResMut<MouseConfig>,
) {
    let Ok(window) = windows.single() else { return };
    let cursor = window.cursor_position().unwrap_or_default();

    // Scroll wheel adjusts interaction radius (only when not over UI panel)
    let over_panel = cursor.x > window.width() - UI_PANEL_WIDTH;
    if scroll.delta.y != 0.0 && !over_panel {
        mouse_config.radius_pixels =
            (mouse_config.radius_pixels * 1.01_f32.powf(scroll.delta.y)).clamp(10.0, 1000.0);
    }
}

fn keyboard_system(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut sim_state: ResMut<SimState>,
    manifest: Res<SceneManifest>,
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
        let next = (sim_state.scene_index + 1) % manifest.0.len();
        commands.trigger(LoadScene(next));
    }
}
