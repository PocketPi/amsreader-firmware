//! AMS Reader — ESP32 firmware entry point (Rust, `esp-idf-svc`).

fn main() {
    esp_idf_svc::sys::link_patches();

    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("AMS Reader (Rust) — ESP32");
    log::info!(
        "CRC self-check: {:04x}",
        ams_core::crc16_x25(&[0x7e, 0xa0, 0x00, 0x01])
    );
}
