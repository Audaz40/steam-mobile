//! Binary translation module
//! 
//! Translates x86 instructions to ARM instructions for native execution

use crate::cpu::{X86Instruction, InstructionOperand, Register};

/// ARM instruction representation
#[derive(Debug, Clone)]
pub struct ARMInstruction {
    pub opcode: ARMOpcode,
    pub rd: Option<ARMRegister>,
    pub rn: Option<ARMRegister>,
    pub rm: Option<ARMRegister>,
    pub immediate: Option<u32>,
    pub condition: ARMCondition,
}

#[derive(Debug, Clone)]
pub enum ARMOpcode {
    // Data processing
    MOV,
    ADD,
    SUB,
    MUL,
    AND,
    ORR,
    EOR,
    CMP,
    
    // Memory operations
    LDR,
    STR,
    LDM,
    STM,
    
    // Branch operations
    B,
    BL,
    BX,
    
    // Special
    NOP,
}

#[derive(Debug, Clone, Copy)]
pub enum ARMRegister {
    R0, R1, R2, R3, R4, R5, R6, R7,
    R8, R9, R10, R11, R12, SP, LR, PC,
}

#[derive(Debug, Clone)]
pub enum ARMCondition {
    EQ, NE, CS, CC, MI, PL, VS, VC,
    HI, LS, GE, LT, GT, LE, AL, NV,
}

/// Binary translator that converts x86 instructions to ARM
pub struct BinaryTranslator {
    translation_cache: std::collections::HashMap<Vec<u8>, Vec<ARMInstruction>>,
    register_mapping: std::collections::HashMap<Register, ARMRegister>,
}

impl BinaryTranslator {
    /// Creates a new binary translator
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut translator = BinaryTranslator {
            translation_cache: std::collections::HashMap::new(),
            register_mapping: std::collections::HashMap::new(),
        };
        
        // Initialize register mapping
        translator.init_register_mapping();
        
