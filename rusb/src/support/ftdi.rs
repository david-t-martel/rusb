//! Convenience helpers for FTDI USB-to-serial adapters. The implementation
//! focuses on common FT232/FT2232 style bridges and relies exclusively on the
//! public `rusb` APIs so it can run anywhere our backends are supported.
//!
//! TODO: Add MPSSE helper functions for SPI/I2C/JTAG bitbanging
//! TODO: Add async variants of read/write methods
//! TODO: Add support for FT4232H and FT2232H high-speed modes
//! TODO: Add support for reading EEPROM configuration
//! TODO: Add support for FT-X series chips
//! TODO: Add proper DTR/RTS control methods
//! TODO: Add read timeout configuration separate from transfer timeout
//! TODO: Add methods to query chip type and capabilities

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
const FTDI_SIO_RESET_PURGE_RX: u16 = 1;
const FTDI_SIO_RESET_PURGE_TX: u16 = 2;
const FTDI_SIO_SET_LATENCY_TIMER_REQUEST: u8 = 9;
const FTDI_SIO_SET_BITMODE_REQUEST: u8 = 0x0B;
const USB_WRITE_REQUEST: u8 = 0x40;

/// Simple wrapper that owns the rusb handle plus endpoint metadata.
/// TODO: Add chip type detection (FT232R, FT2232H, FT4232H, etc.)
/// TODO: Cache baud rate and other settings to avoid redundant control transfers
/// TODO: Add buffer for read operations to handle FTDI status bytes
pub struct FtdiDevice {
    handle: DeviceHandle,
    in_ep: u8,
    out_ep: u8,
    interface: u8,
    // TODO: Add chip_type: FtdiChipType field
    // TODO: Add current_baud: Option<u32>
    // TODO: Add read_buffer: Vec<u8> for handling FTDI modem status bytes
}

/// Bit-bang operating modes supported by FTDI chips.
#[derive(Debug, Clone, Copy)]
pub enum BitMode {
    Reset = 0x00,
    BitBang = 0x01,
    Mpsse = 0x02,
    SyncBitBang = 0x04,
    MCUHost = 0x08,
    FastOpto = 0x10,
    CBusBitBang = 0x20,
    SyncFifo = 0x40,
}

/// Hardware/software flow control options.
#[derive(Debug, Clone, Copy)]
pub enum FlowControl {
    None,
    RtsCts,
    DtrDsr,
    XonXoff,
}

impl FtdiDevice {
    /// Searches the USB bus for a matching VID/PID and opens it.
    /// TODO: Add variant that takes VID/PID parameters for custom FTDI devices
    pub fn open_first() -> Result<Self, Error> {
        let list = crate::devices()?;
        Self::open_from_list(&list)
    }

    /// Tries to open the first FTDI adapter from an existing `DeviceList`.
    /// TODO: Return more specific error when no device found
    /// TODO: Add method to open by serial number
    pub fn open_from_list(list: &DeviceList) -> Result<Self, Error> {
        for dev in list.iter() {
            if let Ok(desc) = dev.get_device_descriptor() {
                if desc.vendor_id == FTDI_VID && DEFAULT_PIDS.contains(&desc.product_id) {
                    // TODO: Detect actual endpoint addresses from descriptors
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

    /// Enables/disables hardware/software flow control.
    pub fn set_flow_control(&self, mode: FlowControl) -> Result<(), Error> {
        let (mask, value) = match mode {
            FlowControl::None => (0, 0),
            FlowControl::RtsCts => (0, 0x0100),
            FlowControl::DtrDsr => (0, 0x0200),
            FlowControl::XonXoff => (0x1311, 0),
        };
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

    /// Adjusts the latency timer (1-255 ms). Lower values reduce buffering.
    pub fn set_latency_timer(&self, timer_ms: u8) -> Result<(), Error> {
        let value = timer_ms.max(1);
        let request = ControlRequest {
            request_type: USB_WRITE_REQUEST,
            request: FTDI_SIO_SET_LATENCY_TIMER_REQUEST,
            value: value as u16,
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

    /// Configures bit-bang mode for GPIO or MPSSE use cases.
    pub fn set_bit_mode(&self, mask: u8, mode: BitMode) -> Result<(), Error> {
        let request = ControlRequest {
            request_type: USB_WRITE_REQUEST,
            request: FTDI_SIO_SET_BITMODE_REQUEST,
            value: ((mode as u16) << 8) | mask as u16,
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

    /// Clears the RX FIFO.
    pub fn purge_rx(&self) -> Result<(), Error> {
        self.reset_pipe(FTDI_SIO_RESET_PURGE_RX)
    }

    /// Clears the TX FIFO.
    pub fn purge_tx(&self) -> Result<(), Error> {
        self.reset_pipe(FTDI_SIO_RESET_PURGE_TX)
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
    /// TODO: Strip FTDI modem status bytes (first 2 bytes of each packet)
    /// TODO: Add variant with configurable timeout
    /// TODO: Handle short reads properly
    pub fn read(&self, data: &mut [u8]) -> Result<usize, Error> {
        self.handle.bulk_transfer(
            self.in_ep,
            TransferBuffer::In(data),
            Duration::from_millis(500),
        )
    }

    // TODO: Add read_with_timeout() method
    // TODO: Add write_with_timeout() method
    // TODO: Add get_modem_status() method to read DTR/RTS/CTS/DSR/RI/DCD
    // TODO: Add set_dtr() and set_rts() methods
    // TODO: Add get_queue_status() to check bytes available
    // TODO: Add MPSSE mode helpers (write_mpsse, read_mpsse, etc.)
    // TODO: Add SPI helper methods for common operations
    // TODO: Add I2C helper methods for common operations

    fn reset_pipe(&self, pipe: u16) -> Result<(), Error> {
        let request = ControlRequest {
            request_type: USB_WRITE_REQUEST,
            request: FTDI_SIO_RESET_REQUEST,
            value: pipe,
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
}

fn compute_ftdi_divisor(baud: u32) -> Option<u32> {
    // TODO: Support different clock rates for FT2232H/FT4232H (60 MHz, 120 MHz)
    // TODO: Validate baud rate is achievable with reasonable error
    if baud == 0 {
        return None;
    }
    // FTDI base clock 3 MHz, fractional divisor encoded in 1/8 increments.
    let base_clock = 3_000_000u32;
    let divisor = ((base_clock << 3) / baud).max(1);
    let divisor = divisor.min(0x1FFFF);
    Some(divisor)
}

// TODO: Add tests for FTDI helper functions
// TODO: Add tests for baud rate calculation accuracy
// TODO: Add example program for basic UART communication
// TODO: Add example program for MPSSE mode (SPI/I2C)
// TODO: Document common pin configurations for different FTDI chips
