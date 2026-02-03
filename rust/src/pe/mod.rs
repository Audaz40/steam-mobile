//! PE (Portable Executable) file loader module
//! 
//! Handles loading and parsing Windows PE files for execution

use std::collections::HashMap;
use crate::memory::{MemoryManager, MemoryProtection};

/// PE file magic numbers
const PE_MAGIC: u16 = 0x5A4D; // MZ
const PE_SIGNATURE: u32 = 0x00004550; // PE00

/// PE file header structures
#[derive(Debug, Clone)]
pub struct DOSHeader {
    pub magic: u16,
    pub last_page_size: u16,
    pub pages_in_file: u16,
    pub relocations: u16,
    pub header_size: u16,
    pub min_extra_paragraphs: u16,
    pub max_extra_paragraphs: u16,
    pub initial_ss: u16,
    pub initial_sp: u16,
    pub checksum: u16,
    pub initial_ip: u16,
    pub initial_cs: u16,
    pub relocations_offset: u32,
    pub overlay_number: u16,
    pub reserved1: [u16; 4],
    pub oem_id: u16,
    pub oem_info: u16,
    pub reserved2: [u16; 10],
    pub pe_header_offset: u32,
}

#[derive(Debug, Clone)]
pub struct COFFHeader {
    pub machine: u16,
    pub number_of_sections: u16,
    pub time_date_stamp: u32,
    pub symbol_table_pointer: u32,
    pub number_of_symbols: u32,
    pub optional_header_size: u16,
    pub characteristics: u16,
}

#[derive(Debug, Clone)]
pub struct OptionalHeader32 {
    pub magic: u16,
    pub linker_version: u8,
    pub linker_revision: u8,
    pub code_size: u32,
    pub initialized_data_size: u32,
    pub uninitialized_data_size: u32,
    pub entry_point: u32,
    pub code_base: u32,
    pub data_base: u32,
    pub image_base: u32,
    pub section_alignment: u32,
    pub file_alignment: u32,
    pub os_version: u16,
    pub os_major: u16,
    pub user_version: u16,
    pub user_major: u16,
    pub subsystem_version: u16,
    pub subsystem_major: u16,
    pub win32_version: u32,
    pub image_size: u32,
    pub headers_size: u32,
    pub checksum: u32,
    pub subsystem: u16,
    pub dll_characteristics: u16,
    pub stack_reserve_size: u32,
    pub stack_commit_size: u32,
    pub heap_reserve_size: u32,
    pub heap_commit_size: u32,
    pub loader_flags: u32,
    pub data_directories_count: u32,
}

#[derive(Debug, Clone)]
pub struct DataDirectory {
    pub virtual_address: u32,
    pub size: u32,
}

#[derive(Debug, Clone)]
pub struct SectionHeader {
    pub name: [u8; 8],
    pub virtual_size: u32,
    pub virtual_address: u32,
    pub raw_data_size: u32,
    pub raw_data_offset: u32,
    pub relocations_offset: u32,
    pub line_numbers_offset: u32,
    pub relocations_count: u16,
    pub line_numbers_count: u16,
    pub characteristics: u32,
}

#[derive(Debug, Clone)]
pub struct ImportDirectoryEntry {
    pub import_lookup_table: u32,
    pub time_date_stamp: u32,
    pub forwarder_chain: u32,
    pub dll_name_rva: u32,
    pub import_address_table: u32,
}

/// PE file representation
#[derive(Debug)]
pub struct PEFile {
    pub dos_header: DOSHeader,
    pub coff_header: COFFHeader,
    pub optional_header: Option<OptionalHeader32>,
    pub data_directories: Vec<DataDirectory>,
    pub sections: Vec<SectionHeader>,
    pub raw_data: Vec<u8>,
}

/// PE loader
pub struct PELoader {
    loaded_modules: HashMap<String, u32>, // Module name -> base address
}

impl PELoader {
    /// Creates a new PE loader
    pub fn new() -> Self {
        PELoader {
            loaded_modules: HashMap::new(),
        }
    }

