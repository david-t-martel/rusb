#![cfg(target_os = "linux")]

//! Linux-specific USB backend implementation.

use crate::{Device, DeviceDescriptor, DeviceList, Error};
use libudev::{Context, Enumerator};
use std::fs::File;
use std::os::unix::io::AsRawFd;

/// The linux-specific device structure.
pub struct LinuxDevice {
    device: libudev::Device,
}

/// The linux-specific device handle.
pub struct LinuxDeviceHandle {
    _fd: i32,
}

impl LinuxDevice {
    fn from_udev(device: libudev::Device) -> Self {
        Self { device }
    }
}

pub fn devices() -> Result<DeviceList, Error> {
    let context = Context::new().map_err(|e| Error::Os(e.raw_os_error().unwrap_or(0)))?;
    let mut enumerator = Enumerator::new(&context).map_err(|e| Error::Os(e.raw_os_error().unwrap_or(0)))?;

    enumerator
        .add_match_subsystem("usb")
        .map_err(|e| Error::Os(e.raw_os_error().unwrap_or(0)))?;

    let devices = enumerator
        .scan_devices()
        .map_err(|e| Error::Os(e.raw_os_error().unwrap_or(0)))?
        .map(LinuxDevice::from_udev)
        .map(|ld| Device { inner: ld })
        .collect::<Vec<Device>>();

    Ok(DeviceList { devices })
}

pub fn open(device: &Device) -> Result<crate::DeviceHandle, Error> {
    let devnum = device.inner.device.devnum();
    let busnum = device.inner.device.busnum();
    let path = format!("/dev/bus/usb/{:03}/{:03}", busnum, devnum);

    let file = File::open(path).map_err(|e| Error::Os(e.raw_os_error().unwrap_or(0)))?;

    Ok(crate::DeviceHandle {
        inner: LinuxDeviceHandle {
            _fd: file.as_raw_fd(),
        },
    })
}

pub fn get_device_descriptor(device: &Device) -> Result<DeviceDescriptor, Error> {
    let dev = &device.inner.device;

    let vid = get_sysattr_as::<u16>(dev, "idVendor")?;
    let pid = get_sysattr_as::<u16>(dev, "idProduct")?;
    let device_class = get_sysattr_as::<u8>(dev, "bDeviceClass")?;
    let device_subclass = get_sysattr_as::<u8>(dev, "bDeviceSubClass")?;
    let device_protocol = get_sysattr_as::<u8>(dev, "bDeviceProtocol")?;
    let num_configurations = get_sysattr_as::<u8>(dev, "bNumConfigurations")?;
    let usb_version = get_sysattr_as::<u16>(dev, "bcdUSB")?;
    let device_version = get_sysattr_as::<u16>(dev, "bcdDevice")?;
    let max_packet_size_0 = get_sysattr_as::<u8>(dev, "bMaxPacketSize0")?;

    // These are string indexes, not the strings themselves.
    let manufacturer_string_index = get_sysattr_as::<u8>(dev, "iManufacturer").unwrap_or(0);
    let product_string_index = get_sysattr_as::<u8>(dev, "iProduct").unwrap_or(0);
    let serial_number_string_index = get_sysattr_as::<u8>(dev, "iSerialNumber").unwrap_or(0);

    Ok(DeviceDescriptor {
        length: 18, // bLength, always 18 for device descriptor
        descriptor_type: 0x01, // bDescriptorType, DEVICE
        usb_version,
        device_class,
        device_subclass,
        device_protocol,
        max_packet_size_0,
        vendor_id: vid,
        product_id: pid,
        device_version,
        manufacturer_string_index,
        product_string_index,
        serial_number_string_index,
        num_configurations,
    })
}

// Helper function to read and parse a sysattr value from udev.
fn get_sysattr_as<T: std::str::FromStr>(device: &libudev::Device, attr: &str) -> Result<T, Error> {
    let val_str = device
        .sysattr_value(attr)
        .ok_or(Error::Os(0))? // Better error needed
        .to_str()
        .ok_or(Error::Os(0))?; // Better error needed

    // udev stores hex values as plain strings, so we parse from hex.
    T::from_str(&val_str).map_err(|_| Error::Os(0)) // Better error needed
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
