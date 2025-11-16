//! Convenience helpers for FTDI USB-to-serial adapters. The implementation
//! focuses on common FT232/FT2232 style bridges and relies exclusively on the
//! public `rusb` APIs so it can run anywhere our backends are supported.

use crate::{
    ControlRequest, ControlTransferData, Device, DeviceHandle, DeviceList, Error, TransferBuffer,
};
use std::time::Duration;

/// Default FTDI vendor ID.
pub const FTDI_VID: u16 = 0x0403;
const DEFAULT_PIDS: &[u16] = &[0x6001, 0x6010, 0x6011, 0x6014];
const FTDI_SIO_SET_BAUDRATE_REQUEST: u8 = 3;
const FTDI_SIO_SET_DATA_REQUEST: u8 = 4;
const FTDI_SIO_SET_FLOW_CTRL_REQUEST: u8 = 2;
const FTDI_SIO_RESET_REQUEST: u8 = 0;
const USB_WRITE_REQUEST: u8 = 0x40;

/// Simple wrapper that owns the rusb handle plus endpoint metadata.
pub struct FtdiDevice {
    handle: DeviceHandle,
    in_ep: u8,
    out_ep: u8,
    interface: u8,
}

impl FtdiDevice {
    /// Searches the USB bus for a matching VID/PID and opens it.
    pub fn open_first() -> Result<Self, Error> {
        let list = crate::devices()?;
        Self::open_from_list(&list)
    }

    /// Tries to open the first FTDI adapter from an existing `DeviceList`.
    pub fn open_from_list(list: &DeviceList) -> Result<Self, Error> {
        for dev in list.iter() {
            if let Ok(desc) = dev.get_device_descriptor() {
                if desc.vendor_id == FTDI_VID && DEFAULT_PIDS.contains(&desc.product_id) {
                    return Self::open_device(dev, 0x81, 0x02, 0);
                }
            }
        }
        Err(Error::NotSupported)
    }

    /// Creates an `FtdiDevice` from a known `Device` and endpoint assignments.
    pub fn open_device(
        device: &Device,
        in_ep: u8,
        out_ep: u8,
        interface: u8,
    ) -> Result<Self, Error> {
        let handle = device.open()?;
        let ftdi = Self {
            handle,
            in_ep,
            out_ep,
            interface,
        };
        ftdi.reset()?;
        Ok(ftdi)
    }

    /// Issues the FTDI reset request (purge RX/TX FIFOs).
    pub fn reset(&self) -> Result<(), Error> {
        let request = ControlRequest {
            request_type: USB_WRITE_REQUEST,
            request: FTDI_SIO_RESET_REQUEST,
            value: 0,
            index: self.interface as u16,
        };
        self.handle
            .control_transfer(
                request,
                ControlTransferData::None,
                Duration::from_millis(100),
            )
            .map(|_| ())
    }

    /// Sets the UART baud rate. The simplified divider algorithm mirrors libftdi.
    pub fn set_baud_rate(&self, baud: u32) -> Result<(), Error> {
        let divisor = compute_ftdi_divisor(baud).ok_or(Error::NotSupported)?;
        let value = (divisor & 0xFFFF) as u16;
        let index = ((divisor >> 16) & 0xFFFF) as u16 | ((self.interface as u16) << 8);
        let request = ControlRequest {
            request_type: USB_WRITE_REQUEST,
            request: FTDI_SIO_SET_BAUDRATE_REQUEST,
            value,
            index,
        };
        self.handle
            .control_transfer(
                request,
                ControlTransferData::None,
                Duration::from_millis(100),
            )
            .map(|_| ())
    }

    /// Configures word length/parity/stop bits. Parameters are libftdi-style values.
    pub fn configure_line(&self, value: u16) -> Result<(), Error> {
        let request = ControlRequest {
            request_type: USB_WRITE_REQUEST,
            request: FTDI_SIO_SET_DATA_REQUEST,
            value,
            index: self.interface as u16,
        };
        self.handle
            .control_transfer(
                request,
                ControlTransferData::None,
                Duration::from_millis(100),
            )
            .map(|_| ())
    }

    /// Enables/disables RTS/CTS or DTR/DSR flow control.
    pub fn set_flow_control(&self, mask: u16, value: u16) -> Result<(), Error> {
        let request = ControlRequest {
            request_type: USB_WRITE_REQUEST,
            request: FTDI_SIO_SET_FLOW_CTRL_REQUEST,
            value: mask,
            index: ((value as u32) | ((self.interface as u32) << 8)) as u16,
        };
        self.handle
            .control_transfer(
                request,
                ControlTransferData::None,
                Duration::from_millis(100),
            )
            .map(|_| ())
    }

    /// Writes data to the OUT endpoint.
    pub fn write(&self, data: &[u8]) -> Result<usize, Error> {
        self.handle.bulk_transfer(
            self.out_ep,
            TransferBuffer::Out(data),
            Duration::from_millis(500),
        )
    }

    /// Reads into the provided buffer from the IN endpoint.
    pub fn read(&self, data: &mut [u8]) -> Result<usize, Error> {
        self.handle.bulk_transfer(
            self.in_ep,
            TransferBuffer::In(data),
            Duration::from_millis(500),
        )
    }
}

fn compute_ftdi_divisor(baud: u32) -> Option<u32> {
    if baud == 0 {
        return None;
    }
    // FTDI base clock 3 MHz, fractional divisor encoded in 1/8 increments.
    let base_clock = 3_000_000u32;
    let divisor = ((base_clock << 3) / baud).max(1);
    let divisor = divisor.min(0x1FFFF);
    Some(divisor)
}
