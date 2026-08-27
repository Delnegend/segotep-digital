//! CPU package power monitor for Linux and Windows.

#[cfg(target_os = "linux")]
use log::debug;
#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "linux")]
use std::path::{Path, PathBuf};
#[cfg(target_os = "linux")]
use std::time::Instant;

pub struct CpuPowerMonitor {
    #[cfg(target_os = "linux")]
    energy_path: Option<PathBuf>,
    #[cfg(target_os = "linux")]
    max_energy_range_uj: Option<u64>,
    #[cfg(target_os = "linux")]
    last_energy_uj: u64,
    #[cfg(target_os = "linux")]
    last_timestamp: Instant,
    last_calculated_watts: u16,
}

impl Default for CpuPowerMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl CpuPowerMonitor {
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn new() -> Self {
        #[cfg(target_os = "linux")]
        let (energy_path, max_range) = find_energy_source();

        #[cfg(target_os = "linux")]
        let initial_energy = energy_path
            .as_ref()
            .map_or(0, |path| read_u64_from_file(path).unwrap_or(0));

        Self {
            #[cfg(target_os = "linux")]
            energy_path,
            #[cfg(target_os = "linux")]
            max_energy_range_uj: max_range,
            #[cfg(target_os = "linux")]
            last_energy_uj: initial_energy,
            #[cfg(target_os = "linux")]
            last_timestamp: Instant::now(),
            last_calculated_watts: 0,
        }
    }

    /// Calculates CPU power in Watts based on the energy counter delta.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::as_conversions,
        clippy::arithmetic_side_effects,
        clippy::cast_precision_loss,
        clippy::missing_const_for_fn
    )]
    pub fn get_power_watts(&mut self) -> u16 {
        #[cfg(target_os = "linux")]
        {
            let Some(ref path) = self.energy_path else {
                return 0;
            };

            let now = Instant::now();
            let elapsed = now.duration_since(self.last_timestamp).as_secs_f64();
            if elapsed < 0.1 {
                return self.last_calculated_watts;
            }

            if let Some(current_energy_uj) = read_u64_from_file(path) {
                let delta_uj = if current_energy_uj >= self.last_energy_uj {
                    current_energy_uj - self.last_energy_uj
                } else if let Some(max_range) = self.max_energy_range_uj {
                    // Counter wrapped around
                    (max_range - self.last_energy_uj) + current_energy_uj
                } else {
                    0
                };

                // Energy is in microjoules (uJ). Watts = (delta uJ / 1,000,000) / seconds
                let watts = (delta_uj as f64 / 1_000_000.0) / elapsed;
                let watts_clamped = watts.round().clamp(0.0, 65535.0) as u16;

                self.last_energy_uj = current_energy_uj;
                self.last_timestamp = now;
                self.last_calculated_watts = watts_clamped;

                watts_clamped
            } else {
                self.last_calculated_watts
            }
        }

        #[cfg(target_os = "windows")]
        {
            // On Windows without kernel driver, return last sampled or 0
            self.last_calculated_watts
        }
    }
}

#[cfg(target_os = "linux")]
fn read_u64_from_file(path: &Path) -> Option<u64> {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
}

/// Discovers RAPL powercap energy file or hwmon energy/power inputs.
#[cfg(target_os = "linux")]
fn find_energy_source() -> (Option<PathBuf>, Option<u64>) {
    // 1. Check /sys/class/powercap/intel-rapl/intel-rapl:0/energy_uj
    let rapl_package = Path::new("/sys/class/powercap/intel-rapl/intel-rapl:0/energy_uj");
    if rapl_package.exists() {
        let max_range = read_u64_from_file(Path::new(
            "/sys/class/powercap/intel-rapl/intel-rapl:0/max_energy_range_uj",
        ));
        debug!(
            "Found Intel/AMD RAPL energy file: {}",
            rapl_package.display()
        );
        return (Some(rapl_package.to_path_buf()), max_range);
    }

    // 2. Check /sys/class/hwmon for amd_energy or similar power1_input
    let hwmon_dir = Path::new("/sys/class/hwmon");
    if hwmon_dir.exists()
        && let Ok(entries) = fs::read_dir(hwmon_dir)
    {
        for entry in entries.flatten() {
            let path = entry.path();
            let energy_node = path.join("energy1_input");
            if energy_node.exists() {
                debug!("Found hwmon energy node: {}", energy_node.display());
                return (Some(energy_node), None);
            }
        }
    }

    (None, None)
}
