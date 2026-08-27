//! CPU utilization calculation via /proc/stat or sysinfo.

use std::fs;
use sysinfo::System;

pub struct CpuLoadMonitor {
    sys: System,
    last_idle: u64,
    last_total: u64,
    has_proc_stat: bool,
}

impl CpuLoadMonitor {
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_cpu_usage();

        let (idle, total, ok) = read_proc_stat_cpu();

        Self {
            sys,
            last_idle: idle,
            last_total: total,
            has_proc_stat: ok,
        }
    }

    /// Fetches overall CPU utilization percentage (0 - 100).
    pub fn get_load_pct(&mut self) -> u8 {
        if self.has_proc_stat {
            let (idle, total, ok) = read_proc_stat_cpu();
            if ok && total > self.last_total {
                let total_diff = (total - self.last_total) as f64;
                let idle_diff = (idle - self.last_idle) as f64;

                self.last_idle = idle;
                self.last_total = total;

                let usage = ((total_diff - idle_diff) / total_diff) * 100.0;
                return usage.round().clamp(0.0, 100.0) as u8;
            }
        }

        // Fallback to sysinfo
        self.sys.refresh_cpu_usage();
        self.sys.global_cpu_usage().round().clamp(0.0, 100.0) as u8
    }
}

fn read_proc_stat_cpu() -> (u64, u64, bool) {
    if let Ok(content) = fs::read_to_string("/proc/stat") {
        for line in content.lines() {
            if line.starts_with("cpu ") {
                let parts: Vec<u64> = line
                    .split_whitespace()
                    .skip(1)
                    .filter_map(|s| s.parse::<u64>().ok())
                    .collect();

                if parts.len() >= 4 {
                    // user + nice + system + idle + iowait + irq + softirq + steal
                    let idle = parts[3] + parts.get(4).copied().unwrap_or(0);
                    let total: u64 = parts.iter().sum();
                    return (idle, total, true);
                }
            }
        }
    }
    (0, 0, false)
}
