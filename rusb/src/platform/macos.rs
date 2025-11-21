//! macOS-specific USB backend implementation.
//!
//! TODO: Add support for isochronous transfers
//! TODO: Add interface claiming/releasing
//! TODO: Add configuration descriptor parsing
//! TODO: Add string descriptor reading
//! TODO: Add device reset support
//! TODO: Add clear halt support
//! TODO: Add hotplug notification using IOServiceAddMatchingNotification
//! TODO: Fix potential memory leak - device_interface is shared between Device and DeviceHandle
//! TODO: Add better error code mapping from IOReturn to Error

use crate::{
    ControlRequest, ControlTransferData, Device, DeviceDescriptor, DeviceList, Error,
    TransferBuffer, TransferDirection,
};
use core_foundation_sys::base::{CFUUIDGetUUIDBytes, kCFAllocatorDefault};
use io_kit_sys::base::{kIOMasterPortDefault, mach_port_t};
use io_kit_sys::iterator::IOIteratorNext;
use io_kit_sys::object::IOObjectRelease;
use io_kit_sys::ret::{IOReturn, kIOReturnSuccess};
use io_kit_sys::service::{IOServiceGetMatchingServices, IOServiceMatching};
use io_kit_sys::types::{io_iterator_t, io_object_t};
use io_kit_sys::usb::{
    IOCreatePlugInInterfaceForService, IOUSBDeviceInterface, kIOCFPlugInInterfaceID,
    kIOUSBDeviceClassName, kIOUSBDeviceInterfaceID, kIOUSBDeviceUserClientTypeID,
};
use std::ffi::c_void;
use std::mem::zeroed;
use std::ptr;
use std::time::Duration;

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
/// TODO: CRITICAL - device_interface is not properly reference counted, causes potential use-after-free
/// TODO: Cache additional device properties (speed, location ID, etc.)
pub struct MacosDevice {
    device_interface: *mut *mut IOUSBDeviceInterface,
    descriptor: DeviceDescriptor,
    // TODO: Add location_id: u32
    // TODO: Add device_speed: DeviceSpeed
}

impl Drop for MacosDevice {
    fn drop(&mut self) {
        unsafe {
            (**self.device_interface).Release(self.device_interface);
        }
    }
}

/// The macOS-specific device handle.
/// TODO: Track claimed interfaces
/// TODO: Store interface handles for composite devices
/// TODO: CRITICAL - Ensure proper cleanup of device_interface pointer
pub struct MacosDeviceHandle {
    device_interface: *mut *mut IOUSBDeviceInterface,
    // TODO: Add interface_handles: HashMap<u8, *mut *mut IOUSBInterfaceInterface>
    // TODO: Add claimed_interfaces: HashSet<u8>
}

impl Drop for MacosDeviceHandle {
    fn drop(&mut self) {
        unsafe {
            (**self.device_interface).USBDeviceClose(self.device_interface);
        }
    }
}

