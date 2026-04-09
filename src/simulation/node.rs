use std::cell::UnsafeCell;

use bevy::prelude::*;
use bevy::render::render_graph::{Node, NodeRunError, RenderGraphContext};
use bevy::render::render_resource::*;
use bevy::render::renderer::{RenderContext, RenderDevice};
use bevy::render::view::ViewTarget;

use bytemuck::Zeroable;

use super::gpu_state::*;
use crate::types::*;

/// Extracted simulation data, copied from main world each frame.
#[derive(Resource, Clone)]
pub struct ExtractedSimData {
    pub params: SimParams,
    pub shapes: Vec<SimShapeGpu>,
    pub grid_size: [u32; 2],
    pub resolution: [f32; 2],
    pub do_reset: bool,
    pub is_paused: bool,
    pub substep_count: u32,
    pub substep_index: u32,
    pub mouse_down: bool,
    pub mouse_position: [f32; 2],
    pub mouse_velocity: [f32; 2],
    pub bukkit_count_x: u32,
    pub bukkit_count_y: u32,
}

impl Default for ExtractedSimData {
    fn default() -> Self {
        Self {
            params: SimParams::default(),
            shapes: Vec::new(),
            grid_size: [128, 72],
            resolution: [1024.0, 576.0],
            do_reset: true,
            is_paused: false,
            substep_count: 0,
            substep_index: 0,
            mouse_down: false,
            mouse_position: [0.0; 2],
            mouse_velocity: [0.0; 2],
            bukkit_count_x: 0,
            bukkit_count_y: 0,
        }
    }
}

/// Inner mutable state for the node, wrapped in UnsafeCell for interior mutability.
struct PbmpmNodeInner {
    pipelines: Option<PbmpmPipelines>,
    pipeline_format: Option<TextureFormat>,
    state: GpuSimState,
    buffer_idx: u32,
    view_target_query: Option<QueryState<&'static ViewTarget>>,
}

/// The main render graph node that drives simulation compute + particle rendering.
/// Uses UnsafeCell because Node::run takes &self but we need mutation.
/// Safety: render graph nodes execute single-threaded.
pub struct PbmpmNode {
    inner: UnsafeCell<PbmpmNodeInner>,
}

// Safety: render graph guarantees single-threaded access to nodes
unsafe impl Send for PbmpmNode {}
unsafe impl Sync for PbmpmNode {}

impl Default for PbmpmNode {
    fn default() -> Self {
        Self {
            inner: UnsafeCell::new(PbmpmNodeInner {
                pipelines: None,
                pipeline_format: None,
                state: GpuSimState::default(),
                buffer_idx: 0,
                view_target_query: None,
            }),
        }
    }
}

impl PbmpmNodeInner {
    fn ensure_pipelines(&mut self, device: &RenderDevice, format: TextureFormat) {
        if self.pipelines.is_none() || self.pipeline_format != Some(format) {
            info!("Creating PB-MPM GPU pipelines for format {:?}...", format);
            self.pipelines = Some(PbmpmPipelines::new(device, format));
            self.pipeline_format = Some(format);
        }
    }

    fn build_sim_constants(
        &self,
        data: &ExtractedSimData,
        iteration: u32,
        substep_idx: u32,
    ) -> SimConstantsGpu {
        let params = &data.params;
        let mouse_activation = if data.mouse_down {
            500.0 / params.sim_rate as f32 * (data.grid_size[0] as f32 / 128.0)
        } else {
            0.0
        };

        SimConstantsGpu {
            grid_size: data.grid_size,
            delta_time: 1.0 / params.sim_rate as f32,
            mouse_activation,
            mouse_position: data.mouse_position,
            mouse_velocity: data.mouse_velocity,
            mouse_function: params.mouse_function as u32 as f32,
            elasticity_ratio: params.elasticity_ratio,
            gravity_strength: params.gravity_strength,
            liquid_relaxation: params.liquid_relaxation,
            elastic_relaxation: params.elastic_relaxation,
            liquid_viscosity: params.liquid_viscosity,
            fixed_point_multiplier: 10u32.pow(params.fixed_point_multiplier_exponent),
            use_grid_volume_for_liquid: params.use_grid_volume_for_liquid as u32,
            particles_per_cell_axis: params.particles_per_cell_axis,
            friction_angle: params.friction_angle,
            plasticity: params.plasticity,
            mouse_radius: params.mouse_radius / params.sim_res_divisor as f32,
            shape_count: data.shapes.len() as u32,
            sim_frame: substep_idx,
            bukkit_count: data.bukkit_count_x * data.bukkit_count_y,
            bukkit_count_x: data.bukkit_count_x,
            bukkit_count_y: data.bukkit_count_y,
            iteration,
            iteration_count: params.iteration_count,
            border_friction: params.border_friction,
        }
    }

