fn main() {
    let lib_dir = windows_kernel_bindgen::get_windows_kits_km_dir(windows_kernel_bindgen::KernelDirectoryType::Library)
        .expect("Failed to retrieve windows kits km dir")
        .join("x64");

    println!("cargo:rustc-link-search=native={}", lib_dir.to_str().unwrap());
}