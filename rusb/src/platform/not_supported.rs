#![cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos", all(target_arch = "wasm32", feature = "webusb"), target_os = "android")))]

//! Fallback implementation for unsupported platforms.

use crate::{Device, DeviceDescriptor, DeviceList, Error};

/// A placeholder for the device structure.
pub struct NotSupportedDevice;

/// A placeholder for the device handle.
pub struct NotSupportedDeviceHandle;

pub fn devices() -> Result<DeviceList, Error> {
    Err(Error::NotSupported)
}

pub fn open(_device: &Device) -> Result<crate::DeviceHandle, Error> {
    Err(Error::NotSupported)
}

pub fn get_device_descriptor(_device: &Device) -> Result<DeviceDescriptor, Error> {
    Err(Error::NotSupported)
}

// Transfer functions to be implemented later.
pub fn control_transfer() -> Result<(), Error> {
    Err(Error::NotSupported)
}

pub fn bulk_transfer() -> Result<(), Error> {
    Err(Error::NotSupported)
}

pub fn interrupt_transfer() -> Result<(), Error> {
    Err(Error::NotSupported)
}