use bevy::feathers::controls::*;
use bevy::feathers::theme::*;
use bevy::feathers::tokens;
use bevy::prelude::*;
use bevy::ui::Checked;
use bevy::ui_widgets::{
    checkbox_self_update, observe, slider_self_update, Activate, SliderPrecision, SliderStep,
    SliderValue,
};

use crate::scene::*;
use crate::shape_editor::ShapeInteraction;
use crate::types::*;

// --- Marker components ---
#[derive(Component)]
pub struct GravitySlider;
#[derive(Component)]
pub struct IterationSlider;
#[derive(Component)]
pub struct ElasticitySlider;
#[derive(Component)]
pub struct LiquidRelaxSlider;
#[derive(Component)]
pub struct ElasticRelaxSlider;
#[derive(Component)]
pub struct FrictionAngleSlider;
#[derive(Component)]
pub struct PlasticitySlider;
#[derive(Component)]
pub struct BorderFrictionSlider;
#[derive(Component)]
pub struct ViscositySlider;
#[derive(Component)]
pub struct ParticlesPerCellSlider;
#[derive(Component)]
pub struct FpMultSlider;
#[derive(Component)]
pub struct GridVolumeCheckbox;
#[derive(Component)]
pub struct SceneNameLabel;
#[derive(Component)]
pub struct GridStatsLabel;
#[derive(Component)]
pub struct SimRateButton;
#[derive(Component)]
pub struct PixelsPerCellButton;
#[derive(Component)]
pub struct RenderModeButton;
#[derive(Component)]
pub struct MouseFnButton;
#[derive(Component)]
pub struct ShapeInfoLabel;
#[derive(Component)]
pub struct ShapeTypeButton;
#[derive(Component)]
pub struct ShapeFunctionButton;
#[derive(Component)]
pub struct ShapeMaterialButton;

