use bevy::ecs::system::SystemParam;
use bevy::feathers::controls::*;
use bevy::feathers::theme::*;
use bevy::feathers::tokens;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::picking::hover::HoverMap;
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
#[derive(Component)]
pub struct UiPanel;
#[derive(Component)]
pub struct ShapeSizeXSlider;
#[derive(Component)]
pub struct ShapeSizeYSlider;
#[derive(Component)]
pub struct ShapeRotationSlider;
#[derive(Component)]
pub struct ShapeEmitRateSlider;
#[derive(Component)]
pub struct SaveSceneButton;

pub fn setup_ui(mut commands: Commands, params: Res<SimParams>, manifest: Res<SceneManifest>) {
    let scene_name = manifest
        .0
        .first()
        .map(|e| e.name.as_str())
        .unwrap_or("None");

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Px(UI_PANEL_WIDTH),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(8.0)),
            row_gap: Val::Px(3.0),
            overflow: Overflow::scroll_y(),
            ..default()
        },
        UiPanel,
        ThemeBackgroundColor(tokens::WINDOW_BG),
        GlobalZIndex(100),
        children![
            // Title
            (
                Text::new("PB-MPM Simulation"),
                ThemedText,
                TextFont {
                    font_size: 16.0,
                    ..default()
                }
            ),
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
                        button(
                            ButtonProps::default(),
                            (),
                            Spawn((Text::new("<"), ThemedText))
                        ),
                        observe(
                            |_: On<Activate>,
                             mut commands: Commands,
                             state: Res<SimState>,
                             manifest: Res<SceneManifest>| {
                                if manifest.0.is_empty() {
                                    return;
                                }
                                let next =
                                    (state.scene_index + manifest.0.len() - 1) % manifest.0.len();
                                commands.trigger(LoadScene(next));
                            }
                        ),
                    ),
                    (
                        Text::new(scene_name),
                        SceneNameLabel,
                        ThemedText,
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        }
                    ),
                    (
                        button(
                            ButtonProps::default(),
                            (),
                            Spawn((Text::new(">"), ThemedText))
                        ),
                        observe(
                            |_: On<Activate>,
                             mut commands: Commands,
                             state: Res<SimState>,
                             manifest: Res<SceneManifest>| {
                                if manifest.0.is_empty() {
                                    return;
                                }
                                let next = (state.scene_index + 1) % manifest.0.len();
                                commands.trigger(LoadScene(next));
                            }
                        ),
                    ),
                ],
            ),
            // Reset / Pause
            (
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(4.0),
                    ..default()
                },
                children![
                    (
                        button(
                            ButtonProps::default(),
                            (),
                            Spawn((Text::new("Reset (F5)"), ThemedText))
                        ),
                        observe(|_: On<Activate>, mut commands: Commands| {
                            commands.trigger(ResetSimulation);
                        }),
                    ),
                    (
                        button(
                            ButtonProps::default(),
                            (),
                            Spawn((Text::new("Pause"), ThemedText))
                        ),
                        observe(|_: On<Activate>, mut s: ResMut<SimState>| {
                            s.is_paused = !s.is_paused;
                        }),
                    ),
                ],
            ),
            // Cycling buttons
            (
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(4.0),
                    align_items: AlignItems::Center,
                    ..default()
                },
                children![
                    (
                        Text::new("Px/Cell"),
                        ThemedText,
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                        Node {
                            width: Val::Px(60.0),
                            ..default()
                        }
                    ),
                    (
                        button(
                            ButtonProps::default(),
                            PixelsPerCellButton,
                            Spawn((Text::new(format!("{}", params.sim_res_divisor)), ThemedText))
                        ),
                        observe(
                            |ev: On<Activate>,
                             mut commands: Commands,
                             mut p: ResMut<SimParams>,
                             q: Query<&Children, With<PixelsPerCellButton>>,
                             mut qt: Query<&mut Text>| {
                                let divs = [1u32, 2, 4, 8, 16];
                                let i = divs
                                    .iter()
                                    .position(|&d| d == p.sim_res_divisor)
                                    .unwrap_or(3);
                                p.sim_res_divisor = divs[(i + 1) % divs.len()];
                                commands.trigger(ResetSimulation);
                                update_btn_text(
                                    ev.event_target(),
                                    &q,
                                    &mut qt,
                                    &format!("{}", p.sim_res_divisor),
                                );
                            }
                        ),
                    ),
                ],
            ),
            (
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(4.0),
                    align_items: AlignItems::Center,
                    ..default()
                },
                children![
                    (
                        Text::new("Sim Rate"),
                        ThemedText,
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                        Node {
                            width: Val::Px(60.0),
                            ..default()
                        }
                    ),
                    (
                        button(
                            ButtonProps::default(),
                            SimRateButton,
                            Spawn((Text::new(format!("{} Hz", params.sim_rate)), ThemedText))
                        ),
                        observe(
                            |ev: On<Activate>,
                             mut p: ResMut<SimParams>,
                             q: Query<&Children, With<SimRateButton>>,
                             mut qt: Query<&mut Text>| {
                                let rates = [15u32, 30, 60, 120, 240, 480, 600, 1200, 2400];
                                let i = rates.iter().position(|&r| r == p.sim_rate).unwrap_or(4);
                                p.sim_rate = rates[(i + 1) % rates.len()];
                                update_btn_text(
                                    ev.event_target(),
                                    &q,
                                    &mut qt,
                                    &format!("{} Hz", p.sim_rate),
                                );
                            }
                        ),
                    ),
                ],
            ),
            (
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(4.0),
                    align_items: AlignItems::Center,
                    ..default()
                },
                children![
                    (
                        Text::new("Mouse"),
                        ThemedText,
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                        Node {
                            width: Val::Px(60.0),
                            ..default()
                        }
                    ),
                    (
                        button(
                            ButtonProps::default(),
                            MouseFnButton,
                            Spawn((Text::new("Grab"), ThemedText))
                        ),
                        observe(
                            |ev: On<Activate>,
                             mut p: ResMut<SimParams>,
                             q: Query<&Children, With<MouseFnButton>>,
                             mut qt: Query<&mut Text>| {
                                p.mouse_function = match p.mouse_function {
                                    MouseFunction::Grab => MouseFunction::Push,
                                    _ => MouseFunction::Grab,
                                };
                                let l = match p.mouse_function {
                                    MouseFunction::Grab => "Grab",
                                    MouseFunction::Push => "Push",
                                };
                                update_btn_text(ev.event_target(), &q, &mut qt, l);
                            }
                        ),
                    ),
                ],
            ),
            (
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(4.0),
                    align_items: AlignItems::Center,
                    ..default()
                },
                children![
                    (
                        Text::new("Render"),
                        ThemedText,
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                        Node {
                            width: Val::Px(60.0),
                            ..default()
                        }
                    ),
                    (
                        button(
                            ButtonProps::default(),
                            RenderModeButton,
                            Spawn((Text::new("Standard"), ThemedText))
                        ),
                        observe(
                            |ev: On<Activate>,
                             mut p: ResMut<SimParams>,
                             q: Query<&Children, With<RenderModeButton>>,
                             mut qt: Query<&mut Text>| {
                                p.render_mode = match p.render_mode {
                                    RenderMode::Standard => RenderMode::Compression,
                                    RenderMode::Compression => RenderMode::Velocity,
                                    _ => RenderMode::Standard,
                                };
                                let l = match p.render_mode {
                                    RenderMode::Standard => "Standard",
                                    RenderMode::Compression => "Compress",
                                    RenderMode::Velocity => "Velocity",
                                };
                                update_btn_text(ev.event_target(), &q, &mut qt, l);
                            }
                        ),
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
            (mk_slider(
                "Particles/Axis",
                ParticlesPerCellSlider,
                1.0,
                8.0,
                params.particles_per_cell_axis as f32,
                1.0,
                0
            )),
            (mk_slider(
                "Gravity",
                GravitySlider,
                0.0,
                5.0,
                params.gravity_strength,
                0.01,
                2
            )),
            (mk_slider(
                "Viscosity",
                ViscositySlider,
                0.0,
                1.0,
                params.liquid_viscosity,
                0.01,
                2
            )),
            (mk_slider(
                "Iterations",
                IterationSlider,
                2.0,
                100.0,
                params.iteration_count as f32,
                1.0,
                0
            )),
            (mk_slider(
                "Elasticity",
                ElasticitySlider,
                0.0,
                1.0,
                params.elasticity_ratio,
                0.01,
                2
            )),
            (mk_slider(
                "Liq Relax",
                LiquidRelaxSlider,
                0.0,
                10.0,
                params.liquid_relaxation,
                0.01,
                2
            )),
            (mk_slider(
                "Elas Relax",
                ElasticRelaxSlider,
                0.0,
                10.0,
                params.elastic_relaxation,
                0.01,
                2
            )),
            (mk_slider(
                "Friction Ang",
                FrictionAngleSlider,
                0.0,
                45.0,
                params.friction_angle,
                0.1,
                1
            )),
            (mk_slider(
                "Plasticity",
                PlasticitySlider,
                0.0,
                1.0,
                params.plasticity,
                0.01,
                2
            )),
            (mk_slider(
                "Border Fric",
                BorderFrictionSlider,
                0.0,
                1.0,
                params.border_friction,
                0.01,
                2
            )),
            (mk_slider(
                "FP Mult Exp",
                FpMultSlider,
                3.0,
                10.0,
                params.fixed_point_multiplier_exponent as f32,
                1.0,
                0
            )),
            // Selected shape info
            (
                Text::new("No shape selected"),
                ShapeInfoLabel,
                ThemedText,
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                Node {
                    margin: UiRect::top(Val::Px(8.0)),
                    ..default()
                },
            ),
            (
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(4.0),
                    align_items: AlignItems::Center,
                    ..default()
                },
                children![
                    (
                        Text::new("Type"),
                        ThemedText,
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                        Node {
                            width: Val::Px(50.0),
                            ..default()
                        }
                    ),
                    (
                        button(
                            ButtonProps::default(),
                            ShapeTypeButton,
                            Spawn((Text::new("--"), ThemedText))
                        ),
                        observe(
                            |ev: On<Activate>,
                             interaction: Res<ShapeInteraction>,
                             mut shapes: Query<&mut SimShapeData>,
                             q: Query<&Children, With<ShapeTypeButton>>,
                             mut qt: Query<&mut Text>| {
                                if let Some(entity) = interaction.selected {
                                    if let Ok(mut shape) = shapes.get_mut(entity) {
                                        let next = shape.shape_type.cycle_next();
                                        shape.shape_type = next;
                                        update_btn_text(
                                            ev.event_target(),
                                            &q,
                                            &mut qt,
                                            next.label(),
                                        );
                                    }
                                }
                            }
                        ),
                    ),
                    (
                        button(
                            ButtonProps::default(),
                            ShapeFunctionButton,
                            Spawn((Text::new("--"), ThemedText))
                        ),
                        observe(
                            |ev: On<Activate>,
                             interaction: Res<ShapeInteraction>,
                             mut shapes: Query<&mut SimShapeData>,
                             q: Query<&Children, With<ShapeFunctionButton>>,
                             mut qt: Query<&mut Text>| {
                                if let Some(entity) = interaction.selected {
                                    if let Ok(mut shape) = shapes.get_mut(entity) {
                                        let next = shape.function.cycle_next();
                                        shape.function = next;
                                        update_btn_text(
                                            ev.event_target(),
                                            &q,
                                            &mut qt,
                                            next.label(),
                                        );
                                    }
                                }
                            }
                        ),
                    ),
                    (
                        button(
                            ButtonProps::default(),
                            ShapeMaterialButton,
                            Spawn((Text::new("--"), ThemedText))
                        ),
                        observe(
                            |ev: On<Activate>,
                             interaction: Res<ShapeInteraction>,
                             mut shapes: Query<&mut SimShapeData>,
                             q: Query<&Children, With<ShapeMaterialButton>>,
                             mut qt: Query<&mut Text>| {
                                if let Some(entity) = interaction.selected {
                                    if let Ok(mut shape) = shapes.get_mut(entity) {
                                        let next = shape.emit_material.cycle_next();
                                        shape.emit_material = next;
                                        update_btn_text(
                                            ev.event_target(),
                                            &q,
                                            &mut qt,
                                            next.label(),
                                        );
                                    }
                                }
                            }
                        ),
                    ),
                ],
            ),
            // Shape size/rotation/emission sliders
            (mk_slider("Size X", ShapeSizeXSlider, 1.0, 500.0, 50.0, 1.0, 0)),
            (mk_slider("Size Y", ShapeSizeYSlider, 1.0, 500.0, 50.0, 1.0, 0)),
            (mk_slider("Rotation", ShapeRotationSlider, -180.0, 180.0, 0.0, 1.0, 0)),
            (mk_slider("Emit Rate", ShapeEmitRateSlider, 0.0, 20.0, 2.5, 0.1, 1)),
            // Save scene button
            (
                button(
                    ButtonProps::default(),
                    SaveSceneButton,
                    Spawn((Text::new("Save Scene (JSON)"), ThemedText)),
                ),
                observe(
                    |_: On<Activate>,
                     state: Res<SimState>,
                     params: Res<SimParams>,
                     windows: Query<&Window>,
                     shapes: Query<&SimShapeData>| {
                        let Ok(window) = windows.single() else {
                            return;
                        };
                        save_scene_to_file(
                            &state,
                            &params,
                            &shapes,
                            window.width(),
                            window.height(),
                        );
                    },
                ),
            ),
            // Stats
            (
                Text::new("Grid: --"),
                GridStatsLabel,
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(Color::srgba(0.5, 0.5, 0.5, 1.0)),
                Node {
                    margin: UiRect::top(Val::Px(8.0)),
                    ..default()
                },
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

/// SystemParam bundling all entity references for the param sliders, used to
/// push current SimParams values back into the slider widgets after scene load.
#[derive(SystemParam)]
pub struct ParamSliderEntities<'w, 's> {
    gravity: Query<'w, 's, Entity, With<GravitySlider>>,
    iterations: Query<'w, 's, Entity, With<IterationSlider>>,
    elasticity: Query<'w, 's, Entity, With<ElasticitySlider>>,
    liq_relax: Query<'w, 's, Entity, With<LiquidRelaxSlider>>,
    elas_relax: Query<'w, 's, Entity, With<ElasticRelaxSlider>>,
    friction: Query<'w, 's, Entity, With<FrictionAngleSlider>>,
    plasticity: Query<'w, 's, Entity, With<PlasticitySlider>>,
    border_fric: Query<'w, 's, Entity, With<BorderFrictionSlider>>,
    viscosity: Query<'w, 's, Entity, With<ViscositySlider>>,
    ppc: Query<'w, 's, Entity, With<ParticlesPerCellSlider>>,
    fp: Query<'w, 's, Entity, With<FpMultSlider>>,
    grid_vol: Query<'w, 's, Entity, With<GridVolumeCheckbox>>,
}

/// Push current SimParams values into the slider widget components so the UI
/// reflects programmatic param changes (e.g. after loading a scene).
fn push_params_to_sliders(
    commands: &mut Commands,
    params: &SimParams,
    entities: &ParamSliderEntities,
) {
    if let Ok(e) = entities.gravity.single() {
        commands
            .entity(e)
            .insert(SliderValue(params.gravity_strength));
    }
    if let Ok(e) = entities.iterations.single() {
        commands
            .entity(e)
            .insert(SliderValue(params.iteration_count as f32));
    }
    if let Ok(e) = entities.elasticity.single() {
        commands
            .entity(e)
            .insert(SliderValue(params.elasticity_ratio));
    }
    if let Ok(e) = entities.liq_relax.single() {
        commands
            .entity(e)
            .insert(SliderValue(params.liquid_relaxation));
    }
    if let Ok(e) = entities.elas_relax.single() {
        commands
            .entity(e)
            .insert(SliderValue(params.elastic_relaxation));
    }
    if let Ok(e) = entities.friction.single() {
        commands
            .entity(e)
            .insert(SliderValue(params.friction_angle));
    }
    if let Ok(e) = entities.plasticity.single() {
        commands.entity(e).insert(SliderValue(params.plasticity));
    }
    if let Ok(e) = entities.border_fric.single() {
        commands
            .entity(e)
            .insert(SliderValue(params.border_friction));
    }
    if let Ok(e) = entities.viscosity.single() {
        commands
            .entity(e)
            .insert(SliderValue(params.liquid_viscosity));
    }
    if let Ok(e) = entities.ppc.single() {
        commands
            .entity(e)
            .insert(SliderValue(params.particles_per_cell_axis as f32));
    }
    if let Ok(e) = entities.fp.single() {
        commands
            .entity(e)
            .insert(SliderValue(params.fixed_point_multiplier_exponent as f32));
    }
    if let Ok(e) = entities.grid_vol.single() {
        if params.use_grid_volume_for_liquid {
            commands.entity(e).insert(Checked);
        } else {
            commands.entity(e).remove::<Checked>();
        }
    }
}

/// Observer for `SceneLoadedEvent`. Handles UI-level synchronization after a
/// scene has been loaded: updates the scene name label and pushes the new
/// `SimParams` values into the slider widgets so the UI reflects them.
pub fn on_scene_loaded(
    trigger: On<SceneLoadedEvent>,
    mut commands: Commands,
    params: Res<SimParams>,
    mut q_name: Query<&mut Text, With<SceneNameLabel>>,
    slider_entities: ParamSliderEntities,
) {
    if let Ok(mut text) = q_name.single_mut() {
        text.0 = trigger.event().name.clone();
    }
    push_params_to_sliders(&mut commands, &params, &slider_entities);
}

/// SystemParam bundling all sliders that drive SimParams.
#[derive(SystemParam)]
pub struct ParamSliderQueries<'w, 's> {
    gravity: Query<'w, 's, &'static SliderValue, With<GravitySlider>>,
    iterations: Query<'w, 's, &'static SliderValue, With<IterationSlider>>,
    elasticity: Query<'w, 's, &'static SliderValue, With<ElasticitySlider>>,
    liq_relax: Query<'w, 's, &'static SliderValue, With<LiquidRelaxSlider>>,
    elas_relax: Query<'w, 's, &'static SliderValue, With<ElasticRelaxSlider>>,
    friction: Query<'w, 's, &'static SliderValue, With<FrictionAngleSlider>>,
    plasticity: Query<'w, 's, &'static SliderValue, With<PlasticitySlider>>,
    border_fric: Query<'w, 's, &'static SliderValue, With<BorderFrictionSlider>>,
    viscosity: Query<'w, 's, &'static SliderValue, With<ViscositySlider>>,
    ppc: Query<'w, 's, &'static SliderValue, With<ParticlesPerCellSlider>>,
    fp: Query<'w, 's, &'static SliderValue, With<FpMultSlider>>,
    grid_vol: Query<'w, 's, Has<Checked>, With<GridVolumeCheckbox>>,
}

/// Sync slider values back to SimParams every frame
pub fn sync_params(sliders: ParamSliderQueries, mut params: ResMut<SimParams>) {
    if let Ok(v) = sliders.gravity.single() {
        params.gravity_strength = v.0;
    }
    if let Ok(v) = sliders.iterations.single() {
        params.iteration_count = v.0 as u32;
    }
    if let Ok(v) = sliders.elasticity.single() {
        params.elasticity_ratio = v.0;
    }
    if let Ok(v) = sliders.liq_relax.single() {
        params.liquid_relaxation = v.0;
    }
    if let Ok(v) = sliders.elas_relax.single() {
        params.elastic_relaxation = v.0;
    }
    if let Ok(v) = sliders.friction.single() {
        params.friction_angle = v.0;
    }
    if let Ok(v) = sliders.plasticity.single() {
        params.plasticity = v.0;
    }
    if let Ok(v) = sliders.border_fric.single() {
        params.border_friction = v.0;
    }
    if let Ok(v) = sliders.viscosity.single() {
        params.liquid_viscosity = v.0;
    }
    if let Ok(v) = sliders.ppc.single() {
        params.particles_per_cell_axis = v.0 as u32;
    }
    if let Ok(v) = sliders.fp.single() {
        params.fixed_point_multiplier_exponent = v.0 as u32;
    }
    if let Ok(checked) = sliders.grid_vol.single() {
        params.use_grid_volume_for_liquid = checked;
    }
}

/// Update the shape info label and button text based on current selection
pub fn update_shape_info(
    interaction: Res<ShapeInteraction>,
    shapes: Query<&SimShapeData>,
    mut q_info: Query<&mut Text, With<ShapeInfoLabel>>,
    q_type_btn: Query<&Children, With<ShapeTypeButton>>,
    q_func_btn: Query<&Children, With<ShapeFunctionButton>>,
    q_mat_btn: Query<&Children, With<ShapeMaterialButton>>,
    mut q_text: Query<&mut Text, (Without<ShapeInfoLabel>, Without<GridStatsLabel>)>,
) {
    let info_text = if let Some(entity) = interaction.selected {
        if let Ok(shape) = shapes.get(entity) {
            let shape_type = shape.shape_type.label();
            let func = shape.function.label();
            let mat = shape.emit_material.label();

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

/// Toggle UI panel visibility with backtick key
pub fn toggle_ui(keys: Res<ButtonInput<KeyCode>>, mut q_panel: Query<&mut Node, With<UiPanel>>) {
    if keys.just_pressed(KeyCode::Backquote) {
        if let Ok(mut node) = q_panel.single_mut() {
            if node.display == Display::None {
                node.display = Display::Flex;
            } else {
                node.display = Display::None;
            }
        }
    }
}

/// Sync shape property sliders bidirectionally with the selected shape.
#[allow(clippy::too_many_arguments)]
pub fn sync_shape_sliders(
    mut commands: Commands,
    interaction: Res<ShapeInteraction>,
    mut shapes: Query<&mut SimShapeData>,
    q_size_x: Query<(Entity, &SliderValue), With<ShapeSizeXSlider>>,
    q_size_y: Query<(Entity, &SliderValue), With<ShapeSizeYSlider>>,
    q_rotation: Query<(Entity, &SliderValue), With<ShapeRotationSlider>>,
    q_emit_rate: Query<(Entity, &SliderValue), With<ShapeEmitRateSlider>>,
    mut prev_selection: Local<Option<Entity>>,
) {
    let Some(entity) = interaction.selected else {
        *prev_selection = None;
        return;
    };
    let selection_changed = *prev_selection != Some(entity);
    *prev_selection = Some(entity);

    if selection_changed {
        // Push shape values to sliders when selection changes
        if let Ok(shape) = shapes.get(entity) {
            let is_circle = shape.shape_type.is_circle();
            if let Ok((e, _)) = q_size_x.single() {
                let val = if is_circle {
                    shape.radius
                } else {
                    shape.half_size.x
                };
                commands.entity(e).insert(SliderValue(val));
            }
            if let Ok((e, _)) = q_size_y.single() {
                let val = if is_circle {
                    shape.radius
                } else {
                    shape.half_size.y
                };
                commands.entity(e).insert(SliderValue(val));
            }
            if let Ok((e, _)) = q_rotation.single() {
                commands.entity(e).insert(SliderValue(shape.rotation));
            }
            if let Ok((e, _)) = q_emit_rate.single() {
                commands.entity(e).insert(SliderValue(shape.emission_rate));
            }
        }
    } else {
        // Pull slider values into shape
        if let Ok(mut shape) = shapes.get_mut(entity) {
            let is_circle = shape.shape_type.is_circle();
            if let Ok((_, v)) = q_size_x.single() {
                if is_circle {
                    shape.radius = v.0;
                } else {
                    shape.half_size.x = v.0;
                }
            }
            if let Ok((_, v)) = q_size_y.single() {
                if is_circle {
                    shape.radius = v.0;
                } else {
                    shape.half_size.y = v.0;
                }
            }
            if let Ok((_, v)) = q_rotation.single() {
                shape.rotation = v.0;
            }
            if let Ok((_, v)) = q_emit_rate.single() {
                shape.emission_rate = v.0;
            }
        }
    }
}

/// Save current scene to a JSON file.
fn save_scene_to_file(
    _state: &SimState,
    params: &SimParams,
    shapes: &Query<&SimShapeData>,
    width: f32,
    height: f32,
) {
    use serde::Serialize;

    #[derive(Serialize)]
    struct SavedScene {
        version: u32,
        resolution: [f64; 2],
        settings: Vec<SavedSetting>,
        shapes: Vec<SimShape>,
    }

    #[derive(Serialize)]
    struct SavedSetting {
        name: String,
        value: serde_json::Value,
        #[serde(rename = "type")]
        setting_type: String,
    }

    let mut settings = Vec::new();
    let defaults = SimParams::default();

    macro_rules! save_if_changed {
        ($name:expr, $field:ident, $kind:expr) => {
            if params.$field != defaults.$field {
                settings.push(SavedSetting {
                    name: $name.to_string(),
                    value: serde_json::json!(params.$field),
                    setting_type: $kind.to_string(),
                });
            }
        };
    }

    save_if_changed!("iterationCount", iteration_count, "range");
    save_if_changed!("simResDivisor", sim_res_divisor, "combo");
    save_if_changed!("particlesPerCellAxis", particles_per_cell_axis, "range");
    save_if_changed!("gravityStrength", gravity_strength, "range");
    save_if_changed!("liquidViscosity", liquid_viscosity, "range");
    save_if_changed!("elasticityRatio", elasticity_ratio, "range");
    save_if_changed!("liquidRelaxation", liquid_relaxation, "range");
    save_if_changed!("elasticRelaxation", elastic_relaxation, "range");
    save_if_changed!("frictionAngle", friction_angle, "range");
    save_if_changed!("plasticity", plasticity, "range");
    save_if_changed!("borderFriction", border_friction, "range");

    let sim_shapes: Vec<SimShape> = shapes.iter().map(SimShape::from).collect();

    let scene = SavedScene {
        version: 2,
        resolution: [width as f64, height as f64],
        settings,
        shapes: sim_shapes,
    };

    let path = "saved_scene.json";
    match serde_json::to_string_pretty(&scene) {
        Ok(json) => {
            if std::fs::write(path, &json).is_ok() {
                info!("Scene saved to {path}");
            } else {
                warn!("Failed to write scene file");
            }
        }
        Err(e) => warn!("Failed to serialize scene: {e}"),
    }
}

/// Update stats label
pub fn update_stats(
    params: Res<SimParams>,
    particle_count: Res<ParticleCount>,
    shapes: Query<&SimShapeData>,
    windows: Query<&Window>,
    mut q_stats: Query<&mut Text, With<GridStatsLabel>>,
) {
    let Ok(window) = windows.single() else { return };
    let gx = (window.width() / params.sim_res_divisor as f32).max(1.0) as u32;
    let gy = (window.height() / params.sim_res_divisor as f32).max(1.0) as u32;
    if let Ok(mut text) = q_stats.single_mut() {
        let count = particle_count.get();
        text.0 = format!(
            "Grid: {}x{} | Shapes: {} | Particles: {:.1}k",
            gx,
            gy,
            shapes.iter().count(),
            count as f32 / 1000.0
        );
    }
}

// --- UI Scroll support ---
// Bevy's Overflow::scroll_y() sets layout but doesn't auto-handle scroll events.
// We need to pipe MouseWheel events into ScrollPosition updates, same as Bevy's scroll example.

const SCROLL_LINE_HEIGHT: f32 = 21.0;

/// Custom scroll event that propagates up the UI hierarchy.
#[derive(EntityEvent, Debug)]
#[entity_event(propagate, auto_propagate)]
pub struct UiScroll {
    entity: Entity,
    delta: Vec2,
}

/// Read mouse wheel events and trigger UiScroll on hovered UI entities.
pub fn send_scroll_events(
    mut mouse_wheel: MessageReader<MouseWheel>,
    hover_map: Res<HoverMap>,
    mut commands: Commands,
) {
    for wheel in mouse_wheel.read() {
        let mut delta = -Vec2::new(wheel.x, wheel.y);
        if wheel.unit == MouseScrollUnit::Line {
            delta *= SCROLL_LINE_HEIGHT;
        }
        for pointer_map in hover_map.values() {
            for &entity in pointer_map.keys() {
                commands.trigger(UiScroll { entity, delta });
            }
        }
    }
}

/// Handle UiScroll events by updating ScrollPosition on scrollable nodes.
pub fn on_scroll(
    mut scroll: On<UiScroll>,
    mut query: Query<(&mut ScrollPosition, &Node, &ComputedNode)>,
) {
    let Ok((mut scroll_position, node, computed)) = query.get_mut(scroll.entity) else {
        return;
    };
    let max_offset = (computed.content_size() - computed.size()) * computed.inverse_scale_factor();
    let delta = &mut scroll.delta;

    if node.overflow.y == OverflowAxis::Scroll && delta.y != 0.0 {
        let at_limit = if delta.y > 0.0 {
            scroll_position.y >= max_offset.y
        } else {
            scroll_position.y <= 0.0
        };
        if !at_limit {
            scroll_position.y += delta.y;
            delta.y = 0.0;
        }
    }

    if *delta == Vec2::ZERO {
        scroll.propagate(false);
    }
}
