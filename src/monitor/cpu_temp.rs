//! Linux CPU temperature monitor using /sys/class/hwmon and sysinfo fallback.

use log::debug;
use std::fs;
use std::path::{Path, PathBuf};
use sysinfo::Components;

pub struct CpuTempMonitor {
    direct_sensor_path: Option<PathBuf>,
    components: Components,
}

impl Default for CpuTempMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl CpuTempMonitor {
    #[must_use]
    pub fn new() -> Self {
        let direct_sensor_path = find_hwmon_cpu_temp();
        let mut components = Components::new();
        components.refresh(true);

        Self {
            direct_sensor_path,
            components,
        }
    }

    /// Fetches the current CPU temperature in whole degrees Celsius.
    #[allow(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        clippy::as_conversions,
        clippy::arithmetic_side_effects
    )]
    pub fn get_temp(&mut self) -> u8 {
        // Method 1: Direct sysfs hwmon read (fastest, zero overhead)
        if let Some(ref path) = self.direct_sensor_path
            && let Ok(content) = fs::read_to_string(path)
            && let Ok(milli_c) = content.trim().parse::<i32>()
        {
            let temp = (milli_c / 1000).clamp(0, 255) as u8;
            return temp;
        }

        // Method 2: Fallback to sysinfo components
        self.components.refresh(false);
        let mut best_temp: f32 = 0.0;

        for component in &self.components {
            let label = component.label().to_lowercase();
            let is_cpu = label.contains("cpu")
                || label.contains("tctl")
                || label.contains("tdie")
                || label.contains("package")
                || label.contains("core");

            if is_cpu
                && let Some(temp) = component.temperature()
                && temp > best_temp
            {
                best_temp = temp;
            }
        }

        if best_temp > 0.0 {
            best_temp.round().clamp(0.0, 255.0) as u8
        } else {
            0
        }
    }
}

/// Searches `/sys/class/hwmon/` for known CPU temperature sensor nodes.
#[allow(clippy::indexing_slicing)]
fn find_hwmon_cpu_temp() -> Option<PathBuf> {
    let hwmon_dir = Path::new("/sys/class/hwmon");
    if !hwmon_dir.exists() {
        return None;
    }

    if let Ok(entries) = fs::read_dir(hwmon_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name_file = path.join("name");

            if let Ok(name) = fs::read_to_string(name_file) {
                let name = name.trim().to_lowercase();
                // Check for common CPU hwmon driver names
                if name.contains("k10temp")
                    || name.contains("coretemp")
                    || name.contains("zenpower")
                    || name.contains("cpu_thermal")
                {
                    // Look for temp1_input, temp2_input (Tctl/Tdie)
                    for i in 1..=8 {
                        let temp_file = path.join(format!("temp{i}_input"));
                        let label_file = path.join(format!("temp{i}_label"));

                        if temp_file.exists() {
                            if let Ok(label) = fs::read_to_string(&label_file) {
                                let label = label.trim().to_lowercase();
                                if label.contains("tctl")
                                    || label.contains("tdie")
                                    || label.contains("package")
                                {
                                    debug!(
                                        "Found CPU temp sensor: {} ({label})",
                                        temp_file.display()
                                    );
                                    return Some(temp_file);
                                }
                            }
                            // If no specific label, take the first temp input
                            if i == 1 {
                                debug!("Found CPU temp sensor: {}", temp_file.display());
                                return Some(temp_file);
                            }
                        }
                    }
                }
            }
        }
    }

    None
}
