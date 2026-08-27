//! USB HID packet representation and serialization for Segotep Digital coolers.

pub const VENDOR_ID: u16 = 0x1A86;
pub const PRODUCT_ID: u16 = 0xA001;

/// Default model ID when hardware auto-detection does not return a report (1 for standard / 3 for Ice Moon)
pub const DEFAULT_FALLBACK_MODEL_ID: u8 = 1;

/// Magic header bytes sent at byte index 1 and 2.
pub const MAGIC_HEADER: [u8; 2] = [0xDC, 0xDD];

/// Screen control constants
pub const SCREEN_STATE_ON: u8 = 0x00;
pub const SCREEN_STATE_OFF: u8 = 0x0E;
pub const FLASH_VALUE1_OFF: u8 = 0x0F;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegotepPacket {
    /// Device model ID (auto-detected from hardware report or fallback)
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
            model_id: DEFAULT_FALLBACK_MODEL_ID,
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
    /// Serializes the telemetry metrics into the 34-byte USB HID report.
    ///
    /// # Packet Layout (matching official `SendLatestValues` byte stream):
    /// - Bytes 0..2: `[0x00, 0xDC, 0xDD]`
    /// - Byte 3: Screen power state (`0x00` ON, `0x0E` OFF)
    /// - Byte 4: `flash_value1` (`model_id` when screen ON, `0x0F` when screen OFF)
    /// - Bytes 5..12: `[0; 8]`
    /// - Byte 13: `0x01` (Fixed protocol header flag)
    /// - Bytes 14..16: `[0; 3]`
    /// - Byte 17: `0x0C` (Fixed protocol header flag, decimal 12)
    /// - Bytes 18..20: `[0; 3]`
    /// - Byte 21: CPU Temp (`latestValues[0]`)
    /// - Byte 22: CPU Load (`latestValues[1]`)
    /// - Bytes 23..24: CPU Power Watts LE (`latestValues[3]`)
    /// - Bytes 25..26: CPU Clock MHz LE (`latestValues[2]`)
    /// - Byte 27: GPU Temp (`latestValues[4]`)
    /// - Byte 28: GPU Load (`latestValues[5]`)
    /// - Bytes 29..30: GPU Power Watts LE (`latestValues[7]`)
    /// - Bytes 31..32: GPU Clock MHz LE (`latestValues[6]`)
    /// - Byte 33: `is_fahrenheit` (1 if °F, 0 if °C)
    #[must_use]
    #[allow(clippy::indexing_slicing, clippy::bool_to_int_with_if)]
    pub const fn serialize(&self) -> [u8; 34] {
        let mut packet = [0u8; 34];

        // Magic Header: 0, 220 (0xDC), 221 (0xDD)
        packet[1] = MAGIC_HEADER[0];
        packet[2] = MAGIC_HEADER[1];

        // Model ID: sent in byte index 4 (and byte 3 is screen on/off)
        packet[4] = self.model_id;

        // Fixed protocol markers from official app
        packet[13] = 0x01;
        packet[17] = 0x0C;

        // Telemetry Data (second array in C#)
        packet[21] = self.cpu_temp;
        packet[22] = self.cpu_load;

        // In C# official app:
        // result4 is CpuPower -> placed at bytes 23 & 24
        let cpu_pwr = self.cpu_power_watts.to_le_bytes();
        packet[23] = cpu_pwr[0];
        packet[24] = cpu_pwr[1];

        // result3 is CpuClock -> placed at bytes 25 & 26
        let cpu_clk = self.cpu_clock_mhz.to_le_bytes();
        packet[25] = cpu_clk[0];
        packet[26] = cpu_clk[1];

        packet[27] = self.gpu_temp;
        packet[28] = self.gpu_load;

        // result8 is GpuPower -> placed at bytes 29 & 30
        let gpu_pwr = self.gpu_power_watts.to_le_bytes();
        packet[29] = gpu_pwr[0];
        packet[30] = gpu_pwr[1];

        // result7 is GpuClock -> placed at bytes 31 & 32
        let gpu_clk = self.gpu_clock_mhz.to_le_bytes();
        packet[31] = gpu_clk[0];
        packet[32] = gpu_clk[1];

        packet[33] = if self.is_fahrenheit { 1 } else { 0 };

        // If screen is OFF, official app overrides indices 3 and 4:
        // second[3] = 14 (0x0E), second[4] = 15 (0x0F)
        if !self.screen_on {
            packet[3] = SCREEN_STATE_OFF;
            packet[4] = FLASH_VALUE1_OFF;
        }

        packet
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
        assert_eq!(bytes[3], 0x00); // Screen ON
        assert_eq!(bytes[4], 1); // Default fallback model_id
        assert_eq!(bytes[13], 0x01);
        assert_eq!(bytes[17], 0x0C);
        assert_eq!(bytes[33], 0); // Celsius
    }

    #[test]
    fn test_packet_values_encoding() {
        let packet = SegotepPacket {
            model_id: 3,
            screen_on: true,
            cpu_temp: 55,
            cpu_load: 80,
            cpu_power_watts: 125,
            cpu_clock_mhz: 4800,
            gpu_temp: 65,
            gpu_load: 99,
            gpu_power_watts: 250,
            gpu_clock_mhz: 2100,
            is_fahrenheit: true,
        };

        let bytes = packet.serialize();

        assert_eq!(bytes[4], 3);
        assert_eq!(bytes[13], 0x01);
        assert_eq!(bytes[17], 0x0C);
        assert_eq!(bytes[21], 55);
        assert_eq!(bytes[22], 80);
        assert_eq!(u16::from_le_bytes([bytes[23], bytes[24]]), 125);
        assert_eq!(u16::from_le_bytes([bytes[25], bytes[26]]), 4800);
        assert_eq!(bytes[27], 65);
        assert_eq!(bytes[28], 99);
        assert_eq!(u16::from_le_bytes([bytes[29], bytes[30]]), 250);
        assert_eq!(u16::from_le_bytes([bytes[31], bytes[32]]), 2100);
        assert_eq!(bytes[33], 1);
    }

    #[test]
    fn test_screen_off_encoding() {
        let packet = SegotepPacket {
            model_id: 3,
            screen_on: false,
            ..Default::default()
        };

        let bytes = packet.serialize();

        assert_eq!(bytes[3], 0x0E);
        assert_eq!(bytes[4], 0x0F);
    }
}
