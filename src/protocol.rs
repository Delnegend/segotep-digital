//! Protocol implementation for Segotep Digital / Ice Moon series AIO displays.
//!
//! Based on reverse-engineered USB HID protocol from official Windows client software.

pub const VENDOR_ID: u16 = 0x1a86;
pub const PRODUCT_ID: u16 = 0xa001;

pub const DEFAULT_MODEL_ID_ICE_MOON: u8 = 3;

/// Represents the telemetry and display payload sent to the Segotep AIO block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegotepPacket {
    /// Model identifier (e.g. 3 for Ice Moon)
    pub model_id: u8,
    /// Whether the 7-segment display is powered on
    pub screen_on: bool,
    /// Temperature in Celsius (0-255)
    pub cpu_temp: u8,
    /// CPU Load percentage (0-100)
    pub cpu_load: u8,
    /// CPU Power in Watts (0-65535)
    pub cpu_power_watts: u16,
    /// CPU Frequency in MHz (0-65535)
    pub cpu_clock_mhz: u16,
    /// GPU Temperature in Celsius (0-255)
    pub gpu_temp: u8,
    /// GPU Load percentage (0-100)
    pub gpu_load: u8,
    /// GPU Power in Watts (0-65535)
    pub gpu_power_watts: u16,
    /// GPU Frequency in MHz (0-65535)
    pub gpu_clock_mhz: u16,
    /// Temperature unit display toggle: false = Celsius, true = Fahrenheit
    pub is_fahrenheit: bool,
}

impl Default for SegotepPacket {
    fn default() -> Self {
        Self {
            model_id: DEFAULT_MODEL_ID_ICE_MOON,
            screen_on: true,
            cpu_temp: 0,
            cpu_load: 0,
            cpu_power_watts: 0,
            cpu_clock_mhz: 0,
            gpu_temp: 0,
            gpu_load: 0,
            gpu_power_watts: 0,
            gpu_clock_mhz: 0,
            is_fahrenheit: false,
        }
    }
}

impl SegotepPacket {
    /// Serializes the packet into the exact 34-byte HID report expected by the device.
    pub fn serialize(&self) -> [u8; 34] {
        let mut buf = [0u8; 34];

        // Byte 0: Report ID (0x00)
        buf[0] = 0x00;

        // Bytes 1..2: Magic header (220, 221 / 0xDC, 0xDD)
        buf[1] = 0xDC;
        buf[2] = 0xDD;

        // Byte 3 & 4: Screen power & Model ID
        if self.screen_on {
            buf[3] = 0x00;
            buf[4] = self.model_id;
        } else {
            // Screen off magic bytes
            buf[3] = 14; // 0x0E
            buf[4] = 15; // 0x0F
        }

        // Bytes 5..12: Reserved (zeros)
        // Byte 13: Protocol flag 0x01
        buf[13] = 0x01;

        // Bytes 14..16: Reserved (zeros)
        // Byte 17: Payload length indicator (12 / 0x0C)
        buf[17] = 0x0C;

        // Bytes 18..20: Reserved (zeros)

        // Telemetry payload:
        // Byte 21: CPU Temp
        buf[21] = self.cpu_temp;

        // Byte 22: CPU Load (%)
        buf[22] = self.cpu_load;

        // Bytes 23..24: CPU Power (Little Endian, in Watts)
        buf[23] = (self.cpu_power_watts & 0xFF) as u8;
        buf[24] = ((self.cpu_power_watts >> 8) & 0xFF) as u8;

        // Bytes 25..26: CPU Clock (Little Endian, in MHz)
        buf[25] = (self.cpu_clock_mhz & 0xFF) as u8;
        buf[26] = ((self.cpu_clock_mhz >> 8) & 0xFF) as u8;

        // Byte 27: GPU Temp
        buf[27] = self.gpu_temp;

        // Byte 28: GPU Load (%)
        buf[28] = self.gpu_load;

        // Bytes 29..30: GPU Power (Little Endian, in Watts)
        buf[29] = (self.gpu_power_watts & 0xFF) as u8;
        buf[30] = ((self.gpu_power_watts >> 8) & 0xFF) as u8;

        // Bytes 31..32: GPU Clock (Little Endian, in MHz)
        buf[31] = (self.gpu_clock_mhz & 0xFF) as u8;
        buf[32] = ((self.gpu_clock_mhz >> 8) & 0xFF) as u8;

        // Byte 33: Temperature Unit (0 = Celsius, 1 = Fahrenheit)
        buf[33] = if self.is_fahrenheit { 1 } else { 0 };

        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_serialization() {
        let packet = SegotepPacket {
            model_id: 3,
            screen_on: true,
            cpu_temp: 45,
            cpu_load: 25,
            cpu_power_watts: 65,
            cpu_clock_mhz: 4200,
            gpu_temp: 50,
            gpu_load: 10,
            gpu_power_watts: 120,
            gpu_clock_mhz: 2100,
            is_fahrenheit: false,
        };

        let raw = packet.serialize();
        assert_eq!(raw[0], 0x00);
        assert_eq!(raw[1], 0xDC);
        assert_eq!(raw[2], 0xDD);
        assert_eq!(raw[3], 0x00);
        assert_eq!(raw[4], 0x03);
        assert_eq!(raw[13], 0x01);
        assert_eq!(raw[17], 0x0C);
        assert_eq!(raw[21], 45); // CPU temp
        assert_eq!(raw[22], 25); // CPU load
        assert_eq!(u16::from_le_bytes([raw[23], raw[24]]), 65); // CPU power
        assert_eq!(u16::from_le_bytes([raw[25], raw[26]]), 4200); // CPU clock
        assert_eq!(raw[27], 50); // GPU temp
        assert_eq!(raw[28], 10); // GPU load
        assert_eq!(u16::from_le_bytes([raw[29], raw[30]]), 120); // GPU power
        assert_eq!(u16::from_le_bytes([raw[31], raw[32]]), 2100); // GPU clock
        assert_eq!(raw[33], 0); // Celsius
    }

    #[test]
    fn test_screen_off_serialization() {
        let packet = SegotepPacket {
            screen_on: false,
            ..Default::default()
        };

        let raw = packet.serialize();
        assert_eq!(raw[3], 14);
        assert_eq!(raw[4], 15);
    }
}
