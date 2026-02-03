//! System call emulation module
//! 
//! Provides Windows API system call emulation on mobile platforms

use std::collections::HashMap;
use crate::memory::MemoryManager;

/// Windows API function identifiers
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindowsAPI {
    // Kernel32.dll functions
    GetLastError,
    GetCurrentProcess,
    GetCurrentThreadId,
    GetCurrentProcessId,
    VirtualAlloc,
    VirtualFree,
    VirtualProtect,
    CreateFileA,
    ReadFile,
    WriteFile,
    CloseHandle,
    GetTickCount,
    GetSystemTime,
    ExitProcess,
    
    // User32.dll functions
    MessageBoxA,
    MessageBoxW,
    GetWindowTextA,
    GetWindowTextW,
    SetWindowTextA,
    SetWindowTextW,
    FindWindowA,
    FindWindowW,
    SendMessageA,
    SendMessageW,
    PostMessageA,
    PostMessageW,
    
    // GDI32.dll functions
    CreateCompatibleDC,
    CreateCompatibleBitmap,
    SelectObject,
    DeleteObject,
    BitBlt,
    StretchBlt,
    
    // Kernel32.dll file functions
    CreateFileMappingA,
    MapViewOfFile,
    UnmapViewOfFile,
    FlushViewOfFile,
    
    // Registry functions
    RegOpenKeyA,
    RegOpenKeyW,
    RegQueryValueA,
    RegQueryValueW,
    RegSetValueA,
    RegSetValueW,
    RegCloseKey,
    
    // Thread functions
    CreateThread,
    WaitForSingleObject,
    WaitForMultipleObjects,
    SetEvent,
    ResetEvent,
    
    // Memory functions
    GlobalAlloc,
    GlobalFree,
    GlobalLock,
    GlobalUnlock,
    LocalAlloc,
    LocalFree,
    LocalLock,
    LocalUnlock,
    
    // Unknown/unsupported
    Unknown,
}

/// System call context
#[derive(Debug)]
pub struct SyscallContext {
    pub api_function: WindowsAPI,
    pub args: Vec<u32>,
    pub return_address: u32,
    pub thread_id: u32,
}

/// System call emulator
pub struct SyscallEmulator {
    // File handles
    next_file_handle: u32,
    file_handles: HashMap<u32, FileHandle>,
    
    // Memory allocations
    next_memory_handle: u32,
    memory_allocations: HashMap<u32, MemoryAllocation>,
    
    // Thread information
    current_thread_id: u32,
    current_process_id: u32,
    
    // System state
    last_error: u32,
    system_time: u64,
    
    // Registry simulation
    registry: HashMap<String, RegistryValue>,
}

/// File handle representation
#[derive(Debug)]
struct FileHandle {
    id: u32,
    path: String,
    mode: FileMode,
    position: u64,
}

#[derive(Debug)]
enum FileMode {
    Read,
    Write,
    ReadWrite,
    Append,
}

/// Memory allocation tracking
#[derive(Debug)]
struct MemoryAllocation {
    address: u32,
    size: u32,
    protection: u32,
}

/// Registry value
#[derive(Debug)]
enum RegistryValue {
    String(String),
    Dword(u32),
    Binary(Vec<u8>),
}

impl SyscallEmulator {
    /// Creates a new system call emulator
    pub fn new() -> Self {
        SyscallEmulator {
            next_file_handle: 1000,
            file_handles: HashMap::new(),
            next_memory_handle: 2000,
            memory_allocations: HashMap::new(),
            current_thread_id: 1,
            current_process_id: 100,
            last_error: 0,
            system_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            registry: HashMap::new(),
        }
    }

