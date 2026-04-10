use std::sync::atomic::Ordering;
use std::sync::Mutex;

use bevy::prelude::*;
use bevy::render::render_graph::{Node, NodeRunError, RenderGraphContext};
use bevy::render::render_resource::*;
use bevy::render::renderer::{RenderContext, RenderDevice, RenderQueue};
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
    pub particle_count: ParticleCount,
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
            particle_count: ParticleCount::default(),
        }
    }
}

/// Inner mutable state for the node.
struct PbmpmNodeInner {
    pipelines: Option<PbmpmPipelines>,
    pipeline_format: Option<TextureFormat>,
    state: GpuSimState,
    buffer_idx: u32,
    view_target_query: Option<QueryState<&'static ViewTarget>>,
    readback_frame: u32,
}

/// The main render graph node that drives simulation compute + particle rendering.
/// Uses Mutex for interior mutability since Node::run takes &self. The render
/// graph executes nodes single-threaded so the mutex is always uncontended.
pub struct PbmpmNode {
    inner: Mutex<PbmpmNodeInner>,
}

impl Default for PbmpmNode {
    fn default() -> Self {
        Self {
            inner: Mutex::new(PbmpmNodeInner {
                pipelines: None,
                pipeline_format: None,
                state: GpuSimState::default(),
                buffer_idx: 0,
                view_target_query: None,
                readback_frame: 0,
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
        let inner = self.inner.get_mut().unwrap();
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
        let mut guard = self.inner.lock().unwrap();
        let this = &mut *guard;

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

        // Get the render queue for buffer writes (avoids per-dispatch buffer allocation).
        let queue = world.resource::<RenderQueue>();

        // Ensure the persistent shape buffer is large enough and write the current shapes.
        let shapes_owned: Vec<SimShapeGpu> = if data.shapes.is_empty() {
            vec![SimShapeGpu::zeroed()]
        } else {
            data.shapes.clone()
        };
        this.state.ensure_shape_buffer(
            device,
            shapes_owned.len() as u32,
            std::mem::size_of::<SimShapeGpu>() as u64,
        );
        queue.write_buffer(
            this.state.shape_buf(),
            0,
            bytemuck::cast_slice(&shapes_owned),
        );

        // Pre-allocate enough uniform buffers for this frame's substeps.
        // Each substep needs: iteration_count (g2p2g) + 4 (emit, bukkit_count, bukkit_allocate, bukkit_insert)
        let substep_count = if data.is_paused {
            0
        } else {
            data.substep_count
        };
        let uniforms_per_substep = data.params.iteration_count as usize + 4;
        let total_uniforms = (substep_count as usize * uniforms_per_substep).max(1);
        let uniform_size = std::mem::size_of::<SimConstantsGpu>() as u64;
        this.state
            .ensure_uniform_pool(device, total_uniforms, uniform_size);

        let mut uniform_idx: usize = 0;
        let state = &this.state;
        let encoder = render_context.command_encoder();

        // Clear grid buffers at start of frame
        for buf in state.grid_buffers.iter().flatten() {
            encoder.clear_buffer(buf, 0, None);
        }

        // Simulation substeps
        let mut substep_index = data.substep_index;

        for _substep in 0..substep_count {
            // Build the base sim constants for this substep once; only the
            // `iteration` field varies per dispatch within the substep.
            let mut sim_constants = this.build_sim_constants(data, 0, substep_index);

            for iteration in 0..data.params.iteration_count {
                sim_constants.iteration = iteration;
                let sim_uniform = state.uniform(uniform_idx);
                queue.write_buffer(sim_uniform, 0, bytemuck::cast_slice(&[sim_constants]));
                uniform_idx += 1;

                let current_grid = state.grid(this.buffer_idx as usize);
                let next_grid = state.grid(((this.buffer_idx + 1) % 3) as usize);
                let next_next_grid = state.grid(((this.buffer_idx + 2) % 3) as usize);
                this.buffer_idx = (this.buffer_idx + 1) % 3;

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
                            resource: state.particles().as_entire_binding(),
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
                            resource: state.bukkit_thread_data_buf().as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 6,
                            resource: state.bukkit_particle_data_buf().as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 7,
                            resource: state.shape_buf().as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 8,
                            resource: state.free_indices().as_entire_binding(),
                        },
                    ],
                );

                let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                    label: Some("g2p2g"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&pipelines.g2p2g);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.dispatch_workgroups_indirect(state.bukkit_dispatch_buf(), 0);
            }

            // Emission - reuses the same sim_constants (iteration is irrelevant here).
            sim_constants.iteration = 0;
            let emit_uniform = state.uniform(uniform_idx);
            queue.write_buffer(emit_uniform, 0, bytemuck::cast_slice(&[sim_constants]));
            uniform_idx += 1;

            let grid = state.grid(this.buffer_idx as usize);

            let emit_bg = device.create_bind_group(
                "particle_emit",
                &BindGroupLayout::from(pipelines.particle_emit.get_bind_group_layout(0)),
                &[
                    BindGroupEntry {
                        binding: 0,
                        resource: emit_uniform.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: state.particle_count().as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: state.particles().as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 3,
                        resource: state.shape_buf().as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 4,
                        resource: state.free_indices().as_entire_binding(),
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

            // set_indirect_args doesn't use sim_constants - bind directly.
            let args_bg = device.create_bind_group(
                "set_indirect_args",
                &BindGroupLayout::from(pipelines.set_indirect_args.get_bind_group_layout(0)),
                &[
                    BindGroupEntry {
                        binding: 0,
                        resource: state.particle_count().as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: state.sim_dispatch().as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: state.render_dispatch().as_entire_binding(),
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

            // Bukkitize particles
            // Clear bukkit buffers
            encoder.clear_buffer(state.bukkit_count_buf(), 0, None);
            encoder.clear_buffer(state.bukkit_count_buf2(), 0, None);
            encoder.clear_buffer(state.bukkit_thread_data_buf(), 0, None);
            encoder.clear_buffer(state.bukkit_particle_data_buf(), 0, None);
            encoder.clear_buffer(state.bukkit_particle_allocator_buf(), 0, None);
            encoder.copy_buffer_to_buffer(
                state.bukkit_blank_dispatch_buf(),
                0,
                state.bukkit_dispatch_buf(),
                0,
                state.bukkit_dispatch_buf().size(),
            );

            // Each bukkit dispatch needs its own uniform slot since the writes
            // accumulate before submit.
            let bk_count_uniform = state.uniform(uniform_idx);
            queue.write_buffer(bk_count_uniform, 0, bytemuck::cast_slice(&[sim_constants]));
            uniform_idx += 1;

            let bk_count_bg = device.create_bind_group(
                "bukkit_count",
                &BindGroupLayout::from(pipelines.bukkit_count.get_bind_group_layout(0)),
                &[
                    BindGroupEntry {
                        binding: 0,
                        resource: bk_count_uniform.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: state.particle_count().as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: state.particles().as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 3,
                        resource: state.bukkit_count_buf().as_entire_binding(),
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
                pass.dispatch_workgroups_indirect(state.sim_dispatch(), 0);
            }

            let bk_alloc_uniform = state.uniform(uniform_idx);
            queue.write_buffer(bk_alloc_uniform, 0, bytemuck::cast_slice(&[sim_constants]));
            uniform_idx += 1;

            let bk_alloc_bg = device.create_bind_group(
                "bukkit_allocate",
                &BindGroupLayout::from(pipelines.bukkit_allocate.get_bind_group_layout(0)),
                &[
                    BindGroupEntry {
                        binding: 0,
                        resource: bk_alloc_uniform.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: state.bukkit_count_buf().as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: state.bukkit_dispatch_buf().as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 3,
                        resource: state.bukkit_thread_data_buf().as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 4,
                        resource: state.bukkit_particle_allocator_buf().as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 5,
                        resource: state.bukkit_index_start_buf().as_entire_binding(),
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

            let bk_insert_uniform = state.uniform(uniform_idx);
            queue.write_buffer(bk_insert_uniform, 0, bytemuck::cast_slice(&[sim_constants]));
            uniform_idx += 1;

            let bk_insert_bg = device.create_bind_group(
                "bukkit_insert",
                &BindGroupLayout::from(pipelines.bukkit_insert.get_bind_group_layout(0)),
                &[
                    BindGroupEntry {
                        binding: 0,
                        resource: bk_insert_uniform.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: state.particle_count().as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: state.bukkit_count_buf2().as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 3,
                        resource: state.particles().as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 4,
                        resource: state.bukkit_particle_data_buf().as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 5,
                        resource: state.bukkit_index_start_buf().as_entire_binding(),
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
                pass.dispatch_workgroups_indirect(state.sim_dispatch(), 0);
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
                        resource: state.particles().as_entire_binding(),
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
                pass.draw_indirect(state.render_dispatch(), 0);
            }
        }

        // Readback particle count using two-phase approach:
        // Even frames: map staging buffer (contains data from previous copy), read, unmap
        // Odd frames: copy particle count to staging buffer
        // This ensures we never copy to a mapped buffer.
        this.readback_frame += 1;
        if this.readback_frame.is_multiple_of(60) {
            // Phase 1: read from staging (data from a previous copy)
            let particle_count = data.particle_count.clone();
            let staging = state.particle_count_staging_buf().clone();
            let staging2 = staging.clone();
            device.map_buffer(&staging.slice(..), MapMode::Read, move |result| {
                if result.is_ok() {
                    let mapped = staging2.slice(..).get_mapped_range();
                    if mapped.len() >= 4 {
                        let count =
                            u32::from_le_bytes([mapped[0], mapped[1], mapped[2], mapped[3]]);
                        particle_count
                            .0
                            .store(count.min(MAX_PARTICLE_COUNT), Ordering::Relaxed);
                    }
                    drop(mapped);
                    staging2.unmap();
                }
            });
            // Poll to complete synchronously
            let _ = device.wgpu_device().poll(PollType::Wait {
                submission_index: None,
                timeout: None,
            });
        } else if this.readback_frame % 60 == 30 {
            // Phase 2: copy particle count to staging (will be read in phase 1 later)
            encoder.copy_buffer_to_buffer(
                state.particle_count(),
                0,
                state.particle_count_staging_buf(),
                0,
                16,
            );
        }

        Ok(())
    }
}
