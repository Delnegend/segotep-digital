# Architecture & Technical Design

This document describes the internal architecture, cross-platform telemetry pipelines, reverse-engineered USB HID protocol, and hardware communication flow of `segotep-digital`.

---

## 1. High-Level System Architecture

`segotep-digital` is designed as a standalone, zero-overhead Rust daemon and CLI tool that drives Segotep Digital series AIO liquid cooler screens across both Linux and Windows without mandatory official desktop utilities.

```mermaid
flowchart TB
    subgraph CoreDaemon["segotep-digital Core Daemon"]
        CLI["CLI Parser (clap)<br/>Parses flags: interval, model ID, Fahrenheit, screen off"]
        SIG["Signal Handler (ctrlc)<br/>Traps SIGINT / SIGTERM for graceful screen teardown"]
        LOOP["Main Polling Loop<br/>Default 1000ms tick cycle"]
        PACKET["Packet Encoder (SegotepPacket)<br/>Serializes 34-byte Little-Endian HID Report"]
    end

    subgraph TelemetrySubsystem["Hardware Telemetry Engine (SystemTelemetry)"]
        direction TB
        LINUX_SRC["Linux Native Sources<br/>• /sys/class/hwmon (Temp via k10temp / coretemp)<br/>• /sys/class/powercap/intel-rapl (Watts via RAPL counters)<br/>• /proc/stat (Load delta ticks)<br/>• /sys/devices/system/cpu (Scaling frequency)"]
        WIN_SRC["Windows Multi-Source Engine<br/>• Tier 1: Segotep LDGT JSON Buffer (2MB mapped memory)<br/>• Tier 2: AIDA64 / HWiNFO / LibreHardwareMonitor XML (256KB)<br/>• Tier 3: Native User-Space sysinfo Fallback"]
    end

    subgraph HardwareLayer["Segotep Hardware USB Endpoint"]
        HID_API["hidapi (WCH CH55x / Custom MCU)<br/>Target: VID 0x1A86 | PID 0xA001"]
        SCREEN["7-Segment Hardware Pump Display<br/>CPU Temp (°C/°F) • Load (%) • Power (W) • Clock (MHz)"]
    end

    CLI -->|Config Options| LOOP
    SIG -->|Termination Event| LOOP
    TelemetrySubsystem -->|HardwareMetrics Snapshot| LOOP
    LOOP -->|Periodic Tick| PACKET
    PACKET -->|34-byte Output Report| HID_API
    HID_API -->|Direct USB Transfers| SCREEN
```

### Flow Summary
1. **CLI & Signal Initialization**: The application parses command-line arguments (polling rate, model ID overrides, temperature units) and hooks OS termination signals (`Ctrl+C`, `SIGTERM`).
2. **Telemetry Sampling**: Every tick (default: 1000ms), `SystemTelemetry` samples CPU and GPU metrics via OS-specific pipelines.
3. **Packet Encoding**: Metrics are converted into a fixed 34-byte packet layout required by the Segotep microcontroller.
4. **HID Transmission**: The packet is transferred synchronously over USB HID to update the 7-segment display digits and status LEDs.

---

## 2. Hardware USB HID Protocol & Packet Anatomy

The display communicates over USB HID (`VID: 0x1A86`, `PID: 0xA001`) using a 34-byte output report.

### 34-Byte Packet Anatomy
```mermaid
packet-beta
0-7: "Byte 0: Report ID (0x00)"
8-15: "Byte 1: Magic Header 0xDC"
16-23: "Byte 2: Magic Header 0xDD"
24-31: "Byte 3: State (0x00 ON / 0x0E OFF)"
32-39: "Byte 4: Model ID (1 or 3)"
40-103: "Bytes 5-12: Reserved Padding (0x00)"
104-111: "Byte 13: Protocol Marker (0x01)"
112-135: "Bytes 14-16: Reserved (0x00)"
136-143: "Byte 17: Protocol Marker (0x0C)"
144-167: "Bytes 18-20: Reserved (0x00)"
168-175: "Byte 21: CPU Temp (°C / °F)"
176-183: "Byte 22: CPU Load (%)"
184-199: "Bytes 23-24: CPU Power Watts (LE u16)"
200-215: "Bytes 25-26: CPU Clock MHz (LE u16)"
216-223: "Byte 27: GPU Temp (°C / °F)"
224-231: "Byte 28: GPU Load (%)"
232-247: "Bytes 29-30: GPU Power Watts (LE u16)"
248-263: "Bytes 31-32: GPU Clock MHz (LE u16)"
264-271: "Byte 33: Unit (0x00=°C, 0x01=°F)"
```

