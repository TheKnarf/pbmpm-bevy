use bevy::prelude::*;

use crate::types::*;

/// Tracks shape selection and dragging state.
#[derive(Resource, Default)]
pub struct ShapeInteraction {
    pub selected: Option<Entity>,
    pub hovered: Option<Entity>,
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
    input: Res<InputState>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    params: Res<SimParams>,
    interaction: Res<ShapeInteraction>,
    windows: Query<&Window>,
    shapes: Query<(Entity, &SimShapeData)>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let w = window.width();
    let h = window.height();

    // Draw mouse interaction radius circle at cursor position
    let mouse_world = shape_to_world(input.mouse_position, w, h);
    let mouse_active = mouse_buttons.pressed(MouseButton::Left) && interaction.dragging.is_none();
    let alpha = if mouse_active { 0.5 } else { 0.15 };
    gizmos
        .circle_2d(
            Isometry2d::from_translation(mouse_world),
            params.mouse_radius,
            Color::srgba(0.5, 0.5, 0.5, alpha),
        )
        .resolution(48);

    for (entity, shape) in shapes.iter() {
        let is_selected = interaction.selected == Some(entity);
        let is_hovered = interaction.hovered == Some(entity);

        let color = if is_selected {
            Color::srgba(0.3, 0.3, 1.0, 0.8)
        } else if is_hovered {
            Color::WHITE.with_alpha(0.6)
        } else {
            match ShapeFunction::from_u32(shape.function) {
                ShapeFunction::Emit => match MaterialType::from_u32(shape.emit_material) {
                    MaterialType::Liquid => Color::srgba(1.0, 0.3, 0.3, 0.4),
                    MaterialType::Elastic => Color::srgba(1.0, 1.0, 0.3, 0.4),
                    MaterialType::Sand => Color::srgba(1.0, 1.0, 0.3, 0.4),
                    MaterialType::Visco => Color::srgba(1.0, 0.5, 1.0, 0.4),
                },
                ShapeFunction::Collider => Color::srgba(0.5, 0.5, 0.5, 0.4),
                ShapeFunction::Drain => Color::srgba(1.0, 0.2, 0.2, 0.4),
                ShapeFunction::InitialEmit => Color::srgba(0.0, 0.8, 0.8, 0.4),
            }
        };

        let pos = shape.position;
        let world_pos = shape_to_world(pos, w, h);

        if ShapeType::from_u32(shape.shape_type).is_circle() {
            // Circle
            gizmos.circle_2d(Isometry2d::from_translation(world_pos), shape.radius, color);
        } else {
            // Box
            let hs = shape.half_size;
            let rot_rad = shape.rotation.to_radians();
            let iso = Isometry2d::new(world_pos, Rot2::radians(-rot_rad)); // negate for Y-flip
            gizmos.rect_2d(iso, hs * 2.0, color);
        }
    }
}

/// Handle mouse interaction with shapes: hover detection, selection, and dragging.
pub fn shape_mouse_interaction(
    mut interaction: ResMut<ShapeInteraction>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut shapes: Query<(Entity, &mut SimShapeData)>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };

    // Don't interact if cursor is over the UI panel
    if cursor.x > window.width() - UI_PANEL_WIDTH {
        interaction.hovered = None;
        return;
    }

    let mouse_pos = cursor_to_shape_space(cursor);
    let selection_range = 20.0;

    // Hover detection
    if interaction.dragging.is_none() {
        let mut best_dist = f32::INFINITY;
        let mut best_entity = None;

        for (entity, shape) in shapes.iter() {
            let pos = shape.position;

            let dist = if ShapeType::from_u32(shape.shape_type).is_circle() {
                dist_to_circle(mouse_pos, pos, shape.radius)
            } else {
                dist_to_box(mouse_pos, pos, shape.half_size, shape.rotation)
            };

            if dist < selection_range && dist < best_dist {
                best_dist = dist;
                best_entity = Some(entity);
            }
        }

        interaction.hovered = best_entity;
    }

    // Selection and drag start
    if mouse_buttons.just_pressed(MouseButton::Left) {
        if let Some(hovered) = interaction.hovered {
            interaction.selected = Some(hovered);
            if let Ok((_, shape)) = shapes.get(hovered) {
                interaction.dragging = Some(DragState {
                    mouse_start: mouse_pos,
                    shape_start_pos: shape.position,
                });
            }
        } else if cursor.x <= window.width() - UI_PANEL_WIDTH {
            // Deselect when clicking empty area (but not on UI panel)
            interaction.selected = None;
        }
    }

    // Dragging
    if mouse_buttons.pressed(MouseButton::Left) {
        if let (Some(entity), Some(drag)) = (interaction.selected, &interaction.dragging) {
            let offset = mouse_pos - drag.mouse_start;
            let new_pos = drag.shape_start_pos + offset;
            if let Ok((_, mut shape)) = shapes.get_mut(entity) {
                shape.position = new_pos;
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
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut interaction: ResMut<ShapeInteraction>,
    windows: Query<&Window>,
    shapes: Query<(Entity, &SimShapeData)>,
) {
    // Delete selected shape
    if keys.just_pressed(KeyCode::Delete) || keys.just_pressed(KeyCode::Backspace) {
        if let Some(entity) = interaction.selected {
            commands.entity(entity).despawn();
            interaction.selected = None;
            interaction.hovered = None;
        }
    }

    // Duplicate selected shape (Ctrl+D)
    if keys.just_pressed(KeyCode::KeyD)
        && (keys.pressed(KeyCode::SuperLeft) || keys.pressed(KeyCode::SuperRight))
    {
        if let Some(entity) = interaction.selected {
            if let Ok((_, shape)) = shapes.get(entity) {
                let mut new_shape = shape.clone();
                // Offset the duplicate slightly
                new_shape.position.x += 20.0;
                new_shape.position.y += 20.0;
                // Generate a new id
                let count = shapes.iter().count();
                new_shape.id = format!("shape{}", count);
                let new_entity = commands.spawn(new_shape).id();
                interaction.selected = Some(new_entity);
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
        let cx = window.width() / 2.0;
        let cy = window.height() / 2.0;
        let count = shapes.iter().count();
        let new_shape = SimShapeData {
            id: format!("shape{}", count),
            position: Vec2::new(cx, cy),
            half_size: Vec2::new(50.0, 50.0),
            rotation: 0.0,
            shape_type: 0,    // Box
            function: 0,      // Emitter
            emit_material: 0, // Liquid
            emission_rate: 2.5,
            emission_speed: 0.0,
            radius: 50.0,
        };
        let new_entity = commands.spawn(new_shape).id();
        interaction.selected = Some(new_entity);
    }
}
