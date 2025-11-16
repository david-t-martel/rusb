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
    // To be implemented:
    // 1. Use the `web_sys::usb()` interface to get the USB singleton.
    // 2. Call `request_device()` to prompt the user to select a device.
    // 3. This is an async operation and will require a different function signature.
    // See: https://developer.mozilla.org/en-US/docs/Web/API/USB/requestDevice
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
    // To be implemented:
    // The DeviceDescriptor fields are directly accessible as properties of the web_sys::UsbDevice object.
    // See: https://developer.mozilla.org/en-US/docs/Web/API/USBDevice
    let usb_device = &device.inner.0;
    Ok(DeviceDescriptor {
        length: 18,
        descriptor_type: 1,
        usb_version: usb_device.usb_version_major() as u16, // Note: This is just the major version.
        device_class: usb_device.device_class(),
        device_subclass: usb_device.device_subclass(),
        device_protocol: usb_device.device_protocol(),
        max_packet_size_0: 0, // Not directly available.
        vendor_id: usb_device.vendor_id(),
        product_id: usb_device.product_id(),
        device_version: usb_device.device_version_major() as u16, // Note: This is just the major version.
        manufacturer_string_index: 0, // Not available.
        product_string_index: 0, // Not available.
        serial_number_string_index: 0, // Not available.
        num_configurations: usb_device.configurations().length() as u8,
    })
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