        Ok(translator)
    }

    /// Initializes x86 to ARM register mapping
    fn init_register_mapping(&mut self) {
        self.register_mapping.insert(Register::EAX, ARMRegister::R0);
        self.register_mapping.insert(Register::EBX, ARMRegister::R1);
        self.register_mapping.insert(Register::ECX, ARMRegister::R2);
        self.register_mapping.insert(Register::EDX, ARMRegister::R3);
        self.register_mapping.insert(Register::ESI, ARMRegister::R4);
        self.register_mapping.insert(Register::EDI, ARMRegister::R5);
        self.register_mapping.insert(Register::EBP, ARMRegister::R6);
        self.register_mapping.insert(Register::ESP, ARMRegister::SP);
    }

    /// Translates a single x86 instruction to ARM
    pub fn translate_instruction(&mut self, x86_inst: X86Instruction) -> Result<Vec<ARMInstruction>, Box<dyn std::error::Error>> {
        // Check cache first
        let x86_bytes = self.x86_instruction_to_bytes(&x86_inst);
        if let Some(cached_translation) = self.translation_cache.get(&x86_bytes) {
            return Ok(cached_translation.clone());
        }

        // Translate instruction
        let arm_instructions = self.translate_instruction_internal(&x86_inst)?;
        
        // Cache the translation
        self.translation_cache.insert(x86_bytes, arm_instructions.clone());
        
        Ok(arm_instructions)
    }

    /// Converts x86 instruction to byte representation for caching
    fn x86_instruction_to_bytes(&self, inst: &X86Instruction) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(inst.opcode);
        
        if let Some(modrm) = inst.modrm {
            bytes.push(modrm);
        }
        
        if let Some(sib) = inst.sib {
            bytes.push(sib);
        }
        
        if let Some(disp) = inst.displacement {
            bytes.extend_from_slice(&disp.to_le_bytes());
        }
        
        if let Some(imm) = inst.immediate {
            bytes.extend_from_slice(&imm.to_le_bytes());
        }
        
        bytes
    }

    /// Internal translation logic
    fn translate_instruction_internal(&mut self, x86_inst: &X86Instruction) -> Result<Vec<ARMInstruction>, Box<dyn std::error::Error>> {
        match x86_inst.opcode {
            0x90 => Ok(vec![ARMInstruction {
                opcode: ARMOpcode::NOP,
                rd: None,
                rn: None,
                rm: None,
                immediate: None,
                condition: ARMCondition::AL,
            }]),
            
            0xB8..=0xBF => { // MOV reg32, imm32
                let target_reg = self.map_x86_register(x86_inst.opcode - 0xB8)?;
                let immediate = x86_inst.immediate.ok_or("Missing immediate for MOV reg, imm")?;
                
                Ok(vec![ARMInstruction {
                    opcode: ARMOpcode::MOV,
                    rd: Some(target_reg),
                    rn: None,
                    rm: None,
                    immediate: Some(immediate),
                    condition: ARMCondition::AL,
                }])
            }
            
            0x89 => { // MOV r/m32, r32
                self.translate_mov_reg_to_mem(x86_inst)
            }
            
            0x8B => { // MOV r32, r/m32
                self.translate_mov_mem_to_reg(x86_inst)
            }
            
            0x01 => { // ADD r/m32, r32
                self.translate_add_reg_to_mem(x86_inst)
            }
            
            0x03 => { // ADD r32, r/m32
                self.translate_add_mem_to_reg(x86_inst)
            }
            
            0xFF => { // Various instructions
                self.translate_ff_group(x86_inst)
            }
            
            _ => Err(format!("Unsupported x86 opcode: 0x{:02X}", x86_inst.opcode).into()),
        }
    }

    /// Translates MOV r/m32, r32
    fn translate_mov_reg_to_mem(&mut self, x86_inst: &X86Instruction) -> Result<Vec<ARMInstruction>, Box<dyn std::error::Error>> {
        if x86_inst.operands.len() != 2 {
            return Err("MOV instruction requires 2 operands".into());
        }

        let src_reg = match &x86_inst.operands[1] {
            InstructionOperand::Register(reg) => self.map_x86_register_enum(reg)?,
            _ => return Err("MOV r/m32, r32: source must be register".into()),
        };

        match &x86_inst.operands[0] {
            InstructionOperand::Register(dest_reg) => {
                let dest_arm_reg = self.map_x86_register_enum(dest_reg)?;
                Ok(vec![ARMInstruction {
                    opcode: ARMOpcode::MOV,
                    rd: Some(dest_arm_reg),
                    rn: Some(src_reg.clone()),
                    rm: None,
                    immediate: None,
                    condition: ARMCondition::AL,
                }])
            }
            InstructionOperand::Memory(_) => {
                // Memory addressing - more complex translation
                self.translate_memory_store(x86_inst, src_reg)
            }
            _ => Err("MOV r/m32, r32: invalid destination".into()),
        }
    }

    /// Translates MOV r32, r/m32
    fn translate_mov_mem_to_reg(&mut self, x86_inst: &X86Instruction) -> Result<Vec<ARMInstruction>, Box<dyn std::error::Error>> {
        if x86_inst.operands.len() != 2 {
            return Err("MOV instruction requires 2 operands".into());
        }

        let dest_reg = match &x86_inst.operands[0] {
            InstructionOperand::Register(reg) => self.map_x86_register_enum(reg)?,
            _ => return Err("MOV r32, r/m32: destination must be register".into()),
        };

        match &x86_inst.operands[1] {
            InstructionOperand::Register(src_reg) => {
                let src_arm_reg = self.map_x86_register_enum(src_reg)?;
                Ok(vec![ARMInstruction {
                    opcode: ARMOpcode::MOV,
                    rd: Some(dest_reg),
                    rn: Some(src_arm_reg.clone()),
                    rm: None,
                    immediate: None,
                    condition: ARMCondition::AL,
                }])
            }
            InstructionOperand::Memory(_) => {
                // Memory addressing - more complex translation
                self.translate_memory_load(x86_inst, dest_reg)
            }
            _ => Err("MOV r32, r/m32: invalid source".into()),
        }
    }

    /// Translates ADD r/m32, r32
    fn translate_add_reg_to_mem(&mut self, x86_inst: &X86Instruction) -> Result<Vec<ARMInstruction>, Box<dyn std::error::Error>> {
        if x86_inst.operands.len() != 2 {
            return Err("ADD instruction requires 2 operands".into());
        }

        let src_reg = match &x86_inst.operands[1] {
            InstructionOperand::Register(reg) => self.map_x86_register_enum(reg)?,
            _ => return Err("ADD r/m32, r32: source must be register".into()),
        };

        match &x86_inst.operands[0] {
            InstructionOperand::Register(dest_reg) => {
                let dest_arm_reg = self.map_x86_register_enum(dest_reg)?;
                Ok(vec![ARMInstruction {
                    opcode: ARMOpcode::ADD,
                    rd: Some(dest_arm_reg),
                    rn: Some(dest_arm_reg.clone()),
                    rm: Some(src_reg.clone()),
                    immediate: None,
                    condition: ARMCondition::AL,
                }])
            }
            InstructionOperand::Memory(_) => {
                // Memory addressing - needs load-modify-store sequence
                self.translate_add_to_memory(x86_inst, src_reg)
            }
            _ => Err("ADD r/m32, r32: invalid destination".into()),
        }
    }

    /// Translates ADD r32, r/m32
    fn translate_add_mem_to_reg(&mut self, x86_inst: &X86Instruction) -> Result<Vec<ARMInstruction>, Box<dyn std::error::Error>> {
        if x86_inst.operands.len() != 2 {
            return Err("ADD instruction requires 2 operands".into());
        }

        let dest_reg = match &x86_inst.operands[0] {
            InstructionOperand::Register(reg) => self.map_x86_register_enum(reg)?,
            _ => return Err("ADD r32, r/m32: destination must be register".into()),
        };

        match &x86_inst.operands[1] {
            InstructionOperand::Register(src_reg) => {
                let src_arm_reg = self.map_x86_register_enum(src_reg)?;
                Ok(vec![ARMInstruction {
                    opcode: ARMOpcode::ADD,
                    rd: Some(dest_reg),
                    rn: Some(dest_reg),
                    rm: Some(src_arm_reg),
                    immediate: None,
                    condition: ARMCondition::AL,
                }])
            }
            InstructionOperand::Memory(_) => {
                // Memory addressing - needs load-add sequence
                self.translate_add_from_memory(x86_inst, dest_reg)
            }
            _ => Err("ADD r32, r/m32: invalid source".into()),
        }
    }

    /// Translates 0xFF instruction group
    fn translate_ff_group(&mut self, x86_inst: &X86Instruction) -> Result<Vec<ARMInstruction>, Box<dyn std::error::Error>> {
        if x86_inst.operands.is_empty() {
            return Err("0xFF instruction requires operands".into());
        }

        // Get the reg field from ModR/M to determine the specific instruction
        let reg_field = if let Some(modrm) = x86_inst.modrm {
            (modrm >> 3) & 0x07
        } else {
            return Err("0xFF instruction requires ModR/M byte".into());
        };

        match reg_field {
            0 => self.translate_inc(x86_inst),     // INC r/m32
            1 => self.translate_dec(x86_inst),     // DEC r/m32
            2 => self.translate_call(x86_inst),    // CALL r/m32
            4 => self.translate_jmp(x86_inst),     // JMP r/m32
            6 => self.translate_push(x86_inst),    // PUSH r/m32
            _ => Err(format!("Unimplemented 0xFF instruction with reg field: {}", reg_field).into()),
        }
    }

    /// Translates INC instruction
    fn translate_inc(&mut self, x86_inst: &X86Instruction) -> Result<Vec<ARMInstruction>, Box<dyn std::error::Error>> {
        match &x86_inst.operands[0] {
            InstructionOperand::Register(reg) => {
                let arm_reg = self.map_x86_register_enum(reg)?;
                Ok(vec![ARMInstruction {
                    opcode: ARMOpcode::ADD,
                    rd: Some(arm_reg),
                    rn: Some(arm_reg),
                    rm: None,
                    immediate: Some(1),
                    condition: ARMCondition::AL,
                }])
            }
            InstructionOperand::Memory(_) => {
                // Memory INC - needs load-modify-store sequence
                self.translate_inc_memory(x86_inst)
            }
            _ => Err("INC: invalid operand".into()),
        }
    }

    /// Translates DEC instruction
    fn translate_dec(&mut self, x86_inst: &X86Instruction) -> Result<Vec<ARMInstruction>, Box<dyn std::error::Error>> {
        match &x86_inst.operands[0] {
            InstructionOperand::Register(reg) => {
                let arm_reg = self.map_x86_register_enum(reg)?;
                Ok(vec![ARMInstruction {
                    opcode: ARMOpcode::SUB,
                    rd: Some(arm_reg),
                    rn: Some(arm_reg),
                    rm: None,
                    immediate: Some(1),
                    condition: ARMCondition::AL,
                }])
            }
            InstructionOperand::Memory(_) => {
                // Memory DEC - needs load-modify-store sequence
                self.translate_dec_memory(x86_inst)
            }
            _ => Err("DEC: invalid operand".into()),
        }
    }

    /// Translates CALL instruction
    fn translate_call(&mut self, x86_inst: &X86Instruction) -> Result<Vec<ARMInstruction>, Box<dyn std::error::Error>> {
        match &x86_inst.operands[0] {
            InstructionOperand::Register(reg) => {
                let arm_reg = self.map_x86_register_enum(reg)?;
                Ok(vec![
                    // Push return address (LR)
                    ARMInstruction {
                        opcode: ARMOpcode::STR,
                        rd: Some(ARMRegister::LR),
                        rn: Some(ARMRegister::SP),
                        rm: None,
                        immediate: Some((-4i32) as u32), // Pre-decrement
                        condition: ARMCondition::AL,
                    },
                    // Branch to target
                    ARMInstruction {
                        opcode: ARMOpcode::BX,
                        rd: Some(arm_reg),
                        rn: None,
                        rm: None,
                        immediate: None,
                        condition: ARMCondition::AL,
                    },
                ])
            }
            InstructionOperand::Memory(_) => {
                // Memory CALL - needs load-then-call sequence
                self.translate_call_memory(x86_inst)
            }
            _ => Err("CALL: invalid operand".into()),
        }
    }

    /// Translates JMP instruction
    fn translate_jmp(&mut self, x86_inst: &X86Instruction) -> Result<Vec<ARMInstruction>, Box<dyn std::error::Error>> {
        match &x86_inst.operands[0] {
            InstructionOperand::Register(reg) => {
                let arm_reg = self.map_x86_register_enum(reg)?;
                Ok(vec![ARMInstruction {
                    opcode: ARMOpcode::BX,
                    rd: Some(arm_reg),
                    rn: None,
                    rm: None,
                    immediate: None,
                    condition: ARMCondition::AL,
                }])
            }
            InstructionOperand::Memory(_) => {
                // Memory JMP - needs load-then-jump sequence
                self.translate_jmp_memory(x86_inst)
            }
            _ => Err("JMP: invalid operand".into()),
        }
    }

    /// Translates PUSH instruction
    fn translate_push(&mut self, x86_inst: &X86Instruction) -> Result<Vec<ARMInstruction>, Box<dyn std::error::Error>> {
        match &x86_inst.operands[0] {
            InstructionOperand::Register(reg) => {
                let arm_reg = self.map_x86_register_enum(reg)?;
                Ok(vec![ARMInstruction {
                    opcode: ARMOpcode::STR,
                    rd: Some(arm_reg),
                    rn: Some(ARMRegister::SP),
                    rm: None,
                    immediate: Some((-4i32) as u32), // Pre-decrement
                    condition: ARMCondition::AL,
                }])
            }
            InstructionOperand::Memory(_) => {
                // Memory PUSH - needs load-then-push sequence
                self.translate_push_memory(x86_inst)
            }
            _ => Err("PUSH: invalid operand".into()),
        }
    }

    /// Maps x86 register to ARM register
    fn map_x86_register(&self, reg_index: u8) -> Result<ARMRegister, Box<dyn std::error::Error>> {
        let x86_reg = match reg_index {
            0 => Register::EAX,
            1 => Register::ECX,
            2 => Register::EDX,
            3 => Register::EBX,
            4 => Register::ESP,
            5 => Register::EBP,
            6 => Register::ESI,
            7 => Register::EDI,
            _ => return Err("Invalid register index".into()),
        };
        
        self.register_mapping.get(&x86_reg)
            .copied()
            .ok_or("Register not mapped".into())
    }

    /// Maps x86 register enum to ARM register
    fn map_x86_register_enum(&self, reg: &Register) -> Result<ARMRegister, Box<dyn std::error::Error>> {
        self.register_mapping.get(reg)
            .copied()
            .ok_or("Register not mapped".into())
    }

    // Memory operation translations (simplified)
    fn translate_memory_store(&self, _x86_inst: &X86Instruction, _src_reg: ARMRegister) -> Result<Vec<ARMInstruction>, Box<dyn std::error::Error>> {
        // Simplified memory store - would need full address calculation
        Ok(vec![])
    }

    fn translate_memory_load(&self, _x86_inst: &X86Instruction, _dest_reg: ARMRegister) -> Result<Vec<ARMInstruction>, Box<dyn std::error::Error>> {
        // Simplified memory load - would need full address calculation
        Ok(vec![])
    }

    fn translate_add_to_memory(&self, _x86_inst: &X86Instruction, _src_reg: ARMRegister) -> Result<Vec<ARMInstruction>, Box<dyn std::error::Error>> {
        // Simplified add to memory - would need load-modify-store sequence
        Ok(vec![])
    }

    fn translate_add_from_memory(&self, _x86_inst: &X86Instruction, _dest_reg: ARMRegister) -> Result<Vec<ARMInstruction>, Box<dyn std::error::Error>> {
        // Simplified add from memory - would need load-add sequence
        Ok(vec![])
    }

    fn translate_inc_memory(&self, _x86_inst: &X86Instruction) -> Result<Vec<ARMInstruction>, Box<dyn std::error::Error>> {
        // Simplified memory inc
        Ok(vec![])
    }

    fn translate_dec_memory(&self, _x86_inst: &X86Instruction) -> Result<Vec<ARMInstruction>, Box<dyn std::error::Error>> {
        // Simplified memory dec
        Ok(vec![])
    }

    fn translate_call_memory(&self, _x86_inst: &X86Instruction) -> Result<Vec<ARMInstruction>, Box<dyn std::error::Error>> {
        // Simplified memory call
        Ok(vec![])
    }

    fn translate_jmp_memory(&self, _x86_inst: &X86Instruction) -> Result<Vec<ARMInstruction>, Box<dyn std::error::Error>> {
        // Simplified memory jump
        Ok(vec![])
    }

    fn translate_push_memory(&self, _x86_inst: &X86Instruction) -> Result<Vec<ARMInstruction>, Box<dyn std::error::Error>> {
        // Simplified memory push
        Ok(vec![])
    }

    /// Gets translation statistics
    pub fn get_stats(&self) -> TranslationStats {
        TranslationStats {
            cache_size: self.translation_cache.len(),
            register_mappings: self.register_mapping.len(),
        }
    }
}