pub fn devices() -> Result<DeviceList, Error> {
    // TODO: Add caching to avoid expensive IOKit enumeration on every call
    // TODO: Filter devices by class/subclass/protocol if needed
    // TODO: Add better error messages for IOKit errors
    let mut devices = Vec::new();
    let mut iterator: io_iterator_t = 0;

    unsafe {
        let matching_dict = IOServiceMatching(kIOUSBDeviceClassName as *const i8);
        if matching_dict.is_null() {
            return Err(Error::Os(-1));
        }

        let result =
            IOServiceGetMatchingServices(kIOMasterPortDefault, matching_dict, &mut iterator);
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
                (**(plugin_interface as *mut *mut IOUSBDeviceInterface)).QueryInterface(
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

                let result =
                    unsafe { (**device_interface).DeviceRequestTO(device_interface, &mut request) };

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
    // TODO: CRITICAL - Increment reference count on device_interface to avoid use-after-free
    // TODO: Consider using USBDeviceOpen first, only use USBDeviceOpenSeize if that fails
    // TODO: Add better error messages (e.g., "device already opened by another process")
    let result = unsafe {
        (**device.inner.device_interface).USBDeviceOpenSeize(device.inner.device_interface)
    };
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

pub fn control_transfer(
    handle: &crate::DeviceHandle,
    request: ControlRequest,
    data: ControlTransferData<'_>,
    timeout: Duration,
) -> Result<usize, Error> {
    if let Some(direction) = data.direction() {
        let expected = if request.request_type & 0x80 != 0 {
            TransferDirection::In
        } else {
            TransferDirection::Out
        };
        if direction != expected {
            return Err(Error::NotSupported);
        }
    }

    if data.len() > u16::MAX as usize {
        return Err(Error::NotSupported);
    }

    let (no_data_timeout, completion_timeout) = timeout_components(timeout);
    let mut request_packet = IOUSBDevRequestTO {
        bmRequestType: request.request_type,
        bRequest: request.request,
        wValue: request.value,
        wIndex: request.index,
        wLength: data.len() as u16,
        pData: match data {
            ControlTransferData::None => ptr::null_mut(),
            ControlTransferData::In(buffer) => buffer.as_mut_ptr() as *mut c_void,
            ControlTransferData::Out(buffer) => buffer.as_ptr() as *mut c_void,
        },
        wLenDone: 0,
        noDataTimeout: no_data_timeout,
        completionTimeout: completion_timeout,
    };

    let status = unsafe {
        (**handle.inner.device_interface)
            .DeviceRequestTO(handle.inner.device_interface, &mut request_packet)
    };
    io_result(status)?;

    Ok(match data {
        ControlTransferData::None => 0,
        _ => request_packet.wLenDone as usize,
    })
}

pub fn bulk_transfer(
    handle: &crate::DeviceHandle,
    endpoint: u8,
    buffer: TransferBuffer<'_>,
    timeout: Duration,
) -> Result<usize, Error> {
    pipe_transfer(handle, endpoint, buffer, timeout)
}

pub fn interrupt_transfer(
    handle: &crate::DeviceHandle,
    endpoint: u8,
    buffer: TransferBuffer<'_>,
    timeout: Duration,
) -> Result<usize, Error> {
    pipe_transfer(handle, endpoint, buffer, timeout)
}

fn pipe_transfer(
    handle: &crate::DeviceHandle,
    endpoint: u8,
    buffer: TransferBuffer<'_>,
    timeout: Duration,
) -> Result<usize, Error> {
    let expected = if endpoint & 0x80 != 0 {
        TransferDirection::In
    } else {
        TransferDirection::Out
    };

    if buffer.direction() != expected {
        return Err(Error::NotSupported);
    }

    let (no_data_timeout, completion_timeout) = timeout_components(timeout);

    unsafe {
        match buffer {
            TransferBuffer::In(buffer) => {
                let mut size = buffer.len() as u32;
                let status = (**handle.inner.device_interface).ReadPipeTO(
                    handle.inner.device_interface,
                    endpoint,
                    buffer.as_mut_ptr() as *mut c_void,
                    &mut size,
                    no_data_timeout,
                    completion_timeout,
                );
                io_result(status)?;
                Ok(size as usize)
            }
            TransferBuffer::Out(buffer) => {
                if buffer.len() > u32::MAX as usize {
                    return Err(Error::NotSupported);
                }
                let status = (**handle.inner.device_interface).WritePipeTO(
                    handle.inner.device_interface,
                    endpoint,
                    buffer.as_ptr() as *mut c_void,
                    buffer.len() as u32,
                    no_data_timeout,
                    completion_timeout,
                );
                io_result(status)?;
                Ok(buffer.len())
            }
        }
    }
}

fn timeout_components(timeout: Duration) -> (u32, u32) {
    let millis = timeout.as_millis().min(u32::MAX as u128) as u32;
    if millis == 0 {
        (0, 0)
    } else {
        (millis, millis)
    }
}

fn io_result(code: IOReturn) -> Result<(), Error> {
    // TODO: Map common IOReturn codes to more specific Error variants
    // kIOReturnNoDevice, kIOReturnNotOpen, kIOReturnTimeout, etc.
    if code == kIOReturnSuccess {
        Ok(())
    } else {
        Err(Error::Os(code))
    }
}

pub fn claim_interface(_handle: &crate::DeviceHandle, _interface: u8) -> Result<(), Error> {
    // TODO: Implement claim_interface via IOKit
    Err(Error::NotSupported)
}

pub fn release_interface(_handle: &crate::DeviceHandle, _interface: u8) -> Result<(), Error> {
    // TODO: Implement release_interface via IOKit
    Err(Error::NotSupported)
}

pub fn set_interface_alt_setting(
    _handle: &crate::DeviceHandle,
    _interface: u8,
    _alt_setting: u8,
) -> Result<(), Error> {
    // TODO: Implement set_interface_alt_setting via IOKit
    Err(Error::NotSupported)
}

pub fn reset_device(_handle: &crate::DeviceHandle) -> Result<(), Error> {
    // TODO: Implement reset_device via IOKit
    Err(Error::NotSupported)
}

pub fn clear_halt(_handle: &crate::DeviceHandle, _endpoint: u8) -> Result<(), Error> {
    // TODO: Implement clear_halt via IOKit
    Err(Error::NotSupported)
}

pub fn detach_kernel_driver(_handle: &crate::DeviceHandle, _interface: u8) -> Result<(), Error> {
    Err(Error::NotSupported)
}

pub fn attach_kernel_driver(_handle: &crate::DeviceHandle, _interface: u8) -> Result<(), Error> {
    Err(Error::NotSupported)
}

// TODO: Add tests for macOS-specific functionality
// TODO: Add tests for proper reference counting of device_interface
// TODO: Add benchmarks comparing against libusb-1.0 on macOS
// TODO: Add support for getting device location ID and port numbers
// TODO: Implement proper interface claiming for composite devices
