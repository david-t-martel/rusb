# ESP32 Native USB Serial

Modern ESP32-C3/ESP32-S3 boards expose a native USB-CDC interface (VID
`0x303A`). The `rusb::support::esp32` module wraps the class requests required to
configure line coding and handshake signals before falling back to the bulk
IN/OUT endpoints.

## Example

```rust,no_run
use rusb::support::esp32::Esp32SerialBridge;

fn main() -> Result<(), rusb::Error> {
    let bridge = Esp32SerialBridge::open_first()?;
    bridge.set_line_coding(921_600, 0, 0, 8)?; // 8N1
    bridge.write(b"AT+RST\r\n")?;
    let mut buf = [0u8; 128];
    let len = bridge.read(&mut buf)?;
    println!("reply: {:?}", &buf[..len]);
    Ok(())
}
```

The helper also exposes `set_control_lines` to toggle DTR/RTS—for example to
enter the ESP32 bootloader.

## Notes

- Many ESP32 boards still rely on external CP210x/FTDI bridges; use the FTDI
  helper (or write a Silicon Labs equivalent) for those targets.
- The helper is synchronous today; once the async transfer work lands it can be
  extended with Futures to match the WebUSB backend.
