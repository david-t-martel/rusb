use rusb::support::esp32::Esp32SerialBridge;
use std::env;
use std::error::Error;
use std::fs;
use std::time::Instant;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let image_path = match args.next() {
        Some(path) => path,
        None => {
            eprintln!("usage: cargo run --example esp32_s3_flash <firmware.bin>");
            return Ok(());
        }
    };
    let chunk = env::var("ESP32_CHUNK")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1024usize);
    let image = fs::read(&image_path)?;
    println!("Loaded {} bytes", image.len());

    let bridge = Esp32SerialBridge::open_first()?;
    println!("Opened ESP32 CDC interface; toggling boot pins...");
    bridge.enter_bootloader_sequence()?;
    bridge.set_line_coding(921_600, 0, 0, 8)?;

    let start = Instant::now();
    for (i, chunk_data) in image.chunks(chunk).enumerate() {
        let address = (i * chunk) as u32;
        bridge.write_flash_block(address, chunk_data)?;
        if i % 10 == 0 {
            println!("Wrote block {} at 0x{:08x}", i, address);
        }
    }
    println!(
        "Transfer complete in {:.2?}. Resetting to firmware...",
        start.elapsed()
    );
    bridge.set_control_lines(false, false)?;
    Ok(())
}