pub fn setup_ui(mut commands: Commands, params: Res<SimParams>, manifest: Res<SceneManifest>) {
    let scene_name = manifest
        .0
        .first()
        .map(|e| e.name.as_str())
        .unwrap_or("None");

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Px(310.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(8.0)),
            row_gap: Val::Px(3.0),
            overflow: Overflow::scroll_y(),
            ..default()
        },
        ThemeBackgroundColor(tokens::WINDOW_BG),
        GlobalZIndex(100),
        children![
            // Title
            (Text::new("PB-MPM Simulation"), ThemedText, TextFont { font_size: 16.0, ..default() }),

            // Scene navigation
            (
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(4.0),
                    align_items: AlignItems::Center,
                    ..default()
                },
                children![
                    (
                        button(ButtonProps::default(), (), Spawn((Text::new("<"), ThemedText))),
                        observe(|_: On<Activate>, mut state: ResMut<SimState>, mut params: ResMut<SimParams>,
                                manifest: Res<SceneManifest>, windows: Query<&Window>,
                                mut q: Query<&mut Text, With<SceneNameLabel>>| {
                            if manifest.0.is_empty() { return; }
                            state.scene_index = (state.scene_index + manifest.0.len() - 1) % manifest.0.len();
                            do_load_scene(&mut state, &mut params, &manifest, &windows, &mut q);
                        }),
                    ),
                    (Text::new(scene_name), SceneNameLabel, ThemedText, TextFont { font_size: 13.0, ..default() }),
                    (
                        button(ButtonProps::default(), (), Spawn((Text::new(">"), ThemedText))),
                        observe(|_: On<Activate>, mut state: ResMut<SimState>, mut params: ResMut<SimParams>,
                                manifest: Res<SceneManifest>, windows: Query<&Window>,
                                mut q: Query<&mut Text, With<SceneNameLabel>>| {
                            if manifest.0.is_empty() { return; }
                            state.scene_index = (state.scene_index + 1) % manifest.0.len();
                            do_load_scene(&mut state, &mut params, &manifest, &windows, &mut q);
                        }),
                    ),
                ],
            ),

            // Reset / Pause
            (
                Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(4.0), ..default() },
                children![
                    (
                        button(ButtonProps::default(), (), Spawn((Text::new("Reset (F5)"), ThemedText))),
                        observe(|_: On<Activate>, mut s: ResMut<SimState>| { s.do_reset = true; }),
                    ),
                    (
                        button(ButtonProps::default(), (), Spawn((Text::new("Pause"), ThemedText))),
                        observe(|_: On<Activate>, mut s: ResMut<SimState>| { s.is_paused = !s.is_paused; }),
                    ),
                ],
            ),

            // Cycling buttons
            (
                Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(4.0), align_items: AlignItems::Center, ..default() },
                children![
                    (Text::new("Px/Cell"), ThemedText, TextFont { font_size: 11.0, ..default() }, Node { width: Val::Px(60.0), ..default() }),
                    (
                        button(ButtonProps::default(), PixelsPerCellButton, Spawn((Text::new(format!("{}", params.sim_res_divisor)), ThemedText))),
                        observe(|ev: On<Activate>, mut p: ResMut<SimParams>, mut s: ResMut<SimState>,
                                q: Query<&Children, With<PixelsPerCellButton>>, mut qt: Query<&mut Text>| {
                            let divs = [1u32, 2, 4, 8, 16];
                            let i = divs.iter().position(|&d| d == p.sim_res_divisor).unwrap_or(3);
                            p.sim_res_divisor = divs[(i + 1) % divs.len()];
                            s.do_reset = true;
                            update_btn_text(ev.event_target(), &q, &mut qt, &format!("{}", p.sim_res_divisor));
                        }),
                    ),
                ],
            ),
            (
                Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(4.0), align_items: AlignItems::Center, ..default() },
                children![
                    (Text::new("Sim Rate"), ThemedText, TextFont { font_size: 11.0, ..default() }, Node { width: Val::Px(60.0), ..default() }),
                    (
                        button(ButtonProps::default(), SimRateButton, Spawn((Text::new(format!("{} Hz", params.sim_rate)), ThemedText))),
                        observe(|ev: On<Activate>, mut p: ResMut<SimParams>,
                                q: Query<&Children, With<SimRateButton>>, mut qt: Query<&mut Text>| {
                            let rates = [15u32, 30, 60, 120, 240, 480, 600, 1200, 2400];
                            let i = rates.iter().position(|&r| r == p.sim_rate).unwrap_or(4);
                            p.sim_rate = rates[(i + 1) % rates.len()];
                            update_btn_text(ev.event_target(), &q, &mut qt, &format!("{} Hz", p.sim_rate));
                        }),
                    ),
                ],
            ),
            (
                Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(4.0), align_items: AlignItems::Center, ..default() },
                children![
                    (Text::new("Mouse"), ThemedText, TextFont { font_size: 11.0, ..default() }, Node { width: Val::Px(60.0), ..default() }),
                    (
                        button(ButtonProps::default(), MouseFnButton, Spawn((Text::new("Grab"), ThemedText))),
                        observe(|ev: On<Activate>, mut p: ResMut<SimParams>,
                                q: Query<&Children, With<MouseFnButton>>, mut qt: Query<&mut Text>| {
                            p.mouse_function = match p.mouse_function { MouseFunction::Grab => MouseFunction::Push, _ => MouseFunction::Grab };
                            let l = match p.mouse_function { MouseFunction::Grab => "Grab", MouseFunction::Push => "Push" };
                            update_btn_text(ev.event_target(), &q, &mut qt, l);
                        }),
                    ),
                ],
            ),
            (
                Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(4.0), align_items: AlignItems::Center, ..default() },
                children![
                    (Text::new("Render"), ThemedText, TextFont { font_size: 11.0, ..default() }, Node { width: Val::Px(60.0), ..default() }),
                    (
                        button(ButtonProps::default(), RenderModeButton, Spawn((Text::new("Standard"), ThemedText))),
                        observe(|ev: On<Activate>, mut p: ResMut<SimParams>,
                                q: Query<&Children, With<RenderModeButton>>, mut qt: Query<&mut Text>| {
                            p.render_mode = match p.render_mode {
                                RenderMode::Standard => RenderMode::Compression,
                                RenderMode::Compression => RenderMode::Velocity,
                                _ => RenderMode::Standard,
                            };
                            let l = match p.render_mode { RenderMode::Standard => "Standard", RenderMode::Compression => "Compress", RenderMode::Velocity => "Velocity" };
                            update_btn_text(ev.event_target(), &q, &mut qt, l);
                        }),
                    ),
                ],
            ),

            // Checkbox
            (
                checkbox(
                    Checked, // default is true
                    Spawn((Text::new("Grid Volume for Liquid"), ThemedText)),
                ),
                GridVolumeCheckbox,
                observe(checkbox_self_update),
            ),

            // Sliders
            (mk_slider("Particles/Axis", ParticlesPerCellSlider, 1.0, 8.0, params.particles_per_cell_axis as f32, 1.0, 0)),
            (mk_slider("Gravity", GravitySlider, 0.0, 5.0, params.gravity_strength, 0.01, 2)),
            (mk_slider("Viscosity", ViscositySlider, 0.0, 1.0, params.liquid_viscosity, 0.01, 2)),
            (mk_slider("Iterations", IterationSlider, 2.0, 100.0, params.iteration_count as f32, 1.0, 0)),
            (mk_slider("Elasticity", ElasticitySlider, 0.0, 1.0, params.elasticity_ratio, 0.01, 2)),
            (mk_slider("Liq Relax", LiquidRelaxSlider, 0.0, 10.0, params.liquid_relaxation, 0.01, 2)),
            (mk_slider("Elas Relax", ElasticRelaxSlider, 0.0, 10.0, params.elastic_relaxation, 0.01, 2)),
            (mk_slider("Friction Ang", FrictionAngleSlider, 0.0, 45.0, params.friction_angle, 0.1, 1)),
            (mk_slider("Plasticity", PlasticitySlider, 0.0, 1.0, params.plasticity, 0.01, 2)),
            (mk_slider("Border Fric", BorderFrictionSlider, 0.0, 1.0, params.border_friction, 0.01, 2)),
            (mk_slider("FP Mult Exp", FpMultSlider, 3.0, 10.0, params.fixed_point_multiplier_exponent as f32, 1.0, 0)),

            // Selected shape info
            (
                Text::new("No shape selected"),
                ShapeInfoLabel,
                ThemedText,
                TextFont { font_size: 11.0, ..default() },
                Node { margin: UiRect::top(Val::Px(8.0)), ..default() },
            ),
            (
                Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(4.0), align_items: AlignItems::Center, ..default() },
                children![
                    (Text::new("Type"), ThemedText, TextFont { font_size: 11.0, ..default() }, Node { width: Val::Px(50.0), ..default() }),
                    (
                        button(ButtonProps::default(), ShapeTypeButton, Spawn((Text::new("--"), ThemedText))),
                        observe(|ev: On<Activate>, mut state: ResMut<SimState>, interaction: Res<ShapeInteraction>,
                                q: Query<&Children, With<ShapeTypeButton>>, mut qt: Query<&mut Text>| {
                            if let Some(idx) = interaction.selected_index {
                                if let Some(shape) = state.shapes.get_mut(idx) {
                                    let cur = shape.shape.as_f32() as u32;
                                    let next = if cur == 0 { 1 } else { 0 };
                                    shape.shape = StringOrNumber::Int(next as i64);
                                    let label = if next == 0 { "Box" } else { "Circle" };
                                    update_btn_text(ev.event_target(), &q, &mut qt, label);
                                }
                            }
                        }),
                    ),
                    (
                        button(ButtonProps::default(), ShapeFunctionButton, Spawn((Text::new("--"), ThemedText))),
                        observe(|ev: On<Activate>, mut state: ResMut<SimState>, interaction: Res<ShapeInteraction>,
                                q: Query<&Children, With<ShapeFunctionButton>>, mut qt: Query<&mut Text>| {
                            if let Some(idx) = interaction.selected_index {
                                if let Some(shape) = state.shapes.get_mut(idx) {
                                    let cur = shape.function.as_f32() as u32;
                                    let next = (cur + 1) % 4;
                                    shape.function = StringOrNumber::Int(next as i64);
                                    let label = match next { 0 => "Emit", 1 => "Collider", 2 => "Drain", _ => "InitEmit" };
                                    update_btn_text(ev.event_target(), &q, &mut qt, label);
                                }
                            }
                        }),
                    ),
                    (
                        button(ButtonProps::default(), ShapeMaterialButton, Spawn((Text::new("--"), ThemedText))),
                        observe(|ev: On<Activate>, mut state: ResMut<SimState>, interaction: Res<ShapeInteraction>,
                                q: Query<&Children, With<ShapeMaterialButton>>, mut qt: Query<&mut Text>| {
                            if let Some(idx) = interaction.selected_index {
                                if let Some(shape) = state.shapes.get_mut(idx) {
                                    let cur = shape.emit_material.as_f32() as u32;
                                    let next = (cur + 1) % 4;
                                    shape.emit_material = StringOrNumber::Int(next as i64);
                                    let label = match next { 0 => "Liquid", 1 => "Elastic", 2 => "Sand", _ => "Visco" };
                                    update_btn_text(ev.event_target(), &q, &mut qt, label);
                                }
                            }
                        }),
                    ),
                ],
            ),

            // Stats
            (
                Text::new("Grid: --"),
                GridStatsLabel,
                TextFont { font_size: 11.0, ..default() },
                TextColor(Color::srgba(0.5, 0.5, 0.5, 1.0)),
                Node { margin: UiRect::top(Val::Px(8.0)), ..default() },
            ),
        ],
    ));
}

