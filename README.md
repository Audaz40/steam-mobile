# SteamMobile - PC Game Emulation for Mobile Devices

SteamMobile is a sophisticated emulation platform that enables running PC games natively on mobile devices. This project implements a complete x86-to-ARM binary translation system, Windows compatibility layer, and graphics translation pipeline.

## Architecture Overview

### Core Components

1. **Binary Translation Engine** (`rust/src/translator/`)
   - Real-time x86 to ARM instruction translation
   - Instruction caching and optimization
   - Register mapping and memory management

2. **CPU Emulation** (`rust/src/cpu/`)
   - Full x86 CPU emulation with register state
   - Instruction decoding and execution
   - Flag management and control flow

3. **Memory Management** (`rust/src/memory/`)
   - Virtual memory management with 4KB pages
   - Memory protection and allocation
   - Cross-platform memory mapping

4. **PE Loader** (`rust/src/pe/`)
   - Windows PE file parsing and loading
   - Import resolution and relocation processing
   - Section loading with proper permissions

5. **System Call Emulation** (`rust/src/syscall/`)
   - Windows API compatibility layer
   - File I/O, memory management, and registry emulation
   - Thread and process management

6. **Runtime Orchestration** (`rust/src/runtime/`)
   - Frame-by-frame execution budget control
   - Thermal-aware performance governance
   - Unified input and graphics coordination

7. **Input System** (`rust/src/input/`)
   - Touch overlay mapping
   - Controller bindings and axis mapping
   - Unified input action queue

8. **Power & Thermal Management** (`rust/src/power/`)
   - Thermal policy thresholds and FPS recommendations
   - Frame pacing and throttling

9. **Android Integration** (`rust/src/ffi/`)
   - JNI bindings for native Android integration
   - Performance monitoring and debugging interfaces
   - Resource management and lifecycle control

### Android App Structure

The Android application (`app/`) provides:

- **Core Engine** (`app/src/main/java/com/mobilepc/engine/core/`)
  - Main emulation coordination
  - Memory bridge between Java and native code
  - Process and thermal management

- **Graphics System** (`app/src/main/java/com/mobilepc/engine/graphics/`)
  - Vulkan surface management
  - Frame rate control
  - Graphics pipeline coordination

- **User Interface** (`app/src/main/java/com/mobilepc/engine/ui/`)
  - Touch-friendly game interface
  - Virtual controls overlay
  - Settings and configuration

- **Input Handling** (`app/src/main/java/com/mobilepc/engine/input/`)
  - Touch input translation
  - Virtual controller mapping
  - Gesture recognition

## Key Features

### ✅ Implemented
- **x86 to ARM Binary Translation**: Real-time instruction translation with caching
- **Windows PE Loading**: Complete PE file format support with imports and relocations
- **Memory Management**: Virtual memory system with page-level protection
- **System Call Emulation**: Core Windows API functions (Kernel32, User32)
- **Android JNI Integration**: Full native bridge for Android apps
- **CPU Emulation**: Complete x86 register set and instruction execution
- **Input Overlay System**: Touch and controller mapping with action queue
- **Thermal Governance**: Thermal policy + frame pacing integration

### 🚧 In Progress
- **DirectX to Vulkan Translation**: Graphics pipeline translation layer
- **Touch UI**: Mobile-optimized game interface

### 📋 Planned
- **Advanced JIT Compilation**: LLVM-based optimization for performance-critical code
- **Multi-threading Support**: SMP emulation for multi-core games
- **Network Emulation**: Windows networking API compatibility
- **Audio System**: DirectSound to Android audio translation

## Technical Specifications

### Supported Architectures
- **Host**: ARM64 (Android devices)
- **Target**: x86 (32-bit Windows applications)

### Performance Optimizations
- **Instruction Caching**: Translated instructions cached for reuse
- **Memory Pooling**: Efficient memory allocation patterns
- **Lazy Loading**: On-demand PE section loading
- **Thermal Throttling**: Dynamic performance adjustment

### Compatibility
- **Windows Versions**: Windows XP through Windows 10 (32-bit)
- **API Level**: Windows API subset focused on gaming applications
- **Graphics**: DirectX 9/10/11 translation to Vulkan
- **Audio**: DirectSound compatibility planned

## Build Requirements

### Rust Dependencies
- `tokio` - Async runtime
- `jni` - Android JNI bindings
- `goblin` - Binary parsing
- `parking_lot` - High-performance synchronization
- `mmap` - Memory mapping utilities

### Android Requirements
- **Minimum SDK**: API 24 (Android 7.0)
- **Target SDK**: API 34 (Android 14)
- **NDK**: r25 or later
- **Vulkan**: Vulkan 1.0+ support required

## Usage Example

```kotlin
// Initialize emulation engine
val enginePtr = EngineCore.initializeEmulation()

// Load and execute a PC game
val success = EngineCore.executePE(enginePtr, "/path/to/game.exe")

// Monitor performance
val stats = EngineCore.getMemoryStats(enginePtr)
val registers = EngineCore.getCPURegisters(enginePtr)

// Step through execution for debugging
val stepSuccess = EngineCore.stepExecution(enginePtr)

// Cleanup when done
EngineCore.cleanupEmulation(enginePtr)
```

## Development Roadmap

### Phase 1: Core Emulation ✅
- [x] Binary translation engine
- [x] CPU and memory management
- [x] PE loading and system calls
- [x] Android integration

### Phase 2: Graphics & Input 🚧
- [ ] DirectX to Vulkan translation
- [ ] Touch input system
- [ ] Audio support
- [ ] Performance optimization

### Phase 3: Advanced Features 📋
- [ ] Multi-threading support
- [ ] Network compatibility
- [ ] Advanced JIT compilation
- [ ] Cloud gaming integration

## Performance Targets

- **CPU Overhead**: < 20% performance penalty vs native
- **Memory Usage**: < 2x native application memory
- **Frame Rate**: 30+ FPS for supported games
- **Startup Time**: < 10 seconds for most applications
- **Battery Life**: < 2x native battery consumption

## Contributing

This is a complex systems programming project requiring expertise in:
- Computer architecture and instruction sets
- Operating systems internals
- Graphics programming (Vulkan, DirectX)
- Mobile development (Android NDK)
- Performance optimization

### Development Guidelines
- Follow Rust best practices and clippy recommendations
- Maintain comprehensive test coverage
- Document all public APIs
- Profile and optimize critical paths
- Ensure thread safety for all shared state

## License

This project is dual-licensed under MIT or Apache-2.0, allowing for both open source and commercial use.

## Acknowledgments

This project builds upon concepts from:
- **Box64**: x86 emulation on ARM
- **Wine**: Windows API compatibility layer
- **DXVK**: DirectX to Vulkan translation
- **QEMU**: Dynamic binary translation techniques

## Disclaimer

This project is for educational and research purposes. Commercial use of game emulation may require additional licensing and compliance with game publisher terms of service.

---

**SteamMobile** - Bringing PC Gaming to Mobile Devices 🎮📱
