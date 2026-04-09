use bevy::prelude::*;

#[derive(Resource)]
pub struct TimeRegulation {
    prev_time_ms: f64,
    time_accumulator_ms: f64,
    estimated_render_timestep_ms: f64,
    frames_above_target: u32,
    sim_frame_count_cap: u32,
}

impl Default for TimeRegulation {
    fn default() -> Self {
        Self {
            prev_time_ms: 0.0,
            time_accumulator_ms: 0.0,
            estimated_render_timestep_ms: 1000.0 / 60.0,
            frames_above_target: 0,
            sim_frame_count_cap: 20,
        }
    }
}

impl TimeRegulation {
    pub fn compute_substeps(
        &mut self,
        current_time_ms: f64,
        sim_rate: u32,
        do_pause: bool,
        do_reset: bool,
    ) -> u32 {
        if do_reset {
            self.estimated_render_timestep_ms = 1000.0 / 60.0;
            self.prev_time_ms = current_time_ms - self.estimated_render_timestep_ms;
            self.sim_frame_count_cap = 20;
            self.frames_above_target = 0;
            self.time_accumulator_ms = 0.0;
        }

        if self.prev_time_ms == 0.0 {
            self.prev_time_ms = current_time_ms - self.estimated_render_timestep_ms;
        }

        let delta_time_ms = current_time_ms - self.prev_time_ms;

        if delta_time_ms < self.estimated_render_timestep_ms {
            self.estimated_render_timestep_ms = delta_time_ms;
        } else if delta_time_ms > 2.0 * self.estimated_render_timestep_ms {
            self.frames_above_target += 1;
        } else {
            self.frames_above_target = 0;
        }

        if self.frames_above_target >= 10 {
            self.frames_above_target = 0;
        }

        self.prev_time_ms = current_time_ms;
        self.time_accumulator_ms += delta_time_ms;

        let mut substep_count =
            (self.time_accumulator_ms * sim_rate as f64 / 1000.0).floor() as u32;

        if do_pause {
            substep_count = 0;
            self.time_accumulator_ms = 0.0;
        } else {
            self.time_accumulator_ms -= 1000.0 * (substep_count as f64 / sim_rate as f64);
        }

        if substep_count > self.sim_frame_count_cap {
            substep_count = self.sim_frame_count_cap;
        }

        substep_count
    }

    pub fn last_render_timestep_secs(&self) -> f64 {
        self.estimated_render_timestep_ms / 1000.0
    }
}
