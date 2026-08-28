//! Hardware telemetry collection for Linux and Windows.

pub mod cpu_freq;
pub mod cpu_load;
pub mod cpu_power;
pub mod cpu_temp;
#[cfg(target_os = "windows")]
pub mod windows_native;
#[cfg(target_os = "windows")]
pub mod windows_pawnio;

use cpu_freq::CpuFreqMonitor;
use cpu_load::CpuLoadMonitor;
use cpu_power::CpuPowerMonitor;
use cpu_temp::CpuTempMonitor;
#[cfg(target_os = "windows")]
use windows_native::{WindowsNvmlGpuMonitor, WindowsPdhMonitor};
#[cfg(target_os = "windows")]
use windows_pawnio::WindowsPawnIoDriver;

pub struct SystemTelemetry {
    temp: CpuTempMonitor,
    load: CpuLoadMonitor,
    power: CpuPowerMonitor,
    freq: CpuFreqMonitor,
    #[cfg(target_os = "windows")]
    pdh_monitor: WindowsPdhMonitor,
    #[cfg(target_os = "windows")]
    nvml_monitor: WindowsNvmlGpuMonitor,
    #[cfg(target_os = "windows")]
    pawnio_driver: WindowsPawnIoDriver,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct HardwareMetrics {
    pub cpu_temp: u8,
    pub cpu_load: u8,
    pub cpu_power_watts: u16,
    pub cpu_clock_mhz: u16,
    pub gpu_temp: u8,
    pub gpu_load: u8,
    pub gpu_power_watts: u16,
    pub gpu_clock_mhz: u16,
}

impl Default for SystemTelemetry {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemTelemetry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            temp: CpuTempMonitor::new(),
            load: CpuLoadMonitor::new(),
            power: CpuPowerMonitor::new(),
            freq: CpuFreqMonitor::new(),
            #[cfg(target_os = "windows")]
            pdh_monitor: WindowsPdhMonitor::new(),
            #[cfg(target_os = "windows")]
            nvml_monitor: WindowsNvmlGpuMonitor::new(),
            #[cfg(target_os = "windows")]
            pawnio_driver: WindowsPawnIoDriver::new(),
        }
    }

    /// Samples all telemetry values from hardware, PDH counters, and NVML.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn sample(&mut self) -> HardwareMetrics {
        #[cfg(target_os = "windows")]
        let (pdh_pwr, pdh_clk, pdh_load) = self.pdh_monitor.sample();
        #[cfg(target_os = "windows")]
        let (nvml_temp, nvml_pwr, nvml_clk, nvml_load) = self.nvml_monitor.sample();
        #[cfg(target_os = "windows")]
        let pawn_temp = self.pawnio_driver.get_cpu_temp();

        let base_temp = self.temp.get_temp();
        let base_load = self.load.get_load_pct();
        let base_power = self.power.get_power_watts();
        let base_freq = self.freq.get_freq_mhz();

        #[cfg(target_os = "windows")]
        {
            let final_cpu_load = if let Some(l) = pdh_load {
                (l.round().clamp(0.0, 100.0)) as u8
            } else {
                base_load
            };

            let final_cpu_temp = pawn_temp.unwrap_or(base_temp);
            let final_cpu_power = pdh_pwr.unwrap_or(base_power);
            let final_cpu_clock = pdh_clk.unwrap_or(base_freq);
            let final_gpu_temp = nvml_temp.unwrap_or(0);
            let final_gpu_power = nvml_pwr.unwrap_or(0);
            let final_gpu_clock = nvml_clk.unwrap_or(0);
            let final_gpu_load = if let Some(l) = nvml_load {
                (l.round().clamp(0.0, 100.0)) as u8
            } else {
                0
            };

            HardwareMetrics {
                cpu_temp: final_cpu_temp,
                cpu_load: final_cpu_load,
                cpu_power_watts: final_cpu_power,
                cpu_clock_mhz: final_cpu_clock,
                gpu_temp: final_gpu_temp,
                gpu_load: final_gpu_load,
                gpu_power_watts: final_gpu_power,
                gpu_clock_mhz: final_gpu_clock,
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            HardwareMetrics {
                cpu_temp: base_temp,
                cpu_load: base_load,
                cpu_power_watts: base_power,
                cpu_clock_mhz: base_freq,
                gpu_temp: 0,
                gpu_load: 0,
                gpu_power_watts: 0,
                gpu_clock_mhz: 0,
            }
        }
    }
}
