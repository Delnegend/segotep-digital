//! CPU Frequency monitor in MHz for Linux and Windows.

#[cfg(target_os = "linux")]
use std::fs;
use sysinfo::System;

pub struct CpuFreqMonitor {
    sys: System,
}

impl Default for CpuFreqMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl CpuFreqMonitor {
    #[must_use]
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_cpu_frequency();
        Self { sys }
    }

    /// Fetches the primary/average CPU clock speed in MHz.
    pub fn get_freq_mhz(&mut self) -> u16 {
        #[cfg(target_os = "linux")]
        {
            // Direct read from Linux cpufreq if available (scaling_cur_freq is in kHz)
            if let Ok(content) =
                fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq")
                && let Ok(khz) = content.trim().parse::<u32>()
            {
                return u16::try_from(khz / 1000).unwrap_or(u16::MAX);
            }
        }

        // Cross-platform fallback via sysinfo (works on Windows & Linux)
        self.sys.refresh_cpu_frequency();
        let cpus = self.sys.cpus();
        let total = cpus
            .iter()
            .fold(0_u64, |sum, cpu| sum.saturating_add(cpu.frequency()));
        let count = u64::try_from(cpus.len()).unwrap_or(u64::MAX);
        if let Some(avg_mhz) = total.checked_div(count) {
            return u16::try_from(avg_mhz).unwrap_or(u16::MAX);
        }

        0
    }
}
