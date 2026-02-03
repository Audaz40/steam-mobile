//! SteamMobile Core Library
//! 
//! This library provides the core functionality for running PC games on mobile devices,
//! including x86 to ARM binary translation, Windows compatibility, and graphics translation.

pub mod cpu;
pub mod memory;
pub mod translator;
pub mod pe;
pub mod syscall;
pub mod ffi;
pub mod graphics;
pub mod dxvk;
pub mod input;
pub mod power;
pub mod runtime;

use std::sync::Arc;
use std::sync::Mutex;

/// Main emulation engine that coordinates all components
pub struct EmulationEngine {
    cpu: Arc<Mutex<cpu::CPU>>,
    memory: Arc<Mutex<memory::MemoryManager>>,
    translator: Arc<Mutex<translator::BinaryTranslator>>,
}

impl EmulationEngine {
    /// Creates a new emulation engine instance
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let memory = Arc::new(Mutex::new(memory::MemoryManager::new()?));
        let cpu = Arc::new(Mutex::new(cpu::CPU::new(memory.clone())?));
        let translator = Arc::new(Mutex::new(translator::BinaryTranslator::new()?));

        Ok(EmulationEngine {
            cpu,
            memory,
            translator,
        })
    }

    /// Loads and executes a Windows PE executable
    pub fn execute_pe(&mut self, pe_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut pe_loader = pe::PELoader::new();
        let entry_point = pe_loader.load(pe_path, self.memory.clone())?;
        
        // Set CPU to entry point and start execution
        {
            let mut cpu = self.cpu.lock().unwrap();
            cpu.set_instruction_pointer(entry_point);
        }

        Ok(())
    }

    /// Executes a single instruction and returns step stats.
    pub fn step(&mut self) -> Result<ExecutionStep, Box<dyn std::error::Error>> {
        let instruction = {
            let mut cpu = self.cpu.lock().unwrap();
            cpu.fetch_instruction()?
        };

        let translated_len = {
            let mut translator = self.translator.lock().unwrap();
            let translated = translator.translate_instruction(instruction.clone())?;
            translated.len()
        };

        {
            let mut cpu = self.cpu.lock().unwrap();
            cpu.execute_instruction(instruction)?;
        }

        Ok(ExecutionStep { translated_len })
    }

    /// Executes up to the given instruction budget.
    pub fn run_for(&mut self, instruction_budget: u32) -> Result<ExecutionStats, Box<dyn std::error::Error>> {
        let mut executed = 0u32;
        while executed < instruction_budget {
            self.step()?;
            executed += 1;
        }

        let ip = self.cpu.lock().unwrap().get_instruction_pointer();

        Ok(ExecutionStats {
            instructions_executed: executed as u64,
            last_instruction_pointer: ip,
        })
    }

    /// Checks thermal state and adjusts performance accordingly
    fn check_thermal_state(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Implementation would interface with Android thermal management
        Ok(())
    }
}

/// Execution step statistics
#[derive(Debug)]
pub struct ExecutionStep {
    pub translated_len: usize,
}

/// Execution statistics for a run
#[derive(Debug)]
pub struct ExecutionStats {
    pub instructions_executed: u64,
    pub last_instruction_pointer: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = EmulationEngine::new();
        assert!(engine.is_ok());
    }
}