    fn build_render_constants(&self, data: &ExtractedSimData) -> RenderConstantsGpu {
        let view_pos = [
            data.grid_size[0] as f32 / 2.0,
            data.grid_size[1] as f32 / 2.0,
        ];
        RenderConstantsGpu {
            particle_radius_timestamp: [0.5, 0.0],
            canvas_size: data.resolution,
            view_pos,
            view_extent: view_pos,
            render_mode: data.params.render_mode as u32 as f32,
            delta_time: 1.0 / data.params.sim_rate as f32,
            _pad0: 0.0,
            _pad1: 0.0,
        }
    }
}

impl Node for PbmpmNode {
    fn update(&mut self, world: &mut World) {
        // Safety: we have &mut self so exclusive access is guaranteed
        let inner = self.inner.get_mut();
        if inner.view_target_query.is_none() {
            inner.view_target_query = Some(world.query::<&ViewTarget>());
        }
    }

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        // Safety: render graph guarantees single-threaded node execution
        let this = unsafe { &mut *self.inner.get() };

        let data = match world.get_resource::<ExtractedSimData>() {
            Some(d) => d,
            None => return Ok(()),
        };

        // Get device from world resource to avoid borrow conflicts with render_context
        let device = world.resource::<RenderDevice>();

        // Get the surface format from the ViewTarget (camera's render texture)
        let (surface_format, view_texture) = {
            let query = this.view_target_query.as_mut();
            match query.and_then(|q| q.iter_manual(world).next()) {
                Some(vt) => (
                    vt.main_texture_format(),
                    Some(vt.main_texture_view().clone()),
                ),
                None => return Ok(()),
            }
        };

        this.ensure_pipelines(device, surface_format);
        let pipelines = this.pipelines.as_ref().unwrap();

        // Handle reset
        if data.do_reset || !this.state.initialized || this.state.grid_size != data.grid_size {
            this.state.initialize(device, data.grid_size);
            this.buffer_idx = 0;
        }

        let state = &this.state;