    /// Emulates a Windows API call
    pub fn emulate_syscall(&mut self, context: SyscallContext, memory: &mut MemoryManager) -> Result<u32, Box<dyn std::error::Error>> {
        self.last_error = 0; // Reset last error
        
        let result = match context.api_function {
            WindowsAPI::GetLastError => self.handle_get_last_error(),
            WindowsAPI::GetCurrentProcess => self.handle_get_current_process(),
            WindowsAPI::GetCurrentThreadId => self.handle_get_current_thread_id(),
            WindowsAPI::GetCurrentProcessId => self.handle_get_current_process_id(),
            WindowsAPI::VirtualAlloc => self.handle_virtual_alloc(&context.args, memory)?,
            WindowsAPI::VirtualFree => self.handle_virtual_free(&context.args, memory)?,
            WindowsAPI::VirtualProtect => self.handle_virtual_protect(&context.args, memory)?,
            WindowsAPI::CreateFileA => self.handle_create_file_a(&context.args, memory)?,
            WindowsAPI::ReadFile => self.handle_read_file(&context.args, memory)?,
            WindowsAPI::WriteFile => self.handle_write_file(&context.args, memory)?,
            WindowsAPI::CloseHandle => self.handle_close_handle(&context.args)?,
            WindowsAPI::GetTickCount => self.handle_get_tick_count(),
            WindowsAPI::GetSystemTime => self.handle_get_system_time(&context.args, memory)?,
            WindowsAPI::ExitProcess => self.handle_exit_process(&context.args),
            WindowsAPI::MessageBoxA => self.handle_message_box_a(&context.args, memory)?,
            WindowsAPI::MessageBoxW => self.handle_message_box_w(&context.args, memory)?,
            WindowsAPI::GetWindowTextA => self.handle_get_window_text_a(&context.args, memory)?,
            WindowsAPI::GetWindowTextW => self.handle_get_window_text_w(&context.args, memory)?,
            WindowsAPI::SetWindowTextA => self.handle_set_window_text_a(&context.args, memory)?,
            WindowsAPI::SetWindowTextW => self.handle_get_window_text_w(&context.args, memory)?,
            WindowsAPI::CreateThread => self.handle_create_thread(&context.args, memory)?,
            WindowsAPI::WaitForSingleObject => self.handle_wait_for_single_object(&context.args)?,
            WindowsAPI::GlobalAlloc => self.handle_global_alloc(&context.args, memory)?,
            WindowsAPI::GlobalFree => self.handle_global_free(&context.args)?,
            WindowsAPI::RegOpenKeyA => self.handle_reg_open_key_a(&context.args, memory)?,
            WindowsAPI::RegQueryValueA => self.handle_reg_query_value_a(&context.args, memory)?,
            _ => {
                self.last_error = 1; // ERROR_INVALID_FUNCTION
                0 // Return error
            }
        };

        Ok(result)
    }

    /// Handles GetLastError()
    fn handle_get_last_error(&mut self) -> u32 {
        let error = self.last_error;
        self.last_error = 0; // Clear after reading
        error
    }

    /// Handles GetCurrentProcess()
    fn handle_get_current_process(&self) -> u32 {
        // Return pseudo handle for current process
        -1i32 as u32
    }

    /// Handles GetCurrentThreadId()
    fn handle_get_current_thread_id(&self) -> u32 {
        self.current_thread_id
    }

    /// Handles GetCurrentProcessId()
    fn handle_get_current_process_id(&self) -> u32 {
        self.current_process_id
    }

    /// Handles VirtualAlloc()
    fn handle_virtual_alloc(&mut self, args: &[u32], memory: &mut MemoryManager) -> Result<u32, Box<dyn std::error::Error>> {
        if args.len() < 3 {
            self.last_error = 87; // ERROR_INVALID_PARAMETER
            return Ok(0);
        }

        let address = args[0];
        let size = args[1];
        let _allocation_type = args[2];
        let protect = if args.len() > 3 { args[3] } else { 0x40 }; // PAGE_EXECUTE_READWRITE

        // For simplicity, ignore allocation_type and always allocate
        let allocated_address = if address == 0 {
            memory.allocate(size, crate::memory::MemoryProtection::read_write())?
        } else {
            memory.allocate_at(address, size, crate::memory::MemoryProtection::read_write())?
        };

        // Track allocation
        self.memory_allocations.insert(
            self.next_memory_handle,
            MemoryAllocation {
                address: allocated_address,
                size,
                protection: protect,
            }
        );
        self.next_memory_handle += 1;

        Ok(allocated_address)
    }