### Key Field Descriptions
- **Magic Bytes (`0xDC 0xDD`)**: Required prefix for all valid command packets sent to the device.
- **State Byte (Index 3)**: `0x00` instructs the MCU to stay active and display incoming metrics; `0x0E` commands screen power down.
- **Model ID (Index 4)**: `1` for standard digital coolers; `3` for Ice Moon 360 series coolers.
- **Fixed Markers (`0x01` at index 13, `0x0C` at index 17)**: Protocol framing constants discovered through binary reverse engineering of the official driver.
- **16-bit Metric Encodings (Indices 23–26 & 29–32)**: Serialized as little-endian unsigned 16-bit integers (`u16`).

### Protocol Flow & Lifecycle Handshake
```mermaid
sequenceDiagram
    autonumber
    participant App as segotep-digital Daemon
    participant OS as OS Kernel (hidraw / Win32 HID)
    participant MCU as Segotep Display MCU (0x1a86:0xa001)

    Note over App,MCU: Initialization & Handshake Phase
    App->>OS: hid_open(0x1a86, 0xa001)
    OS-->>App: HID Device Handle

    opt Auto-Detection Handshake
        App->>MCU: hid_get_input_report() [500ms timeout]
        alt Report Received
            MCU-->>App: 64-byte Info Report (Model ID, CapMask, Fahrenheit flag)
        else Probed Timeout
            Note over App: Fall back to Model ID 3 (Ice Moon) or 1
        end
    end

    Note over App,MCU: Continuous Real-Time Telemetry Streaming
    loop Every Interval (e.g. 1000ms)
        App->>App: Sample CPU / GPU Metrics
        App->>App: Encode 34-byte SegotepPacket
        App->>MCU: hid_write(34-byte report)
        Note over MCU: Refresh 7-segment display digits & LEDs
    end

    Note over App,MCU: Graceful Shutdown Phase
    critical Signal Trapped (Ctrl+C / SIGTERM / --screen-off)
        App->>App: Build Screen-OFF Report [0xDC, 0xDD, 0x0E, 0x0F, ...]
        App->>MCU: hid_write(Screen-OFF report)
        Note over MCU: Blank display screen & turn off LEDs
        App->>OS: hid_close()
    end
```

---

## 3. Cross-Platform Telemetry Collection Pipeline

Because user-space applications cannot read hardware MSR registers (such as AMD SMU power or Intel RAPL energy counters) the same way across operating systems, `segotep-digital` implements tailored OS telemetry backends.

```mermaid
flowchart TD
    subgraph TelemetryCollector["SystemTelemetry::sample()"]
        SAMPLE_START(["Read System Sensors"])
    end

    subgraph LinuxPipeline["Linux Pipeline (Direct Kernel sysfs)"]
        direction TB
        L_TEMP["/sys/class/hwmon/*<br/>Reads k10temp (Tctl/Tdie) or coretemp"]
        L_PWR["/sys/class/powercap/intel-rapl<br/>Calculates Watts from delta energy counters (uj)"]
        L_LOAD["/proc/stat<br/>Calculates Load % from delta user/nice/system/idle ticks"]
        L_FREQ["/sys/devices/system/cpu/cpu*/cpufreq/scaling_cur_freq<br/>Averages active CPU core frequencies in MHz"]
    end

    subgraph WindowsPipeline["Windows Multi-Source Pipeline"]
        direction TB
        W_T1["Tier 1: Segotep LDGT Engine (2MB Shared Memory)<br/>• Maps 'shareMemory_LDGTInfo' double-buffered bank<br/>• Auto-spawns background helper if present on disk<br/>• Fast zero-allocation JSON parser for Tctl/Tdie & RAPL Watts"]
        W_T2["Tier 2: AIDA64 / HWiNFO / LibreHardwareMonitor (256KB)<br/>• Maps 'AIDA64_SensorValues' shared memory stream<br/>• XML parser for TCPU, PPCU (Package Watts), and Clocks"]
        W_T3["Tier 3: Windows Native sysinfo Fallback<br/>• User-space CPU Load % and Base Frequency<br/>• Requires no drivers or background applications"]
    end

    SAMPLE_START -->|cfg target_os = linux| LinuxPipeline
    SAMPLE_START -->|cfg target_os = windows| WindowsPipeline

    W_T1 -->|Missing Data / Standalone| W_T2
    W_T2 -->|Missing Data / Standalone| W_T3

    LinuxPipeline --> MERGE["HardwareMetrics Output Struct<br/>cpu_temp, cpu_load, cpu_power, cpu_freq, gpu_*"]
    WindowsPipeline --> MERGE
```

### OS Implementation Differences
- **On Linux**: 100% native and driverless. The kernel provides unprivileged read access to thermal and powercap counters via `sysfs` (`/sys/class/hwmon` and `/sys/class/powercap/intel-rapl`).
- **On Windows**: Windows blocks direct user-space MSR access. The application executes a cascading 3-tier fallback to capture live die temperatures and wattage without hard dependencies on any single utility.

