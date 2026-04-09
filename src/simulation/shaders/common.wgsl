// PB-MPM Common Shader Definitions
// Ported from EA SEED's WebGPU implementation

// --- Dispatch Sizes ---
const ParticleDispatchSize = 64u;
const GridDispatchSize = 8u;
const BukkitSize = 6u;
const BukkitHaloSize = 1u;

fn divUp(threadCount: u32, divisor: u32) -> u32 {
    return (threadCount + divisor - 1u) / divisor;
}

// --- Sim Enums ---
const SolverTypePBMPM = 1;

const MouseFunctionPush = 0.0;
const MouseFunctionGrab = 1.0;

const MaterialLiquid = 0.0;
const MaterialElastic = 1.0;
const MaterialSand = 2.0;
const MaterialVisco = 3.0;

const GuardianSize = 3u;
const GuardianSizeF = 3.0;

const ShapeTypeBox = 0.0;
const ShapeTypeCircle = 1.0;

const ShapeFunctionEmit = 0.0;
const ShapeFunctionCollider = 1.0;
const ShapeFunctionDrain = 2.0;
const ShapeFunctionInitialEmit = 3.0;

// --- Render Enums ---
const RenderModeStandard = 0.0;
const RenderModeCompression = 1.0;
const RenderModeVelocity = 2.0;

// --- SimConstants Uniform ---
struct SimConstants {
    gridSize: vec2u,
    deltaTime: f32,
    mouseActivation: f32,
    mousePosition: vec2f,
    mouseVelocity: vec2f,
    mouseFunction: f32,
    elasticityRatio: f32,
    gravityStrength: f32,
    liquidRelaxation: f32,
    elasticRelaxation: f32,
    liquidViscosity: f32,
    fixedPointMultiplier: u32,
    useGridVolumeForLiquid: u32,
    particlesPerCellAxis: u32,
    frictionAngle: f32,
    plasticity: f32,
    mouseRadius: f32,
    shapeCount: u32,
    simFrame: u32,
    bukkitCount: u32,
    bukkitCountX: u32,
    bukkitCountY: u32,
    iteration: u32,
    iterationCount: u32,
    borderFriction: f32,
};

// --- SimShape ---
struct SimShape {
    position: vec2f,
    halfSize: vec2f,
    radius: f32,
    rotation: f32,
    functionality: f32,
    shapeType: f32,
    emitMaterial: f32,
    emissionRate: f32,
    emissionSpeed: f32,
    padding: f32,
};

// --- RenderConstants ---
struct RenderConstants {
    particleRadiusTimestamp: vec2f,
    canvasSize: vec2f,
    viewPos: vec2f,
    viewExtent: vec2f,
    renderMode: f32,
    deltaTime: f32,
    _pad0: f32,
    _pad1: f32,
};

// --- Particle ---
struct Particle {
    position: vec2f,
    displacement: vec2f,
    deformationGradient: mat2x2f,
    deformationDisplacement: mat2x2f,
    liquidDensity: f32,
    mass: f32,
    material: f32,
    volume: f32,
    lambda: f32,
    logJp: f32,
    color: vec3f,
    enabled: f32,
};

// --- BukkitThreadData ---
struct BukkitThreadData {
    rangeStart: u32,
    rangeCount: u32,
    bukkitX: u32,
    bukkitY: u32,
};

// --- Grid Helpers ---
fn gridVertexIndex(gridVertex: vec2u, gridSize: vec2u) -> u32 {
    return 4u * (gridVertex.y * gridSize.x + gridVertex.x);
}

fn decodeFixedPoint(fixedPoint: i32, fixedPointMultiplier: u32) -> f32 {
    return f32(fixedPoint) / f32(fixedPointMultiplier);
}

fn encodeFixedPoint(floatingPoint: f32, fixedPointMultiplier: u32) -> i32 {
    return i32(floatingPoint * f32(fixedPointMultiplier));
}

// --- Matrix Operations ---
struct SVDResult {
    U: mat2x2f,
    Sigma: vec2f,
    Vt: mat2x2f,
};

fn svd(m: mat2x2f) -> SVDResult {
    let E = (m[0][0] + m[1][1]) * 0.5;
    let F = (m[0][0] - m[1][1]) * 0.5;
    let G = (m[0][1] + m[1][0]) * 0.5;
    let H = (m[0][1] - m[1][0]) * 0.5;

    let Q = sqrt(E * E + H * H);
    let R = sqrt(F * F + G * G);
    let sx = Q + R;
    let sy = Q - R;

    // Guard atan2(0,0) which is undefined on Metal (macOS)
    let a1 = select(atan2(G, F), 0.0, F == 0.0 && G == 0.0);
    let a2 = select(atan2(H, E), 0.0, E == 0.0 && H == 0.0);

    let theta = (a2 - a1) * 0.5;
    let phi = (a2 + a1) * 0.5;

    let U = rot(phi);
    let Sigma = vec2f(sx, sy);
    let Vt = rot(theta);

    return SVDResult(U, Sigma, Vt);
}

