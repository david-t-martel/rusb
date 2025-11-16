#[cfg(any(target_os = "linux", target_os = "android"))]
pub mod linux;
#[cfg(any(target_os = "linux", target_os = "android"))]
pub use self::linux::{
    LinuxDevice as Device, LinuxDeviceHandle as DeviceHandle, bulk_transfer, control_transfer,
    devices, get_device_descriptor, interrupt_transfer, open,
};

#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "windows")]
pub use self::windows::{
    WindowsDevice as Device, WindowsDeviceHandle as DeviceHandle, bulk_transfer, control_transfer,
    devices, get_device_descriptor, interrupt_transfer, open,
};

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "macos")]
pub use self::macos::{
    MacosDevice as Device, MacosDeviceHandle as DeviceHandle, bulk_transfer, control_transfer,
    devices, get_device_descriptor, interrupt_transfer, open,
};

#[cfg(all(target_arch = "wasm32", feature = "webusb"))]
pub mod wasm;
#[cfg(all(target_arch = "wasm32", feature = "webusb"))]
pub use self::wasm::{
    WasmDevice as Device, WasmDeviceHandle as DeviceHandle, bulk_transfer, control_transfer,
    devices, get_device_descriptor, interrupt_transfer, open,
};

#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_os = "windows",
    target_os = "macos",
    all(target_arch = "wasm32", feature = "webusb")
)))]
pub mod not_supported;
#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_os = "windows",
    target_os = "macos",
    all(target_arch = "wasm32", feature = "webusb")
)))]
pub use self::not_supported::{
    NotSupportedDevice as Device, NotSupportedDeviceHandle as DeviceHandle, bulk_transfer,
    control_transfer, devices, get_device_descriptor, interrupt_transfer, open,
};
