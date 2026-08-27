//! Windows shared memory sensor provider (Segotep LDGT, `HWiNFO`, `AIDA64`).

#[cfg(target_os = "windows")]
use std::ffi::c_void;
#[cfg(target_os = "windows")]
use std::path::Path;
#[cfg(target_os = "windows")]
use std::process::Command;
#[cfg(target_os = "windows")]
use std::ptr::null;
#[cfg(target_os = "windows")]
use tracing::{info, warn};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Memory::{
    CreateFileMappingA, FILE_MAP_ALL_ACCESS, MapViewOfFile, PAGE_READWRITE, UnmapViewOfFile,
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
    handle: HANDLE,
    buffer_ptr: *const u8,
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
            handle: std::ptr::null_mut(),
            buffer_ptr: std::ptr::null(),
            is_open: false,
            tried_autostart: false,
            sample_count: 0,
        }
    }

    /// Creates or opens `shareMemory_LDGTInfo` memory-mapped file (2MB buffer required by LDGT).
    #[allow(clippy::as_conversions)]
    pub fn try_open(&mut self) -> bool {
        if self.is_open && !self.buffer_ptr.is_null() {
            return true;
        }

        self.close();

        let name = b"shareMemory_LDGTInfo\0";
        let handle = unsafe {
            CreateFileMappingA(
                INVALID_HANDLE_VALUE,
                null(),
                PAGE_READWRITE,
                0,
                2_097_152,
                name.as_ptr(),
            )
        };

        if handle.is_null() {
            warn!("Failed to create/open shared memory bank");
            return false;
        }

        let map_ptr = unsafe { MapViewOfFile(handle, FILE_MAP_ALL_ACCESS, 0, 0, 2_097_152) };

        if map_ptr.Value.is_null() {
            unsafe { CloseHandle(handle) };
            return false;
        }

        self.handle = handle;
        self.buffer_ptr = map_ptr.Value.cast::<u8>();
        self.is_open = true;
        info!("Initialized 2MB Windows LDGT shared memory sensor bank");

        if !self.tried_autostart {
            self.tried_autostart = true;
            try_spawn_background_helper();
        }

        true
    }

    /// Reads real-time hardware telemetry from shared memory.
    #[allow(
        clippy::indexing_slicing,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::as_conversions
    )]
    pub fn sample(&mut self) -> WindowsSensorValues {
        if !self.try_open() {
            return WindowsSensorValues::default();
        }

        self.sample_count = self.sample_count.saturating_add(1);

        // Read the entire 2MB active view across both bank 0 and bank 1
        let slice = unsafe { std::slice::from_raw_parts(self.buffer_ptr, 2_097_152) };
        let text = String::from_utf8_lossy(slice);

        let parsed = parse_ldgt_json_telemetry(&text);
        if parsed.cpu_temp.is_some() || parsed.cpu_power.is_some() {
            if self.sample_count % 10 == 1 {
                info!(
                    "Hardware Sensor Stream Active -> CPU Temp: {:?}°C, Power: {:?}W, Clock: {:?}MHz",
                    parsed.cpu_temp, parsed.cpu_power, parsed.cpu_clock
                );
            }
        } else if self.sample_count % 5 == 1 {
            let non_zero_count = slice.iter().filter(|&&b| b != 0).count();
            info!(
                "Waiting for LDGT hardware sensor engine (mapped 2MB buffer, {} non-zero bytes)...",
                non_zero_count
            );
        }

        parsed
    }

    pub fn close(&mut self) {
        if !self.buffer_ptr.is_null() {
            unsafe {
                let _ = UnmapViewOfFile(
                    windows_sys::Win32::System::Memory::MEMORY_MAPPED_VIEW_ADDRESS {
                        Value: self.buffer_ptr.cast_mut().cast::<c_void>(),
                    },
                );
            }
            self.buffer_ptr = null();
        }
        if !self.handle.is_null() {
            unsafe {
                let _ = CloseHandle(self.handle);
            }
            self.handle = std::ptr::null_mut();
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
    let standard_path = Path::new("C:\\Program Files\\Segotep DigitalCAP\\LDGT.exe");
    if standard_path.exists() {
        info!(
            "Launching Segotep LDGT sensor engine from {}",
            standard_path.display()
        );
        let fallback_dir = Path::new("C:\\Program Files\\Segotep DigitalCAP");
        let _ = Command::new(standard_path)
            .current_dir(standard_path.parent().unwrap_or(fallback_dir))
            .spawn();
    }
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
