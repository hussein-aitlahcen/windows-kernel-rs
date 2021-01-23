fn main() {
    println!("cargo:rustc-link-lib=ntoskrnl");
    println!("cargo:rerun-if-changed=src/wrapper_ntifs.h");

    windows_kernel_bindgen::generate_bindings("ntifs", |x| 
        x
        .blacklist_type(".*")
        .whitelist_function(".*")
        .whitelist_recursively(false)
    );

    let include_dir = windows_kernel_bindgen::get_windows_kits_km_dir(windows_kernel_bindgen::KernelDirectoryType::Include)
        .expect("Failed to retrieve windows kits km dir");

    cc::Build::new()
        .flag("/kernel")
        .include(include_dir)
        .file("src/exception_free.c")
        .compile("exception_free");
}
