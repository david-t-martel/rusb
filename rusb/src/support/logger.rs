//! Simple logging facade for serial-style transfers. Wraps any `DeviceHandle`
//! and records timestamped TX/RX frames to an arbitrary `Write` sink.

use crate::{
    ControlRequest, ControlTransferData, DeviceHandle, Error, TransferBuffer,
};
use std::io::{Result as IoResult, Write};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Logs bulk transfers on a pair of endpoints.
pub struct ChannelLogger<W: Write> {
    handle: DeviceHandle,
    in_ep: u8,
    out_ep: u8,
    sink: Mutex<W>,
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
            Duration::from_millis(500),
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
            Duration::from_millis(500),
        )?;
        self.log_frame("RX", &buf[..len])
            .map_err(|_| Error::Unknown)?;
        Ok(len)
    }

    pub fn control_transfer(
        &self,
        request: ControlRequest,
        data: ControlTransferData<'_>,
        timeout: Duration,
    ) -> Result<usize, Error> {
        let res = self.handle.control_transfer(request, data, timeout);

        if let Ok(mut sink) = self.sink.lock() {
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default();
            write!(
                sink,
                "[CTRL] {}.{:03}: ReqType:{:02x} Req:{:02x} Val:{:04x} Idx:{:04x} ",
                ts.as_secs(),
                ts.subsec_millis(),
                request.request_type,
                request.request,
                request.value,
                request.index
            )
            .ok();

            match &res {
                Ok(len) => writeln!(sink, "-> OK ({} bytes)", len).ok(),
                Err(e) => writeln!(sink, "-> ERR {:?}", e).ok(),
            };
        }

        res
    }

    pub fn into_handle(self) -> DeviceHandle {
        self.handle
    }

    fn log_frame(&self, label: &str, data: &[u8]) -> IoResult<()> {
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
}
