//! Core library for communicating with Segotep Digital series AIO CPU coolers.

pub mod device;
pub mod monitor;
pub mod protocol;

pub use device::{DeviceInfo, SegotepDevice};
pub use monitor::{HardwareMetrics, SystemTelemetry};
pub use protocol::{DEFAULT_MODEL_ID_ICE_MOON, PRODUCT_ID, SegotepPacket, VENDOR_ID};
