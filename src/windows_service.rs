//! Windows background service manager and dispatcher.

#[cfg(target_os = "windows")]
use std::env;
#[cfg(target_os = "windows")]
use std::ffi::{c_ulong, c_void};
#[cfg(target_os = "windows")]
use std::ptr::{null, null_mut};
#[cfg(target_os = "windows")]
use std::sync::Mutex;
#[cfg(target_os = "windows")]
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
#[cfg(target_os = "windows")]
use std::thread::sleep;
#[cfg(target_os = "windows")]
use std::time::Duration;
#[cfg(target_os = "windows")]
use tracing::info;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::GetLastError;
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Services::{
    ChangeServiceConfigA, CloseServiceHandle, ControlService, CreateServiceA, DeleteService,
    OpenSCManagerA, OpenServiceA, QueryServiceStatus, RegisterServiceCtrlHandlerExA,
    SC_MANAGER_ALL_ACCESS, SC_MANAGER_CREATE_SERVICE, SERVICE_ALL_ACCESS, SERVICE_AUTO_START,
    SERVICE_CHANGE_CONFIG, SERVICE_CONTROL_SHUTDOWN, SERVICE_CONTROL_STOP, SERVICE_ERROR_NORMAL,
    SERVICE_NO_CHANGE, SERVICE_QUERY_STATUS, SERVICE_RUNNING, SERVICE_START, SERVICE_START_PENDING,
    SERVICE_STATUS, SERVICE_STATUS_HANDLE, SERVICE_STOP, SERVICE_STOP_PENDING, SERVICE_STOPPED,
    SERVICE_TABLE_ENTRYA, SERVICE_WIN32_OWN_PROCESS, SetServiceStatus, StartServiceA,
    StartServiceCtrlDispatcherA,
};

#[cfg(target_os = "windows")]
use crate::protocol::SegotepPacket;
#[cfg(target_os = "windows")]
use crate::{SegotepDevice, SystemTelemetry};

#[cfg(target_os = "windows")]
static SERVICE_NAME: &[u8] = b"SegotepDigitalService\0";
#[cfg(target_os = "windows")]
static SERVICE_DISPLAY_NAME: &[u8] = b"Segotep Digital Cooler Service\0";

#[cfg(target_os = "windows")]
static SERVICE_RUNNING_FLAG: AtomicBool = AtomicBool::new(true);
#[cfg(target_os = "windows")]
static SERVICE_STATUS_HANDLE_VAL: AtomicPtr<c_void> = AtomicPtr::new(null_mut());

#[derive(Clone, Copy)]
struct ServiceConfig {
    interval_ms: u64,
    model_id: u8,
    fahrenheit: bool,
    vid: u16,
    pid: u16,
}

static SERVICE_CONFIG: Mutex<Option<ServiceConfig>> = Mutex::new(None);

