#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
use alloc::vec::Vec;
use embassy_executor::Spawner;
use embassy_time::{Delay, Duration, Timer};
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Level, Output, OutputConfig, OutputPin};
use esp_hal::peripherals;
use esp_hal::rmt::{PulseCode, Rmt, TxChannelConfig};
use esp_hal::rmt::{TxChannel, TxChannelCreator};
use esp_hal::timer::systimer::SystemTimer;
use esp_hal::timer::timg::TimerGroup;
// use esp_wifi::ble::controller::BleConnector;
use log::info;
use precir::commands::{
    build_data_frames, change_page, get_final_frame, get_image_parameter_frame, get_wakeup_command,
};
use precir::{frame_to_pulses, pp16_symbol_duration};

extern crate alloc;
// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    // generator version: 0.5.0

    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 64 * 1024);
    // COEX needs more RAM - so we've added some more
    esp_alloc::heap_allocator!(#[unsafe(link_section = ".dram2_uninit")] size: 64 * 1024);

    let timer0 = SystemTimer::new(peripherals.SYSTIMER);
    esp_hal_embassy::init(timer0.alarm0);

    info!("Embassy initialized!");

    let rng = esp_hal::rng::Rng::new(peripherals.RNG);
    let timer1 = TimerGroup::new(peripherals.TIMG0);
    let wifi_init =
        esp_wifi::init(timer1.timer0, rng).expect("Failed to initialize WIFI/BLE controller");
    let (mut _wifi_controller, _interfaces) = esp_wifi::wifi::new(&wifi_init, peripherals.WIFI)
        .expect("Failed to initialize WIFI controller");
    // let _connector = BleConnector::new(&wifi_init, peripherals.BT);

    let freq = esp_hal::time::Rate::from_mhz(80);
    let rmt = Rmt::new(peripherals.RMT, freq).unwrap();
    let plid: [u8; 4] = [0xd0, 0x39, 0xc3, 0xde];
    let mut tx = rmt
        .channel0
        .configure_tx(
            peripherals.GPIO7,
            TxChannelConfig::default()
                .with_clk_divider(1)
                .with_idle_output(true) // actively drive low when idle
                .with_idle_output_level(Level::Low)
                .with_carrier_modulation(true) // enable 1.25 MHz bursts
                .with_carrier_high(32) // 400 ns high
                .with_carrier_low(32) // 400 ns low  → ~1.25 MHz
                .with_carrier_level(Level::High), // carrier present when RMT level is High
        )
        .unwrap();
    let wakeup_frame = get_wakeup_command(plid);
    info!("Frame: {:?}", &wakeup_frame);
    let wakeup_pulses = frame_to_pulses(wakeup_frame);
    for _ in 0..1000 {
        let txing = tx.transmit(wakeup_pulses.as_slice()).unwrap();
        tx = txing.wait().unwrap();
    }
    info!("Wakeup done");

    let mut img = build_black_square(16, 16);
    let padding = (20 - (img.len() % 20)) % 20;
    img.extend(core::iter::repeat(0).take(padding));

    // let param_frame = get_image_parameter_frame(plid, 8, 8, 0, 0, img.len() as u16);
    let param_frame = get_image_parameter_frame(plid, 16, 16, 0, 0, img.len() as u16);
    info!("Param frame: {:?}", param_frame);
    let param_pulses = frame_to_pulses(param_frame);
    let txing = tx.transmit(param_pulses.as_slice()).unwrap();
    tx = txing.wait().unwrap();
    info!("Transmitted Param frame");

    let data_frames = build_data_frames(plid, &img);
    for frame in data_frames {
        info!("Data frame: {:?}", frame);
        let pulses = frame_to_pulses(frame);
        let txing = tx.transmit(pulses.as_slice()).unwrap();
        tx = txing.wait().unwrap();
    }
    info!("Transmitted Data frames");

    let final_frame = get_final_frame(plid);
    info!("Final frame: {:?}", final_frame);
    let final_pulses = frame_to_pulses(final_frame);
    let txing = tx.transmit(final_pulses.as_slice()).unwrap();
    tx = txing.wait().unwrap();
    info!("Transmitted Final frame");

    // TODO: Spawn some tasks

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0-rc.0/examples/src/bin
}

fn build_black_square(width: u16, height: u16) -> Vec<u8> {
    let row_bytes = (width + 7) / 8; // ceil(width/8)
    let mut img: Vec<u8> = Vec::new();
    for _ in 0..height {
        for _ in 0..row_bytes {
            img.push(0x00); // all black pixels
        }
    }
    img
}

async fn transmit_burst(pin: &mut Output<'static>, microseconds: u32) {
    let half_period_ns = 400; // 1.25 MHz → 800 ns full period → 400 ns half period

    let cycles = (microseconds * 1000) / half_period_ns; // approximate number of half-cycles

    for _ in 0..cycles {
        pin.set_high();
        Timer::after(Duration::from_nanos(half_period_ns as u64)).await;
        pin.set_low();
        Timer::after(Duration::from_nanos(half_period_ns as u64)).await;
    }
}

/// Transmit a Vec<u8> using PPM according to pp16_symbol_duration.
pub async fn transmit_ppm(pin: &mut Output<'static>, data: &[u8]) {
    // For each byte, transmit high nibble then low nibble
    for &byte in data {
        let nibbles = [(byte >> 4) & 0x0F, byte & 0x0F];
        for &nibble in &nibbles {
            let duration = pp16_symbol_duration(nibble);
            transmit_burst(pin, duration as u32).await;
        }
    }
    // One extra burst at the end (n+1 bursts)
    transmit_burst(pin, 27).await; // Could be a fixed "stop" burst
}