    /// Loads a PE file into memory
    pub fn load(&mut self, file_path: &str, memory: std::sync::Arc<std::sync::Mutex<MemoryManager>>) -> Result<u32, Box<dyn std::error::Error>> {
        // Read PE file from disk
        let file_data = std::fs::read(file_path)?;
        
        // Parse PE file
        let pe_file = self.parse_pe(&file_data)?;
        
        // Allocate memory for the image
        let image_base = self.allocate_image_memory(&pe_file, memory.clone())?;
        
        // Load sections into memory
        self.load_sections(&pe_file, image_base, memory.clone())?;
        
        // Process imports
        self.process_imports(&pe_file, image_base, memory.clone())?;
        
        // Process relocations if needed
        if image_base != pe_file.optional_header.as_ref().unwrap().image_base {
            self.process_relocations(&pe_file, image_base, memory.clone())?;
        }
        
        // Store loaded module info
        let module_name = std::path::Path::new(file_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        self.loaded_modules.insert(module_name, image_base);
        
        // Return entry point
        let entry_point = pe_file.optional_header.as_ref()
            .unwrap()
            .entry_point;
        Ok(image_base + entry_point)
    }

    /// Parses PE file structure
    fn parse_pe(&self, data: &[u8]) -> Result<PEFile, Box<dyn std::error::Error>> {
        if data.len() < 64 {
            return Err("File too small for DOS header".into());
        }

        // Parse DOS header
        let dos_header = self.parse_dos_header(&data[0..64])?;
        
        if dos_header.magic != PE_MAGIC {
            return Err("Invalid DOS signature".into());
        }

        // Check PE header offset
        if dos_header.pe_header_offset as usize >= data.len() {
            return Err("PE header offset out of bounds".into());
        }

        // Parse PE signature
        let pe_offset = dos_header.pe_header_offset as usize;
        if pe_offset + 4 > data.len() {
            return Err("PE signature out of bounds".into());
        }

        let pe_signature = u32::from_le_bytes([
            data[pe_offset],
            data[pe_offset + 1],
            data[pe_offset + 2],
            data[pe_offset + 3],
        ]);

        if pe_signature != PE_SIGNATURE {
            return Err("Invalid PE signature".into());
        }

        // Parse COFF header
        let coff_offset = pe_offset + 4;
        if coff_offset + 20 > data.len() {
            return Err("COFF header out of bounds".into());
        }

        let coff_header = self.parse_coff_header(&data[coff_offset..coff_offset + 20])?;

        // Parse optional header if present
        let optional_header = if coff_header.optional_header_size > 0 {
            let opt_offset = coff_offset + 20;
            if opt_offset + coff_header.optional_header_size as usize > data.len() {
                return Err("Optional header out of bounds".into());
            }

            Some(self.parse_optional_header(&data[opt_offset..opt_offset + coff_header.optional_header_size as usize])?)
        } else {
            None
        };

        // Parse data directories
        let data_directories = if let Some(ref opt_header) = optional_header {
            let dir_count = opt_header.data_directories_count as usize;
            let dir_offset = coff_offset + 20 + coff_header.optional_header_size as usize;
            
            if dir_offset + dir_count * 8 > data.len() {
                return Err("Data directories out of bounds".into());
            }

            let mut directories = Vec::new();
            for i in 0..dir_count {
                let dir_start = dir_offset + i * 8;
                let virtual_address = u32::from_le_bytes([
                    data[dir_start],
                    data[dir_start + 1],
                    data[dir_start + 2],
                    data[dir_start + 3],
                ]);
                let size = u32::from_le_bytes([
                    data[dir_start + 4],
                    data[dir_start + 5],
                    data[dir_start + 6],
                    data[dir_start + 7],
                ]);
                directories.push(DataDirectory { virtual_address, size });
            }
            directories
        } else {
            Vec::new()
        };

        // Parse section headers
        let sections_offset = coff_offset + 20 + coff_header.optional_header_size as usize + data_directories.len() * 8;
        if sections_offset + coff_header.number_of_sections as usize * 40 > data.len() {
            return Err("Section headers out of bounds".into());
        }

        let mut sections = Vec::new();
        for i in 0..coff_header.number_of_sections {
            let section_offset = sections_offset + i as usize * 40;
            let section = self.parse_section_header(&data[section_offset..section_offset + 40])?;
            sections.push(section);
        }

        Ok(PEFile {
            dos_header,
            coff_header,
            optional_header,
            data_directories,
            sections,
            raw_data: data.to_vec(),
        })
    }

    /// Parses DOS header
    fn parse_dos_header(&self, data: &[u8]) -> Result<DOSHeader, Box<dyn std::error::Error>> {
        Ok(DOSHeader {
            magic: u16::from_le_bytes([data[0], data[1]]),
            last_page_size: u16::from_le_bytes([data[2], data[3]]),
            pages_in_file: u16::from_le_bytes([data[4], data[5]]),
            relocations: u16::from_le_bytes([data[6], data[7]]),
            header_size: u16::from_le_bytes([data[8], data[9]]),
            min_extra_paragraphs: u16::from_le_bytes([data[10], data[11]]),
            max_extra_paragraphs: u16::from_le_bytes([data[12], data[13]]),
            initial_ss: u16::from_le_bytes([data[14], data[15]]),
            initial_sp: u16::from_le_bytes([data[16], data[17]]),
            checksum: u16::from_le_bytes([data[18], data[19]]),
            initial_ip: u16::from_le_bytes([data[20], data[21]]),
            initial_cs: u16::from_le_bytes([data[22], data[23]]),
            relocations_offset: u32::from_le_bytes([data[24], data[25], data[26], data[27]]),
            overlay_number: u16::from_le_bytes([data[28], data[29]]),
            reserved1: [
                u16::from_le_bytes([data[30], data[31]]),
                u16::from_le_bytes([data[32], data[33]]),
                u16::from_le_bytes([data[34], data[35]]),
                u16::from_le_bytes([data[36], data[37]]),
            ],
            oem_id: u16::from_le_bytes([data[38], data[39]]),
            oem_info: u16::from_le_bytes([data[40], data[41]]),
            reserved2: [
                u16::from_le_bytes([data[42], data[43]]),
                u16::from_le_bytes([data[44], data[45]]),
                u16::from_le_bytes([data[46], data[47]]),
                u16::from_le_bytes([data[48], data[49]]),
                u16::from_le_bytes([data[50], data[51]]),
                u16::from_le_bytes([data[52], data[53]]),
                u16::from_le_bytes([data[54], data[55]]),
                u16::from_le_bytes([data[56], data[57]]),
                u16::from_le_bytes([data[58], data[59]]),
                u16::from_le_bytes([data[60], data[61]]),
            ],
            pe_header_offset: u32::from_le_bytes([data[60], data[61], data[62], data[63]]),
        })
    }

    /// Parses COFF header
    fn parse_coff_header(&self, data: &[u8]) -> Result<COFFHeader, Box<dyn std::error::Error>> {
        Ok(COFFHeader {
            machine: u16::from_le_bytes([data[0], data[1]]),
            number_of_sections: u16::from_le_bytes([data[2], data[3]]),
            time_date_stamp: u32::from_le_bytes([data[4], data[5], data[6], data[7]]),
            symbol_table_pointer: u32::from_le_bytes([data[8], data[9], data[10], data[11]]),
            number_of_symbols: u32::from_le_bytes([data[12], data[13], data[14], data[15]]),
            optional_header_size: u16::from_le_bytes([data[16], data[17]]),
            characteristics: u16::from_le_bytes([data[18], data[19]]),
        })
    }

    /// Parses optional header
    fn parse_optional_header(&self, data: &[u8]) -> Result<OptionalHeader32, Box<dyn std::error::Error>> {
        Ok(OptionalHeader32 {
            magic: u16::from_le_bytes([data[0], data[1]]),
            linker_version: data[2],
            linker_revision: data[3],
            code_size: u32::from_le_bytes([data[4], data[5], data[6], data[7]]),
            initialized_data_size: u32::from_le_bytes([data[8], data[9], data[10], data[11]]),
            uninitialized_data_size: u32::from_le_bytes([data[12], data[13], data[14], data[15]]),
            entry_point: u32::from_le_bytes([data[16], data[17], data[18], data[19]]),
            code_base: u32::from_le_bytes([data[20], data[21], data[22], data[23]]),
            data_base: u32::from_le_bytes([data[24], data[25], data[26], data[27]]),
            image_base: u32::from_le_bytes([data[28], data[29], data[30], data[31]]),
            section_alignment: u32::from_le_bytes([data[32], data[33], data[34], data[35]]),
            file_alignment: u32::from_le_bytes([data[36], data[37], data[38], data[39]]),
            os_version: u16::from_le_bytes([data[40], data[41]]),
            os_major: u16::from_le_bytes([data[42], data[43]]),
            user_version: u16::from_le_bytes([data[44], data[45]]),
            user_major: u16::from_le_bytes([data[46], data[47]]),
            subsystem_version: u16::from_le_bytes([data[48], data[49]]),
            subsystem_major: u16::from_le_bytes([data[50], data[51]]),
            win32_version: u32::from_le_bytes([data[52], data[53], data[54], data[55]]),
            image_size: u32::from_le_bytes([data[56], data[57], data[58], data[59]]),
            headers_size: u32::from_le_bytes([data[60], data[61], data[62], data[63]]),
            checksum: u32::from_le_bytes([data[64], data[65], data[66], data[67]]),
            subsystem: u16::from_le_bytes([data[68], data[69]]),
            dll_characteristics: u16::from_le_bytes([data[70], data[71]]),
            stack_reserve_size: u32::from_le_bytes([data[72], data[73], data[74], data[75]]),
            stack_commit_size: u32::from_le_bytes([data[76], data[77], data[78], data[79]]),
            heap_reserve_size: u32::from_le_bytes([data[80], data[81], data[82], data[83]]),
            heap_commit_size: u32::from_le_bytes([data[84], data[85], data[86], data[87]]),
            loader_flags: u32::from_le_bytes([data[88], data[89], data[90], data[91]]),
            data_directories_count: u32::from_le_bytes([data[92], data[93], data[94], data[95]]),
        })
    }

    /// Parses section header
    fn parse_section_header(&self, data: &[u8]) -> Result<SectionHeader, Box<dyn std::error::Error>> {
        let mut name = [0u8; 8];
        name.copy_from_slice(&data[0..8]);

        Ok(SectionHeader {
            name,
            virtual_size: u32::from_le_bytes([data[8], data[9], data[10], data[11]]),
            virtual_address: u32::from_le_bytes([data[12], data[13], data[14], data[15]]),
            raw_data_size: u32::from_le_bytes([data[16], data[17], data[18], data[19]]),
            raw_data_offset: u32::from_le_bytes([data[20], data[21], data[22], data[23]]),
            relocations_offset: u32::from_le_bytes([data[24], data[25], data[26], data[27]]),
            line_numbers_offset: u32::from_le_bytes([data[28], data[29], data[30], data[31]]),
            relocations_count: u16::from_le_bytes([data[32], data[33]]),
            line_numbers_count: u16::from_le_bytes([data[34], data[35]]),
            characteristics: u32::from_le_bytes([data[36], data[37], data[38], data[39]]),
        })
    }

    /// Allocates memory for the PE image
    fn allocate_image_memory(&self, pe_file: &PEFile, memory: std::sync::Arc<std::sync::Mutex<MemoryManager>>) -> Result<u32, Box<dyn std::error::Error>> {
        let opt_header = pe_file.optional_header.as_ref().unwrap();
        let image_size = opt_header.image_size;
        
        // Try to allocate at preferred base first
        let preferred_base = opt_header.image_base;
        if let Ok(_) = memory.lock().unwrap().allocate_at(preferred_base, image_size, MemoryProtection::read_write()) {
            return Ok(preferred_base);
        }

        // Allocate at any available location
        let base_address = memory.lock().unwrap().allocate(image_size, MemoryProtection::read_write())?;
        Ok(base_address)
    }

    /// Loads sections into memory
    fn load_sections(&self, pe_file: &PEFile, image_base: u32, memory: std::sync::Arc<std::sync::Mutex<MemoryManager>>) -> Result<(), Box<dyn std::error::Error>> {
        for section in &pe_file.sections {
            let section_va = image_base + section.virtual_address;
            let section_size = std::cmp::max(section.virtual_size, section.raw_data_size);
            
            // Determine memory protection
            let protection = self.section_characteristics_to_protection(section.characteristics);
            
            // Set section protection
            memory.lock().unwrap().protect(section_va, section_size, protection)?;
            
            // Copy raw data if present
            if section.raw_data_size > 0 && section.raw_data_offset > 0 {
                let raw_start = section.raw_data_offset as usize;
                let raw_end = raw_start + section.raw_data_size as usize;
                
                if raw_end <= pe_file.raw_data.len() {
                    memory.lock().unwrap().write_bytes(section_va, &pe_file.raw_data[raw_start..raw_end])?;
                }
            }
            
            // Zero-fill remaining space if virtual_size > raw_data_size
            if section.virtual_size > section.raw_data_size {
                let fill_start = section_va + section.raw_data_size;
                let fill_size = section.virtual_size - section.raw_data_size;
                let fill_data = vec![0u8; fill_size as usize];
                memory.lock().unwrap().write_bytes(fill_start, &fill_data)?;
            }
        }

        Ok(())
    }

    /// Converts section characteristics to memory protection
    fn section_characteristics_to_protection(&self, characteristics: u32) -> MemoryProtection {
        let is_code = characteristics & 0x20 != 0; // IMAGE_SCN_CNT_CODE
        let _is_initialized_data = characteristics & 0x40 != 0; // IMAGE_SCN_CNT_INITIALIZED_DATA
        let _is_uninitialized_data = characteristics & 0x80 != 0; // IMAGE_SCN_CNT_UNINITIALIZED_DATA
        let _is_readable = characteristics & 0x40000000 != 0; // IMAGE_SCN_MEM_READ
        let is_writable = characteristics & 0x80000000 != 0; // IMAGE_SCN_MEM_WRITE
        let is_executable = characteristics & 0x20000000 != 0; // IMAGE_SCN_MEM_EXECUTE

        match (is_code, is_writable, is_executable) {
            (true, false, true) => MemoryProtection::execute_read(),
            (true, true, true) => MemoryProtection::execute_read_write(),
            (false, true, false) => MemoryProtection::read_write(),
            (false, false, false) => MemoryProtection::read_only(),
            _ => MemoryProtection::read_write(),
        }
    }

    /// Processes import table
    fn process_imports(&self, pe_file: &PEFile, image_base: u32, memory: std::sync::Arc<std::sync::Mutex<MemoryManager>>) -> Result<(), Box<dyn std::error::Error>> {
        let opt_header = pe_file.optional_header.as_ref().unwrap();
        
        // Find import directory
        if opt_header.data_directories_count < 2 {
            return Ok(()); // No import directory
        }

        let import_dir = &pe_file.data_directories[1]; // Second directory is imports
        if import_dir.virtual_address == 0 || import_dir.size == 0 {
            return Ok(()); // No imports
        }

        let import_table_va = image_base + import_dir.virtual_address;
        let mut current_entry_va = import_table_va;

        loop {
            // Read import directory entry
            let entry_data = memory.lock().unwrap().read_bytes(current_entry_va, 20)?;
            let entry = ImportDirectoryEntry {
                import_lookup_table: u32::from_le_bytes([entry_data[0], entry_data[1], entry_data[2], entry_data[3]]),
                time_date_stamp: u32::from_le_bytes([entry_data[4], entry_data[5], entry_data[6], entry_data[7]]),
                forwarder_chain: u32::from_le_bytes([entry_data[8], entry_data[9], entry_data[10], entry_data[11]]),
                dll_name_rva: u32::from_le_bytes([entry_data[12], entry_data[13], entry_data[14], entry_data[15]]),
                import_address_table: u32::from_le_bytes([entry_data[16], entry_data[17], entry_data[18], entry_data[19]]),
            };

            // Check for end of table
            if entry.import_lookup_table == 0 && entry.time_date_stamp == 0 && 
               entry.forwarder_chain == 0 && entry.dll_name_rva == 0 && entry.import_address_table == 0 {
                break;
            }

            // Read DLL name
            let dll_name_va = image_base + entry.dll_name_rva;
            let dll_name = self.read_null_terminated_string(dll_name_va, memory.clone())?;
            
            // Process this import
            self.process_single_import(&dll_name, entry.import_address_table, image_base, memory.clone())?;

            current_entry_va += 20;
        }

        Ok(())
    }

    /// Processes imports from a single DLL
    fn process_single_import(&self, dll_name: &str, iat_rva: u32, image_base: u32, memory: std::sync::Arc<std::sync::Mutex<MemoryManager>>) -> Result<(), Box<dyn std::error::Error>> {
        let iat_va = image_base + iat_rva;
        let mut current_entry_va = iat_va;

        loop {
            // Read import address table entry
            let entry_value = memory.lock().unwrap().read_u32(current_entry_va)?;
            
            // Check for end of table
            if entry_value == 0 {
                break;
            }

            // Determine if this is an ordinal import (high bit set)
            if entry_value & 0x80000000 != 0 {
                // Ordinal import - not supported yet
                let ordinal = entry_value & 0xFFFF;
                eprintln!("Ordinal import {} from {} not supported", ordinal, dll_name);
            } else {
                // Named import
                let hint_name_va = image_base + entry_value;
                let _hint = memory.lock().unwrap().read_u16(hint_name_va)?;
                let function_name = self.read_null_terminated_string(hint_name_va + 2, memory.clone())?;
                
                // Resolve function address (simplified - would need actual module loading)
                let function_address = self.resolve_import_function(dll_name, &function_name)?;
                
                // Write resolved address to IAT
                memory.lock().unwrap().write_u32(current_entry_va, function_address)?;
            }

            current_entry_va += 4;
        }

        Ok(())
    }

    /// Reads a null-terminated string from memory
    fn read_null_terminated_string(&self, address: u32, memory: std::sync::Arc<std::sync::Mutex<MemoryManager>>) -> Result<String, Box<dyn std::error::Error>> {
        let mut string = String::new();
        let mut current_address = address;

        loop {
            let byte = memory.lock().unwrap().read_u8(current_address)?;
            if byte == 0 {
                break;
            }
            string.push(byte as char);
            current_address += 1;
        }

        Ok(string)
    }

    /// Resolves import function address (simplified)
    fn resolve_import_function(&self, dll_name: &str, function_name: &str) -> Result<u32, Box<dyn std::error::Error>> {
        // This is a very simplified implementation
        // In a real implementation, this would load the DLL and resolve the function
        
        match (dll_name, function_name) {
            ("kernel32.dll", "GetLastError") => Ok(0x7c801d7a),
            ("kernel32.dll", "GetCurrentProcess") => Ok(0x7c8020ba),
            ("kernel32.dll", "VirtualAlloc") => Ok(0x7c809a90),
            ("kernel32.dll", "VirtualFree") => Ok(0x7c809c4c),
            ("kernel32.dll", "ExitProcess") => Ok(0x7c81caa2),
            ("user32.dll", "MessageBoxA") => Ok(0x77d5050b),
            ("user32.dll", "GetWindowTextA") => Ok(0x77d4c6e0),
            _ => {
                eprintln!("Unresolved import: {}.{}", dll_name, function_name);
                Ok(0xDEADBEEF) // Placeholder address
            }
        }
    }

    /// Processes relocations
    fn process_relocations(&self, pe_file: &PEFile, image_base: u32, memory: std::sync::Arc<std::sync::Mutex<MemoryManager>>) -> Result<(), Box<dyn std::error::Error>> {
        let opt_header = pe_file.optional_header.as_ref().unwrap();
        
        // Find relocation directory
        if opt_header.data_directories_count < 6 {
            return Ok(()); // No relocation directory
        }

        let reloc_dir = &pe_file.data_directories[5]; // Sixth directory is base relocations
        if reloc_dir.virtual_address == 0 || reloc_dir.size == 0 {
            return Ok(()); // No relocations
        }

        let preferred_base = opt_header.image_base;
        let delta = image_base.wrapping_sub(preferred_base);

        if delta == 0 {
            return Ok(()); // No relocation needed
        }

        let reloc_table_va = image_base + reloc_dir.virtual_address;
        let mut current_block_va = reloc_table_va;
        let block_end_va = reloc_table_va + reloc_dir.size;

        while current_block_va < block_end_va {
            // Read relocation block header
            let block_data = memory.lock().unwrap().read_bytes(current_block_va, 8)?;
            let page_rva = u32::from_le_bytes([block_data[0], block_data[1], block_data[2], block_data[3]]);
            let block_size = u32::from_le_bytes([block_data[4], block_data[5], block_data[6], block_data[7]]);

            if block_size < 8 {
                break; // Invalid block size
            }

            let entry_count = (block_size - 8) / 2;
            let entries_start = current_block_va + 8;

            for i in 0..entry_count {
                let entry_va = entries_start + i * 2;
                let entry_data = memory.lock().unwrap().read_u16(entry_va)?;
                
                let reloc_type = entry_data >> 12;
                let offset = entry_data & 0xFFF;

                if reloc_type == 0 {
                    continue; // Absolute relocation, ignore
                }

                let target_va = image_base + page_rva + offset as u32;

                match reloc_type {
                    3 => { // IMAGE_REL_BASED_HIGHLOW
                        let original = memory.lock().unwrap().read_u32(target_va)?;
                        let relocated = original.wrapping_add(delta);
                        memory.lock().unwrap().write_u32(target_va, relocated)?;
                    }
                    10 => { // IMAGE_REL_BASED_DIR64 (not used in 32-bit)
                        // Shouldn't happen in 32-bit PE
                    }
                    _ => {
                        eprintln!("Unsupported relocation type: {}", reloc_type);
                    }
                }
            }

            current_block_va += block_size as u32;
        }

        Ok(())
    }

    /// Gets loaded modules
    pub fn get_loaded_modules(&self) -> &HashMap<String, u32> {
        &self.loaded_modules
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pe_loader_creation() {
        let loader = PELoader::new();
        assert_eq!(loader.get_loaded_modules().len(), 0);
    }

    #[test]
    fn test_dos_header_parsing() {
        let loader = PELoader::new();
        let mut data = vec![0u8; 64];
        
        // Set magic number
        data[0] = b'M';
        data[1] = b'Z';
        
        let dos_header = loader.parse_dos_header(&data).unwrap();
        assert_eq!(dos_header.magic, PE_MAGIC);
    }

    #[test]
    fn test_coff_header_parsing() {
        let loader = PELoader::new();
        let data = [
            0x4C, 0x01, // Machine (i386)
            0x01, 0x00, // Number of sections
            0x12, 0x34, 0x56, 0x78, // Time/date stamp
            0x00, 0x00, 0x00, 0x00, // Symbol table pointer
            0x00, 0x00, 0x00, 0x00, // Number of symbols
            0xE0, 0x00, // Optional header size
            0x02, 0x00, // Characteristics
        ];
        
        let coff_header = loader.parse_coff_header(&data).unwrap();
        assert_eq!(coff_header.machine, 0x014C); // i386
        assert_eq!(coff_header.number_of_sections, 1);
        assert_eq!(coff_header.optional_header_size, 0x00E0);
    }
}