#[cfg(target_os = "windows")]
pub fn install_service(
    interval_ms: u64,
    model_id: u8,
    fahrenheit: bool,
    vid: u16,
    pid: u16,
) -> Result<(), String> {
    let current_exe = env::current_exe().map_err(|e| format!("Failed to get exe path: {e}"))?;
    let exe_path = current_exe.to_string_lossy();

    let mut bin_path = format!("\"{exe_path}\" --service -i {interval_ms} -m {model_id}");
    if fahrenheit {
        bin_path.push_str(" -f");
    }
    bin_path.push_str(&format!(" --vid {vid:04x} --pid {pid:04x}"));
    let bin_path_c = format!("{bin_path}\0");

    unsafe {
        let scm = OpenSCManagerA(
            null(),
            null(),
            SC_MANAGER_ALL_ACCESS | SC_MANAGER_CREATE_SERVICE,
        );
        if scm.is_null() {
            return Err(format!(
                "Failed to open Service Control Manager (error code {}). Please run as Administrator.",
                GetLastError()
            ));
        }

        let mut service = CreateServiceA(
            scm,
            SERVICE_NAME.as_ptr(),
            SERVICE_DISPLAY_NAME.as_ptr(),
            SERVICE_ALL_ACCESS,
            SERVICE_WIN32_OWN_PROCESS,
            SERVICE_AUTO_START,
            SERVICE_ERROR_NORMAL,
            bin_path_c.as_ptr(),
            null(),
            null_mut(),
            null(),
            null(),
            null(),
        );

        // If already exists, update configuration idempotently
        if service.is_null() {
            let err = GetLastError();
            // 1073 = ERROR_SERVICE_EXISTS
            if err == 1073 {
                info!("Service already exists. Updating service configuration...");
                service = OpenServiceA(
                    scm,
                    SERVICE_NAME.as_ptr(),
                    SERVICE_CHANGE_CONFIG | SERVICE_START | SERVICE_STOP | SERVICE_QUERY_STATUS,
                );
                if service.is_null() {
                    let _ = CloseServiceHandle(scm);
                    return Err(format!(
                        "Failed to open existing service for update (error code {}).",
                        GetLastError()
                    ));
                }

                let update_ok = ChangeServiceConfigA(
                    service,
                    SERVICE_NO_CHANGE,
                    SERVICE_AUTO_START,
                    SERVICE_NO_CHANGE,
                    bin_path_c.as_ptr(),
                    null(),
                    null_mut(),
                    null(),
                    null(),
                    null(),
                    SERVICE_DISPLAY_NAME.as_ptr(),
                );

                if update_ok == 0 {
                    let update_err = GetLastError();
                    let _ = CloseServiceHandle(service);
                    let _ = CloseServiceHandle(scm);
                    return Err(format!(
                        "Failed to update service config (error code {update_err})."
                    ));
                }
            } else {
                let _ = CloseServiceHandle(scm);
                return Err(format!(
                    "Failed to create Windows Service (error code {err})."
                ));
            }
        }

        // Check status and restart or start the service with new configuration
        let mut status: SERVICE_STATUS = std::mem::zeroed();
        if QueryServiceStatus(service, &raw mut status) != 0
            && status.dwCurrentState == SERVICE_RUNNING
        {
            info!("Restarting service with updated parameters...");
            let mut stop_status: SERVICE_STATUS = std::mem::zeroed();
            let _ = ControlService(service, SERVICE_CONTROL_STOP, &raw mut stop_status);
            sleep(Duration::from_millis(800));
        }

        let start_ok = StartServiceA(service, 0, null());
        let _ = CloseServiceHandle(service);
        let _ = CloseServiceHandle(scm);

        if start_ok == 0 {
            let start_err = GetLastError();
            // 1056 = ERROR_SERVICE_ALREADY_RUNNING
            if start_err != 1056 {
                return Err(format!(
                    "Service configured, but failed to start (error code {start_err})."
                ));
            }
        }

        info!("Segotep Digital Windows Service successfully installed and started.");
        Ok(())
    }
}

#[cfg(target_os = "windows")]
pub fn uninstall_service() -> Result<(), String> {
    unsafe {
        let scm = OpenSCManagerA(null(), null(), SC_MANAGER_ALL_ACCESS);
        if scm.is_null() {
            return Err(format!(
                "Failed to open Service Control Manager (error code {}). Please run as Administrator.",
                GetLastError()
            ));
        }

        let service = OpenServiceA(scm, SERVICE_NAME.as_ptr(), SERVICE_ALL_ACCESS);
        if service.is_null() {
            let err = GetLastError();
            let _ = CloseServiceHandle(scm);
            // 1060 = ERROR_SERVICE_DOES_NOT_EXIST
            if err == 1060 {
                info!("Service 'SegotepDigitalService' is not installed.");
                return Ok(());
            }
            return Err(format!(
                "Failed to open service for removal (error code {err})."
            ));
        }

        let mut status: SERVICE_STATUS = std::mem::zeroed();
        let _ = ControlService(service, SERVICE_CONTROL_STOP, &raw mut status);
        sleep(Duration::from_millis(500));

        let del_ok = DeleteService(service);
        let _ = CloseServiceHandle(service);
        let _ = CloseServiceHandle(scm);

        if del_ok == 0 {
            let err = GetLastError();
            return Err(format!(
                "Failed to delete Windows Service (error code {err})."
            ));
        }

        info!("Segotep Digital Windows Service successfully removed.");
        Ok(())
    }
}

