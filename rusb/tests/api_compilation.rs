use rusb::DeviceHandle;

#[allow(dead_code)]
fn test_api(handle: &DeviceHandle) {
    let _ = handle.claim_interface(0);
    let _ = handle.release_interface(0);
    let _ = handle.set_interface_alt_setting(0, 0);
    let _ = handle.reset_device();
    let _ = handle.clear_halt(0x81);
    let _ = handle.detach_kernel_driver(0);
    let _ = handle.attach_kernel_driver(0);
    #[cfg(not(all(target_arch = "wasm32", feature = "webusb")))]
    let _ = handle.read_string_descriptor_ascii(1);
}
