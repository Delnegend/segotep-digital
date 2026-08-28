//! Windows direct kernel hardware sensor interface via the signed PawnIO driver.

#[cfg(target_os = "windows")]
use std::ffi::c_void;
#[cfg(target_os = "windows")]
use std::ptr::null_mut;
#[cfg(target_os = "windows")]
use tracing::info;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::{
    CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::IO::DeviceIoControl;

#[cfg(target_os = "windows")]
const DEVICE_TYPE: u32 = 41394 << 16;
#[cfg(target_os = "windows")]
const IOCTL_PIO_LOAD_BINARY: u32 = DEVICE_TYPE | (0x821 << 2);
#[cfg(target_os = "windows")]
const IOCTL_PIO_EXECUTE_FN: u32 = DEVICE_TYPE | (0x841 << 2);
#[cfg(target_os = "windows")]
const FN_NAME_LENGTH: usize = 32;

// Embedded signed PawnIO bytecode modules
#[cfg(target_os = "windows")]
static AMD_FAMILY17_BIN: &[u8] = include_bytes!("../resources/AMDFamily17.bin");
#[cfg(target_os = "windows")]
static INTEL_MSR_BIN: &[u8] = include_bytes!("../resources/IntelMSR.bin");

#[cfg(target_os = "windows")]
const F17H_M01H_THM_TCON_CUR_TMP: u32 = 0x0005_9800;
#[cfg(target_os = "windows")]
const F17H_TEMP_RANGE_SEL_MASK: u32 = 0x0008_0000;
#[cfg(target_os = "windows")]
const F17H_TEMP_TJ_SEL_MASK: u32 = 0x0003_0000;

#[cfg(target_os = "windows")]
pub struct WindowsPawnIoDriver {
    handle: HANDLE,
    is_amd: bool,
    is_intel: bool,
    initialized: bool,
}

#[cfg(target_os = "windows")]
impl Default for WindowsPawnIoDriver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "windows")]
impl WindowsPawnIoDriver {
    #[must_use]
    pub fn new() -> Self {
        let mut driver = Self {
            handle: INVALID_HANDLE_VALUE,
            is_amd: false,
            is_intel: false,
            initialized: false,
        };

        driver.try_connect();
        driver
    }

    fn try_connect(&mut self) -> bool {
        if self.handle != INVALID_HANDLE_VALUE {
            return true;
        }

        // Wide string for \\?\GLOBALROOT\Device\PawnIO
        let device_path: Vec<u16> = "\\\\?\\GLOBALROOT\\Device\\PawnIO"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        let handle = unsafe {
            CreateFileW(
                device_path.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                null_mut(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                null_mut(),
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            return false;
        }

        self.handle = handle;
        self.initialize_modules();
        true
    }

    #[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
    fn initialize_modules(&mut self) {
        if self.handle == INVALID_HANDLE_VALUE {
            return;
        }

        // Try loading AMD Family 17h/19h/1Ah module first
        let mut bytes_returned: u32 = 0;
        let success_amd = unsafe {
            DeviceIoControl(
                self.handle,
                IOCTL_PIO_LOAD_BINARY,
                AMD_FAMILY17_BIN.as_ptr().cast::<c_void>(),
                AMD_FAMILY17_BIN.len() as u32,
                null_mut(),
                0,
                &raw mut bytes_returned,
                null_mut(),
            )
        };

        if success_amd != 0 {
            self.is_amd = true;
            self.initialized = true;
            info!("Direct Hardware Driver: Loaded AMD Zen SMN telemetry module");
            return;
        }

        // If AMD module fails, try Intel MSR module
        let success_intel = unsafe {
            DeviceIoControl(
                self.handle,
                IOCTL_PIO_LOAD_BINARY,
                INTEL_MSR_BIN.as_ptr().cast::<c_void>(),
                INTEL_MSR_BIN.len() as u32,
                null_mut(),
                0,
                &raw mut bytes_returned,
                null_mut(),
            )
        };

        if success_intel != 0 {
            self.is_intel = true;
            self.initialized = true;
            info!("Direct Hardware Driver: Loaded Intel MSR telemetry module");
        }
    }

    #[allow(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::indexing_slicing
    )]
    pub fn read_smn(&self, address: u32) -> Option<u32> {
        if self.handle == INVALID_HANDLE_VALUE || !self.is_amd {
            return None;
        }

        let mut input_buf = [0u8; FN_NAME_LENGTH + size_of::<i64>()];
        let fn_name = b"ioctl_read_smn";
        input_buf[..fn_name.len()].copy_from_slice(fn_name);

        let addr_i64 = i64::from(address);
        input_buf[FN_NAME_LENGTH..].copy_from_slice(&addr_i64.to_ne_bytes());

        let mut output_buf = [0i64; 1];
        let mut bytes_returned: u32 = 0;

        let ok = unsafe {
            DeviceIoControl(
                self.handle,
                IOCTL_PIO_EXECUTE_FN,
                input_buf.as_ptr().cast::<c_void>(),
                input_buf.len() as u32,
                output_buf.as_mut_ptr().cast::<c_void>(),
                size_of::<[i64; 1]>() as u32,
                &raw mut bytes_returned,
                null_mut(),
            )
        };

        if ok != 0 && bytes_returned >= 8 {
            Some(output_buf[0] as u32)
        } else {
            None
        }
    }

    /// Reads direct CPU temperature in whole degrees Celsius.
    #[allow(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )]
    pub fn get_cpu_temp(&mut self) -> Option<u8> {
        if !self.try_connect() {
            return None;
        }

        if self.is_amd
            && let Some(raw) = self.read_smn(F17H_M01H_THM_TCON_CUR_TMP)
        {
            let temp_offset_flag = (raw & F17H_TEMP_RANGE_SEL_MASK) != 0
                || (raw & F17H_TEMP_TJ_SEL_MASK) == F17H_TEMP_TJ_SEL_MASK;

            let raw_temp = (raw >> 21) & 0x7FF;
            let mut temp = (raw_temp as f32) * 0.125;
            if temp_offset_flag {
                temp -= 49.0;
            }

            if (1.0..=125.0).contains(&temp) {
                return Some(temp.round().clamp(0.0, 255.0) as u8);
            }
        }

        None
    }
}

#[cfg(target_os = "windows")]
impl Drop for WindowsPawnIoDriver {
    fn drop(&mut self) {
        if self.handle != INVALID_HANDLE_VALUE {
            unsafe {
                let _ = CloseHandle(self.handle);
            }
            self.handle = INVALID_HANDLE_VALUE;
        }
    }
}
