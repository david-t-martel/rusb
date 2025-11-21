//! Helpers for interacting with STM32 devices that expose DFU or CDC/ACM
//! interfaces over USB. This is intentionally lightweight so downstream tools
//! can extend it with board-specific commands.
//!
//! TODO: Add support for DFU file format parsing (.dfu files)
//! TODO: Add support for reading device protection status
//! TODO: Add support for setting/clearing read protection
//! TODO: Add support for DFU_CLRSTATUS command
//! TODO: Add async variants of blocking operations
//! TODO: Add progress callback for flash operations
//! TODO: Add verification after flashing
//! TODO: Add support for STM32 unique ID reading

use crate::{
    ControlRequest, ControlTransferData, Device, DeviceHandle, DeviceList, Error, TransferBuffer,
};
use std::thread;
use std::time::{Duration, Instant};

const STM32_VID: u16 = 0x0483; // STMicroelectronics
const DFU_PID: u16 = 0xDF11;
const USB_TYPE_CLASS_INTERFACE_OUT: u8 = 0x21;
const USB_TYPE_CLASS_INTERFACE_IN: u8 = 0xA1;

/// High-level access to a DFU-capable STM32 bootloader.
/// TODO: Add device type detection (STM32F1, F4, L4, H7, etc.)
/// TODO: Cache DFU status to avoid redundant queries
/// TODO: Track flash state machine
pub struct Stm32DfuDevice {
    handle: DeviceHandle,
    interface: u8,
    // TODO: Add device_type: Option<Stm32DeviceType>
    // TODO: Add last_status: Option<DfuStatus>
}

impl Stm32DfuDevice {
    /// Opens the first DFU interface it can find.
    pub fn open_first() -> Result<Self, Error> {
        let list = crate::devices()?;
        Self::open_from_list(&list)
    }

    pub fn open_from_list(list: &DeviceList) -> Result<Self, Error> {
        for dev in list.iter() {
            if let Ok(desc) = dev.get_device_descriptor() {
                if desc.vendor_id == STM32_VID && desc.product_id == DFU_PID {
                    return Self::open_device(dev, 0);
                }
            }
        }
        Err(Error::NotSupported)
    }

    pub fn open_device(device: &Device, interface: u8) -> Result<Self, Error> {
        let handle = device.open()?;
        Ok(Self { handle, interface })
    }

    /// Sends DFU_DETACH so the MCU can reset back into firmware.
    pub fn detach(&self, timeout_ms: u16) -> Result<(), Error> {
        let request = ControlRequest {
            request_type: USB_TYPE_CLASS_INTERFACE_OUT,
            request: 0, // DFU_DETACH
            value: timeout_ms,
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

    /// Issues DFU_DNLOAD for the given block number.
    pub fn download_block(&self, block_num: u16, payload: &[u8]) -> Result<(), Error> {
        let request = ControlRequest {
            request_type: USB_TYPE_CLASS_INTERFACE_OUT,
            request: 1, // DFU_DNLOAD
            value: block_num,
            index: self.interface as u16,
        };
        self.handle
            .control_transfer(
                request,
                ControlTransferData::Out(payload),
                Duration::from_secs(1),
            )
            .map(|_| ())
    }

    /// Reads a block via DFU_UPLOAD.
    pub fn upload_block(&self, block_num: u16, buf: &mut [u8]) -> Result<usize, Error> {
        let request = ControlRequest {
            request_type: USB_TYPE_CLASS_INTERFACE_IN,
            request: 2, // DFU_UPLOAD
            value: block_num,
            index: self.interface as u16,
        };
        self.handle.control_transfer(
            request,
            ControlTransferData::In(buf),
            Duration::from_secs(1),
        )
    }

    /// Polls the status via DFU_GETSTATUS.
    /// TODO: Parse status buffer into a proper struct
    /// TODO: Return DfuStatus with state, status, poll_timeout fields
    pub fn get_status(&self, buf: &mut [u8; 6]) -> Result<(), Error> {
        let request = ControlRequest {
            request_type: USB_TYPE_CLASS_INTERFACE_IN,
            request: 3,
            value: 0,
            index: self.interface as u16,
        };
        self.handle
            .control_transfer(
                request,
                ControlTransferData::In(buf),
                Duration::from_millis(100),
            )
            .map(|_| ())
    }

    // TODO: Add get_state() method (DFU_GETSTATE command)
    // TODO: Add abort() method (DFU_ABORT command)
    // TODO: Add clear_status() method (DFU_CLRSTATUS command)

    /// Waits until the device reports it is ready or a timeout occurs.
    pub fn wait_while_busy(&self, timeout: Duration) -> Result<(), Error> {
        let start = Instant::now();
        let mut buf = [0u8; 6];
        loop {
            self.get_status(&mut buf)?;
            if buf[0] == 0 {
                return Ok(());
            }
            if start.elapsed() > timeout {
                return Err(Error::Unknown);
            }
            let poll_timeout = u32::from_le_bytes([buf[1], buf[2], buf[3], 0]);
            thread::sleep(Duration::from_millis(poll_timeout as u64));
        }
    }

    /// Issues the STM32 DFU mass erase command sequence.
    pub fn mass_erase(&self) -> Result<(), Error> {
        // DFU suffix 0x41 0x00 triggers mass erase on STM32 DFUSE bootloaders.
        self.download_block(0, &[0x41, 0x00])?;
        self.wait_while_busy(Duration::from_secs(5))
    }

    /// Convenience wrapper for detach + small delay.
    pub fn leave_dfu(&self) -> Result<(), Error> {
        self.detach(1000)?;
        thread::sleep(Duration::from_millis(1200));
        Ok(())
    }
}

/// Minimal CDC/ACM helper for STLink virtual COM ports.
/// TODO: Add line coding configuration (baud rate, stop bits, parity, data bits)
/// TODO: Add control line state management (DTR, RTS)
/// TODO: Add break signal support
pub struct Stm32VirtualCom {
    handle: DeviceHandle,
    in_ep: u8,
    out_ep: u8,
}

impl Stm32VirtualCom {
    pub fn new(handle: DeviceHandle, in_ep: u8, out_ep: u8) -> Self {
        Self {
            handle,
            in_ep,
            out_ep,
        }
    }

    pub fn write(&self, data: &[u8]) -> Result<usize, Error> {
        self.handle.bulk_transfer(
            self.out_ep,
            TransferBuffer::Out(data),
            Duration::from_millis(200),
        )
    }

    pub fn read(&self, buf: &mut [u8]) -> Result<usize, Error> {
        self.handle.bulk_transfer(
            self.in_ep,
            TransferBuffer::In(buf),
            Duration::from_millis(200),
        )
    }

    // TODO: Add set_line_coding() method
    // TODO: Add set_control_line_state() method
    // TODO: Add get_line_coding() method
    // TODO: Add send_break() method
}

// TODO: Add tests for DFU operations
// TODO: Add tests for virtual COM operations
// TODO: Add example program for DFU flashing
// TODO: Add support for parsing DFU file format
// TODO: Document supported STM32 bootloader versions and their capabilities
// TODO: Add helper to detect STM32 chip ID and flash size