    /// Handles VirtualFree()
    fn handle_virtual_free(&mut self, args: &[u32], _memory: &mut MemoryManager) -> Result<u32, Box<dyn std::error::Error>> {
        if args.len() < 2 {
            self.last_error = 87; // ERROR_INVALID_PARAMETER
            return Ok(0);
        }

        let _address = args[0];
        let _size = args[1];
        let _free_type = if args.len() > 2 { args[2] } else { 0x8000 }; // MEM_RELEASE

        // For simplicity, we'll just return success
        // In a real implementation, we'd need to track and free allocations
        
        Ok(1) // Success
    }

    /// Handles VirtualProtect()
    fn handle_virtual_protect(&mut self, args: &[u32], memory: &mut MemoryManager) -> Result<u32, Box<dyn std::error::Error>> {
        if args.len() < 3 {
            self.last_error = 87; // ERROR_INVALID_PARAMETER
            return Ok(0);
        }

        let address = args[0];
        let size = args[1];
        let new_protect = args[2];

        // Convert Windows protection flags to our MemoryProtection
        let protection = match new_protect {
            0x01 => crate::memory::MemoryProtection::read_only(),       // PAGE_READONLY
            0x02 => crate::memory::MemoryProtection::read_write(),      // PAGE_READWRITE
            0x04 => crate::memory::MemoryProtection::read_write(),      // PAGE_READWRITE (same as RW for simplicity)
            0x10 => crate::memory::MemoryProtection::execute_read(),    // PAGE_EXECUTE_READ
            0x20 => crate::memory::MemoryProtection::execute_read_write(), // PAGE_EXECUTE_READWRITE
            0x40 => crate::memory::MemoryProtection::execute_read_write(), // PAGE_EXECUTE_READWRITE
            _ => crate::memory::MemoryProtection::read_write(),
        };

        memory.protect(address, size, protection)?;
        
        Ok(1) // Success
    }

    /// Handles CreateFileA()
    fn handle_create_file_a(&mut self, args: &[u32], memory: &MemoryManager) -> Result<u32, Box<dyn std::error::Error>> {
        if args.len() < 5 {
            self.last_error = 87; // ERROR_INVALID_PARAMETER
            return Ok(-1i32 as u32); // INVALID_HANDLE_VALUE
        }

        let filename_ptr = args[0];
        let _desired_access = args[1];
        let _share_mode = args[2];
        let _creation_disposition = args[3];
        let _flags_and_attributes = args[4];

        // Read filename from memory
        let filename = self.read_string_from_memory(filename_ptr, memory)?;
        
        // For simplicity, we'll just create a mock file handle
        let handle = self.next_file_handle;
        self.next_file_handle += 1;

        self.file_handles.insert(handle, FileHandle {
            id: handle,
            path: filename,
            mode: FileMode::ReadWrite,
            position: 0,
        });

        Ok(handle)
    }

    /// Handles ReadFile()
    fn handle_read_file(&mut self, args: &[u32], memory: &mut MemoryManager) -> Result<u32, Box<dyn std::error::Error>> {
        if args.len() < 4 {
            self.last_error = 87; // ERROR_INVALID_PARAMETER
            return Ok(0);
        }

        let handle = args[0];
        let buffer = args[1];
        let bytes_to_read = args[2];
        let bytes_read_ptr = args[3];

        // Check if handle exists
        if !self.file_handles.contains_key(&handle) {
            self.last_error = 6; // ERROR_INVALID_HANDLE
            return Ok(0);
        }

        // For simplicity, we'll just write zeros to the buffer
        for i in 0..bytes_to_read {
            memory.write_u8(buffer + i, 0)?;
        }

        // Write bytes read
        if bytes_read_ptr != 0 {
            memory.write_u32(bytes_read_ptr, bytes_to_read)?;
        }

        Ok(1) // Success
    }

