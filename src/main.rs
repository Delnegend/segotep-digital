use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::sleep;
use std::time::Duration;

use clap::Parser;
use log::{error, info, warn};

mod device;
mod monitor;
mod protocol;

use device::SegotepDevice;
use monitor::SystemTelemetry;
use protocol::{DEFAULT_MODEL_ID_ICE_MOON, PRODUCT_ID, SegotepPacket, VENDOR_ID};

#[derive(Parser, Debug)]
#[command(
    name = "segotep-digital-rs",
    author = "Delnegend <kiennguyen19323@gmail.com>",
    version = "0.1.0",
    about = "Linux driver and service for Segotep Ice Moon / Digital series AIO CPU coolers"
)]
struct Args {
    /// Update interval in milliseconds
    #[arg(short, long, default_value_t = 1000)]
    interval_ms: u64,

    /// Device model ID (default: 3 for Ice Moon)
    #[arg(short, long, default_value_t = DEFAULT_MODEL_ID_ICE_MOON)]
    model_id: u8,

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

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();

    let vid = args
        .vid
        .as_deref()
        .map(|s| parse_hex_id(s, VENDOR_ID))
        .unwrap_or(VENDOR_ID);
    let pid = args
        .pid
        .as_deref()
        .map(|s| parse_hex_id(s, PRODUCT_ID))
        .unwrap_or(PRODUCT_ID);

    info!("Starting Segotep Digital Linux Driver");
    info!(
        "Target Device: VID=0x{:04x}, PID=0x{:04x}, Model={}",
        vid, pid, args.model_id
    );
    info!(
        "Update interval: {}ms, Fahrenheit: {}, Screen OFF: {}",
        args.interval_ms, args.fahrenheit, args.screen_off
    );

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    // Signal handler for graceful termination
    if let Err(e) = ctrlc::set_handler(move || {
        info!("Received termination signal. Exiting...");
        r.store(false, Ordering::Relaxed);
    }) {
        warn!("Failed to set Ctrl-C handler: {}", e);
    }

    let mut dev = match SegotepDevice::with_custom_ids(vid, pid) {
        Ok(d) => d,
        Err(e) => {
            error!("Failed to initialize HID API: {}", e);
            return;
        }
    };

    let mut telemetry = SystemTelemetry::new();
    let tick_interval = Duration::from_millis(args.interval_ms.max(100));

    while running.load(Ordering::Relaxed) {
        if dev.connect().is_err() {
            warn!(
                "Waiting for Segotep AIO USB device to connect (VID=0x{:04x}, PID=0x{:04x})...",
                vid, pid
            );
            sleep(Duration::from_secs(2));
            continue;
        }

        let metrics = telemetry.sample();

        if args.verbose {
            info!(
                "Telemetry -> CPU: {}°C, {}%, {}W, {}MHz",
                metrics.cpu_temp, metrics.cpu_load, metrics.cpu_power_watts, metrics.cpu_clock_mhz
            );
        }

        let packet = SegotepPacket {
            model_id: args.model_id,
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
            error!("Failed to send data: {}. Will reconnect.", e);
        }

        sleep(tick_interval);
    }

    info!("Segotep Digital driver stopped cleanly.");
}
