//! Hardware telemetry collection for Linux.

pub mod cpu_freq;
pub mod cpu_load;
pub mod cpu_power;
pub mod cpu_temp;

use cpu_freq::CpuFreqMonitor;
use cpu_load::CpuLoadMonitor;
use cpu_power::CpuPowerMonitor;
use cpu_temp::CpuTempMonitor;

pub struct SystemTelemetry {
    temp: CpuTempMonitor,
    load: CpuLoadMonitor,
    power: CpuPowerMonitor,
    freq: CpuFreqMonitor,
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
        }
    }

    /// Samples all telemetry values.
    pub fn sample(&mut self) -> HardwareMetrics {
        let cpu_temp = self.temp.get_temp();
        let cpu_load = self.load.get_load_pct();
        let cpu_power_watts = self.power.get_power_watts();
        let cpu_clock_mhz = self.freq.get_freq_mhz();

        HardwareMetrics {
            cpu_temp,
            cpu_load,
            cpu_power_watts,
            cpu_clock_mhz,
            gpu_temp: 0,
            gpu_load: 0,
            gpu_power_watts: 0,
            gpu_clock_mhz: 0,
        }
    }
}
