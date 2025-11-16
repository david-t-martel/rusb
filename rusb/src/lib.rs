//! A safe, native Rust wrapper for USB device access.
#![warn(missing_docs)]

// Platform-specific backend implementations.
// The appropriate module will be selected at compile time.
mod platform;

/// A handle to a USB device.
pub struct DeviceHandle {
    #[cfg(target_os = "linux")]
    inner: platform::linux::LinuxDeviceHandle,
    #[cfg(target_os = "windows")]
    inner: platform::windows::WindowsDeviceHandle,
    #[cfg(target_os = "macos")]
    inner: platform::macos::MacosDeviceHandle,
    #[cfg(all(target_arch = "wasm32", feature = "webusb"))]
    inner: platform::wasm::WasmDeviceHandle,
    #[cfg(target_os = "android")]
    inner: platform::android::AndroidDeviceHandle,
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos", all(target_arch = "wasm32", feature = "webusb"), target_os = "android")))]
    inner: platform::not_supported::NotSupportedDeviceHandle,
}

impl DeviceHandle {
    // Methods for device I/O will go here.
}

/// A physical USB device.
pub struct Device {
    #[cfg(target_os = "linux")]
    inner: platform::linux::LinuxDevice,
    #[cfg(target_os = "windows")]
    inner: platform::windows::WindowsDevice,
    #[cfg(target_os = "macos")]
    inner: platform::macos::MacosDevice,
    #[cfg(all(target_arch = "wasm32", feature = "webusb"))]
    inner: platform::wasm::WasmDevice,
    #[cfg(target_os = "android")]
    inner: platform::android::AndroidDevice,
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos", all(target_arch = "wasm32", feature = "webusb"), target_os = "android")))]
    inner: platform::not_supported::NotSupportedDevice,
}

impl Device {
    /// Opens the device to get a `DeviceHandle`.
    pub fn open(&self) -> Result<DeviceHandle, Error> {
        platform::open(self)
    }

    /// Reads the device descriptor.
    pub fn get_device_descriptor(&self) -> Result<DeviceDescriptor, Error> {
        platform::get_device_descriptor(self)
    }
}

/// A list of all attached USB devices.
pub struct DeviceList {
    devices: Vec<Device>,
}

impl DeviceList {
    /// Returns an iterator over the devices.
    pub fn iter(&self) -> impl Iterator<Item = &Device> {
        self.devices.iter()
    }
}

/// Enumerates all attached USB devices.
pub fn devices() -> Result<DeviceList, Error> {
    platform::devices()
}

/// The primary error type for this library.
#[derive(Debug)]
pub enum Error {
    /// The requested operation is not supported on the current platform.
    NotSupported,
    /// The underlying OS returned an error.
    Os(i32),
    // Other specific errors will be added here.
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotSupported => write!(f, "Operation not supported on this platform"),
            Error::Os(err) => write!(f, "An OS-level error occurred: {}", err),
        }
    }
}

impl std::error::Error for Error {}

#[cfg(target_os = "windows")]
impl From<windows::core::Error> for Error {
    fn from(err: windows::core::Error) -> Self {
        Error::Os(err.code().0)
    }
}

/// A USB device descriptor.
#[derive(Debug)]
pub struct DeviceDescriptor {
    /// The length of the descriptor.
    pub length: u8,
    /// The descriptor type.
    pub descriptor_type: u8,
    /// The USB specification release number.
    pub usb_version: u16,
    /// The device class.
    pub device_class: u8,
    /// The device subclass.
    pub device_subclass: u8,
    /// The device protocol.
    pub device_protocol: u8,
    /// The maximum packet size for endpoint 0.
    pub max_packet_size_0: u8,
    /// The vendor ID.
    pub vendor_id: u16,
    /// The product ID.
    pub product_id: u16,
    /// The device release number.
    pub device_version: u16,
    /// The index of the manufacturer string descriptor.
    pub manufacturer_string_index: u8,
    /// The index of the product string descriptor.
    pub product_string_index: u8,
    /// The index of the serial number string descriptor.
    pub serial_number_string_index: u8,
    /// The number of possible configurations.
    pub num_configurations: u8,
}
