//! A Rust wrapper for libusb.

mod platform;
pub mod support;

use std::fmt;
use std::time::Duration;

/// A list of USB devices.
pub struct DeviceList {
    devices: Vec<Device>,
}

impl DeviceList {
    /// Returns an iterator over the devices in the list.
    pub fn iter(&self) -> std::slice::Iter<'_, Device> {
        self.devices.iter()
    }
}

/// A USB device.
pub struct Device {
    inner: platform::Device,
}

impl Device {
    /// Opens the device.
    #[cfg(not(all(target_arch = "wasm32", feature = "webusb")))]
    pub fn open(&self) -> Result<DeviceHandle, Error> {
        platform::open(self)
    }

    /// Opens the device (WebUSB builds use async semantics).
    #[cfg(all(target_arch = "wasm32", feature = "webusb"))]
    pub async fn open(&self) -> Result<DeviceHandle, Error> {
        platform::open(self).await
    }

    /// Returns the device descriptor.
    pub fn get_device_descriptor(&self) -> Result<DeviceDescriptor, Error> {
        platform::get_device_descriptor(self)
    }
}

/// A handle to an open USB device.
pub struct DeviceHandle {
    inner: platform::DeviceHandle,
}

impl DeviceHandle {
    /// Performs a USB control transfer using the provided setup packet,
    /// payload buffer, and timeout.
    #[cfg(not(all(target_arch = "wasm32", feature = "webusb")))]
    pub fn control_transfer<'a>(
        &self,
        request: ControlRequest,
        data: ControlTransferData<'a>,
        timeout: Duration,
    ) -> Result<usize, Error> {
        platform::control_transfer(self, request, data, timeout)
    }

    /// Performs a USB bulk transfer on the given endpoint.
    #[cfg(not(all(target_arch = "wasm32", feature = "webusb")))]
    pub fn bulk_transfer<'a>(
        &self,
        endpoint: u8,
        buffer: TransferBuffer<'a>,
        timeout: Duration,
    ) -> Result<usize, Error> {
        platform::bulk_transfer(self, endpoint, buffer, timeout)
    }

    /// Performs a USB interrupt transfer on the given endpoint.
    #[cfg(not(all(target_arch = "wasm32", feature = "webusb")))]
    pub fn interrupt_transfer<'a>(
        &self,
        endpoint: u8,
        buffer: TransferBuffer<'a>,
        timeout: Duration,
    ) -> Result<usize, Error> {
        platform::interrupt_transfer(self, endpoint, buffer, timeout)
    }

    /// Performs a USB control transfer (async variant for WebUSB builds).
    #[cfg(all(target_arch = "wasm32", feature = "webusb"))]
    pub async fn control_transfer<'a>(
        &self,
        request: ControlRequest,
        data: ControlTransferData<'a>,
        timeout: Duration,
    ) -> Result<usize, Error> {
        platform::control_transfer(self, request, data, timeout).await
    }

    /// Performs a USB bulk transfer on the given endpoint (WebUSB builds).
    #[cfg(all(target_arch = "wasm32", feature = "webusb"))]
    pub async fn bulk_transfer<'a>(
        &self,
        endpoint: u8,
        buffer: TransferBuffer<'a>,
        timeout: Duration,
    ) -> Result<usize, Error> {
        platform::bulk_transfer(self, endpoint, buffer, timeout).await
    }

    /// Performs a USB interrupt transfer on the given endpoint (WebUSB builds).
    #[cfg(all(target_arch = "wasm32", feature = "webusb"))]
    pub async fn interrupt_transfer<'a>(
        &self,
        endpoint: u8,
        buffer: TransferBuffer<'a>,
        timeout: Duration,
    ) -> Result<usize, Error> {
        platform::interrupt_transfer(self, endpoint, buffer, timeout).await
    }
}

#[cfg(target_os = "linux")]
impl DeviceHandle {
    /// Returns the underlying usbfs file descriptor on Linux.
    pub fn as_raw_fd(&self) -> std::os::unix::io::RawFd {
        self.inner.as_raw_fd()
    }
}

/// Setup packet metadata for a USB control transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlRequest {
    pub request_type: u8,
    pub request: u8,
    pub value: u16,
    pub index: u16,
}

/// Data payload for a control transfer.
pub enum ControlTransferData<'a> {
    /// No payload stage.
    None,
    /// IN transfer (device-to-host) with mutable buffer to fill.
    In(&'a mut [u8]),
    /// OUT transfer (host-to-device) with read-only payload.
    Out(&'a [u8]),
}

impl<'a> ControlTransferData<'a> {
    pub(crate) fn len(&self) -> usize {
        match self {
            ControlTransferData::None => 0,
            ControlTransferData::In(buf) => buf.len(),
            ControlTransferData::Out(buf) => buf.len(),
        }
    }

    pub(crate) fn direction(&self) -> Option<TransferDirection> {
        match self {
            ControlTransferData::None => None,
            ControlTransferData::In(_) => Some(TransferDirection::In),
            ControlTransferData::Out(_) => Some(TransferDirection::Out),
        }
    }
}

/// Buffer wrapper for bulk and interrupt transfers.
pub enum TransferBuffer<'a> {
    In(&'a mut [u8]),
    Out(&'a [u8]),
}

impl<'a> TransferBuffer<'a> {
    pub(crate) fn len(&self) -> usize {
        match self {
            TransferBuffer::In(buf) => buf.len(),
            TransferBuffer::Out(buf) => buf.len(),
        }
    }

    pub(crate) fn direction(&self) -> TransferDirection {
        match self {
            TransferBuffer::In(_) => TransferDirection::In,
            TransferBuffer::Out(_) => TransferDirection::Out,
        }
    }
}

/// Logical direction of a USB transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferDirection {
    In,
    Out,
}

/// An error from the USB library.
#[derive(Debug)]
pub enum Error {
    /// An OS-level error.
    Os(i32),
    /// The operation is not supported.
    NotSupported,
    /// An unknown error.
    Unknown,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Os(code) => write!(f, "OS error {}", code),
            Error::NotSupported => write!(f, "Operation not supported"),
            Error::Unknown => write!(f, "Unknown error"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Os(err.raw_os_error().unwrap_or(-1))
    }
}

#[cfg(target_os = "windows")]
impl From<windows::core::Error> for Error {
    fn from(err: windows::core::Error) -> Self {
        Error::Os(err.code().0)
    }
}

/// A device descriptor.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct DeviceDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub usb_version: u16,
    pub device_class: u8,
    pub device_subclass: u8,
    pub device_protocol: u8,
    pub max_packet_size_0: u8,
    pub vendor_id: u16,
    pub product_id: u16,
    pub device_version: u16,
    pub manufacturer_string_index: u8,
    pub product_string_index: u8,
    pub serial_number_string_index: u8,
    pub num_configurations: u8,
}

/// Returns a list of all USB devices.
#[cfg(not(all(target_arch = "wasm32", feature = "webusb")))]
pub fn devices() -> Result<DeviceList, Error> {
    platform::devices()
}

/// Returns a list of all USB devices. (WebUSB builds)
#[cfg(all(target_arch = "wasm32", feature = "webusb"))]
pub async fn devices() -> Result<DeviceList, Error> {
    platform::devices().await
}

/// Initializes the WebUSB thread pool when `webusb-threads` is enabled.
#[cfg(all(target_arch = "wasm32", feature = "webusb-threads"))]
pub async fn init_webusb_threads(workers: Option<usize>) -> Result<(), Error> {
    platform::init_thread_pool(workers).await
}
