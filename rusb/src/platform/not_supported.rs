#![cfg(not(any(
    target_os = "linux",
    target_os = "windows",
    target_os = "macos",
    all(target_arch = "wasm32", feature = "webusb"),
    target_os = "android"
)))]

//! Fallback implementation for unsupported platforms.

use crate::{
    ConfigurationDescriptor, ControlRequest, ControlTransferData, Device, DeviceDescriptor,
    DeviceHandle, DeviceList, Error, Speed, TransferBuffer,
};

/// A placeholder for the device structure.
pub struct NotSupportedDevice;

impl NotSupportedDevice {
    pub fn bus_number(&self) -> u8 {
        0
    }

    pub fn address(&self) -> u8 {
        0
    }

    pub fn speed(&self) -> Speed {
        Speed::Unknown
    }
}

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

pub fn get_active_configuration(_device: &Device) -> Result<ConfigurationDescriptor, Error> {
    Err(Error::NotSupported)
}

pub fn get_configuration_descriptor(
    _device: &Device,
    _index: u8,
) -> Result<ConfigurationDescriptor, Error> {
    Err(Error::NotSupported)
}

pub fn get_config_descriptor_by_value(
    _device: &Device,
    _value: u8,
) -> Result<ConfigurationDescriptor, Error> {
    Err(Error::NotSupported)
}

pub fn claim_interface(_handle: &DeviceHandle, _interface: u8) -> Result<(), Error> {
    Err(Error::NotSupported)
}

pub fn release_interface(_handle: &DeviceHandle, _interface: u8) -> Result<(), Error> {
    Err(Error::NotSupported)
}

pub fn set_interface_alt_setting(
    _handle: &DeviceHandle,
    _interface: u8,
    _alt_setting: u8,
) -> Result<(), Error> {
    Err(Error::NotSupported)
}

pub fn reset_device(_handle: &DeviceHandle) -> Result<(), Error> {
    Err(Error::NotSupported)
}

pub fn clear_halt(_handle: &DeviceHandle, _endpoint: u8) -> Result<(), Error> {
    Err(Error::NotSupported)
}

pub fn detach_kernel_driver(_handle: &DeviceHandle, _interface: u8) -> Result<(), Error> {
    Err(Error::NotSupported)
}

pub fn attach_kernel_driver(_handle: &DeviceHandle, _interface: u8) -> Result<(), Error> {
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
