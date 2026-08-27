//! Segotep Digital Core Library
//!
//! Provides reverse-engineered USB HID protocol serialization, device communication,
//! and hardware telemetry monitoring for Segotep Ice Moon / Digital series AIO CPU coolers.

pub mod device;
pub mod monitor;
pub mod protocol;

pub use device::{DeviceInfo, SegotepDevice};
pub use monitor::{HardwareMetrics, SystemTelemetry};
pub use protocol::{PRODUCT_ID, SegotepPacket, VENDOR_ID};
