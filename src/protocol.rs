//! USB HID packet representation and serialization for Segotep Digital coolers.

pub const VENDOR_ID: u16 = 0x1A86;
pub const PRODUCT_ID: u16 = 0xA001;

pub const DEFAULT_MODEL_ID_ICE_MOON: u8 = 3;

/// Magic header bytes sent at byte index 1 and 2.
pub const MAGIC_HEADER: [u8; 2] = [0xDC, 0xDD];

/// Screen control constants
pub const SCREEN_STATE_ON: u8 = 0x00;
pub const SCREEN_STATE_OFF: u8 = 0x0E;
pub const FLASH_VALUE1_OFF: u8 = 0x0F;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegotepPacket {
    /// Device model ID (e.g. 3 for Ice Moon)
    pub model_id: u8,
    /// Screen power state
    pub screen_on: bool,
    /// CPU Temperature in °C (or °F if `is_fahrenheit` is true)
    pub cpu_temp: u8,
    /// CPU Utilization (0 - 100%)
    pub cpu_load: u8,
    /// CPU Package Power in Watts
    pub cpu_power_watts: u16,
    /// CPU Clock Speed in MHz
    pub cpu_clock_mhz: u16,
    /// GPU Temperature in °C
    pub gpu_temp: u8,
    /// GPU Utilization (0 - 100%)
    pub gpu_load: u8,
    /// GPU Package Power in Watts
    pub gpu_power_watts: u16,
    /// GPU Clock Speed in MHz
    pub gpu_clock_mhz: u16,
    /// Whether temperature is displayed in Fahrenheit
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
    /// Serializes the telemetry data into the exact 34-byte USB HID report.
    #[must_use]
    #[allow(clippy::indexing_slicing, clippy::bool_to_int_with_if)]
    pub const fn serialize(&self) -> [u8; 34] {
        let mut buf = [0u8; 34];

        // buf[0] = HID Report ID 0x00
        buf[0] = 0x00;

        // Magic header 220, 221 (0xDC, 0xDD)
        buf[1] = MAGIC_HEADER[0];
        buf[2] = MAGIC_HEADER[1];

        // Screen state and flash profile
        if self.screen_on {
            buf[3] = SCREEN_STATE_ON;
            buf[4] = self.model_id;
        } else {
            buf[3] = SCREEN_STATE_OFF;
            buf[4] = FLASH_VALUE1_OFF;
        }

        // Fixed protocol indicators
        buf[13] = 0x01;
        buf[17] = 0x0C; // Data payload length: 12 bytes

        // CPU Telemetry
        buf[21] = self.cpu_temp;
        buf[22] = self.cpu_load;

        // CPU Power (u16 Little Endian)
        let cpu_pwr_bytes = self.cpu_power_watts.to_le_bytes();
        buf[23] = cpu_pwr_bytes[0];
        buf[24] = cpu_pwr_bytes[1];

        // CPU Clock (u16 Little Endian)
        let cpu_clk_bytes = self.cpu_clock_mhz.to_le_bytes();
        buf[25] = cpu_clk_bytes[0];
        buf[26] = cpu_clk_bytes[1];

        // GPU Telemetry
        buf[27] = self.gpu_temp;
        buf[28] = self.gpu_load;

        // GPU Power (u16 Little Endian)
        let gpu_pwr_bytes = self.gpu_power_watts.to_le_bytes();
        buf[29] = gpu_pwr_bytes[0];
        buf[30] = gpu_pwr_bytes[1];

        // GPU Clock (u16 Little Endian)
        let gpu_clk_bytes = self.gpu_clock_mhz.to_le_bytes();
        buf[31] = gpu_clk_bytes[0];
        buf[32] = gpu_clk_bytes[1];

        // Fahrenheit flag
        buf[33] = if self.is_fahrenheit { 1 } else { 0 };

        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_serialization_defaults() {
        let packet = SegotepPacket::default();
        let bytes = packet.serialize();

        assert_eq!(bytes.len(), 34);
        assert_eq!(bytes[0], 0x00);
        assert_eq!(bytes[1], 0xDC);
        assert_eq!(bytes[2], 0xDD);
        assert_eq!(bytes[3], 0x00);
        assert_eq!(bytes[4], 0x03);
        assert_eq!(bytes[13], 0x01);
        assert_eq!(bytes[17], 0x0C);
        assert_eq!(bytes[33], 0x00);
    }

    #[test]
    fn test_packet_values_encoding() {
        let packet = SegotepPacket {
            model_id: 3,
            screen_on: true,
            cpu_temp: 65,
            cpu_load: 45,
            cpu_power_watts: 125,
            cpu_clock_mhz: 4800,
            gpu_temp: 55,
            gpu_load: 80,
            gpu_power_watts: 250,
            gpu_clock_mhz: 2200,
            is_fahrenheit: true,
        };

        let bytes = packet.serialize();

        assert_eq!(bytes[21], 65);
        assert_eq!(bytes[22], 45);
        assert_eq!(u16::from_le_bytes([bytes[23], bytes[24]]), 125);
        assert_eq!(u16::from_le_bytes([bytes[25], bytes[26]]), 4800);
        assert_eq!(bytes[27], 55);
        assert_eq!(bytes[28], 80);
        assert_eq!(u16::from_le_bytes([bytes[29], bytes[30]]), 250);
        assert_eq!(u16::from_le_bytes([bytes[31], bytes[32]]), 2200);
        assert_eq!(bytes[33], 1);
    }

    #[test]
    fn test_screen_off_encoding() {
        let packet = SegotepPacket {
            screen_on: false,
            ..Default::default()
        };

        let bytes = packet.serialize();
        assert_eq!(bytes[3], 0x0E);
        assert_eq!(bytes[4], 0x0F);
    }
}
