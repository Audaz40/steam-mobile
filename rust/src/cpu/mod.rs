//! CPU emulation module
//! 
//! Provides x86 CPU emulation with ARM backend execution

use std::sync::{Arc, Mutex};
use crate::memory::MemoryManager;

/// x86 CPU registers
#[derive(Debug, Clone)]
pub struct X86Registers {
    // General purpose registers (32-bit)
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
    pub esi: u32,
    pub edi: u32,
    pub ebp: u32,
    pub esp: u32,
    
    // Instruction pointer
    pub eip: u32,
    
    // Flags register
    pub eflags: u32,
    
    // Segment registers
    pub cs: u16,
    pub ds: u16,
    pub es: u16,
    pub fs: u16,
    pub gs: u16,
    pub ss: u16,
}

impl Default for X86Registers {
    fn default() -> Self {
        X86Registers {
            eax: 0,
            ebx: 0,
            ecx: 0,
            edx: 0,
            esi: 0,
            edi: 0,
            ebp: 0,
            esp: 0x7FFF_F000, // Default stack pointer
            eip: 0,
            eflags: 0,
            cs: 0,
            ds: 0,
            es: 0,
            fs: 0,
            gs: 0,
            ss: 0,
        }
    }
}

/// x86 instruction representation
#[derive(Debug, Clone)]
pub struct X86Instruction {
    pub opcode: u8,
    pub modrm: Option<u8>,
    pub sib: Option<u8>,
    pub displacement: Option<i32>,
    pub immediate: Option<u32>,
    pub operands: Vec<InstructionOperand>,
    pub length: u8,
}

