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
    temp_mon: CpuTempMonitor,
    load_mon: CpuLoadMonitor,
    power_mon: CpuPowerMonitor,
    freq_mon: CpuFreqMonitor,
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

impl SystemTelemetry {
    pub fn new() -> Self {
        Self {
            temp_mon: CpuTempMonitor::new(),
            load_mon: CpuLoadMonitor::new(),
            power_mon: CpuPowerMonitor::new(),
            freq_mon: CpuFreqMonitor::new(),
        }
    }

    /// Samples all telemetry values.
    pub fn sample(&mut self) -> HardwareMetrics {
        let cpu_temp = self.temp_mon.get_temp();
        let cpu_load = self.load_mon.get_load_pct();
        let cpu_power_watts = self.power_mon.get_power_watts();
        let cpu_clock_mhz = self.freq_mon.get_freq_mhz();

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
