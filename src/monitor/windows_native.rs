//! Native Windows hardware sensor providers (PDH performance counters, NVML/NvAPI, NTDLL).

#[cfg(target_os = "windows")]
use std::ffi::c_void;
#[cfg(target_os = "windows")]
use tracing::debug;
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Performance::{
    PDH_FMT_DOUBLE, PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData,
    PdhGetFormattedCounterValue, PdhOpenQueryW,
};

#[repr(C)]
#[derive(Copy, Clone)]
#[allow(non_snake_case)]
struct PdhFmtCounterValueDouble {
    CStatus: u32,
    _padding: u32,
    doubleValue: f64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NativeHardwareSample {
    pub cpu_power_w: Option<u16>,
    pub cpu_clock_mhz: Option<u16>,
    pub cpu_load_pct: Option<f32>,
    pub gpu_temp_c: Option<u8>,
    pub gpu_power_w: Option<u16>,
    pub gpu_clock_mhz: Option<u16>,
    pub gpu_load_pct: Option<f32>,
}

#[cfg(target_os = "windows")]
pub struct WindowsPdhMonitor {
    query: isize,
    cpu_power_counter: isize,
    cpu_clock_counter: isize,
    cpu_util_counter: isize,
    has_collected_first_sample: bool,
}

#[cfg(target_os = "windows")]
impl Default for WindowsPdhMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "windows")]
impl WindowsPdhMonitor {
    #[must_use]
    pub fn new() -> Self {
        let mut monitor = Self {
            query: 0,
            cpu_power_counter: 0,
            cpu_clock_counter: 0,
            cpu_util_counter: 0,
            has_collected_first_sample: false,
        };
        monitor.init();
        monitor
    }

    fn init(&mut self) {
        unsafe {
            let mut query = 0;
            if PdhOpenQueryW(std::ptr::null(), 0, &raw mut query) != 0 {
                return;
            }
            self.query = query;

            // 1. CPU Package RAPL Power (milliwatts)
            let pwr_path: Vec<u16> = "\\Energy Meter(rapl_package0_pkg)\\Power\0"
                .encode_utf16()
                .collect();
            let mut pwr_counter = 0;
            let _ = PdhAddEnglishCounterW(query, pwr_path.as_ptr(), 0, &raw mut pwr_counter);
            self.cpu_power_counter = pwr_counter;

            // 2. CPU Actual Frequency (MHz)
            let clk_path: Vec<u16> = "\\Processor Information(_Total)\\Actual Frequency\0"
                .encode_utf16()
                .collect();
            let mut clk_counter = 0;
            let _ = PdhAddEnglishCounterW(query, clk_path.as_ptr(), 0, &raw mut clk_counter);
            self.cpu_clock_counter = clk_counter;

            // 3. CPU % Processor Utility
            let util_path: Vec<u16> = "\\Processor Information(_Total)\\% Processor Utility\0"
                .encode_utf16()
                .collect();
            let mut util_counter = 0;
            let _ = PdhAddEnglishCounterW(query, util_path.as_ptr(), 0, &raw mut util_counter);
            self.cpu_util_counter = util_counter;

            // Prime the initial query collection
            let _ = PdhCollectQueryData(query);
        }
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::as_conversions
    )]
    pub fn sample(&mut self) -> (Option<u16>, Option<u16>, Option<f32>) {
        if self.query == 0 {
            return (None, None, None);
        }

        unsafe {
            if PdhCollectQueryData(self.query) != 0 {
                return (None, None, None);
            }

            if !self.has_collected_first_sample {
                self.has_collected_first_sample = true;
                return (None, None, None);
            }

            let mut cpu_power = None;
            let mut cpu_clock = None;
            let mut cpu_load = None;

            if self.cpu_power_counter != 0 {
                let mut val: PdhFmtCounterValueDouble = std::mem::zeroed();
                if PdhGetFormattedCounterValue(
                    self.cpu_power_counter,
                    PDH_FMT_DOUBLE,
                    std::ptr::null_mut(),
                    (&raw mut val).cast(),
                ) == 0
                    && val.CStatus == 0
                {
                    let mw = val.doubleValue;
                    if mw > 0.0 {
                        // mW -> Watts
                        let watts = (mw / 1000.0).round().clamp(0.0, 65535.0) as u16;
                        cpu_power = Some(watts);
                    }
                }
            }

            if self.cpu_clock_counter != 0 {
                let mut val: PdhFmtCounterValueDouble = std::mem::zeroed();
                if PdhGetFormattedCounterValue(
                    self.cpu_clock_counter,
                    PDH_FMT_DOUBLE,
                    std::ptr::null_mut(),
                    (&raw mut val).cast(),
                ) == 0
                    && val.CStatus == 0
                {
                    let mhz = val.doubleValue;
                    if mhz > 100.0 {
                        cpu_clock = Some(mhz.round().clamp(0.0, 65535.0) as u16);
                    }
                }
            }

            if self.cpu_util_counter != 0 {
                let mut val: PdhFmtCounterValueDouble = std::mem::zeroed();
                if PdhGetFormattedCounterValue(
                    self.cpu_util_counter,
                    PDH_FMT_DOUBLE,
                    std::ptr::null_mut(),
                    (&raw mut val).cast(),
                ) == 0
                    && val.CStatus == 0
                {
                    let util = val.doubleValue;
                    if util >= 0.0 {
                        cpu_load = Some(util.clamp(0.0, 100.0) as f32);
                    }
                }
            }

            (cpu_power, cpu_clock, cpu_load)
        }
    }
}