    /// Handles WriteFile()
    fn handle_write_file(&mut self, args: &[u32], memory: &mut MemoryManager) -> Result<u32, Box<dyn std::error::Error>> {
        if args.len() < 4 {
            self.last_error = 87; // ERROR_INVALID_PARAMETER
            return Ok(0);
        }

        let handle = args[0];
        let _buffer = args[1];
        let bytes_to_write = args[2];
        let bytes_written_ptr = args[3];

        // Check if handle exists
        if !self.file_handles.contains_key(&handle) {
            self.last_error = 6; // ERROR_INVALID_HANDLE
            return Ok(0);
        }

        // For simplicity, just log the message
        eprintln!("WriteFile: {}", bytes_to_write);

        // Write bytes written
        if bytes_written_ptr != 0 {
            memory.write_u32(bytes_written_ptr, bytes_to_write)?;
        }

        Ok(1) // Success
    }

    /// Handles CloseHandle()
    fn handle_close_handle(&mut self, args: &[u32]) -> Result<u32, Box<dyn std::error::Error>> {
        if args.is_empty() {
            self.last_error = 87; // ERROR_INVALID_PARAMETER
            return Ok(0);
        }

        let handle = args[0];

        // Remove file handle if it exists
        self.file_handles.remove(&handle);

        Ok(1) // Success
    }

    /// Handles GetTickCount()
    fn handle_get_tick_count(&self) -> u32 {
        (self.system_time / 1000) as u32
    }

    /// Handles GetSystemTime()
    fn handle_get_system_time(&mut self, args: &[u32], memory: &mut MemoryManager) -> Result<u32, Box<dyn std::error::Error>> {
        if args.is_empty() {
            self.last_error = 87; // ERROR_INVALID_PARAMETER
            return Ok(0);
        }

        let system_time_ptr = args[0];

        // Get current time
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();

        // Convert to FILETIME structure (simplified)
        let file_time = now.as_millis() as u64;

        // Write FILETIME to memory (low dword, high dword)
        memory.write_u32(system_time_ptr, (file_time & 0xFFFFFFFF) as u32)?;
        memory.write_u32(system_time_ptr + 4, (file_time >> 32) as u32)?;

        Ok(1) // Success
    }

    /// Handles ExitProcess()
    fn handle_exit_process(&mut self, args: &[u32]) -> u32 {
        let exit_code = if args.len() > 0 { args[0] } else { 0 };
        
        // In a real implementation, this would terminate the process
        eprintln!("Process exiting with code: {}", exit_code);
        
        // This should not return
        std::process::exit(exit_code as i32);
    }

    /// Handles MessageBoxA()
    fn handle_message_box_a(&mut self, args: &[u32], memory: &MemoryManager) -> Result<u32, Box<dyn std::error::Error>> {
        if args.len() < 4 {
            self.last_error = 87; // ERROR_INVALID_PARAMETER
            return Ok(0);
        }

        let _hwnd = args[0];
        let text_ptr = args[1];
        let caption_ptr = args[2];
        let _type_flags = args[3];

        // Read text and caption from memory
        let text = if text_ptr != 0 {
            self.read_string_from_memory(text_ptr, memory)?
        } else {
            String::new()
        };

        let caption = if caption_ptr != 0 {
            self.read_string_from_memory(caption_ptr, memory)?
        } else {
            "SteamMobile".to_string()
        };

        // For mobile, we'll just log the message
        eprintln!("MessageBox: {} - {}", caption, text);

        // Return IDOK for simplicity
        Ok(1)
    }

