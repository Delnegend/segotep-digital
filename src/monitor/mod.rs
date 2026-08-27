//! Hardware telemetry collection for Linux and Windows.

pub mod cpu_freq;
pub mod cpu_load;
pub mod cpu_power;
pub mod cpu_temp;
#[cfg(target_os = "windows")]
pub mod windows_sensors;

use cpu_freq::CpuFreqMonitor;
use cpu_load::CpuLoadMonitor;
use cpu_power::CpuPowerMonitor;
use cpu_temp::CpuTempMonitor;
#[cfg(target_os = "windows")]
pub use windows_sensors::WindowsSensorSource;
#[cfg(target_os = "windows")]
use windows_sensors::WindowsSharedMemoryReader;

pub struct SystemTelemetry {
    temp: CpuTempMonitor,
    load: CpuLoadMonitor,
    power: CpuPowerMonitor,
    freq: CpuFreqMonitor,
    #[cfg(target_os = "windows")]
    win_sensors: WindowsSharedMemoryReader,
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
            win_sensors: WindowsSharedMemoryReader::new(WindowsSensorSource::Auto),
        }
    }

    #[cfg(target_os = "windows")]
    pub fn set_windows_sensor_source(&mut self, source: WindowsSensorSource) {
        self.win_sensors.set_source(source);
    }

    /// Samples all telemetry values from hardware or shared memory.
    pub fn sample(&mut self) -> HardwareMetrics {
        #[cfg(target_os = "windows")]
        let win_vals = self.win_sensors.sample();

        let base_temp = self.temp.get_temp();
        let base_load = self.load.get_load_pct();
        let base_power = self.power.get_power_watts();
        let base_freq = self.freq.get_freq_mhz();

        #[cfg(target_os = "windows")]
        {
            HardwareMetrics {
                cpu_temp: win_vals.cpu_temp.unwrap_or(base_temp),
                cpu_load: base_load,
                cpu_power_watts: win_vals.cpu_power.unwrap_or(base_power),
                cpu_clock_mhz: win_vals.cpu_clock.unwrap_or(base_freq),
                gpu_temp: win_vals.gpu_temp.unwrap_or(0),
                gpu_load: 0,
                gpu_power_watts: win_vals.gpu_power.unwrap_or(0),
                gpu_clock_mhz: win_vals.gpu_clock.unwrap_or(0),
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
