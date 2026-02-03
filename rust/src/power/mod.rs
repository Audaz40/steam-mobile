//! Power and thermal management module.
//!
//! Provides thermal state tracking, frame pacing, and performance governance.

/// Thermal state of the device.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThermalState {
    Nominal,
    Warm,
    Hot,
    Critical,
}

/// Thermal policy thresholds and FPS limits.
#[derive(Debug, Clone, Copy)]
pub struct ThermalPolicy {
    pub warm_threshold_c: f32,
    pub hot_threshold_c: f32,
    pub critical_threshold_c: f32,
    pub nominal_fps: u32,
    pub warm_fps: u32,
    pub hot_fps: u32,
    pub critical_fps: u32,
}

impl Default for ThermalPolicy {
    fn default() -> Self {
        Self {
            warm_threshold_c: 40.0,
            hot_threshold_c: 47.0,
            critical_threshold_c: 55.0,
            nominal_fps: 60,
            warm_fps: 45,
            hot_fps: 30,
            critical_fps: 15,
        }
    }
}

/// Thermal manager that tracks temperature and recommends throttling.
pub struct ThermalManager {
    policy: ThermalPolicy,
    current_temp_c: f32,
    state: ThermalState,
}

impl ThermalManager {
    pub fn new(policy: ThermalPolicy) -> Self {
        Self {
            policy,
            current_temp_c: 35.0,
            state: ThermalState::Nominal,
        }
    }

    pub fn update_temperature(&mut self, temperature_c: f32) -> ThermalState {
        self.current_temp_c = temperature_c;
        self.state = if temperature_c >= self.policy.critical_threshold_c {
            ThermalState::Critical
        } else if temperature_c >= self.policy.hot_threshold_c {
            ThermalState::Hot
        } else if temperature_c >= self.policy.warm_threshold_c {
            ThermalState::Warm
        } else {
            ThermalState::Nominal
        };
        self.state
    }

    pub fn state(&self) -> ThermalState {
        self.state
    }

    pub fn recommended_fps(&self) -> u32 {
        match self.state {
            ThermalState::Nominal => self.policy.nominal_fps,
            ThermalState::Warm => self.policy.warm_fps,
            ThermalState::Hot => self.policy.hot_fps,
            ThermalState::Critical => self.policy.critical_fps,
        }
    }

    pub fn throttle_factor(&self) -> f32 {
        let nominal = self.policy.nominal_fps.max(1) as f32;
        self.recommended_fps() as f32 / nominal
    }

    pub fn current_temperature(&self) -> f32 {
        self.current_temp_c
    }
}

/// Frame pacing helper to compute frame budgets.
#[derive(Debug, Clone, Copy)]
pub struct FramePacer {
    target_fps: u32,
}

impl FramePacer {
    pub fn new(target_fps: u32) -> Self {
        Self {
            target_fps: target_fps.max(1),
        }
    }

    pub fn update_target_fps(&mut self, target_fps: u32) {
        self.target_fps = target_fps.max(1);
    }

    pub fn target_fps(&self) -> u32 {
        self.target_fps
    }

    pub fn frame_budget_ms(&self) -> u32 {
        1000 / self.target_fps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thermal_transitions() {
        let mut manager = ThermalManager::new(ThermalPolicy::default());
        assert_eq!(manager.update_temperature(36.0), ThermalState::Nominal);
        assert_eq!(manager.update_temperature(45.0), ThermalState::Warm);
        assert_eq!(manager.update_temperature(50.0), ThermalState::Hot);
        assert_eq!(manager.update_temperature(58.0), ThermalState::Critical);
    }
}
