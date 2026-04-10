use bevy::prelude::*;

use crate::mouse_input::MouseConfig;
use crate::ui::{cursor_over_panel, UiPanelVisible};
use pbmpm_bevy::*;

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

/// Convert a screen-space cursor position (origin top-left, Y down) to
/// Bevy 2D world space (origin center, Y up).
fn cursor_to_world(cursor: Vec2, window_w: f32, window_h: f32) -> Vec2 {
    Vec2::new(cursor.x - window_w / 2.0, window_h / 2.0 - cursor.y)
}

/// Distance from a point to a circle border.
fn dist_to_circle(point: Vec2, center: Vec2, radius: f32) -> f32 {
    ((point - center).length() - radius).abs()
}

/// Distance from a point to a box border. Rotation matches the shader's
/// convention (positive = visually clockwise in Y-up world space).
fn dist_to_box(point: Vec2, center: Vec2, half_size: Vec2, rotation_deg: f32) -> f32 {
    let rot_rad = rotation_deg.to_radians();
    let offset = point - center;
    let cos_r = rot_rad.cos();
    let sin_r = rot_rad.sin();
    // local = R(rotation) * offset, matching SimShape::collide() in WGSL.
    let local = Vec2::new(
        offset.x * cos_r - offset.y * sin_r,
        offset.x * sin_r + offset.y * cos_r,
    );
    let d = local.abs() - half_size;
    let outside_dist = d.max(Vec2::ZERO).length();
    let inside_dist = d.x.max(d.y).min(0.0);
    (outside_dist + inside_dist).abs()
}

/// Draw shape outlines using gizmos.
pub fn draw_shape_overlay(
    mut gizmos: Gizmos,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_config: Res<MouseConfig>,
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
    let cursor = window.cursor_position().unwrap_or_default();
    let mouse_world = cursor_to_world(cursor, w, h);
    let mouse_active = mouse_buttons.pressed(MouseButton::Left) && interaction.dragging.is_none();
    let alpha = if mouse_active { 0.5 } else { 0.15 };
    gizmos
        .circle_2d(
            Isometry2d::from_translation(mouse_world),
            mouse_config.radius_pixels,
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
            match shape.function {
                ShapeFunction::Emit => match shape.emit_material {
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

        if shape.shape_type.is_circle() {
            gizmos.circle_2d(
                Isometry2d::from_translation(shape.position),
                shape.radius,
                color,
            );
        } else {
            // The shader treats positive rotation as visually clockwise
            // (HTML canvas convention). Negate for Bevy's CCW-positive Rot2.
            let rot_rad = -shape.rotation.to_radians();
            let iso = Isometry2d::new(shape.position, Rot2::radians(rot_rad));
            gizmos.rect_2d(iso, shape.half_size * 2.0, color);
        }
    }
}

/// Handle mouse interaction with shapes: hover detection, selection, and dragging.
pub fn shape_mouse_interaction(
    mut interaction: ResMut<ShapeInteraction>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    panel: Res<UiPanelVisible>,
    windows: Query<&Window>,
    mut shapes: Query<(Entity, &mut SimShapeData)>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };

    // Don't interact if cursor is over the visible UI panel
    if cursor_over_panel(cursor.x, window.width(), &panel) {
        interaction.hovered = None;
        return;
    }

    let mouse_world = cursor_to_world(cursor, window.width(), window.height());
    let selection_range = 20.0;

    // Hover detection
    if interaction.dragging.is_none() {
        let mut best_dist = f32::INFINITY;
        let mut best_entity = None;

        for (entity, shape) in shapes.iter() {
            let dist = if shape.shape_type.is_circle() {
                dist_to_circle(mouse_world, shape.position, shape.radius)
            } else {
                dist_to_box(mouse_world, shape.position, shape.half_size, shape.rotation)
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
                    mouse_start: mouse_world,
                    shape_start_pos: shape.position,
                });
            }
        } else {
            // Deselect when clicking empty canvas
            interaction.selected = None;
        }
    }

    // Dragging
    if mouse_buttons.pressed(MouseButton::Left) {
        if let (Some(entity), Some(drag)) = (interaction.selected, &interaction.dragging) {
            let offset = mouse_world - drag.mouse_start;
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

    // Spawn a shape and select it.
    let spawn_and_select =
        |commands: &mut Commands, interaction: &mut ShapeInteraction, shape: SimShapeData| {
            let entity = commands.spawn(shape).id();
            interaction.selected = Some(entity);
        };

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
                spawn_and_select(&mut commands, &mut interaction, new_shape);
            }
        }
    }

    // New shape (Ctrl+N) — spawned at world origin (viewport center)
    if keys.just_pressed(KeyCode::KeyN)
        && (keys.pressed(KeyCode::SuperLeft) || keys.pressed(KeyCode::SuperRight))
    {
        let new_shape = SimShapeData {
            position: Vec2::ZERO,
            half_size: Vec2::new(50.0, 50.0),
            shape_type: ShapeType::Box,
            function: ShapeFunction::Emit,
            emit_material: MaterialType::Liquid,
            emission_rate: 2.5,
            radius: 50.0,
            ..default()
        };
        spawn_and_select(&mut commands, &mut interaction, new_shape);
    }
}
