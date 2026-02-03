//! High-level runtime orchestration.
//!
//! Coordinates CPU emulation, system calls, graphics translation, input, and power policies.

use crate::dxvk::DXVKTranslator;
use crate::input::{InputAction, InputManager, InputOverlay, TouchPoint};
use crate::power::{FramePacer, ThermalManager, ThermalPolicy, ThermalState};
use crate::syscall::SyscallEmulator;
use crate::{EmulationEngine, ExecutionStats};

/// Configuration for running a game session.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub instruction_budget_per_frame: u32,
    pub thermal_policy: ThermalPolicy,
    pub target_fps: u32,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            instruction_budget_per_frame: 50_000,
            thermal_policy: ThermalPolicy::default(),
            target_fps: 60,
        }
    }
}

/// Game profile describing launch parameters.
#[derive(Debug, Clone)]
pub struct GameProfile {
    pub exe_path: String,
    pub arguments: Vec<String>,
    pub working_directory: Option<String>,
}

/// Runtime telemetry for a single frame.
#[derive(Debug)]
pub struct FrameSummary {
    pub instructions_executed: u64,
    pub last_instruction_pointer: u32,
    pub thermal_state: ThermalState,
    pub target_fps: u32,
    pub input_actions: Vec<InputAction>,
}

/// Main runtime orchestrator.
pub struct SteamMobileRuntime {
    engine: EmulationEngine,
    syscall: SyscallEmulator,
    graphics: DXVKTranslator,
    input: InputManager,
    thermal: ThermalManager,
    frame_pacer: FramePacer,
    config: RuntimeConfig,
}

impl SteamMobileRuntime {
    pub fn new(config: RuntimeConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let engine = EmulationEngine::new()?;
        let mut graphics = DXVKTranslator::new()?;
        graphics.initialize()?;

        let overlay = InputOverlay::new();
        let input = InputManager::new(overlay);
        let thermal = ThermalManager::new(config.thermal_policy);
        let frame_pacer = FramePacer::new(config.target_fps);

        Ok(Self {
            engine,
            syscall: SyscallEmulator::new(),
            graphics,
            input,
            thermal,
            frame_pacer,
            config,
        })
    }

    pub fn load_game(&mut self, profile: &GameProfile) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(dir) = &profile.working_directory {
            std::env::set_current_dir(dir)?;
        }
        self.engine.execute_pe(&profile.exe_path)?;
        Ok(())
    }

    pub fn ingest_touch(&mut self, touch: TouchPoint) {
        self.input.handle_touch(touch);
    }

    pub fn update_temperature(&mut self, temperature_c: f32) {
        let state = self.thermal.update_temperature(temperature_c);
        self.frame_pacer
            .update_target_fps(self.thermal.recommended_fps());
        if matches!(state, ThermalState::Critical) {
            self.config.instruction_budget_per_frame =
                (self.config.instruction_budget_per_frame / 2).max(10_000);
        }
    }

    pub fn tick_frame(&mut self) -> Result<FrameSummary, Box<dyn std::error::Error>> {
        let throttle = self.thermal.throttle_factor();
        let budget = ((self.config.instruction_budget_per_frame as f32) * throttle)
            .round()
            .max(1.0) as u32;

        let stats: ExecutionStats = self.engine.run_for(budget)?;
        let actions = self.input.drain_actions();

        Ok(FrameSummary {
            instructions_executed: stats.instructions_executed,
            last_instruction_pointer: stats.last_instruction_pointer,
            thermal_state: self.thermal.state(),
            target_fps: self.frame_pacer.target_fps(),
            input_actions: actions,
        })
    }

    pub fn graphics_mut(&mut self) -> &mut DXVKTranslator {
        &mut self.graphics
    }

    pub fn syscall_mut(&mut self) -> &mut SyscallEmulator {
        &mut self.syscall
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_creation() {
        let runtime = SteamMobileRuntime::new(RuntimeConfig::default());
        assert!(runtime.is_ok());
    }
}