    /// Handles MessageBoxW() (Unicode version)
    fn handle_message_box_w(&mut self, args: &[u32], memory: &MemoryManager) -> Result<u32, Box<dyn std::error::Error>> {
        if args.len() < 4 {
            self.last_error = 87; // ERROR_INVALID_PARAMETER
            return Ok(0);
        }

        let _hwnd = args[0];
        let text_ptr = args[1];
        let caption_ptr = args[2];
        let _type_flags = args[3];

        // Read Unicode text and caption from memory
        let text = if text_ptr != 0 {
            self.read_wide_string_from_memory(text_ptr, memory)?
        } else {
            String::new()
        };

        let caption = if caption_ptr != 0 {
            self.read_wide_string_from_memory(caption_ptr, memory)?
        } else {
            "SteamMobile".to_string()
        };

        // For mobile, we'll just log the message
        eprintln!("MessageBoxW: {} - {}", caption, text);

        // Return IDOK for simplicity
        Ok(1)
    }

    /// Handles GetWindowTextA()
    fn handle_get_window_text_a(&mut self, args: &[u32], memory: &mut MemoryManager) -> Result<u32, Box<dyn std::error::Error>> {
        if args.len() < 3 {
            self.last_error = 87; // ERROR_INVALID_PARAMETER
            return Ok(0);
        }

        let _hwnd = args[0];
        let buffer_ptr = args[1];
        let buffer_size = args[2];

        // For simplicity, return empty string
        if buffer_size > 0 && buffer_ptr != 0 {
            memory.write_u8(buffer_ptr, 0)?; // Null terminator
        }

        Ok(0) // Length of string copied
    }

    /// Handles GetWindowTextW()
    fn handle_get_window_text_w(&mut self, args: &[u32], memory: &mut MemoryManager) -> Result<u32, Box<dyn std::error::Error>> {
        if args.len() < 3 {
            self.last_error = 87; // ERROR_INVALID_PARAMETER
            return Ok(0);
        }

        let _hwnd = args[0];
        let buffer_ptr = args[1];
        let buffer_size = args[2];

        // For simplicity, return empty string
        if buffer_size > 0 && buffer_ptr != 0 {
            memory.write_u16(buffer_ptr, 0)?; // Null terminator
        }

        Ok(0) // Length of string copied
    }

    /// Handles SetWindowTextA()
    fn handle_set_window_text_a(&mut self, args: &[u32], memory: &MemoryManager) -> Result<u32, Box<dyn std::error::Error>> {
        if args.len() < 2 {
            self.last_error = 87; // ERROR_INVALID_PARAMETER
            return Ok(0);
        }

        let _hwnd = args[0];
        let text_ptr = args[1];

        // Read text from memory
        let text = if text_ptr != 0 {
            self.read_string_from_memory(text_ptr, memory)?
        } else {
            String::new()
        };

        // For simplicity, just log the text change
        eprintln!("SetWindowText: {}", text);

        Ok(1) // Success
    }

    /// Handles CreateThread()
    fn handle_create_thread(&mut self, args: &[u32], memory: &mut MemoryManager) -> Result<u32, Box<dyn std::error::Error>> {
        if args.len() < 6 {
            self.last_error = 87; // ERROR_INVALID_PARAMETER
            return Ok(0);
        }

        let _security_attributes = args[0];
        let _stack_size = args[1];
        let _start_address = args[2];
        let _parameter = args[3];
        let _creation_flags = args[4];
        let thread_id_ptr = args[5];

        // Generate new thread ID
        let thread_id = self.current_thread_id + 1;
        self.current_thread_id = thread_id;

        // Write thread ID if requested
        if thread_id_ptr != 0 {
            memory.write_u32(thread_id_ptr, thread_id)?;
        }

        // For simplicity, return a mock thread handle
        let thread_handle = self.next_file_handle;
        self.next_file_handle += 1;

        Ok(thread_handle)
    }

    /// Handles WaitForSingleObject()
    fn handle_wait_for_single_object(&mut self, args: &[u32]) -> Result<u32, Box<dyn std::error::Error>> {
        if args.len() < 2 {
            self.last_error = 87; // ERROR_INVALID_PARAMETER
            return Ok(0xFFFFFFFF); // WAIT_FAILED
        }

        let handle = args[0];
        let _timeout = args[1];

        // For simplicity, always return immediately with object signaled
        Ok(0) // WAIT_OBJECT_0
    }

