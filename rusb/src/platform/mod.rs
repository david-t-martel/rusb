#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "linux")]
pub use self::linux::{bulk_transfer, control_transfer, devices, get_device_descriptor, interrupt_transfer, open, LinuxDevice as Device, LinuxDeviceHandle as DeviceHandle};

#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "windows")]
pub use self::windows::{bulk_transfer, control_transfer, devices, get_device_descriptor, interrupt_transfer, open, WindowsDevice as Device, WindowsDeviceHandle as DeviceHandle};

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "macos")]
pub use self::macos::{bulk_transfer, control_transfer, devices, get_device_descriptor, interrupt_transfer, open, MacosDevice as Device, MacosDeviceHandle as DeviceHandle};

#[cfg(all(target_arch = "wasm32", feature = "webusb"))]
pub mod wasm;
#[cfg(all(target_arch = "wasm32", feature = "webusb"))]
pub use self::wasm::{bulk_transfer, control_transfer, devices, get_device_descriptor, interrupt_transfer, open, WasmDevice as Device, WasmDeviceHandle as DeviceHandle};

#[cfg(target_os = "android")]
pub mod android;
#[cfg(target_os = "android")]
pub use self::android::{bulk_transfer, control_transfer, devices, get_device_descriptor, interrupt_transfer, open, AndroidDevice as Device, AndroidDeviceHandle as DeviceHandle};

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos", all(target_arch = "wasm32", feature = "webusb"), target_os = "android")))]
pub mod not_supported;
#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos", all(target_arch = "wasm32", feature = "webusb"), target_os = "android")))]
pub use self::not_supported::{bulk_transfer, control_transfer, devices, get_device_descriptor, interrupt_transfer, open, NotSupportedDevice as Device, NotSupportedDeviceHandle as DeviceHandle};