fn mk_slider<M: Component>(
    label: &str,
    marker: M,
    min: f32,
    max: f32,
    value: f32,
    step: f32,
    precision: i32,
) -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(4.0),
            ..default()
        },
        children![
            (
                Text::new(label),
                ThemedText,
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                Node {
                    width: Val::Px(85.0),
                    ..default()
                },
            ),
            (
                slider(
                    SliderProps { min, max, value },
                    (marker, SliderStep(step), SliderPrecision(precision)),
                ),
                observe(slider_self_update),
            ),
        ],
    )
}

fn update_btn_text(
    target: Entity,
    q_parent: &Query<&Children, impl bevy::ecs::query::QueryFilter>,
    q_text: &mut Query<&mut Text>,
    label: &str,
) {
    if let Ok(children) = q_parent.get(target) {
        for child in children.iter() {
            if let Ok(mut text) = q_text.get_mut(child) {
                text.0 = label.to_string();
            }
        }
    }
}

pub fn do_load_scene(
    sim_state: &mut SimState,
    params: &mut SimParams,
    manifest: &SceneManifest,
    windows: &Query<&Window>,
    q_name: &mut Query<&mut Text, With<SceneNameLabel>>,
) {
    if let Some(entry) = manifest.0.get(sim_state.scene_index) {
        if let Some(scene_file) = load_scene(&entry.scene) {
            let Ok(window) = windows.single() else { return };
            *params = SimParams::default();
            apply_scene(
                &scene_file,
                sim_state,
                params,
                window.width(),
                window.height(),
            );
        }
        if let Ok(mut text) = q_name.single_mut() {
            text.0 = entry.name.clone();
        }
    }
}

