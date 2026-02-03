//! Foreign Function Interface module
//! 
//! Provides JNI bindings for Android integration

#[cfg(target_os = "android")]
use jni::{JNIEnv, objects::{JClass, JString, JObject}, sys::jlong};

use std::sync::{Arc, Mutex};
use crate::EmulationEngine;

/// Global emulation engine instance
static mut EMULATION_ENGINE: Option<Arc<Mutex<EmulationEngine>>> = None;
static mut EMULATION_ENGINE_INIT: std::sync::Once = std::sync::Once::new();

/// Initializes the global emulation engine
fn get_emulation_engine() -> Arc<Mutex<EmulationEngine>> {
    unsafe {
        EMULATION_ENGINE_INIT.call_once(|| {
            EMULATION_ENGINE = Some(Arc::new(Mutex::new(
                EmulationEngine::new().expect("Failed to create emulation engine")
            )));
        });
        EMULATION_ENGINE.as_ref().unwrap().clone()
    }
}

/// JNI method to initialize the emulation engine
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_mobilepc_engine_EngineCore_initializeEmulation(
    env: JNIEnv,
    _class: JClass,
) -> jlong {
    match EmulationEngine::new() {
        Ok(engine) => {
            let engine_arc = Arc::new(Mutex::new(engine));
            let engine_ptr = Arc::into_raw(engine_arc) as jlong;
            engine_ptr
        }
        Err(e) => {
            eprintln!("Failed to initialize emulation engine: {}", e);
            0
        }
    }
}

