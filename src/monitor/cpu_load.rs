//! CPU load percentage monitor using Linux /proc/stat with sysinfo fallback.

use std::fs;
use std::time::Instant;
use sysinfo::System;

pub struct CpuLoadMonitor {
    sys: System,
    last_stat: Option<(u64, u64)>, // (total_time, work_time)
    last_timestamp: Instant,
    last_pct: u8,
}

impl Default for CpuLoadMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl CpuLoadMonitor {
    #[must_use]
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_cpu_usage();

        let initial_stat = read_proc_stat_cpu();

        Self {
            sys,
            last_stat: initial_stat,
            last_timestamp: Instant::now(),
            last_pct: 0,
        }
    }

    /// Fetches the overall CPU utilization as a percentage (0-100).
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::as_conversions,
        clippy::arithmetic_side_effects,
        clippy::cast_precision_loss
    )]
    pub fn get_load_pct(&mut self) -> u8 {
        // Method 1: High precision /proc/stat delta calculation
        if let Some((prev_total, prev_work)) = self.last_stat {
            let now = Instant::now();
            if now.duration_since(self.last_timestamp).as_millis() >= 200 {
                if let Some((cur_total, cur_work)) = read_proc_stat_cpu() {
                    let total_delta = cur_total.saturating_sub(prev_total);
                    let work_delta = cur_work.saturating_sub(prev_work);

                    if total_delta > 0 {
                        let pct = ((work_delta as f64 / total_delta as f64) * 100.0)
                            .round()
                            .clamp(0.0, 100.0) as u8;

                        self.last_stat = Some((cur_total, cur_work));
                        self.last_timestamp = now;
                        self.last_pct = pct;
                        return pct;
                    }
                }
            } else {
                return self.last_pct;
            }
        }

        // Method 2: Fallback to sysinfo
        self.sys.refresh_cpu_usage();
        let global_usage = self.sys.global_cpu_usage();
        global_usage.round().clamp(0.0, 100.0) as u8
    }
}

/// Reads line 1 of `/proc/stat`: "cpu user nice system idle iowait irq softirq steal guest `guest_nice`"
#[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects)]
fn read_proc_stat_cpu() -> Option<(u64, u64)> {
    let content = fs::read_to_string("/proc/stat").ok()?;
    let first_line = content.lines().next()?;
    if !first_line.starts_with("cpu ") {
        return None;
    }

    let values: Vec<u64> = first_line
        .split_whitespace()
        .skip(1)
        .filter_map(|s| s.parse::<u64>().ok())
        .collect();

    if values.len() < 4 {
        return None;
    }

    // idle_time = idle (idx 3) + iowait (idx 4 if exists)
    let idle = values[3] + values.get(4).unwrap_or(&0);
    let total: u64 = values.iter().sum();
    let work = total.saturating_sub(idle);

    Some((total, work))
}
