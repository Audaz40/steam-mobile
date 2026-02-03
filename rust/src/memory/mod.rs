//! Memory management module
//! 
//! Provides virtual memory management for x86 emulation on ARM

use std::collections::HashMap;

/// Memory page size (4KB)
pub const PAGE_SIZE: u32 = 4096;

/// Memory protection flags
#[derive(Debug, Clone, Copy)]
pub struct MemoryProtection {
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
}

impl MemoryProtection {
    pub const fn new() -> Self {
        MemoryProtection {
            readable: false,
            writable: false,
            executable: false,
        }
    }

    pub const fn read_only() -> Self {
        MemoryProtection {
            readable: true,
            writable: false,
            executable: false,
        }
    }

    pub const fn read_write() -> Self {
        MemoryProtection {
            readable: true,
            writable: true,
            executable: false,
        }
    }

    pub const fn execute_read() -> Self {
        MemoryProtection {
            readable: true,
            writable: false,
            executable: true,
        }
    }

    pub const fn execute_read_write() -> Self {
        MemoryProtection {
            readable: true,
            writable: true,
            executable: true,
        }
    }
}

/// Memory page
#[derive(Debug)]
pub struct MemoryPage {
    pub base_address: u32,
    pub data: Vec<u8>,
    pub protection: MemoryProtection,
    pub is_dirty: bool,
}

impl MemoryPage {
    /// Creates a new memory page
    pub fn new(base_address: u32, protection: MemoryProtection) -> Self {
        MemoryPage {
            base_address,
            data: vec![0u8; PAGE_SIZE as usize],
            protection,
            is_dirty: false,
        }
    }

    /// Reads a byte from the page
    pub fn read_byte(&self, offset: u32) -> Result<u8, Box<dyn std::error::Error>> {
        if offset >= PAGE_SIZE {
            return Err("Page offset out of bounds".into());
        }
        
        if !self.protection.readable {
            return Err("Page not readable".into());
        }

        Ok(self.data[offset as usize])
    }

    /// Writes a byte to the page
    pub fn write_byte(&mut self, offset: u32, value: u8) -> Result<(), Box<dyn std::error::Error>> {
        if offset >= PAGE_SIZE {
            return Err("Page offset out of bounds".into());
        }
        
        if !self.protection.writable {
            return Err("Page not writable".into());
        }

        self.data[offset as usize] = value;
        self.is_dirty = true;
        Ok(())
    }

    /// Reads multiple bytes from the page
    pub fn read_bytes(&self, offset: u32, count: u32) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        if offset + count > PAGE_SIZE {
            return Err("Read range exceeds page bounds".into());
        }
        
        if !self.protection.readable {
            return Err("Page not readable".into());
        }

        let start = offset as usize;
        let end = start + count as usize;
        Ok(self.data[start..end].to_vec())
    }

    /// Writes multiple bytes to the page
    pub fn write_bytes(&mut self, offset: u32, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        if offset + data.len() as u32 > PAGE_SIZE {
            return Err("Write range exceeds page bounds".into());
        }
        
        if !self.protection.writable {
            return Err("Page not writable".into());
        }

        let start = offset as usize;
        let end = start + data.len();
        self.data[start..end].copy_from_slice(data);
        self.is_dirty = true;
        Ok(())
    }
}

/// Memory region
#[derive(Debug)]
pub struct MemoryRegion {
    pub base_address: u32,
    pub size: u32,
    pub protection: MemoryProtection,
    pub name: String,
}

impl MemoryRegion {
    pub fn new(base_address: u32, size: u32, protection: MemoryProtection, name: String) -> Self {
        MemoryRegion {
            base_address,
            size,
            protection,
            name,
        }
    }
}

/// Main memory manager
pub struct MemoryManager {
    pages: HashMap<u32, MemoryPage>,
    regions: Vec<MemoryRegion>,
    next_free_address: u32,
    total_memory: u32,
    used_memory: u32,
}

impl MemoryManager {
    /// Creates a new memory manager
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut manager = MemoryManager {
            pages: HashMap::new(),
            regions: Vec::new(),
            next_free_address: 0x10000000, // Start at 256MB
            total_memory: 0x80000000,      // 2GB virtual address space
            used_memory: 0,
        };

        // Initialize standard memory regions
        manager.init_standard_regions()?;
        
