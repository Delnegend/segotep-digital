# Architecture & Technical Design

This document describes the internal architecture, cross-platform telemetry pipelines, reverse-engineered USB HID protocol, and hardware communication flow of `segotep-digital`.

---

## 1. High-Level System Architecture

`segotep-digital` is a standalone, 100% open-source Rust daemon and system service that drives Segotep Digital series AIO liquid cooler screens across Linux and Windows without proprietary vendor utilities.

```mermaid
flowchart TB
    subgraph CoreDaemon["segotep-digital Core Engine"]
        CLI["CLI Parser & Service Dispatcher (clap / windows_service)<br/>Interactive CLI, systemd, or native Windows Service"]
        SIG["Signal & SCM Handler (ctrlc / Win32 SCM)<br/>Traps termination signals for graceful screen teardown"]
        LOOP["Main Polling Loop<br/>Default 1000ms tick cycle"]
        PACKET["Packet Encoder (SegotepPacket)<br/>Serializes 34-byte Little-Endian HID Report"]
    end

    subgraph TelemetrySubsystem["Hardware Telemetry Engine (SystemTelemetry)"]
        direction TB
        LINUX_SRC["Linux Direct Kernel Pipeline<br/>• /sys/class/hwmon (k10temp / coretemp)<br/>• /sys/class/powercap/intel-rapl (RAPL energy in Joules/Watts)<br/>• /proc/stat (Accurate CPU Load %)<br/>• /sys/devices/system/cpu (Core clock frequencies)"]
        WIN_SRC["Windows Native & Kernel Pipeline<br/>• Windows Performance Counters (PDH): CPU Power (Watts) & Boost Clocks (MHz)<br/>• NVIDIA NVML (nvml.dll): GPU Temp, Power, Load %, and Clocks<br/>• PawnIO Kernel Driver: Direct AMD Zen SMN / Intel MSR DTS CPU Temp"]
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
1. **CLI & Service Dispatching**: Arguments are parsed to configure refresh rates, model ID overrides, and units. On Windows, the process can run interactively or dispatch directly through the Windows Service Control Manager (SCM).
2. **Telemetry Sampling**: Every tick (default: 1000ms), `SystemTelemetry` samples CPU and GPU metrics via OS-tailored pipelines.
3. **Packet Encoding**: Metrics are converted into the fixed 34-byte packet format expected by the Segotep display microcontroller.
4. **HID Transmission**: The packet is transferred synchronously over USB HID to refresh the 7-segment display digits and status indicators.

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
- **Magic Bytes (`0xDC 0xDD`)**: Required framing prefix for all valid command packets sent to the device.
- **State Byte (Index 3)**: `0x00` instructs the MCU to stay active and refresh digits; `0x0E` commands screen power down (blank digits and LEDs).
- **Model ID (Index 4)**: `1` for standard digital coolers; `3` for Ice Moon 360 series coolers. When the screen is OFF, this byte is overridden to `0x0F` (`FLASH_VALUE1_OFF`) instead of the model ID (see `src/protocol.rs`).
- **Fixed Markers (`0x01` at index 13, `0x0C` at index 17)**: Protocol framing constants discovered through reverse engineering.
- **16-bit Metric Encodings (Indices 23–26 & 29–32)**: Serialized as little-endian unsigned 16-bit integers (`u16`).

### Protocol Flow & Lifecycle Handshake
```mermaid
sequenceDiagram
    autonumber
    participant App as segotep-digital Daemon / Service
    participant OS as OS Kernel (hidraw / Win32 HID)
    participant MCU as Segotep Display MCU (0x1a86:0xa001)

    Note over App,MCU: Initialization Phase
    App->>OS: hid_open(0x1a86, 0xa001)
    OS-->>App: HID Device Handle

    Note over App,MCU: Continuous Real-Time Telemetry Streaming
    loop Every Interval (e.g. 1000ms)
        App->>App: Sample CPU / GPU Metrics
        App->>App: Encode 34-byte SegotepPacket
        App->>MCU: hid_write(34-byte report)
        Note over MCU: Refresh 7-segment display digits & LEDs
    end

    Note over App,MCU: Graceful Shutdown Phase
    critical Signal Trapped / Service Stop (Ctrl+C / SCM Stop / --screen-off)
        App->>App: Build Screen-OFF Report [0x00, 0xDC, 0xDD, 0x0E, 0x0F, ...]<br/>(Byte 0 = Report ID 0x00; Byte 3 = 0x0E OFF; Byte 4 = 0x0F override)
        App->>MCU: hid_write(Screen-OFF report)
        Note over MCU: Blank display screen & turn off LEDs
        App->>OS: hid_close()
    end
