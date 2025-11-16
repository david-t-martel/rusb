//! A Rust wrapper for libusb.

mod platform;

use std::fmt;

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
    pub fn open(&self) -> Result<DeviceHandle, Error> {
        platform::open(self)
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
    /// Performs a control transfer.
    pub fn control_transfer(&self) -> Result<(), Error> {
        platform::control_transfer()
    }

    /// Performs a bulk transfer.
    pub fn bulk_transfer(&self) -> Result<(), Error> {
        platform::bulk_transfer()
    }

    /// Performs an interrupt transfer.
    pub fn interrupt_transfer(&self) -> Result<(), Error> {
        platform::interrupt_transfer()
    }
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
pub fn devices() -> Result<DeviceList, Error> {
    platform::devices()
}