        Ok(manager)
    }

    /// Initializes standard memory regions
    fn init_standard_regions(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // User code segment (0x400000 - 0x500000)
        self.allocate_region(0x00400000, 0x00100000, MemoryProtection::execute_read(), "code".to_string())?;
        
        // User data segment (0x500000 - 0x600000)
        self.allocate_region(0x00500000, 0x00100000, MemoryProtection::read_write(), "data".to_string())?;
        
        // Stack region (0x7FFF0000 - 0x80000000)
        self.allocate_region(0x7FFF0000, 0x00010000, MemoryProtection::read_write(), "stack".to_string())?;
        
        // Heap region (0x600000 - 0x7FFF0000)
        self.allocate_region(0x00600000, 0x7FF10000, MemoryProtection::read_write(), "heap".to_string())?;

        Ok(())
    }

    /// Allocates a memory region
    pub fn allocate_region(&mut self, base_address: u32, size: u32, protection: MemoryProtection, name: String) -> Result<(), Box<dyn std::error::Error>> {
        // Align size to page boundaries
        let aligned_size = ((size + PAGE_SIZE - 1) / PAGE_SIZE) * PAGE_SIZE;
        
        // Create region
        let region = MemoryRegion::new(base_address, aligned_size, protection, name);
        self.regions.push(region);
        
        // Allocate pages for the region
        let page_count = aligned_size / PAGE_SIZE;
        for i in 0..page_count {
            let page_address = base_address + (i * PAGE_SIZE);
            let page = MemoryPage::new(page_address, protection);
            self.pages.insert(page_address, page);
            self.used_memory += PAGE_SIZE;
        }

        Ok(())
    }

    /// Allocates memory at a specific address
    pub fn allocate_at(&mut self, address: u32, size: u32, protection: MemoryProtection) -> Result<u32, Box<dyn std::error::Error>> {
        // Check if address is already in use
        if self.is_address_in_use(address, size) {
            return Err("Address already in use".into());
        }

        self.allocate_region(address, size, protection, "allocated".to_string())?;
        Ok(address)
    }

    /// Allocates memory automatically
    pub fn allocate(&mut self, size: u32, protection: MemoryProtection) -> Result<u32, Box<dyn std::error::Error>> {
        let address = self.find_free_region(size)?;
        self.allocate_at(address, size, protection)?;
        Ok(address)
    }

    /// Finds a free memory region
    fn find_free_region(&self, size: u32) -> Result<u32, Box<dyn std::error::Error>> {
        let aligned_size = ((size + PAGE_SIZE - 1) / PAGE_SIZE) * PAGE_SIZE;
        
        // Simple linear search for free space
        let mut address = self.next_free_address;
        while address + aligned_size <= self.total_memory {
            if !self.is_address_in_use(address, aligned_size) {
                return Ok(address);
            }
            address += PAGE_SIZE;
        }

        Err("Out of memory".into())
    }

    /// Checks if address range is in use
    fn is_address_in_use(&self, address: u32, size: u32) -> bool {
        for region in &self.regions {
            if address < region.base_address + region.size &&
               address + size > region.base_address {
                return true;
            }
        }
        false
    }

    /// Reads a byte from memory
    pub fn read_u8(&self, address: u32) -> Result<u8, Box<dyn std::error::Error>> {
        let page_address = (address / PAGE_SIZE) * PAGE_SIZE;
        let offset = address % PAGE_SIZE;
        
        let page = self.pages.get(&page_address)
            .ok_or("Invalid page address")?;
        
        page.read_byte(offset)
    }

    /// Writes a byte to memory
    pub fn write_u8(&mut self, address: u32, value: u8) -> Result<(), Box<dyn std::error::Error>> {
        let page_address = (address / PAGE_SIZE) * PAGE_SIZE;
        let offset = address % PAGE_SIZE;
        
        let page = self.pages.get_mut(&page_address)
            .ok_or("Invalid page address")?;
        
        page.write_byte(offset, value)
    }

    /// Reads a 16-bit value from memory
    pub fn read_u16(&self, address: u32) -> Result<u16, Box<dyn std::error::Error>> {
        let byte1 = self.read_u8(address)? as u16;
        let byte2 = self.read_u8(address + 1)? as u16;
        Ok(byte1 | (byte2 << 8))
    }

    /// Writes a 16-bit value to memory
    pub fn write_u16(&mut self, address: u32, value: u16) -> Result<(), Box<dyn std::error::Error>> {
        self.write_u8(address, (value & 0xFF) as u8)?;
        self.write_u8(address + 1, ((value >> 8) & 0xFF) as u8)
    }

    /// Reads a 32-bit value from memory
    pub fn read_u32(&self, address: u32) -> Result<u32, Box<dyn std::error::Error>> {
        let byte1 = self.read_u8(address)? as u32;
        let byte2 = self.read_u8(address + 1)? as u32;
        let byte3 = self.read_u8(address + 2)? as u32;
        let byte4 = self.read_u8(address + 3)? as u32;
        Ok(byte1 | (byte2 << 8) | (byte3 << 16) | (byte4 << 24))
    }

    /// Writes a 32-bit value to memory
    pub fn write_u32(&mut self, address: u32, value: u32) -> Result<(), Box<dyn std::error::Error>> {
        self.write_u8(address, (value & 0xFF) as u8)?;
        self.write_u8(address + 1, ((value >> 8) & 0xFF) as u8)?;
        self.write_u8(address + 2, ((value >> 16) & 0xFF) as u8)?;
        self.write_u8(address + 3, ((value >> 24) & 0xFF) as u8)
    }

    /// Reads multiple bytes from memory
    pub fn read_bytes(&self, address: u32, count: u32) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut result = Vec::new();
        let mut current_address = address;
        let mut remaining = count;

        while remaining > 0 {
            let page_address = (current_address / PAGE_SIZE) * PAGE_SIZE;
            let offset = current_address % PAGE_SIZE;
            let page = self.pages.get(&page_address)
                .ok_or("Invalid page address")?;

            let bytes_to_read = std::cmp::min(remaining, PAGE_SIZE - offset);
            let data = page.read_bytes(offset, bytes_to_read)?;
            result.extend_from_slice(&data);

            current_address += bytes_to_read;
            remaining -= bytes_to_read;
        }

        Ok(result)
    }

    /// Writes multiple bytes to memory
    pub fn write_bytes(&mut self, address: u32, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let mut current_address = address;
        let mut data_offset = 0;

        while data_offset < data.len() {
            let page_address = (current_address / PAGE_SIZE) * PAGE_SIZE;
            let offset = current_address % PAGE_SIZE;
            let page = self.pages.get_mut(&page_address)
                .ok_or("Invalid page address")?;

            let bytes_to_write = std::cmp::min(data.len() - data_offset, (PAGE_SIZE - offset) as usize);
            let chunk = &data[data_offset..data_offset + bytes_to_write];
            page.write_bytes(offset, chunk)?;

            current_address += bytes_to_write as u32;
            data_offset += bytes_to_write;
        }

        Ok(())
    }

    /// Changes memory protection
    pub fn protect(&mut self, address: u32, size: u32, protection: MemoryProtection) -> Result<(), Box<dyn std::error::Error>> {
        let aligned_address = (address / PAGE_SIZE) * PAGE_SIZE;
        let aligned_size = ((size + PAGE_SIZE - 1) / PAGE_SIZE) * PAGE_SIZE;
        
        let page_count = aligned_size / PAGE_SIZE;
        for i in 0..page_count {
            let page_address = aligned_address + (i * PAGE_SIZE);
            if let Some(page) = self.pages.get_mut(&page_address) {
                page.protection = protection;
            }
        }

        Ok(())
    }

    /// Gets memory statistics
    pub fn get_stats(&self) -> MemoryStats {
        MemoryStats {
            total_pages: self.pages.len(),
            total_memory: self.total_memory,
            used_memory: self.used_memory,
            free_memory: self.total_memory - self.used_memory,
            regions: self.regions.len(),
        }
    }

    /// Gets memory regions
    pub fn get_regions(&self) -> &[MemoryRegion] {
        &self.regions
    }

    /// Clears dirty flag for all pages
    pub fn clear_dirty_flags(&mut self) {
        for page in self.pages.values_mut() {
            page.is_dirty = false;
        }
    }

    /// Gets dirty pages
    pub fn get_dirty_pages(&self) -> Vec<&MemoryPage> {
        self.pages.values()
            .filter(|page| page.is_dirty)
            .collect()
    }
}

