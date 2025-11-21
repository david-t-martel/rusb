//! A Rust wrapper for libusb.
//!
//! TODO: Add comprehensive module-level documentation with examples
//! TODO: Consider adding feature flags for optional functionality (async, hotplug, etc.)

mod platform;
pub mod support;

use std::fmt;
use std::time::Duration;

/// A list of USB devices.
/// TODO: Implement IntoIterator trait for ergonomic iteration
/// TODO: Add filtering methods (by_vid_pid, by_class, etc.)
pub struct DeviceList {
    devices: Vec<Device>,
}

impl DeviceList {
    /// Returns an iterator over the devices in the list.
    pub fn iter(&self) -> std::slice::Iter<'_, Device> {
        self.devices.iter()
    }

    // TODO: Add len() method to query number of devices
    // TODO: Add is_empty() method
}

/// A USB device.
/// TODO: Add methods to get bus number and device address
/// TODO: Add method to get device speed (low/full/high/super)
/// TODO: Add support for reading string descriptors (manufacturer, product, serial)
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

    // TODO: Add get_configuration_descriptor(config_index) method
    // TODO: Add get_active_configuration() method
    // TODO: Add reset() method to reset the device
}

/// A handle to an open USB device.
/// TODO: Add support for claiming/releasing interfaces
/// TODO: Add support for setting configuration
/// TODO: Add support for setting alternate interface settings
/// TODO: Add support for clearing halt condition on endpoints
/// TODO: Add support for reading string descriptors
/// TODO: Add support for detaching/attaching kernel drivers (Linux)
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

    /// Claims the interface with the given number.
    pub fn claim_interface(&self, interface: u8) -> Result<(), Error> {
        platform::claim_interface(self, interface)
    }

    /// Releases the interface with the given number.
    pub fn release_interface(&self, interface: u8) -> Result<(), Error> {
        platform::release_interface(self, interface)
    }

    /// Sets the alternate setting for the interface.
    pub fn set_interface_alt_setting(&self, interface: u8, alt_setting: u8) -> Result<(), Error> {
        platform::set_interface_alt_setting(self, interface, alt_setting)
    }

    /// Resets the device.
    pub fn reset_device(&self) -> Result<(), Error> {
        platform::reset_device(self)
    }

    /// Clears the halt condition on the endpoint.
    pub fn clear_halt(&self, endpoint: u8) -> Result<(), Error> {
        platform::clear_halt(self, endpoint)
    }

    /// Detaches the kernel driver from the interface.
    pub fn detach_kernel_driver(&self, interface: u8) -> Result<(), Error> {
        platform::detach_kernel_driver(self, interface)
    }

    /// Attaches the kernel driver to the interface.
    pub fn attach_kernel_driver(&self, interface: u8) -> Result<(), Error> {
        platform::attach_kernel_driver(self, interface)
    }

    /// Reads a string descriptor from the device.
    #[cfg(not(all(target_arch = "wasm32", feature = "webusb")))]
    pub fn read_string_descriptor(
        &self,
        index: u8,
        lang_id: u16,
        buffer: &mut [u8],
    ) -> Result<usize, Error> {
        let request = ControlRequest {
            request_type: 0x80, // Device to Host, Standard, Device
            request: 0x06,      // GET_DESCRIPTOR
            value: (0x03 << 8) | (index as u16),
            index: lang_id,
        };

        self.control_transfer(
            request,
            ControlTransferData::In(buffer),
            Duration::from_secs(1),
        )
    }

    /// Reads a string descriptor and converts it to an ASCII string.
    #[cfg(not(all(target_arch = "wasm32", feature = "webusb")))]
    pub fn read_string_descriptor_ascii(&self, index: u8) -> Result<String, Error> {
        let mut buf = [0u8; 255];
        let len = self.read_string_descriptor(index, 0x0409, &mut buf)?;

        if len < 2 {
            return Err(Error::Unknown);
        }

        let b_length = buf[0] as usize;
        let b_descriptor_type = buf[1];

        if b_descriptor_type != 0x03 {
            return Err(Error::Unknown);
        }

        // String is UTF-16LE
        let utf16_len = (b_length - 2) / 2;
        let mut utf16 = Vec::with_capacity(utf16_len);
        for i in 0..utf16_len {
            let lower = buf[2 + i * 2] as u16;
            let upper = buf[2 + i * 2 + 1] as u16;
            utf16.push(lower | (upper << 8));
        }

        String::from_utf16(&utf16).map_err(|_| Error::Unknown)
    }

    // TODO: Add isochronous transfer support for all platforms
    // TODO: Add async transfer submission API for better performance on native platforms
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
/// TODO: Add more specific error variants for better error handling:
/// - NoDevice (device disconnected)
/// - AccessDenied (permission issues)
/// - Busy (resource already in use)
/// - Timeout (operation timed out)
/// - Overflow (buffer overflow)
/// - Pipe (pipe/endpoint error)
/// - Interrupted (system call interrupted)
/// - InvalidParam (invalid parameter)
/// TODO: Implement proper Display messages that are user-friendly
/// TODO: Add context information to errors (which device, endpoint, etc.)
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
/// TODO: Add helper methods to interpret fields (e.g., usb_version_string())
/// TODO: Add PartialEq and Eq derives for comparison
/// TODO: Validate that length and descriptor_type are correct
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct DeviceDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub usb_version: u16,  // TODO: Store as BCD properly (major.minor.patch)
    pub device_class: u8,
    pub device_subclass: u8,
    pub device_protocol: u8,
    pub max_packet_size_0: u8,
    pub vendor_id: u16,
    pub product_id: u16,
    pub device_version: u16,  // TODO: Store as BCD properly
    pub manufacturer_string_index: u8,
    pub product_string_index: u8,
    pub serial_number_string_index: u8,
    pub num_configurations: u8,
}

// TODO: Add ConfigurationDescriptor struct
// TODO: Add InterfaceDescriptor struct
// TODO: Add EndpointDescriptor struct

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

// TODO: Add hotplug API for device arrival/removal notifications (all platforms)
// TODO: Add function to get library version
// TODO: Add context management for better resource cleanup and thread safety
// TODO: Add logging/tracing support for debugging USB issues
// TODO: Consider adding a builder pattern for DeviceHandle configuration
