#![cfg(target_os = "android")]

//! Android-specific USB backend implementation.

use crate::{Device, DeviceDescriptor, DeviceList, Error};
use jni::JNIEnv;
use jni::objects::{JObject, JValue};

/// The Android-specific device structure.
pub struct AndroidDevice {
    // Information about the device, like its Android UsbDevice object.
}

/// The Android-specific device handle.
pub struct AndroidDeviceHandle<'a> {
    pub connection: JObject<'a>,
}

pub fn devices() -> Result<DeviceList, Error> {
    // Implementation to enumerate devices using JNI will go here.
    Ok(DeviceList {
        devices: Vec::new(),
    })
}

pub fn open<'a>(env: &'a JNIEnv<'a>, device: JObject<'a>) -> Result<crate::DeviceHandle, Error> {
    let usb_manager = env.get_static_field("android/content/Context", "USB_SERVICE", "Ljava/lang/String;").unwrap().l().unwrap();
    let usb_manager = env.call_method(env.get_context(), "getSystemService", "(Ljava/lang/String;)Ljava/lang/Object;", &[JValue::from(usb_manager)]).unwrap().l().unwrap();
    let connection = env.call_method(usb_manager, "openDevice", "(Landroid/hardware/usb/UsbDevice;)Landroid/hardware/usb/UsbDeviceConnection;", &[JValue::from(device)]).unwrap().l().unwrap();

    Ok(crate::DeviceHandle {
        inner: AndroidDeviceHandle {
            connection,
        },
    })
}

pub fn get_device_descriptor(device: &Device) -> Result<DeviceDescriptor, Error> {
    unimplemented!()
}