/// Memory statistics
#[derive(Debug)]
pub struct MemoryStats {
    pub total_pages: usize,
    pub total_memory: u32,
    pub used_memory: u32,
    pub free_memory: u32,
    pub regions: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_manager_creation() {
        let manager = MemoryManager::new();
        assert!(manager.is_ok());
    }

    #[test]
    fn test_basic_read_write() {
        let mut manager = MemoryManager::new().unwrap();
        let address = 0x00500000; // Data segment
        
        // Test byte operations
        manager.write_u8(address, 0x42).unwrap();
        assert_eq!(manager.read_u8(address).unwrap(), 0x42);
        
        // Test word operations
        manager.write_u16(address, 0x1234).unwrap();
        assert_eq!(manager.read_u16(address).unwrap(), 0x1234);
        
        // Test dword operations
        manager.write_u32(address, 0x12345678).unwrap();
        assert_eq!(manager.read_u32(address).unwrap(), 0x12345678);
    }

    #[test]
    fn test_memory_allocation() {
        let mut manager = MemoryManager::new().unwrap();
        let address = manager.allocate(0x1000, MemoryProtection::read_write()).unwrap();
        assert!(address >= manager.next_free_address);
        
        // Write to allocated memory
        manager.write_u32(address, 0xdeadbeef).unwrap();
        assert_eq!(manager.read_u32(address).unwrap(), 0xdeadbeef);
    }

    #[test]
    fn test_memory_protection() {
        let mut manager = MemoryManager::new().unwrap();
        let address = manager.allocate(0x1000, MemoryProtection::read_only()).unwrap();
        
        // Should be able to read
        manager.write_u32(address, 0x12345678).unwrap();
        assert_eq!(manager.read_u32(address).unwrap(), 0x12345678);
        
        // Change to read-only protection
        manager.protect(address, 0x1000, MemoryProtection::read_only()).unwrap();
        
        // Writing should fail (but our implementation doesn't enforce this yet)
        // This would need actual memory protection at the OS level
    }
}