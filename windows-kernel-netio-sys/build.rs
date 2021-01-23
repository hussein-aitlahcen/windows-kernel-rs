fn main() {
    println!("cargo:rustc-link-lib=netio");
    println!("cargo:rerun-if-changed=src/wrapper_wsk.h");

    windows_kernel_bindgen::generate_bindings("wsk", |x| {
        x
        .whitelist_function("Wsk.*")
        .whitelist_type("PFN_WSK.*")
        .whitelist_type("_WSK.*")
        .whitelist_type("WSK.*")
        .whitelist_type("PWSK.*")
        .whitelist_type("NPIID")
        .whitelist_type("PNPIID")
        .whitelist_var("WSK.*")
        .whitelist_recursively(false)
    });
}
