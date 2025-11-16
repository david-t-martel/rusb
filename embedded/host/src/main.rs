use rusb::support::{esp32::Esp32SerialBridge, logger::ChannelLogger};
use std::fs::File;
use std::io::{self, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to ESP32 serial bridge...");
    let bridge = Esp32SerialBridge::open_first()?;
    let logger = ChannelLogger::new(bridge.into_handle(), 0x81, 0x02, File::create("serial.log")?);
    logger.write(b"AT+GMR\r\n")?;
    let mut buf = [0u8; 256];
    let n = logger.read(&mut buf)?;
    io::stdout().write_all(&buf[..n])?;
    Ok(())
}
