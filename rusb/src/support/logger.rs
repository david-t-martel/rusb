//! Simple logging facade for serial-style transfers.  Wraps any `DeviceHandle`
//! and records timestamped TX/RX frames to an arbitrary `Write` sink.
//!
//! TODO: Add support for logging control transfers
//! TODO: Add support for logging interrupt transfers
//! TODO: Add configurable log format (hex, ascii, mixed)
//! TODO: Add support for filtering by direction
//! TODO: Add support for pcap/pcapng output format for Wireshark
//! TODO: Add timestamps with microsecond precision
//! TODO: Add frame numbering for tracking sequence

use crate::{DeviceHandle, Error, TransferBuffer};
use std::io::{Result as IoResult, Write};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Logs bulk transfers on a pair of endpoints.
/// TODO: Add configuration options (timestamp format, hex format, etc.)
/// TODO: Add frame counter
/// TODO: Support multiple log sinks
pub struct ChannelLogger<W: Write> {
    handle: DeviceHandle,
    in_ep: u8,
    out_ep: u8,
    sink: Mutex<W>,
    // TODO: Add config: LogConfig
    // TODO: Add frame_counter: AtomicU64
}

impl<W: Write> ChannelLogger<W> {
    pub fn new(handle: DeviceHandle, in_ep: u8, out_ep: u8, sink: W) -> Self {
        Self {
            handle,
            in_ep,
            out_ep,
            sink: Mutex::new(sink),
        }
    }

    /// Writes data while capturing a timestamp and hex dump in the log sink.
    pub fn write(&self, data: &[u8]) -> Result<usize, Error> {
        let written = self.handle.bulk_transfer(
            self.out_ep,
            TransferBuffer::Out(data),
            std::time::Duration::from_millis(500),
        )?;
        self.log_frame("TX", &data[..written])
            .map_err(|_| Error::Unknown)?;
        Ok(written)
    }

    /// Reads data and logs the captured bytes.
    pub fn read(&self, buf: &mut [u8]) -> Result<usize, Error> {
        let len = self.handle.bulk_transfer(
            self.in_ep,
            TransferBuffer::In(buf),
            std::time::Duration::from_millis(500),
        )?;
        self.log_frame("RX", &buf[..len])
            .map_err(|_| Error::Unknown)?;
        Ok(len)
    }

    pub fn into_handle(self) -> DeviceHandle {
        self.handle
    }

    fn log_frame(&self, label: &str, data: &[u8]) -> IoResult<()> {
        // TODO: Make format configurable (hex, ascii, mixed)
        // TODO: Add frame counter
        // TODO: Use high-precision timestamps
        let mut sink = self.sink.lock().expect("logger poisoned");
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        write!(
            sink,
            "[{label}] {}.{:03}: ",
            ts.as_secs(),
            ts.subsec_millis()
        )?;
        for byte in data {
            write!(sink, "{byte:02X} ")?;
        }
        writeln!(sink)?;
        Ok(())
    }

    // TODO: Add log_control_transfer() method
    // TODO: Add set_format() method to change output format
}

// TODO: Add tests for logger functionality
// TODO: Add example program demonstrating logging
// TODO: Add support for async logging to avoid blocking transfers
// TODO: Add pcap/pcapng writer for protocol analysis in Wireshark
