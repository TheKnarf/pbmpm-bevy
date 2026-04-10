//! Adapter from window/mouse input into the engine's `SimInteraction`
//! resource. The simulation crate is input-agnostic; this is the bridge
//! that turns "left click in the canvas area" into a generic interaction.

use bevy::prelude::*;
use pbmpm_bevy::*;

use crate::shape_editor::ShapeInteraction;
use crate::ui::UI_PANEL_WIDTH;

/// User-facing settings for the mouse interaction. Held in screen-space
/// units (pixels) and translated to sim-space when forwarding to the
/// simulation each frame.
#[derive(Resource, Debug, Clone)]
pub struct MouseConfig {
    pub mode: InteractionMode,
    pub radius_pixels: f32,
}

impl Default for MouseConfig {
    fn default() -> Self {
        Self {
            mode: InteractionMode::Grab,
            radius_pixels: 100.0,
        }
    }
}

/// Update the simulation's `SimInteraction` and `SimViewport` resources
/// from the current window cursor and mouse buttons. This is the only
/// system that bridges OS input → physics input.
pub fn drive_sim_interaction(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    shape_interaction: Res<ShapeInteraction>,
    config: Res<MouseConfig>,
    time: Res<Time>,
    windows: Query<&Window>,
    mut sim_interaction: ResMut<SimInteraction>,
    mut sim_viewport: ResMut<SimViewport>,
    mut prev_cursor: Local<Option<Vec2>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let res_w = window.width();
    let res_h = window.height();
    sim_viewport.resolution = Vec2::new(res_w, res_h);

    let cursor = window.cursor_position().unwrap_or_default();
    let prev = prev_cursor.unwrap_or(cursor);
    *prev_cursor = Some(cursor);

    // Cursor is screen-space (origin top-left, Y down). Convert to Bevy
    // 2D world space (origin center, Y up).
    let to_world = |c: Vec2| Vec2::new(c.x - res_w * 0.5, res_h * 0.5 - c.y);
    let pos_world = to_world(cursor);
    let prev_world = to_world(prev);

    let dt = time.delta_secs();
    let velocity = if dt > 0.0 {
        (pos_world - prev_world) / dt
    } else {
        Vec2::ZERO
    };

    let over_panel = cursor.x > res_w - UI_PANEL_WIDTH;
    let active = mouse_buttons.pressed(MouseButton::Left)
        && shape_interaction.dragging.is_none()
        && !over_panel;

    sim_interaction.active = active;
    sim_interaction.mode = config.mode;
    sim_interaction.position = pos_world;
    sim_interaction.velocity = velocity;
    sim_interaction.radius = config.radius_pixels;
}
