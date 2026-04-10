/// Shader source management.
/// Common WGSL code is concatenated with per-shader code at initialization time.
const COMMON: &str = include_str!("shaders/common.wgsl");
const G2P2G: &str = include_str!("shaders/g2p2g.wgsl");
const PARTICLE_EMIT: &str = include_str!("shaders/particle_emit.wgsl");
const PARTICLE_RENDER: &str = include_str!("shaders/particle_render.wgsl");
const BUKKIT_COUNT: &str = include_str!("shaders/bukkit_count.wgsl");
const BUKKIT_ALLOCATE: &str = include_str!("shaders/bukkit_allocate.wgsl");
const BUKKIT_INSERT: &str = include_str!("shaders/bukkit_insert.wgsl");
const SET_INDIRECT_ARGS: &str = include_str!("shaders/set_indirect_args.wgsl");

fn build(main_code: &str) -> String {
    format!("{}\n\n{}", COMMON, main_code)
}

pub fn g2p2g() -> String {
    build(G2P2G)
}

pub fn particle_emit() -> String {
    build(PARTICLE_EMIT)
}

pub fn particle_render() -> String {
    build(PARTICLE_RENDER)
}

pub fn bukkit_count() -> String {
    build(BUKKIT_COUNT)
}

pub fn bukkit_allocate() -> String {
    build(BUKKIT_ALLOCATE)
}

pub fn bukkit_insert() -> String {
    build(BUKKIT_INSERT)
}

pub fn set_indirect_args() -> String {
    build(SET_INDIRECT_ARGS)
}
