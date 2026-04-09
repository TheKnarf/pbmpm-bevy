use bevy::prelude::*;

use crate::types::*;

/// Tracks shape selection and dragging state.
#[derive(Resource, Default)]
pub struct ShapeInteraction {
    pub selected_index: Option<usize>,
    pub hovered_index: Option<usize>,
    pub dragging: Option<DragState>,
}

pub struct DragState {
    pub mouse_start: Vec2,
    pub shape_start_pos: Vec2,
}

/// Convert shape pixel-space position (origin top-left, Y down) to Bevy world-space
/// (origin center, Y up) for gizmo drawing.
fn shape_to_world(pos: Vec2, window_w: f32, window_h: f32) -> Vec2 {
    Vec2::new(pos.x - window_w / 2.0, window_h / 2.0 - pos.y)
}

/// Convert Bevy screen-space cursor position (origin top-left, Y down) to shape pixel-space.
/// In Bevy 0.18, `Window::cursor_position()` returns (0,0) at top-left, Y down.
fn cursor_to_shape_space(cursor: Vec2) -> Vec2 {
    cursor // Same coordinate system
}

/// Distance from a point to a circle border.
fn dist_to_circle(point: Vec2, center: Vec2, radius: f32) -> f32 {
    ((point - center).length() - radius).abs()
}

/// Distance from a point to a box border (with rotation in degrees).
fn dist_to_box(point: Vec2, center: Vec2, half_size: Vec2, rotation_deg: f32) -> f32 {
    let rot_rad = rotation_deg.to_radians();
    let offset = point - center;
    let cos_r = rot_rad.cos();
    let sin_r = rot_rad.sin();
    let local = Vec2::new(
        offset.x * cos_r + offset.y * sin_r,
        -offset.x * sin_r + offset.y * cos_r,
    );
    let d = local.abs() - half_size;
    let outside_dist = d.max(Vec2::ZERO).length();
    let inside_dist = d.x.max(d.y).min(0.0);
    (outside_dist + inside_dist).abs()
}

/// Draw shape outlines using gizmos.
pub fn draw_shape_overlay(
    mut gizmos: Gizmos,
    sim_state: Res<SimState>,
    interaction: Res<ShapeInteraction>,
    windows: Query<&Window>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let w = window.width();
    let h = window.height();

    for (i, shape) in sim_state.shapes.iter().enumerate() {
        let is_selected = interaction.selected_index == Some(i);
        let is_hovered = interaction.hovered_index == Some(i);

        let color = if is_selected {
            Color::srgba(0.3, 0.3, 1.0, 0.8)
        } else if is_hovered {
            Color::WHITE.with_alpha(0.6)
        } else {
            match shape.function.as_f32() as u32 {
                0 => match shape.emit_material.as_f32() as u32 {
                    0 => Color::srgba(1.0, 0.3, 0.3, 0.4), // Liquid emitter
                    1 => Color::srgba(1.0, 1.0, 0.3, 0.4), // Elastic emitter
                    2 => Color::srgba(1.0, 1.0, 0.3, 0.4), // Sand emitter
                    _ => Color::srgba(1.0, 0.5, 1.0, 0.4), // Visco emitter
                },
                1 => Color::srgba(0.5, 0.5, 0.5, 0.4), // Collider
                2 => Color::srgba(1.0, 0.2, 0.2, 0.4), // Drain
                3 => Color::srgba(0.0, 0.8, 0.8, 0.4), // Initial emitter
                _ => Color::srgba(0.5, 0.5, 0.5, 0.4),
            }
        };

        let pos = shape.position.as_vec2();
        let world_pos = shape_to_world(pos, w, h);
        let shape_type = shape.shape.as_f32() as u32;

        if shape_type == 1 {
            // Circle
            gizmos.circle_2d(Isometry2d::from_translation(world_pos), shape.radius, color);
        } else {
            // Box
            let hs = shape.half_size.as_vec2();
            let rot_rad = shape.rotation.to_radians();
            let iso = Isometry2d::new(world_pos, Rot2::radians(-rot_rad)); // negate for Y-flip
            gizmos.rect_2d(iso, hs * 2.0, color);
        }
    }
}

