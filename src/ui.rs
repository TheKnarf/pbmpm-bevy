use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::scene::*;
use crate::types::*;

pub fn ui_system(
    mut contexts: EguiContexts,
    mut params: ResMut<SimParams>,
    mut sim_state: ResMut<SimState>,
    manifest: Res<SceneManifest>,
    windows: Query<&Window>,
) {
    let Some(ctx) = contexts.try_ctx_mut() else {
        return;
    };

    egui::SidePanel::left("settings")
        .default_width(280.0)
        .show(ctx, |ui| {
            ui.heading("PB-MPM Simulation");
            ui.separator();

            // Scene selector
            let scene_names: Vec<&str> = manifest.0.iter().map(|e| e.name.as_str()).collect();
            if !scene_names.is_empty() {
                let mut scene_idx = sim_state.scene_index;
                egui::ComboBox::from_label("Scene")
                    .selected_text(scene_names.get(scene_idx).copied().unwrap_or(""))
                    .show_ui(ui, |ui| {
                        for (i, name) in scene_names.iter().enumerate() {
                            ui.selectable_value(&mut scene_idx, i, *name);
                        }
                    });
                if scene_idx != sim_state.scene_index {
                    sim_state.scene_index = scene_idx;
                    if let Some(entry) = manifest.0.get(scene_idx) {
                        if let Some(scene) = load_scene(&entry.scene) {
                            let Ok(window) = windows.single() else { return };
                            let mut new_params = SimParams::default();
                            apply_scene(
                                &scene,
                                &mut sim_state,
                                &mut new_params,
                                window.width(),
                                window.height(),
                            );
                            *params = new_params;
                        }
                    }
                }
            }

            ui.horizontal(|ui| {
                if ui.button("Reset (F5)").clicked() {
                    sim_state.do_reset = true;
                }
                if ui.button(if sim_state.is_paused {
                    "Resume"
                } else {
                    "Pause"
                })
                .clicked()
                {
                    sim_state.is_paused = !sim_state.is_paused;
                }
            });

            ui.separator();
            ui.label("Simulation Settings");

            // Sim Res Divisor
            let divisors = [1u32, 2, 4, 8, 16];
            let mut divisor_idx = divisors
                .iter()
                .position(|&d| d == params.sim_res_divisor)
                .unwrap_or(3);
            egui::ComboBox::from_label("Pixels per Cell")
                .selected_text(format!("{}", params.sim_res_divisor))
                .show_ui(ui, |ui| {
                    for (i, d) in divisors.iter().enumerate() {
                        ui.selectable_value(&mut divisor_idx, i, format!("{}", d));
                    }
                });
            let old_divisor = params.sim_res_divisor;
            params.sim_res_divisor = divisors[divisor_idx];
            if params.sim_res_divisor != old_divisor {
                sim_state.do_reset = true;
            }

            ui.add(
                egui::Slider::new(&mut params.particles_per_cell_axis, 1..=8)
                    .text("Particles/Cell Axis"),
            );

            // Sim Rate
            let rates = [15u32, 30, 60, 120, 240, 480, 600, 1200, 2400];
            let mut rate_idx = rates
                .iter()
                .position(|&r| r == params.sim_rate)
                .unwrap_or(4);
            egui::ComboBox::from_label("Sim Rate (Hz)")
                .selected_text(format!("{}", params.sim_rate))
                .show_ui(ui, |ui| {
                    for (i, r) in rates.iter().enumerate() {
                        ui.selectable_value(&mut rate_idx, i, format!("{}", r));
                    }
                });
            params.sim_rate = rates[rate_idx];

            ui.checkbox(
                &mut params.use_grid_volume_for_liquid,
                "Grid Volume for Liquid",
            );

            ui.add(
                egui::Slider::new(&mut params.fixed_point_multiplier_exponent, 3..=10)
                    .text("log10(FP Mult)"),
            );
            ui.add(
                egui::Slider::new(&mut params.gravity_strength, 0.0..=5.0).text("Gravity"),
            );
            ui.add(
                egui::Slider::new(&mut params.liquid_viscosity, 0.0..=1.0)
                    .text("Liquid Viscosity"),
            );

            // Mouse function
            let mut mf = params.mouse_function as usize;
            egui::ComboBox::from_label("Mouse Function")
                .selected_text(match params.mouse_function {
                    MouseFunction::Push => "Push",
                    MouseFunction::Grab => "Grab",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut mf, 0, "Push");
                    ui.selectable_value(&mut mf, 1, "Grab");
                });
            params.mouse_function = if mf == 0 {
                MouseFunction::Push
            } else {
                MouseFunction::Grab
            };

            ui.add(
                egui::Slider::new(&mut params.iteration_count, 2..=100)
                    .text("Iteration Count"),
            );
            ui.add(
                egui::Slider::new(&mut params.elasticity_ratio, 0.0..=1.0)
                    .text("Elasticity Ratio"),
            );
            ui.add(
                egui::Slider::new(&mut params.liquid_relaxation, 0.0..=10.0)
                    .text("Liquid Relaxation"),
            );
            ui.add(
                egui::Slider::new(&mut params.elastic_relaxation, 0.0..=10.0)
                    .text("Elastic Relaxation"),
            );
            ui.add(
                egui::Slider::new(&mut params.friction_angle, 0.0..=45.0)
                    .text("Sand Friction Angle"),
            );
            ui.add(
                egui::Slider::new(&mut params.plasticity, 0.0..=1.0).text("Visco Plasticity"),
            );
            ui.add(
                egui::Slider::new(&mut params.border_friction, 0.0..=1.0)
                    .text("Border Friction"),
            );

            // Render mode
            let mut rm = params.render_mode as usize;
            egui::ComboBox::from_label("Render Mode")
                .selected_text(match params.render_mode {
                    RenderMode::Standard => "Standard",
                    RenderMode::Compression => "Compression",
                    RenderMode::Velocity => "Velocity",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut rm, 0, "Standard");
                    ui.selectable_value(&mut rm, 1, "Compression");
                    ui.selectable_value(&mut rm, 2, "Velocity");
                });
            params.render_mode = match rm {
                1 => RenderMode::Compression,
                2 => RenderMode::Velocity,
                _ => RenderMode::Standard,
            };

            ui.separator();
            ui.label(format!(
                "Grid: {}x{}",
                sim_state.grid_size[0], sim_state.grid_size[1]
            ));
            ui.label(format!(
                "Shapes: {}",
                sim_state.shapes.len()
            ));
        });
}
