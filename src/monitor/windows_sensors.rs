//! Windows shared memory sensor provider (Segotep LDGT, `HWiNFO`, `AIDA64`, `LibreHardwareMonitor`).

#[cfg(target_os = "windows")]
use std::ffi::c_void;
#[cfg(target_os = "windows")]
use std::path::Path;
#[cfg(target_os = "windows")]
use std::process::Command;
#[cfg(target_os = "windows")]
use std::ptr::null;
#[cfg(target_os = "windows")]
use tracing::{debug, info, warn};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Memory::{
    CreateFileMappingA, FILE_MAP_ALL_ACCESS, FILE_MAP_READ, MapViewOfFile, OpenFileMappingA,
    PAGE_READWRITE, UnmapViewOfFile,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct WindowsSensorValues {
    pub cpu_temp: Option<u8>,
    pub cpu_power: Option<u16>,
    pub cpu_clock: Option<u16>,
    pub gpu_temp: Option<u8>,
    pub gpu_power: Option<u16>,
    pub gpu_clock: Option<u16>,
}

#[cfg(target_os = "windows")]
pub struct WindowsSharedMemoryReader {
    ldgt_handle: HANDLE,
    ldgt_ptr: *const u8,
    aida_handle: HANDLE,
    aida_ptr: *const u8,
    is_open: bool,
    tried_autostart: bool,
    sample_count: u32,
}

#[cfg(target_os = "windows")]
impl Default for WindowsSharedMemoryReader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "windows")]
impl WindowsSharedMemoryReader {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            ldgt_handle: std::ptr::null_mut(),
            ldgt_ptr: std::ptr::null(),
            aida_handle: std::ptr::null_mut(),
            aida_ptr: std::ptr::null(),
            is_open: false,
            tried_autostart: false,
            sample_count: 0,
        }
    }

    /// Creates or opens `shareMemory_LDGTInfo` and `AIDA64_SensorValues` memory banks.
    #[allow(clippy::as_conversions, clippy::if_not_else)]
    pub fn try_open(&mut self) -> bool {
        if self.is_open && (!self.ldgt_ptr.is_null() || !self.aida_ptr.is_null()) {
            return true;
        }

        self.close();

        // 1. Primary: Segotep LDGT / HWiNFO shared memory (2MB double-buffer)
        let ldgt_name = b"shareMemory_LDGTInfo\0";
        let ldgt_handle = unsafe {
            CreateFileMappingA(
                INVALID_HANDLE_VALUE,
                null(),
                PAGE_READWRITE,
                0,
                2_097_152,
                ldgt_name.as_ptr(),
            )
        };

        if !ldgt_handle.is_null() {
            let map_ptr =
                unsafe { MapViewOfFile(ldgt_handle, FILE_MAP_ALL_ACCESS, 0, 0, 2_097_152) };
            if map_ptr.Value.is_null() {
                unsafe { CloseHandle(ldgt_handle) };
            } else {
                self.ldgt_handle = ldgt_handle;
                self.ldgt_ptr = map_ptr.Value.cast::<u8>();
                self.is_open = true;
                info!("Initialized 2MB Windows LDGT/HWiNFO shared memory sensor bank");
            }
        }

        // 2. Secondary: AIDA64 / HWiNFO / LibreHardwareMonitor XML memory bank
        let aida_name = b"AIDA64_SensorValues\0";
        let aida_handle = unsafe { OpenFileMappingA(FILE_MAP_READ, 0, aida_name.as_ptr()) };
        if !aida_handle.is_null() {
            let aida_map = unsafe { MapViewOfFile(aida_handle, FILE_MAP_READ, 0, 0, 262_144) };
            if aida_map.Value.is_null() {
                unsafe { CloseHandle(aida_handle) };
            } else {
                self.aida_handle = aida_handle;
                self.aida_ptr = aida_map.Value.cast::<u8>();
                self.is_open = true;
                info!("Attached to AIDA64/HWiNFO sensor stream");
            }
        }

        if !self.tried_autostart {
            self.tried_autostart = true;
            try_spawn_background_helper();
        }

        self.is_open
    }

    /// Reads real-time hardware telemetry across all active shared memory sources.
    #[allow(
        clippy::indexing_slicing,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::as_conversions,
        clippy::useless_let_if_seq
    )]
    pub fn sample(&mut self) -> WindowsSensorValues {
        if !self.try_open() {
            return WindowsSensorValues::default();
        }

        self.sample_count = self.sample_count.saturating_add(1);
        let mut values = if self.ldgt_ptr.is_null() {
            WindowsSensorValues::default()
        } else {
            let slice = unsafe { std::slice::from_raw_parts(self.ldgt_ptr, 2_097_152) };
            let text = String::from_utf8_lossy(slice);
            parse_ldgt_json_telemetry(&text)
        };

        // If LDGT is missing any values, supplement from AIDA64/LibreHardwareMonitor XML
        if (values.cpu_temp.is_none() || values.cpu_power.is_none()) && !self.aida_ptr.is_null() {
            let slice = unsafe { std::slice::from_raw_parts(self.aida_ptr, 262_144) };
            let text = String::from_utf8_lossy(slice);
            let aida_vals = parse_aida64_xml_telemetry(&text);
            if values.cpu_temp.is_none() {
                values.cpu_temp = aida_vals.cpu_temp;
            }
            if values.cpu_power.is_none() {
                values.cpu_power = aida_vals.cpu_power;
            }
            if values.cpu_clock.is_none() {
                values.cpu_clock = aida_vals.cpu_clock;
            }
        }

        if values.cpu_temp.is_some() || values.cpu_power.is_some() {
            if self.sample_count % 10 == 1 {
                info!(
                    "Hardware Sensor Stream Active -> CPU Temp: {:?}°C, Power: {:?}W, Clock: {:?}MHz",
                    values.cpu_temp, values.cpu_power, values.cpu_clock
                );
            }
        } else if self.sample_count % 5 == 1 {
            debug!("Waiting for hardware sensor engine to populate telemetry memory...");
        }

        values
    }

    pub fn close(&mut self) {
        if !self.ldgt_ptr.is_null() {
            unsafe {
                let _ = UnmapViewOfFile(
                    windows_sys::Win32::System::Memory::MEMORY_MAPPED_VIEW_ADDRESS {
                        Value: self.ldgt_ptr.cast_mut().cast::<c_void>(),
                    },
                );
            }
            self.ldgt_ptr = null();
        }
        if !self.ldgt_handle.is_null() {
            unsafe {
                let _ = CloseHandle(self.ldgt_handle);
            }
            self.ldgt_handle = std::ptr::null_mut();
        }

        if !self.aida_ptr.is_null() {
            unsafe {
                let _ = UnmapViewOfFile(
                    windows_sys::Win32::System::Memory::MEMORY_MAPPED_VIEW_ADDRESS {
                        Value: self.aida_ptr.cast_mut().cast::<c_void>(),
                    },
                );
            }
            self.aida_ptr = null();
        }
        if !self.aida_handle.is_null() {
            unsafe {
                let _ = CloseHandle(self.aida_handle);
            }
            self.aida_handle = std::ptr::null_mut();
        }

        self.is_open = false;
    }
}

