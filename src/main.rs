use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::sleep;
use std::time::Duration;

use clap::Parser;
use segotep_digital::{
    DEFAULT_MODEL_ID_ICE_MOON, PRODUCT_ID, SegotepDevice, SegotepPacket, SystemTelemetry, VENDOR_ID,
};
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(
    name = "segotep-digital",
    author,
    version,
    about = "Linux driver and service for Segotep Ice Moon / Digital series AIO CPU coolers"
)]
struct Args {
    /// Update interval in milliseconds
    #[arg(short, long, default_value_t = 1000)]
    interval_ms: u64,

    /// Override device model ID (auto-detected from hardware by default)
    #[arg(short, long)]
    model_id: Option<u8>,

    /// Display temperature in Fahrenheit instead of Celsius
    #[arg(short = 'f', long)]
    fahrenheit: bool,

    /// Turn off the 7-segment display screen
    #[arg(long)]
    screen_off: bool,

    /// Custom USB Vendor ID (hex string e.g. 1a86)
    #[arg(long)]
    vid: Option<String>,

    /// Custom USB Product ID (hex string e.g. a001)
    #[arg(long)]
    pid: Option<String>,

    /// Print telemetry metrics to stdout on each tick
    #[arg(short, long)]
    verbose: bool,
}

fn parse_hex_id(id_str: &str, default: u16) -> u16 {
    let clean = id_str.trim_start_matches("0x").trim_start_matches("0X");
    u16::from_str_radix(clean, 16).unwrap_or(default)
}

fn initialize_device_connection(
    dev: &mut SegotepDevice,
    override_id: Option<u8>,
    fahrenheit: bool,
) -> u8 {
    let mut resolved_id = override_id.unwrap_or(DEFAULT_MODEL_ID_ICE_MOON);

    match dev.read_info(500) {
        Ok(Some(info)) => {
            if let Some(id) = override_id {
                resolved_id = id;
                info!(
                    "Device connected -> Auto-detected Model ID: {}, using manual override: {id}",
                    info.model_id
                );
            } else if info.model_id > 0 {
                resolved_id = info.model_id;
                info!(
                    "Device connected -> Auto-detected hardware Model ID: {} (CapMask: 0x{:02x}, Fahrenheit: {})",
                    info.model_id, info.capability_mask, info.is_fahrenheit_capable
                );
            } else {
                info!("Device connected -> Fallback Model ID: {resolved_id}");
            }

            if fahrenheit && !info.is_fahrenheit_capable {
                warn!(
                    "Hardware report indicates Fahrenheit may not be supported on this display revision."
                );
            }
        }
        Ok(None) => {
            info!(
                "Device connected -> No input report received within timeout, using Model ID: {resolved_id}"
            );
        }
        Err(e) => {
            debug!("Device report query notice: {e}");
        }
    }

    resolved_id
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();

    let vid = args
        .vid
        .as_deref()
        .map_or(VENDOR_ID, |s| parse_hex_id(s, VENDOR_ID));
    let pid = args
        .pid
        .as_deref()
        .map_or(PRODUCT_ID, |s| parse_hex_id(s, PRODUCT_ID));

    info!("Starting Segotep Digital Linux Driver");
    info!("Target Device: VID=0x{vid:04x}, PID=0x{pid:04x}");
    info!(
        "Update interval: {}ms, Fahrenheit: {}, Screen OFF: {}",
        args.interval_ms, args.fahrenheit, args.screen_off
    );

    let running = Arc::new(AtomicBool::new(true));
    let r = Arc::clone(&running);

    // Signal handler for graceful termination
    if let Err(e) = ctrlc::set_handler(move || {
        info!("Received termination signal. Exiting...");
        r.store(false, Ordering::Relaxed);
    }) {
        warn!("Failed to set Ctrl-C handler: {e}");
    }

    let mut dev = match SegotepDevice::with_custom_ids(vid, pid) {
        Ok(d) => d,
        Err(e) => {
            error!("Failed to initialize HID API: {e}");
            return;
        }
    };

    let mut telemetry = SystemTelemetry::new();
    let tick_interval = Duration::from_millis(args.interval_ms.max(100));
    let mut initial_connection = true;
    let mut active_model_id = args.model_id.unwrap_or(DEFAULT_MODEL_ID_ICE_MOON);

    while running.load(Ordering::Relaxed) {
        if dev.connect().is_err() {
            warn!(
                "Waiting for Segotep AIO USB device to connect (VID=0x{vid:04x}, PID=0x{pid:04x})..."
            );
            initial_connection = true;
            sleep(Duration::from_secs(2));
            continue;
        }

        if initial_connection {
            active_model_id =
                initialize_device_connection(&mut dev, args.model_id, args.fahrenheit);
            initial_connection = false;
        }

        let metrics = telemetry.sample();

        if args.verbose {
            info!(
                "Telemetry -> CPU: {}°C, {}%, {}W, {}MHz",
                metrics.cpu_temp, metrics.cpu_load, metrics.cpu_power_watts, metrics.cpu_clock_mhz
            );
        }

        let packet = SegotepPacket {
            model_id: active_model_id,
            screen_on: !args.screen_off,
            cpu_temp: metrics.cpu_temp,
            cpu_load: metrics.cpu_load,
            cpu_power_watts: metrics.cpu_power_watts,
            cpu_clock_mhz: metrics.cpu_clock_mhz,
            gpu_temp: metrics.gpu_temp,
            gpu_load: metrics.gpu_load,
            gpu_power_watts: metrics.gpu_power_watts,
            gpu_clock_mhz: metrics.gpu_clock_mhz,
            is_fahrenheit: args.fahrenheit,
        };

        if let Err(e) = dev.send(&packet) {
            error!("Failed to send data: {e}. Will reconnect.");
            initial_connection = true;
        }

        sleep(tick_interval);
    }

    info!("Segotep Digital driver stopped cleanly.");
}