fn det(m: mat2x2f) -> f32 {
    return m[0][0] * m[1][1] - m[0][1] * m[1][0];
}

fn tr(m: mat2x2f) -> f32 {
    return m[0][0] + m[1][1];
}

fn rot(theta: f32) -> mat2x2f {
    let ct = cos(theta);
    let st = sin(theta);
    return mat2x2f(ct, st, -st, ct);
}

fn inverse(m: mat2x2f) -> mat2x2f {
    let a = m[0][0];
    let b = m[1][0];
    let c = m[0][1];
    let d = m[1][1];
    return (1.0 / det(m)) * mat2x2f(d, -c, -b, a);
}

fn outerProduct2(x: vec2f, y: vec2f) -> mat2x2f {
    return mat2x2f(x * y.x, x * y.y);
}

fn diag(d: vec2f) -> mat2x2f {
    return mat2x2f(d.x, 0.0, 0.0, d.y);
}

const Identity = mat2x2f(1.0, 0.0, 0.0, 1.0);
const ZeroMatrix = mat2x2f(0.0, 0.0, 0.0, 0.0);

// --- Particle Helpers ---
fn projectInsideGuardian(p: vec2f, gridSize: vec2u, guardianSize: f32) -> vec2f {
    let clampMin = vec2f(guardianSize);
    let clampMax = vec2f(gridSize) - vec2f(guardianSize, guardianSize) - vec2f(1.0, 1.0);
    return clamp(p, vec2f(clampMin), vec2f(clampMax));
}

fn insideGuardian(id: vec2u, gridSize: vec2u, guardianSize: u32) -> bool {
    if (id.x <= guardianSize) { return false; }
    if (id.x >= (gridSize.x - guardianSize - 1u)) { return false; }
    if (id.y <= guardianSize) { return false; }
    if (id.y >= gridSize.y - guardianSize - 1u) { return false; }
    return true;
}

struct QuadraticWeightInfo {
    weights: array<vec2f, 3>,
    cellIndex: vec2f,
}

fn pow2(x: vec2f) -> vec2f {
    return x * x;
}

fn quadraticWeightInit(position: vec2f) -> QuadraticWeightInfo {
    let roundDownPosition = floor(position);
    let offset = (position - roundDownPosition) - 0.5;
    return QuadraticWeightInfo(
        array(
            0.5 * pow2(0.5 - offset),
            0.75 - pow2(offset),
            0.5 * pow2(0.5 + offset)
        ),
        roundDownPosition - 1.0,
    );
}

// --- Collision ---
struct CollideResult {
    collides: bool,
    penetration: f32,
    normal: vec2f,
    pointOnCollider: vec2f,
};

fn collide(shape: SimShape, pos: vec2f) -> CollideResult {
    if (shape.shapeType == ShapeTypeCircle) {
        let offset = shape.position - pos;
        let offsetLen = length(offset);
        let normal = offset * select(1.0 / offsetLen, 0.0, offsetLen == 0.0);
        return CollideResult(
            offsetLen <= shape.radius,
            -(offsetLen - shape.radius),
            normal,
            shape.position + normal * shape.radius,
        );
    } else if (shape.shapeType == ShapeTypeBox) {
        let offset = pos - shape.position;
        let R = rot(shape.rotation / 180.0 * 3.14159);
        let rotOffset = R * offset;
        let sx = sign(rotOffset.x);
        let sy = sign(rotOffset.y);
        let penetration = -(abs(rotOffset) - shape.halfSize);
        let normal = transpose(R) * select(vec2f(sx, 0.0), vec2f(0.0, sy), penetration.y < penetration.x);
        let minPen = min(penetration.x, penetration.y);
        let pointOnBox = shape.position + transpose(R) * clamp(rotOffset, -shape.halfSize, shape.halfSize);
        return CollideResult(
            minPen > 0.0,
            minPen,
            -normal,
            pointOnBox
        );
    } else {
        return CollideResult(false, 0.0, vec2f(0.0, 0.0), vec2f(0.0, 0.0));
    }
}

// --- Bukkit Helpers ---
fn bukkitAddressToIndex(address: vec2u, bukkitCountX: u32) -> u32 {
    return address.y * bukkitCountX + address.x;
}

fn positionToBukkitId(position: vec2f) -> vec2i {
    return vec2i((position) / f32(BukkitSize));
}

// --- Random ---
fn hash(input: u32) -> u32 {
    let state = input * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn randomFloat(input: u32) -> f32 {
    return f32(hash(input) % 10000u) / 9999.0;
}