#[cfg(target_os = "windows")]
impl Drop for WindowsPdhMonitor {
    fn drop(&mut self) {
        if self.query != 0 {
            unsafe {
                let _ = PdhCloseQuery(self.query);
            }
            self.query = 0;
        }
    }
}

// ---------------------------------------------------------------------------
// NVIDIA NVML Native User-space GPU Monitoring
// ---------------------------------------------------------------------------

type NvmlInitFn = unsafe extern "C" fn() -> i32;
type NvmlShutdownFn = unsafe extern "C" fn() -> i32;
type NvmlDeviceGetCountFn = unsafe extern "C" fn(*mut u32) -> i32;
type NvmlDeviceGetHandleByIndexFn = unsafe extern "C" fn(u32, *mut *mut c_void) -> i32;
type NvmlDeviceGetTemperatureFn = unsafe extern "C" fn(*mut c_void, u32, *mut u32) -> i32;
type NvmlDeviceGetPowerUsageFn = unsafe extern "C" fn(*mut c_void, *mut u32) -> i32;
type NvmlDeviceGetClockInfoFn = unsafe extern "C" fn(*mut c_void, u32, *mut u32) -> i32;

#[repr(C)]
struct NvmlUtilization {
    gpu: u32,
    memory: u32,
}
type NvmlDeviceGetUtilizationRatesFn =
    unsafe extern "C" fn(*mut c_void, *mut NvmlUtilization) -> i32;

pub struct WindowsNvmlGpuMonitor {
    device_handle: *mut c_void,
    get_temp: Option<NvmlDeviceGetTemperatureFn>,
    get_power: Option<NvmlDeviceGetPowerUsageFn>,
    get_clock: Option<NvmlDeviceGetClockInfoFn>,
    get_utilization: Option<NvmlDeviceGetUtilizationRatesFn>,
    shutdown_fn: Option<NvmlShutdownFn>,
}

