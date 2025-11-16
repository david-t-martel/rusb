#![cfg(target_os = "android")]

//! Android-specific USB backend implementation.

use crate::{Device, DeviceDescriptor, DeviceList, Error};
use jni::JNIEnv;
use jni::objects::{JObject, JValue};

/// The Android-specific device structure.
pub struct AndroidDevice {
    // To be implemented: Should hold a JObject representing the Android UsbDevice.
}

/// The Android-specific device handle.
pub struct AndroidDeviceHandle<'a> {
    pub connection: JObject<'a>,
}

pub fn devices() -> Result<DeviceList, Error> {
    // To be implemented:
    // 1. Get the UsbManager system service.
    // 2. Call getDeviceList() to get a HashMap of devices.
    // 3. Iterate over the HashMap and create AndroidDevice structs.
    // See: https://developer.android.com/reference/android/hardware/usb/UsbManager#getDeviceList()
    Ok(DeviceList {
        devices: Vec::new(),
    })
}

pub fn open<'a>(_env: &'a JNIEnv<'a>, _device: JObject<'a>) -> Result<crate::DeviceHandle, Error> {
    // To be implemented:
    // 1. Get the UsbManager.
    // 2. Call openDevice() on the UsbManager with the UsbDevice object.
    // 3. This requires handling user permissions.
    // See: https://developer.android.com/reference/android/hardware/usb/UsbManager#openDevice(android.hardware.usb.UsbDevice)
    unimplemented!()
}

pub fn get_device_descriptor(_device: &Device) -> Result<DeviceDescriptor, Error> {
    // To be implemented:
    // 1. The DeviceDescriptor is not directly available from the Android UsbDevice object.
    // 2. You must use the UsbDeviceConnection.
    // 3. Call getRawDescriptors() to get the raw descriptor data as a byte array.
    // 4. Parse the byte array to extract the DeviceDescriptor.
    // See: https://developer.android.com/reference/android/hardware/usb/UsbDeviceConnection#getRawDescriptors()
    unimplemented!()
}

// Transfer functions to be implemented later.
pub fn control_transfer() -> Result<(), Error> {
    Ok(())
}
pub fn bulk_transfer() -> Result<(), Error> {
    Ok(())
}
pub fn interrupt_transfer() -> Result<(), Error> {
    Ok(())
}