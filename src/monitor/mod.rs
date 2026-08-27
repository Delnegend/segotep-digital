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
            win_sensors: WindowsSharedMemoryReader::new(),
        }
    }

    /// Samples all telemetry values from hardware or shared memory.
    pub fn sample(&mut self) -> HardwareMetrics {
        #[cfg(target_os = "windows")]
        let win_vals = self.win_sensors.sample();

        let mut cpu_temp = self.temp.get_temp();
        let cpu_load = self.load.get_load_pct();
        let mut cpu_power_watts = self.power.get_power_watts();
        let mut cpu_clock_mhz = self.freq.get_freq_mhz();

        #[allow(unused_mut)]
        let mut gpu_temp = 0;
        #[allow(unused_mut)]
        let mut gpu_load = 0;
        #[allow(unused_mut)]
        let mut gpu_power_watts = 0;
        #[allow(unused_mut)]
        let mut gpu_clock_mhz = 0;

        #[cfg(target_os = "windows")]
        {
            if let Some(t) = win_vals.cpu_temp {
                cpu_temp = t;
            }
            if let Some(p) = win_vals.cpu_power {
                cpu_power_watts = p;
            }
            if let Some(c) = win_vals.cpu_clock {
                cpu_clock_mhz = c;
            }
            if let Some(gt) = win_vals.gpu_temp {
                gpu_temp = gt;
            }
            if let Some(gp) = win_vals.gpu_power {
                gpu_power_watts = gp;
            }
            if let Some(gc) = win_vals.gpu_clock {
                gpu_clock_mhz = gc;
            }
        }

        HardwareMetrics {
            cpu_temp,
            cpu_load,
            cpu_power_watts,
            cpu_clock_mhz,
            gpu_temp,
            gpu_load,
            gpu_power_watts,
            gpu_clock_mhz,
        }
    }
}
