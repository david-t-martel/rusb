#![cfg(target_os = "windows")]

use libloading::Library;
use std::collections::BTreeSet;
use std::env;
use std::ffi::c_int;
use std::ptr;
use std::time::{Duration, Instant};

#[test]
fn compare_windows_transfers() {
    print_debug_hints();
    let spec = match TestSpec::from_env() {
        Some(spec) => spec,
        None => {
            eprintln!("windows_compare: set RUSB_TEST_VID/RUSB_TEST_PID to opt in");
            return;
        }
    };

    let devices = match rusb::devices() {
        Ok(list) => list,
        Err(err) => {
            eprintln!("windows_compare: failed to list rusb devices: {err:?}");
            return;
        }
    };

    let device = match devices
        .iter()
        .find(|dev| match dev.get_device_descriptor() {
            Ok(desc) => desc.vendor_id == spec.vid && desc.product_id == spec.pid,
            Err(_) => false,
        }) {
        Some(dev) => dev,
        None => {
            eprintln!(
                "windows_compare: could not find device {:04x}:{:04x} via rusb",
                spec.vid, spec.pid
            );
            return;
        }
    };

    let handle = match device.open() {
        Ok(handle) => handle,
        Err(err) => {
            eprintln!("windows_compare: rusb open failed: {err:?}");
            return;
        }
    };

    let api = match unsafe { LibusbApi::new() } {
        Ok(api) => api,
        Err(err) => {
            eprintln!("windows_compare: failed to load libusb: {err}");
            return;
        }
    };

    let mut ctx = ptr::null_mut();
    unsafe {
        (api.init)(&mut ctx);
    }

    let handle_c = unsafe { (api.open_device_with_vid_pid)(ctx, spec.vid, spec.pid) };
    if handle_c.is_null() {
        unsafe { (api.exit)(ctx) };
        eprintln!(
            "windows_compare: libusb_open_device_with_vid_pid could not open {:04x}:{:04x}",
            spec.vid, spec.pid
        );
        return;
    }

    let mut info = Vec::new();
    if spec.do_control {
        if let Some(result) = run_control_pair(&handle, &api, handle_c) {
            info.push(result);
        }
    }

    if let (Some(out_ep), Some(in_ep)) = (spec.bulk_out, spec.bulk_in) {
        if let Some(result) = run_bulk_pair(&handle, &api, handle_c, out_ep, in_ep) {
            info.push(result);
        }
    }

    if let Some(ep) = spec.interrupt_ep {
        if let Some(result) = run_interrupt_pair(&handle, &api, handle_c, ep) {
            info.push(result);
        }
    }

    unsafe {
        (api.close)(handle_c);
        (api.exit)(ctx);
    }

    if info.is_empty() {
        eprintln!("windows_compare: no tests executed; provide endpoint env vars to opt in");
    } else {
        eprintln!("windows_compare summary:");
        for line in info {
            eprintln!("  - {line}");
        }
    }
}

struct TestSpec {
    vid: u16,
    pid: u16,
    bulk_in: Option<u8>,
    bulk_out: Option<u8>,
    interrupt_ep: Option<u8>,
    iterations: usize,
    do_control: bool,
}

