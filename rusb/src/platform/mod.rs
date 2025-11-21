#[cfg(any(target_os = "linux", target_os = "android"))]
pub mod linux;
#[cfg(any(target_os = "linux", target_os = "android"))]
pub use self::linux::{
    LinuxDevice as Device, LinuxDeviceHandle as DeviceHandle, attach_kernel_driver, bulk_transfer,
    claim_interface, clear_halt, control_transfer, detach_kernel_driver, devices,
    get_device_descriptor, interrupt_transfer, open, release_interface, reset_device,
    set_interface_alt_setting,
};

#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "windows")]
pub use self::windows::{
    WindowsDevice as Device, WindowsDeviceHandle as DeviceHandle, attach_kernel_driver,
    bulk_transfer, claim_interface, clear_halt, control_transfer, detach_kernel_driver, devices,
    get_device_descriptor, interrupt_transfer, open, release_interface, reset_device,
    set_interface_alt_setting,
};

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "macos")]
pub use self::macos::{
    MacosDevice as Device, MacosDeviceHandle as DeviceHandle, attach_kernel_driver, bulk_transfer,
    claim_interface, clear_halt, control_transfer, detach_kernel_driver, devices,
    get_device_descriptor, interrupt_transfer, open, release_interface, reset_device,
    set_interface_alt_setting,
};

#[cfg(all(target_arch = "wasm32", feature = "webusb"))]
pub mod wasm;
#[cfg(all(target_arch = "wasm32", feature = "webusb-threads"))]
pub use self::wasm::init_thread_pool;
#[cfg(all(target_arch = "wasm32", feature = "webusb"))]
pub use self::wasm::{
    WasmDevice as Device, WasmDeviceHandle as DeviceHandle, attach_kernel_driver, bulk_transfer,
    claim_interface, clear_halt, control_transfer, detach_kernel_driver, devices,
    get_device_descriptor, interrupt_transfer, open, release_interface, reset_device,
    set_interface_alt_setting,
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
    NotSupportedDevice as Device, NotSupportedDeviceHandle as DeviceHandle, attach_kernel_driver,
    bulk_transfer, claim_interface, clear_halt, control_transfer, detach_kernel_driver, devices,
    get_device_descriptor, interrupt_transfer, open, release_interface, reset_device,
    set_interface_alt_setting,
};
