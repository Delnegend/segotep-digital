use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::sleep;
use std::time::Duration;

use clap::Parser;
use segotep_digital::protocol::DEFAULT_FALLBACK_MODEL_ID;
#[cfg(target_os = "windows")]
use segotep_digital::windows_service;
use segotep_digital::{PRODUCT_ID, SegotepDevice, SegotepPacket, SystemTelemetry, VENDOR_ID};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(
    name = "segotep-digital",
    author,
    version,
    about = "Cross-platform driver and service for Segotep Ice Moon / Digital series AIO CPU coolers"
)]
struct Args {
    /// Update interval in milliseconds
    #[arg(short, long, default_value_t = 1000)]
    interval_ms: u64,

    /// Device model ID (1 for Digital standard, 3 for Ice Moon)
    #[arg(short, long, default_value_t = DEFAULT_FALLBACK_MODEL_ID)]
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

    /// Install as a background Windows Service (auto-starts on system boot)
    #[cfg(target_os = "windows")]
    #[arg(long)]
    install_service: bool,

    /// Stop and uninstall the background Windows Service
    #[cfg(target_os = "windows")]
    #[arg(long)]
    uninstall_service: bool,

    /// Run as a Windows Service dispatcher (invoked by Windows SCM)
    #[cfg(target_os = "windows")]
    #[arg(long, hide = true)]
    service: bool,
}

fn parse_hex_id(id_str: &str, default: u16) -> u16 {
    let clean = id_str.trim_start_matches("0x").trim_start_matches("0X");
    u16::from_str_radix(clean, 16).unwrap_or(default)
}

#[allow(clippy::cognitive_complexity)]
fn main() {
    let args = Args::parse();

    let default_level = if args.verbose { "debug" } else { "info" };
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .compact()
        .init();

    let vid = args
        .vid
        .as_deref()
        .map_or(VENDOR_ID, |s| parse_hex_id(s, VENDOR_ID));
    let pid = args
        .pid
        .as_deref()
        .map_or(PRODUCT_ID, |s| parse_hex_id(s, PRODUCT_ID));

    #[cfg(target_os = "windows")]
    if args.install_service {
        info!("Installing Segotep Digital Windows Service...");
        if let Err(e) =
            windows_service::install_service(args.interval_ms, args.model_id, args.fahrenheit, vid, pid)
        {
            error!("Service installation failed: {e}");
            std::process::exit(1);
        }
        return;
    }

    #[cfg(target_os = "windows")]
    if args.uninstall_service {
        info!("Uninstalling Segotep Digital Windows Service...");
        if let Err(e) = windows_service::uninstall_service() {
            error!("Service uninstallation failed: {e}");
            std::process::exit(1);
        }
        return;
    }

    #[cfg(target_os = "windows")]
    if args.service {
        if let Err(e) =
            windows_service::run_service(args.interval_ms, args.model_id, args.fahrenheit, vid, pid)
        {
            error!("Windows Service execution failed: {e}");
            std::process::exit(1);
        }
        return;
    }

    info!("Starting Segotep Digital Driver");
    info!("Target Device: VID=0x{vid:04x}, PID=0x{pid:04x}");
    info!("Model ID: {}", args.model_id);
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
    let mut is_connected = false;

    while running.load(Ordering::Relaxed) {
        if dev.connect().is_err() {
            warn!(
                "Waiting for Segotep AIO USB device to connect (VID=0x{vid:04x}, PID=0x{pid:04x})..."
            );
            is_connected = false;
            sleep(Duration::from_secs(2));
            continue;
        }

        if !is_connected {
            info!("Device connected -> Active Model ID: {}", args.model_id);
            is_connected = true;
        }

        let metrics = telemetry.sample();

        if args.verbose {
            info!(
                "Telemetry -> CPU: {}°C, {}%, {}W, {}MHz (Model: {})",
                metrics.cpu_temp,
                metrics.cpu_load,
                metrics.cpu_power_watts,
                metrics.cpu_clock_mhz,
                args.model_id
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
            error!("Failed to send data: {e}. Will reconnect.");
            is_connected = false;
        }

        if args.screen_off {
            info!("Screen-OFF packet sent successfully. Exiting.");
            break;
        }

        sleep(tick_interval);
    }

    info!("Segotep Digital driver stopped cleanly.");
}
