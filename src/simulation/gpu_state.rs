use bevy::render::render_resource::*;
use bevy::render::renderer::RenderDevice;

use crate::types::*;

/// Holds all persistent GPU buffers for the simulation.
#[derive(Default)]
pub struct GpuSimState {
    pub initialized: bool,
    pub grid_size: [u32; 2],

    // Particle buffers
    pub particle_buffer: Option<Buffer>,
    pub particle_count_buffer: Option<Buffer>,
    pub particle_sim_dispatch_buffer: Option<Buffer>,
    pub particle_render_dispatch_buffer: Option<Buffer>,
    pub particle_free_indices_buffer: Option<Buffer>,

    // Grid buffers (3 rotating)
    pub grid_buffers: [Option<Buffer>; 3],

    // Bukkit system
    pub bukkit_count_buffer: Option<Buffer>,
    pub bukkit_count_buffer2: Option<Buffer>,
    pub bukkit_thread_data: Option<Buffer>,
    pub bukkit_particle_data: Option<Buffer>,
    pub bukkit_dispatch: Option<Buffer>,
    pub bukkit_blank_dispatch: Option<Buffer>,
    pub bukkit_particle_allocator: Option<Buffer>,
    pub bukkit_index_start: Option<Buffer>,

    pub bukkit_count_x: u32,
    pub bukkit_count_y: u32,

    // Staging buffer for particle count readback
    pub particle_count_staging: Option<Buffer>,

    // Pool of persistent uniform buffers, one per dispatch in a frame.
    // Reused across frames; grows as needed.
    pub uniform_pool: Vec<Buffer>,

    // Persistent shape storage buffer (resized as the shape count changes).
    pub shape_buffer: Option<Buffer>,
    pub shape_buffer_capacity: u32,
}

