//! macOS-specific USB backend implementation.

use crate::{Device, DeviceDescriptor, DeviceList, Error};
use core_foundation_sys::base::{kCFAllocatorDefault, CFUUIDGetUUIDBytes};
use io_kit_sys::base::{kIOMasterPortDefault, mach_port_t};
use io_kit_sys::iterator::IOIteratorNext;
use io_kit_sys::object::IOObjectRelease;
use io_kit_sys::service::{IOServiceGetMatchingServices, IOServiceMatching};
use io_kit_sys::types::{io_iterator_t, io_object_t};
use io_kit_sys::usb::{
    kIOUSBDeviceClassName, kIOUSBDeviceUserClientTypeID, kIOCFPlugInInterfaceID,
    kIOUSBDeviceInterfaceID, IOUSBDeviceInterface, IOCreatePlugInInterfaceForService,
};
use std::mem::zeroed;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct IOUSBDevRequestTO {
    pub bmRequestType: u8,
    pub bRequest: u8,
    pub wValue: u16,
    pub wIndex: u16,
    pub wLength: u16,
    pub pData: *mut ::std::os::raw::c_void,
    pub wLenDone: u32,
    pub noDataTimeout: u32,
    pub completionTimeout: u32,
}

/// The macOS-specific device structure.
pub struct MacosDevice {
    device_interface: *mut *mut IOUSBDeviceInterface,
    descriptor: DeviceDescriptor,
}

impl Drop for MacosDevice {
    fn drop(&mut self) {
        unsafe {
            (**self.device_interface).Release(self.device_interface);
        }
    }
}

/// The macOS-specific device handle.
pub struct MacosDeviceHandle {
    device_interface: *mut *mut IOUSBDeviceInterface,
}

impl Drop for MacosDeviceHandle {
    fn drop(&mut self) {
        unsafe {
            (**self.device_interface).USBDeviceClose(self.device_interface);
        }
    }
}

pub fn devices() -> Result<DeviceList, Error> {
    let mut devices = Vec::new();
    let mut iterator: io_iterator_t = 0;

    unsafe {
        let matching_dict = IOServiceMatching(kIOUSBDeviceClassName as *const i8);
        if matching_dict.is_null() {
            return Err(Error::Os(-1));
        }

        let result = IOServiceGetMatchingServices(kIOMasterPortDefault, matching_dict, &mut iterator);
        if result != 0 {
            return Err(Error::Os(result));
        }
    }

    let mut device = unsafe { IOIteratorNext(iterator) };
    while device != 0 {
        let mut plugin_interface = std::ptr::null_mut();
        let mut score = 0;
        let result = unsafe {
            IOCreatePlugInInterfaceForService(
                device,
                kIOUSBDeviceUserClientTypeID,
                kIOCFPlugInInterfaceID,
                &mut plugin_interface,
                &mut score,
            )
        };

        if result == 0 {
            let mut device_interface = std::ptr::null_mut();
            let result = unsafe {
                (**(plugin_interface as *mut *mut IOUSBDeviceInterface))
                    .QueryInterface(
                        plugin_interface,
                        CFUUIDGetUUIDBytes(kIOUSBDeviceInterfaceID),
                        &mut device_interface,
                    )
            };

            if result == 0 {
                let mut descriptor: DeviceDescriptor = unsafe { zeroed() };
                let mut request = IOUSBDevRequestTO {
                    bmRequestType: 0x80,
                    bRequest: 6,
                    wValue: (1 << 8) | 0,
                    wIndex: 0,
                    wLength: std::mem::size_of::<DeviceDescriptor>() as u16,
                    pData: &mut descriptor as *mut _ as *mut std::ffi::c_void,
                    wLenDone: 0,
                    noDataTimeout: 1000,
                    completionTimeout: 1000,
                };

                let result = unsafe {
                    (**device_interface).DeviceRequestTO(device_interface, &mut request)
                };

                if result == 0 {
                    devices.push(Device {
                        inner: MacosDevice {
                            device_interface,
                            descriptor,
                        },
                    });
                }
            }
            unsafe {
                (**(plugin_interface as *mut *mut IOUSBDeviceInterface)).Release(plugin_interface);
            }
        }

        unsafe { IOObjectRelease(device) };
        device = unsafe { IOIteratorNext(iterator) };
    }

    unsafe {
        IOObjectRelease(iterator);
    }

    Ok(DeviceList { devices })
}

pub fn open(device: &Device) -> Result<crate::DeviceHandle, Error> {
    let result = unsafe { (**device.inner.device_interface).USBDeviceOpenSeize(device.inner.device_interface) };
    if result != 0 {
        return Err(Error::Os(result));
    }

    Ok(crate::DeviceHandle {
        inner: MacosDeviceHandle {
            device_interface: device.inner.device_interface,
        },
    })
}

pub fn get_device_descriptor(device: &Device) -> Result<DeviceDescriptor, Error> {
    Ok(device.inner.descriptor.clone())
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