/// Handle mouse interaction with shapes: hover detection, selection, and dragging.
pub fn shape_mouse_interaction(
    mut sim_state: ResMut<SimState>,
    mut interaction: ResMut<ShapeInteraction>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };

    // Don't interact if cursor is over the UI panel (left 310px)
    if cursor.x < 310.0 {
        interaction.hovered_index = None;
        return;
    }

    let mouse_pos = cursor_to_shape_space(cursor);
    let selection_range = 20.0;

    // Hover detection
    if interaction.dragging.is_none() {
        let mut best_dist = f32::INFINITY;
        let mut best_idx = None;

        for (i, shape) in sim_state.shapes.iter().enumerate() {
            let pos = shape.position.as_vec2();
            let shape_type = shape.shape.as_f32() as u32;

            let dist = if shape_type == 1 {
                dist_to_circle(mouse_pos, pos, shape.radius)
            } else {
                dist_to_box(mouse_pos, pos, shape.half_size.as_vec2(), shape.rotation)
            };

            if dist < selection_range && dist < best_dist {
                best_dist = dist;
                best_idx = Some(i);
            }
        }

        interaction.hovered_index = best_idx;
    }

    // Selection and drag start
    if mouse_buttons.just_pressed(MouseButton::Left) {
        if let Some(hovered) = interaction.hovered_index {
            interaction.selected_index = Some(hovered);
            let shape_pos = sim_state.shapes[hovered].position.as_vec2();
            interaction.dragging = Some(DragState {
                mouse_start: mouse_pos,
                shape_start_pos: shape_pos,
            });
        } else if cursor.x >= 310.0 {
            // Deselect when clicking empty area (but not on UI panel)
            interaction.selected_index = None;
        }
    }

    // Dragging
    if mouse_buttons.pressed(MouseButton::Left) {
        if let (Some(idx), Some(drag)) = (interaction.selected_index, &interaction.dragging) {
            let offset = mouse_pos - drag.mouse_start;
            let new_pos = drag.shape_start_pos + offset;
            if let Some(shape) = sim_state.shapes.get_mut(idx) {
                shape.position.x = new_pos.x as f64;
                shape.position.y = new_pos.y as f64;
            }
        }
    }

    // Drag end
    if mouse_buttons.just_released(MouseButton::Left) {
        interaction.dragging = None;
    }
}

/// Handle keyboard shortcuts for shape editing.
pub fn shape_keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    mut sim_state: ResMut<SimState>,
    mut interaction: ResMut<ShapeInteraction>,
    windows: Query<&Window>,
) {
    // Delete selected shape
    if keys.just_pressed(KeyCode::Delete) || keys.just_pressed(KeyCode::Backspace) {
        if let Some(idx) = interaction.selected_index {
            if idx < sim_state.shapes.len() {
                sim_state.shapes.remove(idx);
                interaction.selected_index = None;
                interaction.hovered_index = None;
            }
        }
    }

    // Duplicate selected shape (Ctrl+D)
    if keys.just_pressed(KeyCode::KeyD)
        && (keys.pressed(KeyCode::SuperLeft) || keys.pressed(KeyCode::SuperRight))
    {
        if let Some(idx) = interaction.selected_index {
            if let Some(shape) = sim_state.shapes.get(idx).cloned() {
                let mut new_shape = shape;
                // Offset the duplicate slightly
                new_shape.position.x += 20.0;
                new_shape.position.y += 20.0;
                // Generate a new id
                let new_id = format!("shape{}", sim_state.shapes.len());
                new_shape.id = new_id;
                let new_idx = sim_state.shapes.len();
                sim_state.shapes.push(new_shape);
                interaction.selected_index = Some(new_idx);
            }
        }
    }

    // New shape (Ctrl+N)
    if keys.just_pressed(KeyCode::KeyN)
        && (keys.pressed(KeyCode::SuperLeft) || keys.pressed(KeyCode::SuperRight))
    {
        let Ok(window) = windows.single() else {
            return;
        };
        let cx = (window.width() / 2.0) as f64;
        let cy = (window.height() / 2.0) as f64;
        let new_shape = SimShape {
            id: format!("shape{}", sim_state.shapes.len()),
            position: Vec2Json { x: cx, y: cy },
            half_size: Vec2Json { x: 50.0, y: 50.0 },
            rotation: 0.0,
            shape: StringOrNumber::Int(0),         // Box
            function: StringOrNumber::Int(0),      // Emitter
            emit_material: StringOrNumber::Int(0), // Liquid
            emission_rate: StringOrNumber::Float(2.5),
            emission_speed: StringOrNumber::Float(0.0),
            radius: 50.0,
        };
        let idx = sim_state.shapes.len();
        sim_state.shapes.push(new_shape);
        interaction.selected_index = Some(idx);
    }
}