---

## 4. Windows Multi-Tier Sensor Engine Fallback

On Windows, hardware telemetry is evaluated in runtime tiers. The highest-fidelity active sensor source is chosen automatically:

```mermaid
flowchart TD
    SAMPLE(["Sample Hardware Telemetry"]) --> TIER1{"Tier 1: Segotep LDGT Engine<br/>('shareMemory_LDGTInfo' 2MB)"}

    TIER1 -->|Memory Mapped & Non-Empty| PARSE_JSON["Parse JSON Stream<br/>• CPU Tctl/Tdie Temperature<br/>• CPU Package Power (Watts)<br/>• Real-time Core Frequency"]
    TIER1 -->|Not Found / Missing Values| TIER2{"Tier 2: AIDA64 / HWiNFO64 / LHM<br/>('AIDA64_SensorValues' 256KB)"}

    PARSE_JSON --> EMIT(["Precision HardwareMetrics Output"])

    TIER2 -->|Stream Active| PARSE_XML["Parse XML Stream<br/>• Extract TCPU (°C) & PPCU (Watts)<br/>• Extract GPU Temperature & Watts"]
    TIER2 -->|Not Running / Empty| TIER3["Tier 3: Native sysinfo Fallback<br/>• User-space CPU Load (%) & Base Clocks<br/>• Zero dependencies"]

    PARSE_XML --> EMIT
    TIER3 --> EMIT
```

### Fallback Tiers Explained
1. **Tier 1 (Segotep LDGT Shared Memory)**:
   - Allocates a 2MB double-buffered memory map (`shareMemory_LDGTInfo`).
   - Automatically detects and spawns `LDGT.exe` from `C:\Program Files\Segotep DigitalCAP\` or portable directory if present.
   - Extracts exact `CPU (Tctl/Tdie)` temperature, `CPU Package Power` (RAPL Watts), and core clocks from structured JSON.
2. **Tier 2 (AIDA64 / HWiNFO64 / LibreHardwareMonitor Shared Memory)**:
   - If the Segotep software is not installed, but the user has **AIDA64**, **HWiNFO64** (with shared memory enabled), or **LibreHardwareMonitor** running, the daemon attaches to `AIDA64_SensorValues` (256KB XML map).
   - Reads `TCPU`, `PCPU`, and clock nodes directly from shared memory.
3. **Tier 3 (Pure Standalone sysinfo)**:
   - If no Ring-0 hardware monitors are active, `segotep-digital` falls back to user-space `sysinfo`.
   - Continues driving CPU load percentage, core counts, and nominal clock speeds to the cooler without throwing errors or halting.

---

## 5. Screen State Machine & Lifecycle

The daemon maintains an explicit state machine to guarantee recovery from USB hot-unplugs and ensure screens are never left frozen with outdated values on exit:

```mermaid
stateDiagram-v2
    [*] --> Disconnected: App Launched

    Disconnected --> Probing: Scan for VID 0x1A86 / PID 0xA001
    Probing --> Disconnected: Device Not Found (Retry every 2000ms)

    Probing --> Initializing: Device Found & Opened via hidapi
    Initializing --> AutoDetecting: Query Hardware Feature Report (500ms timeout)
    
    AutoDetecting --> Streaming: Feature Report Handshake OK (Model ID Resolved)
    AutoDetecting --> Streaming: Handshake Timeout (Fallback / User Override -m)

    state Streaming {
        [*] --> SampleMetrics: Timer Fired (e.g. 1000ms)
        SampleMetrics --> BuildPacket: Collect CPU/GPU Metrics
        BuildPacket --> SendReport: Serialize 34-byte Report
        SendReport --> WaitTick: hid_write() Succeeded
        WaitTick --> SampleMetrics: Next Polling Interval
    }

    Streaming --> Disconnected: USB Cable Unplugged / Write Error
    Streaming --> TurningOff: Signal Caught (Ctrl+C / SIGTERM)
    Streaming --> TurningOff: Explicit CLI Flag --screen-off
    
    TurningOff --> Disconnected: Transmit Screen-OFF Packet (State: 0x0E, Model: 0x0F)
    Disconnected --> [*]: Process Exit
```

### State Definitions
- **`Disconnected`**: Initial state before USB attachment or following a communication error / physical disconnect.
- **`Probing`**: Periodically checks connected HID device tables for Segotep hardware.
- **`Initializing`**: Establishes a handle via `hidapi` and sets up OS telemetry hooks.
- **`AutoDetecting`**: Attempts to read the device's capability header to resolve Model ID (1 vs. 3) automatically.
- **`Streaming`**: The active polling loop sending live 34-byte telemetry reports at the user-specified interval.
- **`TurningOff`**: Final graceful teardown state where a blanking packet (`0x0E 0x0F`) is transmitted to turn off LEDs and the 7-segment display.
