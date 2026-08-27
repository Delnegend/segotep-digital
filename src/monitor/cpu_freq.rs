//! CPU Frequency monitor in MHz.

use std::fs;
use sysinfo::System;

pub struct CpuFreqMonitor {
    sys: System,
}

impl CpuFreqMonitor {
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_cpu_frequency();
        Self { sys }
    }

    /// Fetches the primary/average CPU clock speed in MHz.
    pub fn get_freq_mhz(&mut self) -> u16 {
        // Direct read from Linux cpufreq if available (scaling_cur_freq is in kHz)
        if let Ok(content) =
            fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq")
        {
            if let Ok(khz) = content.trim().parse::<u32>() {
                return (khz / 1000).clamp(0, 65535) as u16;
            }
        }

        // Fallback to sysinfo
        self.sys.refresh_cpu_frequency();
        let cpus = self.sys.cpus();
        if !cpus.is_empty() {
            let avg_mhz: u64 =
                cpus.iter().map(|c| c.frequency()).sum::<u64>() / (cpus.len() as u64);
            return avg_mhz.clamp(0, 65535) as u16;
        }

        0
    }
}
