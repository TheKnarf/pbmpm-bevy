pub mod gpu_state;
pub mod node;
pub mod shaders;

use bevy::prelude::*;
use bevy::render::render_graph::{RenderGraph, RenderLabel};
use bevy::render::{Extract, RenderApp};

use crate::*;
use node::*;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct PbmpmNodeLabel;

/// Tracks whether a reset has been requested this frame.
#[derive(Resource, Default)]
struct ResetPending(bool);

pub struct PbmpmPlugin;

impl Plugin for PbmpmPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SimParams>();
        app.init_resource::<SimState>();
        app.init_resource::<TimeRegulation>();
        app.init_resource::<ParticleCount>();
        app.init_resource::<SimInteraction>();
        app.init_resource::<SimViewport>();
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
    interaction: Res<SimInteraction>,
    viewport: Res<SimViewport>,
    particle_count: Res<ParticleCount>,
    mut reset_pending: ResMut<ResetPending>,
    mut time_reg: ResMut<TimeRegulation>,
    time: Res<Time>,
    mut source: ResMut<ExtractedSimDataSource>,
    shape_query: Query<&SimShapeData>,
) {
    let res_w = viewport.resolution.x.max(1.0);
    let res_h = viewport.resolution.y.max(1.0);

    // Compute grid size from host-provided viewport.
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
    let half_viewport = Vec2::new(res_w * 0.5, res_h * 0.5);

    // Convert a world-space position (Bevy convention: origin center, Y up,
    // units = pixels at zoom 1) into sim grid coordinates (origin bottom-left,
    // Y up, units = grid cells).
    let world_to_sim = |world: Vec2| -> [f32; 2] {
        let pixel = world + half_viewport;
        [pixel.x * render_to_sim_scale, pixel.y * render_to_sim_scale]
    };

    // Convert shapes to GPU format
    let gpu_shapes: Vec<SimShapeGpu> = shape_query
        .iter()
        .map(|s| {
            let is_circle = s.shape_type.is_circle();
            let hs = s.half_size * render_to_sim_scale;
            SimShapeGpu {
                position: world_to_sim(s.position),
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
        interaction: SimInteractionSnapshot {
            active: interaction.active,
            mode: interaction.mode,
            position: world_to_sim(interaction.position),
            velocity: (interaction.velocity * render_to_sim_scale).to_array(),
            radius: interaction.radius * render_to_sim_scale,
        },
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
