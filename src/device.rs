//! USB HID connection and communication management.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

use hidapi::{HidApi, HidDevice};
use log::{debug, error, info, warn};

use crate::protocol::{SegotepPacket, PRODUCT_ID, VENDOR_ID};

/// Device capabilities and status received from device input reports.
#[derive(Debug, Clone, Copy)]
pub struct DeviceInfo {
    pub model_id: u8,
    pub capability_mask: u8,
    pub is_fahrenheit_capable: bool,
}

pub struct SegotepDevice {
    api: HidApi,
    device: Option<HidDevice>,
    vendor_id: u16,
    product_id: u16,
}

impl SegotepDevice {
    /// Creates a new device manager instance.
    pub fn new() -> Result<Self, hidapi::HidError> {
        let api = HidApi::new()?;
        Ok(Self {
            api,
            device: None,
            vendor_id: VENDOR_ID,
            product_id: PRODUCT_ID,
        })
    }

    /// Allows overriding VID/PID if using an alternative model.
    pub fn with_custom_ids(vendor_id: u16, product_id: u16) -> Result<Self, hidapi::HidError> {
        let api = HidApi::new()?;
        Ok(Self {
            api,
            device: None,
            vendor_id,
            product_id,
        })
    }

    /// Attempts to connect or reconnect to the USB HID device.
    pub fn connect(&mut self) -> Result<(), String> {
        if self.device.is_some() {
            return Ok(());
        }

        // Refresh enumerated devices list
        if let Err(e) = self.api.refresh_devices() {
            debug!("Device refresh warning: {}", e);
        }

        match self.api.open(self.vendor_id, self.product_id) {
            Ok(dev) => {
                info!(
                    "Successfully connected to Segotep device (VID=0x{:04x}, PID=0x{:04x})",
                    self.vendor_id, self.product_id
                );
                self.device = Some(dev);
                Ok(())
            }
            Err(e) => Err(format!(
                "Failed to open device (VID=0x{:04x}, PID=0x{:04x}): {}",
                self.vendor_id, self.product_id, e
            )),
        }
    }

    /// Sends a telemetry packet to the AIO screen.
    pub fn send(&mut self, packet: &SegotepPacket) -> Result<(), String> {
        let data = packet.serialize();

        if let Some(ref dev) = self.device {
            match dev.write(&data) {
                Ok(bytes_written) => {
                    debug!("Sent {} bytes to screen", bytes_written);
                    Ok(())
                }
                Err(e) => {
                    warn!("Write failed ({}), disconnecting device...", e);
                    self.device = None;
                    Err(format!("Write error: {}", e))
                }
            }
        } else {
            Err("Device is not connected".into())
        }
    }

    /// Read optional incoming report to fetch device hardware capabilities.
    pub fn read_info(&mut self, timeout_ms: i32) -> Result<Option<DeviceInfo>, String> {
        if let Some(ref dev) = self.device {
            let mut buf = [0u8; 64];
            match dev.read_timeout(&mut buf, timeout_ms) {
                Ok(len) if len >= 10 => {
                    let model_id = buf[5];
                    let capability_mask = buf[8];
                    let is_fahrenheit_capable = buf[9] == 1;

                    Ok(Some(DeviceInfo {
                        model_id,
                        capability_mask,
                        is_fahrenheit_capable,
                    }))
                }
                Ok(_) => Ok(None),
                Err(e) => {
                    debug!("Read info timed out or failed: {}", e);
                    Ok(None)
                }
            }
        } else {
            Err("Device not connected".into())
        }
    }

    /// Runs a resilient update loop with auto-reconnect.
    pub fn run_loop<F>(
        &mut self,
        interval: Duration,
        running: Arc<AtomicBool>,
        mut get_telemetry: F,
    ) where
        F: FnMut() -> SegotepPacket,
    {
        info!("Starting update loop with interval {:?}", interval);

        while running.load(Ordering::Relaxed) {
            if self.device.is_none() {
                if let Err(e) = self.connect() {
                    warn!("{}. Retrying in 2 seconds...", e);
                    sleep(Duration::from_secs(2));
                    continue;
                }
            }

            let packet = get_telemetry();
            if let Err(e) = self.send(&packet) {
                error!("Telemetry send failed: {}", e);
            }

            sleep(interval);
        }

        // On shutdown, optionally clear or maintain state
        info!("Shutting down device communication...");
    }
}
