# Serial Logger Helper

`rusb::support::logger::ChannelLogger` wraps a `DeviceHandle` and logs every bulk
IN/OUT transfer (timestamp + hex dump) to any `Write` sink (stdout, file, etc.).
This is useful for debugging bootloaders or CDC conversations.

Example (wrapping the ESP32 bridge):

```rust,no_run
use rusb::support::{esp32::Esp32SerialBridge, logger::ChannelLogger};
use std::fs::File;

fn main() -> Result<(), rusb::Error> {
    let bridge = Esp32SerialBridge::open_first()?;
    let logger = ChannelLogger::new(bridge.into_handle(), 0x81, 0x02, File::create("serial.log")?);
    logger.write(b"AT+RST\r\n")?;
    let mut buf = [0u8; 128];
    logger.read(&mut buf)?;
    Ok(())
}
```

Because it owns the `DeviceHandle`, you can integrate it with any support module
by calling `into_handle()` and recreating the helper afterwards.