impl Default for WindowsNvmlGpuMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowsNvmlGpuMonitor {
    #[must_use]
    #[allow(clippy::similar_names)]
    pub fn new() -> Self {
        #[cfg(target_os = "windows")]
        unsafe {
            let nvml_lib = LoadLibraryA(c"nvml.dll".as_ptr().cast());
            if nvml_lib.is_null() {
                return Self::empty();
            }

            let init_fn_ptr = GetProcAddress(nvml_lib, c"nvmlInit_v2".as_ptr().cast())
                .or_else(|| GetProcAddress(nvml_lib, c"nvmlInit".as_ptr().cast()));
            let Some(init_raw) = init_fn_ptr else {
                return Self::empty();
            };
            let init_fn: NvmlInitFn = std::mem::transmute(init_raw);
            if init_fn() != 0 {
                return Self::empty();
            }

            let count_fn_ptr = GetProcAddress(nvml_lib, c"nvmlDeviceGetCount_v2".as_ptr().cast())
                .or_else(|| GetProcAddress(nvml_lib, c"nvmlDeviceGetCount".as_ptr().cast()));
            let handle_fn_ptr =
                GetProcAddress(nvml_lib, c"nvmlDeviceGetHandleByIndex_v2".as_ptr().cast()).or_else(
                    || GetProcAddress(nvml_lib, c"nvmlDeviceGetHandleByIndex".as_ptr().cast()),
                );

            let Some(count_raw) = count_fn_ptr else {
                return Self::empty();
            };
            let Some(handle_raw) = handle_fn_ptr else {
                return Self::empty();
            };

            let count_fn: NvmlDeviceGetCountFn = std::mem::transmute(count_raw);
            let handle_fn: NvmlDeviceGetHandleByIndexFn = std::mem::transmute(handle_raw);

            let mut count = 0;
            if count_fn(&raw mut count) != 0 || count == 0 {
                return Self::empty();
            }

            let mut device_handle = std::ptr::null_mut();
            if handle_fn(0, &raw mut device_handle) != 0 || device_handle.is_null() {
                return Self::empty();
            }

            let get_temp: Option<NvmlDeviceGetTemperatureFn> =
                GetProcAddress(nvml_lib, c"nvmlDeviceGetTemperature".as_ptr().cast())
                    .map(|p| std::mem::transmute(p));
            let get_power: Option<NvmlDeviceGetPowerUsageFn> =
                GetProcAddress(nvml_lib, c"nvmlDeviceGetPowerUsage".as_ptr().cast())
                    .map(|p| std::mem::transmute(p));
            let get_clock: Option<NvmlDeviceGetClockInfoFn> =
                GetProcAddress(nvml_lib, c"nvmlDeviceGetClockInfo".as_ptr().cast())
                    .map(|p| std::mem::transmute(p));
            let get_utilization: Option<NvmlDeviceGetUtilizationRatesFn> =
                GetProcAddress(nvml_lib, c"nvmlDeviceGetUtilizationRates".as_ptr().cast())
                    .map(|p| std::mem::transmute(p));
            let shutdown_fn: Option<NvmlShutdownFn> =
                GetProcAddress(nvml_lib, c"nvmlShutdown".as_ptr().cast())
                    .map(|p| std::mem::transmute(p));

            debug!("Initialized native NVIDIA NVML GPU monitoring engine");

            Self {
                device_handle,
                get_temp,
                get_power,
                get_clock,
                get_utilization,
                shutdown_fn,
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            Self::empty()
        }
    }

    const fn empty() -> Self {
        Self {
            device_handle: std::ptr::null_mut(),
            get_temp: None,
            get_power: None,
            get_clock: None,
            get_utilization: None,
            shutdown_fn: None,
        }
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        clippy::as_conversions
    )]
    pub fn sample(&self) -> (Option<u8>, Option<u16>, Option<u16>, Option<f32>) {
        if self.device_handle.is_null() {
            return (None, None, None, None);
        }

        let mut temp = None;
        let mut power = None;
        let mut clock = None;
        let mut util = None;

        unsafe {
            if let Some(get_temp) = self.get_temp {
                let mut temp_val = 0;
                // Sensor type 0 = NVML_TEMPERATURE_GPU
                if get_temp(self.device_handle, 0, &raw mut temp_val) == 0 {
                    temp = Some(temp_val.clamp(0, 255) as u8);
                }
            }

            if let Some(get_power) = self.get_power {
                let mut mw_val = 0;
                if get_power(self.device_handle, &raw mut mw_val) == 0 && mw_val > 0 {
                    let w = ((mw_val as f64) / 1000.0).round().clamp(0.0, 65535.0) as u16;
                    power = Some(w);
                }
            }

            if let Some(get_clock) = self.get_clock {
                let mut clock_mhz = 0;
                // Clock type 0 = NVML_CLOCK_GRAPHICS
                if get_clock(self.device_handle, 0, &raw mut clock_mhz) == 0 && clock_mhz > 0 {
                    clock = Some(clock_mhz.clamp(0, 65535) as u16);
                }
            }

            if let Some(get_util) = self.get_utilization {
                let mut rates = NvmlUtilization { gpu: 0, memory: 0 };
                if get_util(self.device_handle, &raw mut rates) == 0 {
                    util = Some(rates.gpu.clamp(0, 100) as f32);
                }
            }
        }

        (temp, power, clock, util)
    }
}

impl Drop for WindowsNvmlGpuMonitor {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown_fn {
            unsafe {
                let _ = shutdown();
            }
        }
    }
}