#[cfg(target_os = "windows")]
impl Drop for WindowsSharedMemoryReader {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(target_os = "windows")]
fn try_spawn_background_helper() {
    let search_paths = [
        Path::new("C:\\Program Files\\Segotep DigitalCAP\\LDGT.exe"),
        Path::new("C:\\Program Files (x86)\\Segotep DigitalCAP\\LDGT.exe"),
        Path::new(".\\LDGT.exe"),
    ];

    for path in search_paths {
        if path.exists() {
            info!("Launching sensor engine helper from {}", path.display());
            let parent_dir = path.parent().unwrap_or_else(|| Path::new("."));
            let _ = Command::new(path).current_dir(parent_dir).spawn();
            return;
        }
    }
    warn!(
        "No local LDGT sensor engine found; listening for AIDA64 / HWiNFO / LibreHardwareMonitor shared memory"
    );
}

/// Parses AIDA64 / `HWiNFO` XML shared memory format (`<temp><id>TCPU</id><val>48</val></temp>`).
#[allow(
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::as_conversions
)]
fn parse_aida64_xml_telemetry(text: &str) -> WindowsSensorValues {
    let mut values = WindowsSensorValues::default();

    for block in text
        .split("</temp>")
        .chain(text.split("</pwr>"))
        .chain(text.split("</clk>"))
    {
        let lower = block.to_lowercase();
        if (lower.contains("<id>tcpu") || lower.contains("<id>tdie") || lower.contains("<id>tctl"))
            && let Some(val) = extract_xml_val(block)
        {
            values.cpu_temp = Some(val.round().clamp(0.0, 255.0) as u8);
        }
        if (lower.contains("<id>pcpu") || lower.contains("<id>ppkg") || lower.contains("package"))
            && let Some(val) = extract_xml_val(block)
        {
            values.cpu_power = Some(val.round().clamp(0.0, 65535.0) as u16);
        }
        if (lower.contains("<id>ccpu") || lower.contains("<id>core0"))
            && let Some(val) = extract_xml_val(block)
            && val >= 100.0
        {
            values.cpu_clock = Some(val.round().clamp(0.0, 65535.0) as u16);
        }
    }

    values
}