/// Translation statistics
#[derive(Debug)]
pub struct TranslationStats {
    pub cache_size: usize,
    pub register_mappings: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::{X86Instruction, InstructionOperand, Register};

    #[test]
    fn test_translator_creation() {
        let translator = BinaryTranslator::new();
        assert!(translator.is_ok());
    }

    #[test]
    fn test_nop_translation() {
        let mut translator = BinaryTranslator::new().unwrap();
        let nop_instruction = X86Instruction {
            opcode: 0x90,
            modrm: None,
            sib: None,
            displacement: None,
            immediate: None,
            operands: vec![],
            length: 1,
        };

        let result = translator.translate_instruction(nop_instruction);
        assert!(result.is_ok());
        
        let arm_instructions = result.unwrap();
        assert_eq!(arm_instructions.len(), 1);
        assert!(matches!(arm_instructions[0].opcode, ARMOpcode::NOP));
    }

    #[test]
    fn test_mov_reg_imm_translation() {
        let mut translator = BinaryTranslator::new().unwrap();
        let mov_instruction = X86Instruction {
            opcode: 0xB8, // MOV EAX, imm32
            modrm: None,
            sib: None,
            displacement: None,
            immediate: Some(0x12345678),
            operands: vec![InstructionOperand::Immediate(0x12345678)],
            length: 5,
        };

        let result = translator.translate_instruction(mov_instruction);
        assert!(result.is_ok());
        
        let arm_instructions = result.unwrap();
        assert_eq!(arm_instructions.len(), 1);
        assert!(matches!(arm_instructions[0].opcode, ARMOpcode::MOV));
        assert_eq!(arm_instructions[0].immediate, Some(0x12345678));
    }
}