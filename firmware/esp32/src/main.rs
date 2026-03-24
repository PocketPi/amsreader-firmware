//! AMS Reader — ESP32: read HAN UART and parse frames (HDLC / M-Bus / DSMR chain).

use ams_core::han::{try_unwrap_han, GbtParser, HdlcParser, MbusParser, ParserContext, UnwrapResult};
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::uart::config::Config as UartConfig;
use esp_idf_hal::uart::UartDriver;
use esp_idf_hal::units::Hertz;
use esp_idf_svc::sys::TickType_t;

const HAN_BUF: usize = 2048;

fn han_baud() -> u32 {
    option_env!("AMS_HAN_BAUD")
        .and_then(|s| s.parse().ok())
        .unwrap_or(2400)
}

fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("AMS Reader (Rust) — HAN UART + parse");

    let peripherals = Peripherals::take().expect("peripherals");
    let pins = peripherals.pins;

    // Default: UART1 TX=GPIO17, RX=GPIO16 (common HAN wiring). Override with build flags:
    // `cargo build --release --manifest-path esp32/Cargo.toml --config 'env.AMS_HAN_TX="21"'` etc. not supported;
    // use source edit or we add sdkconfig if needed.
    let uart_cfg = UartConfig::default()
        .baudrate(Hertz(han_baud()))
        .data_bits(esp_idf_hal::uart::config::DataBits::DataBits8)
        .parity_even()
        .stop_bits(esp_idf_hal::uart::config::StopBits::STOP1);

    let uart = UartDriver::new(
        peripherals.uart1,
        pins.gpio17,
        pins.gpio16,
        Option::<esp_idf_hal::gpio::AnyIOPin>::None,
        Option::<esp_idf_hal::gpio::AnyIOPin>::None,
        &uart_cfg,
    )
    .expect("UART1");

    let mut rx_buf: Vec<u8> = Vec::with_capacity(HAN_BUF);
    let mut ctx = ParserContext::default();
    let mut hdlc = HdlcParser::new();
    let mut mbus = MbusParser::new();
    let mut gbt = GbtParser::new();
    let mut serial_flushed = false;

    let mut chunk = [0u8; 256];
    let tick_short: TickType_t = 1;

    loop {
        match uart.read(&mut chunk, tick_short) {
            Ok(0) => continue,
            Ok(n) => {
                if !serial_flushed {
                    rx_buf.clear();
                    serial_flushed = true;
                    continue;
                }
                for &b in &chunk[..n] {
                    if rx_buf.len() >= HAN_BUF {
                        log::warn!("HAN buffer overflow, clearing");
                        rx_buf.clear();
                        continue;
                    }
                    rx_buf.push(b);
                    match try_unwrap_han(&mut rx_buf, &mut ctx, &mut hdlc, &mut mbus, &mut gbt) {
                        UnwrapResult::Incomplete => {}
                        UnwrapResult::Intermediate => {
                            rx_buf.clear();
                        }
                        UnwrapResult::Complete {
                            frame_type,
                            payload_offset,
                            payload_len,
                            ..
                        } => {
                            let end = payload_offset + payload_len;
                            if end <= rx_buf.len() {
                                let payload = &rx_buf[payload_offset..end];
                                log::info!(
                                    "HAN frame type=0x{:02X} len={} preview={:02x?}",
                                    frame_type,
                                    payload_len,
                                    payload.iter().take(16).copied().collect::<Vec<_>>()
                                );
                            }
                            rx_buf.clear();
                        }
                        UnwrapResult::Error(e) => {
                            log::warn!("HAN parse error: {:?}", e);
                            rx_buf.clear();
                        }
                    }
                }
            }
            Err(e) => {
                log::trace!("UART read: {:?}", e);
            }
        }
    }
}
