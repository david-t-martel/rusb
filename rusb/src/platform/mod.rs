#[cfg(target_os = "linux")]
#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "linux")]
pub use self::linux::{devices, open, get_device_descriptor};

#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "windows")]
pub use self::windows::{devices, open, get_device_descriptor};

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "macos")]
pub use self::macos::{devices, open, get_device_descriptor};

#[cfg(all(target_arch = "wasm32", feature = "webusb"))]
pub mod wasm;
#[cfg(all(target_arch = "wasm32", feature = "webusb"))]
pub use self::wasm::{devices, open, get_device_descriptor};

#[cfg(target_os = "android")]
pub mod android;
#[cfg(target_os = "android")]
pub use self::android::{devices, open, get_device_descriptor};

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos", all(target_arch = "wasm32", feature = "webusb"), target_os = "android")))]
pub mod not_supported;
#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos", all(target_arch = "wasm32", feature = "webusb"), target_os = "android")))]
pub use self::not_supported::{devices, open, get_device_descriptor};