/// Sync slider values back to SimParams every frame
#[allow(clippy::too_many_arguments)]
pub fn sync_params(
    q_gravity: Query<&SliderValue, With<GravitySlider>>,
    q_iterations: Query<&SliderValue, With<IterationSlider>>,
    q_elasticity: Query<&SliderValue, With<ElasticitySlider>>,
    q_liq_relax: Query<&SliderValue, With<LiquidRelaxSlider>>,
    q_elas_relax: Query<&SliderValue, With<ElasticRelaxSlider>>,
    q_friction: Query<&SliderValue, With<FrictionAngleSlider>>,
    q_plasticity: Query<&SliderValue, With<PlasticitySlider>>,
    q_border_fric: Query<&SliderValue, With<BorderFrictionSlider>>,
    q_viscosity: Query<&SliderValue, With<ViscositySlider>>,
    q_ppc: Query<&SliderValue, With<ParticlesPerCellSlider>>,
    q_fp: Query<&SliderValue, With<FpMultSlider>>,
    q_grid_vol: Query<Has<Checked>, With<GridVolumeCheckbox>>,
    mut params: ResMut<SimParams>,
) {
    if let Ok(v) = q_gravity.single() {
        params.gravity_strength = v.0;
    }
    if let Ok(v) = q_iterations.single() {
        params.iteration_count = v.0 as u32;
    }
    if let Ok(v) = q_elasticity.single() {
        params.elasticity_ratio = v.0;
    }
    if let Ok(v) = q_liq_relax.single() {
        params.liquid_relaxation = v.0;
    }
    if let Ok(v) = q_elas_relax.single() {
        params.elastic_relaxation = v.0;
    }
    if let Ok(v) = q_friction.single() {
        params.friction_angle = v.0;
    }
    if let Ok(v) = q_plasticity.single() {
        params.plasticity = v.0;
    }
    if let Ok(v) = q_border_fric.single() {
        params.border_friction = v.0;
    }
    if let Ok(v) = q_viscosity.single() {
        params.liquid_viscosity = v.0;
    }
    if let Ok(v) = q_ppc.single() {
        params.particles_per_cell_axis = v.0 as u32;
    }
    if let Ok(v) = q_fp.single() {
        params.fixed_point_multiplier_exponent = v.0 as u32;
    }
    if let Ok(checked) = q_grid_vol.single() {
        params.use_grid_volume_for_liquid = checked;
    }
}

