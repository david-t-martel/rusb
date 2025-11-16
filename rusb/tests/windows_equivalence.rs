#![cfg(target_os = "windows")]

use libloading::Library;
use std::collections::BTreeSet;
use std::ffi::c_int;
use std::ptr;

#[test]
fn compare_with_c_libusb() {
    if std::env::var("RUSB_COMPARE_LIBUSB").ok().as_deref() != Some("1") {
        return;
    }

    if let Err(err) = run_compare() {
        panic!("libusb equivalence test failed: {err}");
    }
}

fn run_compare() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        let api = LibusbApi::load()?;
        let mut ctx = ptr::null_mut();
        (api.init)(&mut ctx);
        let mut list = ptr::null_mut();
        let count = (api.get_device_list)(ctx, &mut list);
        if count < 0 {
            (api.exit)(ctx);
            return Err("libusb_get_device_list returned error".into());
        }

        let c_set = collect_c_descriptors(&api, list, count as usize)?;
        (api.free_device_list)(list, 1);
        (api.exit)(ctx);

        let rust_devices = rusb::devices()?;
        let rust_set = rust_devices
            .iter()
            .filter_map(|dev| dev.get_device_descriptor().ok())
            .map(|desc| (desc.vendor_id, desc.product_id))
            .collect::<BTreeSet<_>>();

        assert_eq!(rust_set, c_set, "Rust and C libusb views differ");
    }
    Ok(())
}

unsafe fn collect_c_descriptors(
    api: &LibusbApi,
    list: *const *mut libusb_device,
    len: usize,
) -> Result<BTreeSet<(u16, u16)>, Box<dyn std::error::Error>> {
    let devices = std::slice::from_raw_parts(list, len);
    let mut set = BTreeSet::new();
    for &dev in devices {
        let mut descriptor = libusb_device_descriptor::default();
        if (api.get_device_descriptor)(dev, &mut descriptor) == 0 {
            set.insert((descriptor.idVendor, descriptor.idProduct));
        }
    }
    Ok(set)
}

struct LibusbApi {
    _lib: Library,
    init: unsafe extern "C" fn(*mut *mut libusb_context) -> c_int,
    exit: unsafe extern "C" fn(*mut libusb_context),
    get_device_list:
        unsafe extern "C" fn(*mut libusb_context, *mut *const *mut libusb_device) -> isize,
    free_device_list: unsafe extern "C" fn(*const *mut libusb_device, c_int),
    get_device_descriptor:
        unsafe extern "C" fn(*mut libusb_device, *mut libusb_device_descriptor) -> c_int,
}

impl LibusbApi {
    unsafe fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let lib = Library::new("libusb-1.0.dll")?;
        Ok(Self {
            init: *lib.get(b"libusb_init\0")?,
            exit: *lib.get(b"libusb_exit\0")?,
            get_device_list: *lib.get(b"libusb_get_device_list\0")?,
            free_device_list: *lib.get(b"libusb_free_device_list\0")?,
            get_device_descriptor: *lib.get(b"libusb_get_device_descriptor\0")?,
            _lib: lib,
        })
    }
}

#[repr(C)]
struct libusb_context;
#[repr(C)]
struct libusb_device;

#[repr(C)]
#[derive(Default)]
struct libusb_device_descriptor {
    _length: u8,
    _descriptor_type: u8,
    _bcd_usb: u16,
    _device_class: u8,
    _device_sub_class: u8,
    _device_protocol: u8,
    _max_packet_size0: u8,
    idVendor: u16,
    idProduct: u16,
    _bcd_device: u16,
    _i_manufacturer: u8,
    _i_product: u8,
    _i_serial_number: u8,
    _num_configurations: u8,
}
