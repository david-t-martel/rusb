//! Windows-specific USB backend implementation.

//! Windows-specific USB backend implementation.

use crate::{Device, DeviceDescriptor, DeviceList, Error};
use std::ffi::OsString;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::ffi::OsStringExt;
use windows::core::{GUID, PCWSTR};
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW,
    SetupDiGetDeviceInterfaceDetailW, DIGCF_DEVICEINTERFACE, DIGCF_PRESENT,
    SP_DEVICE_INTERFACE_DATA, SP_DEVICE_INTERFACE_DETAIL_DATA_W,
};
use windows::Win32::Devices::Usb::{
    WinUsb_Free, WinUsb_GetDescriptor, WinUsb_Initialize, WINUSB_INTERFACE_HANDLE,
};
use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_OVERLAPPED, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};

// Same GUID as in libusb's windows_winusb.c
const GUID_DEVINTERFACE_USB_DEVICE: GUID = GUID::from_u128(0xA5DCBF10_6530_11D2_901F_00C04FB951ED);

/// The windows-specific device structure.
#[derive(Debug)]
pub struct WindowsDevice {
    device_path: OsString,
}

/// The windows-specific device handle.
pub struct WindowsDeviceHandle {
    pub handle: HANDLE,
}

impl Drop for WindowsDeviceHandle {
    fn drop(&mut self) {
        if self.handle != INVALID_HANDLE_VALUE {
            let _ = unsafe { CloseHandle(self.handle) };
        }
    }
}

pub fn devices() -> Result<DeviceList, Error> {
    let dev_info_set = unsafe {
        SetupDiGetClassDevsW(
            Some(&GUID_DEVINTERFACE_USB_DEVICE),
            None,
            None,
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        )
    }?;

    if dev_info_set.is_invalid() {
        return Err(Error::from(windows::core::Error::from_win32()));
    }

    let mut devices = Vec::new();
    let mut dev_interface_data: SP_DEVICE_INTERFACE_DATA = unsafe { std::mem::zeroed() };
    dev_interface_data.cbSize = std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32;
    let mut i = 0;

    while unsafe { SetupDiEnumDeviceInterfaces(dev_info_set, None, &GUID_DEVINTERFACE_USB_DEVICE, i, &mut dev_interface_data) }.is_ok() {
        i += 1;
        let mut required_size = 0;

        // First call to get the required buffer size
        unsafe {
            let _ = SetupDiGetDeviceInterfaceDetailW(
                dev_info_set,
                &dev_interface_data,
                None,
                0,
                Some(&mut required_size),
                None,
            );
        };

        let mut detail_data_buffer = vec![0u8; required_size as usize];
        let detail_data = detail_data_buffer.as_mut_ptr() as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W;
        unsafe { (*detail_data).cbSize = std::mem::size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32 };

        if unsafe {
            SetupDiGetDeviceInterfaceDetailW(
                dev_info_set,
                &dev_interface_data,
                Some(detail_data),
                required_size,
                Some(&mut required_size),
                None,
            )
        }
        .is_err()
        {
            continue;
        }

        let device_path_wide = unsafe { (*detail_data).DevicePath.as_slice() };
        let null_pos = device_path_wide.iter().position(|&c| c == 0).unwrap_or(device_path_wide.len());
        let device_path = OsString::from_wide(&device_path_wide[..null_pos]);

        devices.push(Device {
            inner: WindowsDevice { device_path },
        });
    }

    unsafe { let _ = SetupDiDestroyDeviceInfoList(dev_info_set); };

    Ok(DeviceList { devices })
}

pub fn open(device: &Device) -> Result<crate::DeviceHandle, Error> {
    let path_wide: Vec<u16> = device.inner.device_path.encode_wide().chain(Some(0)).collect();

    let handle = unsafe {
        CreateFileW(
            PCWSTR(path_wide.as_ptr()),
            0x80000000 | 0x40000000, // GENERIC_READ | GENERIC_WRITE
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_OVERLAPPED,
            None,
        )
    }?;

    if handle == INVALID_HANDLE_VALUE {
        return Err(Error::from(windows::core::Error::from_win32()));
    }

    Ok(crate::DeviceHandle {
        inner: WindowsDeviceHandle { handle },
    })
}

pub fn get_device_descriptor(device: &Device) -> Result<DeviceDescriptor, Error> {
    let handle = open(device)?;
    let mut usb_handle: WINUSB_INTERFACE_HANDLE = WINUSB_INTERFACE_HANDLE(0);

    if unsafe { WinUsb_Initialize(handle.inner.handle, &mut usb_handle) }.is_err() {
        return Err(Error::from(windows::core::Error::from_win32()));
    }

    let mut descriptor: DeviceDescriptor = unsafe { std::mem::zeroed() };
    let mut length = 0;

    let result = unsafe {
        WinUsb_GetDescriptor(
            usb_handle,
            0x01, // USB_DEVICE_DESCRIPTOR_TYPE
            0,
            0,
            Some(std::slice::from_raw_parts_mut(
                &mut descriptor as *mut _ as *mut u8,
                std::mem::size_of::<DeviceDescriptor>(),
            )),
            &mut length,
        )
    };

    unsafe { WinUsb_Free(usb_handle) };

    if result.is_err() {
        return Err(Error::from(windows::core::Error::from_win32()));
    }

    Ok(descriptor)
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