/// Update the shape info label and button text based on current selection
pub fn update_shape_info(
    sim_state: Res<SimState>,
    interaction: Res<ShapeInteraction>,
    mut q_info: Query<&mut Text, With<ShapeInfoLabel>>,
    q_type_btn: Query<&Children, With<ShapeTypeButton>>,
    q_func_btn: Query<&Children, With<ShapeFunctionButton>>,
    q_mat_btn: Query<&Children, With<ShapeMaterialButton>>,
    mut q_text: Query<&mut Text, (Without<ShapeInfoLabel>, Without<GridStatsLabel>)>,
) {
    let info_text = if let Some(idx) = interaction.selected_index {
        if let Some(shape) = sim_state.shapes.get(idx) {
            let shape_type = if shape.shape.as_f32() as u32 == 0 {
                "Box"
            } else {
                "Circle"
            };
            let func = match shape.function.as_f32() as u32 {
                0 => "Emit",
                1 => "Collider",
                2 => "Drain",
                _ => "InitEmit",
            };
            let mat = match shape.emit_material.as_f32() as u32 {
                0 => "Liquid",
                1 => "Elastic",
                2 => "Sand",
                _ => "Visco",
            };

            // Update button labels
            if let Ok(children) = q_type_btn.single() {
                for child in children.iter() {
                    if let Ok(mut t) = q_text.get_mut(child) {
                        t.0 = shape_type.to_string();
                    }
                }
            }
            if let Ok(children) = q_func_btn.single() {
                for child in children.iter() {
                    if let Ok(mut t) = q_text.get_mut(child) {
                        t.0 = func.to_string();
                    }
                }
            }
            if let Ok(children) = q_mat_btn.single() {
                for child in children.iter() {
                    if let Ok(mut t) = q_text.get_mut(child) {
                        t.0 = mat.to_string();
                    }
                }
            }

            format!("Shape {} [{}] {} {}", shape.id, shape_type, func, mat)
        } else {
            "No shape selected".to_string()
        }
    } else {
        "Click a shape to select (Cmd+N new, Del remove)".to_string()
    };

    if let Ok(mut text) = q_info.single_mut() {
        text.0 = info_text;
    }
}

/// Update stats label
pub fn update_stats(sim_state: Res<SimState>, mut q_stats: Query<&mut Text, With<GridStatsLabel>>) {
    if let Ok(mut text) = q_stats.single_mut() {
        text.0 = format!(
            "Grid: {}x{} | Shapes: {}",
            sim_state.grid_size[0],
            sim_state.grid_size[1],
            sim_state.shapes.len()
        );
    }
}