fn extract_xml_val(block: &str) -> Option<f64> {
    let start = block.find("<val>")?.saturating_add(5);
    let end = block.get(start..)?.find("</val>")?;
    block
        .get(start..start.saturating_add(end))?
        .trim()
        .parse::<f64>()
        .ok()
}

/// Parses the JSON portion of the LDGT memory buffer.
#[allow(
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::as_conversions
)]
fn parse_ldgt_json_telemetry(text: &str) -> WindowsSensorValues {
    let Some(first_brace) = text.find('{') else {
        return WindowsSensorValues::default();
    };

    let Some(last_brace) = text.rfind('}') else {
        return WindowsSensorValues::default();
    };

    if last_brace <= first_brace {
        return WindowsSensorValues::default();
    }

    let json_block = text.get(first_brace..=last_brace).unwrap_or("");
    parse_json_str(json_block)
}

#[allow(
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::as_conversions,
    clippy::similar_names
)]
fn parse_json_str(json_str: &str) -> WindowsSensorValues {
    let mut values = WindowsSensorValues::default();

    for sensor_chunk in json_str.split("},\"").flat_map(|s| s.split("],\"")) {
        let lower = sensor_chunk.to_lowercase();

        // CPU Temperature: prioritize Tctl/Tdie, package temp, core temp
        let matches_cpu_temp = (lower.contains("tctl")
            || lower.contains("tdie")
            || lower.contains("cpu package")
            || lower.contains("cpu temperature")
            || lower.contains("core temp"))
            && (lower.contains("temperature") || lower.contains("temp"));

        if matches_cpu_temp && let Some(val) = extract_json_value(sensor_chunk) {
            let current = values.cpu_temp.unwrap_or(0);
            let val_u8 = (val.round().clamp(0.0, 255.0)) as u8;
            if val_u8 > current {
                values.cpu_temp = Some(val_u8);
            }
        }

        // CPU Package Power: look for package power or total power in Watts
        let matches_cpu_pwr = (lower.contains("cpu package power")
            || lower.contains("package power")
            || lower.contains("core power")
            || lower.contains("cpu power"))
            && lower.contains("power");

        if matches_cpu_pwr && let Some(val) = extract_json_value(sensor_chunk) {
            let current = values.cpu_power.unwrap_or(0);
            let val_u16 = (val.round().clamp(0.0, 65535.0)) as u16;
            if val_u16 > current {
                values.cpu_power = Some(val_u16);
            }
        }

        // CPU Clock: look for core clock or perf clock with realistic frequency (> 100 MHz)
        let matches_cpu_clk = (lower.contains("cpu clock")
            || lower.contains("core clock")
            || lower.contains("clock (perf"))
            && lower.contains("clock");

        if matches_cpu_clk && let Some(val) = extract_json_value(sensor_chunk) {
            let val_u16 = (val.round().clamp(0.0, 65535.0)) as u16;
            if val_u16 >= 100 {
                let current = values.cpu_clock.unwrap_or(0);
                if val_u16 > current {
                    values.cpu_clock = Some(val_u16);
                }
            }
        }

        // GPU Temperature
        let matches_gpu_temp = (lower.contains("gpu temperature") || lower.contains("gpu core"))
            && lower.contains("temperature");

        if matches_gpu_temp && let Some(val) = extract_json_value(sensor_chunk) {
            values.gpu_temp = Some((val.round().clamp(0.0, 255.0)) as u8);
        }

        // GPU Power
        let matches_gpu_pwr = (lower.contains("gpu power") || lower.contains("gpu core power"))
            && lower.contains("power");

        if matches_gpu_pwr && let Some(val) = extract_json_value(sensor_chunk) {
            values.gpu_power = Some((val.round().clamp(0.0, 65535.0)) as u16);
        }

        // GPU Clock
        let matches_gpu_clk = (lower.contains("gpu clock") || lower.contains("gpu memory clock"))
            && lower.contains("clock");

        if matches_gpu_clk && let Some(val) = extract_json_value(sensor_chunk) {
            values.gpu_clock = Some((val.round().clamp(0.0, 65535.0)) as u16);
        }
    }

    values
}

fn extract_json_value(segment: &str) -> Option<f64> {
    // Looks for `"value":"48.5"` or `"value":48.5`
    let idx = segment.find("\"value\":")?;
    let rest = segment
        .get(idx.saturating_add(8)..)?
        .trim_start_matches('"');
    let end_idx = rest.find(['"', '}', ',', ']'])?;
    let num_str = rest.get(..end_idx)?;
    num_str.trim().parse::<f64>().ok()
}
