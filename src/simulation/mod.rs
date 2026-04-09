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

pub struct PbmpmPlugin;

impl Plugin for PbmpmPlugin {
    fn build(&self, app: &mut App) {
        // Main world: compute extracted data each frame
        app.init_resource::<ExtractedSimDataSource>();

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

/// Source data prepared in main world, ready for extraction.
#[derive(Resource, Default, Clone)]
struct ExtractedSimDataSource(ExtractedSimData);

#[allow(clippy::too_many_arguments)]
fn prepare_extracted_data(
    params: Res<SimParams>,
    mut sim_state: ResMut<SimState>,
    input: Res<InputState>,
    particle_count: Res<ParticleCount>,
    mut time_reg: ResMut<TimeRegulation>,
    time: Res<Time>,
    windows: Query<&Window>,
    mut source: ResMut<ExtractedSimDataSource>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let res_w = window.width();
    let res_h = window.height();

    // Update grid size based on current resolution and divisor
    sim_state.grid_size = [
        (res_w / params.sim_res_divisor as f32).max(1.0) as u32,
        (res_h / params.sim_res_divisor as f32).max(1.0) as u32,
    ];
    sim_state.resolution = [res_w, res_h];

    // Reset substep index before computing substeps (matches original JS behavior)
    if sim_state.do_reset {
        sim_state.substep_index = 0;
    }

    // Compute substeps
    let current_time_ms = time.elapsed_secs_f64() * 1000.0;
    let substep_count = time_reg.compute_substeps(
        current_time_ms,
        params.sim_rate,
        sim_state.is_paused,
        sim_state.do_reset,
    );

    let render_to_sim_scale = 1.0 / params.sim_res_divisor as f32;

    // Convert shapes to GPU format
    let gpu_shapes: Vec<SimShapeGpu> = sim_state
        .shapes
        .iter()
        .map(|s| {
            let mut pos = [
                s.position.x as f32 * render_to_sim_scale,
                s.position.y as f32 * render_to_sim_scale,
            ];
            // Flip Y
            pos[1] = sim_state.grid_size[1] as f32 - pos[1];

            let shape_type = s.shape.as_f32();
            if shape_type == 1.0 {
                // Circle
                SimShapeGpu {
                    position: pos,
                    half_size: [0.0, 0.0],
                    radius: s.radius * render_to_sim_scale,
                    rotation: s.rotation,
                    shape_type,
                    functionality: s.function.as_f32(),
                    emit_material: s.emit_material.as_f32(),
                    emission_rate: s.emission_rate.as_f32(),
                    emission_speed: s.emission_speed.as_f32(),
                    padding: 0.0,
                }
            } else {
                // Box
                let hs = s.half_size.as_vec2() * render_to_sim_scale;
                SimShapeGpu {
                    position: pos,
                    half_size: [hs.x, hs.y],
                    radius: 0.0,
                    rotation: s.rotation,
                    shape_type,
                    functionality: s.function.as_f32(),
                    emit_material: s.emit_material.as_f32(),
                    emission_rate: s.emission_rate.as_f32(),
                    emission_speed: s.emission_speed.as_f32(),
                    padding: 0.0,
                }
            }
        })
        .collect();

    // Mouse position in sim space
    let mouse_pos_sim = [
        sim_state.grid_size[0] as f32 * (input.mouse_position.x / res_w),
        sim_state.grid_size[1] as f32 * (1.0 - input.mouse_position.y / res_h),
    ];
    let prev_pos_sim = [
        sim_state.grid_size[0] as f32 * (input.mouse_prev_position.x / res_w),
        sim_state.grid_size[1] as f32 * (1.0 - input.mouse_prev_position.y / res_h),
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

    let bukkit_count_x = (sim_state.grid_size[0] as f32 / BUKKIT_SIZE as f32).ceil() as u32;
    let bukkit_count_y = (sim_state.grid_size[1] as f32 / BUKKIT_SIZE as f32).ceil() as u32;

    source.0 = ExtractedSimData {
        params: params.clone(),
        shapes: gpu_shapes,
        grid_size: sim_state.grid_size,
        resolution: sim_state.resolution,
        do_reset: sim_state.do_reset,
        is_paused: sim_state.is_paused,
        substep_count,
        substep_index: sim_state.substep_index,
        mouse_down: input.mouse_down,
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
    sim_state.do_reset = false;
}

fn extract_sim_data(mut commands: Commands, source: Extract<Res<ExtractedSimDataSource>>) {
    commands.insert_resource(source.0.clone());
}