#[derive(Debug, Clone)]
pub enum InstructionOperand {
    Register(Register),
    Memory(MemoryOperand),
    Immediate(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Register {
    EAX, EBX, ECX, EDX,
    ESI, EDI, EBP, ESP,
    AX, BX, CX, DX,
    SI, DI, BP, SP,
    AL, BL, CL, DL,
    AH, BH, CH, DH,
}

#[derive(Debug, Clone)]
pub struct MemoryOperand {
    pub base: Option<Register>,
    pub index: Option<Register>,
    pub scale: u8,
    pub displacement: i32,
}

/// Main CPU emulation structure
pub struct CPU {
    registers: X86Registers,
    memory: Arc<Mutex<MemoryManager>>,
    instruction_cache: std::collections::HashMap<u32, X86Instruction>,
}

impl CPU {
    /// Creates a new CPU instance
    pub fn new(memory: Arc<Mutex<MemoryManager>>) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(CPU {
            registers: X86Registers::default(),
            memory,
            instruction_cache: std::collections::HashMap::new(),
        })
    }

    /// Sets the instruction pointer to a specific address
    pub fn set_instruction_pointer(&mut self, address: u32) {
        self.registers.eip = address;
    }

    /// Gets the current instruction pointer
    pub fn get_instruction_pointer(&self) -> u32 {
        self.registers.eip
    }

    /// Fetches and decodes the current instruction
    pub fn fetch_instruction(&mut self) -> Result<X86Instruction, Box<dyn std::error::Error>> {
        let eip = self.registers.eip;
        
        // Check cache first
        if let Some(cached_instruction) = self.instruction_cache.get(&eip) {
            self.registers.eip += cached_instruction.length as u32;
            return Ok(cached_instruction.clone());
        }

        // Fetch instruction bytes from memory
        let instruction_bytes = {
            let memory = self.memory.lock().unwrap();
            memory.read_bytes(eip, 15)? // Max x86 instruction length
        };

        // Decode instruction
        let instruction = self.decode_instruction(&instruction_bytes)?;
        
        // Cache the decoded instruction
        self.instruction_cache.insert(eip, instruction.clone());
        
        // Advance instruction pointer
        self.registers.eip += instruction.length as u32;
        
        Ok(instruction)
    }

    /// Decodes x86 instruction bytes
    fn decode_instruction(&self, bytes: &[u8]) -> Result<X86Instruction, Box<dyn std::error::Error>> {
        if bytes.is_empty() {
            return Err("Empty instruction bytes".into());
        }

        let opcode = bytes[0];
        let mut instruction = X86Instruction {
            opcode,
            modrm: None,
            sib: None,
            displacement: None,
            immediate: None,
            operands: Vec::new(),
            length: 1,
        };

        // Handle ModR/M byte if present
        if self.needs_modrm(opcode) {
            if bytes.len() < 2 {
                return Err("Incomplete ModR/M byte".into());
            }
            instruction.modrm = Some(bytes[1]);
            instruction.length += 1;
            
            // Parse ModR/M to extract operands
            self.parse_modrm(&mut instruction, bytes[1])?;
        }

        // Handle SIB byte
        if let Some(modrm) = instruction.modrm {
            if self.needs_sib(modrm) && bytes.len() > instruction.length as usize {
                instruction.sib = Some(bytes[instruction.length as usize]);
                instruction.length += 1;
            }
        }

        // Handle displacement
        if let Some(displacement_size) = self.get_displacement_size(opcode, instruction.modrm) {
            let start = instruction.length as usize;
            if bytes.len() < start + displacement_size {
                return Err("Incomplete displacement".into());
            }
            
            let displacement = match displacement_size {
                1 => bytes[start] as i8 as i32,
                2 => i16::from_le_bytes([bytes[start], bytes[start + 1]]) as i32,
                4 => i32::from_le_bytes([bytes[start], bytes[start + 1], bytes[start + 2], bytes[start + 3]]),
                _ => return Err("Invalid displacement size".into()),
            };
            
            instruction.displacement = Some(displacement);
            instruction.length += displacement_size as u8;
        }

        // Handle immediate values
        if let Some(imm_size) = self.get_immediate_size(opcode) {
            let start = instruction.length as usize;
            if bytes.len() < start + imm_size {
                return Err("Incomplete immediate".into());
            }
            
            let immediate = match imm_size {
                1 => bytes[start] as i8 as i32,
                2 => i16::from_le_bytes([bytes[start], bytes[start + 1]]) as i32,
                4 => i32::from_le_bytes([bytes[start], bytes[start + 1], bytes[start + 2], bytes[start + 3]]),
                _ => return Err("Invalid immediate size".into()),
            };
            
            instruction.immediate = Some(immediate as u32);
            instruction.length += imm_size as u8;
            instruction.operands.push(InstructionOperand::Immediate(immediate));
        }

        Ok(instruction)
    }

    /// Executes a translated instruction on ARM backend
    pub fn execute_instruction(&mut self, instruction: X86Instruction) -> Result<(), Box<dyn std::error::Error>> {
        // This would interface with ARM execution backend
        // For now, we'll simulate basic execution
        
        match instruction.opcode {
            0x90 => { // NOP
                // Do nothing
            }
            0xB8 => { // MOV EAX, imm32
                if let Some(imm) = instruction.immediate {
                    self.registers.eax = imm;
                }
            }
            0x89 => { // MOV r/m32, r32
                self.execute_mov_reg_to_mem(&instruction)?;
            }
            0x8B => { // MOV r32, r/m32
                self.execute_mov_mem_to_reg(&instruction)?;
            }
            0x01 => { // ADD r/m32, r32
                self.execute_add_reg_to_mem(&instruction)?;
            }
            0x03 => { // ADD r32, r/m32
                self.execute_add_mem_to_reg(&instruction)?;
            }
            0xFF => { // Various instructions based on ModR/M reg field
                self.execute_ff_group(&instruction)?;
            }
            _ => {
                return Err(format!("Unimplemented opcode: 0x{:02X}", instruction.opcode).into());
            }
        }

        Ok(())
    }

    /// Checks if instruction needs ModR/M byte
    fn needs_modrm(&self, opcode: u8) -> bool {
        match opcode {
            0x01..=0x33 | 0x39..=0x3B | 0x50..=0x5F | 0x68..=0x6F | 
            0x70..=0x7F | 0x80..=0x8F | 0x90..=0x9F | 0xA0..=0xA3 |
            0xB0..=0xBF | 0xC0..=0xC7 | 0xD0..=0xD7 | 0xE0..=0xEF |
            0xF0..=0xFF => true,
            _ => false,
        }
    }

    /// Checks if instruction needs SIB byte
    fn needs_sib(&self, modrm: u8) -> bool {
        let mod_field = (modrm >> 6) & 0x03;
        let rm_field = modrm & 0x07;
        mod_field != 3 && rm_field == 4
    }

    /// Gets displacement size for instruction
    fn get_displacement_size(&self, _opcode: u8, modrm: Option<u8>) -> Option<usize> {
        if let Some(modrm) = modrm {
            let mod_field = (modrm >> 6) & 0x03;
            match mod_field {
                0 => None,
                1 => Some(1),
                2 => Some(4),
                3 => None,
                _ => None,
            }
        } else {
            None
        }
    }

    /// Gets immediate size for instruction
    fn get_immediate_size(&self, opcode: u8) -> Option<usize> {
        match opcode {
            0x04..=0x05 | 0x0C..=0x0D | 0x24..=0x25 | 0x34..=0x35 | 
            0xB0..=0xB7 => Some(1),
            0x68 | 0xB8..=0xBF => Some(4),
            0xC1 | 0xC6 | 0xD1 | 0xD6 => Some(1),
            0xC0 | 0xD0 => Some(1),
            _ => None,
        }
    }

    /// Parses ModR/M byte to extract operands
    fn parse_modrm(&self, instruction: &mut X86Instruction, modrm: u8) -> Result<(), Box<dyn std::error::Error>> {
        let mod_field = (modrm >> 6) & 0x03;
        let reg_field = (modrm >> 3) & 0x07;
        let rm_field = modrm & 0x07;

        // Add register operand based on reg field
        instruction.operands.push(InstructionOperand::Register(
            self.modrm_to_register(reg_field)
        ));

        // Add memory/register operand based on mod and rm fields
        if mod_field == 3 {
            // Register direct addressing
            instruction.operands.push(InstructionOperand::Register(
                self.modrm_to_register(rm_field)
            ));
        } else {
            // Memory addressing
            let memory_operand = self.parse_memory_addressing(mod_field, rm_field, instruction)?;
            instruction.operands.push(InstructionOperand::Memory(memory_operand));
        }

        Ok(())
    }

    /// Converts ModR/M reg field to register
    fn modrm_to_register(&self, reg: u8) -> Register {
        match reg {
            0 => Register::EAX,
            1 => Register::ECX,
            2 => Register::EDX,
            3 => Register::EBX,
            4 => Register::ESP,
            5 => Register::EBP,
            6 => Register::ESI,
            7 => Register::EDI,
            _ => Register::EAX, // Should never happen
        }
    }

    /// Parses memory addressing from ModR/M
    fn parse_memory_addressing(&self, mod_field: u8, rm_field: u8, instruction: &X86Instruction) -> Result<MemoryOperand, Box<dyn std::error::Error>> {
        let base = if mod_field == 0 && rm_field == 5 {
            None // Special case: [disp32]
        } else {
            Some(self.modrm_to_register(rm_field))
        };

        let (index, scale) = if let Some(sib) = instruction.sib {
            let scale = (sib >> 6) & 0x03;
            let index = (sib >> 3) & 0x07;
            (Some(self.modrm_to_register(index)), scale + 1)
        } else {
            (None, 1)
        };

        let displacement = instruction.displacement.unwrap_or(0);

        Ok(MemoryOperand {
            base,
            index,
            scale,
            displacement,
        })
    }

    /// Executes MOV r/m32, r32 instruction
    fn execute_mov_reg_to_mem(&mut self, instruction: &X86Instruction) -> Result<(), Box<dyn std::error::Error>> {
        if instruction.operands.len() != 2 {
            return Err("MOV instruction requires 2 operands".into());
        }

        let src_value = match &instruction.operands[1] {
            InstructionOperand::Register(reg) => self.get_register_value(reg),
            _ => return Err("MOV r/m32, r32: source must be register".into()),
        };

        match &instruction.operands[0] {
            InstructionOperand::Register(reg) => self.set_register_value(reg, src_value),
            InstructionOperand::Memory(mem_op) => {
                let address = self.calculate_memory_address(mem_op)?;
                let mut memory = self.memory.lock().unwrap();
                memory.write_u32(address, src_value)?;
            }
            _ => return Err("MOV r/m32, r32: invalid destination".into()),
        }

        Ok(())
    }

    /// Executes MOV r32, r/m32 instruction
    fn execute_mov_mem_to_reg(&mut self, instruction: &X86Instruction) -> Result<(), Box<dyn std::error::Error>> {
        if instruction.operands.len() != 2 {
            return Err("MOV instruction requires 2 operands".into());
        }

        let src_value = match &instruction.operands[1] {
            InstructionOperand::Register(reg) => self.get_register_value(reg),
            InstructionOperand::Memory(mem_op) => {
                let address = self.calculate_memory_address(mem_op)?;
                let memory = self.memory.lock().unwrap();
                memory.read_u32(address)?
            }
            _ => return Err("MOV r32, r/m32: invalid source".into()),
        };

        match &instruction.operands[0] {
            InstructionOperand::Register(reg) => self.set_register_value(reg, src_value),
            _ => return Err("MOV r32, r/m32: destination must be register".into()),
        }

        Ok(())
    }

    /// Executes ADD r/m32, r32 instruction
    fn execute_add_reg_to_mem(&mut self, instruction: &X86Instruction) -> Result<(), Box<dyn std::error::Error>> {
        if instruction.operands.len() != 2 {
            return Err("ADD instruction requires 2 operands".into());
        }

        let src_value = match &instruction.operands[1] {
            InstructionOperand::Register(reg) => self.get_register_value(reg),
            _ => return Err("ADD r/m32, r32: source must be register".into()),
        };

        match &instruction.operands[0] {
            InstructionOperand::Register(reg) => {
                let current_value = self.get_register_value(reg);
                let result = current_value.wrapping_add(src_value);
                self.set_register_value(reg, result);
                self.update_flags_add(current_value, src_value, result);
            }
            InstructionOperand::Memory(mem_op) => {
                let address = self.calculate_memory_address(mem_op)?;
                let current_value;
                let result;
                {
                    let mut memory = self.memory.lock().unwrap();
                    current_value = memory.read_u32(address)?;
                    result = current_value.wrapping_add(src_value);
                    memory.write_u32(address, result)?;
                }
                self.update_flags_add(current_value, src_value, result);
            }
            _ => return Err("ADD r/m32, r32: invalid destination".into()),
        }

        Ok(())
    }

    /// Executes ADD r32, r/m32 instruction
    fn execute_add_mem_to_reg(&mut self, instruction: &X86Instruction) -> Result<(), Box<dyn std::error::Error>> {
        if instruction.operands.len() != 2 {
            return Err("ADD instruction requires 2 operands".into());
        }

        let src_value = match &instruction.operands[1] {
            InstructionOperand::Register(reg) => self.get_register_value(reg),
            InstructionOperand::Memory(mem_op) => {
                let address = self.calculate_memory_address(mem_op)?;
                let memory = self.memory.lock().unwrap();
                memory.read_u32(address)?
            }
            _ => return Err("ADD r32, r/m32: invalid source".into()),
        };

        match &instruction.operands[0] {
            InstructionOperand::Register(reg) => {
                let current_value = self.get_register_value(reg);
                let result = current_value.wrapping_add(src_value);
                self.set_register_value(reg, result);
                self.update_flags_add(current_value, src_value, result);
            }
            _ => return Err("ADD r32, r/m32: destination must be register".into()),
        }

        Ok(())
    }

    /// Executes instructions in the 0xFF group
    fn execute_ff_group(&mut self, instruction: &X86Instruction) -> Result<(), Box<dyn std::error::Error>> {
        if instruction.operands.is_empty() {
            return Err("0xFF instruction requires operands".into());
        }

        // Get the reg field from ModR/M to determine the specific instruction
        let reg_field = if let Some(modrm) = instruction.modrm {
            (modrm >> 3) & 0x07
        } else {
            return Err("0xFF instruction requires ModR/M byte".into());
        };

        match reg_field {
            0 => {
                // INC r/m32
                self.execute_inc(&instruction.operands[0])?;
            }
            1 => {
                // DEC r/m32
                self.execute_dec(&instruction.operands[0])?;
            }
            2 => {
                // CALL r/m32
                let target = self.get_operand_value(&instruction.operands[0])?;
                self.execute_call(target)?;
            }
            4 => {
                // JMP r/m32
                let target = self.get_operand_value(&instruction.operands[0])?;
                self.registers.eip = target;
            }
            6 => {
                // PUSH r/m32
                let value = self.get_operand_value(&instruction.operands[0])?;
                self.execute_push(value)?;
            }
            _ => {
                return Err(format!("Unimplemented 0xFF instruction with reg field: {}", reg_field).into());
            }
        }

        Ok(())
    }

    /// Executes INC instruction
    fn execute_inc(&mut self, operand: &InstructionOperand) -> Result<(), Box<dyn std::error::Error>> {
        let current_value = self.get_operand_value(operand)?;
        let result = current_value.wrapping_add(1);
        self.set_operand_value(operand, result)?;
        self.update_flags_add(current_value, 1, result);
        Ok(())
    }

    /// Executes DEC instruction
    fn execute_dec(&mut self, operand: &InstructionOperand) -> Result<(), Box<dyn std::error::Error>> {
        let current_value = self.get_operand_value(operand)?;
        let result = current_value.wrapping_sub(1);
        self.set_operand_value(operand, result)?;
        self.update_flags_sub(current_value, 1, result);
        Ok(())
    }

    /// Executes CALL instruction
    fn execute_call(&mut self, target: u32) -> Result<(), Box<dyn std::error::Error>> {
        // Push return address onto stack
        let return_address = self.registers.eip;
        self.execute_push(return_address)?;
        
        // Jump to target
        self.registers.eip = target;
        Ok(())
    }

    /// Executes PUSH instruction
    fn execute_push(&mut self, value: u32) -> Result<(), Box<dyn std::error::Error>> {
        // Decrease stack pointer
        self.registers.esp = self.registers.esp.wrapping_sub(4);
        
        // Write value to stack
        let mut memory = self.memory.lock().unwrap();
        memory.write_u32(self.registers.esp, value)?;
        
        Ok(())
    }

    /// Gets value from operand
    fn get_operand_value(&self, operand: &InstructionOperand) -> Result<u32, Box<dyn std::error::Error>> {
        match operand {
            InstructionOperand::Register(reg) => Ok(self.get_register_value(reg)),
            InstructionOperand::Memory(mem_op) => {
                let address = self.calculate_memory_address(mem_op)?;
                let memory = self.memory.lock().unwrap();
                memory.read_u32(address)
            }
            InstructionOperand::Immediate(value) => Ok(*value as u32),
        }
    }

    /// Sets value for operand
    fn set_operand_value(&mut self, operand: &InstructionOperand, value: u32) -> Result<(), Box<dyn std::error::Error>> {
        match operand {
            InstructionOperand::Register(reg) => {
                self.set_register_value(reg, value);
                Ok(())
            }
            InstructionOperand::Memory(mem_op) => {
                let address = self.calculate_memory_address(mem_op)?;
                let mut memory = self.memory.lock().unwrap();
                memory.write_u32(address, value)
            }
            InstructionOperand::Immediate(_) => {
                Err("Cannot set immediate value".into())
            }
        }
    }

    /// Calculates memory address from memory operand
    fn calculate_memory_address(&self, mem_op: &MemoryOperand) -> Result<u32, Box<dyn std::error::Error>> {
        let mut address = mem_op.displacement as u32;

        if let Some(base_reg) = &mem_op.base {
            address = address.wrapping_add(self.get_register_value(base_reg));
        }

        if let Some(index_reg) = &mem_op.index {
            let index_value = self.get_register_value(index_reg);
            address = address.wrapping_add(index_value.wrapping_mul(mem_op.scale as u32));
        }

        Ok(address)
    }

    /// Gets register value
    fn get_register_value(&self, reg: &Register) -> u32 {
        match reg {
            Register::EAX => self.registers.eax,
            Register::EBX => self.registers.ebx,
            Register::ECX => self.registers.ecx,
            Register::EDX => self.registers.edx,
            Register::ESI => self.registers.esi,
            Register::EDI => self.registers.edi,
            Register::EBP => self.registers.ebp,
            Register::ESP => self.registers.esp,
            Register::AX => self.registers.eax & 0xFFFF,
            Register::BX => self.registers.ebx & 0xFFFF,
            Register::CX => self.registers.ecx & 0xFFFF,
            Register::DX => self.registers.edx & 0xFFFF,
            Register::SI => self.registers.esi & 0xFFFF,
            Register::DI => self.registers.edi & 0xFFFF,
            Register::BP => self.registers.ebp & 0xFFFF,
            Register::SP => self.registers.esp & 0xFFFF,
            Register::AL => self.registers.eax & 0xFF,
            Register::BL => self.registers.ebx & 0xFF,
            Register::CL => self.registers.ecx & 0xFF,
            Register::DL => self.registers.edx & 0xFF,
            Register::AH => (self.registers.eax >> 8) & 0xFF,
            Register::BH => (self.registers.ebx >> 8) & 0xFF,
            Register::CH => (self.registers.ecx >> 8) & 0xFF,
            Register::DH => (self.registers.edx >> 8) & 0xFF,
        }
    }

    /// Sets register value
    fn set_register_value(&mut self, reg: &Register, value: u32) {
        match reg {
            Register::EAX => self.registers.eax = value,
            Register::EBX => self.registers.ebx = value,
            Register::ECX => self.registers.ecx = value,
            Register::EDX => self.registers.edx = value,
            Register::ESI => self.registers.esi = value,
            Register::EDI => self.registers.edi = value,
            Register::EBP => self.registers.ebp = value,
            Register::ESP => self.registers.esp = value,
            Register::AX => self.registers.eax = (self.registers.eax & 0xFFFF_0000) | (value & 0xFFFF),
            Register::BX => self.registers.ebx = (self.registers.ebx & 0xFFFF_0000) | (value & 0xFFFF),
            Register::CX => self.registers.ecx = (self.registers.ecx & 0xFFFF_0000) | (value & 0xFFFF),
            Register::DX => self.registers.edx = (self.registers.edx & 0xFFFF_0000) | (value & 0xFFFF),
            Register::SI => self.registers.esi = (self.registers.esi & 0xFFFF_0000) | (value & 0xFFFF),
            Register::DI => self.registers.edi = (self.registers.edi & 0xFFFF_0000) | (value & 0xFFFF),
            Register::BP => self.registers.ebp = (self.registers.ebp & 0xFFFF_0000) | (value & 0xFFFF),
            Register::SP => self.registers.esp = (self.registers.esp & 0xFFFF_0000) | (value & 0xFFFF),
            Register::AL => self.registers.eax = (self.registers.eax & 0xFFFF_FF00) | (value & 0xFF),
            Register::BL => self.registers.ebx = (self.registers.ebx & 0xFFFF_FF00) | (value & 0xFF),
            Register::CL => self.registers.ecx = (self.registers.ecx & 0xFFFF_FF00) | (value & 0xFF),
            Register::DL => self.registers.edx = (self.registers.edx & 0xFFFF_FF00) | (value & 0xFF),
            Register::AH => self.registers.eax = (self.registers.eax & 0xFF00_FFFF) | ((value & 0xFF) << 8),
            Register::BH => self.registers.ebx = (self.registers.ebx & 0xFF00_FFFF) | ((value & 0xFF) << 8),
            Register::CH => self.registers.ecx = (self.registers.ecx & 0xFF00_FFFF) | ((value & 0xFF) << 8),
            Register::DH => self.registers.edx = (self.registers.edx & 0xFF00_FFFF) | ((value & 0xFF) << 8),
        }
    }

    /// Updates flags after addition
    fn update_flags_add(&mut self, src1: u32, src2: u32, result: u32) {
        // Zero flag
        if result == 0 {
            self.registers.eflags |= 0x40; // ZF
        } else {
            self.registers.eflags &= !0x40;
        }

        // Sign flag
        if (result as i32) < 0 {
            self.registers.eflags |= 0x80; // SF
        } else {
            self.registers.eflags &= !0x80;
        }

        // Carry flag
        if result < src1 || result < src2 {
            self.registers.eflags |= 0x01; // CF
        } else {
            self.registers.eflags &= !0x01;
        }

        // Overflow flag (simplified)
        let src1_signed = src1 as i32;
        let src2_signed = src2 as i32;
        let result_signed = result as i32;
        
        if (src1_signed >= 0 && src2_signed >= 0 && result_signed < 0) ||
           (src1_signed < 0 && src2_signed < 0 && result_signed >= 0) {
            self.registers.eflags |= 0x800; // OF
        } else {
            self.registers.eflags &= !0x800;
        }
    }

    /// Updates flags after subtraction
    fn update_flags_sub(&mut self, src1: u32, src2: u32, result: u32) {
        // Zero flag
        if result == 0 {
            self.registers.eflags |= 0x40; // ZF
        } else {
            self.registers.eflags &= !0x40;
        }

        // Sign flag
        if (result as i32) < 0 {
            self.registers.eflags |= 0x80; // SF
        } else {
            self.registers.eflags &= !0x80;
        }

        // Carry flag (for subtraction, set if src1 < src2)
        if src1 < src2 {
            self.registers.eflags |= 0x01; // CF
        } else {
            self.registers.eflags &= !0x01;
        }

        // Overflow flag (simplified)
        let src1_signed = src1 as i32;
        let src2_signed = src2 as i32;
        let result_signed = result as i32;
        
        if (src1_signed >= 0 && src2_signed < 0 && result_signed < 0) ||
           (src1_signed < 0 && src2_signed >= 0 && result_signed >= 0) {
            self.registers.eflags |= 0x800; // OF
        } else {
            self.registers.eflags &= !0x800;
        }
    }

    /// Gets current register state (for debugging)
    pub fn get_registers(&self) -> &X86Registers {
        &self.registers
    }
}