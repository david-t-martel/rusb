#![cfg(all(target_arch = "wasm32", feature = "webusb"))]

//! WebUSB-specific USB backend implementation.

use crate::{Device, DeviceDescriptor, DeviceList, Error};
use wasm_bindgen_futures::JsFuture;
use web_sys::UsbDevice;

/// The WebUSB-specific device structure.
pub struct WasmDevice(pub web_sys::UsbDevice);

/// The WebUSB-specific device handle.
pub struct WasmDeviceHandle {
    pub device: web_sys::UsbDevice,
}

pub fn devices() -> Result<DeviceList, Error> {
    // Implementation to enumerate devices using WebUSB will go here.
    Ok(DeviceList {
        devices: Vec::new(),
    })
}

pub async fn open(device: &Device) -> Result<crate::DeviceHandle, Error> {
    let device = &device.inner.0;
    JsFuture::from(device.open()).await.unwrap();
    Ok(crate::DeviceHandle {
        inner: WasmDeviceHandle {
            device: device.clone(),
        },
    })
}

pub fn get_device_descriptor(device: &Device) -> Result<DeviceDescriptor, Error> {
    unimplemented!()
}