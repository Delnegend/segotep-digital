# Architecture & Technical Design

This document describes the internal architecture, cross-platform telemetry pipelines, reverse-engineered USB HID protocol, and hardware communication flow of `segotep-digital`.

---

## 1. High-Level System Architecture

`segotep-digital` is a lightweight, zero-overhead Rust daemon and CLI tool that drives Segotep Digital series AIO liquid cooler screens across both Linux and Windows.

```mermaid
flowchart TB
    subgraph CoreDaemon["segotep-digital Core Daemon"]
        CLI["CLI Parser (clap)<br/>Interval / Model ID / Fahrenheit / Screen Off"]
        SIG["Signal Handler (ctrlc)<br/>Graceful Termination"]
        LOOP["Main Polling Loop (1000ms default)"]
        PACKET["Packet Encoder (SegotepPacket)<br/>34-byte Little-Endian HID Report"]
    end

    subgraph TelemetrySubsystem["Hardware Telemetry Engine (SystemTelemetry)"]
        direction TB
        LINUX_SRC["Linux Native Sources<br/>• /sys/class/hwmon (Temp)<br/>• /sys/class/powercap/intel-rapl (Power)<br/>• /proc/stat (Load)<br/>• /sys/devices/system/cpu (Clock)"]
        WIN_SRC["Windows Multi-Source Engine<br/>• Tier 1: Segotep LDGT JSON Buffer (2MB)<br/>• Tier 2: AIDA64 / HWiNFO / LibreHardwareMonitor XML (256KB)<br/>• Tier 3: Native User-Space sysinfo Fallback"]
    end

    subgraph HardwareLayer["Segotep Hardware USB Endpoint"]
        HID_API["hidapi (WCH CH55x / Custom MCU)<br/>VID: 0x1A86 | PID: 0xA001"]
        SCREEN["7-Segment Hardware Display<br/>CPU Temp • Load % • Power (W) • Clock (MHz)"]
    end

    CLI --> LOOP
    SIG --> LOOP
    TelemetrySubsystem -->|HardwareMetrics| LOOP
    LOOP --> PACKET
    PACKET -->|34-byte Report| HID_API
    HID_API --> SCREEN
```

---

## 2. Hardware USB HID Protocol & Packet Anatomy

The display communicates over USB HID (`VID: 0x1A86`, `PID: 0xA001`) via a fixed 34-byte output report sent at a regular polling interval (default: 1000ms).

```mermaid
packet-beta
0-7: "Byte 0: Report ID (0x00)"
8-15: "Byte 1: Magic 0xDC"
16-23: "Byte 2: Magic 0xDD"
24-31: "Byte 3: State (0x00 ON / 0x0E OFF)"
32-39: "Byte 4: Model ID (1 or 3)"
40-103: "Bytes 5-12: Reserved (0x00)"
104-111: "Byte 13: Header Marker (0x01)"
112-135: "Bytes 14-16: Reserved (0x00)"
136-143: "Byte 17: Header Marker (0x0C)"
144-167: "Bytes 18-20: Reserved (0x00)"
168-175: "Byte 21: CPU Temp (°C)"
176-183: "Byte 22: CPU Load (%)"
184-199: "Bytes 23-24: CPU Power Watts (LE u16)"
200-215: "Bytes 25-26: CPU Clock MHz (LE u16)"
216-223: "Byte 27: GPU Temp (°C)"
224-231: "Byte 28: GPU Load (%)"
232-247: "Bytes 29-30: GPU Power Watts (LE u16)"
248-263: "Bytes 31-32: GPU Clock MHz (LE u16)"
264-271: "Byte 33: Unit (0=°C, 1=°F)"
```

### Protocol Flow & Handshake
```mermaid
sequenceDiagram
    autonumber
    participant App as segotep-digital
    participant OS as OS Kernel (Win32 / hidraw)
    participant MCU as Segotep Display MCU (0x1a86:0xa001)

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

    loop Every Interval (e.g. 1000ms)
        App->>App: SystemTelemetry::sample()
        App->>App: Encode 34-byte packet [0xDC, 0xDD, ...]
        App->>MCU: hid_write(34-byte report)
        Note over MCU: Update 7-segment display digits & LEDs
    end

    critical Graceful Shutdown (SIGINT / Ctrl-C)
        App->>App: Encode Screen-OFF packet [0xDC, 0xDD, 0x0E, 0x0F, ...]
        App->>MCU: hid_write(Screen-OFF report)
        App->>OS: hid_close()
    end
```

---

## 3. Cross-Platform Telemetry Collection Pipeline

Getting precise CPU Package Power (Watts) and CPU Die Temperature (`Tctl/Tdie`) requires different approaches across operating systems due to kernel isolation:

```mermaid
flowchart TD
    subgraph TelemetryCollector["SystemTelemetry::sample()"]
        SAMPLE_START(["Collect Hardware Metrics"])
    end

    subgraph LinuxPipeline["Linux Pipeline (Kernel sysfs)"]
        direction TB
        L_TEMP["/sys/class/hwmon/*<br/>k10temp / coretemp"]
        L_PWR["/sys/class/powercap/intel-rapl<br/>MSR Energy Counter Sampling"]
        L_LOAD["/proc/stat<br/>Delta User/Nice/System/Idle ticks"]
        L_FREQ["/sys/devices/system/cpu/cpu*/cpufreq/scaling_cur_freq"]
    end

    subgraph WindowsPipeline["Windows Multi-Source Pipeline"]
        direction TB
        W_T1["Tier 1: Segotep LDGT / HWiNFO64 Driver (2MB)<br/>• 'shareMemory_LDGTInfo' double buffer<br/>• Auto-spawns LDGT helper if present<br/>• Zero-allocation JSON stream parser"]
        W_T2["Tier 2: AIDA64 / HWiNFO / LibreHardwareMonitor (256KB)<br/>• 'AIDA64_SensorValues' shared memory<br/>• XML telemetry parser for TCPU / PPCU (Watts) / Clock"]
        W_T3["Tier 3: Windows Native sysinfo Fallback<br/>• Pure user-space Load % and Base Frequency<br/>• No driver or external service needed"]
    end

    SAMPLE_START -->|cfg target_os = linux| LinuxPipeline
    SAMPLE_START -->|cfg target_os = windows| WindowsPipeline

    W_T1 -->|Missing Data / Standalone| W_T2
    W_T2 -->|Missing Data / Standalone| W_T3

    LinuxPipeline --> MERGE["HardwareMetrics Output"]
    WindowsPipeline --> MERGE
```

---

## 4. Windows Shared Memory IPC & Sensor Engine Architecture

Because user-space Windows applications cannot read x86 MSR registers (like `MSR_RAPL_POWER_UNIT` `0x611` or AMD SMU Mailbox `0x3B10528`) without a Microsoft-attested Ring-0 driver, `segotep-digital` implements a multi-tier zero-overhead shared memory bridge:

```mermaid
flowchart LR
    subgraph SegotepApp["segotep-digital (Rust Process)"]
        MAPPER["WindowsSharedMemoryReader"]
        JSON_P["JSON Stream Parser<br/>(LDGT Bank)"]
        XML_P["XML Stream Parser<br/>(AIDA64 Bank)"]
    end

    subgraph WinKernelMemory["Windows Kernel Shared Memory Bank"]
        MEM_LDGT["shareMemory_LDGTInfo (2MB)<br/>• Double-buffered JSON live telemetry"]
        MEM_AIDA["AIDA64_SensorValues (256KB)<br/>• Standardized XML sensor stream"]
    end

    subgraph SensorProviders["Hardware Sensor Engines"]
        ENGINE_LDGT["Segotep LDGT Helper<br/>• Auto-detected & spawned"]
        EXT_MONITORS["3rd-Party Monitor<br/>• HWiNFO64 / AIDA64 / LibreHardwareMonitor"]
        RING0["Attested Ring-0 Kernel Driver<br/>(HWiNFO64.sys / WinRing0.sys)"]
        HARDWARE["CPU MSRs (RAPL) & AMD SMU<br/>Tctl/Tdie & Package Power"]
    end

    MAPPER -->|CreateFileMappingA| MEM_LDGT
    MAPPER -->|OpenFileMappingA| MEM_AIDA
    MAPPER -->|Auto-Spawn| ENGINE_LDGT

    ENGINE_LDGT --> RING0
    EXT_MONITORS --> RING0
    RING0 --> HARDWARE
    HARDWARE --> RING0

    RING0 -->|Writes JSON| MEM_LDGT
    EXT_MONITORS -->|Writes XML| MEM_AIDA

    MEM_LDGT --> JSON_P
    MEM_AIDA --> XML_P
    JSON_P --> MAPPER
    XML_P --> MAPPER
```

---

## 5. Screen State Machine & Lifecycle

The display controller follows a robust state transition model to handle connection initialization, telemetry streaming, device disconnect/reconnect cycles, and clean screen power-off on shutdown:

```mermaid
stateDiagram-v2
    [*] --> Disconnected: App Launched

    Disconnected --> Probing: Locate VID 0x1A86 / PID 0xA001
    Probing --> Disconnected: Device Not Found (Retry loop)

    Probing --> Initializing: Device Connected via HID
    Initializing --> AutoDetecting: Query Hardware Feature Report (500ms)
    
    AutoDetecting --> Streaming: Report Received (Model ID Resolved)
    AutoDetecting --> Streaming: Timeout Fallback / User Override (-m)

    state Streaming {
        [*] --> SampleMetrics
        SampleMetrics --> BuildPacket: Read Sensors
        BuildPacket --> SendReport: Encode 34-byte Header & Values
        SendReport --> WaitTick: hid_write() Success
        WaitTick --> SampleMetrics: Interval Elapsed (e.g. 1000ms)
    }

    Streaming --> Disconnected: USB Unplugged / Send Error
    Streaming --> TurningOff: Termination Signal (SIGINT / Ctrl-C)
    Streaming --> TurningOff: Flag --screen-off
    
    TurningOff --> Disconnected: Transmit Screen-OFF Packet (0x0E / 0x0F)
    Disconnected --> [*]: Process Terminated
```
