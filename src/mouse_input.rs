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
#[allow(clippy::too_many_arguments)]
pub fn drive_sim_interaction(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    shape_interaction: Res<ShapeInteraction>,
    config: Res<MouseConfig>,
    params: Res<SimParams>,
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

    let render_to_sim = 1.0 / params.sim_res_divisor as f32;
    let grid_w = (res_w * render_to_sim).max(1.0);
    let grid_h = (res_h * render_to_sim).max(1.0);

    // Cursor in sim grid coordinates (origin bottom-left, Y up).
    let to_sim = |c: Vec2| Vec2::new(grid_w * (c.x / res_w), grid_h * (1.0 - c.y / res_h));
    let pos_sim = to_sim(cursor);
    let prev_sim = to_sim(prev);

    let dt = time.delta_secs();
    let velocity = if dt > 0.0 {
        (pos_sim - prev_sim) / dt
    } else {
        Vec2::ZERO
    };

    let over_panel = cursor.x > res_w - UI_PANEL_WIDTH;
    let active = mouse_buttons.pressed(MouseButton::Left)
        && shape_interaction.dragging.is_none()
        && !over_panel;

    sim_interaction.active = active;
    sim_interaction.mode = config.mode;
    sim_interaction.position = pos_sim;
    sim_interaction.velocity = velocity;
    sim_interaction.radius = config.radius_pixels * render_to_sim;
}