/// JNI method to execute a PE file
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_mobilepc_engine_EngineCore_executePE(
    env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
    file_path: JString,
) -> jboolean {
    if engine_ptr == 0 {
        eprintln!("Invalid engine pointer");
        return 0;
    }

    let engine = unsafe {
        Arc::from_raw(engine_ptr as *mut Mutex<EmulationEngine>)
    };

    let result = match env.get_string(file_path) {
        Ok(path) => {
            let path_str = path.to_str().unwrap_or_default();
            match engine.lock().unwrap().execute_pe(path_str) {
                Ok(_) => {
                    eprintln!("Successfully executed PE file: {}", path_str);
                    1 // JNI_TRUE
                }
                Err(e) => {
                    eprintln!("Failed to execute PE file {}: {}", path_str, e);
                    0 // JNI_FALSE
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to get file path string: {}", e);
            0 // JNI_FALSE
        }
    };

    // Don't drop the Arc - we're just borrowing it
    Arc::into_raw(engine);
    result
}

/// JNI method to get memory statistics
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_mobilepc_engine_EngineCore_getMemoryStats(
    env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) -> JObject {
    if engine_ptr == 0 {
        return JObject::null();
    }

    let engine = unsafe {
        Arc::from_raw(engine_ptr as *mut Mutex<EmulationEngine>)
    };

    let stats = {
        let engine_guard = engine.lock().unwrap();
        let memory = engine_guard.memory.lock().unwrap();
        memory.get_stats()
    };

    // Create a Java HashMap with the statistics
    let hashmap_class = env.find_class("java/util/HashMap").unwrap();
    let hashmap = env.new_object(hashmap_class, "()V", &[]).unwrap();

    // Put statistics into the HashMap
    let put_method = env.get_method_id(hashmap_class, "put", "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;").unwrap();

    // Helper function to create Java string and put it in the map
    macro_rules! put_stat {
        ($key:expr, $value:expr) => {
            let key_str = env.new_string($key).unwrap();
            let value_str = env.new_string($value.to_string()).unwrap();
            env.call_method_unchecked(
                hashmap,
                put_method,
                jni::signature::("(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;"),
                &[(&key_str).into(), (&value_str).into()]
            ).unwrap();
        };
    }

    put_stat!("total_pages", stats.total_pages);
    put_stat!("total_memory", stats.total_memory);
    put_stat!("used_memory", stats.used_memory);
    put_stat!("free_memory", stats.free_memory);
    put_stat!("regions", stats.regions);

    Arc::into_raw(engine);
    hashmap
}

/// JNI method to get CPU registers
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_mobilepc_engine_EngineCore_getCPURegisters(
    env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) -> JObject {
    if engine_ptr == 0 {
        return JObject::null();
    }

    let engine = unsafe {
        Arc::from_raw(engine_ptr as *mut Mutex<EmulationEngine>)
    };

    let registers = {
        let engine_guard = engine.lock().unwrap();
        let cpu = engine_guard.cpu.lock().unwrap();
        cpu.get_registers().clone()
    };

    // Create a Java HashMap with the register values
    let hashmap_class = env.find_class("java/util/HashMap").unwrap();
    let hashmap = env.new_object(hashmap_class, "()V", &[]).unwrap();

    let put_method = env.get_method_id(hashmap_class, "put", "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;").unwrap();

    macro_rules! put_reg {
        ($key:expr, $value:expr) => {
            let key_str = env.new_string($key).unwrap();
            let value_str = env.new_string(format!("0x{:08X}", $value)).unwrap();
            env.call_method_unchecked(
                hashmap,
                put_method,
                jni::signature::("(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;"),
                &[(&key_str).into(), (&value_str).into()]
            ).unwrap();
        };
    }

    put_reg!("EAX", registers.eax);
    put_reg!("EBX", registers.ebx);
    put_reg!("ECX", registers.ecx);
    put_reg!("EDX", registers.edx);
    put_reg!("ESI", registers.esi);
    put_reg!("EDI", registers.edi);
    put_reg!("EBP", registers.ebp);
    put_reg!("ESP", registers.esp);
    put_reg!("EIP", registers.eip);
    put_reg!("EFLAGS", registers.eflags);

    Arc::into_raw(engine);
    hashmap
}

/// JNI method to set CPU register
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_mobilepc_engine_EngineCore_setCPURegister(
    env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
    register_name: JString,
    value: jint,
) -> jboolean {
    if engine_ptr == 0 {
        return 0;
    }

    let engine = unsafe {
        Arc::from_raw(engine_ptr as *mut Mutex<EmulationEngine>)
    };

    let result = match env.get_string(register_name) {
        Ok(reg_name) => {
            let reg_str = reg_name.to_str().unwrap_or_default();
            let mut success = false;

            {
                let mut engine_guard = engine.lock().unwrap();
                let mut cpu = engine_guard.cpu.lock().unwrap();
                
                match reg_str.to_uppercase().as_str() {
                    "EAX" => { cpu.registers.eax = value as u32; success = true; }
                    "EBX" => { cpu.registers.ebx = value as u32; success = true; }
                    "ECX" => { cpu.registers.ecx = value as u32; success = true; }
                    "EDX" => { cpu.registers.edx = value as u32; success = true; }
                    "ESI" => { cpu.registers.esi = value as u32; success = true; }
                    "EDI" => { cpu.registers.edi = value as u32; success = true; }
                    "EBP" => { cpu.registers.ebp = value as u32; success = true; }
                    "ESP" => { cpu.registers.esp = value as u32; success = true; }
                    "EIP" => { cpu.registers.eip = value as u32; success = true; }
                    "EFLAGS" => { cpu.registers.eflags = value as u32; success = true; }
                    _ => {
                        eprintln!("Unknown register: {}", reg_str);
                        success = false;
                    }
                }
            }

            if success {
                1 // JNI_TRUE
            } else {
                0 // JNI_FALSE
            }
        }
        Err(e) => {
            eprintln!("Failed to get register name string: {}", e);
            0 // JNI_FALSE
        }
    };

    Arc::into_raw(engine);
    result
}

/// JNI method to read memory
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_mobilepc_engine_EngineCore_readMemory(
    env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
    address: jint,
    size: jint,
) -> jbyteArray {
    if engine_ptr == 0 || size <= 0 {
        return JObject::null().into_inner();
    }

    let engine = unsafe {
        Arc::from_raw(engine_ptr as *mut Mutex<EmulationEngine>)
    };

    let result = {
        let engine_guard = engine.lock().unwrap();
        let memory = engine_guard.memory.lock().unwrap();
        
        match memory.read_bytes(address as u32, size as u32) {
            Ok(data) => {
                match env.new_byte_array(data.len() as i32) {
                    Ok(byte_array) => {
                        if env.set_byte_array_region(byte_array, 0, &data).is_ok() {
                            Some(byte_array)
                        } else {
                            eprintln!("Failed to set byte array region");
                            None
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to create byte array: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to read memory at 0x{:08X}: {}", address, e);
                None
            }
        }
    };

    Arc::into_raw(engine);
    
    match result {
        Some(array) => array,
        None => JObject::null().into_inner(),
    }
}

/// JNI method to write memory
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_mobilepc_engine_EngineCore_writeMemory(
    env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
    address: jint,
    data: jbyteArray,
) -> jboolean {
    if engine_ptr == 0 {
        return 0;
    }

    let engine = unsafe {
        Arc::from_raw(engine_ptr as *mut Mutex<EmulationEngine>)
    };

    let result = match env.get_byte_array_elements(data, jni::objects::ReleaseMode::NoCopyBack) {
        Ok(data_elements) => {
            let data_slice = unsafe {
                std::slice::from_raw_parts(data_elements.as_ptr() as *const u8, data_elements.size()? as usize)
            };
            
            let success = {
                let engine_guard = engine.lock().unwrap();
                let mut memory = engine_guard.memory.lock().unwrap();
                
                match memory.write_bytes(address as u32, data_slice) {
                    Ok(_) => true,
                    Err(e) => {
                        eprintln!("Failed to write memory at 0x{:08X}: {}", address, e);
                        false
                    }
                }
            };

            env.release_byte_array_elements(data, data_elements, jni::objects::ReleaseMode::NoCopyBack);
            success
        }
        Err(e) => {
            eprintln!("Failed to get byte array elements: {}", e);
            false
        }
    };

    Arc::into_raw(engine);
    if result { 1 } else { 0 }
}

/// JNI method to step execution
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_mobilepc_engine_EngineCore_stepExecution(
    _env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) -> jboolean {
    if engine_ptr == 0 {
        return 0;
    }

    let engine = unsafe {
        Arc::from_raw(engine_ptr as *mut Mutex<EmulationEngine>)
    };

    let result = {
        let mut engine_guard = engine.lock().unwrap();
        
        // Execute one instruction
        match engine_guard.run() {
            Ok(_) => true,
            Err(e) => {
                eprintln!("Execution step failed: {}", e);
                false
            }
        }
    };

    Arc::into_raw(engine);
    if result { 1 } else { 0 }
}

/// JNI method to cleanup the emulation engine
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_mobilepc_engine_EngineCore_cleanupEmulation(
    _env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) {
    if engine_ptr != 0 {
        let _handle = unsafe {
            Arc::from_raw(engine_ptr as *mut Mutex<EmulationEngine>)
        };
        // Engine will be dropped when Arc goes out of scope
    }
}

/// JNI method to get translation statistics
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_mobilepc_engine_EngineCore_getTranslationStats(
    env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) -> JObject {
    if engine_ptr == 0 {
        return JObject::null();
    }

    let engine = unsafe {
        Arc::from_raw(engine_ptr as *mut Mutex<EmulationEngine>)
    };

    let stats = {
        let engine_guard = engine.lock().unwrap();
        let translator = engine_guard.translator.lock().unwrap();
        translator.get_stats()
    };

    // Create a Java HashMap with the statistics
    let hashmap_class = env.find_class("java/util/HashMap").unwrap();
    let hashmap = env.new_object(hashmap_class, "()V", &[]).unwrap();

    let put_method = env.get_method_id(hashmap_class, "put", "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;").unwrap();

    macro_rules! put_stat {
        ($key:expr, $value:expr) => {
            let key_str = env.new_string($key).unwrap();
            let value_str = env.new_string($value.to_string()).unwrap();
            env.call_method_unchecked(
                hashmap,
                put_method,
                jni::signature::("(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;"),
                &[(&key_str).into(), (&value_str).into()]
            ).unwrap();
        };
    }

    put_stat!("cache_size", stats.cache_size);
    put_stat!("register_mappings", stats.register_mappings);

    Arc::into_raw(engine);
    hashmap
}

/// JNI method to check if emulation is running
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_mobilepc_engine_EngineCore_isEmulationRunning(
    _env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) -> jboolean {
    if engine_ptr == 0 {
        return 0;
    }

    let engine = unsafe {
        Arc::from_raw(engine_ptr as *mut Mutex<EmulationEngine>)
    };

    let is_running = {
        let engine_guard = engine.lock().unwrap();
        let cpu = engine_guard.cpu.lock().unwrap();
        // Simple check - if EIP is not 0, assume we're running
        cpu.get_instruction_pointer() != 0
    };

    Arc::into_raw(engine);
    if is_running { 1 } else { 0 }
}

/// JNI method to reset the emulation engine
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_mobilepc_engine_EngineCore_resetEmulation(
    _env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) -> jboolean {
    if engine_ptr == 0 {
        return 0;
    }

    let engine = unsafe {
        Arc::from_raw(engine_ptr as *mut Mutex<EmulationEngine>)
    };

    let result = {
        // Reset CPU registers
        {
            let mut engine_guard = engine.lock().unwrap();
            let mut cpu = engine_guard.cpu.lock().unwrap();
            
            // Reset registers to default state
            cpu.registers = crate::cpu::X86Registers::default();
        }

        // Clear memory (optional - could keep allocated memory)
        {
            let mut engine_guard = engine.lock().unwrap();
            let mut memory = engine_guard.memory.lock().unwrap();
            memory.clear_dirty_flags();
        }

        true
    };

    Arc::into_raw(engine);
    if result { 1 } else { 0 }
}

/// JNI method to get system call statistics
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_mobilepc_engine_EngineCore_getSystemStats(
    env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) -> JObject {
    if engine_ptr == 0 {
        return JObject::null();
    }

    let engine = unsafe {
        Arc::from_raw(engine_ptr as *mut Mutex<EmulationEngine>)
    };

    // For now, return a simple status map
    let hashmap_class = env.find_class("java/util/HashMap").unwrap();
    let hashmap = env.new_object(hashmap_class, "()V", &[]).unwrap();

    let put_method = env.get_method_id(hashmap_class, "put", "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;").unwrap();

    macro_rules! put_stat {
        ($key:expr, $value:expr) => {
            let key_str = env.new_string($key).unwrap();
            let value_str = env.new_string($value.to_string()).unwrap();
            env.call_method_unchecked(
                hashmap,
                put_method,
                jni::signature::("(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;"),
                &[(&key_str).into(), (&value_str).into()]
            ).unwrap();
        };
    }

    put_stat!("status", "running");
    put_stat!("engine_ptr", engine_ptr);
    put_stat!("timestamp", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs());

    Arc::into_raw(engine);
    hashmap
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emulation_engine_creation() {
        // This test would only work on Android
        #[cfg(target_os = "android")]
        {
            let engine = EmulationEngine::new();
            assert!(engine.is_ok());
        }
    }
}