    /// Handles GlobalAlloc()
    fn handle_global_alloc(&mut self, args: &[u32], memory: &mut MemoryManager) -> Result<u32, Box<dyn std::error::Error>> {
        if args.len() < 2 {
            self.last_error = 87; // ERROR_INVALID_PARAMETER
            return Ok(0);
        }

        let _flags = args[0];
        let size = args[1];

        // Allocate memory
        let address = memory.allocate(size, crate::memory::MemoryProtection::read_write())?;

        Ok(address)
    }

    /// Handles GlobalFree()
    fn handle_global_free(&mut self, args: &[u32]) -> Result<u32, Box<dyn std::error::Error>> {
        if args.is_empty() {
            self.last_error = 87; // ERROR_INVALID_PARAMETER
            return Ok(0);
        }

        let handle = args[0];

        // For simplicity, always return success
        // In a real implementation, we'd track and free allocations
        
        Ok(0) // Return NULL for success (GlobalFree returns NULL on success)
    }

    /// Handles RegOpenKeyA()
    fn handle_reg_open_key_a(&mut self, args: &[u32], memory: &mut MemoryManager) -> Result<u32, Box<dyn std::error::Error>> {
        if args.len() < 3 {
            self.last_error = 87; // ERROR_INVALID_PARAMETER
            return Ok(0);
        }

        let _key = args[0];
        let subkey_ptr = args[1];
        let result_handle_ptr = args[2];

        // Read subkey from memory
        let _subkey = if subkey_ptr != 0 {
            self.read_string_from_memory(subkey_ptr, memory)?
        } else {
            String::new()
        };

        // For simplicity, always return success with a mock handle
        let handle = self.next_file_handle;
        self.next_file_handle += 1;

        if result_handle_ptr != 0 {
            memory.write_u32(result_handle_ptr, handle)?;
        }

        Ok(0) // ERROR_SUCCESS
    }

    /// Handles RegQueryValueA()
    fn handle_reg_query_value_a(&mut self, args: &[u32], memory: &mut MemoryManager) -> Result<u32, Box<dyn std::error::Error>> {
        if args.len() < 4 {
            self.last_error = 87; // ERROR_INVALID_PARAMETER
            return Ok(2); // ERROR_FILE_NOT_FOUND
        }

        let _key = args[0];
        let value_name_ptr = args[1];
        let data_ptr = args[2];
        let data_size_ptr = args[3];

        // Read value name from memory
        let _value_name = if value_name_ptr != 0 {
            self.read_string_from_memory(value_name_ptr, memory)?
        } else {
            String::new()
        };

        // For simplicity, return empty value
        if data_size_ptr != 0 {
            let current_size = memory.read_u32(data_size_ptr)?;
            if current_size > 0 && data_ptr != 0 {
                memory.write_u8(data_ptr, 0)?; // Null terminator
                memory.write_u32(data_size_ptr, 1)?; // Size of null terminator
            }
        }

        Ok(0) // ERROR_SUCCESS
    }

    /// Reads a null-terminated ASCII string from memory
    fn read_string_from_memory(&self, address: u32, memory: &MemoryManager) -> Result<String, Box<dyn std::error::Error>> {
        let mut string = String::new();
        let mut current_address = address;

        loop {
            let byte = memory.read_u8(current_address)?;
            if byte == 0 {
                break;
            }
            string.push(byte as char);
            current_address += 1;
        }

        Ok(string)
    }

    /// Reads a null-terminated Unicode string from memory
    fn read_wide_string_from_memory(&self, address: u32, memory: &MemoryManager) -> Result<String, Box<dyn std::error::Error>> {
        let mut string = String::new();
        let mut current_address = address;

        loop {
            let wide_char = memory.read_u16(current_address)?;
            if wide_char == 0 {
                break;
            }
            
            // Convert UTF-16 to UTF-8 (simplified)
            if wide_char <= 0x7F {
                string.push(wide_char as u8 as char);
            } else {
                // For non-ASCII characters, use replacement character
                string.push('\u{FFFD}');
            }
            
            current_address += 2;
        }

        Ok(string)
    }