#[cfg(target_os = "windows")]
pub fn run_service(
    interval_ms: u64,
    model_id: u8,
    fahrenheit: bool,
    vid: u16,
    pid: u16,
) -> Result<(), String> {
    {
        let mut cfg = SERVICE_CONFIG
            .lock()
            .map_err(|e| format!("Lock error: {e}"))?;
        *cfg = Some(ServiceConfig {
            interval_ms,
            model_id,
            fahrenheit,
            vid,
            pid,
        });
    }

    let service_table = [
        SERVICE_TABLE_ENTRYA {
            // SAFETY: lpServiceName is documented [in] (read-only) by the SCM for
            // SERVICE_WIN32_OWN_PROCESS; SERVICE_NAME is a null-terminated static byte string.
            lpServiceName: SERVICE_NAME.as_ptr() as *mut u8,
            lpServiceProc: Some(service_main),
        },
        SERVICE_TABLE_ENTRYA {
            lpServiceName: null_mut(),
            lpServiceProc: None,
        },
    ];

    let ok = unsafe { StartServiceCtrlDispatcherA(service_table.as_ptr()) };
    if ok == 0 {
        return Err(format!(
            "Failed to start service control dispatcher (error code {}).",
            unsafe { GetLastError() }
        ));
    }

    Ok(())
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn service_handler(
    dw_control: u32,
    _dw_event_type: u32,
    _lp_event_data: *mut c_void,
    _lp_context: *mut c_void,
) -> u32 {
    match dw_control {
        SERVICE_CONTROL_STOP | SERVICE_CONTROL_SHUTDOWN => {
            SERVICE_RUNNING_FLAG.store(false, Ordering::Relaxed);
            let handle = SERVICE_STATUS_HANDLE_VAL.load(Ordering::Relaxed);
            if !handle.is_null() {
                let status = SERVICE_STATUS {
                    dwServiceType: SERVICE_WIN32_OWN_PROCESS,
                    dwCurrentState: SERVICE_STOP_PENDING,
                    dwControlsAccepted: 0,
                    dwWin32ExitCode: 0,
                    dwServiceSpecificExitCode: 0,
                    dwCheckPoint: 1,
                    dwWaitHint: 3000,
                };
                unsafe {
                    SetServiceStatus(handle.cast(), &status);
                }
            }
            0
        }
        _ => 120, // NO_ERROR / ERROR_CALL_NOT_IMPLEMENTED
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn service_main(
    _dw_num_service_args: c_ulong,
    _lp_service_arg_vectors: *mut *mut u8,
) {
    let handle: SERVICE_STATUS_HANDLE = unsafe {
        RegisterServiceCtrlHandlerExA(SERVICE_NAME.as_ptr(), Some(service_handler), null_mut())
    };

    if handle.is_null() {
        return;
    }

    SERVICE_STATUS_HANDLE_VAL.store(handle.cast(), Ordering::Relaxed);

    let mut status = SERVICE_STATUS {
        dwServiceType: SERVICE_WIN32_OWN_PROCESS,
        dwCurrentState: SERVICE_START_PENDING,
        dwControlsAccepted: SERVICE_CONTROL_STOP | SERVICE_CONTROL_SHUTDOWN,
        dwWin32ExitCode: 0,
        dwServiceSpecificExitCode: 0,
        dwCheckPoint: 0,
        dwWaitHint: 2000,
    };
    unsafe {
        SetServiceStatus(handle, &status);
    }

    let config = {
        let lock = SERVICE_CONFIG.lock();
        lock.ok().and_then(|c| *c).unwrap_or(ServiceConfig {
            interval_ms: 1000,
            model_id: 3,
            fahrenheit: false,
            vid: 0x1a86,
            pid: 0xa001,
        })
    };

    status.dwCurrentState = SERVICE_RUNNING;
    unsafe {
        SetServiceStatus(handle, &status);
    }

    let mut dev = match SegotepDevice::with_custom_ids(config.vid, config.pid) {
        Ok(d) => d,
        Err(_) => {
            status.dwCurrentState = SERVICE_STOPPED;
            unsafe {
                SetServiceStatus(handle, &status);
            }
            return;
        }
    };

    let mut telemetry = SystemTelemetry::new();
    let tick_interval = Duration::from_millis(config.interval_ms.max(100));

    while SERVICE_RUNNING_FLAG.load(Ordering::Relaxed) {
        if dev.connect().is_err() {
            sleep(Duration::from_secs(2));
            continue;
        }

        let metrics = telemetry.sample();
        let packet = SegotepPacket {
            model_id: config.model_id,
            screen_on: true,
            cpu_temp: metrics.cpu_temp,
            cpu_load: metrics.cpu_load,
            cpu_power_watts: metrics.cpu_power_watts,
            cpu_clock_mhz: metrics.cpu_clock_mhz,
            gpu_temp: metrics.gpu_temp,
            gpu_load: metrics.gpu_load,
            gpu_power_watts: metrics.gpu_power_watts,
            gpu_clock_mhz: metrics.gpu_clock_mhz,
            is_fahrenheit: config.fahrenheit,
        };

        let _ = dev.send(&packet);
        sleep(tick_interval);
    }

    // Gracefully blank display before stopping service
    let off_packet = SegotepPacket {
        model_id: config.model_id,
        screen_on: false,
        cpu_temp: 0,
        cpu_load: 0,
        cpu_power_watts: 0,
        cpu_clock_mhz: 0,
        gpu_temp: 0,
        gpu_load: 0,
        gpu_power_watts: 0,
        gpu_clock_mhz: 0,
        is_fahrenheit: config.fahrenheit,
    };
    let _ = dev.send(&off_packet);

    status.dwCurrentState = SERVICE_STOPPED;
    unsafe {
        SetServiceStatus(handle, &status);
    }
}