impl GpuSimState {
    pub fn initialize(&mut self, device: &RenderDevice, grid_size: [u32; 2]) {
        self.grid_size = grid_size;

        let bukkit_count_x = (grid_size[0] as f32 / BUKKIT_SIZE as f32).ceil() as u32;
        let bukkit_count_y = (grid_size[1] as f32 / BUKKIT_SIZE as f32).ceil() as u32;
        self.bukkit_count_x = bukkit_count_x;
        self.bukkit_count_y = bukkit_count_y;

        self.particle_buffer = Some(device.create_buffer(&BufferDescriptor {
            label: Some("particles"),
            size: (MAX_PARTICLE_COUNT as u64) * (PARTICLE_FLOAT_COUNT as u64) * 4,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        }));

        self.particle_count_buffer = Some(create_4u32_buffer(
            device,
            "particle_count",
            &[0, 0, 0, 0],
            BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
        ));

        self.particle_sim_dispatch_buffer = Some(create_4u32_buffer(
            device,
            "sim_dispatch",
            &[0, 1, 1, 0],
            BufferUsages::STORAGE | BufferUsages::INDIRECT | BufferUsages::COPY_DST,
        ));

        self.particle_render_dispatch_buffer = Some(create_4u32_buffer(
            device,
            "render_dispatch",
            &[6, 0, 0, 0],
            BufferUsages::STORAGE | BufferUsages::INDIRECT | BufferUsages::COPY_DST,
        ));

        self.particle_free_indices_buffer = Some(device.create_buffer(&BufferDescriptor {
            label: Some("free_indices"),
            size: 4 + (MAX_PARTICLE_COUNT as u64) * 4,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        }));

        let grid_buffer_size = (grid_size[0] as u64) * (grid_size[1] as u64) * 4 * 4;
        for i in 0..3 {
            self.grid_buffers[i] = Some(device.create_buffer(&BufferDescriptor {
                label: Some(&format!("grid_{}", i)),
                size: grid_buffer_size,
                usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
        }

        let bukkit_count = bukkit_count_x * bukkit_count_y;

        self.bukkit_count_buffer = Some(device.create_buffer(&BufferDescriptor {
            label: Some("bukkit_count"),
            size: (bukkit_count as u64) * 4,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));

        self.bukkit_count_buffer2 = Some(device.create_buffer(&BufferDescriptor {
            label: Some("bukkit_count2"),
            size: (bukkit_count as u64) * 4,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));

        self.bukkit_thread_data = Some(device.create_buffer(&BufferDescriptor {
            label: Some("bukkit_thread_data"),
            size: 10 * (bukkit_count as u64) * 4 * 4,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));

        self.bukkit_particle_data = Some(device.create_buffer(&BufferDescriptor {
            label: Some("bukkit_particle_data"),
            size: (MAX_PARTICLE_COUNT as u64) * 4,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));

        self.bukkit_dispatch = Some(create_4u32_buffer(
            device,
            "bukkit_dispatch",
            &[0, 1, 1, 0],
            BufferUsages::STORAGE | BufferUsages::INDIRECT | BufferUsages::COPY_DST,
        ));

        self.bukkit_blank_dispatch = Some(create_4u32_buffer(
            device,
            "bukkit_blank_dispatch",
            &[0, 1, 1, 0],
            BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
        ));

        self.bukkit_particle_allocator = Some(create_4u32_buffer(
            device,
            "bukkit_allocator",
            &[0, 0, 0, 0],
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
        ));

        self.bukkit_index_start = Some(device.create_buffer(&BufferDescriptor {
            label: Some("bukkit_index_start"),
            size: (bukkit_count as u64) * 4,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        }));

        self.particle_count_staging = Some(create_4u32_buffer(
            device,
            "particle_count_staging",
            &[0, 0, 0, 0],
            BufferUsages::MAP_READ | BufferUsages::COPY_DST,
        ));

        self.initialized = true;
    }

    // Accessors that unwrap the Option<Buffer> fields. After `initialize` they
    // are always Some, so unwrapping is safe in all consumers.
    #[inline]
    pub fn particles(&self) -> &Buffer {
        self.particle_buffer.as_ref().unwrap()
    }
    #[inline]
    pub fn particle_count(&self) -> &Buffer {
        self.particle_count_buffer.as_ref().unwrap()
    }
    #[inline]
    pub fn sim_dispatch(&self) -> &Buffer {
        self.particle_sim_dispatch_buffer.as_ref().unwrap()
    }
    #[inline]
    pub fn render_dispatch(&self) -> &Buffer {
        self.particle_render_dispatch_buffer.as_ref().unwrap()
    }
    #[inline]
    pub fn free_indices(&self) -> &Buffer {
        self.particle_free_indices_buffer.as_ref().unwrap()
    }
    #[inline]
    pub fn grid(&self, idx: usize) -> &Buffer {
        self.grid_buffers[idx].as_ref().unwrap()
    }
    #[inline]
    pub fn bukkit_count_buf(&self) -> &Buffer {
        self.bukkit_count_buffer.as_ref().unwrap()
    }
    #[inline]
    pub fn bukkit_count_buf2(&self) -> &Buffer {
        self.bukkit_count_buffer2.as_ref().unwrap()
    }
    #[inline]
    pub fn bukkit_thread_data_buf(&self) -> &Buffer {
        self.bukkit_thread_data.as_ref().unwrap()
    }
    #[inline]
    pub fn bukkit_particle_data_buf(&self) -> &Buffer {
        self.bukkit_particle_data.as_ref().unwrap()
    }
    #[inline]
    pub fn bukkit_dispatch_buf(&self) -> &Buffer {
        self.bukkit_dispatch.as_ref().unwrap()
    }
    #[inline]
    pub fn bukkit_blank_dispatch_buf(&self) -> &Buffer {
        self.bukkit_blank_dispatch.as_ref().unwrap()
    }
    #[inline]
    pub fn bukkit_particle_allocator_buf(&self) -> &Buffer {
        self.bukkit_particle_allocator.as_ref().unwrap()
    }
    #[inline]
    pub fn bukkit_index_start_buf(&self) -> &Buffer {
        self.bukkit_index_start.as_ref().unwrap()
    }
    #[inline]
    pub fn particle_count_staging_buf(&self) -> &Buffer {
        self.particle_count_staging.as_ref().unwrap()
    }

    /// Ensure the uniform pool has at least `count` buffers (grow if needed).
    pub fn ensure_uniform_pool(&mut self, device: &RenderDevice, count: usize, size: u64) {
        while self.uniform_pool.len() < count {
            self.uniform_pool
                .push(device.create_buffer(&BufferDescriptor {
                    label: Some("sim_uniform_pool"),
                    size,
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }));
        }
    }

    #[inline]
    pub fn uniform(&self, idx: usize) -> &Buffer {
        &self.uniform_pool[idx]
    }

    /// Ensure the persistent shape buffer is large enough for `shape_count` shapes.
    /// Reallocates only when the count exceeds current capacity.
    pub fn ensure_shape_buffer(&mut self, device: &RenderDevice, shape_count: u32, stride: u64) {
        let needed = shape_count.max(1);
        if self.shape_buffer.is_none() || needed > self.shape_buffer_capacity {
            // Grow with some headroom to avoid frequent reallocation.
            let new_capacity = needed.next_power_of_two().max(8);
            self.shape_buffer = Some(device.create_buffer(&BufferDescriptor {
                label: Some("shapes"),
                size: stride * new_capacity as u64,
                usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.shape_buffer_capacity = new_capacity;
        }
    }

    #[inline]
    pub fn shape_buf(&self) -> &Buffer {
        self.shape_buffer.as_ref().unwrap()
    }
}

fn create_4u32_buffer(
    device: &RenderDevice,
    label: &str,
    values: &[u32; 4],
    usage: BufferUsages,
) -> Buffer {
    device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some(label),
        contents: bytemuck::cast_slice(values),
        usage,
    })
}

/// Holds all compute and render pipelines.
pub struct PbmpmPipelines {
    pub g2p2g: ComputePipeline,
    pub particle_emit: ComputePipeline,
    pub bukkit_count: ComputePipeline,
    pub bukkit_allocate: ComputePipeline,
    pub bukkit_insert: ComputePipeline,
    pub set_indirect_args: ComputePipeline,
    pub particle_render: RenderPipeline,
}

impl PbmpmPipelines {
    pub fn new(device: &RenderDevice, surface_format: TextureFormat) -> Self {
        use super::shaders;

        let create_compute = |label: &str, source: &str| -> ComputePipeline {
            let module = device.create_and_validate_shader_module(ShaderModuleDescriptor {
                label: Some(label),
                source: ShaderSource::Wgsl(source.into()),
            });
            device.create_compute_pipeline(&RawComputePipelineDescriptor {
                label: Some(label),
                layout: None,
                module: &module,
                entry_point: Some("csMain"),
                compilation_options: Default::default(),
                cache: None,
            })
        };

        let g2p2g = create_compute("g2p2g", &shaders::g2p2g());
        let particle_emit = create_compute("particle_emit", &shaders::particle_emit());
        let bukkit_count = create_compute("bukkit_count", &shaders::bukkit_count());
        let bukkit_allocate = create_compute("bukkit_allocate", &shaders::bukkit_allocate());
        let bukkit_insert = create_compute("bukkit_insert", &shaders::bukkit_insert());
        let set_indirect_args = create_compute("set_indirect_args", &shaders::set_indirect_args());

        let render_module = device.create_and_validate_shader_module(ShaderModuleDescriptor {
            label: Some("particle_render"),
            source: ShaderSource::Wgsl(shaders::particle_render().into()),
        });

        let particle_render = device.create_render_pipeline(&RawRenderPipelineDescriptor {
            label: Some("particle_render"),
            layout: None,
            vertex: RawVertexState {
                module: &render_module,
                entry_point: Some("vertexMain"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(RawFragmentState {
                module: &render_module,
                entry_point: Some("fragmentMain"),
                targets: &[Some(ColorTargetState {
                    format: surface_format,
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::SrcAlpha,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::SrcAlpha,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add,
                        },
                    }),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            g2p2g,
            particle_emit,
            bukkit_count,
            bukkit_allocate,
            bukkit_insert,
            set_indirect_args,
            particle_render,
        }
    }
}