        // Build shape buffer
        let shapes = if data.shapes.is_empty() {
            vec![SimShapeGpu::zeroed()]
        } else {
            data.shapes.clone()
        };
        let shape_buffer = device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("shapes"),
            contents: bytemuck::cast_slice(&shapes),
            usage: BufferUsages::STORAGE,
        });

        let encoder = render_context.command_encoder();

        // Clear grid buffers at start of frame
        for buf in state.grid_buffers.iter().flatten() {
            encoder.clear_buffer(buf, 0, None);
        }

        // Simulation substeps
        let substep_count = if data.is_paused {
            0
        } else {
            data.substep_count
        };
        let mut substep_index = data.substep_index;

        for _substep in 0..substep_count {
            for iteration in 0..data.params.iteration_count {
                let sim_constants = this.build_sim_constants(data, iteration, substep_index);
                let sim_uniform = device.create_buffer_with_data(&BufferInitDescriptor {
                    label: Some("sim_constants"),
                    contents: bytemuck::cast_slice(&[sim_constants]),
                    usage: BufferUsages::UNIFORM,
                });

                let current_grid = state.grid_buffers[this.buffer_idx as usize]
                    .as_ref()
                    .unwrap();
                let next_grid = state.grid_buffers[((this.buffer_idx + 1) % 3) as usize]
                    .as_ref()
                    .unwrap();
                let next_next_grid = state.grid_buffers[((this.buffer_idx + 2) % 3) as usize]
                    .as_ref()
                    .unwrap();
                this.buffer_idx = (this.buffer_idx + 1) % 3;

                // G2P2G dispatch
                let bind_group = device.create_bind_group(
                    "g2p2g",
                    &BindGroupLayout::from(pipelines.g2p2g.get_bind_group_layout(0)),
                    &[
                        BindGroupEntry {
                            binding: 0,
                            resource: sim_uniform.as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: state.particle_buffer.as_ref().unwrap().as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 2,
                            resource: current_grid.as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 3,
                            resource: next_grid.as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 4,
                            resource: next_next_grid.as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 5,
                            resource: state
                                .bukkit_thread_data
                                .as_ref()
                                .unwrap()
                                .as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 6,
                            resource: state
                                .bukkit_particle_data
                                .as_ref()
                                .unwrap()
                                .as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 7,
                            resource: shape_buffer.as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 8,
                            resource: state
                                .particle_free_indices_buffer
                                .as_ref()
                                .unwrap()
                                .as_entire_binding(),
                        },
                    ],
                );

                {
                    let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                        label: Some("g2p2g"),
                        timestamp_writes: None,
                    });
                    pass.set_pipeline(&pipelines.g2p2g);
                    pass.set_bind_group(0, &bind_group, &[]);
                    pass.dispatch_workgroups_indirect(state.bukkit_dispatch.as_ref().unwrap(), 0);
                }
            }

            // Emission
            {
                let sim_constants = this.build_sim_constants(data, 0, substep_index);
                let sim_uniform = device.create_buffer_with_data(&BufferInitDescriptor {
                    label: Some("sim_constants_emit"),
                    contents: bytemuck::cast_slice(&[sim_constants]),
                    usage: BufferUsages::UNIFORM,
                });

                let grid = state.grid_buffers[this.buffer_idx as usize]
                    .as_ref()
                    .unwrap();

                // Particle emit
                let emit_bg = device.create_bind_group(
                    "particle_emit",
                    &BindGroupLayout::from(pipelines.particle_emit.get_bind_group_layout(0)),
                    &[
                        BindGroupEntry {
                            binding: 0,
                            resource: sim_uniform.as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: state
                                .particle_count_buffer
                                .as_ref()
                                .unwrap()
                                .as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 2,
                            resource: state.particle_buffer.as_ref().unwrap().as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 3,
                            resource: shape_buffer.as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 4,
                            resource: state
                                .particle_free_indices_buffer
                                .as_ref()
                                .unwrap()
                                .as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 5,
                            resource: grid.as_entire_binding(),
                        },
                    ],
                );

                let grid_dispatch_x = div_up(data.grid_size[0], GRID_DISPATCH_SIZE);
                let grid_dispatch_y = div_up(data.grid_size[1], GRID_DISPATCH_SIZE);

                {
                    let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                        label: Some("particle_emit"),
                        timestamp_writes: None,
                    });
                    pass.set_pipeline(&pipelines.particle_emit);
                    pass.set_bind_group(0, &emit_bg, &[]);
                    pass.dispatch_workgroups(grid_dispatch_x, grid_dispatch_y, 1);
                }

                // Set indirect args
                let args_bg = device.create_bind_group(
                    "set_indirect_args",
                    &BindGroupLayout::from(pipelines.set_indirect_args.get_bind_group_layout(0)),
                    &[
                        BindGroupEntry {
                            binding: 0,
                            resource: state
                                .particle_count_buffer
                                .as_ref()
                                .unwrap()
                                .as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: state
                                .particle_sim_dispatch_buffer
                                .as_ref()
                                .unwrap()
                                .as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 2,
                            resource: state
                                .particle_render_dispatch_buffer
                                .as_ref()
                                .unwrap()
                                .as_entire_binding(),
                        },
                    ],
                );
                {
                    let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                        label: Some("set_indirect_args"),
                        timestamp_writes: None,
                    });
                    pass.set_pipeline(&pipelines.set_indirect_args);
                    pass.set_bind_group(0, &args_bg, &[]);
                    pass.dispatch_workgroups(1, 1, 1);
                }
            }

            // Bukkitize particles
            {
                let sim_constants = this.build_sim_constants(data, 0, substep_index);
                let sim_uniform = device.create_buffer_with_data(&BufferInitDescriptor {
                    label: Some("sim_constants_bukkit"),
                    contents: bytemuck::cast_slice(&[sim_constants]),
                    usage: BufferUsages::UNIFORM,
                });

                // Clear bukkit buffers
                encoder.clear_buffer(state.bukkit_count_buffer.as_ref().unwrap(), 0, None);
                encoder.clear_buffer(state.bukkit_count_buffer2.as_ref().unwrap(), 0, None);
                encoder.clear_buffer(state.bukkit_thread_data.as_ref().unwrap(), 0, None);
                encoder.clear_buffer(state.bukkit_particle_data.as_ref().unwrap(), 0, None);
                encoder.clear_buffer(state.bukkit_particle_allocator.as_ref().unwrap(), 0, None);
                encoder.copy_buffer_to_buffer(
                    state.bukkit_blank_dispatch.as_ref().unwrap(),
                    0,
                    state.bukkit_dispatch.as_ref().unwrap(),
                    0,
                    state.bukkit_dispatch.as_ref().unwrap().size(),
                );

                // Bukkit count
                let bk_count_bg = device.create_bind_group(
                    "bukkit_count",
                    &BindGroupLayout::from(pipelines.bukkit_count.get_bind_group_layout(0)),
                    &[
                        BindGroupEntry {
                            binding: 0,
                            resource: sim_uniform.as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: state
                                .particle_count_buffer
                                .as_ref()
                                .unwrap()
                                .as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 2,
                            resource: state.particle_buffer.as_ref().unwrap().as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 3,
                            resource: state
                                .bukkit_count_buffer
                                .as_ref()
                                .unwrap()
                                .as_entire_binding(),
                        },
                    ],
                );
                {
                    let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                        label: Some("bukkit_count"),
                        timestamp_writes: None,
                    });
                    pass.set_pipeline(&pipelines.bukkit_count);
                    pass.set_bind_group(0, &bk_count_bg, &[]);
                    pass.dispatch_workgroups_indirect(
                        state.particle_sim_dispatch_buffer.as_ref().unwrap(),
                        0,
                    );
                }

                // Bukkit allocate
                let bk_alloc_bg = device.create_bind_group(
                    "bukkit_allocate",
                    &BindGroupLayout::from(pipelines.bukkit_allocate.get_bind_group_layout(0)),
                    &[
                        BindGroupEntry {
                            binding: 0,
                            resource: sim_uniform.as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: state
                                .bukkit_count_buffer
                                .as_ref()
                                .unwrap()
                                .as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 2,
                            resource: state.bukkit_dispatch.as_ref().unwrap().as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 3,
                            resource: state
                                .bukkit_thread_data
                                .as_ref()
                                .unwrap()
                                .as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 4,
                            resource: state
                                .bukkit_particle_allocator
                                .as_ref()
                                .unwrap()
                                .as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 5,
                            resource: state
                                .bukkit_index_start
                                .as_ref()
                                .unwrap()
                                .as_entire_binding(),
                        },
                    ],
                );
                {
                    let bk_dispatch_x = div_up(data.bukkit_count_x, GRID_DISPATCH_SIZE);
                    let bk_dispatch_y = div_up(data.bukkit_count_y, GRID_DISPATCH_SIZE);
                    let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                        label: Some("bukkit_allocate"),
                        timestamp_writes: None,
                    });
                    pass.set_pipeline(&pipelines.bukkit_allocate);
                    pass.set_bind_group(0, &bk_alloc_bg, &[]);
                    pass.dispatch_workgroups(bk_dispatch_x, bk_dispatch_y, 1);
                }

                // Bukkit insert
                let bk_insert_bg = device.create_bind_group(
                    "bukkit_insert",
                    &BindGroupLayout::from(pipelines.bukkit_insert.get_bind_group_layout(0)),
                    &[
                        BindGroupEntry {
                            binding: 0,
                            resource: sim_uniform.as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: state
                                .particle_count_buffer
                                .as_ref()
                                .unwrap()
                                .as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 2,
                            resource: state
                                .bukkit_count_buffer2
                                .as_ref()
                                .unwrap()
                                .as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 3,
                            resource: state.particle_buffer.as_ref().unwrap().as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 4,
                            resource: state
                                .bukkit_particle_data
                                .as_ref()
                                .unwrap()
                                .as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 5,
                            resource: state
                                .bukkit_index_start
                                .as_ref()
                                .unwrap()
                                .as_entire_binding(),
                        },
                    ],
                );
                {
                    let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                        label: Some("bukkit_insert"),
                        timestamp_writes: None,
                    });
                    pass.set_pipeline(&pipelines.bukkit_insert);
                    pass.set_bind_group(0, &bk_insert_bg, &[]);
                    pass.dispatch_workgroups_indirect(
                        state.particle_sim_dispatch_buffer.as_ref().unwrap(),
                        0,
                    );
                }
            }

            substep_index += 1;
        }

        // Render particles to the camera's ViewTarget texture
        if let Some(ref target_view) = view_texture {
            let render_constants = this.build_render_constants(data);
            let render_uniform = device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("render_constants"),
                contents: bytemuck::cast_slice(&[render_constants]),
                usage: BufferUsages::UNIFORM,
            });

            let render_bg = device.create_bind_group(
                "particle_render",
                &BindGroupLayout::from(pipelines.particle_render.get_bind_group_layout(0)),
                &[
                    BindGroupEntry {
                        binding: 0,
                        resource: render_uniform.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: state.particle_buffer.as_ref().unwrap().as_entire_binding(),
                    },
                ],
            );

            {
                let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("particle_render"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: target_view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Default::default()),
                            store: StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                pass.set_pipeline(&pipelines.particle_render);
                pass.set_bind_group(0, &render_bg, &[]);
                pass.draw_indirect(state.particle_render_dispatch_buffer.as_ref().unwrap(), 0);
            }
        }

        Ok(())
    }
}
