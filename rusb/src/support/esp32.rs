//! Helpers for ESP32 devices that expose USB-CDC or native USB serial bridges.
//! Many ESP32 development boards rely on CP210x/FTDI bridges, but newer boards
//! (ESP32-S3, ESP32-C3) expose native USB CDC ACM interfaces which can be
//! accessed directly via `rusb`.

use crate::{
    ControlRequest, ControlTransferData, Device, DeviceHandle, DeviceList, Error, TransferBuffer,
};
use std::time::Duration;

const ESPRESSIF_VID: u16 = 0x303A;
const CDC_SET_LINE_CODING: u8 = 0x20;
const CDC_SET_CONTROL_LINE_STATE: u8 = 0x22;
const USB_CLASS_REQUEST_OUT: u8 = 0x21;

/// Serial bridge for native ESP32-Sx USB CDC interfaces.
pub struct Esp32SerialBridge {
    handle: DeviceHandle,
    in_ep: u8,
    out_ep: u8,
    interface: u8,
}

impl Esp32SerialBridge {
    /// Opens the first USB CDC interface with the Espressif vendor ID.
    pub fn open_first() -> Result<Self, Error> {
        let list = crate::devices()?;
        Self::open_from_list(&list)
    }

    pub fn open_from_list(list: &DeviceList) -> Result<Self, Error> {
        for dev in list.iter() {
            if let Ok(desc) = dev.get_device_descriptor() {
                if desc.vendor_id == ESPRESSIF_VID {
                    return Self::open_device(dev, 0x81, 0x02, 0);
                }
            }
        }
        Err(Error::NotSupported)
    }

    pub fn open_device(
        device: &Device,
        in_ep: u8,
        out_ep: u8,
        interface: u8,
    ) -> Result<Self, Error> {
        let handle = device.open()?;
        let bridge = Self {
            handle,
            in_ep,
            out_ep,
            interface,
        };
        bridge.set_control_lines(true, true)?;
        Ok(bridge)
    }

    /// Configures baud rate/format using the USB CDC ACM `SET_LINE_CODING` request.
    pub fn set_line_coding(
        &self,
        baud: u32,
        stop_bits: u8,
        parity: u8,
        data_bits: u8,
    ) -> Result<(), Error> {
        let mut payload = [0u8; 7];
        payload[0..4].copy_from_slice(&baud.to_le_bytes());
        payload[4] = stop_bits;
        payload[5] = parity;
        payload[6] = data_bits;
        let request = ControlRequest {
            request_type: USB_CLASS_REQUEST_OUT,
            request: CDC_SET_LINE_CODING,
            value: 0,
            index: self.interface as u16,
        };
        self.handle
            .control_transfer(
                request,
                ControlTransferData::Out(&payload),
                Duration::from_millis(100),
            )
            .map(|_| ())
    }

    /// Asserts or de-asserts RTS/DTR lines via `SET_CONTROL_LINE_STATE`.
    pub fn set_control_lines(&self, dtr: bool, rts: bool) -> Result<(), Error> {
        let mut value = 0u16;
        if dtr {
            value |= 0x1;
        }
        if rts {
            value |= 0x2;
        }
        let request = ControlRequest {
            request_type: USB_CLASS_REQUEST_OUT,
            request: CDC_SET_CONTROL_LINE_STATE,
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

    pub fn write(&self, data: &[u8]) -> Result<usize, Error> {
        self.handle.bulk_transfer(
            self.out_ep,
            TransferBuffer::Out(data),
            Duration::from_millis(500),
        )
    }

    pub fn read(&self, buf: &mut [u8]) -> Result<usize, Error> {
        self.handle.bulk_transfer(
            self.in_ep,
            TransferBuffer::In(buf),
            Duration::from_millis(500),
        )
    }
}