    /// Maps API name to enum
    pub fn map_api_name(name: &str) -> WindowsAPI {
        match name.to_lowercase().as_str() {
            "getlasterror" => WindowsAPI::GetLastError,
            "getcurrentprocess" => WindowsAPI::GetCurrentProcess,
            "getcurrentthreadid" => WindowsAPI::GetCurrentThreadId,
            "getcurrentprocessid" => WindowsAPI::GetCurrentProcessId,
            "virtualalloc" => WindowsAPI::VirtualAlloc,
            "virtualfree" => WindowsAPI::VirtualFree,
            "virtualprotect" => WindowsAPI::VirtualProtect,
            "createfilea" => WindowsAPI::CreateFileA,
            "readfile" => WindowsAPI::ReadFile,
            "writefile" => WindowsAPI::WriteFile,
            "closehandle" => WindowsAPI::CloseHandle,
            "gettickcount" => WindowsAPI::GetTickCount,
            "getsystemtime" => WindowsAPI::GetSystemTime,
            "exitprocess" => WindowsAPI::ExitProcess,
            "messageboxa" => WindowsAPI::MessageBoxA,
            "messageboxw" => WindowsAPI::MessageBoxW,
            "getwindowtexta" => WindowsAPI::GetWindowTextA,
            "getwindowtextw" => WindowsAPI::GetWindowTextW,
            "setwindowtexta" => WindowsAPI::SetWindowTextA,
            "setwindowtextw" => WindowsAPI::SetWindowTextW,
            "createthread" => WindowsAPI::CreateThread,
            "waitforsingleobject" => WindowsAPI::WaitForSingleObject,
            "globalalloc" => WindowsAPI::GlobalAlloc,
            "globalfree" => WindowsAPI::GlobalFree,
            "regopenkeya" => WindowsAPI::RegOpenKeyA,
            "regqueryvaluea" => WindowsAPI::RegQueryValueA,
            _ => WindowsAPI::Unknown,
        }
    }

    /// Gets current system state
    pub fn get_system_state(&self) -> SystemState {
        SystemState {
            current_thread_id: self.current_thread_id,
            current_process_id: self.current_process_id,
            last_error: self.last_error,
            open_file_handles: self.file_handles.len(),
            memory_allocations: self.memory_allocations.len(),
        }
    }
}

/// System state information
#[derive(Debug)]
pub struct SystemState {
    pub current_thread_id: u32,
    pub current_process_id: u32,
    pub last_error: u32,
    pub open_file_handles: usize,
    pub memory_allocations: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syscall_emulator_creation() {
        let emulator = SyscallEmulator::new();
        assert_eq!(emulator.current_thread_id, 1);
        assert_eq!(emulator.current_process_id, 100);
        assert_eq!(emulator.last_error, 0);
    }

    #[test]
    fn test_get_last_error() {
        let mut emulator = SyscallEmulator::new();
        emulator.last_error = 42;
        
        let error = emulator.handle_get_last_error();
        assert_eq!(error, 42);
        assert_eq!(emulator.last_error, 0); // Should be cleared
    }

    #[test]
    fn test_api_mapping() {
        assert_eq!(SyscallEmulator::map_api_name("GetLastError"), WindowsAPI::GetLastError);
        assert_eq!(SyscallEmulator::map_api_name("virtualalloc"), WindowsAPI::VirtualAlloc);
        assert_eq!(SyscallEmulator::map_api_name("unknown"), WindowsAPI::Unknown);
    }

    #[test]
    fn test_virtual_alloc() {
        let mut emulator = SyscallEmulator::new();
        let mut memory = MemoryManager::new().unwrap();
        
        let args = vec![0, 0x1000, 0x3000]; // address, size, allocation_type
        let result = emulator.handle_virtual_alloc(&args, &mut memory).unwrap();
        
        assert_ne!(result, 0); // Should return valid address
        assert_eq!(emulator.memory_allocations.len(), 1);
    }
}