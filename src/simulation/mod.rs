pub mod gpu_state;
pub mod node;
pub mod shaders;

use bevy::prelude::*;
use bevy::render::render_graph::{RenderGraph, RenderLabel};
use bevy::render::{Extract, RenderApp};

use crate::time_regulation::TimeRegulation;
use crate::types::*;
use node::*;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct PbmpmNodeLabel;

/// Tracks whether a reset has been requested this frame.
#[derive(Resource, Default)]
struct ResetPending(bool);

pub struct PbmpmPlugin;

impl Plugin for PbmpmPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ExtractedSimDataSource>();
        app.init_resource::<ResetPending>();
        app.add_observer(on_reset_requested);

        app.add_systems(PostUpdate, prepare_extracted_data);

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<ExtractedSimData>();

        render_app.add_systems(ExtractSchedule, extract_sim_data);

        // Add render graph node
        let mut graph = render_app.world_mut().resource_mut::<RenderGraph>();
        graph.add_node(PbmpmNodeLabel, PbmpmNode::default());

        // Run before camera driver - we render to ViewTarget, camera pipeline composites to swap chain
        use bevy::render::graph::CameraDriverLabel;
        graph.add_node_edge(PbmpmNodeLabel, CameraDriverLabel);
    }
}

fn on_reset_requested(_trigger: On<ResetSimulation>, mut pending: ResMut<ResetPending>) {
    pending.0 = true;
}

/// Source data prepared in main world, ready for extraction.
#[derive(Resource, Default, Clone)]
struct ExtractedSimDataSource(ExtractedSimData);

#[allow(clippy::too_many_arguments)]
fn prepare_extracted_data(
    params: Res<SimParams>,
    mut sim_state: ResMut<SimState>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    interaction: Res<crate::shape_editor::ShapeInteraction>,
    particle_count: Res<ParticleCount>,
    mut reset_pending: ResMut<ResetPending>,
    mut time_reg: ResMut<TimeRegulation>,
    time: Res<Time>,
    windows: Query<&Window>,
    mut source: ResMut<ExtractedSimDataSource>,
    shape_query: Query<&SimShapeData>,
    mut prev_cursor: Local<Option<Vec2>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let res_w = window.width();
    let res_h = window.height();

    // Compute grid size from current window resolution.
    let grid_size = [
        (res_w / params.sim_res_divisor as f32).max(1.0) as u32,
        (res_h / params.sim_res_divisor as f32).max(1.0) as u32,
    ];
    let resolution = [res_w, res_h];

    let do_reset = reset_pending.0;

    // Reset substep index before computing substeps (matches original JS behavior)
    if do_reset {
        sim_state.substep_index = 0;
    }

    // Compute substeps
    let current_time_ms = time.elapsed_secs_f64() * 1000.0;
    let substep_count = time_reg.compute_substeps(
        current_time_ms,
        params.sim_rate,
        sim_state.is_paused,
        do_reset,
    );

    let render_to_sim_scale = 1.0 / params.sim_res_divisor as f32;

    // Convert shapes to GPU format
    let gpu_shapes: Vec<SimShapeGpu> = shape_query
        .iter()
        .map(|s| {
            let pos = [
                s.position.x * render_to_sim_scale,
                grid_size[1] as f32 - s.position.y * render_to_sim_scale, // Flip Y
            ];
            let is_circle = s.shape_type.is_circle();
            let hs = s.half_size * render_to_sim_scale;
            SimShapeGpu {
                position: pos,
                half_size: if is_circle { [0.0, 0.0] } else { [hs.x, hs.y] },
                radius: if is_circle {
                    s.radius * render_to_sim_scale
                } else {
                    0.0
                },
                rotation: s.rotation,
                shape_type: s.shape_type.to_gpu(),
                functionality: s.function.to_gpu(),
                emit_material: s.emit_material.to_gpu(),
                emission_rate: s.emission_rate,
                emission_speed: s.emission_speed,
                padding: 0.0,
            }
        })
        .collect();

    // Mouse position in sim space (track prev cursor in a Local for velocity).
    let cursor = window.cursor_position().unwrap_or_default();
    let prev = prev_cursor.unwrap_or(cursor);
    *prev_cursor = Some(cursor);

    let mouse_pos_sim = [
        grid_size[0] as f32 * (cursor.x / res_w),
        grid_size[1] as f32 * (1.0 - cursor.y / res_h),
    ];
    let prev_pos_sim = [
        grid_size[0] as f32 * (prev.x / res_w),
        grid_size[1] as f32 * (1.0 - prev.y / res_h),
    ];
    let dt = time_reg.last_render_timestep_secs() as f32;
    let mouse_vel = if dt > 0.0 {
        [
            (mouse_pos_sim[0] - prev_pos_sim[0]) / dt,
            (mouse_pos_sim[1] - prev_pos_sim[1]) / dt,
        ]
    } else {
        [0.0, 0.0]
    };

    let bukkit_count_x = (grid_size[0] as f32 / BUKKIT_SIZE as f32).ceil() as u32;
    let bukkit_count_y = (grid_size[1] as f32 / BUKKIT_SIZE as f32).ceil() as u32;

    source.0 = ExtractedSimData {
        params: params.clone(),
        shapes: gpu_shapes,
        grid_size,
        resolution,
        do_reset,
        is_paused: sim_state.is_paused,
        substep_count,
        substep_index: sim_state.substep_index,
        mouse_down: mouse_buttons.pressed(MouseButton::Left) && interaction.dragging.is_none(),
        mouse_position: mouse_pos_sim,
        mouse_velocity: mouse_vel,
        bukkit_count_x,
        bukkit_count_y,
        particle_count: particle_count.clone(),
    };

    // Advance substep index (reset already handled above before extraction)
    if !sim_state.is_paused {
        sim_state.substep_index += substep_count;
    }
    reset_pending.0 = false;
}

fn extract_sim_data(mut commands: Commands, source: Extract<Res<ExtractedSimDataSource>>) {
    commands.insert_resource(source.0.clone());
}
