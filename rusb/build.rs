use std::env;

fn main() {
    let target = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let has_webusb = env::var("CARGO_FEATURE_WEBUSB").is_ok();
    if target == "wasm32" && has_webusb {
        println!("cargo:rustc-cfg=web_sys_unstable_apis");
    }
}
