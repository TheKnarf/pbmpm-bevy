use wgpu::util::DeviceExt;

use crate::types::*;

/// Holds all persistent GPU buffers for the simulation.
#[derive(Default)]
pub struct GpuSimState {
    pub initialized: bool,
    pub grid_size: [u32; 2],

    // Particle buffers
    pub particle_buffer: Option<wgpu::Buffer>,
    pub particle_count_buffer: Option<wgpu::Buffer>,
    pub particle_sim_dispatch_buffer: Option<wgpu::Buffer>,
    pub particle_render_dispatch_buffer: Option<wgpu::Buffer>,
    pub particle_free_indices_buffer: Option<wgpu::Buffer>,

    // Grid buffers (3 rotating)
    pub grid_buffers: [Option<wgpu::Buffer>; 3],

    // Bukkit system
    pub bukkit_count_buffer: Option<wgpu::Buffer>,
    pub bukkit_count_buffer2: Option<wgpu::Buffer>,
    pub bukkit_thread_data: Option<wgpu::Buffer>,
    pub bukkit_particle_data: Option<wgpu::Buffer>,
    pub bukkit_dispatch: Option<wgpu::Buffer>,
    pub bukkit_blank_dispatch: Option<wgpu::Buffer>,
    pub bukkit_particle_allocator: Option<wgpu::Buffer>,
    pub bukkit_index_start: Option<wgpu::Buffer>,

    pub bukkit_count_x: u32,
    pub bukkit_count_y: u32,
}

impl GpuSimState {
    pub fn initialize(&mut self, device: &wgpu::Device, grid_size: [u32; 2]) {
        self.grid_size = grid_size;

        let bukkit_count_x = (grid_size[0] as f32 / BUKKIT_SIZE as f32).ceil() as u32;
        let bukkit_count_y = (grid_size[1] as f32 / BUKKIT_SIZE as f32).ceil() as u32;
        self.bukkit_count_x = bukkit_count_x;
        self.bukkit_count_y = bukkit_count_y;

        // Particle buffer (storage only - written by shaders)
        self.particle_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("particles"),
            size: (MAX_PARTICLE_COUNT as u64) * (PARTICLE_FLOAT_COUNT as u64) * 4,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        }));

        // Particle count buffer [count, 0, 0, 0]
        self.particle_count_buffer =
            Some(create_4u32_buffer(device, "particle_count", &[0, 0, 0, 0],
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC));

        // Sim dispatch [workgroups_x, 1, 1, 0]
        self.particle_sim_dispatch_buffer =
            Some(create_4u32_buffer(device, "sim_dispatch", &[0, 1, 1, 0],
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::COPY_DST));

        // Render dispatch [vertex_count=6, instance_count=0, first_vertex=0, first_instance=0]
        self.particle_render_dispatch_buffer =
            Some(create_4u32_buffer(device, "render_dispatch", &[6, 0, 0, 0],
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::COPY_DST));

        // Free indices buffer: [count] + [index0, index1, ...]
        self.particle_free_indices_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("free_indices"),
            size: 4 + (MAX_PARTICLE_COUNT as u64) * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        }));

        // Grid buffers (3)
        let grid_buffer_size = (grid_size[0] as u64) * (grid_size[1] as u64) * 4 * 4;
        for i in 0..3 {
            self.grid_buffers[i] = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("grid_{}", i)),
                size: grid_buffer_size,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
        }

        // Bukkit system
        let bukkit_count = bukkit_count_x * bukkit_count_y;

        self.bukkit_count_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bukkit_count"),
            size: (bukkit_count as u64) * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));

        self.bukkit_count_buffer2 = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bukkit_count2"),
            size: (bukkit_count as u64) * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));

        self.bukkit_thread_data = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bukkit_thread_data"),
            size: 10 * (bukkit_count as u64) * 4 * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));

        self.bukkit_particle_data = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bukkit_particle_data"),
            size: (MAX_PARTICLE_COUNT as u64) * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));

        self.bukkit_dispatch =
            Some(create_4u32_buffer(device, "bukkit_dispatch", &[0, 1, 1, 0],
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::COPY_DST));

        self.bukkit_blank_dispatch =
            Some(create_4u32_buffer(device, "bukkit_blank_dispatch", &[0, 1, 1, 0],
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST));

        self.bukkit_particle_allocator =
            Some(create_4u32_buffer(device, "bukkit_allocator", &[0, 0, 0, 0],
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST));

        self.bukkit_index_start = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bukkit_index_start"),
            size: (bukkit_count as u64) * 4,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        }));

        self.initialized = true;
    }
}

fn create_4u32_buffer(
    device: &wgpu::Device,
    label: &str,
    values: &[u32; 4],
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: bytemuck::cast_slice(values),
        usage,
    })
}

/// Holds all compute and render pipelines.
pub struct PbmpmPipelines {
    pub g2p2g: wgpu::ComputePipeline,
    pub particle_emit: wgpu::ComputePipeline,
    pub bukkit_count: wgpu::ComputePipeline,
    pub bukkit_allocate: wgpu::ComputePipeline,
    pub bukkit_insert: wgpu::ComputePipeline,
    pub set_indirect_args: wgpu::ComputePipeline,
    pub particle_render: wgpu::RenderPipeline,
}

impl PbmpmPipelines {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        use super::shaders;

        let create_compute = |label: &str, source: &str| -> wgpu::ComputePipeline {
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(label),
                layout: None, // auto layout
                module: &module,
                entry_point: Some("csMain"),
                compilation_options: Default::default(),
                cache: None,
            })
        };

        let g2p2g_src = shaders::g2p2g();
        let emit_src = shaders::particle_emit();
        let bk_count_src = shaders::bukkit_count();
        let bk_alloc_src = shaders::bukkit_allocate();
        let bk_insert_src = shaders::bukkit_insert();
        let set_args_src = shaders::set_indirect_args();
        let render_src = shaders::particle_render();

        let g2p2g = create_compute("g2p2g", &g2p2g_src);
        let particle_emit = create_compute("particle_emit", &emit_src);
        let bukkit_count = create_compute("bukkit_count", &bk_count_src);
        let bukkit_allocate = create_compute("bukkit_allocate", &bk_alloc_src);
        let bukkit_insert = create_compute("bukkit_insert", &bk_insert_src);
        let set_indirect_args = create_compute("set_indirect_args", &set_args_src);

        // Render pipeline
        let render_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("particle_render"),
            source: wgpu::ShaderSource::Wgsl(render_src.into()),
        });

        let particle_render = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("particle_render"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &render_module,
                entry_point: Some("vertexMain"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &render_module,
                entry_point: Some("fragmentMain"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
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
