// G2P2G - Main MPM Grid-to-Particle-to-Grid kernel

@group(0) @binding(0) var<uniform> g_simConstants: SimConstants;
@group(0) @binding(1) var<storage, read_write> g_particles: array<Particle>;
@group(0) @binding(2) var<storage> g_gridSrc: array<i32>;
@group(0) @binding(3) var<storage, read_write> g_gridDst: array<atomic<i32>>;
@group(0) @binding(4) var<storage, read_write> g_gridToBeCleared: array<i32>;
@group(0) @binding(5) var<storage> g_bukkitThreadData: array<BukkitThreadData>;
@group(0) @binding(6) var<storage> g_bukkitParticleData: array<u32>;
@group(0) @binding(7) var<storage> g_shapes: array<SimShape>;
@group(0) @binding(8) var<storage, read_write> g_freeIndices: array<atomic<i32>>;

const TotalBukkitEdgeLength = BukkitSize + BukkitHaloSize * 2u;
const TileDataSizePerEdge = TotalBukkitEdgeLength * 4u;
const TileDataSize = TileDataSizePerEdge * TileDataSizePerEdge;
var<workgroup> s_tileData: array<atomic<i32>, TileDataSize>;
var<workgroup> s_tileDataDst: array<atomic<i32>, TileDataSize>;

fn localGridIndex(index: vec2u) -> u32 {
    return (index.y * TotalBukkitEdgeLength + index.x) * 4u;
}

