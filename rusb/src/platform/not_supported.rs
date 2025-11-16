#![cfg(not(any(
    target_os = "linux",
    target_os = "windows",
    target_os = "macos",
    all(target_arch = "wasm32", feature = "webusb"),
    target_os = "android"
)))]

//! Fallback implementation for unsupported platforms.

use crate::{
    ControlRequest, ControlTransferData, Device, DeviceDescriptor, DeviceHandle, DeviceList, Error,
    TransferBuffer,
};

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

pub fn control_transfer(
    _handle: &DeviceHandle,
    _request: ControlRequest,
    _data: ControlTransferData<'_>,
    _timeout: std::time::Duration,
) -> Result<usize, Error> {
    Err(Error::NotSupported)
}

pub fn bulk_transfer(
    _handle: &DeviceHandle,
    _endpoint: u8,
    _buffer: TransferBuffer<'_>,
    _timeout: std::time::Duration,
) -> Result<usize, Error> {
    Err(Error::NotSupported)
}

pub fn interrupt_transfer(
    _handle: &DeviceHandle,
    _endpoint: u8,
    _buffer: TransferBuffer<'_>,
    _timeout: std::time::Duration,
) -> Result<usize, Error> {
    Err(Error::NotSupported)
}
