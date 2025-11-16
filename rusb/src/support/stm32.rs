//! Helpers for interacting with STM32 devices that expose DFU or CDC/ACM
//! interfaces over USB. This is intentionally lightweight so downstream tools
//! can extend it with board-specific commands.

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
pub struct Stm32DfuDevice {
    handle: DeviceHandle,
    interface: u8,
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
}