@compute @workgroup_size(ParticleDispatchSize)
fn csMain(@builtin(local_invocation_index) indexInGroup: u32, @builtin(workgroup_id) groupId: vec3<u32>) {
    let threadData = g_bukkitThreadData[groupId.x];

    // Load grid
    let localGridOrigin = i32(BukkitSize) * vec2i(vec2u(threadData.bukkitX, threadData.bukkitY)) - vec2i(i32(BukkitHaloSize));
    let idInGroup = vec2i(i32(indexInGroup) % i32(TotalBukkitEdgeLength), i32(indexInGroup) / i32(TotalBukkitEdgeLength));
    let gridVertex = idInGroup + localGridOrigin;
    let gridPosition = vec2f(gridVertex);

    var dx = 0.0;
    var dy = 0.0;
    var w = 0.0;
    var v = 0.0;

    var gridVertexIsValid = all(gridVertex >= vec2i(0i)) && all(gridVertex <= vec2i(g_simConstants.gridSize));

    if (gridVertexIsValid) {
        let gridVertexAddress = gridVertexIndex(vec2u(gridVertex), g_simConstants.gridSize);

        dx = decodeFixedPoint(g_gridSrc[gridVertexAddress + 0u], g_simConstants.fixedPointMultiplier);
        dy = decodeFixedPoint(g_gridSrc[gridVertexAddress + 1u], g_simConstants.fixedPointMultiplier);
        w = decodeFixedPoint(g_gridSrc[gridVertexAddress + 2u], g_simConstants.fixedPointMultiplier);
        v = decodeFixedPoint(g_gridSrc[gridVertexAddress + 3u], g_simConstants.fixedPointMultiplier);

        if (w < 1e-5f) {
            dx = 0.0;
            dy = 0.0;
        } else {
            dx = dx / w;
            dy = dy / w;
        }

        var gridDisplacement = vec2f(dx, dy);

        // Collision detection against collider shapes
        for (var shapeIndex = 0u; shapeIndex < g_simConstants.shapeCount; shapeIndex++) {
            let shape = g_shapes[shapeIndex];
            if (shape.functionality != ShapeFunctionCollider) {
                continue;
            }
            let displacedGridPosition = gridPosition + gridDisplacement;
            let collideResult = collide(shape, displacedGridPosition);
            if (collideResult.collides) {
                let gap = min(0.0, dot(collideResult.normal, collideResult.pointOnCollider - gridPosition));
                let penetration = dot(collideResult.normal, gridDisplacement) - gap;
                let radialImpulse = max(penetration, 0.0);
                gridDisplacement -= radialImpulse * collideResult.normal;
            }
        }

        // Guardian collision
        let displacedGridPosition = gridPosition + gridDisplacement;
        let projectedGridPosition = projectInsideGuardian(displacedGridPosition, g_simConstants.gridSize, GuardianSizeF + 1.0);
        let projectedDifference = projectedGridPosition - displacedGridPosition;

        if (projectedDifference.x != 0.0) {
            gridDisplacement.x = 0.0;
            gridDisplacement.y = mix(gridDisplacement.y, 0.0, g_simConstants.borderFriction);
        }
        if (projectedDifference.y != 0.0) {
            gridDisplacement.y = 0.0;
            gridDisplacement.x = mix(gridDisplacement.x, 0.0, g_simConstants.borderFriction);
        }

        dx = gridDisplacement.x;
        dy = gridDisplacement.y;
    }

    // Save grid to local memory
    let tileDataIndex = localGridIndex(vec2u(idInGroup));
    atomicStore(&s_tileData[tileDataIndex], encodeFixedPoint(dx, g_simConstants.fixedPointMultiplier));
    atomicStore(&s_tileData[tileDataIndex + 1u], encodeFixedPoint(dy, g_simConstants.fixedPointMultiplier));
    atomicStore(&s_tileData[tileDataIndex + 2u], encodeFixedPoint(w, g_simConstants.fixedPointMultiplier));
    atomicStore(&s_tileData[tileDataIndex + 3u], encodeFixedPoint(v, g_simConstants.fixedPointMultiplier));

    workgroupBarrier();

    if (indexInGroup < threadData.rangeCount) {
        let myParticleIndex = g_bukkitParticleData[threadData.rangeStart + indexInGroup];
        var particle = g_particles[myParticleIndex];

      if (particle.enabled != 0.0) {
        var p = particle.position;
        let weightInfo = quadraticWeightInit(p);

        if (g_simConstants.iteration != 0u) {
            // G2P
            var B = ZeroMatrix;
            var d = vec2f(0.0);
            var volume = 0.0;

            for (var i = 0i; i < 3i; i++) {
                for (var j = 0i; j < 3i; j++) {
                    let weight = weightInfo.weights[i].x * weightInfo.weights[j].y;
                    let neighbourCellIndex = vec2i(weightInfo.cellIndex) + vec2i(i, j);
                    let neighbourCellIndexLocal = neighbourCellIndex - localGridOrigin;
                    let gridVertexIdx = localGridIndex(vec2u(neighbourCellIndexLocal));

                    let weightedDisplacement = weight * vec2f(
                        decodeFixedPoint(atomicLoad(&s_tileData[gridVertexIdx + 0u]), g_simConstants.fixedPointMultiplier),
                        decodeFixedPoint(atomicLoad(&s_tileData[gridVertexIdx + 1u]), g_simConstants.fixedPointMultiplier)
                    );

                    let offset = vec2f(neighbourCellIndex) - p + 0.5;
                    B += outerProduct2(weightedDisplacement, offset);
                    d += weightedDisplacement;

                    if (g_simConstants.useGridVolumeForLiquid != 0u) {
                        volume += weight * decodeFixedPoint(atomicLoad(&s_tileData[gridVertexIdx + 3u]), g_simConstants.fixedPointMultiplier);
                    }
                }
            }

            if (g_simConstants.useGridVolumeForLiquid != 0u) {
                volume = 1.0 / volume;
                if (volume < 1.0) {
                    particle.liquidDensity = mix(particle.liquidDensity, volume, 0.1);
                }
            }

            particle.deformationDisplacement = B * 4.0;
            particle.displacement = d;

            // Integration on final iteration
            if (g_simConstants.iteration == g_simConstants.iterationCount - 1u) {
                if (particle.material == MaterialLiquid) {
                    particle.liquidDensity *= (tr(particle.deformationDisplacement) + 1.0);
                    particle.liquidDensity = max(particle.liquidDensity, 0.05);
                } else {
                    particle.deformationGradient = (Identity + particle.deformationDisplacement) * particle.deformationGradient;
                }

                if (particle.material != MaterialLiquid) {
                    var svdResult = svd(particle.deformationGradient);
                    svdResult.Sigma = clamp(svdResult.Sigma, vec2f(0.1), vec2f(10000.0));

                    if (particle.material == MaterialSand) {
                        let sinPhi = sin(g_simConstants.frictionAngle / 180.0 * 3.14159);
                        let alpha = sqrt(2.0 / 3.0) * 2.0 * sinPhi / (3.0 - sinPhi);
                        let beta = 0.5;
                        let eDiag = log(max(abs(svdResult.Sigma), vec2f(1e-6)));
                        let eps = diag(eDiag);
                        let trace = tr(eps) + particle.logJp;
                        let eHat = eps - (trace / 2.0) * Identity;
                        let frobNrm = length(vec2f(eHat[0][0], eHat[1][1]));

                        if (trace >= 0.0) {
                            svdResult.Sigma = vec2f(1.0);
                            particle.logJp = beta * trace;
                        } else {
                            particle.logJp = 0.0;
                            let deltaGammaI = frobNrm + (g_simConstants.elasticityRatio + 1.0) * trace * alpha;
                            if (deltaGammaI > 0.0) {
                                let h = eDiag - deltaGammaI / frobNrm * (eDiag - (trace * 0.5));
                                svdResult.Sigma = exp(h);
                            }
                        }
                    } else if (particle.material == MaterialVisco) {
                        let yieldSurface = exp(1.0 - g_simConstants.plasticity);
                        let J = svdResult.Sigma.x * svdResult.Sigma.y;
                        svdResult.Sigma = clamp(svdResult.Sigma, vec2f(1.0 / yieldSurface), vec2f(yieldSurface));
                        let newJ = svdResult.Sigma.x * svdResult.Sigma.y;
                        svdResult.Sigma *= sqrt(J / newJ);
                    }

                    particle.deformationGradient = svdResult.U * diag(svdResult.Sigma) * svdResult.Vt;
                }

                // Integrate position
                particle.position += particle.displacement;

                // Host-driven interaction (push or grab)
                if (g_simConstants.interactionStrength > 0.0) {
                    let offset = particle.position - g_simConstants.interactionPosition;
                    let lenOffset = max(length(offset), 0.0001);
                    if (lenOffset < g_simConstants.interactionRadius) {
                        let normOffset = offset / lenOffset;
                        if (g_simConstants.interactionMode == InteractionModePush) {
                            particle.displacement += normOffset * g_simConstants.interactionStrength;
                        } else if (g_simConstants.interactionMode == InteractionModeGrab) {
                            particle.displacement = g_simConstants.interactionVelocity * g_simConstants.deltaTime;
                        }
                    }
                }

                // Gravity
                particle.displacement.y -= f32(g_simConstants.gridSize.y) * g_simConstants.gravityStrength * g_simConstants.deltaTime * g_simConstants.deltaTime;

                atomicMax(&g_freeIndices[0], 0i);

                for (var shapeIndex = 0u; shapeIndex < g_simConstants.shapeCount; shapeIndex++) {
                    let shape = g_shapes[shapeIndex];
                    if (shape.functionality == ShapeFunctionCollider) {
                        let collideResult = collide(shape, particle.position);
                        if (collideResult.collides) {
                            particle.displacement -= collideResult.penetration * collideResult.normal;
                        }
                    }
                    if (shape.functionality == ShapeFunctionDrain) {
                        if (collide(shape, particle.position).collides) {
                            particle.enabled = 0.0;
                            let freeIndex = atomicAdd(&g_freeIndices[0], 1i);
                            atomicStore(&g_freeIndices[1u + u32(freeIndex)], i32(myParticleIndex));
                        }
                    }
                }

                particle.position = projectInsideGuardian(particle.position, g_simConstants.gridSize, GuardianSizeF);
            }

            // Save particle
            g_particles[myParticleIndex] = particle;
        }

        // Particle constraint update + P2G
        {
            if (particle.material == MaterialLiquid) {
                let deviatoric = -1.0 * (particle.deformationDisplacement + transpose(particle.deformationDisplacement));
                particle.deformationDisplacement += g_simConstants.liquidViscosity * 0.5 * deviatoric;

                let alpha = 0.5 * (1.0 / particle.liquidDensity - tr(particle.deformationDisplacement) - 1.0);
                particle.deformationDisplacement += g_simConstants.liquidRelaxation * alpha * Identity;
            } else if (particle.material == MaterialElastic || particle.material == MaterialVisco) {
                let F = (Identity + particle.deformationDisplacement) * particle.deformationGradient;
                var svdResult = svd(F);
                let df = det(F);
                let cdf = clamp(abs(df), 0.1, 1000.0);
                let Q = (1.0f / (sign(df) * sqrt(cdf))) * F;
                let alpha = g_simConstants.elasticityRatio;
                let tgt = alpha * (svdResult.U * svdResult.Vt) + (1.0 - alpha) * Q;
                let diff = (tgt * inverse(particle.deformationGradient) - Identity) - particle.deformationDisplacement;
                particle.deformationDisplacement += g_simConstants.elasticRelaxation * diff;
            } else if (particle.material == MaterialSand) {
                let F = (Identity + particle.deformationDisplacement) * particle.deformationGradient;
                var svdResult = svd(F);
                if (particle.logJp == 0.0) {
                    svdResult.Sigma = clamp(svdResult.Sigma, vec2f(1.0, 1.0), vec2f(1000.0, 1000.0));
                }
                let df = det(F);
                let cdf = clamp(abs(df), 0.1, 1.0);
                let Q = (1.0f / (sign(df) * sqrt(cdf))) * F;
                let alpha = g_simConstants.elasticityRatio;
                let tgt = alpha * (svdResult.U * mat2x2f(svdResult.Sigma.x, 0.0, 0.0, svdResult.Sigma.y) * svdResult.Vt) + (1.0 - alpha) * Q;
                let diff = (tgt * inverse(particle.deformationGradient) - Identity) - particle.deformationDisplacement;
                particle.deformationDisplacement += g_simConstants.elasticRelaxation * diff;

                let deviatoric = -1.0 * (particle.deformationDisplacement + transpose(particle.deformationDisplacement));
                particle.deformationDisplacement += g_simConstants.liquidViscosity * 0.5 * deviatoric;
            }

            // P2G
            for (var i = 0i; i < 3i; i++) {
                for (var j = 0i; j < 3i; j++) {
                    let weight = weightInfo.weights[i].x * weightInfo.weights[j].y;
                    let neighbourCellIndex = vec2i(weightInfo.cellIndex) + vec2i(i, j);
                    let neighbourCellIndexLocal = neighbourCellIndex - localGridOrigin;
                    let gridVertexIdx = localGridIndex(vec2u(neighbourCellIndexLocal));

                    let offset = vec2f(neighbourCellIndex) - p + 0.5;
                    let weightedMass = weight * particle.mass;
                    let momentum = weightedMass * (particle.displacement + particle.deformationDisplacement * offset);

                    atomicAdd(&s_tileDataDst[gridVertexIdx + 0u], encodeFixedPoint(momentum.x, g_simConstants.fixedPointMultiplier));
                    atomicAdd(&s_tileDataDst[gridVertexIdx + 1u], encodeFixedPoint(momentum.y, g_simConstants.fixedPointMultiplier));
                    atomicAdd(&s_tileDataDst[gridVertexIdx + 2u], encodeFixedPoint(weightedMass, g_simConstants.fixedPointMultiplier));

                    if (g_simConstants.useGridVolumeForLiquid != 0u) {
                        atomicAdd(&s_tileDataDst[gridVertexIdx + 3u], encodeFixedPoint(particle.volume * weight, g_simConstants.fixedPointMultiplier));
                    }
                }
            }
        }
      } // if (particle.enabled != 0.0)
    }

    workgroupBarrier();

    // Save Grid
    if (gridVertexIsValid) {
        let gridVertexAddress = gridVertexIndex(vec2u(gridVertex), g_simConstants.gridSize);

        let dxi = atomicLoad(&s_tileDataDst[tileDataIndex]);
        let dyi = atomicLoad(&s_tileDataDst[tileDataIndex + 1u]);
        let wi = atomicLoad(&s_tileDataDst[tileDataIndex + 2u]);
        let vi = atomicLoad(&s_tileDataDst[tileDataIndex + 3u]);

        atomicAdd(&g_gridDst[gridVertexAddress + 0u], dxi);
        atomicAdd(&g_gridDst[gridVertexAddress + 1u], dyi);
        atomicAdd(&g_gridDst[gridVertexAddress + 2u], wi);
        atomicAdd(&g_gridDst[gridVertexAddress + 3u], vi);

        g_gridToBeCleared[gridVertexAddress + 0u] = 0i;
        g_gridToBeCleared[gridVertexAddress + 1u] = 0i;
        g_gridToBeCleared[gridVertexAddress + 2u] = 0i;
        g_gridToBeCleared[gridVertexAddress + 3u] = 0i;
    }
}
