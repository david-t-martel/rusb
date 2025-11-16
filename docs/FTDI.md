# FTDI Integration Notes

## Overview

The `rusb::support::ftdi` module provides a minimal, dependency-free helper for
FT232/FT2232 style bridges. It exposes the most common primitives:

- Automatic discovery of VID/PID pairs (`0x0403:0x6001`, `0x6010`, `0x6011`, `0x6014`).
- Control transfers for RESET, baud-rate configuration, line settings, and flow
  control.
- Thin wrappers around the bulk IN/OUT endpoints.

## Usage

```rust,no_run
use rusb::support::ftdi::FtdiDevice;

fn main() -> Result<(), rusb::Error> {
    let dev = FtdiDevice::open_first()?;
    dev.set_baud_rate(115_200)?;
    dev.configure_line(0x0008)?; // 8N1
    dev.write(b"hello world\n")?;
    let mut buf = [0u8; 64];
    let _n = dev.read(&mut buf)?;
    Ok(())
}
```

## Extensibility

The helper focuses on synchronous transfers. For high-throughput or asynchronous
usage consider:

- Pooling multiple reader threads to keep RX FIFOs drained.
- Exposing event/bitmode requests for bit-bang GPIO control.
- Adding latency timer configuration (control request 0x09).

The module intentionally keeps the low-level request encoding visible so it can
serve as a reference for downstream crates.