```

---

## 3. Cross-Platform Telemetry Collection Pipeline

```mermaid
flowchart TD
    SAMPLE_START(["SystemTelemetry::sample()"])

    subgraph LinuxPipeline["Linux Pipeline (Kernel sysfs)"]
        direction TB
        L_TEMP["/sys/class/hwmon/*<br/>k10temp (Tctl/Tdie) or coretemp"]
        L_PWR["/sys/class/powercap/intel-rapl<br/>Calculates Watts from delta energy counters (uj)"]
        L_LOAD["/proc/stat<br/>Calculates Load % from delta CPU ticks"]
        L_FREQ["/sys/devices/system/cpu/cpu*/cpufreq/scaling_cur_freq<br/>Averages active CPU core frequencies in MHz"]
    end

    subgraph WindowsPipeline["Windows Native & Driver Pipeline"]
        direction TB
        W_PDH["Windows Performance Counters (PDH)<br/>• \\Energy Meter(rapl_package0_pkg)\\Power (Watts)<br/>• \\Processor Information(_Total)\\Actual Frequency (MHz)<br/>• \\Processor Information(_Total)\\% Processor Utility (Load)"]
        W_NVML["NVIDIA NVML (nvml.dll)<br/>• GPU Temperature, Power (Watts), Clocks, Engine Load %"]
        W_PAWN["PawnIO Signed Driver (\\\\?\\GLOBALROOT\\Device\\PawnIO)<br/>• Executes sandboxed bytecode for AMD SMN (0x00059800) / Intel MSRs<br/>• Real-time CPU DTS die temperature in °C"]
    end

    SAMPLE_START -->|cfg target_os = linux| LinuxPipeline
    SAMPLE_START -->|cfg target_os = windows| WindowsPipeline

    LinuxPipeline --> MERGE["HardwareMetrics Snapshot<br/>cpu_temp, cpu_load, cpu_power, cpu_freq, gpu_*"]
    WindowsPipeline --> MERGE
```

---

## 4. Windows Telemetry Architecture: Open & Secure

### 1. Windows Performance Counters (PDH)
- Queries native Windows kernel performance objects via `pdh.dll`.
- Accurately captures live CPU package RAPL power consumption (Watts) and boosted clock frequencies across all physical/logical cores without high polling overhead.

### 2. NVIDIA NVML Dynamic Binding
- Dynamically loads `nvml.dll` at runtime if present on the system.
- Samples GPU temperature, core frequency, wattage, and utilization directly from the graphics driver.

### 3. Signed PawnIO Driver for Direct CPU Temperature
- Desktop motherboard UEFI implementations omit ACPI thermal zones, making user-space temperature queries impossible without kernel execution.
- `segotep-digital` embeds signed PawnIO bytecode modules (`AMDFamily17.bin`, `IntelMSR.bin`).
- Communicates directly with the kernel device handle `\\?\GLOBALROOT\Device\PawnIO` to query the AMD System Management Unit (SMU) thermal register `0x00059800` (or Intel digital thermal MSRs), providing true hardware die temperature with zero third-party GUI applications required.

---

## 5. Screen State Machine & Lifecycle

The daemon maintains an explicit state machine to guarantee recovery from USB hot-unplugs and ensure screens are never left frozen with outdated values on exit:

```mermaid
stateDiagram-v2
    [*] --> Disconnected: App / Service Started

    Disconnected --> Probing: Scan for VID 0x1A86 / PID 0xA001
    Probing --> Disconnected: Device Not Found (Retry every 2000ms)

    Probing --> Initializing: Device Found & Opened via hidapi
    Initializing --> Streaming: Connection Ready

    state Streaming {
        [*] --> SampleMetrics: Timer Fired (e.g. 1000ms)
        SampleMetrics --> BuildPacket: Collect CPU/GPU Metrics
        BuildPacket --> SendReport: Serialize 34-byte Report
        SendReport --> WaitTick: hid_write() Succeeded
        WaitTick --> SampleMetrics: Next Polling Interval
    }

    Streaming --> Disconnected: USB Cable Unplugged / Write Error
    Streaming --> TurningOff: Signal Caught (Ctrl+C / SCM Stop)
    Streaming --> TurningOff: Explicit CLI Flag --screen-off
    
    TurningOff --> Disconnected: Transmit Screen-OFF Packet (State: 0x0E)
    Disconnected --> [*]: Process Exit
```