impl TestSpec {
    fn from_env() -> Option<Self> {
        let vid = parse_env_u16("RUSB_TEST_VID")?;
        let pid = parse_env_u16("RUSB_TEST_PID")?;
        let bulk_in = parse_env_u8("RUSB_TEST_BULK_IN");
        let bulk_out = parse_env_u8("RUSB_TEST_BULK_OUT");
        let interrupt_ep = parse_env_u8("RUSB_TEST_INTERRUPT");
        let iterations = env
            .var("RUSB_TEST_ITERS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(16);
        let do_control = env
            .var("RUSB_TEST_CONTROL")
            .map(|v| v != "0")
            .unwrap_or(true);

        Some(Self {
            vid,
            pid,
            bulk_in,
            bulk_out,
            interrupt_ep,
            iterations,
            do_control,
        })
    }
}

fn parse_env_u16(key: &str) -> Option<u16> {
    parse_env_u32(key).map(|v| v as u16)
}

fn parse_env_u8(key: &str) -> Option<u8> {
    parse_env_u32(key).map(|v| v as u8)
}

fn parse_env_u32(key: &str) -> Option<u32> {
    env::var(key)
        .ok()
        .and_then(|v| u32::from_str_radix(v.trim_start_matches("0x"), 16).ok())
}

fn run_control_pair(
    rusb_handle: &rusb::DeviceHandle,
    api: &LibusbApi,
    c_handle: *mut libusb_device_handle,
) -> Option<String> {
    let mut buf = [0u8; 64];
    let request = rusb::ControlRequest {
        request_type: 0x80,
        request: 0x06,
        value: 0x0100,
        index: 0,
    };
    let mut rusb_total = Duration::ZERO;
    for _ in 0..5 {
        let start = Instant::now();
        let res = rusb_handle.control_transfer(
            request,
            rusb::ControlTransferData::In(&mut buf),
            Duration::from_millis(200),
        );
        match res {
            Ok(_) => {
                rusb_total += start.elapsed();
            }
            Err(err) => {
                eprintln!("control_transfer (rusb) failed: {err:?}");
                return None;
            }
        }
    }

    let mut libusb_total = Duration::ZERO;
    for _ in 0..5 {
        let start = Instant::now();
        let res = unsafe {
            (api.control_transfer)(
                c_handle,
                0x80,
                0x06,
                0x0100,
                0,
                buf.as_mut_ptr(),
                buf.len() as u16,
                200,
            )
        };
        if res < 0 {
            eprintln!("control_transfer (libusb) failed: error {res}");
            return None;
        }
        libusb_total += start.elapsed();
    }

    let rusb_avg = rusb_total / 5;
    let libusb_avg = libusb_total / 5;
    Some(format!(
        "control descriptor fetch: rusb {:?}, libusb {:?}",
        rusb_avg, libusb_avg
    ))
}

fn run_bulk_pair(
    rusb_handle: &rusb::DeviceHandle,
    api: &LibusbApi,
    c_handle: *mut libusb_device_handle,
    bulk_out: u8,
    bulk_in: u8,
) -> Option<String> {
    let iterations = env::var("RUSB_TEST_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8);
    let payload = vec![0xA5; 4096];
    let mut read_buf = vec![0u8; 4096];
    let mut rusb_total = Duration::ZERO;
    for _ in 0..iterations {
        let start = Instant::now();
        if let Err(err) = rusb_handle.bulk_transfer(
            bulk_out,
            rusb::TransferBuffer::Out(&payload),
            Duration::from_millis(500),
        ) {
            eprintln!("rusb bulk OUT failed: {err:?}");
            return None;
        }
        if let Err(err) = rusb_handle.bulk_transfer(
            bulk_in,
            rusb::TransferBuffer::In(&mut read_buf),
            Duration::from_millis(500),
        ) {
            eprintln!("rusb bulk IN failed: {err:?}");
            return None;
        }
        rusb_total += start.elapsed();
    }

    let mut libusb_total = Duration::ZERO;
    for _ in 0..iterations {
        let start = Instant::now();
        unsafe {
            let mut transferred = 0;
            let rc = (api.bulk_transfer)(
                c_handle,
                bulk_out,
                payload.as_ptr() as *mut u8,
                payload.len() as c_int,
                &mut transferred,
                500,
            );
            if rc < 0 {
                eprintln!("libusb bulk OUT failed: {rc}");
                return None;
            }
            let rc = (api.bulk_transfer)(
                c_handle,
                bulk_in,
                read_buf.as_mut_ptr(),
                read_buf.len() as c_int,
                &mut transferred,
                500,
            );
            if rc < 0 {
                eprintln!("libusb bulk IN failed: {rc}");
                return None;
            }
        }
        libusb_total += start.elapsed();
    }

    let rusb_avg = rusb_total / iterations as u32;
    let libusb_avg = libusb_total / iterations as u32;
    Some(format!(
        "bulk round-trip {:?} (rusb) vs {:?} (libusb)",
        rusb_avg, libusb_avg
    ))
}

fn run_interrupt_pair(
    rusb_handle: &rusb::DeviceHandle,
    api: &LibusbApi,
    c_handle: *mut libusb_device_handle,
    ep: u8,
) -> Option<String> {
    let mut buf = vec![0u8; 64];
    let start = Instant::now();
    if let Err(err) = rusb_handle.interrupt_transfer(
        ep,
        rusb::TransferBuffer::In(&mut buf),
        Duration::from_millis(500),
    ) {
        eprintln!("rusb interrupt IN failed: {err:?}");
        return None;
    }
    let rusb_dur = start.elapsed();

    let mut transferred = 0;
    let start = Instant::now();
    let rc = unsafe {
        (api.interrupt_transfer)(
            c_handle,
            ep,
            buf.as_mut_ptr(),
            buf.len() as c_int,
            &mut transferred,
            500,
        )
    };
    if rc < 0 {
        eprintln!("libusb interrupt IN failed: {rc}");
        return None;
    }
    let libusb_dur = start.elapsed();
    Some(format!(
        "interrupt poll {:?} (rusb) vs {:?} (libusb)",
        rusb_dur, libusb_dur
    ))
}

fn print_debug_hints() {
    eprintln!("windows_compare debug helpers:");
    eprintln!("  * usbview.exe (Windows SDK) to inspect endpoints");
    eprintln!("  * Get-PnpDevice -PresentOnly | Where-Object { $_.InstanceId -like 'USB*' }");
    eprintln!("  * pnputil /enum-devices /connected");
}

struct LibusbApi {
    _lib: Library,
    init: unsafe extern "C" fn(*mut *mut libusb_context) -> c_int,
    exit: unsafe extern "C" fn(*mut libusb_context),
    open_device_with_vid_pid:
        unsafe extern "C" fn(*mut libusb_context, u16, u16) -> *mut libusb_device_handle,
    close: unsafe extern "C" fn(*mut libusb_device_handle),
    control_transfer: unsafe extern "C" fn(
        *mut libusb_device_handle,
        u8,
        u8,
        u16,
        u16,
        *mut u8,
        u16,
        u32,
    ) -> c_int,
    bulk_transfer: unsafe extern "C" fn(
        *mut libusb_device_handle,
        u8,
        *mut u8,
        c_int,
        *mut c_int,
        u32,
    ) -> c_int,
    interrupt_transfer: unsafe extern "C" fn(
        *mut libusb_device_handle,
        u8,
        *mut u8,
        c_int,
        *mut c_int,
        u32,
    ) -> c_int,
}

impl LibusbApi {
    unsafe fn new() -> Result<Self, String> {
        let lib = Library::new("libusb-1.0.dll").map_err(|e| e.to_string())?;
        Ok(Self {
            init: *lib.get(b"libusb_init\0").map_err(|e| e.to_string())?,
            exit: *lib.get(b"libusb_exit\0").map_err(|e| e.to_string())?,
            open_device_with_vid_pid: *lib
                .get(b"libusb_open_device_with_vid_pid\0")
                .map_err(|e| e.to_string())?,
            close: *lib.get(b"libusb_close\0").map_err(|e| e.to_string())?,
            control_transfer: *lib
                .get(b"libusb_control_transfer\0")
                .map_err(|e| e.to_string())?,
            bulk_transfer: *lib
                .get(b"libusb_bulk_transfer\0")
                .map_err(|e| e.to_string())?,
            interrupt_transfer: *lib
                .get(b"libusb_interrupt_transfer\0")
                .map_err(|e| e.to_string())?,
            _lib: lib,
        })
    }
}

#[repr(C)]
struct libusb_context;
#[repr(C)]
struct libusb_device_handle;
